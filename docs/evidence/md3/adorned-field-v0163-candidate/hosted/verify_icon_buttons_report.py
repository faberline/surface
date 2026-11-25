#!/usr/bin/env python3
"""Independently read the Icon Buttons report and Chrome PNG evidence."""

from hashlib import file_digest, sha256
import json
from math import floor, isfinite
import re

from PIL import Image

STEM = "material-icon-buttons"
REPORT = f"{STEM}.json"
STEPS = ("M294", "M295", "M296", "M297")
IMAGES = {f"{STEM}-{name}.png": size for name, size in (
    ("wide-light", (1920, 1688)), ("selected-focus", (1920, 1688)),
    ("pressed", (1920, 1688)), ("compact-dark-rtl-200", (640, 1608)),
)}
WIDE, SELECTED, HELD, COMPACT = IMAGES
SPECS = (
    ("command", "儲存", 28, 32, 20), ("toggle", "加星號", 52, 40, 24),
    ("disabled", "鎖定", 48, 56, 24), ("medium", "搜尋", 56, 56, 24),
    ("large", "更多選項", 64, 96, 32), ("extra-large", "收件匣", 184, 136, 40),
)
IDS = tuple(f"icon-button-demo-{kind}" for kind, *_ in SPECS)
PALETTE = (
    "background", "primary", "on_primary", "secondary", "on_secondary",
    "secondary_container", "on_secondary_container", "outline_variant",
)


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def digest(path):
    with path.open("rb") as handle:
        return file_digest(handle, "sha256").hexdigest()


def numbers(values, length):
    require(type(values) is list and len(values) == length
            and all(type(n) in (int, float) and isfinite(n) for n in values), "wrong Icon Button geometry")
    return values


def rect(value):
    box = numbers(value, 4)
    require(min(box[2:]) > 0, "empty Icon Button rectangle")
    return box


def center(box):
    return box[0] + box[2] / 2, box[1] + box[3] / 2


def end(box, axis):
    return box[axis] + box[axis + 2]


def near(a, b):
    return abs(a - b) <= 0.75


def contains(outer, inner):
    return all(outer[i] <= inner[i] + 0.75 and end(inner, i) <= end(outer, i) + 0.75 for i in (0, 1))


def geom(data, viewport, scroll=None):
    require(type(data) is list and tuple(g["id"] for g in data) == IDS, "wrong Icon Button specimen IDs")
    for entry, (_, _, width, height, icon) in zip(data, SPECS, strict=True):
        boxes = {key: rect(entry[key]) for key in ("hit", "face", "icon", "focus")}
        sizes = {"hit": [max(width, 48), max(height, 48)], "face": [width, height],
                 "focus": [width + 10, height + 10], "icon": [icon, icon]}
        for key, (w, h) in sizes.items():
            box = boxes[key]
            require(near(box[2], w) and near(box[3], h)
                    and all(near(a, b) for a, b in zip(center(box), center(boxes["hit"]))),
                    f"wrong Icon Button {key} size or center")
        hit = boxes["hit"]
        require(contains(viewport, hit) and (scroll is None or contains(scroll, hit))
                and contains(hit, boxes["face"]) and contains(boxes["face"], boxes["icon"]),
                "Icon Button target is not fully reachable")
    return data


def ax_button(node, index, pressed=None):
    roles = {"toggleButton", "button"} if index == 1 else {"button"}
    require(node["id"] == IDS[index] and node["name"] == SPECS[index][1]
            and node["role"] in roles and node["disabled"] is (index == 2)
            and node["pressed"] == pressed, "wrong named Chrome AX button, disabled or pressed state")


def color(value, channels=3):
    require(type(value) is list and len(value) == channels
            and all(type(c) is int and 0 <= c <= 255 for c in value), "invalid Icon Button color")
    return tuple(value)


def palette(values):
    result = {}
    for key in PALETTE:
        rgba = color(values[key], 4)
        require(rgba[3] == 255, f"Icon Button token {key} is not opaque")
        result[key] = rgba[:3]
    return result


def pixel(img, x, y):
    point = (floor(2 * x), floor(2 * (44 + y)))
    require(0 <= point[0] < img.width and 0 <= point[1] < img.height, "Icon Button sample escapes the PNG")
    return img.getpixel(point)


