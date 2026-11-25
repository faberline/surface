#!/usr/bin/env python3
"""Exercise the sealed verifier on synthetic proof, without launching a Gallery."""

from copy import deepcopy
import json
from pathlib import Path
from runpy import run_path
import subprocess
from tempfile import TemporaryDirectory
from types import SimpleNamespace

from PIL import Image, ImageDraw

from gallery_transport import require


HERE = Path(__file__).resolve().parent
RUN = run_path(str(HERE / "replay-adaptive-nav.py"))
SOURCE = "1" * 64
BINARIES = "2" * 64


def rect(x, y, width, height):
    return dict(x=x, y=y, width=width, height=height)


def model(profile, target, x, y, cell_width):
    left = x + (cell_width - 56) / 2
    if profile.mode == "horizontal":
        hit = rect(x, y, cell_width, 64)
        pill = rect(x + 10, y + 12, 92, 40)
        icon = rect(pill["x"] + 16, pill["y"] + 8, 24, 24)
        label = rect(pill["x"] + 44, pill["y"] + 12, 32, 16)
    else:
        label_height = 32 if profile.rail else 16
        hit = rect(x, y, cell_width, 68 if profile.rail else 64)
        pill = rect(left, y + 6, 56, 32)
        icon = rect(left + 16, y + 10, 24, 24)
        label = rect(left + 12, y + 42, 32, label_height)
        if profile.rail:
            hit["height"] = 80
    text = RUN["LABELS"][RUN["ITEMS"].index(target)]
    return dict(hit=hit, pill=pill, icon=icon, label=label, text=text)


