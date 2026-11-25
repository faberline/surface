#!/usr/bin/env python3
"""Recount Common Buttons Canvas pixels and verify input and geometry receipts."""

import argparse
from hashlib import sha256
import json
from math import floor, isfinite
from pathlib import Path
import re

from PIL import Image

from verify_forms_pixels import checksum, require

STEM = "material-buttons"
REPORT = f"{STEM}.json"
LIGHT, CHECKED, HELD, COMPACT = (f"{STEM}-{suffix}.png" for suffix in ("light", "checked", "held", "dark-rtl-200"))
REST, EARLY, REDUCED = (f"{STEM}-motion-{suffix}.png" for suffix in ("rest", "early", "reduced"))
MOTION = (REST, EARLY, HELD, REDUCED)
IMAGES = {name: (1920, 2408) for name in (LIGHT, CHECKED, *MOTION)} | {COMPACT: (640, 1848)}
STEPS = ("M316", "M317", "M318", "M319")
SIZES = tuple(zip(("xs", "s", "m", "l", "xl"), (32, 40, 56, 96, 136), (20, 20, 24, 32, 40),
                  (4, 8, 8, 12, 16), (12, 16, 24, 48, 64), strict=True))
COMMANDS = (
    ("elevated", "surface_container_low", "primary"),
    ("filled", "primary", "on_primary"),
    ("tonal", "secondary_container", "on_secondary_container"),
    ("outlined", "background", "on_surface_variant"),
    ("text", "background", "primary"),
)
TOGGLES = (
    ("elevated", "primary", "on_primary"),
    ("filled", "primary", "on_primary"),
    ("tonal", "secondary", "on_secondary"),
    ("outlined", "inverse_surface", "inverse_on_surface"),
)
DETAILS = ("button-demo-leading", "button-demo-trailing", "button-demo-wrapped")
WIDE_VIEW = [0, 0, 960, 1160]
COMPACT_VIEW = [0, 0, 320, 880]


def stable_id(part, suffix):
    return f"button-demo-{part}-{suffix}"


def exact(actual, expected):
    return type(actual) in (int, float) and isfinite(actual) and abs(actual - expected) <= 0.75


def rect(value):
    require(isinstance(value, list) and len(value) == 4 and all(
        type(number) in (int, float) and isfinite(number) for number in value
    ) and min(value[2:]) > 0, f"invalid button paint rectangle: {value}")
    return value


def contains(parent, child):
    px, py, pw, ph = parent
    x, y, w, h = child
    return px - 0.75 <= x and py - 0.75 <= y and x + w <= px + pw + 0.75 and y + h <= py + ph + 0.75


def geometry(entry, view):
    require(set(entry) == {"hit", "face", "label", "focus"}, "wrong button geometry schema")
    hit, face, label, focus = (rect(entry[k]) for k in ("hit", "face", "label", "focus"))
    require(min(hit[2:]) >= 48 and contains(hit, face) and contains(face, label), "wrong hit, face or label geometry")
    require(contains(view, focus) and all(exact(face[n] - focus[n], 5)
            and exact(focus[n + 2] - face[n + 2], 10) for n in range(2)), "wrong exterior focus geometry")
    return hit, face, label, focus


def color_from(palette, name):
    color = palette[name]
    require(isinstance(color, list) and len(color) == 4 and all(type(c) is int and 0 <= c <= 255 for c in color)
            and color[-1] == 255, f"bad opaque RGBA value for {name}")
    return tuple(color[:3])


def sample(image, x, y):
    point = floor(x * 2), floor((y + 44) * 2)
    require(all(0 <= p < limit for p, limit in zip(point, image.size, strict=True)), "pixel is outside button PNG")
    return image.getpixel(point)


def fill(image, face):
    x, y, width, _ = face
    return sample(image, x + width / 2, y + 3.5)


def histogram(image, region):
    x, y, width, height = rect(region)
    pixel = lambda value: floor(value * 2 + 0.5)
    left, top, right, bottom = [pixel(value) for value in (x, y + 44, x + width, y + 44 + height)]
    require(0 <= left < right <= image.width and 0 <= top < bottom <= image.height, "button pixel region is outside PNG")
    return image.crop((left, top, right, bottom)).getcolors(maxcolors=(right - left) * (bottom - top))


def ink(image, region, color):
    return sum(count for count, pixel in histogram(image, region) if pixel == color)


def screenshots(artifacts):
    require({item.name for item in artifacts.glob(f"{STEM}-*.png")} == set(IMAGES), "missing, unexpected or failed Buttons PNG")
    decoded, originals = {}, {}
    for name, size in IMAGES.items():
        path = artifacts / name
        with Image.open(path) as opened:
            require(opened.format == "PNG" and opened.mode in {"RGB", "RGBA"} and opened.size == size,
                    f"wrong button PNG dimensions or format: {name}")
            opened.verify()
        with Image.open(path) as opened:
            opened.load()
            decoded[name] = image = opened.convert("RGB")
        originals[name] = {"size": list(size), "sha256": checksum(path), "decoded_rgb_sha256": sha256(image.tobytes()).hexdigest()}
    return decoded, originals


