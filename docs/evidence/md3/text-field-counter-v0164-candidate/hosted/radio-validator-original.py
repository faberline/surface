#!/usr/bin/env python3
"""Verify stock or focused Surface Radio Chrome evidence offline."""

import argparse
from hashlib import sha256
import json
from math import floor, isfinite
from pathlib import Path, PurePosixPath
import re
import sys

from PIL import Image

STEM = "material-radio"
REPORT = f"browser-{STEM}.json"
SOURCE_FILES = (
    "crates/surface-app/src/controls.rs",
    "crates/surface-app/src/lib.rs",
    "crates/surface-material/src/radio.rs",
    "e2e/src/material.rs",
    "e2e/src/material_radio.rs",
    "examples/material-gallery/src/radio_demo.rs",
    "examples/material-gallery/web/index.html",
)
RADIO_STEPS = [f"M{i}" for i in range(282, 286)]
STOCK_STEPS = [f"M{i}" for i in range(1, 286) if i != 23]
SUITES = {
    "focused": ["material"],
    "material": ["material"],
    "stock-macos": ["smoke", "ime", "material", "bridge-web", "devtools-headless"],
    "stock-linux": [
        "smoke", "ime", "material", "bridge-native", "bridge-native-compact",
        "bridge-web", "devtools-native", "devtools-headless",
    ],
}
DEFAULT_ADAPTER = {"stock-macos": "metal", "stock-linux": "swiftshader"}
SWIFTSHADER = {"vendor": "google", "architecture": "swiftshader", "isFallbackAdapter": True}
SCALE, HEADER = 2, 44
WIDE, COMPACT = (960, 800), (320, 640)
FOCUSED, HORIZONTAL, HELD, DARK = "wide-focused", "wide-horizontal", "wide-held", "compact-dark-rtl-200"
SHOTS = {FOCUSED: WIDE, HORIZONTAL: WIDE, HELD: WIDE, DARK: COMPACT}
DAILY, LOCKED, WEEKLY, MONTHLY = IDS = tuple(f"radio-demo-{name}" for name in ("daily", "locked", "weekly", "monthly"))
NAMES = ("每天", "即時通知", "每週一通知", "每月")
GROUP = "通知頻率"
TAB_ORDER = ["radio-demo-theme", "radio-demo-rtl", "radio-demo-large-text", "radio-demo-layout", WEEKLY, "radio-demo-clear"]
LTR_ARROWS = [
    ("ArrowRight", MONTHLY), ("ArrowDown", DAILY), ("ArrowRight", WEEKLY),
    ("ArrowUp", DAILY), ("ArrowLeft", MONTHLY),
]
RTL_ARROWS = [("ArrowRight", WEEKLY), ("ArrowLeft", MONTHLY), ("ArrowUp", WEEKLY)]
TRACE = [{"trusted": True, "repeat": repeated} for repeated in (True, False, True)]
TOKENS = ("surface", "on_surface", "on_surface_variant", "primary", "secondary")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def manifest_record(path, expected_sha, source_root):
    require(digest(path) == expected_sha, "source manifest SHA-256 mismatch")
    entries = {}
    for line in path.read_text().splitlines():
        match = re.fullmatch(r"([a-f0-9]{64})  (.+)", line)
        require(match is not None, "invalid source manifest line")
        checksum, name = match.groups()
        relative = PurePosixPath(name)
        require(not relative.is_absolute() and ".." not in relative.parts
                and relative.as_posix() == name and name not in entries,
                f"unsafe or duplicate source path: {name}")
        entries[name] = checksum
    require(set(SOURCE_FILES) <= entries.keys(), "source manifest omits required Radio or canvas inputs")
    if source_root is not None:
        root = source_root.resolve(strict=True)
        for name, checksum in entries.items():
            source = (root / name).resolve(strict=True)
            require(source.is_relative_to(root) and digest(source) == checksum, f"source input mismatch: {name}")
    return {"manifest_sha256": expected_sha, "files": len(entries), "source_bytes_checked": source_root is not None,
            "radio_inputs": {name: entries[name] for name in SOURCE_FILES}}


