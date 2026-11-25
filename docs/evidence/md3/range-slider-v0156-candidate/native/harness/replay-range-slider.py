#!/usr/bin/env python3
"""Verify the native horizontal Range slider on sealed headless inputs."""

import argparse
import json
from math import ceil, floor, isclose
import os
from pathlib import Path
import re
import tomllib
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, ink_sample, key, layout, node, settle, visible
from window_check import onscreen_windows

PREFIX = "range-slider-demo-"
SHOW, BACK, THEME, RTL, TEXT, COLLAPSE, SCROLL, ROUTE = (
    PREFIX + part for part in ("show", "back", "theme", "rtl", "large-text", "collapse", "scroll", "route")
)
CONTINUOUS, DISCRETE, EQUAL, DISABLED = (
    PREFIX + name for name in ("continuous", "discrete", "equal", "disabled")
)
# Group name, thumb names, full domain, step, starting pair, track and handle heights.
SAMPLES = {
    CONTINUOUS: ("連續區間", ("連續下限", "連續上限"), (0., 100.), 5., (25., 75.), (16., 44.)),
    DISCRETE: ("階梯區間", ("階梯下限", "階梯上限"), (-10., 90.), 12.5, (15., 65.), (24., 44.)),
    EQUAL: ("相等端點", ("相等下限", "相等上限"), (0., 100.), 5., (50., 50.), (16., 44.)),
    DISABLED: ("停用區間", ("停用下限", "停用上限"), (0., 100.), 5., (25., 75.), (40., 52.)),
}
THUMBS = ("lower", "upper")
TRACKS = ("active", "inactive-min", "inactive-max")
STOPS = ("stop-min", "stop-max")
VIEWPORTS = {"wide": (960, 1040), "compact": (320, 880)}
CAPTURES = (
    "wide-light-rest", "wide-light-discrete-upper-focus", "wide-light-equal-lower-focus",
    "compact-dark-rtl-200-continuous-lower-focus", "compact-dark-rtl-200-discrete-upper-focus",
)
BINARIES = {"surface-material-gallery": "gallery", "surface-devtools": "cli"}
HEX = re.compile(r"[0-9a-f]{64}")


def close(actual, expected, context):
    require(isclose(actual, expected, rel_tol=0, abs_tol=0.003),
            f"{context}: expected {expected}, got {actual}")


def edge(rect, axis):
    return rect[axis] + rect["width" if axis == "x" else "height"]


def center(rect, axis):
    return (rect[axis] + edge(rect, axis)) / 2


def positive(rect):
    return rect["width"] > 0 and rect["height"] > 0


def value_label(target, value):
    # The exercised values have at most two decimals. This rounds a positive
    # magnitude to one place and retains the authored sign and full-width percent.
    rounded = floor(abs(value) * 10 + 0.5) / 10
    sign = ("−" if value < 0 else "+" if target == DISCRETE else "") if rounded else ""
    return f"{sign}{rounded:g}％"


def domains(frame, target, values):
    group_name, names, (low, high), step, _, _ = SAMPLES[target]
    owner_index, owner = find(frame, target)
    require(owner is not None and owner["name"] == group_name and owner["role"] == "Group"
            and owner.get("disabled", False) is (target == DISABLED),
            f"Wrong named Range owner {target}")
    result = {}
    indices = []
    for thumb, name, value in zip(THUMBS, names, values, strict=True):
        handle = f"{target}-{thumb}"
        index, authored = find(frame, handle)
        require(authored is not None and authored["name"] == name and authored["role"] == "Slider"
                and authored.get("parent") == owner_index and "slider_vertical" not in authored
                and authored.get("disabled", False) is (target == DISABLED),
                f"Wrong independent named horizontal Slider {handle}: {authored}")
        expected = {"min": low, "max": high, "step": step, "value": value}
        require(authored["range"] == expected and authored["slider_value_text"] == value_label(target, value),
                f"Wrong numeric domain or localized value on {handle}: {authored['range']}")
        indices.append(index)
        result[thumb] = {"name": name, "range": expected, "text": authored["slider_value_text"]}
    require(indices[0] < indices[1], "Thumb author order changed with physical direction")
    return result


