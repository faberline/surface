#!/usr/bin/env python3
"""Verify native slider forms using sealed inputs and offscreen transports."""

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

PREFIX = "slider-forms-demo-"
SHOW, BACK, THEME, RTL, TEXT, SCROLL, ROUTE = (
    PREFIX + name for name in ("show", "back", "theme", "rtl", "large-text", "scroll", "route")
)
NEGATIVE, NEUTRAL, POSITIVE, CENTERED_DISCRETE = (
    PREFIX + "centered-" + name for name in ("negative", "neutral", "positive", "discrete")
)
VERTICAL_CONTINUOUS, VERTICAL_DISCRETE, DISABLED = (
    PREFIX + "vertical-" + name for name in ("continuous", "discrete", "disabled")
)
CENTERED = {
    NEGATIVE: ("負值", -50., 5.), NEUTRAL: ("中立值", 0., 5.),
    POSITIVE: ("正值", 50., 5.), CENTERED_DISCRETE: ("離散值", -25., 25.),
}
VERTICAL = {
    VERTICAL_CONTINUOUS: ("垂直連續", 25., 5.),
    VERTICAL_DISCRETE: ("垂直階梯", 50., 12.5),
    DISABLED: ("垂直停用", 50., 12.5),
}
VIEWPORTS = {"wide": (960, 1040), "compact": (320, 880)}
CAPTURES = (
    "wide-light-rest", "wide-light-centered-focus", "wide-light-vertical-focus",
    "compact-dark-rtl-200-centered-focus", "compact-dark-rtl-200-vertical-focus",
)
BINARIES = {"surface-material-gallery": "gallery", "surface-devtools": "cli"}
HEX = re.compile(r"[0-9a-f]{64}")


def close(actual, expected, context):
    require(isclose(actual, expected, rel_tol=0, abs_tol=0.003),
            f"{context}: expected {expected}, got {actual}")


def edge(rect, axis):
    return rect[axis] + rect[{"x": "width", "y": "height"}[axis]]


def center(rect, axis):
    return (rect[axis] + edge(rect, axis)) / 2


def positive(rect):
    return rect["width"] > 0 and rect["height"] > 0


def ranged(frame, target, expected):
    vertical = target in VERTICAL
    name, _, step = (VERTICAL if vertical else CENTERED)[target]
    item = node(frame, target)
    require(item["name"] == name and item["role"] == "Slider"
            and item.get("disabled", False) is (target == DISABLED)
            and item.get("slider_vertical", False) is vertical,
            f"Wrong named or oriented slider: {target}")
    require(("slider_vertical" in item) is vertical, "Horizontal slider serialized default orientation")
    domain = {"min": 0. if vertical else -100., "max": 100., "value": expected, "step": step}
    require(item["range"] == domain, f"Wrong {target} numeric range: {item['range']}")
    return domain


