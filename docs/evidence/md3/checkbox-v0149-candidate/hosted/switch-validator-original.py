#!/usr/bin/env python3
"""Verify stock or focused Switch Chrome receipts offline."""

import argparse
from hashlib import sha256
import json
from math import floor, isfinite
from pathlib import Path, PurePosixPath
import re
import sys

from PIL import Image

STEM = "material-switch"
SOURCE_FILES = (
    "crates/surface-material/src/switch.rs",
    "e2e/src/material_switch.rs",
    "examples/material-gallery/src/switch_demo.rs",
    "examples/material-gallery/web/index.html",
)
SWITCH_STEPS = [f"M{i}" for i in range(274, 278)]
STOCK_STEPS = [f"M{i}" for i in range(1, 278) if i != 23]
SUITES = {
    "focused": ["material"],
    "material": ["material"],
    "stock-macos": ["smoke", "ime", "material", "bridge-web", "devtools-headless"],
    "stock-linux": [
        "smoke", "ime", "material", "bridge-native", "bridge-native-compact",
        "bridge-web", "devtools-native", "devtools-headless",
    ],
}
SHOT_WIDE = "wide-settled"
SHOT_COMPACT = "compact-dark-rtl-200"
SHOTS = {SHOT_WIDE: (960, 800), "hovered": (960, 800), "held": (960, 800), SHOT_COMPACT: (320, 640)}
SCALE = 2
HEADER = 44
IDS = ["switch-demo-" + name for name in ("plain", "selected-icon", "both-icons", "disabled-off", "disabled-on")]
NAMES = ["通知", "相機", "行動網路同步", "位置存取", "自動備份"]
START = [False, True, False, False, True]
AFTER_SPACE = [True, True, False, False, True]
AFTER_CLICK = [True, False, False, False, True]
AFTER_LABEL = [True, False, True, False, True]
SWIFTSHADER = {"vendor": "google", "architecture": "swiftshader", "isFallbackAdapter": True}


def need(condition, reason):
    if not condition:
        raise ValueError(reason)


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def manifest_record(path, expected, source_root):
    observed = digest(path)
    need(observed == expected, "source manifest SHA-256 mismatch")
    entries = {}
    for line in path.read_text().splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  (.+)", line)
        need(match is not None, "invalid source manifest line")
        checksum, name = match.groups()
        relative = PurePosixPath(name)
        need(not relative.is_absolute() and ".." not in relative.parts and relative.as_posix() == name
             and name not in entries, f"unsafe or duplicate manifest path: {name}")
        entries[name] = checksum
    need(set(SOURCE_FILES) <= entries.keys(), "source manifest omits Switch or screenshot source")
    if source_root is not None:
        root = source_root.resolve(strict=True)
        for name, checksum in entries.items():
            source = (root / name).resolve(strict=True)
            need(source.is_relative_to(root) and digest(source) == checksum, f"source input mismatch: {name}")
    return {"manifest_sha256": observed, "files": len(entries), "source_bytes_checked": source_root is not None,
            "switch_inputs": {name: entries[name] for name in SOURCE_FILES}}


def runner_record(path, profile):
    raw = path.read_text()
    raw = re.sub(r"^(?:(?:[^\t\n]+\t){2})?\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z ",
                 "", raw, flags=re.M)
    started = re.findall(r"^case (\S+) \.\.\.$", raw, re.M)
    passed = re.findall(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks, \d+(?:\.\d+)?s\)$",
                        raw, re.M)
    summary = re.findall(r"^e2e result: (\d+) passed; (\d+) failed; (\d+) checks;", raw, re.M)
    steps = re.findall(r"^  \[(M\d+)\]", raw, re.M)
    versions = re.findall(r"^browser: ((?:Chrome|Chromium)/\d+(?:\.\d+)+)\b", raw, re.M)
    need(started == [row[0] for row in passed] == SUITES[profile]
         and not re.search(r"^case \S+ \.\.\. FAILED", raw, re.M), "wrong suite sequence or a failed suite")
    counts = {suite: {"steps": int(step), "checks": int(check)} for suite, step, check in passed}
    expected_steps = SWITCH_STEPS if profile == "focused" else STOCK_STEPS
    need(steps == expected_steps and counts["material"]["steps"] == len(expected_steps),
         "missing, extra or out-of-order Material step")
    need(counts["material"]["checks"] >= 300 and all(
        row["checks"] >= row["steps"] > 0 for row in counts.values()
    ), "runner check count is too low")
    total = sum(row["checks"] for row in counts.values())
    need(summary == [(str(len(counts)), "0", str(total))], "runner summary disagrees with suite totals")
    need(len(versions) == 1, "missing or ambiguous runner Chrome version")
    return {"profile": profile, "switch_steps": steps[-4:], "cases": counts, "total_checks": total,
            "browser": versions[0], "sha256": digest(path)}