def geometry(frame, target, values, rtl, engaged=None):
    domains(frame, target, values)
    _, _, (low, high), _, _, (track_height, handle_height) = SAMPLES[target]
    owner = visible(frame, target, 48.)
    rail = visible(frame, f"{target}-rail")
    y = center(rail, "y")
    require(owner["height"] == rail["height"] == max(48., handle_height)
            and rail["y"] == owner["y"], f"Wrong stable Range owner or rail height on {target}")
    close(rail["x"], owner["x"] + 24, "Rail physical start inset")
    close(edge(rail, "x"), edge(owner, "x") - 24, "Rail physical end inset")

    def logical(x):
        return edge(rail, "x") - x if rtl else x - rail["x"]

    length = rail["width"]
    stops = [(value - low) / (high - low) * length for value in values]
    front = engaged or "upper"
    gaps = []
    thumbs = {}
    for thumb, offset in zip(THUMBS, stops, strict=True):
        key = f"{target}-{thumb}"
        hit = visible(frame, key, 48.)
        handle = visible(frame, f"{key}-handle")
        ring = layout(frame, f"{key}-focus")["bounds"]
        width = 2. if thumb == engaged else 4.
        require(same_box(owner, hit, 0.003) and hit["width"] == 48. and hit["height"] == owner["height"]
                and (handle["width"], handle["height"]) == (width, handle_height)
                and (ring["width"], ring["height"]) == (width + 10, handle_height + 10),
                f"Wrong hit, handle or focus dimensions on {key}")
        for area in (hit, handle, ring):
            close(logical(center(area, "x")), offset, f"{key} logical x")
            close(center(area, "y"), y, f"{key} cross center")
        alpha = node(frame, f"{key}-handle")["style"]["background"][3]
        hidden = values[0] == values[1] and thumb != front
        require(alpha == (0 if hidden else 97 if target == DISABLED else 255),
                f"Wrong exposed handle paint at shared stop {key}")
        thumbs[thumb] = {"hit": hit, "handle": handle, "focus": ring, "drawn": not hidden}
        gaps.append(6. + width / 2.)
    if values[0] == values[1]:
        require(thumbs["lower"]["hit"] == thumbs["upper"]["hit"], "Coincident targets are not authored separately at one stop")

    lower, upper = stops
    lower_gap, upper_gap = gaps
    intervals = ((lower + lower_gap, upper - upper_gap), (0., lower - lower_gap), (upper + upper_gap, length))
    pieces = {}
    for part, (start, end) in zip(TRACKS, intervals, strict=True):
        bounds = layout(frame, f"{target}-{part}")["bounds"]
        if end - start < (0.001 if part == "active" else 8.):
            require(not positive(bounds), f"A collapsed {part} is painted")
            continue
        require(same_box(layout(frame, f"{target}-{part}")["clip"], bounds),
                f"Clipped {target}-{part}")
        a, b = sorted((logical(bounds["x"]), logical(edge(bounds, "x"))))
        close(a, start, f"{target}-{part} logical start")
        close(b, end, f"{target}-{part} logical end")
        close(bounds["height"], track_height, f"{target}-{part} thickness")
        close(center(bounds, "y"), y, f"{target}-{part} cross center")
        pieces[part] = bounds
    for part, cap, offset in zip(STOPS, TRACKS[1:], (6., length - 6.), strict=True):
        bounds = layout(frame, f"{target}-{part}")["bounds"]
        if cap not in pieces:
            require(not positive(bounds), f"A collapsed cap draws {part}")
            continue
        require(bounds["width"] == bounds["height"] == 4., f"Wrong {part} size")
        close(logical(center(bounds, "x")), offset, f"{target}-{part} logical center")
        close(center(bounds, "y"), y, f"{target}-{part} cross center")
        pieces[part] = bounds
    ticks = []
    if target == DISCRETE:
        require(find(frame, f"{target}-tick-0")[1] is None and find(frame, f"{target}-tick-8")[1] is None,
                "Duplicate endpoint dots at the range limits")
        for index in range(1, 8):
            part = f"tick-{index}"
            bounds = layout(frame, f"{target}-{part}")["bounds"]
            if not positive(bounds):
                continue
            require(bounds["width"] == bounds["height"] == 4., f"Wrong size of {target}-{part}")
            offset = center(bounds, "x")
            close(logical(offset), length * index / 8., f"{target}-{part} full signed lattice")
            close(center(bounds, "y"), y, f"{target}-{part} cross center")
            require(any(same_box(pieces[track], bounds, 0.003) for track in TRACKS if track in pieces),
                    f"{target}-{part} appears in the gap")
            ticks.append(index)
            pieces[part] = bounds
        require(len(ticks) >= 3, "Too few visible signed discrete stops")
    return {"owner": owner, "rail": rail, "thumbs": thumbs, "pieces": pieces, "ticks": ticks}


