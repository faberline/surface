#!/usr/bin/env python3
"""Verify stock or focused Checkbox Chrome receipts offline."""

import argparse
from hashlib import sha256
import json
from math import floor, isfinite
from pathlib import Path, PurePosixPath
import re
import sys

from PIL import Image

STEM = "material-checkbox"
SOURCE_FILES = (
    "crates/surface-material/src/checkbox.rs",
    "e2e/src/material.rs",
    "e2e/src/material_checkbox.rs",
    "examples/material-gallery/src/checkbox_demo.rs",
    "examples/material-gallery/web/index.html",
)
CHECKBOX_STEPS = [f"M{i}" for i in range(278, 282)]
STOCK_STEPS = [f"M{i}" for i in range(1, 282) if i != 23]
SUITES = {
    "focused": ["material"],
    "material": ["material"],
    "stock-macos": ["smoke", "ime", "material", "bridge-web", "devtools-headless"],
    "stock-linux": [
        "smoke", "ime", "material", "bridge-native", "bridge-native-compact",
        "bridge-web", "devtools-native", "devtools-headless",
    ],
}
WIDE = (960, 800)
COMPACT = (320, 640)
SHOTS = {"wide-focused": WIDE, "wide-held": WIDE, "compact-invalid": COMPACT, "compact-resolved": COMPACT}
SCALE = 2
HEADER = 44
IDS = ["checkbox-demo-" + name for name in ("all", "notes", "attachments", "locked", "consent")]
NAMES = ["同步全部", "筆記", "附件", "已鎖定的專案", "允許在行動網路上同步大型附件"]
HELP_TEXT = "請勾選後再繼續。"
OFF, ON, MIXED = "unchecked", "checked", "mixed"
START = [MIXED, ON, OFF, MIXED, OFF]
ALL_ON = [ON, ON, ON, MIXED, OFF]
CHILD_OFF = [MIXED, OFF, ON, MIXED, OFF]
ALL_OFF = [OFF, OFF, OFF, MIXED, OFF]
ACCEPTED = [OFF, OFF, OFF, MIXED, ON]
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
    need(set(SOURCE_FILES) <= entries.keys(), "source manifest omits Checkbox runner, component, or screenshot source")
    need(len(entries) == 602, "wrong Checkbox source manifest file count")
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
    expected_steps = CHECKBOX_STEPS if profile == "focused" else STOCK_STEPS
    need(steps == expected_steps and counts["material"]["steps"] == len(expected_steps),
         "missing, extra or out-of-order Material step")
    need(counts["material"]["checks"] >= 250 and all(
        row["checks"] >= row["steps"] > 0 for row in counts.values()
    ), "runner check count is too low")
    total = sum(row["checks"] for row in counts.values())
    need(summary == [(str(len(counts)), "0", str(total))], "runner summary disagrees with suite totals")
    need(len(versions) == 1, "missing or ambiguous runner Chrome version")
    return {"profile": profile, "checkbox_steps": steps[-4:], "cases": counts, "total_checks": total,
            "browser": versions[0], "sha256": digest(path)}


def adapter_record(data, runner, expected):
    need(data["browser"] == runner["browser"] and data["errors"] == [] and data["reduced_motion"] is True
         and "failure" not in data, "browser receipt reports errors, wrong motion preference or a different Chrome")
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


def ink_pixels(img, icon, target):
    x, y, w, h = icon
    points = (x * SCALE, (y + HEADER) * SCALE, (x + w) * SCALE, (y + HEADER + h) * SCALE)
    crop = img.crop(tuple(floor(value + .5) for value in points))
    return sum(count for count, color in crop.getcolors(crop.width * crop.height) if color == target)


def geometry(g, rtl, viewport):
    row, art, mark, layer, ring, label = (rect(g[key]) for key in ("row", "art", "mark", "layer", "ring", "label"))
    need(48 <= row[2] <= 480.75 and row[3] >= 48 and contains([0, 0, *viewport], row)
         and all(contains(row, child) for child in (ring, label)), "art or label escaped the hit cell")
    need(art[2:] == [18, 18] and mark == art and layer[2:] == [40, 40] and ring[2:] == [50, 50],
         "wrong box, mark, layer or focus-ring size")
    need(all(close(a, b) for other in (layer, ring) for a, b in zip(center(art), center(other))),
         "the checkbox layers are not centered")
    if rtl:
        need(close(end(row), end(ring)) and close(ring[0] - end(label), 8),
             "RTL artwork is not at the right of the hit cell")
    else:
        need(close(row[0], ring[0]) and close(label[0] - end(ring), 8),
             "LTR artwork is not at the left of the hit cell")
    return g