def adapter_record(data, runner, expected):
    need(data["browser"] == runner["browser"] and data["errors"] == [] and data["reduced_motion"] is False
         and "failure" not in data, "browser receipt reports errors, reduced motion or a different Chrome")
    info = data["adapter"]
    need(type(info["isFallbackAdapter"]) is bool and all(type(info[key]) is str for key in ("vendor", "architecture")),
         "invalid observed WebGPU adapter")
    if expected == "metal":
        need(info["vendor"] == "apple" and info["architecture"].startswith("metal")
             and info["isFallbackAdapter"] is False, "expected observed Apple Metal")
    if expected == "swiftshader":
        need(info == SWIFTSHADER, "expected observed SwiftShader software adapter")
    return info


def numbers(values, count):
    need(type(values) is list and len(values) == count
         and all(type(item) in (int, float) and isfinite(item) for item in values), "invalid geometry")
    return values


def rect(values):
    need(min(numbers(values, 4)[2:]) > 0., "invalid geometry size")
    return values


def close(a, b):
    return abs(a - b) <= 0.75


def end(rectangle, axis=0):
    return rectangle[axis] + rectangle[axis + 2]


def center(rectangle):
    return tuple(rectangle[axis] + rectangle[axis + 2] / 2 for axis in (0, 1))


def contains(outer, inner):
    return all(inner[a] >= outer[a] - .75 and end(inner, a) <= end(outer, a) + .75 for a in (0, 1))


def geometry(g, checked, diameter, icon, rtl, viewport):
    row, track, handle, layer, label = (rect(g[key]) for key in ("row", "track", "handle", "layer", "label"))
    need(48 <= row[2] <= 480.75 and row[3] >= 48 and contains([0, 0, *viewport], row)
         and all(contains(row, child) for child in (track, handle, layer, label)), "row, art or label escaped its hit cell")
    need(track[2:] == [52, 32] and handle[2:] == [diameter, diameter] and layer[2:] == [40, 40],
         "wrong track, handle or state-layer size")
    dx, dy = center(handle)
    lx, ly = center(layer)
    need(close(dx - track[0], 36 if checked != rtl else 16)
         and close(dy, center(track)[1]) and close(lx, dx) and close(ly, dy), "wrong handle or layer center")
    if rtl:
        need(close(track[0] - row[0], 5) and close(label[0] - end(track), 17),
             "RTL track is not anchored to the left of the hit cell")
    else:
        need(close(end(row) - end(track), 5) and close(track[0] - end(label), 17),
             "LTR track is not anchored to the right of the hit cell")
    if icon:
        art = rect(g["icon"])
        need(art[2:] == [16, 16] and contains(handle, art)
             and all(close(a, b) for a, b in zip(center(art), (dx, dy))), "wrong icon size or center")
    else:
        need(g["icon"] is None, "unexpected icon in a plain or selected-only off state")
    return g


def rgb(values, channels=3):
    need(type(values) is list and len(values) == channels
         and all(type(value) is int and 0 <= value <= 255 for value in values), "invalid RGB receipt")
    return tuple(values)


def state(data, expected, changes, rtl=False):
    need(type(data["changes"]) is int and data["changes"] == changes
         and data["checked"] == expected and all(type(value) is bool for value in data["checked"])
         and data["visible"] is True and data["rtl"] is rtl
         and type(data["text_scale"]) in (int, float) and data["text_scale"] == (2 if rtl else 1),
         "wrong fixture state, scale or effective update count")
    return data["palette"]


def token(palette, key):
    *color, alpha = rgb(palette[key], 4)
    need(alpha == 255, f"{key} intended token has transparency")
    return tuple(color)


def ax_value(node, key):
    values = [prop["value"]["value"] for prop in node["properties"] if prop["name"] == key]
    need(len(values) <= 1, f"duplicate AX property {key}")
    return values[0] if values else None


