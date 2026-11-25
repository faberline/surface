#!/usr/bin/env python3
"""Recheck inherited v0.1.70 label color, geometry, notch, and Chrome AX."""

import argparse
from hashlib import sha256
import json
import math
from pathlib import Path
from runpy import run_path
import re

from PIL import Image

PREFIX = "material-text-field-label-motion"
TITLE = "專案名稱"
HELP = "請輸入中文或英文名稱。"
LONG_TITLE = "輸入團隊成員在共用專案清單中容易辨認的完整名稱"
DPR = 2
HEADER = 44
ROOT = Path("/tmp/surface-md3-field-placeholder-focused-v0169-r2.8AArzw7l")
ORACLE = run_path(str(ROOT / "pixel-oracle.py"))
PREP = run_path(str(ROOT / "prepare-focused.py"))
OBS = run_path(str(ROOT / "observe-tabs.py"))
UPDATER_ARG = "--disable-updater-scheduler"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    return sha256(path.read_bytes()).hexdigest()


def color(palette, token):
    rgba = palette[token]
    require(len(rgba) == 4 and rgba[3] == 255 and all(type(n) is int and 0 <= n <= 255 for n in rgba),
            f"{token}: invalid opaque theme color")
    return tuple(rgba[:3])


def image_for(path, compact):
    require(path.is_file() and not path.is_symlink(), f"{path.name}: no original")
    expected = (640, 2008) if compact else (1920, 1728)
    with Image.open(path) as original:
        require(original.format == "PNG" and original.size == expected,
                f"{path.name}: expected original PNG {expected}")
        original.verify()
    with Image.open(path) as original:
        return original.convert("RGB")


def px(image, x, y):
    return image.getpixel((math.floor(x * DPR), math.floor((y + HEADER) * DPR)))


def region(image, rect, ink):
    x, y, width, height = rect
    # Coordinates use positive logical Canvas positions at DPR 2.
    def rounded(n):
        return math.floor(n * DPR + .5)

    box = tuple(map(rounded, (x, y + HEADER, x + width, y + HEADER + height)))
    require(0 <= box[0] < box[2] <= image.width and 0 <= box[1] < box[3] <= image.height,
            "paint crop out of the original PNG")
    crop = image.crop(box)
    counts = crop.getcolors(crop.width * crop.height)
    return next((count for count, pixel in counts if pixel == ink), 0)


def check_contains(rect, compact):
    x, y, width, height = rect
    viewport = (320, 960) if compact else (960, 820)
    require(width > 0 and height > 0 and 0 <= x < x + width <= viewport[0]
            and 0 <= y < y + height <= viewport[1], f"outside Canvas: {rect}")


def clearance(scene, appearance):
    x, inline_top, _, height = scene["inline"]
    _, float_top, span, float_height = scene["floated"]
    return [x, inline_top if appearance == "outlined" else float_top + float_height,
            span * 1.5 + 24, height]


def verify_frame(artifacts, scene, frame, suffix, ms, appearance, focused, ink, neutral_reference=None):
    require(appearance in ("outlined", "filled", "compact"), "wrong appearance")
    compact = appearance == "compact"
    filename = f"{PREFIX}-{suffix}.png"
    require(frame["png"] == filename and frame["ms"] == ms, f"{suffix}: incorrect timestamp or capture")
    path = artifacts / filename
    image = image_for(path, compact)
    palette = scene["palette"]
    require(frame["label_rgb"] == list(ink), f"{suffix}: golden label RGB differs")
    label = frame["label"]
    check_contains(label, compact)
    require(frame["label_pixels"] > 12 and region(image, label, ink) >= frame["label_pixels"],
            f"{suffix}: reported label missing in the original PNG")
    if appearance in ("outlined", "compact"):
        paper = color(palette, "background")
        neutral = color(palette, "outline")
        primary = color(palette, "primary")
        if suffix.endswith("-rest") or suffix == "outlined-first":
            border = neutral
        elif suffix in ("outlined-falling", "outlined-retarget"):
            border = blend(primary, neutral, 44)
        elif suffix in ("outlined-early", "outlined-middle", "rtl-middle"):
            border = blend(neutral, primary, ms)
        else:
            require(suffix in ("outlined-settled", "outlined-reduced"), f"unknown outline phase {suffix}")
            border = primary
        require(frame.get("outline_rgb") == list(border),
                f"{suffix}: missing or incorrect independent timed outline ink")
        x, y, width, height = scene["face"]
        left, _, span, _ = scene["floated"]
        anchor = scene.get("anchor", left)
        x_padding = anchor + (2 if compact else -2)
        actual_notch = [px(image, x_padding, y + .25), px(image, x_padding, y + 2.5)]
        actual_sides = [
            px(image, x + .5, y + height / 2),
            px(image, x + width - .5, y + height / 2),
            px(image, x + width / 2, y + height - .5),
        ]
        columns = range(math.ceil((left - 4) * DPR), math.floor((left + span + 4) * DPR))
        gap = sum(px(image, (column + .5) / DPR, y + .25) == paper for column in columns)
        require(actual_notch == list(map(tuple, frame["notch"]))
                and actual_sides == list(map(tuple, frame["outline_sides"])) == [border] * 3
                and gap == frame["gap_pixel_columns"], f"{suffix}: original notch/sides differ")
    if appearance != "compact" and focused:
        clip = clearance(scene, appearance)
        idle = color(palette, "on_surface_variant")
        if neutral_reference:
            before = image_for(artifacts / neutral_reference, False)
            x, y, w, h = clip
            for row in range(math.ceil(y * DPR), math.floor((y + h) * DPR)):
                for column in range(math.ceil(x * DPR), math.floor((x + w) * DPR)):
                    point = ((column + .5) / DPR, (row + .5) / DPR)
                    require((px(before, *point) == idle) == (px(image, *point) == idle),
                            f"{suffix}: the first-focus neutral mask contains new or missing ink")
            placeholder = 0
        else:
            placeholder = region(image, clip, idle)
        require(placeholder == frame["placeholder_pixels"],
                f"{suffix}: placeholder pixels differ from the original capture")
    return sha(path)