def wide(part, image):
    palette = part["palette"]
    require(len(part["sizes"]) == len(SIZES) and len(part["commands"]) == len(COMMANDS), "missing size or command")
    prior = None
    glyph_counts = []
    for specimen, (suffix, height, icon, gap, padding) in zip(part["sizes"], SIZES, strict=True):
        require(specimen["id"] == stable_id("size", suffix), "button fixture size order differs")
        _, face, label, _ = geometry(specimen["geometry"], WIDE_VIEW)
        glyph = rect(specimen["glyph"])
        require(contains(face, glyph) and all(exact(value, token) for value, token in (
            (face[3], height), (glyph[2], icon), (glyph[3], icon), (label[0] - glyph[0] - icon, gap),
            (specimen["gap"], gap), (glyph[0] - face[0], padding),
            (face[0] + face[2] - (label[0] + label[2]), padding),
        )), "button icon, face size, lateral inset or gap differs")
        require(prior is None or prior[0] + prior[2] < face[0], "wide sizes overlap or wrap")
        prior = face
        pixels = ink(image, glyph, color_from(palette, "on_primary"))
        require(pixels > 40, "size icon lacks actual Canvas paint")
        glyph_counts.append(pixels)
    commands = {}
    for spec, (suffix, background, foreground) in zip(part["commands"], COMMANDS, strict=True):
        require(spec["id"] == stable_id("command", suffix), "button command style order differs")
        _, face, label, _ = geometry(spec["geometry"], WIDE_VIEW)
        face_ink = fill(image, face)
        count = ink(image, label, color_from(palette, foreground))
        require(tuple(spec["fill"]) == face_ink == color_from(palette, background) and count == spec["label_ink"] > 25,
                "command face or label differs from reported pixels")
        if suffix == "outlined":
            require(sample(image, face[0] + 0.5, face[1] + face[3] / 2) == color_from(palette, "outline_variant"),
                    "outlined command stroke color differs")
        commands[spec["id"]] = {"fill": list(face_ink), "label_ink": count}
    return glyph_counts, commands


def checked(part, image, strict):
    require(len(part["states"]) == len(TOGGLES), "missing controlled toggle")
    paints = {}
    for entry, (suffix, background, foreground) in zip(part["states"], TOGGLES, strict=True):
        require(entry["id"] == stable_id("toggle", suffix) and entry["checked"] is True
                and type(entry["requests"]) is int and entry["requests"] == 1, "wrong toggle state or activation count")
        if strict or "geometry" in entry:
            require({"geometry", "fill", "label_ink"} <= entry.keys(), "exact stock lacks checked-toggle paint receipt")
            _, face, label, _ = geometry(entry["geometry"], WIDE_VIEW)
            face_ink = fill(image, face)
            count = ink(image, label, color_from(part["palette"], foreground))
            require(face_ink == color_from(part["palette"], background) and count > 25,
                    "checked face or label paint differs")
            require(tuple(entry["fill"]) == face_ink and entry["label_ink"] == count,
                    "recorded checked face or label paint differs")
            paints[entry["id"]] = {"fill": list(face_ink), "label_ink": count}
    return paints


def motion(part, light, images):
    require(part["originals"] == list(MOTION), "wrong set or order of button motion frames")
    palette = light["palette"]
    face_color, background = (color_from(palette, key) for key in ("primary", "background"))
    target, face, _, _ = geometry(light["sizes"][0]["geometry"], WIDE_VIEW)
    region = rect(part["region"])
    require(all(exact(actual, token) for actual, token in zip(region, [*face[:2], 13, 13], strict=True)),
            "motion corner region differs from the actual XS face")
    hit_x, hit_y = part["hit"]
    require(exact(hit_x, target[0] + target[2] / 2) and exact(hit_y, target[1] + 2) and hit_y < face[1],
            "press missed the XS touch slack")
    distance = lambda pixel, target: sum(abs(a - b) for a, b in zip(pixel, target, strict=True))
    measured = [sum(count for count, rgb in histogram(images[name], region)
                    if distance(rgb, face_color) < distance(rgb, background)) for name in MOTION]
    rest, early, held, reduced = measured
    require(measured == part["corner_pixels"] and 0 < rest < early < held and early < reduced
            and abs(reduced - held) <= 8, "motion PNG does not prove an intermediate frame and snapped Reduce Motion endpoint")
    receipts = tuple(part[key] for key in ("outside_requests", "accepted_mouse", "after_touch_cancel", "after_touch_tap"))
    require(all(type(count) is int for count in receipts) and receipts == (0, 1, 1, 2), "mouse release or touch cancellation receipt differs")
    return measured


