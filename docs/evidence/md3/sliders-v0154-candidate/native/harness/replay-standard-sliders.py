#!/usr/bin/env python3
"""Check native Standard Slider rendering through production headless transports."""

import argparse
import json
from math import ceil, floor, isclose
import os
from pathlib import Path
import re
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, find, focus, ink_sample, key, layout, node, settle, visible
from window_check import onscreen_windows

PREFIX = "slider-demo-"
SHOW, BACK, THEME, RTL, TEXT, SCROLL, ROUTE = (
    PREFIX + value for value in ("show", "back", "theme", "rtl", "large-text", "scroll", "route")
)
XS, S, M, L, XL, DISABLED = (
    PREFIX + value for value in ("xs", "s", "m", "l", "xl", "disabled")
)
PRESETS = {
    XS: ("特小連續", 16, 44, 25., 5.),
    S: ("小型階梯", 24, 44, 50., 12.5),
    M: ("中型滑桿", 40, 52, 50., 10.),
    L: ("大型滑桿", 56, 68, 50., 25.),
    XL: ("特大滑桿", 96, 108, 50., 10.),
    DISABLED: ("停用滑桿", 24, 44, 50., 12.5),
}
BINARIES = {"surface-material-gallery": "gallery", "surface-devtools": "cli"}
VIEWPORTS = {"wide": (960, 1040), "compact": (320, 880)}
CAPTURES = {"wide-light-sliders.png", "wide-light-focused-slider.png",
            "compact-dark-rtl-200-sliders.png", "compact-dark-rtl-200-xl.png"}


def nearly(actual, expected, context):
    require(isclose(actual, expected, abs_tol=0.002), f"{context}: expected {expected}, got {actual}")


def edge(rect, axis):
    return rect[axis] + rect[{"x": "width", "y": "height"}[axis]]


def midpoint(rect, axis):
    return (rect[axis] + edge(rect, axis)) / 2


def ranged(frame, target, expected):
    item = node(frame, target)
    name, _, _, _, step = PRESETS[target]
    require(item["name"] == name and item["role"] == "Slider"
            and item.get("disabled", False) is (target == DISABLED), f"Wrong slider semantics: {target}")
    value = item["range"]
    require(value == {"min": 0., "max": 100., "value": expected, "step": step},
            f"Unexpected authored slider range for {target}: {value}")
    return value


def geometry(frame, target, value, rtl, *, engaged=False):
    ranged(frame, target, value)
    _, thickness, handle_height, _, _ = PRESETS[target]
    hit = visible(frame, target)
    parts = {suffix: layout(frame, f"{target}-{suffix}")["bounds"]
             for suffix in ("rail", "active", "inactive", "handle", "focus", "stop")}
    rail, active, inactive, handle, ring, stop = parts.values()
    require(hit["height"] == max(48, handle_height) and rail["height"] == hit["height"],
            f"Wrong fixed cross-axis hit size: {target}")
    nearly(rail["x"], hit["x"] + 12, f"{target} leading rail")
    nearly(edge(rail, "x"), edge(hit, "x") - 12, f"{target} trailing rail")
    nearly(midpoint(handle, "x"), rail["x"] + rail["width"] * ((100 - value if rtl else value) / 100),
           f"{target} thumb position")
    nearly(midpoint(handle, "y"), midpoint(rail, "y"), f"{target} handle center")
    require((handle["width"], handle["height"]) == (2 if engaged else 4, handle_height),
            f"Wrong handle dimensions: {target}")
    require((ring["width"], ring["height"]) == (handle["width"] + 10, handle_height + 10),
            f"Wrong focus ring dimensions: {target}")
    for cap in (active, inactive):
        if cap["width"] > 0:
            require(cap["height"] == thickness, f"Wrong track thickness: {target}")
            nearly(midpoint(cap, "y"), midpoint(handle, "y"), f"{target} track center")
    if active["width"] > 0:
        nearly(active["x"] if not rtl else edge(active, "x"),
               rail["x"] if not rtl else edge(rail, "x"), f"{target} active endpoint")
        gap = active["x"] - edge(handle, "x") if rtl else handle["x"] - edge(active, "x")
        nearly(gap, 6, f"{target} active track gap")
    if inactive["width"] > 0:
        nearly(inactive["x"] if rtl else edge(inactive, "x"),
               rail["x"] if rtl else edge(rail, "x"), f"{target} inactive endpoint")
        gap = handle["x"] - edge(inactive, "x") if rtl else inactive["x"] - edge(handle, "x")
        nearly(gap, 6, f"{target} inactive track gap")
        require(stop["width"] == stop["height"] == 4, f"Wrong end stop size: {target}")
        inset = stop["x"] - inactive["x"] if rtl else edge(inactive, "x") - edge(stop, "x")
        nearly(inset, 4, f"{target} end stop inset")
    else:
        require(stop["width"] == stop["height"] == 0, f"Stop visible on a collapsed inactive track: {target}")
    return {"hit": hit, **parts}