def runner_record(log, profile):
    raw = log.read_text()
    raw = re.sub(r"^(?:(?:[^\t\n]+\t){2})?\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z ",
                 "", raw, flags=re.M)
    started = re.findall(r"^case (\S+) \.\.\.$", raw, re.M)
    passed = re.findall(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks, \d+(?:\.\d+)?s\)$", raw, re.M)
    summary = re.findall(r"^e2e result: (\d+) passed; (\d+) failed; (\d+) checks;", raw, re.M)
    steps = re.findall(r"^  \[(M\d+)\]", raw, re.M)
    versions = re.findall(r"^browser: ((?:Chrome|Chromium)/\d+(?:\.\d+)+)\b", raw, re.M)
    require(started == [row[0] for row in passed] == SUITES[profile]
            and not re.search(r"^case \S+ \.\.\. FAILED", raw, re.M), "wrong suite sequence or a failed suite")
    cases = {suite: {"steps": int(step), "checks": int(count)} for suite, step, count in passed}
    expected_steps = RADIO_STEPS if profile == "focused" else STOCK_STEPS
    require(steps == expected_steps and cases["material"]["steps"] == len(expected_steps),
            "missing, extra or out-of-order Material step")
    require(all(item["checks"] >= item["steps"] > 0 for item in cases.values()), "impossible check or step count")
    require(cases["material"]["checks"] >= 100, "the Material report has too few checks for the Radio journey")
    total = sum(item["checks"] for item in cases.values())
    require(summary == [(str(len(cases)), "0", str(total))], "runner summary disagrees with suite totals")
    require(len(versions) == 1, "missing or ambiguous Chrome version")
    return {"profile": profile, "browser": versions[0], "cases": cases, "material_steps": len(steps),
            "radio_steps": steps[-4:], "total_checks": total, "sha256": digest(log)}


def adapter_record(report, runner, expected):
    require(report["browser"] == runner["browser"] and report["errors"] == []
            and report["reduced_motion"] is True and "failure" not in report,
            "failed Radio report, page errors, browser version or missing Reduce Motion")
    info = report["adapter"]
    require(type(info["isFallbackAdapter"]) is bool
            and all(isinstance(info[key], str) for key in ("vendor", "architecture")), "invalid WebGPU adapter metadata")
    if expected == "metal":
        require(info["vendor"] == "apple" and info["architecture"].startswith("metal")
                and info["isFallbackAdapter"] is False, "expected actual Apple Metal adapter")
    elif expected == "swiftshader":
        require(info == SWIFTSHADER, "expected actual SwiftShader adapter")
    return info


def numbers(values, count):
    require(type(values) is list and len(values) == count
            and all(type(item) in (int, float) and isfinite(item) for item in values), "invalid geometry")
    return values


def rect(values, allow_empty=False):
    values = numbers(values, 4)
    require(min(values[2:]) > 0 or (allow_empty and values[2:] == [0, 0]), "invalid rectangle size")
    return values


def close(a, b):
    return abs(a - b) <= 0.75


def right(bounds):
    return bounds[0] + bounds[2]


def bottom(bounds):
    return bounds[1] + bounds[3]


def center(bounds):
    return bounds[0] + bounds[2] / 2, bounds[1] + bounds[3] / 2


def contains(outer, inner):
    return all(outer[axis] - 0.75 <= inner[axis]
               and inner[axis] + inner[axis + 2] <= outer[axis] + outer[axis + 2] + 0.75 for axis in (0, 1))