def fill(img, box):
    return pixel(img, center(box)[0], box[1] + 3)


def crop(img, box):
    x, y, width, height = box
    region = [floor(2 * n + 0.5) for n in (x, 44 + y, x + width, 44 + y + height)]
    require(0 <= region[0] < region[2] <= img.width and 0 <= region[1] < region[3] <= img.height,
            "Icon Button paint crop escapes the PNG")
    return img.crop(tuple(region))


def count(img, box, key):
    return sum(sample == key for sample in crop(img, box).getdata())


def mask(img, box, token):
    return [[x, y] for y in range(24) for x in range(24)
            if pixel(img, box[0] + x + 0.5, box[1] + y + 0.5) == token]


def focus_matches(actual, base, ink):
    return all(abs(a - (b * 229 + c * 26) // 255) <= 2 for a, b, c in zip(actual, base, ink))


def validate(root, log, metadata):
    path = root / REPORT
    report = json.loads(path.read_text())
    require(set(report) == {"adapter", "browser", "compact", "keyboard", "page_errors", "pointer", "wide"}
            and report["browser"] == metadata["browser"] and report["adapter"] == metadata["adapter"]
            and type(report["adapter"]["isFallbackAdapter"]) is bool and report["page_errors"] == [],
            "Icon Button failure, page errors or Chrome adapter mismatch")
    require(tuple(re.findall(r"^\s*\[(M29[4-7])\]", log.read_text(), re.M)) == STEPS,
            "missing, duplicate or out-of-order Icon Button step")
    require({p.name for p in root.glob(f"{STEM}-*.png")} == set(IMAGES), "wrong Icon Button PNG set")
    images, captures = {}, {}
    for name, dimensions in IMAGES.items():
        original = root / name
        with Image.open(original) as opened:
            require(opened.format == "PNG" and opened.mode in {"RGB", "RGBA"} and opened.size == dimensions,
                    f"wrong Icon Button screenshot dimensions: {name}")
            opened.verify()
        with Image.open(original) as opened:
            opened.load()
            decoded = opened.convert("RGB")
        images[name] = decoded
        captures[name] = {"size": list(dimensions), "sha256": digest(original),
                          "decoded_rgb_sha256": sha256(decoded.tobytes()).hexdigest()}
    require(len({item["decoded_rgb_sha256"] for item in captures.values()}) == len(IMAGES),
            "duplicate Icon Button screenshot pixels")
    wide, keyboard, pointer, compact = (report[key] for key in ("wide", "keyboard", "pointer", "compact"))
    require([item["capture"] for item in (wide, keyboard, pointer, compact)] == list(IMAGES), "bad Icon Button capture links")
    require(wide["viewport"] == [960, 800] and compact["viewport"] == [320, 760]
            and compact["rtl"] is True and compact["text_scale"] == 2, "wrong Icon Button profiles")
    geometry = geom(wide["geometry"], [0, 0, 960, 800])
    require(len(wide["ax"]) == len(IDS), "missing Chrome AX specimens")
    for i, node in enumerate(wide["ax"]):
        ax_button(node, i, "false" if i == 1 else None)
    for key, value in (("checked_ax", "true"), ("unchecked_ax", "false")):
        ax_button(keyboard[key], 1, value)
    require(keyboard["focus_path"] == [IDS[i] for i in (0, 1, 3, 1)]
            and keyboard["requests"] == [1, 2, 3] and keyboard["repeated_spaces"] == 2,
            "wrong Icon Button focus path, accepted requests or repeat behavior")
    light, selected, held, dark = (images[name] for name in IMAGES)
    tokens = palette(wide["palette"])
    command, toggle, _, outlined, *_ = geometry
    face = toggle["face"]
    for name, rendered, expected in (
        ("command_fill", fill(light, command["face"]), tokens["primary"]),
        ("toggle_off_fill", fill(light, face), tokens["secondary_container"]),
        ("toggle_off_corner", pixel(light, face[0] + 2, face[1] + 8), tokens["background"]),
        ("outline", pixel(light, outlined["face"][0] + 0.5, center(outlined["face"])[1]), tokens["outline_variant"]),
    ):
        require(color(wide[name]) == rendered == expected, f"decoded Icon Button {name} color differs")
    selected_fill = fill(selected, face)
    require(focus_matches(selected_fill, tokens["secondary"], tokens["on_secondary"])
            and selected_fill == color(keyboard["selected_fill"]) == color(keyboard["toggle_on_corner"])
            == pixel(selected, face[0] + 2, face[1] + 8) != color(wide["toggle_off_corner"]),
            "Icon Button checked focus blend or square corner differs")
    off_mask = mask(light, toggle["icon"], tokens["on_secondary_container"])
    on_mask = mask(selected, toggle["icon"], tokens["on_secondary"])
    require(off_mask == wide["off_ink"] and on_mask == keyboard["on_ink"] and off_mask != on_mask
            and min(len(off_mask), len(on_mask)) > 12, "decoded Icon Button artwork mask differs")
    focus = count(selected, toggle["focus"], tokens["secondary"]) - count(selected, face, tokens["secondary"])
    require(focus == keyboard["focus_pixels"] and focus > 40, "decoded exterior Icon Button focus ink differs")
    light_ink = count(light, command["icon"], tokens["on_primary"])
    require(light_ink == wide["command_icon_pixels"] and light_ink > 40, "decoded light command ink differs")

    hit, face = command["hit"], command["face"]
    x, y = numbers(pointer["slack_point"], 2)
    outside = numbers(pointer["outside_point"], 2)
    patch = rect(pointer["corner_patch"])
    require(x < face[0] and contains(hit, [x, y, 1, 1]) and not contains(hit, [*outside, 1, 1])
            and contains([0, 0, 960, 800], [*outside, 1, 1]) and pointer["face"] == face
            and patch == [face[0] + 0.5, face[1] + 3, 6, 8], "wrong Icon Button accepted hit slack, outside, or probe region")
    rest, early, held_count, released = pointer["corner_pixels"]
    require(all(type(n) is int and n >= 0 for n in (rest, early, held_count, released, pointer["reduced_corner_pixels"]))
            and pointer["normal_times_ms"] == [0, 28, 240, 480]
            and rest < early < held_count and released == rest and pointer["reduced_corner_pixels"] > early
            and abs(pointer["reduced_corner_pixels"] - held_count) <= 8,
            "wrong Icon Button rest, held, release or Reduce Motion contours")
    require(pointer["requests_after_cancel"] == 0 and pointer["requests_after_hit"] == 1,
            "wrong Icon Button cancellation or accepted request count")
    held_pixels = sum(sample != tokens["background"] for sample in crop(held, patch).getdata())
    require(held_pixels == held_count, "decoded Icon Button pressed corner differs")

    compact_clip = rect(compact["scroll"])
    small = geom(compact["geometry"], [0, 0, 320, 760], compact_clip)
    tooltip = rect(compact["tooltip"])
    command = small[0]
    require(command["hit"][0] > small[1]["hit"][0] and near(center(command["hit"])[1], center(small[1]["hit"])[1]),
            "first two compact Icon Buttons did not mirror RTL")
    require(compact["description"] == "儲存目前對話" and contains([0, 0, 320, 760], tooltip)
            and end(tooltip, 1) <= min(command["hit"][1], command["focus"][1]) + 0.5,
            "wrong or overlapping compact Icon Button tooltip")
    dark_tokens = palette(compact["palette"])
    dark_fill = fill(dark, command["face"])
    require(sum(tokens["background"]) > sum(dark_tokens["background"]) + 300
            and color(compact["command_fill"]) == dark_fill
            and focus_matches(dark_fill, dark_tokens["primary"], dark_tokens["on_primary"]),
            "decoded dark command focus color differs")
    dark_ink = count(dark, command["icon"], dark_tokens["on_primary"])
    require(dark_ink == compact["command_icon_pixels"] and dark_ink > 40, "decoded dark command ink differs")
    return {"steps": list(STEPS), "report_sha256": digest(path), "pngs": captures, "page_errors": 0,
            "specimens": len(IDS), "focus_pixels": focus, "light_command_ink": light_ink,
            "dark_command_ink": dark_ink, "pressed_corner_pixels": held_pixels,
            "selected_and_unselected_artwork": [len(on_mask), len(off_mask)],
            "compact_rtl_tooltip": compact["description"]}