def geometry(frame, target, value, rtl, *, engaged=False):
    ranged(frame, target, value)
    vertical = target in VERTICAL
    axis, cross = ("y", "x") if vertical else ("x", "y")
    main, cross_size = ("height", "width") if vertical else ("width", "height")
    reversed_axis = vertical or rtl
    if vertical:
        parts = ("rail", "active", "inactive", "handle", "focus", "stop")
    else:
        parts = ("rail", "active", "inactive-min", "inactive-max", "handle", "focus", "stop-min", "stop-max")
    hit = visible(frame, target, 48.)
    shape = {"hit": hit, **{name: layout(frame, f"{target}-{name}")["bounds"] for name in parts}}
    rail, grip, ring = (shape[name] for name in ("rail", "handle", "focus"))
    require(hit[cross_size] == rail[cross_size] == 48., f"{target} has the wrong Small target cross size")
    if vertical:
        require(hit["height"] == 240., f"{target} height differs")
    close(rail[axis], hit[axis] + 12, f"{target} rail start inset")
    close(edge(rail, axis), edge(hit, axis) - 12, f"{target} rail end inset")
    close(center(grip, cross), center(rail, cross), f"{target} thumb cross center")
    effective = 2 if engaged else 4
    require((grip[main], grip[cross_size]) == (effective, 44.), f"{target} handle dimensions differ")
    require((ring[main], ring[cross_size]) == (effective + 10, 54.), f"{target} ring dimensions differ")
    for coordinate in ("x", "y"):
        close(ring[coordinate], grip[coordinate] - 5, f"{target} outline alignment")

    def logical(position):
        return edge(rail, axis) - position if reversed_axis else position - rail[axis]

    length = rail[main]
    thumb = length * (value / 100 if vertical else (value + 100) / 200)
    neutral = length / 2
    gap = 6 + grip[main] / 2
    close(logical(center(grip, axis)), thumb, f"{target} logical thumb")
    if vertical:
        spans = {"active": (0., thumb - gap), "inactive": (thumb + gap, length)}
        stops = {"stop": ("inactive", length - 6.)}
    else:
        spans = {
            "active": (thumb + gap, neutral - 2) if value < 0 else (neutral + 2, thumb - gap),
            "inactive-min": (0., min(thumb - gap, neutral - 2)),
            "inactive-max": (max(thumb + gap, neutral + 2), length),
        }
        stops = {"stop-min": ("inactive-min", 6.), "stop-max": ("inactive-max", length - 6.)}
    for name, (start, end) in spans.items():
        start, end = max(start, 0.), min(end, length)
        area = shape[name]
        if end - start < (0.001 if name == "active" else 8.):
            require(not positive(area), f"{target} draws collapsed {name}")
            continue
        close(area[cross_size], 24., f"{target} {name} thickness")
        close(center(area, cross), center(grip, cross), f"{target} {name} center")
        a, b = sorted([logical(area[axis]), logical(edge(area, axis))])
        close(a, start, f"{target} {name} logical start")
        close(b, end, f"{target} {name} logical end")
    for name, (owner, offset) in stops.items():
        dot = shape[name]
        if positive(shape[owner]):
            require(dot["height"] == dot["width"] == 4., f"{target} {name} dimensions differ")
            close(logical(center(dot, axis)), offset, f"{target} {name} outer inset")
            close(center(dot, cross), center(grip, cross), f"{target} {name} cross center")
        else:
            require(not positive(dot), f"{target} paints a stop on a collapsed cap")
    return shape


def ticks(frame, target, shape, rtl):
    vertical = target in VERTICAL
    axis = "y" if vertical else "x"
    rail = shape["rail"]
    length = rail["height" if vertical else "width"]
    marks = []
    if not vertical:
        require(find(frame, f"{target}-tick-0")[1] is None,
                "Centered lower anchor must own its single endpoint dot")
    for index in range(0 if vertical else 1, 8):
        part = f"{target}-tick-{index}"
        dot = layout(frame, part)["bounds"]
        if not positive(dot):
            continue
        require(dot["width"] == dot["height"] == 4., f"{part} is not a 4 dp dot")
        offset = 6. if index == 0 else length * index / 8
        expected = edge(rail, axis) - offset if vertical or rtl else rail[axis] + offset
        close(center(dot, axis), expected, f"{part} logical position")
        marks.append((index, dot))
    require(len(marks) >= 3, f"{target} discrete dots are missing")
    if not vertical:
        require(not {0, 4} & {index for index, _ in marks}, "Centered tick overlaps an anchor or neutral gap")
    return marks


