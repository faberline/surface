#!/usr/bin/env python3
"""Verify the original Baseline Segmented Button Canvas captures."""

import argparse
from hashlib import sha256
import json
from math import floor, isfinite
from pathlib import Path
import re

from PIL import Image

from verify_forms_pixels import checksum, require

STEM = "material-segmented"
REPORT = f"{STEM}.json"
WIDE, SELECTED, PRESSED, COMPACT, SCROLLED = (
    f"{STEM}-{suffix}.png" for suffix in ("wide", "selection", "pressed", "rtl-200", "scroll-rtl")
)
IMAGES = {WIDE: (1920, 1848), SELECTED: (1920, 1848), PRESSED: (1920, 1848),
          COMPACT: (640, 1848), SCROLLED: (480, 1848)}
STEPS = ("M320", "M321", "M322", "M323")
GROUPS = {
    "comparison": ("radio", (("left-pane", "左側"), ("right-pane", "右側"))),
    "view": ("radio", (("list-view", "清單檢視"), ("table-view", "表格檢視"), ("grid-view", "格狀檢視"))),
    "filters": ("checkbox", (("new", "最新"), ("saved", "已儲存"), ("local", "離線"),
                             ("shared", "已分享"), ("archived", "封存"))),
}
CANVAS_Y = 44


def close(actual, expected):
    return abs(actual - expected) <= .06


def rgb(model, role):
    ink = model["palette"][role]
    require(len(ink) == 4 and all(type(channel) is int and 0 <= channel <= 255 for channel in ink) and ink[3] == 255,
            f"Invalid opaque {role} color")
    return tuple(ink[:3])


def geometry(rects):
    hit, face, focus = (rects[key] for key in ("hit", "face", "focus"))
    require(all(len(rect) == 4 and all(isfinite(n) for n in rect) for rect in (hit, face, focus)),
            "Invalid segment rectangles")
    require(hit[2] >= 48 and hit[3] >= 48 and close(hit[0], face[0]) and close(hit[2], face[2])
            and close(hit[1] + 4, face[1]) and close(hit[3], face[3] + 8)
            and all(close(a - b, offset) for a, b, offset in zip(focus, face, (-5, -5, 10, 10), strict=True)),
            "Segment hits, 4 dp slack or 5 dp focus outset differ")
    return hit, face, focus


def sample(image, point):
    x, y = point
    pixel = floor(2 * x), floor(2 * (y + CANVAS_Y))
    require(all(0 <= n < limit for n, limit in zip(pixel, image.size, strict=True)), "Segment pixel outside original PNG")
    return image.getpixel(pixel)


def count(image, rect, color):
    x, y, w, h = rect
    left, right = floor(2 * x + .5), floor(2 * (x + w) + .5)
    top, bottom = floor(2 * (y + CANVAS_Y) + .5), floor(2 * (y + h + CANVAS_Y) + .5)
    require(0 <= left < right <= image.width and 0 <= top < bottom <= image.height, "Segment paint probe outside original PNG")
    region = image.crop((left, top, right, bottom))
    return sum(number for number, value in region.getcolors(region.width * region.height) if value == color)


def decode(artifacts):
    require({path.name for path in artifacts.glob(f"{STEM}-*.png")} == set(IMAGES),
            "Missing, extra or failed Segmented Button capture")
    captures, pixels = {}, {}
    for name, size in IMAGES.items():
        path = artifacts / name
        with Image.open(path) as image:
            require(image.format == "PNG" and image.mode in ("RGB", "RGBA") and image.size == size,
                    f"Wrong original Segmented Button PNG size or format: {name}")
            image.load()
            pixels[name] = image.convert("RGB")
        captures[name] = {"sha256": checksum(path), "size": list(size),
                          "decoded_rgb_sha256": sha256(pixels[name].tobytes()).hexdigest()}
    require(len({item["decoded_rgb_sha256"] for item in captures.values()}) == len(IMAGES),
            "Reused Segmented Button Canvas captures")
    return captures, pixels