def rgb(values, channels=3):
    need(type(values) is list and len(values) == channels
         and all(type(value) is int and 0 <= value <= 255 for value in values), "invalid RGB receipt")
    return tuple(values)


def state(data, expected, requests, rtl=False):
    need(type(data["requests"]) is int and data["requests"] == requests
         and data["values"] == expected and data["visible"] is True and data["rtl"] is rtl
         and type(data["text_scale"]) in (int, float) and data["text_scale"] == (2 if rtl else 1),
         "wrong fixture state, scale or callback count")
    return data["palette"]


def token(palette, key):
    *color, alpha = rgb(palette[key], 4)
    need(alpha == 255, f"{key} authored token has transparency")
    return tuple(color)


def ax_value(node, key):
    values = [prop["value"]["value"] for prop in node["properties"] if prop["name"] == key]
    need(len(values) <= 1, f"duplicate AX property {key}")
    return values[0] if values else None


def ax_checkbox(node, index, expected):
    expected_checked = {MIXED: "mixed", ON: "true", OFF: "false"}
    disabled = ax_value(node, "disabled")
    need(node["ignored"] is False and node["role"]["value"] == "checkbox" and node["name"]["value"] == NAMES[index]
         and ax_value(node, "checked") == expected_checked[expected]
         and (disabled is True if index == 3 else disabled is None or disabled is False),
         f"wrong Chrome AX checkbox state or name for {IDS[index]}")
    return node


def ax_list(nodes, states):
    need(len(nodes) == len(NAMES), "wrong number of Checkbox AX entries")
    for index, node in enumerate(nodes):
        ax_checkbox(node, index, states[index])
    need(len({node["nodeId"] for node in nodes}) == len(NAMES), "duplicate Checkbox AX node ID")


def description(node, report, invalid):
    raw_invalid = ax_value(node, "invalid")
    flag = raw_invalid is True or raw_invalid == "true"
    message = (node.get("description") or {}).get("value", "")
    need(flag is invalid and report["chrome_invalid"] is invalid and report["chrome_description"] == message,
         "Chrome AX does not agree with the invalid receipt")
    dom = report["dom"]
    if invalid:
        need(message == HELP_TEXT and dom["invalid"] == "true" and dom["text"] == HELP_TEXT
             and isinstance(dom["reference"], str) and bool(dom["reference"].strip()),
             "the invalid Checkbox description does not resolve to its help text")
    else:
        need(message == "" and dom == {"invalid": None, "reference": None, "text": None},
             "accepted Checkbox kept a stale error description")
    return {"invalid": flag, "description": message}


def matching_sample(image, report, key, x, y, expected=None):
    value = sample(image, x, y)
    need(value == rgb(report[key]), f"decoded {key} does not match the runner receipt")
    if expected is not None:
        need(value == expected, f"decoded {key} does not match the authored token")
    return value


def matching_ink(image, region, color, expected, minimum, label):
    count = ink_pixels(image, region, color)
    need(type(expected) is int and count == expected and count > minimum, f"decoded {label} ink differs from the receipt")
    return count