def rgba(rows, x, y):
    require(0 <= y < len(rows) and 0 <= x < len(rows[0]) // 4, f"PNG sample outside frame: {x},{y}")
    return list(rows[y][4*x:4*(x+1)])


def count_color(rows, rect, color):
    return sum(rgba(rows, x, y) == color
               for y in range(ceil(rect["y"]), floor(edge(rect, "y")))
               for x in range(ceil(rect["x"]), floor(edge(rect, "x"))))


def paint_piece(rows, frame, target, name, rect, background):
    x, y = center(rect, "x"), center(rect, "y")
    if name == "active" or name.startswith("inactive"):
        if target in VERTICAL:
            x = rect["x"] + rect["width"] * 0.30
        else:
            y = rect["y"] + rect["height"] * 0.30
    point = [floor(x), floor(y)]
    actual = rgba(rows, *point)
    authored = node(frame, f"{target}-{name}")["style"]["background"]
    if target == DISABLED:
        require(authored[3] in (31, 97) and actual != background and actual[3] == 255,
                f"{target}-{name} disabled artwork is not visible")
    else:
        require(actual == authored, f"{target}-{name} screen pixel {actual} differs from {authored}")
    return {"point": point, "rgba": actual, "authored": authored}


def verify_label(rows, frame, target, shape, rtl, expected):
    label = visible(frame, f"{target}-label")
    ring = visible(frame, f"{target}-focus")
    text_key = f"{target}-label-text"
    require(node(frame, text_key)["text"] == expected == node(frame, target)["slider_value_text"],
            f"{target} visible and accessible value text differ")
    require(label["height"] >= (64 if rtl else 44), f"{target} label did not grow with text")
    hit = shape["hit"]
    if target in VERTICAL:
        close(ring["x"] - edge(label, "x") if rtl else label["x"] - edge(ring, "x"),
              4, "side label clearance")
        require(label["y"] >= hit["y"] and edge(label, "y") <= edge(hit, "y"),
                f"{target} side label goes beyond the control height")
        in_room = label["x"] >= hit["x"] - 120. if rtl else edge(label, "x") <= edge(hit, "x") + 120.
        require(in_room, f"{target} side label goes beyond its reserved area")
    else:
        close(ring["y"] - edge(label, "y"), 4, "above label clearance")
        require(label["x"] >= hit["x"] and edge(label, "x") <= edge(hit, "x"),
                f"{target} horizontal label goes beyond its control width")
    color = node(frame, f"{target}-focus")["focus_color"]
    focus_count = count_color(rows, ring, color) - count_color(rows, shape["handle"], color)
    require(focus_count >= 40, f"No visible focus ring on {target}")
    label_fill = node(frame, f"{target}-label")["style"]["background"]
    point = [floor(center(label, "x")), floor(label["y"] + 8)]
    require(rgba(rows, *point) == label_fill, f"No decoded {target} label fill")
    return {"ring": ring, "focus_pixels": focus_count, "focus_rgba": color, "label": label,
            "label_fill_rgba": label_fill, "text": expected, **ink_sample(frame, rows, text_key)}


def capture_frame(app, plan, title, frame, shapes, *, rtl=False, focused=None):
    shot = capture(app, plan, title)
    require(settle(app) == frame, f"{title} layout changed during transport comparison")
    width, height, rows = read_png((plan.out / shot["name"]).read_bytes())
    require((width, height) == VIEWPORTS["compact" if rtl else "wide"], f"Wrong decoded size for {title}")
    background = node(frame, ROUTE)["style"]["background"]
    require(rgba(rows, 4, 4) == background
            and (max(background[:3]) <= 96 if rtl else min(background[:3]) >= 200),
            "Decoded background disagrees with its light/dark profile")
    samples = {}
    for target, shape in shapes.items():
        fields = {}
        for part, area in shape.items():
            if part in ("hit", "rail", "focus") or not positive(area):
                continue
            require(same_box(layout(frame, f"{target}-{part}")["clip"], area), f"{target}-{part} is clipped")
            fields[part] = paint_piece(rows, frame, target, part, area, background)
        if target in CENTERED and positive(shape["active"]):
            x = floor(center(shape["rail"], "x"))
            y = floor(shape["active"]["y"] + 0.3 * shape["active"]["height"])
            require(rgba(rows, x, y) == background, f"{target} neutral paint cut is missing")
            fields["neutral_gap"] = {"point": [x, y], "rgba": background}
        if target in (CENTERED_DISCRETE, VERTICAL_DISCRETE):
            fields["ticks"] = [
                {"index": index, **paint_piece(rows, frame, target, f"tick-{index}", dot, background)}
                for index, dot in ticks(frame, target, shape, rtl)
            ]
        if focused and focused[0] == target:
            fields["engaged"] = verify_label(rows, frame, target, shape, rtl, focused[1])
        samples[target] = fields
    return {**shot, "background": background, "paint": samples}


def observe_windows(app, samples):
    count = onscreen_windows(app.proc.pid)
    samples.append(count)
    require(count == 0, "The launched Gallery owns a visible window")


def set_value(app, target, request, expected):
    app.wire("Action.setValue", {"id": target, "value": request})
    frame = settle(app)
    focus(frame, target)
    ranged(frame, target, expected)
    return frame, {"request": request, "value": expected}


def keys(app, target, rtl):
    vertical = target in VERTICAL
    initial, increment = (VERTICAL if vertical else CENTERED)[target][1:]
    increasing, decreasing = ("left", "right") if rtl and not vertical else ("right", "left")
    low = 0. if vertical else -100.
    page = 10. if vertical else 20.
    plan = ((increasing, initial + increment), (decreasing, initial),
            ("up", initial + increment), ("down", initial),
            ("page_up", initial + page), ("page_down", initial),
            ("home", low), (decreasing, low), ("end", 100.), (increasing, 100.))
    recorded = []
    for press, expected in plan:
        frame = key(app, press)
        focus(frame, target)
        ranged(frame, target, expected)
        if expected in (low, 100.):
            geometry(frame, target, expected, rtl, engaged=True)
        recorded.append({"key": press, "value": expected})
    return recorded


def replay_profile(plan, profile):
    width, height = VIEWPORTS[profile]
    rtl = profile == "compact"
    samples = []
    result = {"viewport": [width, height], "rtl": rtl, "text_scale": 2 if rtl else 1}
    with launch(plan, profile, width, height) as app:
        observe_windows(app, samples)
        result["opening_tabs"] = focus_by_tab(app, SHOW)
        focus(key(app, "enter"), BACK)
        for setting in (THEME, RTL, TEXT):
            frame = activate(app, setting) if rtl else key(app, "tab")
            focus(frame, setting)
        observe_windows(app, samples)

        cases = {**CENTERED, **VERTICAL}
        result["initial_ranges"] = {target: ranged(frame, target, spec[1])
                                    for target, spec in cases.items()}
        if not rtl:
            rest = {target: geometry(frame, target, spec[1], rtl) for target, spec in cases.items()}
            result["rest"] = {"geometry": rest, "png": capture_frame(app, plan, CAPTURES[0], frame, rest)}

        frame = key(app, "tab")
        focus(frame, NEGATIVE)
        centered_start = geometry(frame, NEGATIVE, -50., rtl, engaged=True)
        chronology = keys(app, NEGATIVE, rtl)
        frame, center_zero = set_value(app, NEGATIVE, 0., 0.)
        zero_shape = geometry(frame, NEGATIVE, 0., rtl, engaged=True)
        frame, center_absolute = set_value(app, NEGATIVE, -37.25, -37.25)
        chosen = geometry(frame, NEGATIVE, -37.25, rtl, engaged=True)
        require(chosen["hit"] == centered_start["hit"] and chosen["rail"] == centered_start["rail"],
                "Centered value changed its hit target or rail")
        title = CAPTURES[3] if rtl else CAPTURES[1]
        photo = capture_frame(app, plan, title, frame, {NEGATIVE: chosen}, rtl=rtl,
                              focused=(NEGATIVE, "−37.3％"))
        result["centered"] = {"keys": chronology, "set_value": [center_zero, center_absolute],
                              "zero_geometry": zero_shape, "geometry": chosen, "png": photo}
        observe_windows(app, samples)

        for target in (NEUTRAL, POSITIVE, CENTERED_DISCRETE):
            frame = key(app, "tab")
            focus(frame, target)
            geometry(frame, target, CENTERED[target][1], rtl, engaged=True)
        frame, absolute = set_value(app, CENTERED_DISCRETE, 37.5, 50.)
        chosen = geometry(frame, CENTERED_DISCRETE, 50., rtl, engaged=True)
        result["centered_discrete"] = {"set_value": absolute, "geometry": chosen,
                                       "ticks": ticks(frame, CENTERED_DISCRETE, chosen, rtl)}

        frame = key(app, "tab")
        focus(frame, VERTICAL_CONTINUOUS)
        scroll = node(frame, SCROLL)["style"]["scroll_y"]
        if rtl:
            require(scroll > 0., "Focus did not reveal the compact vertical section")
        vertical_start = geometry(frame, VERTICAL_CONTINUOUS, 25., rtl, engaged=True)
        require(find(frame, f"{NEGATIVE}-label")[1] is None, "Unfocused Centered bubble still mounted")
        chronology = keys(app, VERTICAL_CONTINUOUS, rtl)
        frame, vertical_absolute = set_value(app, VERTICAL_CONTINUOUS, 37.25, 37.25)
        chosen = geometry(frame, VERTICAL_CONTINUOUS, 37.25, rtl, engaged=True)
        require(chosen["hit"] == vertical_start["hit"] and chosen["rail"] == vertical_start["rail"],
                "Vertical value changed its hit target or rail")
        title = CAPTURES[4] if rtl else CAPTURES[2]
        photo = capture_frame(app, plan, title, frame, {VERTICAL_CONTINUOUS: chosen}, rtl=rtl,
                              focused=(VERTICAL_CONTINUOUS, "37.3％"))
        result["vertical"] = {"keys": chronology, "set_value": vertical_absolute,
                              "scroll_y": scroll, "geometry": chosen, "png": photo}
        observe_windows(app, samples)

        frame = key(app, "tab")
        focus(frame, VERTICAL_DISCRETE)
        frame = key(app, "up")
        focus(frame, VERTICAL_DISCRETE)
        ranged(frame, VERTICAL_DISCRETE, 62.5)
        frame, absolute = set_value(app, VERTICAL_DISCRETE, 43.75, 50.)
        chosen = geometry(frame, VERTICAL_DISCRETE, 50., rtl, engaged=True)
        result["vertical_discrete"] = {"up_value": 62.5, "set_value": absolute,
                                      "geometry": chosen, "ticks": ticks(frame, VERTICAL_DISCRETE, chosen, rtl)}
        app.wire("Action.setValue", {"id": DISABLED, "value": 80.}, expected_error="disabled")
        frame = settle(app)
        ranged(frame, DISABLED, 50.)
        focus(frame, VERTICAL_DISCRETE)
        frame = key(app, "tab")
        focus(frame, BACK)
        focus(key(app, "tab", reverse=True), VERTICAL_DISCRETE)
        focus(activate(app, BACK), SHOW)
        observe_windows(app, samples)

    require(app.proc.returncode is not None, "The launched Gallery child did not exit")
    return {**result, "disabled_set_value": "disabled", "disabled_value": 50.,
            "disabled_skipped_by_tab": True, "closed_focus": SHOW, "visible_windows": samples,
            "gallery_reaped": True}


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
                "crates/surface-app/src/slider.rs", "crates/surface-material/src/standard_slider.rs",
                "crates/surface-material/src/standard_slider/vertical.rs",
                "crates/surface-platform/src/macos_accessibility.rs",
                "examples/material-gallery/src/slider_forms_demo.rs"}
    require(required <= entries.keys(), "Source seal omits the reviewed slider forms")
    for name, checksum in entries.items():
        path = root / name
        require(path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root)
                and digest(path) == checksum, f"Wrong source bytes for {name}")
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == "0.1.55", f"Unexpected source Cargo version {version}")
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
    # These are hand-measured 216 dp fixture rails in native logical pixels.
    def box(x, y, width, height):
        return dict(x=x, y=y, width=width, height=height)

    def fixture(target, value, parts):
        vertical = target in VERTICAL
        name, _, step = (VERTICAL if vertical else CENTERED)[target]
        item = {"stable_id": target, "role": "Slider", "name": name,
                "range": {"min": 0. if vertical else -100., "max": 100., "value": value, "step": step}}
        if vertical:
            item["slider_vertical"] = True
        children = [{"stable_id": f"{target}-{name}"} for name in parts if name != "hit"]
        clip = box(0., 0., 960., 1040.)
        boxes = [{"bounds": bounds, "clip": clip} for bounds in parts.values()]
        return {"dom": {"nodes": [item, *children]}, "layout": {"nodes": boxes}}

    horizontal = {
        "hit": box(20, 80, 240, 48), "rail": box(32, 80, 216, 48),
        "active": box(94, 92, 44, 24), "inactive-min": box(32, 92, 46, 24),
        "inactive-max": box(142, 92, 106, 24), "handle": box(84, 82, 4, 44),
        "focus": box(79, 77, 14, 54), "stop-min": box(36, 102, 4, 4),
        "stop-max": box(240, 102, 4, 4),
    }
    mirror = {name: {**rect, "x": 280 - edge(rect, "x")} for name, rect in horizontal.items()}
    vertical = {
        "hit": box(120, 20, 48, 240), "rail": box(120, 32, 48, 216),
        "active": box(132, 202, 24, 46), "inactive": box(132, 32, 24, 154),
        "handle": box(122, 192, 44, 4), "focus": box(117, 187, 54, 14),
        "stop": box(142, 36, 4, 4),
    }
    assert geometry(fixture(NEGATIVE, -50., horizontal), NEGATIVE, -50., False) == horizontal
    assert geometry(fixture(NEGATIVE, -50., mirror), NEGATIVE, -50., True) == mirror
    for rtl in (False, True):
        assert geometry(fixture(VERTICAL_CONTINUOUS, 25., vertical),
                        VERTICAL_CONTINUOUS, 25., rtl) == vertical

    def rejected(action, message):
        try:
            action()
        except RuntimeError as error:
            require(message in str(error), f"Unexpected self-test failure {error}")
        else:
            raise AssertionError(f"Expected rejection {message}")

    moved = {**horizontal, "active": box(95, 92, 43, 24)}
    rejected(lambda: geometry(fixture(NEGATIVE, -50., moved), NEGATIVE, -50., False), "logical start")
    orientation = fixture(VERTICAL_CONTINUOUS, 25., vertical)
    orientation["dom"]["nodes"][0].pop("slider_vertical")
    rejected(lambda: geometry(orientation, VERTICAL_CONTINUOUS, 25., False), "oriented slider")
    rejected(lambda: geometry(fixture(NEGATIVE, -50., horizontal), NEGATIVE, -50., True), "logical thumb")
    print(json.dumps({"positive_geometry_controls": 4, "negative_geometry_controls": 3,
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
    report = {"schema": "surface.test.slider-forms.native.v1", "source": sources,
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
    window_samples = sum(len(profile["visible_windows"])
                         for recurrence in recurrences for profile in recurrence.values())
    report.update({"rendering": "headless", "reduce_motion": True,
                   "transports": ["Page.screenshot", "CLI", "MCP"],
                   "input_scope": ["Action.key", "Action.setValue", "Action.activate"],
                   "window_samples": window_samples, "replays": recurrences, "byte_identical_pngs": pngs})
    (out / "native-slider-forms-report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    files = sorted(path for path in out.rglob("*") if path.is_file())
    manifest = out / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in files))
    print(json.dumps({"manifest_sha256": digest(manifest), "images_per_replay": len(pngs),
                      "window_samples": window_samples, "visible_windows": 0, "pngs": pngs}, indent=2))


if __name__ == "__main__":
    main()