def pixels(rows, x, y):
    require(0 <= x < len(rows[0]) // 4 and 0 <= y < len(rows), f"PNG sample outside viewport: {(x, y)}")
    return list(rows[y][4 * x:4 * (x + 1)])


def inside_ink(rows, bounds, color):
    xs = range(ceil(bounds["x"]), floor(edge(bounds, "x")))
    ys = range(ceil(bounds["y"]), floor(edge(bounds, "y")))
    return sum(pixels(rows, x, y) == color for x in xs for y in ys)


def check_paint(rows, frame, target, shape, *, focused=False):
    values = {}
    for suffix in ("active", "inactive", "handle", "stop"):
        area = shape[suffix]
        require(area["width"] > 0 and area["height"] > 0, f"{target}-{suffix} absent from test frame")
        x = floor(midpoint(area, "x"))
        y = floor(area["y"] + area["height"] * (0.28 if suffix in ("active", "inactive") else 0.5))
        actual = pixels(rows, x, y)
        expected = node(frame, f"{target}-{suffix}")["style"]["background"]
        if target == DISABLED:
            background = node(frame, ROUTE)["style"]["background"]
            require(expected[3] in (31, 97) and actual != background and actual[3] == 255,
                    f"Disabled {suffix} not visibly painted with a muted color")
        else:
            require(actual == expected, f"{target}-{suffix} decoded ink {actual} != {expected}")
        values[suffix] = {"point": [x, y], "rgba": actual, "authored": expected}
    if focused:
        label = visible(frame, f"{target}-label")
        text_key = f"{target}-label-text"
        ring = visible(frame, f"{target}-focus")
        require(label["x"] >= shape["hit"]["x"] and edge(label, "x") <= edge(shape["hit"], "x"),
                "The indicator left its owner's horizontal bounds")
        nearly(ring["y"] - edge(label, "y"), 4, "indicator clearance above outer focus ring")
        rgba = node(frame, f"{target}-focus")["focus_color"]
        focus_pixels = inside_ink(rows, ring, rgba) - inside_ink(rows, shape["handle"], rgba)
        require(focus_pixels >= 40, f"Missing visible focus stroke: {target}")
        label_node = node(frame, f"{target}-label")
        x, y = floor(midpoint(label, "x")), floor(label["y"] + 8)
        require(pixels(rows, x, y) == label_node["style"]["background"],
                "No decoded value indicator fill")
        values["focus_pixels"] = focus_pixels
        values["label"] = {"text": node(frame, text_key)["text"], "fill_rgba": pixels(rows, x, y),
                           **ink_sample(frame, rows, text_key)}
    return values


def capture_paint(app, plan, title, frame, shapes, *, focused=False):
    shot = capture(app, plan, title)
    width, height, rows = read_png((plan.out / shot["name"]).read_bytes())
    require((width, height) in VIEWPORTS.values(), "Wrong decoded native capture size")
    shot["paint"] = {target: check_paint(rows, frame, target, shape, focused=focused)
                     for target, shape in shapes.items()}
    return shot


def set_value(app, slider, number, expected):
    app.wire("Action.setValue", {"id": slider, "value": number})
    frame = settle(app)
    focus(frame, slider)
    ranged(frame, slider, expected)
    return frame


def key_stops(app, slider, rtl):
    initial, increment = PRESETS[slider][3:]
    increase, decrease = ("left", "right") if rtl else ("right", "left")
    records = []
    for press, expected in ((increase, initial + increment), (decrease, initial),
                            ("up", initial + increment), ("down", initial),
                            ("home", 0.), (decrease, 0.), ("end", 100.), (increase, 100.)):
        frame = key(app, press)
        focus(frame, slider)
        records.append({"key": press, "value": ranged(frame, slider, expected)["value"]})
        if expected in (0., 100.):
            geometry(frame, slider, expected, rtl, engaged=True)
    return records


def ticks(frame, target, shape, rtl):
    marks = []
    rail = shape["rail"]
    for index in range(8):
        child = f"{target}-tick-{index}"
        dot = layout(frame, child)["bounds"]
        if dot["width"] == 0:
            continue
        require(dot["width"] == dot["height"] == 4, f"Wrong discrete dot bounds: {child}")
        logical = 6. if index == 0 else rail["width"] * index / 8
        expected = edge(rail, "x") - logical if rtl else rail["x"] + logical
        nearly(midpoint(dot, "x"), expected, f"{child} coordinate")
        marks.append({"index": index, "bounds": dot})
    require(len(marks) >= 4 and 4 not in {mark["index"] for mark in marks},
            "Visible discrete dots are missing or fill the active handle gap")
    return marks


def sample_windows(app, previous):
    count = onscreen_windows(app.proc.pid)
    previous.append(count)
    require(count == 0, "The launched Gallery created a visible window")


def replay_profile(plan, profile):
    width, height = VIEWPORTS[profile]
    rtl = profile == "compact"
    observations = []
    result = {"viewport": [width, height], "rtl": rtl, "text_scale": 2 if rtl else 1}
    with launch(plan, profile, width, height) as app:
        sample_windows(app, observations)
        result["opening_tabs"] = focus_by_tab(app, SHOW)
        focus(key(app, "enter"), BACK)
        for setting in (THEME, RTL, TEXT):
            frame = activate(app, setting) if rtl else key(app, "tab")
            focus(frame, setting)
        sample_windows(app, observations)

        if not rtl:
            rest = {target: geometry(frame, target, spec[3], rtl) for target, spec in PRESETS.items()}
            initial = capture_paint(app, plan, "wide-light-sliders", frame, rest)
            result["wide_rest"] = {"geometry": rest, "png": initial}

        frame = key(app, "tab")
        focus(frame, XS)
        start = geometry(frame, XS, 25., rtl, engaged=True)
        sequence = key_stops(app, XS, rtl)
        frame = set_value(app, XS, 37.25, 37.25)
        shape = geometry(frame, XS, 37.25, rtl, engaged=True)
        require(shape["hit"] == start["hit"] and shape["rail"] == start["rail"],
                "Value changed the physical hit target")
        label = node(frame, f"{XS}-label-text")["text"]
        require(label == "37.3％", f"Unexpected localized label: {label}")
        capture_name = "compact-dark-rtl-200-sliders" if rtl else "wide-light-focused-slider"
        selected = capture_paint(app, plan, capture_name, frame, {XS: shape}, focused=True)
        result["continuous"] = {"geometry": shape, "keys": sequence, "absolute_value": 37.25, "png": selected}
        sample_windows(app, observations)

        frame = key(app, "tab")
        focus(frame, S)
        require(find(frame, f"{XS}-label")[1] is None, "Unfocused value label stayed mounted")
        shape = geometry(frame, S, 50., rtl, engaged=True)
        dots = ticks(frame, S, shape, rtl)
        sequence = key_stops(app, S, rtl)
        set_value(app, S, 43.75, 50.)
        result["discrete"] = {"geometry": shape, "ticks": dots, "keys": sequence, "tie_value": 50.}
        app.wire("Action.setValue", {"id": DISABLED, "value": 80.}, expected_error="disabled")
        frame = settle(app)
        ranged(frame, DISABLED, 50.)
        focus(frame, S)

        for target in (M, L, XL):
            frame = key(app, "tab")
            focus(frame, target)
            geometry(frame, target, PRESETS[target][3], rtl, engaged=True)
        if rtl:
            scroll = node(frame, SCROLL)["style"]["scroll_y"]
            require(scroll > 0, "Focus did not reveal the XL slider")
            shape = geometry(frame, XL, 50., rtl, engaged=True)
            large = capture_paint(app, plan, "compact-dark-rtl-200-xl", frame, {XL: shape}, focused=True)
            result["compact_xl"] = {"geometry": shape, "png": large, "scroll_y": scroll}
        sample_windows(app, observations)

        frame = key(app, "tab")
        require(frame["interaction"]["focused"] != DISABLED, "Tab focused the disabled slider")
        frame = key(app, "tab", reverse=True)
        focus(frame, XL)
        ranged(frame, DISABLED, 50.)
        focus(activate(app, BACK), SHOW)
        sample_windows(app, observations)
    require(app.proc.returncode is not None, "The child Gallery process did not exit")
    result.update({"closed_focus": SHOW, "disabled_value": 50., "visible_windows": observations,
                   "gallery_reaped": True})
    return result


def checked_pins(manifest, expected):
    manifest = manifest.resolve(strict=True)
    require(re.fullmatch(r"[a-f0-9]{64}", expected) and digest(manifest) == expected,
            "Native binary manifest does not match the reviewed digest")
    paths, pins = {}, {}
    for line in manifest.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        require(name in BINARIES and name not in pins and re.fullmatch(r"[a-f0-9]{64}", checksum),
                "Unexpected binary manifest entry")
        path = manifest.parent / name
        require(path.is_file() and not path.is_symlink() and os.access(path, os.X_OK)
                and digest(path) == checksum, f"Wrong native binary bytes: {name}")
        paths[BINARIES[name]] = path
        pins[name] = checksum
    require(pins.keys() == BINARIES.keys(), "Manifest does not bind both required binaries")
    return paths, pins


def replay(paths, out):
    plan = SimpleNamespace(**paths, out=out)
    out.mkdir()
    return {name: replay_profile(plan, name) for name in VIEWPORTS}


def compare_images(first, second):
    require(all({path.name for path in root.glob("*.png")} == CAPTURES for root in (first, second)),
            "One recurrence lacks a PNG")
    result = {}
    for name in sorted(CAPTURES):
        left, right = first / name, second / name
        require(left.read_bytes() == right.read_bytes(), f"Replays differ in {name}")
        result[name] = digest(left)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary-manifest", type=Path, required=True)
    parser.add_argument("--binary-manifest-sha256", required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check-only", action="store_true")
    mode.add_argument("--out", type=Path, help="fresh private directory under /tmp")
    args = parser.parse_args()
    paths, pins = checked_pins(args.binary_manifest, args.binary_manifest_sha256)
    report = {"schema": "surface.test.standard-sliders.native.v1",
              "native_binary_manifest_sha256": args.binary_manifest_sha256, "binaries": pins}
    if args.check_only:
        print(json.dumps({"status": "pins-verified", **report}, indent=2))
        return
    out = args.out
    require(not out.exists() and out.parent.resolve(strict=True).is_relative_to(Path("/tmp").resolve()),
            "Use a fresh private output directory under /tmp")
    out.mkdir(mode=0o700)
    outputs = [out / name for name in ("replay-1", "replay-2")]
    captures = [replay(paths, path) for path in outputs]
    require(checked_pins(args.binary_manifest, args.binary_manifest_sha256)[1] == pins,
            "Native binaries were replaced during the replays")
    pngs = compare_images(*outputs)
    report.update({"rendering": "headless", "reduce_motion": True,
                   "transports": ["Page.screenshot", "CLI", "MCP"],
                   "replays": captures, "byte_identical_pngs": pngs})
    (out / "native-slider-report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    files = sorted(path for path in out.rglob("*") if path.is_file())
    manifest = out / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in files))
    print(json.dumps({"manifest_sha256": digest(manifest), "images_per_replay": len(pngs),
                      "visible_windows": 0, "pngs": pngs}, indent=2))


if __name__ == "__main__":
    main()