def compact(part, image, *, strict, development_palette=None):
    hit, face, label, focus = geometry(part["geometry"], COMPACT_VIEW)
    order = [stable_id("size", suffix) for suffix, *_ in SIZES]
    order += [stable_id("command", suffix) for suffix, *_ in COMMANDS]
    order += [stable_id("toggle", suffix) for suffix, *_ in TOGGLES]
    require(part["tab_order"] == order + list(DETAILS) and part["back_focus"] == "button-demo-show"
            and label[3] > 48, "compact Tab order, return focus or wrapped label differs")
    palette = part.get("palette", development_palette)
    if strict:
        require({"palette", "scroll", "initial_hit", "icon_slots", "size"} <= part.keys(),
                "exact stock lacks compact theme, reveal or RTL geometry")
        size = part["size"]
        require(size["dark"] is True and size["rtl"] is True and size["text_scale"] == 2
                and [size["width"], size["height"]] == COMPACT_VIEW[2:],
                "wrong compact theme, direction and scale")
        view = rect(part["scroll"])
        initial = rect(part["initial_hit"])
        require(contains(COMPACT_VIEW, view) and contains(view, focus) and contains(view, label)
                and not contains(view, initial) and hit[1] < initial[1], "compact Tab and scroll reveal did not move the target into view")
        icons = part["icon_slots"]
        require([entry["id"] for entry in icons] == list(DETAILS[:2]), "missing RTL icon owners")
        for entry, leading, expected_gap in zip(icons, (True, False), (4, 8), strict=True):
            icon = rect(entry["icon"])
            copy = rect(entry["label"])
            gap = icon[0] - copy[0] - copy[2] if leading else copy[0] - icon[0] - icon[2]
            require(exact(gap, expected_gap) and all(exact(edge, 20) for edge in icon[2:]),
                    "RTL failed to mirror an icon or keep its size/gap")
    require(palette is not None, "provide the independently pinned dark development palette")
    color = color_from(palette, "secondary")
    ring = ink(image, focus, color) - ink(image, face, color)
    text = ink(image, label, color_from(palette, "on_primary"))
    require(ring == part["ring_pixels"] > 60 and text == part["label_pixels"] > 120,
            "compact PNG focus or actual large text ink differs")
    require(fill(image, face) != color_from(palette, "background"), "compact focus face has no paint")
    return {"ring_pixels": ring, "label_pixels": text, "back_focus": part["back_focus"], "tab_order": order + list(DETAILS)}


def verify(artifacts, metadata=None, *, log=None, development_palette=None):
    path = artifacts / REPORT
    data = json.loads(path.read_text())
    strict = development_palette is None
    require(set(data) in ({"browser", "checked", "compact", "held", "light", "page_errors"},
                          {"browser", "adapter", "checked", "compact", "held", "light", "page_errors"})
            and data["page_errors"] == [], "failed, incomplete or altered Common Buttons report")
    require(re.fullmatch(r"Chrome/[\d.]+", data["browser"]), "wrong Buttons Chrome metadata")
    if metadata is not None:
        require(data["browser"] == metadata["browser"] and ("adapter" not in data or data["adapter"] == metadata["adapter"]),
                "Buttons Chrome or WebGPU adapter disagrees")
    if log is not None:
        require(tuple(re.findall(r"^\s*\[(M31[6-9])\]", log.read_text(), re.M)) == STEPS,
                "wrong ordered Common Buttons checkpoints")
    require([data[key]["capture"] for key in ("light", "checked", "held", "compact")] == [LIGHT, CHECKED, HELD, COMPACT],
            "wrong Common Buttons capture names")
    images, captures = screenshots(artifacts)
    require(captures[CHECKED]["decoded_rgb_sha256"] != captures[LIGHT]["decoded_rgb_sha256"], "checked PNG reuses the resting image")
    glyphs, commands = wide(data["light"], images[LIGHT])
    return {"scope": "stock-feature" if strict else "focused development calibration only", "report_sha256": checksum(path),
            "steps": list(STEPS), "captures": captures, "browser": data["browser"], "page_errors": 0,
            "size_icon_pixels": glyphs, "command_pixels": commands,
            "checked_pixels": checked(data["checked"], images[CHECKED], strict),
            "corner_pixels": motion(data["held"], data["light"], images),
            "compact": compact(data["compact"], images[COMPACT], strict=strict, development_palette=development_palette)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    parser.add_argument("--development-dark-report", type=Path, help="Use a prior verified dark palette in focused calibration only")
    args = parser.parse_args()
    palette = None
    if args.development_dark_report:
        palette = json.loads(args.development_dark_report.read_text())["compact"]["palette"]
    print(json.dumps(verify(args.run / "artifacts", log=args.run / "runner.log", development_palette=palette), indent=2))


if __name__ == "__main__":
    main()