def rgba(rows, x, y):
    require(0 <= y < len(rows) and 0 <= x < len(rows[0]) // 4, f"Sample outside image {x},{y}")
    return list(rows[y][4*x:4*(x+1)])


def count_color(rows, rect, color):
    return sum(rgba(rows, x, y) == color
               for y in range(ceil(rect["y"]), floor(edge(rect, "y")))
               for x in range(ceil(rect["x"]), floor(edge(rect, "x"))))


def sample_fill(frame, rows, target, bounds, *, part, disabled, background):
    x = floor(center(bounds, "x"))
    y = floor(bounds["y"] + 0.30 * bounds["height"]) if part in TRACKS else floor(center(bounds, "y"))
    actual = rgba(rows, x, y)
    authored = node(frame, target)["style"]["background"]
    if disabled:
        require(authored[3] in (31, 97) and actual != background and actual[3] == 255,
                f"Disabled paint is missing on {target}")
    else:
        require(actual == authored, f"{target} decoded pixel {actual} differs from {authored}")
    return {"at": [x, y], "rgba": actual, "authored": authored}


def sample_label(frame, rows, target, thumb, shape, rtl):
    key = f"{target}-{thumb}"
    other = f"{target}-{'upper' if thumb == 'lower' else 'lower'}-label"
    require(find(frame, other)[1] is None, "Both value labels are mounted")
    ring = visible(frame, f"{key}-focus")
    label = visible(frame, f"{key}-label")
    text_key = f"{key}-label-text"
    text = node(frame, text_key)["text"]
    require(text == node(frame, key)["slider_value_text"], "Visible and accessible value labels differ")
    require(label["height"] >= (64. if rtl else 44.), "Value label did not grow with text")
    close(ring["y"] - edge(label, "y"), 4., "Focus ring clearance below the label")
    require(label["x"] >= shape["owner"]["x"] - 0.003
            and edge(label, "x") <= edge(shape["owner"], "x") + 0.003,
            "Label goes beyond its owner")
    color = node(frame, f"{key}-focus")["focus_color"]
    paint = count_color(rows, ring, color)
    require(paint >= 40, "Decoded focus ring is missing")
    fill = node(frame, f"{key}-label")["style"]["background"]
    require(rgba(rows, floor(center(label, "x")), floor(label["y"] + 8)) == fill,
            "Decoded value label fill is absent")
    return {"ring": ring, "ring_pixels": paint, "ring_rgba": color, "label": label,
            "label_rgba": fill, "text": text, "ink": ink_sample(frame, rows, text_key)}


def photograph(app, plan, title, frame, shapes, *, rtl=False, focused=None):
    shot = capture(app, plan, title)
    require(settle(app) == frame, f"{title} changed during transport comparison")
    width, height, rows = read_png((plan.out / shot["name"]).read_bytes())
    require((width, height) == VIEWPORTS["compact" if rtl else "wide"], "Wrong decoded viewport")
    background = node(frame, ROUTE)["style"]["background"]
    require(rgba(rows, 4, 4) == background
            and (max(background[:3]) <= 96 if rtl else min(background[:3]) >= 200),
            "Decoded background does not match the requested theme")
    fields = {}
    for target, shape in shapes.items():
        disabled = target == DISABLED
        pieces = {}
        for part, bounds in shape["pieces"].items():
            pieces[part] = sample_fill(frame, rows, f"{target}-{part}", bounds,
                                       part=part, disabled=disabled, background=background)
        for thumb, details in shape["thumbs"].items():
            if details["drawn"]:
                part = f"{thumb}-handle"
                pieces[part] = sample_fill(frame, rows, f"{target}-{part}", details["handle"],
                                           part=part, disabled=disabled, background=background)
        fields[target] = {"pixels": pieces, "caption_ink": ink_sample(frame, rows, f"{target}-caption")}
        active = shape["pieces"].get("active")
        if active:
            painted_colors = [record["rgba"] for part, record in pieces.items() if part in TRACKS]
            point = [floor(active["x"] - 2.), floor(center(active, "y"))]
            gap = rgba(rows, *point)
            require(gap not in painted_colors, "Decoded handle gap contains track paint")
            fields[target]["gap_pixel"] = {"at": point, "rgba": gap}
        if focused and focused[0] == target:
            fields[target]["value_label"] = sample_label(frame, rows, target, focused[1], shape, rtl)
    return {**shot, "background": background, "paint": fields}


def observe_windows(app, samples):
    count = onscreen_windows(app.proc.pid)
    samples.append(count)
    require(count == 0, "The launched Gallery owns a visible window")


def stroke(app, target, thumb, press, expected):
    frame = key(app, press)
    focus(frame, f"{target}-{thumb}")
    domains(frame, target, expected)
    return frame, {"key": press, "values": expected}


def set_value(app, target, thumb, request, expected, *, focused=None):
    app.wire("Action.setValue", {"id": f"{target}-{thumb}", "value": request})
    frame = settle(app)
    focus(frame, f"{target}-{focused or thumb}")
    domains(frame, target, expected)
    return frame, {"set": thumb, "request": request, "values": expected}


def check_fixed_rail(before, after):
    require(before["owner"] == after["owner"] and before["rail"] == after["rail"],
            "Changing a held value moved the shared input rail")


def replay_profile(plan, profile):
    width, height = VIEWPORTS[profile]
    rtl = profile == "compact"
    windows = []
    result = {"viewport": [width, height], "rtl": rtl, "text_scale": 2 if rtl else 1}
    with launch(plan, profile, width, height) as app:
        observe_windows(app, windows)
        result["opening_tabs"] = focus_by_tab(app, SHOW)
        focus(key(app, "enter"), BACK)
        for setting in (THEME, RTL, TEXT):
            frame = activate(app, setting) if rtl else key(app, "tab")
            focus(frame, setting)
        frame = key(app, "tab")
        focus(frame, COLLAPSE)
        observe_windows(app, windows)
        result["initial_domains"] = {
            target: domains(frame, target, spec[4]) for target, spec in SAMPLES.items()
        }
        if not rtl:
            shapes = {target: geometry(frame, target, spec[4], rtl) for target, spec in SAMPLES.items()}
            result["rest"] = {"geometry": shapes, "png": photograph(app, plan, CAPTURES[0], frame, shapes)}

        frame = key(app, "tab")
        focus(frame, f"{CONTINUOUS}-lower")
        initial = geometry(frame, CONTINUOUS, (25., 75.), rtl, "lower")
        changes = []
        for press, expected in (("left" if rtl else "right", (30., 75.)),
                                ("home", (0., 75.)),
                                ("end", (75., 75.)), ("end", (75., 75.))):
            frame, receipt = stroke(app, CONTINUOUS, "lower", press, expected)
            changes.append(receipt)
            if press == "home":
                check_fixed_rail(initial, geometry(frame, CONTINUOUS, expected, rtl, "lower"))
        equal_geometry = geometry(frame, CONTINUOUS, (75., 75.), rtl, "lower")
        check_fixed_rail(initial, equal_geometry)
        # An absolute change on the other thumb has no focus side effect.
        _, other = set_value(app, CONTINUOUS, "upper", 85., (75., 85.), focused="lower")
        frame, chosen = set_value(app, CONTINUOUS, "lower", 37.25, (37.25, 85.))
        shapes = geometry(frame, CONTINUOUS, (37.25, 85.), rtl, "lower")
        check_fixed_rail(initial, shapes)
        result["continuous"] = {"lower_keys": changes, "set_value": [other, chosen],
                                "equal_geometry": equal_geometry, "geometry": shapes}
        if rtl:
            result["continuous"]["png"] = photograph(
                app, plan, CAPTURES[3], frame, {CONTINUOUS: shapes}, rtl=True, focused=(CONTINUOUS, "lower"))
        frame = key(app, "tab")
        focus(frame, f"{CONTINUOUS}-upper")
        require(find(frame, f"{CONTINUOUS}-lower-label")[1] is None,
                "Lower label remains after independently focusing upper")
        upper_receipts = []
        for press, expected in (("home", (37.25, 37.25)), ("up", (37.25, 42.25)),
                                ("end", (37.25, 100.))):
            frame, receipt = stroke(app, CONTINUOUS, "upper", press, expected)
            upper_receipts.append(receipt)
        max_geometry = geometry(frame, CONTINUOUS, (37.25, 100.), rtl, "upper")
        check_fixed_rail(initial, max_geometry)
        result["continuous"]["upper_keys"] = upper_receipts
        result["continuous"]["max_geometry"] = max_geometry
        observe_windows(app, windows)

        frame = key(app, "tab")
        focus(frame, f"{DISCRETE}-lower")
        original = geometry(frame, DISCRETE, (15., 65.), rtl, "lower")
        _, snapped_lower = set_value(app, DISCRETE, "lower", 22.4, (27.5, 65.))
        frame = key(app, "tab")
        focus(frame, f"{DISCRETE}-upper")
        frame, snapped_upper = set_value(app, DISCRETE, "upper", 46.25, (27.5, 52.5))
        discrete = geometry(frame, DISCRETE, (27.5, 52.5), rtl, "upper")
        check_fixed_rail(original, discrete)
        shot = photograph(app, plan, CAPTURES[4 if rtl else 1], frame, {DISCRETE: discrete},
                          rtl=rtl, focused=(DISCRETE, "upper"))
        frame, hit_limit = set_value(app, DISCRETE, "lower", 80., (52.5, 52.5), focused="upper")
        equal_discrete = geometry(frame, DISCRETE, (52.5, 52.5), rtl, "upper")
        check_fixed_rail(original, equal_discrete)
        _, discrete_end = stroke(app, DISCRETE, "upper", "end", (52.5, 90.))
        result["discrete"] = {"set_value": [snapped_lower, snapped_upper, hit_limit],
                              "upper_end": discrete_end, "geometry": discrete,
                              "equal_geometry": equal_discrete, "png": shot,
                              "scroll_y": node(frame, SCROLL)["style"].get("scroll_y", 0.)}
        observe_windows(app, windows)

        frame = key(app, "tab")
        focus(frame, f"{EQUAL}-lower")
        equality = geometry(frame, EQUAL, (50., 50.), rtl, "lower")
        result["equal"] = {"geometry": equality}
        if not rtl:
            result["equal"]["png"] = photograph(
                app, plan, CAPTURES[2], frame, {EQUAL: equality}, focused=(EQUAL, "lower"))
        chronology = []
        for press, expected in (("end", (50., 50.)), ("down", (45., 50.)), ("up", (50., 50.))):
            _, receipt = stroke(app, EQUAL, "lower", press, expected)
            chronology.append(receipt)
        focus(key(app, "tab"), f"{EQUAL}-upper")
        for press, expected in (("home", (50., 50.)), ("up", (50., 55.)), ("down", (50., 50.))):
            _, receipt = stroke(app, EQUAL, "upper", press, expected)
            chronology.append(receipt)
        result["equal"]["keys"] = chronology
        for thumb in THUMBS:
            app.wire("Action.setValue", {"id": f"{DISABLED}-{thumb}", "value": 55.}, expected_error="disabled")
        frame = settle(app)
        focus(frame, f"{EQUAL}-upper")
        result["disabled"] = {"set_value": "disabled", "unchanged": domains(frame, DISABLED, (25., 75.))}
        focus(key(app, "tab"), BACK)
        focus(key(app, "tab", reverse=True), f"{EQUAL}-upper")
        focus(activate(app, BACK), SHOW)
        observe_windows(app, windows)
    require(app.proc.returncode is not None, "Gallery child remained active")
    return {**result, "skipped_disabled_tab": True, "closed_focus": SHOW,
            "visible_windows": windows, "gallery_reaped": True}


def manifest_entries(path, expected):
    require(HEX.fullmatch(expected) and digest(path) == expected, "Reviewed manifest digest differs")
    entries = {}
    for line in path.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        relative = Path(name)
        require(HEX.fullmatch(checksum) and not relative.is_absolute() and ".." not in relative.parts
                and relative.as_posix() == name and name not in entries, "Invalid manifest path or entry")
        entries[name] = checksum
    return entries


def binary_pins(manifest, expected):
    entries = manifest_entries(manifest, expected)
    require(entries.keys() == BINARIES.keys(), "Manifest must bind exactly the native Gallery and CLI")
    paths = {}
    for name, checksum in entries.items():
        path = manifest.parent / name
        require(path.is_file() and not path.is_symlink() and os.access(path, os.X_OK)
                and digest(path) == checksum, f"Wrong native executable bytes for {name}")
        paths[BINARIES[name]] = path
    return paths, entries


def source_pins(root, manifest, expected):
    entries = manifest_entries(manifest, expected)
    required = {"Cargo.toml", "Cargo.lock", "crates/surface/src/microdom.rs",
                "crates/surface-app/src/slider_pair.rs", "crates/surface-material/src/standard_slider.rs",
                "crates/surface-material/src/standard_slider/range_slider.rs",
                "crates/surface-platform/src/macos_accessibility.rs",
                "examples/material-gallery/src/range_slider_demo.rs"}
    require(required <= entries.keys(), "Source seal omits the reviewed two-thumb Range slider")
    for name, checksum in entries.items():
        path = root / name
        require(path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root)
                and digest(path) == checksum, f"Wrong source bytes for {name}")
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == "0.1.56", f"Unexpected source Cargo version {version}")
    return {"manifest_sha256": expected, "files_verified": len(entries), "version": version}