def ax_switch(node, index, checked):
    disabled = ax_value(node, "disabled")
    need(node["ignored"] is False and node["role"]["value"] == "switch" and node["name"]["value"] == NAMES[index]
         and ax_value(node, "checked") == ("true" if checked else "false")
         and (disabled is True if index >= 3 else disabled is None or disabled is False), f"wrong AX switch projection for {IDS[index]}")
    return node


def ax_list(nodes, checked):
    need(len(nodes) == len(NAMES), "wrong number of Switch AX entries")
    for index, node in enumerate(nodes):
        ax_switch(node, index, checked[index])
    need(len({node["nodeId"] for node in nodes}) == len(NAMES), "duplicate AX node ID")


def name(shot):
    return f"{STEM}-{shot}.png"


def png(path, viewport):
    w, h = viewport
    with Image.open(path) as original:
        need(original.format == "PNG" and original.mode in ("RGB", "RGBA")
             and original.size == (w * SCALE, (h + HEADER) * SCALE), f"wrong PNG format or dimensions: {path.name}")
        original.verify()
    with Image.open(path) as original:
        return original.convert("RGB")


def sample(img, x, y):
    return img.getpixel((floor(x * SCALE), floor((y + HEADER) * SCALE)))


def icon_pixels(img, icon, target):
    x, y, w, h = icon
    points = (x * SCALE, (y + HEADER) * SCALE, (x + w) * SCALE, (y + HEADER + h) * SCALE)
    crop = img.crop(tuple(floor(value + .5) for value in points))
    return sum(count for count, color in crop.getcolors(crop.width * crop.height) if color == target)


def checked_pixels(image, g, palette, on_right, reported):
    primary = token(palette, "primary")
    track = g["track"]
    need(sample(image, track[0] + (42 if on_right else 10), center(track)[1]) == primary,
         "settled screenshot selected track color mismatch")
    count = icon_pixels(image, g["icon"], primary)
    need(type(reported) is int and count == reported and count > 16, "decoded icon ink disagrees with the report")
    return count


def layout_record(data, images):
    wide, compact = SHOTS[SHOT_WIDE], SHOTS[SHOT_COMPACT]
    ax = data["accessibility"]
    initial = state(ax["state"], START, 0)
    controls = [geometry(record, checked, size, icon, False, wide) for record, checked, size, icon in zip(
        ax["controls"], START, (16, 24, 24, 24, 24), (False, True, True, True, True), strict=True
    )]
    need(all(close(g["track"][0], controls[0]["track"][0]) for g in controls), "wide switch tracks do not align")
    ax_list(ax["nodes"], START)
    focus = ax_switch(ax["focused_node"], 2, False)
    need(ax_value(focus, "focused") is True and ax["wrap_reverse"] == IDS[2]
         and ax["tab_order"] == ["switch-demo-theme", "switch-demo-rtl", "switch-demo-large-text", *IDS[:3], "switch-demo-back"],
         "wrong focus or enabled-only Tab order")
    need(ax["capture"] == name(SHOT_WIDE), "wrong wide screenshot link")
    wide_png = images[SHOT_WIDE]
    need(sample(wide_png, *center(controls[0]["handle"])) == token(initial, "outline"), "off handle color mismatch")
    wide_ink = checked_pixels(wide_png, controls[1], initial, False, ax["selected_icon_primary_pixels"])

    actions = data["activation"]
    state(actions["space"]["state"], AFTER_SPACE, 1)
    state(actions["click"]["state"], AFTER_CLICK, 2)
    geometry(actions["space"]["geometry"], True, 24, False, False, wide)
    inactive = geometry(actions["click"]["geometry"], False, 16, False, False, wide)
    need(actions["space"]["id"] == IDS[0] and actions["click"]["id"] == IDS[1]
         and actions["disabled_pointer"] == IDS[3:], "wrong activation targets or disabled attempts")
    ax_list(actions["nodes_after"], AFTER_CLICK)

    press = data["press_geometry"]
    for key in ("outside_state", "canceled_state"):
        state(press[key], AFTER_CLICK, 2)
    stages = (("hover", 16), ("mouse_held", 28), ("mouse_release_outside", 16), ("touch_held", 28), ("touch_cancel", 16))
    stages = {key: geometry(press[key], False, size, False, False, wide) for key, size in stages}
    need(all(g["row"] == inactive["row"] and g["track"] == inactive["track"] for g in stages.values()),
         "a press moved the switch hit target or track")
    need(press["captures"] == [name("hovered"), name("held")], "wrong press screenshot links")
    x, y = center(stages["hover"]["handle"])
    pixels = {}
    for shot in ("hovered", "held"):
        key = "hover" if shot == "hovered" else "held"
        for point, dx in (("center", 0), ("edge", 11)):
            label = f"{key}_{point}"
            pixels[label] = sample(images[shot], x + dx, y)
            need(pixels[label] == rgb(press["paint"][label]), f"decoded {label} disagrees with the report")
    need(pixels["hover_center"] != pixels["hover_edge"] != pixels["held_edge"] == pixels["held_center"],
         "the held handle's visible footprint did not enlarge")

    narrow = data["compact"]
    dark = state(narrow["state"], AFTER_LABEL, 3, rtl=True)
    before = geometry(narrow["before"], False, 24, True, True, compact)
    selected = geometry(narrow["selected"], True, 24, True, True, compact)
    need(narrow["viewport"] == list(compact) and narrow["dark"] is True
         and narrow["wide_label_height"] == controls[2]["label"][3]
         and selected["label"][3] >= 96 and selected["label"][3] > 2 * narrow["wide_label_height"],
         "no wrapped 200% RTL label in the compact dark viewport")
    need(before["row"] == selected["row"] and before["label"] == selected["label"]
         and center(selected["handle"])[0] < center(before["handle"])[0], "wrong compact selection geometry")
    px, py = numbers(narrow["label_contact"], 2)
    label_x, label_y = center(selected["label"])
    need(close(px, label_x) and close(py - HEADER, label_y)
         and narrow["focus_restored"] == "switch-demo-show", "wrong label contact or restored launcher focus")
    need(narrow["capture"] == name(SHOT_COMPACT), "wrong compact screenshot link")
    ax_list(narrow["nodes"], AFTER_LABEL)
    dark_ink = checked_pixels(images[SHOT_COMPACT], selected, dark, True, narrow["icon_primary_pixels"])
    return {"role": "switch", "ax_names": NAMES, "disabled": IDS[3:], "tab_order": ax["tab_order"],
            "initial": START, "after_space": AFTER_SPACE, "after_click": AFTER_CLICK, "after_label": AFTER_LABEL,
            "effective_changes": 3, "resting_diameters": [16, 24], "held_diameter": 28, "rgba_source": "authored theme tokens",
            "compact_label_height": selected["label"][3], "wide_icon_ink_pixels": wide_ink,
            "compact_icon_ink_pixels": dark_ink, "press_pixels": pixels}