def blend(start, target, ms):
    return ORACLE["from_lab"](ORACLE["sample"](ORACLE["pose"](start), target, ms)[0])


def ink_triplet(scene):
    return tuple(color(scene["palette"], key) for key in ("on_surface_variant", "primary", "error"))


def spatial_frames(artifacts, report):
    outlined, reversal, filled, compact = (report[key] for key in ("outlined", "reversal", "filled", "compact"))
    for appearance, scene in (("outlined", outlined), ("filled", filled), ("compact", compact)):
        check_contains(scene["face"], appearance == "compact")
        check_contains(scene["floated"], appearance == "compact")
    o_idle, o_focus, _ = ink_triplet(outlined)
    f_idle, f_focus, _ = ink_triplet(filled)
    c_idle, c_focus, _ = ink_triplet(compact)
    turn = blend(o_focus, o_idle, 44)
    captures = {}
    groups = [
        (outlined, outlined["frames"], "outlined", ["rest", "first", "early", "middle", "settled"],
         [0, 0, 14, 44, 800], [False, True, True, True, True],
         [o_idle, o_idle, blend(o_idle, o_focus, 14), blend(o_idle, o_focus, 44), o_focus]),
        (outlined, reversal["frames"], "outlined", ["falling", "retarget", "reduced"],
         [844, 844, 844], [False, True, True], [turn, turn, o_focus]),
        (filled, [filled["rest"], *filled["frames"]], "filled", ["rest", "first", "middle", "settled"],
         [0, 0, 44, 800], [False, True, True, True], [f_idle, f_idle, blend(f_idle, f_focus, 44), f_focus]),
        (compact, compact["frames"], "compact", ["rest", "middle"], [0, 44],
         [False, True], [c_idle, blend(c_idle, c_focus, 44)]),
    ]
    for scene, frames, appearance, suffixes, times, focused, inks in groups:
        prefix = "rtl" if appearance == "compact" else appearance
        for frame, suffix, ms, active, ink in zip(frames, suffixes, times, focused, inks, strict=True):
            reference = f"{PREFIX}-{prefix}-rest.png" if suffix == "first" else None
            captures[frame["png"]] = verify_frame(artifacts, scene, frame, f"{prefix}-{suffix}", ms,
                                                   appearance, active, ink, reference)

    rest, first, early, middle, end = outlined["frames"]
    falling, retarget, reduced = reversal["frames"]
    paper = outlined["palette"]["background"][:3]
    require(rest["label"] == first["label"] and rest["gap_pixel_columns"] == first["gap_pixel_columns"] == 0
            and first["placeholder_pixels"] == early["placeholder_pixels"] == 0
            and early["notch"] == middle["notch"] == end["notch"] == [paper, paper]
            and 8 <= early["gap_pixel_columns"] < middle["gap_pixel_columns"]
            and first["label"][1] > early["label"][1] > middle["label"][1] > end["label"][1],
            "the Outlined notch or label trajectory changed")
    early_image = image_for(artifacts / early["png"], False)
    face_x, face_y, face_width, _ = outlined["face"]
    gap_x = outlined["floated"][0] - 2
    control_x = face_x + face_width * .75
    depths = [.25, .75, 1.25]
    cut_points = [[gap_x, face_y + depth] for depth in depths]
    uncut_points = [[control_x, face_y + depth] for depth in depths]
    cut_rgb, uncut_rgb = ([list(px(early_image, *point)) for point in points]
                          for points in (cut_points, uncut_points))
    require(outlined.get("early_shallow") == {
                "cut_points": cut_points, "uncut_points": uncut_points,
                "cut_rgb": cut_rgb, "uncut_rgb": uncut_rgb,
            } and cut_rgb[0] == paper and uncut_rgb[0] == early["outline_rgb"]
            and any(pixel != paper for pixel in cut_rgb[1:]) and uncut_rgb[2] != paper,
            "the original early notch must clear the top and retain stroke within the thinner stripe")
    require(falling["ms"] == retarget["ms"] == reduced["ms"]
            and falling["label"] == retarget["label"] and reduced["label"] == end["label"]
            and reduced["notch"] == end["notch"] and end["placeholder_pixels"] > 12,
            "reversal or reduced endpoint changed")
    trusted = reversal["trusted"]
    require([key["key"] for key in trusted] == ["Tab"] * 3
            and [key["shift"] for key in trusted] == [False, True, False]
            and trusted[1]["id"] == "surface-text-input", "trusted focus route changed")
    for actual, active in ((outlined["proxy"], False), (outlined["active"], True),
                           (reversal["blurred"], False), (filled["ax"], True)):
        require(actual["label"] == TITLE and actual["description"] == HELP and actual["focused"] is active,
                "textbox name, description or focus changed")
    require(outlined["proxy"]["backend"] != outlined["active"]["backend"],
            "AX projection did not move to the live textarea")

    fill_start, fill_mid, fill_end = filled["frames"]
    preedit = filled["preedit"]
    filename = f"{PREFIX}-filled-preedit.png"
    require(preedit["png"] == filename and preedit["ms"] == 44 and preedit["placeholder_pixels"] == 0
            and fill_start["label"][1] > fill_mid["label"][1] > fill_end["label"][1]
            and fill_start["placeholder_pixels"] == fill_mid["placeholder_pixels"] == 0
            and fill_end["placeholder_pixels"] > 12, "Filled progression or preedit changed")
    x, _, _, height = filled["inline"]
    _, top, width, floating_height = filled["floated"]
    clearance = [x, top + floating_height, width * 1.5 + 24, height]
    image = image_for(artifacts / filename, False)
    require(region(image, clearance, color(filled["palette"], "on_surface")) == preedit["ink_pixels"] > 12
            and region(image, clearance, color(filled["palette"], "on_surface_variant")) == 0,
            "preedit PNG needs actual marked glyphs and no overlapping placeholder")
    captures[filename] = sha(artifacts / filename)

    closed, moving = compact["frames"]
    x, _, width, _ = compact["face"]
    require(x + width - compact["anchor"] == 16 and closed["gap_pixel_columns"] == 0
            and moving["gap_pixel_columns"] > 12
            and moving["label"][1] < closed["label"][1] - 4
            and moving["label"][0] > closed["label"][0] + 3
            and abs(sum(moving["label"][::2]) - sum(closed["label"][::2])) <= 3,
            "the compact RTL label loses its physical right anchor")
    fallback = compact["fallback"]
    filename = f"{PREFIX}-rtl-above-label.png"
    image = image_for(artifacts / filename, True)
    check_contains(fallback["above"], True)
    check_contains(fallback["face"], True)
    ink = color(compact["palette"], "primary")
    require(fallback["png"] == filename and fallback["ms"] == 800 and fallback["float"][2:] == [0, 0]
            and fallback["above"][3] >= 64 and fallback["above"][1] + fallback["above"][3] + 3.5 <= fallback["face"][1]
            and region(image, fallback["above"], ink) == fallback["pixels"] > 40
            and px(image, *fallback["solid_top"]) == ink and fallback["ax"]["label"] == LONG_TITLE
            and fallback["ax"]["focused"] is True, "dark RTL above-label fallback changed")
    captures[filename] = sha(artifacts / filename)

    above = fallback["above"]
    color_frames = fallback["color_frames"]
    probes = fallback["probe_points"]
    require(len(probes) == 4 and above[3] >= 64, "fixed dark fallback lacks separated probe geometry")
    for phase, ms, sample, expect in zip(("rest", "middle"), (0, 44), color_frames,
                                        (c_idle, blend(c_idle, c_focus, 44)), strict=True):
        expected_name = f"{PREFIX}-rtl-above-label-{phase}.png"
        require(sample["png"] == expected_name and sample["ms"] == ms
                and sample["label_rgb"] == list(expect), "dark fallback golden timing or RGB differs")
        image = image_for(artifacts / expected_name, True)
        actual = [px(image, *probe) for probe in probes]
        require(actual == [expect] * 4 and region(image, above, expect) == sample["label_pixels"] > 40,
                "dark wrapped label lacks stable original-size glyph ink")
        if phase == "middle":
            require(sample["solid_probe_rgb"] == list(map(list, actual)), "dark fallback probe receipt changed")
        captures[expected_name] = sha(artifacts / expected_name)
    return captures