def geometry(data, checked, viewport, rtl=False):
    g = {key: rect(data[key], allow_empty=key == "dot")
         for key in ("row", "ring", "dot", "layer", "focus_ring", "label")}
    expected = {"ring": 20, "dot": 10 if checked else 0, "layer": 40, "focus_ring": 30}
    require(all(g[key][2:] == [size, size] for key, size in expected.items()), "wrong Radio ring, dot, layer or focus size")
    row, label = g["row"], g["label"]
    require(row[2] >= 48 and row[3] >= 48 and contains([0, 0, *viewport], row)
            and all(contains(row, shape) for shape in g.values()), "Radio art, label or 48 dp target escapes its row or viewport")
    x, y = center(g["ring"])
    require(all(close(cx, x) and close(cy, y) for cx, cy in map(center, (g["dot"], g["layer"], g["focus_ring"])))
            and close(y, center(row)[1]), "Radio circles have different centers")
    alignment = (right(row) - x, x - right(label)) if rtl else (x - row[0], label[0] - x)
    require(close(alignment[0], 24) and close(alignment[1], 32), "wrong logical Radio artwork placement or label gap")
    return g


def geometries(records, selected, viewport):
    return [geometry(g, item == selected, viewport) for g, item in zip(records, IDS, strict=True)]


def rgb(parts, count=3):
    require(type(parts) is list and len(parts) == count
            and all(type(value) is int and 0 <= value <= 255 for value in parts), "invalid color channels")
    return tuple(parts)


def palette_tokens(palette):
    tokens = {}
    for key in TOKENS:
        value = rgb(palette[key], 4)
        require(value[-1] == 255, f"{key} has a nonopaque token")
        tokens[key] = value[:3]
    return tokens


def state(data, selected, requests, *, rtl=False, horizontal=False, palette=None):
    require(data["selected"] == selected and type(data["requests"]) is int and data["requests"] == requests,
            f"wrong selected Radio or delivered request count: expected {selected!r}, {requests}")
    require(data["visible"] is True and data["rtl"] is rtl and data["horizontal"] is horizontal
            and type(data["text_scale"]) in (int, float) and data["text_scale"] == (2 if rtl else 1),
            "wrong Radio fixture theme-independent options")
    tokens = palette_tokens(data["palette"])
    if palette is not None:
        require(tokens == palette, "Radio palette drift within a single theme")
    return tokens


def ax_prop(node, key):
    values = [prop["value"]["value"] for prop in node["properties"] if prop["name"] == key]
    require(len(values) <= 1, f"duplicate Chrome AX property: {key}")
    return values[0] if values else None


def ax_role(node, role, name):
    require(node["ignored"] is False and node["role"]["value"] == role and node["name"]["value"] == name,
            f"wrong Chrome AX role or name: {role} {name}")


def ax_radio(node, ident, selected, focused=False):
    ax_role(node, "radio", NAMES[IDS.index(ident)])
    require(ax_prop(node, "checked") == ("true" if selected else "false"), f"wrong Chrome checked state: {ident}")
    disabled = ax_prop(node, "disabled")
    require(disabled is True if ident == LOCKED else disabled is False or disabled is None,
            f"wrong Chrome disabled state: {ident}")
    if focused:
        require(ax_prop(node, "focused") is True, f"missing actual projected Chrome focus: {ident}")


def ax_children(tree, checked, focused=None):
    group, nodes = tree["group"], tree["nodes"]
    ax_role(group, "radiogroup", GROUP)
    for ident, node in zip(IDS, nodes, strict=True):
        ax_radio(node, ident, ident == checked, ident == focused)
    child_ids = [node["nodeId"] for node in nodes]
    require(len(set(child_ids)) == len(IDS) and all(child in group["childIds"] for child in child_ids),
            "named radios do not have distinct IDs under the same Chrome AX group")


def arrows(receipt, expected, start_count, palette, rtl=False):
    for offset, (entry, (key, target)) in enumerate(zip(receipt, expected, strict=True), 1):
        require(entry["key"] == key and entry["target"] == target, "wrong arrow key, RTL direction or target")
        state(entry["state"], target, start_count + offset, rtl=rtl, palette=palette)