def wide_record(report, images):
    initial = report["accessibility"]
    palette = state(initial["state"], START, 0)
    ax_list(initial["nodes"], START)
    need(ax_value(initial["nodes"][0], "focused") is True
         and initial["tab_order_to_all"] == ["checkbox-demo-theme", "checkbox-demo-rtl", "checkbox-demo-large-text", IDS[0]],
         "the wide keyboard focus or Tab sequence changed")
    initial_error = description(initial["nodes"][4], initial["invalid"], True)
    ltr = [geometry(layout, False, WIDE) for layout in initial["layouts"]]
    need(len(ltr) == len(IDS) and all(close(g["ring"][0], ltr[0]["ring"][0]) for g in ltr),
         "wide Checkbox artwork columns do not align")
    need(initial["capture"] == name("wide-focused"), "wrong keyboard focus screenshot reference")

    image, paint = images["wide-focused"], initial["paint"]
    cx, cy = center(ltr[0]["art"])
    primary = token(palette, "primary")
    on_primary = token(palette, "on_primary")
    matching_sample(image, paint, "mixed_fill", cx, cy - 5, primary)
    mark = matching_sample(image, paint, "mixed_mark", cx, cy, on_primary)
    ring_color = matching_sample(image, paint, "focus_ring", cx, ltr[0]["ring"][1] + 1.5, token(palette, "secondary"))
    focus_layer = matching_sample(image, paint, "focus_layer", cx + 14, cy)
    background = matching_sample(image, paint, "focus_background", cx + 27, cy, token(palette, "surface"))
    need(focus_layer not in (background, primary), "keyboard focus did not paint a translucent layer")
    check_ink = matching_ink(image, ltr[1]["art"], on_primary, paint["checked_mark_pixels"], 16, "wide check mark")
    off = ltr[2]["art"]
    matching_sample(image, paint, "off_outline", center(off)[0], off[1] + 1, token(palette, "on_surface_variant"))

    keyboard = report["keyboard"]
    for name_, target, selected, requests, focus_index in (
        ("space", IDS[0], ALL_ON, 1, 0), ("enter", IDS[1], CHILD_OFF, 2, 1)
    ):
        action = keyboard[name_]
        need(action["id"] == target, f"wrong keyboard target for {name_}")
        state(action["state"], selected, requests)
        ax_list(action["nodes"], selected)
        need(ax_value(action["nodes"][focus_index], "focused") is True, f"{name_} lost Chrome AX focus")
    need(keyboard["next_tab"] == [IDS[2], IDS[4]] and keyboard["reverse_tab"] == IDS[2],
         "keyboard navigation visited the locked Checkbox")

    pointer = report["pointer"]
    need(pointer["disabled_pointer"] == IDS[3] and pointer["capture"] == name("wide-held"),
         "wrong disabled pointer target or held screenshot")
    for key, id_, values, requests in (
        ("label_click", IDS[1], ALL_ON, 3), ("pointer_held", IDS[0], ALL_ON, 3), ("box_click", IDS[0], ALL_OFF, 4)
    ):
        action = pointer[key]
        need(action["id"] == id_ and action["host"] == "surface", f"wrong physical pointer target or host for {key}")
        state(action["state"], values, requests)
    need(pointer["label_click"]["target"] == f"{IDS[1]}-label", "the Notes label did not receive the pointer")
    for event in ("mouse_release_outside", "touch_cancel"):
        state(pointer[event], ALL_ON, 3)
    ax_list(pointer["nodes_after"], ALL_OFF)

    held = pointer["pointer_held"]
    need(geometry(held["layout"], False, WIDE) == ltr[0], "press changed the Checkbox footprint")
    image = images["wide-held"]
    pressed_layer = matching_sample(image, held, "layer_rgb", cx + 14, cy)
    matching_sample(image, held, "background_rgb", cx + 27, cy, background)
    matching_sample(image, held, "ring_rgb", cx, ltr[0]["ring"][1] + 1.5, ring_color)
    need(pressed_layer not in (focus_layer, background), "physical press did not change the same painted layer")

    return ltr[4]["label"][3], {
        "values": {"initial": START, "space": ALL_ON, "enter": CHILD_OFF, "pointer_box": ALL_OFF},
        "locked": IDS[3], "initial_consent": initial_error, "wide_check_ink": check_ink, "mixed_mark_rgb": mark,
        "focus_ring_rgb": ring_color, "focused_layer_rgb": focus_layer, "pressed_layer_rgb": pressed_layer,
        "callback_counts": {"initial": 0, "space": 1, "enter": 2, "label": 3, "held": 3, "outside": 3, "canceled": 3, "box": 4},
    }


