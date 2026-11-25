#!/usr/bin/env python3
"""Read untouched scoped label motion receipt and reproduce its PNG samples."""

import argparse
from hashlib import sha256
import json
import math
from pathlib import Path
import re

from PIL import Image

PREFIX = "material-text-field-label-motion"
TITLE = "專案名稱"
HELP = "請輸入中文或英文名稱。"
LONG_TITLE = "輸入團隊成員在共用專案清單中容易辨認的完整名稱"
DPR = 2
HEADER = 44


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
        image = original.convert("RGB")
    return image


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


def verify_frame(artifacts, scene, frame, suffix, ms, appearance, focused):
    require(appearance in ("outlined", "filled", "compact"), "wrong appearance")
    compact = appearance == "compact"
    filename = f"{PREFIX}-{suffix}.png"
    require(frame["png"] == filename and frame["ms"] == ms, f"{suffix}: incorrect timestamp or capture")
    path = artifacts / filename
    image = image_for(path, compact)
    palette = scene["palette"]
    ink = color(palette, "primary" if focused else "on_surface_variant")
    label = frame["label"]
    check_contains(label, compact)
    require(frame["label_pixels"] > 12 and region(image, label, ink) >= frame["label_pixels"],
            f"{suffix}: reported label missing in the original PNG")
    if appearance in ("outlined", "compact"):
        paper = color(palette, "background")
        border = color(palette, "primary" if focused else "outline")
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
        x, inline_top, _, height = scene["inline"]
        _, float_top, span, float_height = scene["floated"]
        top = inline_top if appearance == "outlined" else float_top + float_height
        clip = [x, top, span * 1.5 + 24, height]
        placeholder = region(image, clip, color(palette, "on_surface_variant"))
        require(placeholder == frame["placeholder_pixels"],
                f"{suffix}: placeholder pixels differ from the original capture")
    return sha(path)


def verify_run(root, allow_guard_negative):
    artifacts = root / "artifacts"
    path = artifacts / f"{PREFIX}.json"
    report = json.loads(path.read_text())
    require(report["dpr"] == DPR and report["visible_clock"] == "controlled performance.now"
            and report["page_errors"] == [] and "failure" not in report, "wrong clock or failed page")
    require(re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", report["browser"]), "no actual browser version")
    adapter = report["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal-")
            and adapter["isFallbackAdapter"] is False, "Apple Metal Chrome adapter missing")
    original_log = root / "runner.log"
    log = original_log.read_text()
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"]
            and re.findall(r"^  \[(M\d+)\]", log, re.M) == [f"M{i}" for i in range(325, 351)],
            "expected ordered scoped M325–M350 ledger")
    counts = re.findall(r"^case material \.\.\. ok \(26 steps, (\d+) checks,", log, re.M)
    require(len(counts) == 1 and int(counts[0]) >= 26 and
            re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", log, re.M) == counts,
            "expected precise scoped pass and summary count")
    window_path = root / "headless-window-observation.json"
    window = json.loads(window_path.read_text())
    observed = window["launch_observations"]["chrome-primary"]
    guarded, = window["guard_events"]
    require(window["stock_exit"] == 0 and window["visible_owned_windows"] == []
            and guarded["allowed"] is True and guarded["role"] == "chrome-primary"
            and "--headless=new" in guarded["argv"] and guarded["profile"]
            and observed["pids"] == [guarded["pid"]] and observed["samples"] > 0,
            "missing successful exclusively guarded, windowless Chrome observations")
    guard_clean = window["problems"] == [] and window["remaining_observed_processes"] == []
    require(guard_clean or (allow_guard_negative
                            and window["problems"] == ["owned process survived cleanup grace"]
                            and window["remaining_observed_processes"]),
            "owned Chrome did not naturally exit within the guard's bounded grace")
    outlined, reversal, filled, compact = (report[key] for key in ("outlined", "reversal", "filled", "compact"))
    for appearance, scene in (("outlined", outlined), ("filled", filled), ("compact", compact)):
        check_contains(scene["face"], appearance == "compact")
        check_contains(scene["floated"], appearance == "compact")
    ch = {}
    groups = [
        (outlined, outlined, "outlined", ["rest", "first", "early", "middle", "settled"],
         [0, 0, 14, 44, 800], [False, True, True, True, True]),
        (outlined, reversal, "outlined", ["falling", "retarget", "reduced"],
         [844, 844, 844], [False, True, True]),
        (filled, filled, "filled", ["first", "middle", "settled"],
         [0, 44, 800], [True, True, True]),
        (compact, compact, "compact", ["rest", "middle"], [0, 44], [False, True]),
    ]
    for scene, series, appearance, suffixes, times, focused in groups:
        prefix = "rtl" if appearance == "compact" else appearance
        for frame, suffix, ms, active in zip(series["frames"], suffixes, times, focused, strict=True):
            ch[frame["png"]] = verify_frame(artifacts, scene, frame, f"{prefix}-{suffix}", ms, appearance, active)

    rest, first, early, middle, end = outlined["frames"]
    falling, retarget, reduced = reversal["frames"]
    paper = outlined["palette"]["background"][:3]
    focus_color = outlined["palette"]["primary"][:3]
    require(rest["label"] == first["label"] and rest["gap_pixel_columns"] == first["gap_pixel_columns"] == 0
            and first["placeholder_pixels"] == early["placeholder_pixels"] == 0
            and early["notch"] == [paper, focus_color] and middle["notch"] == end["notch"] == [paper, paper]
            and 8 <= early["gap_pixel_columns"] < middle["gap_pixel_columns"]
            and first["label"][1] > early["label"][1] > middle["label"][1] > end["label"][1],
            "the Outlined notch or label trajectory changed")
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
    ch[filename] = sha(artifacts / filename)

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
    ch[filename] = sha(artifacts / filename)
    require(len(ch) == 15 and set(ch) == {p.name for p in artifacts.glob(f"{PREFIX}-*.png")},
            "missing, unexpected or duplicate label motion originals")
    return {"scope": "scoped M325–M350", "browser": report["browser"], "adapter": adapter,
            "checks": int(counts[0]), "steps": 26, "report_sha256": sha(path),
            "runner_log_sha256": sha(original_log), "window_receipt_sha256": sha(window_path),
            "visible_windows": 0, "observed_headless_samples": observed["samples"],
            "guard_passed": guard_clean, "remaining_owned_processes": window["remaining_observed_processes"],
            "captures": ch}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--allow-guard-negative", action="store_true",
                        help="read known diagnostic development evidence without qualifying it")
    args = parser.parse_args()
    print(json.dumps(verify_run(args.root, args.allow_guard_negative), indent=2, sort_keys=True))