def filename(shot):
    return f"{STEM}-{shot}.png"


def png(path, viewport):
    width, height = viewport
    with Image.open(path) as original:
        require(original.format == "PNG" and original.mode in ("RGB", "RGBA")
                and original.size == (width * SCALE, (height + HEADER) * SCALE), f"wrong PNG format or dimensions: {path.name}")
        original.verify()
    with Image.open(path) as original:
        original.load()
        return original.convert("RGB")


def pixel(img, x, y):
    return img.getpixel((floor(x * SCALE), floor((y + HEADER) * SCALE)))


def label_pixels(img, label, color):
    x, y, width, _ = label
    counts = []
    for row in range(2):
        y0 = y + 48 * row
        points = [SCALE * x, SCALE * (y0 + HEADER), SCALE * (x + width), SCALE * (y0 + HEADER + 48)]
        box = tuple(floor(n + 0.5) for n in points)
        require(0 <= box[0] < box[2] <= img.width and 0 <= box[1] < box[3] <= img.height,
                "label bands escape the actual PNG")
        crop = img.crop(box)
        counts.append(sum(n for n, c in crop.getcolors(crop.width * crop.height) if c == color))
    return counts


def paint_match(paint, label, sample, expected=None):
    require(sample == rgb(paint[label]), f"decoded Radio {label} disagrees with the raw report")
    if expected is not None:
        require(sample == expected, f"decoded Radio {label} disagrees with the intended theme token")


def focus_ink(image, g):
    return pixel(image, center(g["ring"])[0], g["focus_ring"][1] + 1.5)


def ring_ink(image, g):
    return pixel(image, center(g["ring"])[0], g["ring"][1] + 1)


def wide(data, image):
    colors = state(data["state"], WEEKLY, 0)
    g = geometries(data["layouts"], WEEKLY, WIDE)
    ax_children(data["accessibility"], WEEKLY, WEEKLY)
    require(data["tab_order"] == TAB_ORDER and data["reverse_tab"] == WEEKLY,
            "checked option did not receive the only Tab stop in both entry directions")
    require(data["capture"] == filename(FOCUSED), "wrong wide focused screenshot link")
    checked, off = g[2], g[0]
    paint = data["paint"]
    paint_match(paint, "indicator", ring_ink(image, checked), colors["primary"])
    paint_match(paint, "dot", pixel(image, *center(checked["dot"])), colors["primary"])
    paint_match(paint, "focus_ring", focus_ink(image, checked), colors["secondary"])
    paint_match(paint, "off_ring", ring_ink(image, off), colors["on_surface_variant"])
    paint_match(paint, "off_center", pixel(image, *center(off["ring"])), colors["surface"])
    return colors, checked["label"][3]


def check_keyboard(data, image, colors):
    arrows(data["arrows"], LTR_ARROWS, 0, colors)
    ax_children(data["selected_ax"], MONTHLY)
    state(data["selected_space"], MONTHLY, 5, palette=colors)
    state(data["cleared"], None, 5, palette=colors)
    ax_children(data["cleared_ax"], None)
    require(data["empty_entry"] == {"forward": DAILY, "reverse": MONTHLY}, "wrong first or last Tab target in the empty group")
    space = data["space"]
    state(space["ignored_repeat"], None, 5, palette=colors)
    state(space["normal"], DAILY, 6, palette=colors)
    state(space["after_selection_repeat"], DAILY, 6, palette=colors)
    require(space["native_events"] == TRACE
            and all(type(event[key]) is bool for event in space["native_events"] for key in ("trusted", "repeat")),
            "missing trusted Space and held repeat events")
    horizontal = data["horizontal"]
    state(horizontal["state"], DAILY, 6, horizontal=True, palette=colors)
    ax_children(horizontal["accessibility"], DAILY)
    require(horizontal["capture"] == filename(HORIZONTAL), "wrong horizontal screenshot link")
    group = rect(horizontal["group"])
    rows = geometries(horizontal["layouts"], DAILY, WIDE)
    require(all(contains(group, g["row"]) for g in rows), "horizontal Radio targets escape the group")
    pairs = list(zip(rows, rows[1:]))
    same_row = lambda before, after: close(before[1], after[1]) and after[0] >= right(before) - 0.5
    require(any(same_row(before["row"], after["row"]) for before, after in pairs)
            and all(same_row(before["row"], after["row"]) or after["row"][1] >= bottom(before["row"]) - 0.5
                    for before, after in pairs), "Radio options fail to form a horizontal ordered layout")
    selected = pixel(image, *center(rows[0]["dot"]))
    require(selected == rgb(horizontal["dot"]) == colors["primary"], "decoded horizontal selected dot mismatch")