def compact_record(narrow, wide_label, images):
    need(narrow["viewport"] == list(COMPACT) and narrow["dark"] is True
         and type(narrow["scrolled_to_help"]) is bool and narrow["wide_label_height"] == wide_label,
         "wrong compact dark 200% RTL profile")
    invalid, resolved = narrow["invalid"], narrow["resolved"]
    before = geometry(invalid["layout"], True, COMPACT)
    after = geometry(resolved["layout"], True, COMPACT)
    palette = state(invalid["state"], ALL_OFF, 4, rtl=True)
    resolved_palette = state(resolved["state"], ACCEPTED, 5, rtl=True)
    need(palette == resolved_palette, "theme palette changed while resolving consent")
    need(invalid["capture"] == name("compact-invalid") and resolved["capture"] == name("compact-resolved"),
         "wrong compact screenshot references")
    clip, help_rect = rect(narrow["clip"]), rect(invalid["help"])
    need(contains([0, 0, *COMPACT], clip) and contains(clip, before["row"]) and contains(clip, after["row"])
         and contains(clip, help_rect) and help_rect[1] >= end(before["row"], 1),
         "the consent checkbox or help escaped the actual scroll clip")
    need(before["label"][3] >= 96 and before["label"][3] > 2 * wide_label
         and close(center(before["art"])[0], center(after["art"])[0]), "the RTL label did not wrap or its art column moved")
    contact_x, contact_y = numbers(resolved["label_contact"], 2)
    lx, ly = center(before["label"])
    need(close(contact_x, lx) and close(contact_y - HEADER, ly) and resolved["host"] == "surface"
         and narrow["focus_restored"] == "checkbox-demo-show", "wrong compact label contact, host or restored focus")
    before_node = ax_checkbox(invalid["node"], 4, OFF)
    after_node = ax_checkbox(resolved["node"], 4, ON)
    informed = description(before_node, invalid["semantics"], True)
    cleared = description(after_node, resolved["semantics"], False)

    image = images["compact-invalid"]
    surface = token(palette, "surface")
    error = token(palette, "error")
    text_ink = token(palette, "on_surface")
    need(max(surface) < 128 and sample(image, 1, 1) == surface
         and sample(images["compact-resolved"], 1, 1) == surface, "compact screenshots do not paint the dark surface")
    error_outline = matching_sample(image, invalid, "outline_rgb", center(before["art"])[0], before["art"][1] + 1, error)
    help_ink = matching_ink(image, help_rect, error, invalid["help_ink_pixels"], 24, "compact help")
    lx, ly, lw, _ = before["label"]
    need(len(invalid["line_ink_pixels"]) == 2, "expected two disjoint wrapped text samples")
    lines = [
        matching_ink(image, [lx, ly + 48 * line, lw, 48], text_ink, count, 24, f"wrapped label line {line + 1}")
        for line, count in enumerate(invalid["line_ink_pixels"])
    ]

    image = images["compact-resolved"]
    x, y = center(after["art"])
    fill = matching_sample(image, resolved, "fill_rgb", x, y - 5, token(palette, "primary"))
    tick = matching_ink(image, after["art"], token(palette, "on_primary"), resolved["check_ink_pixels"], 16, "resolved check mark")
    return {"viewport": COMPACT, "text_scale": 2, "rtl": True, "label_height": before["label"][3],
            "scrolled_to_help": narrow["scrolled_to_help"], "invalid": informed, "resolved": cleared,
            "compact_values": ACCEPTED, "callbacks_before": 4, "callbacks_after": 5, "error_outline": error_outline,
            "resolved_fill": fill, "help_ink": help_ink, "wrapped_line_ink": lines, "resolved_check_ink": tick}


def validate(artifacts, log, profile, adapter, manifest, expected_source_sha, source_root=None):
    stock_adapter = {"stock-linux": "swiftshader", "stock-macos": "metal"}.get(profile)
    need(stock_adapter is None or adapter == stock_adapter, "the stock profile requires its platform adapter")
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
    wide_label, wide = wide_record(report, frames)
    compact = compact_record(report["compact"], wide_label, frames)
    return {"version": 1, "evidence_scope": "focused-diagnostic" if profile == "focused" else profile,
            "source": provenance, "runner": runner, "observed_adapter": observed_adapter,
            "report_sha256": digest(report_path), "page_errors": 0, "screenshots": scans,
            "checkbox": {"role": "checkbox", "ax_names": NAMES, "wide": wide, "compact": compact}}


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
        print(f"Checkbox verifier rejected evidence: {error}", file=sys.stderr)
        return 1
    print(json.dumps(receipt, ensure_ascii=False, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