def shape(draw, item, fill, *, rounded=False):
    x, y, width, height = (int(item[key]) for key in ("x", "y", "width", "height"))
    box = (x, y, x + width - 1, y + height - 1)
    if rounded:
        draw.rounded_rectangle(box, radius=height // 2, fill=tuple(fill))
    else:
        draw.rectangle(box, fill=tuple(fill))


def fixture(profile, path):
    rail = profile.rail
    viewport = rect(0, 0, profile.width, profile.height)
    if rail:
        nav = rect(profile.width - 96, 0, 96, profile.height)
        page = rect(0, 0, profile.width - 96, profile.height)
        selected = model(profile, RUN["SAVED"], nav["x"], 55, 96)
        focused = model(profile, RUN["SETTINGS"], nav["x"], 220, 96)
        reveal = {"before": {**focused["hit"], "y": 360}, "after": focused["hit"]}
    else:
        nav = rect(0, profile.height - 64, profile.width, 64)
        page = rect(0, 0, profile.width, profile.height - 64)
        width = 112 if profile.mode == "horizontal" else profile.width // 5
        selected = model(profile, RUN["SAVED"], (profile.width - width) // 2, nav["y"], width)
        focused = model(profile, RUN["UPDATES"], selected["hit"]["x"] + width, nav["y"], width)
        reveal = None
    measured = {"shell": viewport, "landmark": nav, "page": page, "content": page}
    if rail:
        measured["scroll"] = nav
    palette = {"surface": [41, 32, 41, 255] if rail else [241, 232, 241, 255],
               "selected": [135, 100, 195, 255], "icon": [24, 12, 38, 255],
               "label": [156, 200, 22, 255], "focus": [101, 151, 51, 255]}
    image = Image.new("RGBA", (profile.width, profile.height), (57, 25, 90, 255))
    draw = ImageDraw.Draw(image)
    shape(draw, nav, palette["surface"])
    shape(draw, selected["pill"], palette["selected"], rounded=True)
    shape(draw, selected["icon"], palette["icon"])
    shape(draw, selected["label"], palette["label"])
    focus_rect = focused["pill" if rail else "hit"]
    x = int(focus_rect["x"] + focus_rect["width"] / 2)
    y = int(focus_rect["y"])
    draw.rectangle((x - 5, y, x + 5, y + 1), fill=tuple(palette["focus"]))
    image.save(path)
    keys = [{"key": key, "focus": focus, "semantics": RUN["expected_semantics"](current)}
            for key, focus, current in RUN["expected_keys"](profile)]
    check = RUN["paint"](path, profile, nav, selected, focused, palette)
    editor = rect(16, 48, 80, 48)
    return {
        "mode": profile.mode, "layout": measured, "initial": RUN["expected_semantics"](RUN["HOME"]),
        "keys": keys, "selected": RUN["expected_semantics"](RUN["SAVED"]),
        "capture_focus": keys[-1]["focus"], "disabled_activation": "disabled", "return_focus": RUN["SHOW"],
        "selected_art": selected, "focused_art": focused, "rail_reveal": reveal,
        "page_editor": {"before": editor, "after": editor}, "palette": palette, "pixels": check,
        "image": {"name": path.name, "width": profile.width, "height": profile.height, "scale": 1,
                  "format": "png", "sha256": RUN["digest"](path)},
    }


def rejected(action, phrase):
    try:
        action()
    except (OSError, RuntimeError, ValueError) as error:
        require(phrase in str(error), f"Wrong synthetic rejection: {error}")
        return
    raise RuntimeError(f"Defective synthetic data passed: {phrase}")


def artwork_frame(profile):
    ident = RUN["SAVED"]
    art = model(profile, ident, 50, 30, 96 if profile.rail else 112)
    groups = [
        {"role": "Button", "stable_id": ident, "first_child": 1, "focus_color": [20, 40, 60, 255]},
        {"role": "Group", "stable_id": None if profile.rail else ident + "-indicator",
         "first_child": 3, "next_sibling": 2, "focus_color": [20, 40, 60, 255],
         "style": {"background": [80, 100, 120, 255]}},
        {"role": "Text", "stable_id": None if profile.rail else ident + "-label",
         "text": art["text"], "style": {"foreground": [20, 40, 60, 255]}},
        {"role": "Group", "first_child": None if profile.mode == "horizontal" else 4},
    ]
    contour = {"color": [140, 160, 180, 255]}
    geometry = [art[key] for key in ("hit", "pill", "label", "icon")]
    if profile.mode == "horizontal":
        groups[3]["fill_contour"] = contour
    else:
        groups.append({"role": "Group", "first_child": None, "fill_contour": contour})
        geometry.append(deepcopy(art["icon"]))
    return {"dom": {"nodes": groups}, "layout": {"nodes": [{"bounds": box, "clip": box} for box in geometry]}}


def artwork_controls(out):
    ident = RUN["SAVED"]
    for name in RUN["PROFILES"]:
        profile = RUN["PROFILES"][name]
        _, palette = RUN["parts"](artwork_frame(profile), ident, profile)
        require(palette["icon"] == [140, 160, 180, 255], "Contour leaf color differs")
    compact = RUN["PROFILES"]["compact"]
    missing = artwork_frame(compact)
    del missing["dom"]["nodes"][4]["fill_contour"]
    rejected(lambda: RUN["parts"](missing, ident, compact), "expected passive contour leaf is missing")
    decoy = artwork_frame(compact)
    decoy["dom"]["nodes"][3]["fill_contour"] = {"color": [8, 9, 10, 255]}
    rejected(lambda: RUN["parts"](decoy, ident, compact), "expected the clipped vector wrapper")
    offset = artwork_frame(compact)
    offset["layout"]["nodes"][4]["bounds"]["x"] += 2
    rejected(lambda: RUN["parts"](offset, ident, compact), "contour does not occupy its expected clip")
    labeled = artwork_frame(compact)
    labeled["dom"]["nodes"][4]["name"] = "Inappropriately named art"
    rejected(lambda: RUN["parts"](labeled, ident, compact), "expected passive contour leaf is missing")
    frame = artwork_frame(compact)

    def snapshot(method, params):
        require((method, params) == ("DOM.snapshot", {}), "Diagnostics may only read the snapshot")
        return frame

    def capture_failure():
        app = SimpleNamespace(proc=SimpleNamespace(pid=123), wire=snapshot)
        with RUN["diagnose_failure"](out, "compact", app, {"name": "selected-artwork"}, {"opened": 0}):
            raise RuntimeError("synthetic layout mismatch")

    rejected(capture_failure, "synthetic layout mismatch")
    diagnostic = json.loads((out / "compact-failure.json").read_text())
    require(diagnostic["snapshot"] == frame and diagnostic["stage"] == "selected-artwork"
            and diagnostic["window_samples_before_failure"] == {"opened": 0}, "Missing original snapshot diagnostics")
    return {"artwork_profiles": len(RUN["PROFILES"]), "malformed_artwork_rejected": 4, "diagnostic_snapshot": True}


def main():
    with TemporaryDirectory(prefix="offline-q74-", dir=HERE) as work:
        out = Path(work).resolve()
        diagnostics = out / "diagnostics"
        diagnostics.mkdir()
        diagnostic_checks = artwork_controls(diagnostics)
        fixture_dir = out / "fixture"
        fixture_dir.mkdir()
        out = fixture_dir
        profile = RUN["PROFILES"]["rail"]
        guard = RUN["guard_text"](Path("/usr/bin/true"), profile.width, profile.height)
        correct = RUN["console_args"](profile.width, profile.height)
        for args in (correct, correct[1:], [*correct, "--extra"], ["--windowed", *correct[1:]],
                     [*correct[:-1], "900"]):
            result = subprocess.run(["/bin/sh", "-c", guard, "test-guard", *args], check=False)
            require(result.returncode == (0 if args == correct else 71), "Headless pre-exec wrapper differs")
        trials = {"1": {}, "2": {}}
        processes = []
        for index in trials:
            destination = out / f"repeat-{index}"
            destination.mkdir()
            for name, profile in RUN["PROFILES"].items():
                trials[index][name] = fixture(profile, destination / f"{profile.image}.png")
                processes.append({"profile": name, "replay": index, "pid": 500 + len(processes),
                                  "reaped": True, "argv": RUN["console_args"](profile.width, profile.height),
                                  "visible_windows": dict.fromkeys(RUN["SAMPLES"], 0)})
        binary = {name: f"{i}" * 64 for i, name in enumerate(RUN["BINARIES"], 3)}
        report = {"schema": RUN["SCHEMA"], "transports": RUN["TRANSPORTS"],
                  "source": {"manifest_sha256": SOURCE, "version": "0.1.74", "file_count": 696},
                  "binaries": {"manifest_sha256": BINARIES, "files": binary}, "replays": trials, "processes": processes}
        manifest = out / "SHA256SUMS"

        def read(value, source=SOURCE, binaries=BINARIES):
            (out / RUN["REPORT"]).write_text(json.dumps(value))
            paths = sorted(path for path in out.rglob("*") if path.is_file() and path != manifest)
            manifest.write_text("".join(f"{RUN['digest'](path)}  {path.relative_to(out).as_posix()}\n" for path in paths))
            return RUN["readback"](out, RUN["digest"](manifest), source, binaries)

        positive = read(report)
        require((positive["original_pngs"], positive["replays"], positive["window_samples"], positive["reaped"])
                == (6, 2, 30, 6) and positive["native_resize_covered"] is False, "Synthetic positive did not verify")
        rejected(lambda: read(report, source="0" * 64), "source or executable pins differ")
        rejected(lambda: read(report, binaries="0" * 64), "source or executable pins differ")
        changed = deepcopy(report)
        changed["replays"]["2"]["medium"]["capture_focus"] = RUN["SAVED"]
        rejected(lambda: read(changed), "Repeated geometry, keys, pixels or PNG digests differ")
        rejected_count = 3

        def change_case(name, apply, phrase):
            nonlocal rejected_count
            value = deepcopy(report)
            for repeat in value["replays"].values():
                apply(repeat[name])
            rejected(lambda: read(value), phrase)
            rejected_count += 1

        change_case("compact", lambda x: x["layout"]["landmark"].update(width=590), "Bottom bar")
        change_case("medium", lambda x: x["selected_art"]["pill"].update(height=38), "medium variant")
        change_case("rail", lambda x: x["layout"]["landmark"].update(x=0), "RTL leading rail")
        change_case("rail", lambda x: x["rail_reveal"]["before"].update(y=220), "did not reveal")
        change_case("rail", lambda x: x["keys"][0].update(focus=RUN["EXPLORE"]), "disabled skip")
        change_case("rail", lambda x: x["keys"][0]["semantics"].update(marked_current_pages=[RUN["SAVED"]]),
                    "current page")
        change_case("compact", lambda x: x["selected"].update(landmarks=[]), "named navigation landmark")
        change_case("compact", lambda x: x.update(capture_focus=RUN["SAVED"]), "Focus, semantic activation")
        change_case("medium", lambda x: x["page_editor"].update(after=rect(20, 48, 80, 48)), "independent editor")
        change_case("rail", lambda x: x["pixels"].update(focus_pixels=0), "paint receipt differ")
        for key, value in (("visible_windows", dict.fromkeys(RUN["SAMPLES"], 1)), ("reaped", False),
                           ("argv", ["--width", "840"])):
            changed = deepcopy(report)
            changed["processes"][-1][key] = value
            rejected(lambda: read(changed), "exact headless args or window samples differ")
            rejected_count += 1
        print(json.dumps({"scope": "synthetic verifier controls, no applications launched",
                          "positive": 2, "rejected": rejected_count + 4,
                          **diagnostic_checks}, indent=2))


if __name__ == "__main__":
    main()