def check_pointer(data, image, colors):
    label, whole, held, disabled = (data[name] for name in ("label", "whole_row", "held", "disabled"))
    for step, ident in ((label, WEEKLY), (whole, MONTHLY), (held, DAILY)):
        require(step["id"] == ident and step["host"] == "surface", f"wrong pointer target or canvas focus host: {ident}")
    require(disabled["id"] == LOCKED, "wrong disabled pointer target")
    state(label["state"], WEEKLY, 7, palette=colors)
    state(data["same_label_and_ring"], WEEKLY, 7, palette=colors)
    for record in (whole["state"], held["state"], held["outside"], held["touch_cancel"], disabled["state"]):
        state(record, MONTHLY, 8, palette=colors)
    ax_children(data["accessibility"], MONTHLY)
    gx, gy = numbers(whole["contact"], 2)
    gy -= HEADER
    gap = geometry(whole["layout"], False, WIDE)
    require(right(gap["layer"]) < gx < gap["label"][0] and close(gy, center(gap["ring"])[1])
            and contains(gap["row"], [gx, gy, 1, 1]), "pointer is outside the full target gap")
    require(held["capture"] == filename(HELD), "wrong held screenshot link")
    g = geometry(held["layout"], False, WIDE)
    x, y = center(g["ring"])
    paint = held["paint"]
    layer = pixel(image, x + 17, y)
    paint_match(paint, "layer", layer)
    background = pixel(image, x + 23, y)
    paint_match(paint, "background", background, colors["surface"])
    paint_match(paint, "indicator", ring_ink(image, g), colors["on_surface"])
    paint_match(paint, "focus_ring", focus_ink(image, g), colors["secondary"])
    # The browser case checks translucency. Allow differences in RGB blending
    # space across adapters, while requiring every mixed channel to stay between
    # the authored surface and ink and the point to differ from both endpoints.
    primary = colors["primary"]
    require(layer not in (primary, background) and all(min(a, b) - 1 <= c <= max(a, b) + 1
            for a, b, c in zip(primary, background, layer)), "held Radio layer is not translucent Primary over Surface")
    return {"hosts": {step["id"]: step["host"] for step in (label, whole, held)},
            "gap_contact_canvas": [gx, gy], "held_layer": list(layer), "outside_requests": 8,
            "touch_cancel_requests": 8, "disabled_requests": 8}