def roles(rows, expected):
    require(set(rows) == set(expected), "Wrong Segmented Button AX group inventory")
    for group, active in expected.items():
        role, labels = GROUPS[group]
        want = [{"id": f"segment-{key}", "name": label, "role": role,
                 "checked": key in active, "disabled": key in ("table-view", "local")}
                for key, label in labels]
        require(rows[group] == want, f"Wrong named Chrome AX choices or state for {group}")


def model(row, activations, filters, *, view="grid-view"):
    require(row["comparison"] == "segment-right-pane" and row["view"] == f"segment-{view}"
            and row["activations"] == activations and row["filters"] == [f"segment-{key}" for key in filters],
            "Wrong Segmented Button selection or exact activation count")


def wide_evidence(section, image):
    initial = section["initial"]
    model(initial, 0, ())
    roles(section["roles"], {"comparison": {"right-pane"}, "view": {"grid-view"}, "filters": set()})
    require(not initial["rtl"] and not initial["dark"] and initial["text_scale"] == 1, "Wrong wide theme")
    require(sample(image, (4, 4)) == rgb(initial, "background"), "Wide palette does not match the original PNG")
    rows = {}
    require([row["id"] for row in section["rows"]] == [f"segments-{label}" for label in GROUPS],
            "Wrong Segmented Button geometry rows")
    for row in section["rows"]:
        group = row["id"].removeprefix("segments-")
        require([part["id"] for part in row["segments"]] == [f"segment-{key}" for key, _ in GROUPS[group][1]],
                f"Wrong measured row order: {group}")
        measured = [geometry(part["rects"]) for part in row["segments"]]
        lead = measured[0][0]
        require(all(close(hit[2], lead[2]) and close(face[3], 40) and close(hit[0], lead[0] + i * lead[2])
                    for i, (hit, face, _) in enumerate(measured)), f"Unequal face or split in row: {group}")
        rows[group] = {"slots": len(measured), "width": lead[2]}
    outlines = section["seams"]
    require(len(outlines) == 7 and all(close(row["rect"][2], 1) and close(row["rect"][3], 38)
                                      for row in outlines), "Wrong 1 dp seam inside a 40 dp face")
    require(all(sample(image, row["point"]) == rgb(initial, "outline") == tuple(row["ink"]) for row in outlines),
            "Original PNG seam color differs")
    require(sample(image, section["selected_point"]) == rgb(initial, "selection"),
            "Missing checked icon-only face color")
    check, icon = section["check"], section["icon"]
    require(check[0] + check[2] < icon[0], "Checked glyph crowds its artwork")
    glyphs = [count(image, bounds, rgb(initial, "selection_ink")) for bounds in (check, icon)]
    require(all(pixels >= 8 for pixels in glyphs), "Missing checked symbol or icon-only glyph pixels")
    return {"rows": rows, "painted_seams": len(outlines), "check_and_artwork_pixels": glyphs}