def validate(artifacts, log, profile, adapter, manifest, expected_source_sha, source_root=None):
    provenance = manifest_record(manifest, expected_source_sha, source_root)
    runner = runner_record(log, profile)
    report_path = artifacts / f"browser-{STEM}.json"
    report = json.loads(report_path.read_text())
    need(not (artifacts / name("failure")).exists(), "a failure screenshot is present; use a clean run directory")
    observed_adapter = adapter_record(report, runner, adapter)
    files = {shot: artifacts / name(shot) for shot in SHOTS}
    frames = {shot: png(path, SHOTS[shot]) for shot, path in files.items()}
    scans = {name(shot): {"pixels": list(frames[shot].size), "sha256": digest(path),
                          "decoded_rgb_sha256": sha256(frames[shot].tobytes()).hexdigest()}
             for shot, path in files.items()}
    need(len({entry["decoded_rgb_sha256"] for entry in scans.values()}) == len(SHOTS),
         "the captures reuse decoded pixels")
    return {"version": 1, "source": provenance, "runner": runner, "observed_adapter": observed_adapter,
            "report_sha256": digest(report_path), "page_errors": 0, "screenshots": scans,
            "switch": layout_record(report, frames)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--log", type=Path, required=True)
    parser.add_argument("--profile", choices=SUITES, required=True)
    parser.add_argument("--adapter", choices=("observed", "metal", "swiftshader"))
    parser.add_argument("--source-manifest", type=Path, required=True)
    parser.add_argument("--expected-source-sha256", required=True)
    parser.add_argument("--source-root", type=Path)
    args = parser.parse_args()
    adapter = args.adapter or {"stock-linux": "swiftshader", "stock-macos": "metal"}.get(args.profile, "observed")
    try:
        receipt = validate(args.artifacts, args.log, args.profile, adapter, args.source_manifest,
                           args.expected_source_sha256, args.source_root)
    except (KeyError, IndexError, OSError, TypeError, ValueError) as error:
        print(f"Switch verifier rejected evidence: {error}", file=sys.stderr)
        return 1
    print(json.dumps(receipt, ensure_ascii=False, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