def check_compact(data, image, light, wide_label_height):
    colors = state(data["state"], WEEKLY, 11, rtl=True)
    require(data["viewport"] == list(COMPACT) and data["dark"] is True
            and colors["surface"] != light["surface"] and sum(colors["surface"]) < sum(light["surface"])
            and data["capture"] == filename(DARK), "wrong compact screenshot, viewport or dark palette")
    arrows(data["arrows"], RTL_ARROWS, 8, colors, rtl=True)
    g = geometry(data["layout"], True, COMPACT, rtl=True)
    height = g["label"][3]
    require(data["wide_label_height"] == wide_label_height and height >= 96 and height > 2 * wide_label_height
            and contains(rect(data["clip"]), g["row"]), "compact large RTL label did not wrap inside its scroll clip")
    paint = data["paint"]
    paint_match(paint, "surface", pixel(image, 1, 1), colors["surface"])
    paint_match(paint, "dot", pixel(image, *center(g["dot"])), colors["primary"])
    paint_match(paint, "indicator", ring_ink(image, g), colors["primary"])
    paint_match(paint, "focus_ring", focus_ink(image, g), colors["secondary"])
    ink_counts = label_pixels(image, g["label"], colors["on_surface"])
    require(ink_counts == paint["label_pixels"] and min(ink_counts) > 24, "decoded compact text ink differs or a wrapped line is empty")
    ax = data["accessibility"]
    ax_role(ax["group"], "radiogroup", GROUP)
    ax_radio(ax["radio"], WEEKLY, True, True)
    require(ax["radio"]["nodeId"] in ax["group"]["childIds"], "focused compact radio is outside its named AX group")
    require(data["focus_restored"] == "radio-demo-show", "launcher did not regain projected focus")
    return {"viewport": data["viewport"], "label_height": height, "label_pixels": ink_counts,
            "focus_restored": data["focus_restored"], "arrows": [list(row) for row in RTL_ARROWS]}


def validate(artifacts, log, profile, adapter, manifest, expected_sha, source_root=None):
    required_adapter = DEFAULT_ADAPTER.get(profile, adapter or "observed")
    require(adapter is None or adapter == required_adapter, "stock profile adapter cannot be overridden")
    source = manifest_record(manifest, expected_sha, source_root)
    runner = runner_record(log, profile)
    path = artifacts / REPORT
    report = json.loads(path.read_text())
    observed_adapter = adapter_record(report, runner, required_adapter)
    shots = {name: artifacts / filename(name) for name in SHOTS}
    require(set(shots.values()) == set(artifacts.glob(f"{STEM}-*.png")),
            "wrong Radio PNG set or a failure screenshot is present")
    images = {name: png(path, SHOTS[name]) for name, path in shots.items()}
    records = {path.name: {"size": list(images[name].size), "sha256": digest(path),
                           "decoded_rgb_sha256": sha256(images[name].tobytes()).hexdigest()}
               for name, path in shots.items()}
    require(len({item["decoded_rgb_sha256"] for item in records.values()}) == len(SHOTS), "Radio screenshots reuse decoded pixels")
    palette, height = wide(report["accessibility"], images[FOCUSED])
    check_keyboard(report["keyboard"], images[HORIZONTAL], palette)
    pointer = check_pointer(report["pointer"], images[HELD], palette)
    compact = check_compact(report["compact"], images[DARK], palette, height)
    return {"version": 1, "source": source, "runner": runner, "adapter": observed_adapter,
            "report_sha256": digest(path), "page_errors": 0, "reduced_motion": True,
            "pngs": records, "radio": {"role": "radio", "group": GROUP, "names": list(NAMES),
            "keyboard_focused_chrome_ax": WEEKLY, "tab_order": TAB_ORDER,
            "empty_entry": {"forward": DAILY, "reverse": MONTHLY}, "disabled": LOCKED,
            "target_min_dp": 48, "ring_dp": 20, "dot_dp": 10, "state_layer_dp": 40, "focus_dp": 30,
            "trusted_space_events": TRACE, "final_request_count": 11, "pointer": pointer, "compact": compact}}


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
    try:
        receipt = validate(args.artifacts, args.log, args.profile, args.adapter, args.source_manifest,
                           args.expected_source_sha256, args.source_root)
    except (KeyError, IndexError, OSError, TypeError, ValueError) as error:
        print(f"Radio evidence rejected: {error}", file=sys.stderr)
        return 1
    print(json.dumps(receipt, ensure_ascii=False, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