def verify(artifacts, metadata=None, *, log=None, calibration=False):
    path = artifacts / REPORT
    report = json.loads(path.read_text())
    schema = {"browser", "wide", "keyboard", "pointer", "compact", "page_errors"}
    require(set(report) in (schema, schema | {"adapter"}) and report["page_errors"] == [],
            "Failed, incomplete or altered Segmented Button report")
    require(re.fullmatch(r"Chrome/[\d.]+", report["browser"]), "Wrong Segmented Button Chrome metadata")
    if metadata is not None:
        require(report["browser"] == metadata["browser"] and ("adapter" not in report or report["adapter"] == metadata["adapter"]),
                "Segmented Button Chrome or WebGPU adapter differs")
    if log is not None:
        require(tuple(re.findall(r"^  \[(M32[0-3])\]", log.read_text(), re.M)) == STEPS,
                "Wrong ordered Segmented Button checkpoints")
    captures, pixels = decode(artifacts)
    wide, keyboard, pointer, compact = (report[key] for key in ("wide", "keyboard", "pointer", "compact"))
    require([wide["capture"], keyboard["capture"], pointer["capture"], *compact["captures"]] == list(IMAGES),
            "Wrong original Segmented Button filenames")
    wide_result = wide_evidence(wide, pixels[WIDE])
    model(keyboard["selected"], 3, ("new", "shared"), view="list-view")
    model(keyboard["cleared"], 5, (), view="list-view")
    roles(keyboard["roles"], {"view": {"list-view"}, "filters": {"new", "shared"}})
    key_focus = count(pixels[SELECTED], keyboard["focus"], rgb(keyboard["selected"], "focus"))
    require(key_focus == keyboard["focus_pixels"] > 40, "Original keyboard focus ink differs")
    model(pointer["hold"], 0, ())
    model(pointer["final"], 1, ("saved",))
    saved = next(row for row in wide["rows"][2]["segments"] if row["id"] == "segment-saved")
    hit, face, _ = geometry(saved["rects"])
    x, y = pointer["point"]
    require(hit[0] < x < hit[0] + hit[2] and hit[1] <= y < face[1], "Trusted press was outside the touch slack")
    probe = (x, face[1] + 5)
    held, rest = sample(pixels[PRESSED], probe), sample(pixels[WIDE], probe)
    require(held != rest, "Original held PNG has no visible face feedback")

    initial, final = compact["initial"], compact["final"]
    model(initial, 0, ())
    model(final, 1, ("archived",))
    require(initial["width"] == 320 and final["width"] == 240 and initial["rtl"] is final["rtl"] is True
            and initial["dark"] is final["dark"] is True and initial["text_scale"] == final["text_scale"] == 2,
            "Wrong dark RTL text scale or viewport")
    first, first_face, first_focus = geometry(compact["first"])
    last, _, _ = geometry(compact["last"])
    require(first[0] > last[0] and close(first[1], last[1]) and first_face[3] > 40,
            "Dark compact row did not mirror and scale")
    archive, _, archive_focus = geometry(compact["archive"])
    viewport = compact["viewport"]
    begin, end = compact["track_before"], compact["track_after"]
    require(begin[2] > viewport[2] and close(begin[2], end[2]) and end[0] > begin[0]
            and viewport[0] <= archive[0] < archive[0] + archive[2] <= viewport[0] + viewport[2],
            "RTL reveal shrank or clipped the original hit")
    restored = compact.get("returned_focus")
    require(restored == "segmented-demo-show" or (calibration and restored is None),
            "Segmented Button Back focus restoration proof missing")
    focus = [count(pixels[COMPACT], first_focus, rgb(initial, "focus")),
             count(pixels[SCROLLED], archive_focus, rgb(final, "focus"))]
    require(focus == [compact["compact_pixels"], compact["focus_pixels"]] and min(focus) > 40,
            "Original dark RTL focus pixels differ")
    return {"scope": "focused development calibration" if calibration else "stock-feature",
            "report_sha256": checksum(path), "steps": list(STEPS), "browser": report["browser"],
            "page_errors": 0, "captures": captures, "wide": wide_result, "keyboard_focus_pixels": key_focus,
            "touch": {"target": "segment-saved", "point": [x, y], "held_rgb": list(held), "rest_rgb": list(rest)},
            "rtl": {"focus_pixels": focus, "track_delta": end[0] - begin[0], "returned_focus": restored}}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifacts", type=Path)
    parser.add_argument("--runner-log", type=Path)
    parser.add_argument("--calibration", action="store_true")
    args = parser.parse_args()
    print(json.dumps(verify(args.artifacts, log=args.runner_log, calibration=args.calibration),
                     indent=2, ensure_ascii=False))