def replay(paths, out):
    out.mkdir(mode=0o700)
    plan = SimpleNamespace(**paths, out=out)
    return {profile: replay_profile(plan, profile) for profile in VIEWPORTS}


def compare_images(first, second):
    files = {f"{name}.png" for name in CAPTURES}
    require(all({path.name for path in root.glob("*.png")} == files for root in (first, second)),
            "One recurrence lacks an original PNG")
    matched = {}
    for name in sorted(files):
        left, right = first / name, second / name
        require(left.read_bytes() == right.read_bytes(), f"Replays differ in {name}")
        matched[name] = digest(left)
    require(len(set(matched.values())) == len(files), "Identical images were reused across different scenes")
    return matched


def self_test():
    from copy import deepcopy

    def box(x, y, width, height):
        return {"x": x, "y": y, "width": width, "height": height}

    def fixture(target, values, parts, *, rtl=False, engaged=None):
        group, names, (low, high), step, _, _ = SAMPLES[target]
        nodes = [{"stable_id": target, "role": "Group", "name": group}]
        viewport = box(0, 0, 320, 240)
        boxes = [{"bounds": box(10, 80, 240, 48), "clip": viewport}]
        for suffix, rect in parts.items():
            entry = {"stable_id": f"{target}-{suffix}", "role": "Group", "parent": 0}
            if suffix in THUMBS:
                index = THUMBS.index(suffix)
                entry.update(role="Slider", name=names[index],
                             range={"min": low, "max": high, "step": step, "value": values[index]},
                             slider_value_text=value_label(target, values[index]))
            if suffix.endswith("-handle"):
                front = engaged or "upper"
                shown = values[0] != values[1] or suffix == f"{front}-handle"
                entry["style"] = {"background": [32, 64, 96, 255 if shown else 0]}
            nodes.append(entry)
            if rtl and positive(rect):
                rect = {**rect, "x": 260. - edge(rect, "x")}
            boxes.append({"bounds": rect, "clip": viewport})
        return {"dom": {"nodes": nodes}, "layout": {"nodes": boxes}}

    # An independently calculated 192 dp signed rail at two separated stops.
    discrete = {
        "rail": box(34, 80, 192, 48),
        "lower": box(58, 80, 48, 48), "upper": box(154, 80, 48, 48),
        "lower-handle": box(80, 82, 4, 44), "upper-handle": box(176, 82, 4, 44),
        "lower-focus": box(75, 77, 14, 54), "upper-focus": box(171, 77, 14, 54),
        "active": box(90, 92, 80, 24), "inactive-min": box(34, 92, 40, 24),
        "inactive-max": box(186, 92, 40, 24),
        "stop-min": box(38, 102, 4, 4), "stop-max": box(218, 102, 4, 4),
    }
    for index in range(1, 8):
        discrete[f"tick-{index}"] = box(0, 0, 0, 0) if index in (2, 6) else box(32 + 24 * index, 102, 4, 4)
    for rtl in (False, True):
        actual = geometry(fixture(DISCRETE, (15., 65.), discrete, rtl=rtl), DISCRETE, (15., 65.), rtl)
        require(actual["ticks"] == [1, 3, 4, 5, 7], "Unexpected synthetic signed ticks")

    equal = {
        "rail": box(34, 80, 192, 48),
        "lower": box(106, 80, 48, 48), "upper": box(106, 80, 48, 48),
        "lower-handle": box(129, 82, 2, 44), "upper-handle": box(128, 82, 4, 44),
        "lower-focus": box(124, 77, 12, 54), "upper-focus": box(123, 77, 14, 54),
        "active": box(137, 96, 0, 16), "inactive-min": box(34, 96, 89, 16),
        "inactive-max": box(138, 96, 88, 16),
        "stop-min": box(38, 102, 4, 4), "stop-max": box(218, 102, 4, 4),
    }
    frame = fixture(EQUAL, (50., 50.), equal, engaged="lower")
    require(geometry(frame, EQUAL, (50., 50.), False, "lower")["thumbs"]["lower"]["drawn"],
            "Synthetic equality should paint only the engaged lower thumb")

    def rejected(action, message):
        try:
            action()
        except RuntimeError as error:
            require(message in str(error), f"Unexpected self-test failure {error}")
        else:
            raise AssertionError(f"Expected rejection {message}")

    changed = {**discrete, "active": box(89, 92, 81, 24)}
    rejected(lambda: geometry(fixture(DISCRETE, (15., 65.), changed), DISCRETE, (15., 65.), False),
             "logical start")
    rejected(lambda: geometry(fixture(DISCRETE, (15., 65.), discrete), DISCRETE, (15., 65.), True),
             "logical x")
    changed = deepcopy(frame)
    node(changed, f"{EQUAL}-upper-handle")["style"]["background"][3] = 255
    rejected(lambda: geometry(changed, EQUAL, (50., 50.), False, "lower"), "exposed handle paint")
    changed = deepcopy(frame)
    node(changed, f"{EQUAL}-upper")["parent"] = None
    rejected(lambda: geometry(changed, EQUAL, (50., 50.), False, "lower"), "independent named horizontal Slider")
    print(json.dumps({"positive_geometry_controls": 3, "negative_geometry_controls": 4,
                      "executables_launched": 0}, indent=2))

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("binary-manifest", "source-root", "source-manifest"):
        parser.add_argument(f"--{name}", type=Path)
    parser.add_argument("--binary-manifest-sha256")
    parser.add_argument("--source-manifest-sha256")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--self-test", action="store_true")
    mode.add_argument("--check-only", action="store_true")
    mode.add_argument("--out", type=Path, help="fresh private output directory inside /tmp")
    args = parser.parse_args()
    if not __debug__:
        parser.error("Run without -O.")
    if args.self_test:
        self_test()
        return
    required = (args.binary_manifest, args.binary_manifest_sha256, args.source_root,
                args.source_manifest, args.source_manifest_sha256)
    if not all(required):
        parser.error("Supply both exact manifests, approved digests and the source root.")
    binary_manifest = args.binary_manifest.resolve(strict=True)
    source_manifest = args.source_manifest.resolve(strict=True)
    source_root = args.source_root.resolve(strict=True)
    paths, pins = binary_pins(binary_manifest, args.binary_manifest_sha256)
    sources = source_pins(source_root, source_manifest, args.source_manifest_sha256)
    report = {"schema": "surface.test.range-slider.native.v1", "source": sources,
              "native_binary_manifest_sha256": args.binary_manifest_sha256, "binaries": pins}
    if args.check_only:
        print(json.dumps({"status": "pins-verified", **report}, indent=2))
        return

    out = args.out
    require(not out.exists() and out.parent.resolve(strict=True).is_relative_to(Path("/tmp").resolve()),
            "Use a fresh private output directory under /tmp")
    out.mkdir(mode=0o700)
    outputs = [out / name for name in ("replay-1", "replay-2")]
    recurrences = [replay(paths, path) for path in outputs]
    require(binary_pins(binary_manifest, args.binary_manifest_sha256)[1] == pins,
            "Native binaries were replaced during the replays")
    require(source_pins(source_root, source_manifest, args.source_manifest_sha256) == sources,
            "Source input changed during the replays")
    pngs = compare_images(*outputs)
    require(recurrences[0] == recurrences[1], "Typed runtime or pixel observations differ between recurrences")
    window_samples = sum(len(profile["visible_windows"])
                         for recurrence in recurrences for profile in recurrence.values())
    report.update({"rendering": "headless", "reduce_motion": True,
                   "transports": ["Page.screenshot", "CLI", "MCP"],
                   "input_scope": ["Action.key", "Action.setValue", "Action.activate"],
                   "window_samples": window_samples, "replays": recurrences, "byte_identical_pngs": pngs})
    (out / "native-range-slider-report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    files = sorted(path for path in out.rglob("*") if path.is_file())
    manifest = out / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in files))
    print(json.dumps({"manifest_sha256": digest(manifest), "images_per_replay": len(pngs),
                      "window_samples": window_samples, "visible_windows": 0, "pngs": pngs}, indent=2))


if __name__ == "__main__":
    main()
