"""Read the stored headless browser receipt and independently verify Canvas pixels."""

import argparse
from hashlib import file_digest
import json
from math import floor
from pathlib import Path
import re

from PIL import Image

PREFIX = "material-text-field-disabled-contrast"
SCENES = ("outlined", "filled", "adorned", "counter", "multiline")
PHASES = ("normal", "forced", "restored")
STEPS = [f"M{i}" for i in (*range(241, 248), *range(357, 360))]
SLOTS = {
    "adorned": {"disabled-leading-icon", "disabled-trailing-icon",
                "disabled-prefix-visual", "disabled-suffix-visual"},
    "counter": {"disabled-counter"},
    "multiline": {"disabled-first-line", "disabled-second-line", "disabled-description"},
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def read(path):
    return json.loads(path.read_text())


def cropped(image, rect):
    x, y, width, height = rect
    box = tuple(floor(value * 2 + 0.5) for value in (x, y, x + width, y + height))
    left, top, right, bottom = box
    require(0 <= left < right <= image.width and 0 <= top < bottom <= image.height,
            f"crop lies outside the captured Canvas: {box}")
    return image.crop(box)


def load_paint(path, frame):
    with Image.open(path) as png:
        require(png.format == "PNG" and list(png.size) == frame["size"], f"invalid Canvas PNG {path.name}")
        return png.convert("RGB")


def near(pixel, ink):
    return max(abs(a - b) for a, b in zip(pixel, ink)) <= 2


def verify_captures(artifacts):
    report_path = artifacts / f"{PREFIX}.json"
    report = read(report_path)
    require(report.get("dpr") == 2 and not report.get("failure") and report.get("page_errors") == [],
            "field case did not finish with the expected scale and error state")
    expected = {f"{PREFIX}-{scene}-{phase}.png" for scene in SCENES for phase in PHASES}
    require(expected == {path.name for path in artifacts.glob(f"{PREFIX}*.png")},
            "capture inventory differs")
    captures, samples = {}, {}
    for scene in SCENES:
        journey = report[scene]
        frames = journey["frames"]
        require([frame["phase"] for frame in frames] == list(PHASES), f"wrong phase order for {scene}")
        required_parts = {"disabled-floating-label", "read-only-floating-label", "read-only-value"}
        if scene != "multiline":
            required_parts.add("disabled-disabled-value")
        required_parts.update(SLOTS.get(scene, ()))
        geometry = journey["geometry"]["ink_rects"]
        require(set(geometry) == required_parts, f"incomplete region inventory for {scene}")
        initial, initial_draft = None, None
        samples[scene] = {}
        for frame in frames:
            phase = frame["phase"]
            filename = f"{PREFIX}-{scene}-{phase}.png"
            require(frame["png"] == filename, "report points at an unrelated screenshot")
            path = artifacts / filename
            image = load_paint(path, frame)
            captures[filename] = sha(path)
            paint = frame["paint"]
            gray = paint["gray"]
            if phase == "forced":
                css = frame["css"]
                require(css["disabled_text"]["rgba"] == [*gray, 255]
                        and frame["system_colors"]["disabled_text"] == [*gray, 255]
                        and css["pairs"]["canvas"]["foreground"] == [*paint["ordinary"], 255]
                        and css["pairs"]["canvas"]["background"] == [*paint["paper"], 255]
                        and gray not in (paint["paper"], paint["ordinary"]), "CSS does not witness both ink roles")
            else:
                require(gray is None and frame["css"] is None and frame["system_colors"] is None,
                        "forced palette remained after restore")
            locks = frame["ax"]
            require(locks["disabled"]["state"] == "disabled" and locks["read_only"]["state"] == "read-only"
                    and locks["multiline"] == (scene == "multiline") and locks == frames[0]["ax"],
                    f"semantic lock state changed at {scene}/{phase}")
            for area in ("face", "boundary"):
                x, y = paint[area]["point"]
                require(list(image.getpixel((floor(2 * x), floor(2 * y)))) == paint[area]["rgb"],
                        f"geometry witness changed at {scene}/{phase}/{area}")
            require({glyph["part"] for glyph in paint["glyphs"]} == required_parts
                    and len(paint["glyphs"]) == len(required_parts), "duplicate or missing ink check")
            for glyph in paint["glyphs"]:
                part = glyph["part"]
                require(geometry[part] == glyph["bounds"], "glyph region is not linked to measured geometry")
                crop = cropped(image, glyph["bounds"])
                expected_ink = [tuple(rgb) for rgb in glyph["expected_rgb"]]
                if phase == "forced":
                    rgb = tuple(gray if part.startswith("disabled-") else paint["ordinary"])
                    require(glyph["exact_opaque"] and expected_ink == [rgb, rgb], "wrong exact host ink assertion")
                if glyph["exact_opaque"]:
                    count = sum(pixel == expected_ink[0] for pixel in crop.getdata())
                else:
                    count = sum(any(near(pixel, ink) for ink in expected_ink) for pixel in crop.getdata())
                require(count == glyph["matching_pixels"] and count >= glyph["minimum"] > 0,
                        f"pixel readback changed at {scene}/{phase}/{part}: {count}")
                if phase == "restored":
                    require(crop.tobytes() == cropped(initial, glyph["bounds"]).tobytes(),
                            f"normal colors did not restore exactly at {scene}/{part}")
            draft = frame["focused_draft"]
            if scene == "filled":
                require(draft["active"] and draft["value"] == draft["dom"] == draft["caller"] == "Surface draft"
                        and draft["web_selection"] == [11, 13, "backward"]
                        and draft["runtime_selection"] == [13, 11] and not draft["composing"],
                        "the active editor lost its draft or directed selection")
                if initial_draft is None:
                    initial_draft = draft
                require(draft == initial_draft, "host contrast changed the active editor")
            else:
                require(draft is None, "unexpected focused draft receipt")
            if phase == "normal":
                initial = image
            samples[scene][phase] = sum(glyph["matching_pixels"] for glyph in paint["glyphs"])
    return {"browser": report["browser"], "adapter": report["adapter"], "png_sha256": captures,
            "ink_pixels": samples, "report_sha256": sha(report_path)}


def verify(run):
    log = run / "runner.log"
    raw_log = log.read_text()
    require(re.findall(r"^  \[(M\d+)\]", raw_log, re.M) == STEPS, "wrong scoped step list")
    result = re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", raw_log, re.M)
    require(len(result) == 1 and int(result[0]) > 0, "scoped browser process did not finish")
    observer = read(run / "headless-window-observation.json")
    require(observer["stock_exit"] == 0 and not any(observer[key] for key in (
        "problems", "remaining_observed_processes", "visible_owned_windows")),
        "window observer reports a failure")
    events = observer["guard_events"]
    require(len(events) == 1 and events[0]["role"] == "chrome-primary" and events[0]["allowed"]
            and events[0]["argv"].count("--headless=new") == 1
            and observer["launch_observations"]["chrome-primary"]["samples"] > 0,
            "no positive headless Chrome observation")
    require(not read(run / "owned-shutdown.json")["after_cleanup"], "owned process survived")
    invocation = read(run / "invocation.json")
    require(invocation["inputs"] == read(run / "inputs-after.json"), "input bytes changed while Chrome ran")
    proof = verify_captures(run / "artifacts")
    return {**proof, "checks": int(result[0]), "steps": STEPS,
            "visible_owned_windows": 0,
            "chrome_samples": observer["launch_observations"]["chrome-primary"]["samples"],
            "log_sha256": sha(log), "input_receipt_sha256": sha(run / "inputs-after.json")}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.run), indent=2, sort_keys=True))