def verify_color_case(artifacts, appearance, block):
    idle, primary, error = ink_triplet(block)
    key = ORACLE["pose"](idle)
    at_focus = ORACLE["sample"](key, primary, 44)
    focus_mid = ORACLE["from_lab"](at_focus[0])
    toward_error = ORACLE["from_lab"](ORACLE["sample"](at_focus, error, 44)[0])
    fresh_start = blend(focus_mid, error, 44)
    require(toward_error != fresh_start, "retarget sample must discriminate resetting velocity")
    clearing = blend(error, primary, 44)
    samples = [
        ("rest", 1600, idle), ("focus-mid", 1644, focus_mid), ("error-first", 1644, focus_mid),
        ("error-mid", 1688, toward_error), ("error-settled", 2488, error),
        ("clearing", 2532, clearing), ("reversed", 2532, clearing), ("reduced", 2532, error),
    ]
    probes = block["probe_points"]
    check_contains(block["face"], False)
    check_contains(block["floated"], False)
    check_contains(block["helper"], False)
    require(len(probes) == 4 and all(len(point) == 2 for point in probes)
            and all(right[0] - left[0] >= 3 for left, right in zip(probes, probes[1:])),
            f"{appearance}: fixed glyph sample points not separate")
    captures = {}
    rest_box = block["frames"][0]["label"]
    for frame, (phase, time, ink) in zip(block["frames"], samples, strict=True):
        name = f"{PREFIX}-{appearance}-color-{phase}.png"
        require(frame["png"] == name and frame["ms"] == time and frame["label_rgb"] == list(ink),
                f"{appearance}/{phase}: wrong fixed motion state")
        bounds = frame["label"]
        check_contains(bounds, False)
        require(all(bounds[0] <= x < bounds[0] + bounds[2] and bounds[1] <= y < bounds[1] + bounds[3]
                    for x, y in probes) and abs(bounds[1] - rest_box[1]) <= .5,
                f"{appearance}/{phase}: fixed floating positions changed")
        image = image_for(artifacts / name, False)
        actual = [px(image, *probe) for probe in probes]
        require(actual == [ink] * 4 and frame["solid_probe_rgb"] == list(map(list, actual))
                and region(image, bounds, ink) >= frame["label_pixels"] > 12,
                f"{appearance}/{phase}: actual original ink does not match the golden color")
        captures[name] = sha(artifacts / name)
    ax = block["ax"]
    for state, active, invalid in (("rest", False, False), ("focused", True, False), ("error_proxy", False, True),
                                    ("error", True, True), ("cleared", True, False), ("reversed", True, True)):
        node = ax[state]
        description = "請輸入有效的專案名稱。" if invalid else HELP
        require(node["label"] == TITLE and description in node["description"] and node["value"] == "草稿"
                and node["focused"] is active and node["invalid"] is invalid and node["named_textboxes"] == 1
                and type(node["backend"]) is int and node["backend"] > 0, "fixed AX value, name or validation differs")
    active = ax["focused"]["backend"]
    require(all(ax[state]["backend"] == active for state in ("error", "cleared", "reversed"))
            and all(ax[state]["backend"] != active for state in ("rest", "error_proxy")),
            "separate or stale active Chrome textarea backend")
    require(block["model"] == {"value": "草稿", "edits": 1, "submits": 0, "error": True}
            and sum(event["key"] == "Enter" for event in block["trusted"]) == 3,
            "the draft changed during real focus or validation keys")
    return captures


def verify_artifacts(artifacts):
    path = artifacts / f"{PREFIX}.json"
    report = json.loads(path.read_text())
    require(report["dpr"] == DPR and report["visible_clock"] == "controlled performance.now"
            and report["page_errors"] == [] and "failure" not in report, "wrong clock or failed page")
    require(re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", report["browser"]), "no actual browser version")
    adapter = report["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal-")
            and adapter["isFallbackAdapter"] is False, "Apple Metal Chrome adapter missing")
    require("label_color" in report and report["label_color"]["space"] == "Oklab"
            and report["label_color"]["spring"] == {"stiffness": 3800, "damping_ratio": 1},
            "missing the typed .68 color test")
    captures = spatial_frames(artifacts, report)
    for appearance in ("outlined", "filled"):
        captures.update(verify_color_case(artifacts, appearance, report["label_color"][appearance]))
    require(len(captures) == 34 and set(captures) == {p.name for p in artifacts.glob(f"{PREFIX}-*.png")},
            "missing, extra or duplicate label color PNG")
    return {"report_sha256": sha(path), "browser": report["browser"], "adapter": adapter, "captures": captures}


def read(path):
    return json.loads(path.read_text())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(verify_artifacts(args.artifacts), ensure_ascii=False, indent=2))
