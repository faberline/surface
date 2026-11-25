#!/usr/bin/env python3
"""Verify vertical Centered and Standard inset sliders against sealed native inputs."""

import argparse
from hashlib import sha256
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

CENTERED = "vertical-centered-slider-demo"
INSET = "slider-inset-demo"
VIEWPORTS = {"wide": (960, 1040), "compact": (320, 880)}
CENTERED_SAMPLES = {
    "negative": ("負值平衡", -50., 5.),
    "neutral": ("中立平衡", 0., 5.),
    "positive": ("正值平衡", 50., 5.),
    "discrete": ("階梯平衡", -25., 25.),
    "disabled": ("停用平衡", 50., 25.),
}
# Name, starting value, vertical orientation, cross track thickness, handle height.
INSET_SAMPLES = {
    "horizontal-medium": ("中型水平音量", 65., False, 40., 52.),
    "horizontal-large": ("大型水平音量", 0., False, 56., 68.),
    "horizontal-xl": ("特大水平音量", 65., False, 96., 108.),
    "short": ("短軌道音量", 50., False, 40., 52.),
    "vertical-medium": ("中型垂直音量", 65., True, 40., 52.),
    "vertical-large": ("大型垂直音量", 0., True, 56., 68.),
    "vertical-xl": ("特大垂直音量", 65., True, 96., 108.),
    "disabled": ("停用垂直音量", 65., True, 56., 68.),
}
CAPTURES = (
    "centered-wide-light-rest", "centered-wide-light-signed-focus", "centered-compact-dark-rtl-200",
    "inset-wide-light-rest", "inset-wide-light-relocated", "inset-wide-light-minimum",
    "inset-compact-dark-rtl-200-horizontal", "inset-compact-dark-rtl-200-vertical",
)
BINARIES = {"surface-material-gallery": "gallery", "surface-devtools": "cli"}
HEX = re.compile(r"[0-9a-f]{64}")


def close(actual, expected, context):
    require(isclose(actual, expected, rel_tol=0, abs_tol=0.003),
            f"{context}: expected {expected}, got {actual}")


def edge(rect, axis):
    return rect[axis] + rect["width" if axis == "x" else "height"]


def center(rect, axis):
    return (rect[axis] + edge(rect, axis)) / 2.


def positive(rect):
    return rect["width"] > 0 and rect["height"] > 0


def overlaps(left, right):
    return all(max(left[k], right[k]) < min(edge(left, k), edge(right, k)) for k in ("x", "y"))


def value_label(value, signed):
    rounded = floor(abs(value) * 10 + 0.5) / 10
    sign = ("−" if value < 0 else "+" if signed else "") if rounded else ""
    return f"{sign}{rounded:g}％"


def inspect_value(frame, route, sample, value):
    if route == CENTERED:
        name, _, step = CENTERED_SAMPLES[sample]
        low, high, vertical = -100., 100., True
    else:
        name, _, vertical, _, _ = INSET_SAMPLES[sample]
        low, high, step = 0., 100., 5.
    target = f"{route}-{sample}"
    item = node(frame, target)
    expected = {"min": low, "max": high, "value": value, "step": step}
    require(item["role"] == "Slider" and item["name"] == name
            and item.get("slider_vertical", False) is vertical
            and item.get("disabled", False) is (sample == "disabled")
            and item["range"] == expected and item["slider_value_text"] == value_label(value, route == CENTERED),
            f"{target} numeric metadata, value text, orientation or role differs")
    return {"name": name, "vertical": vertical, "range": expected, "text": item["slider_value_text"]}


def shape(frame, route, sample, value, rtl, *, engaged=False):
    info = inspect_value(frame, route, sample, value)
    target = f"{route}-{sample}"
    vertical = info["vertical"]
    axis, cross = ("y", "x") if vertical else ("x", "y")
    major, minor = ("height", "width") if vertical else ("width", "height")
    hit = visible(frame, target, 48.)
    bounds = lambda part: layout(frame, f"{target}-{part}")["bounds"]
    rail = visible(frame, f"{target}-rail")
    grip, ring = bounds("handle"), bounds("focus")
    thickness, handle = (24., 44.) if route == CENTERED else INSET_SAMPLES[sample][-2:]
    if vertical:
        require(hit["height"] == 240., f"{target} target height differs")
    elif sample == "short":
        require(hit["width"] == 88., "The short fixture changed width")
    require(hit[minor] == rail[minor] == max(48., handle), f"{target} cross width changed")
    close(rail[axis], hit[axis] + 12., f"{target} rail physical start")
    close(edge(rail, axis), edge(hit, axis) - 12., f"{target} rail physical end")
    width = 2. if engaged else 4.
    require((grip[major], grip[minor]) == (width, handle)
            and (ring[major], ring[minor]) == (width + 10., handle + 10.),
            "Wrong engaged handle or ring dimensions")
    length = rail[major]
    reverse = vertical or rtl

    def offset(position):
        return edge(rail, axis) - position if reverse else position - rail[axis]

    lo, hi = info["range"]["min"], info["range"]["max"]
    thumb = length * (value - lo) / (hi - lo)
    close(offset(center(grip, axis)), thumb, "Handle's logical value position")
    close(center(grip, cross), center(rail, cross), "Handle cross-axis alignment")
    close(center(ring, axis), center(grip, axis), "Focus ring main-axis alignment")
    close(center(ring, cross), center(grip, cross), "Focus ring cross-axis alignment")
    gap = 6. + width / 2.
    midpoint = length / 2.
    if route == CENTERED:
        spans = {
            "active": (thumb + gap, midpoint - 2.) if value < 0 else (midpoint + 2., thumb - gap),
            "inactive-min": (0., min(thumb - gap, midpoint - 2.)),
            "inactive-max": (max(thumb + gap, midpoint + 2.), length),
        }
        stops = {"stop-min": ("inactive-min", 6.), "stop-max": ("inactive-max", length - 6.)}
    else:
        spans = {"active": (0., thumb - gap), "inactive": (thumb + gap, length)}
        stops = {"stop": ("inactive", length - 6.)}
    pieces = {}
    for part, (start, end) in spans.items():
        rect = bounds(part)
        start, end = max(start, 0.), min(end, length)
        if end - start < (0.001 if part == "active" else 8.):
            require(not positive(rect), f"{target} paints an empty {part}")
            continue
        require(same_box(layout(frame, f"{target}-{part}")["clip"], rect), f"{target} clips {part}")
        close(rect[minor], thickness, f"{part} thickness")
        close(center(rect, cross), center(rail, cross), f"{part} cross center")
        a, b = sorted((offset(rect[axis]), offset(edge(rect, axis))))
        close(a, start, f"{part} logical start")
        close(b, end, f"{part} logical end")
        pieces[part] = rect
    for part, (cap, position) in stops.items():
        rect = bounds(part)
        if cap not in pieces:
            require(not positive(rect), f"{target} paints a stop on a collapsed cap")
            continue
        require(rect["width"] == rect["height"] == 4., f"Wrong size of {part}")
        close(offset(center(rect, axis)), position, f"{part} logical position")
        close(center(rect, cross), center(rail, cross), f"{part} cross center")
        pieces[part] = rect
    if route == CENTERED and sample in ("discrete", "disabled"):
        require(find(frame, f"{target}-tick-0")[1] is None, "Duplicate minimum stop")
        ticks = []
        for index in range(1, 8):
            part = f"tick-{index}"
            rect = bounds(part)
            if not positive(rect):
                continue
            require(index != 4 and rect["width"] == rect["height"] == 4.,
                    "A tick crosses the neutral cut or has the wrong size")
            close(offset(center(rect, axis)), length * index / 8., "Signed tick on its original lattice")
            close(center(rect, cross), center(rail, cross), "Tick cross center")
            require(any(same_box(pieces[name], rect) for name in spans if name in pieces),
                    "A tick falls outside its painted segment")
            pieces[part] = rect
            ticks.append(index)
        require(len(ticks) >= 3, "Too few signed detents in the actual visual rail")
    current = {**info, "hit": hit, "rail": rail, "handle": grip, "focus": ring,
               "pieces": pieces, "neutral_clear": length / 2 if route == CENTERED else None}
    if route == INSET:
        current["icon"] = inspect_icon(frame, target, sample, value, rtl, current)
    return current


def inspect_icon(frame, target, sample, value, rtl, details):
    vertical = details["vertical"]
    axis = "y" if vertical else "x"
    reverse = vertical or rtl
    parts = details["pieces"]
    visible_slots = []
    side = 32. if sample.endswith("-xl") else 24.
    padding = 8. if side == 32. else 6.
    major = "height" if vertical else "width"
    available = lambda name: parts.get(name, {}).get(major, 0.)
    expected = None
    if available("active") >= side + 2. * padding:
        expected = "active"
    elif available("inactive") >= side + 2. * padding + 8.:
        expected = "inactive"
    for name in ("active", "inactive"):
        slot = f"{target}-inset-icon-{name}"
        rect = layout(frame, slot)["bounds"]
        ink = node(frame, f"{slot}-glyph")["fill_contour"]["color"]
        if not positive(rect):
            clip = layout(frame, f"{slot}-glyph")["clip"]
            require(not positive(clip), "Hidden icon still has an open glyph clip")
            continue
        require(rect["width"] == rect["height"] == side, "Wrong measured glyph size")
        require(same_box(layout(frame, f"{slot}-glyph")["clip"], rect), "Glyph clip truncates its artwork")
        require(name in parts and same_box(parts[name], rect, 0.003), "Glyph lies outside its track")
        obstacles = [details["handle"], details["focus"]]
        obstacles.extend(area for part, area in parts.items() if part.startswith(("stop", "tick")))
        for suffix in ("label", "label-text"):
            if find(frame, f"{target}-{suffix}")[1] is not None:
                obstacles.append(layout(frame, f"{target}-{suffix}")["bounds"])
        require(not any(positive(mark) and overlaps(rect, mark) for mark in obstacles),
                "Glyph overlaps a stop, tick, handle, focus ring or label")
        if name == "active":
            nominal = edge(parts[name], axis) - edge(rect, axis) if reverse else rect[axis] - parts[name][axis]
        else:
            stop = parts["stop"]
            nominal = rect[axis] - edge(stop, axis) if reverse else stop[axis] - edge(rect, axis)
        close(nominal, padding, "Glyph token padding at its preferred cap")
        require(ink[3] == (97 if sample == "disabled" else 255), "Wrong glyph opacity")
        points = node(frame, f"{slot}-glyph")["fill_contour"]["points"]
        visible_slots.append({"side": name, "bounds": rect, "ink": ink,
                              "points": len(points), "minimum_glyph": value == 0.})
    if expected is None:
        require(not visible_slots, "Short rail still has a positive icon slot")
        return None
    require(len(visible_slots) == 1 and visible_slots[0]["side"] == expected,
            f"{target} did not place just its {expected} glyph")
    return visible_slots[0]


def rgba(rows, x, y):
    require(0 <= y < len(rows) and 0 <= x < len(rows[0]) // 4, f"PNG sample outside frame at {x},{y}")
    return list(rows[y][4*x:4*(x+1)])


def paint_piece(frame, rows, target, part, rect, vertical, disabled, background):
    x, y = floor(center(rect, "x")), floor(center(rect, "y"))
    if part in ("active", "inactive", "inactive-min", "inactive-max"):
        if vertical:
            x = floor(rect["x"] + rect["width"] * 0.30)
        else:
            y = floor(rect["y"] + rect["height"] * 0.30)
    pixel = rgba(rows, x, y)
    authored = node(frame, f"{target}-{part}")["style"]["background"]
    if disabled:
        require(authored[3] in (31, 97) and pixel != background and pixel[3] == 255,
                f"Disabled {part} ink is missing")
    elif part.startswith("inactive") or part == "active":
        # Inset icons can cover the track center. Try additional known
        # interior positions; none land in a corner or on a stop at the cap.
        locations = [(x, y)]
        axis = "y" if vertical else "x"
        dimension = "height" if vertical else "width"
        for fraction in (0.25, 0.75):
            coordinate = floor(rect[axis] + rect[dimension] * fraction)
            locations.append((x, coordinate) if vertical else (coordinate, y))
        matches = [(px, py) for px, py in locations if rgba(rows, px, py) == authored]
        require(matches, f"{target}-{part} decoded paint differs from authored {authored}")
        x, y = matches[0]
        pixel = authored
    else:
        require(pixel == authored, f"{target}-{part} decoded paint differs from authored {authored}")
    return {"at": [x, y], "actual": pixel, "authored": authored}


def painted_label(frame, rows, target, details, rtl):
    ring = visible(frame, f"{target}-focus")
    badge = visible(frame, f"{target}-label")
    ink = node(frame, f"{target}-focus")["focus_color"]
    points = sum(rgba(rows, x, y) == ink
                 for y in range(ceil(ring["y"]), floor(edge(ring, "y")))
                 for x in range(ceil(ring["x"]), floor(edge(ring, "x"))))
    require(points >= 40, "The visible focus outline has too few painted pixels")
    label_key = f"{target}-label-text"
    require(node(frame, label_key)["text"] == details["text"], "Visible and accessible value labels differ")
    require(badge["height"] >= (64. if rtl else 44.), "Value label did not scale")
    if details["vertical"]:
        clearance = ring["x"] - edge(badge, "x") if rtl else badge["x"] - edge(ring, "x")
    else:
        clearance = ring["y"] - edge(badge, "y")
    close(clearance, 4., "Focus outline clearance to the label")
    color = node(frame, f"{target}-label")["style"]["background"]
    require(rgba(rows, floor(center(badge, "x")), floor(badge["y"] + 8)) == color, "Label fill is missing")
    return {"ring": ring, "ring_pixels": points, "ring_ink": ink, "label": badge,
            "label_ink": color, "text": details["text"], "glyphs": ink_sample(frame, rows, label_key)}


def painted_icon(rows, details, track_color, disabled):
    icon = details["icon"]
    bounds = icon["bounds"]

    def sample(tx, ty):
        x = floor(bounds["x"] + tx * bounds["width"] / 24.)
        y = floor(bounds["y"] + ty * bounds["height"] / 24.)
        return [x, y], rgba(rows, x, y)

    # This body and asymmetric mute-only mark have a safe interior margin
    # in the sealed Gallery contours. Their positions detect rotated art.
    body_at, body = sample(9., 12.)
    mark_at, mark = sample(21., 9.)
    if disabled:
        require(body[3] == 255 and body != track_color, "Disabled glyph does not paint over its track")
    else:
        require(body == icon["ink"], "Measured glyph has no actual body ink")
        require(mark == (icon["ink"] if icon["minimum_glyph"] else track_color),
                "Mute mark does not match the accepted minimum")
    mask = bytes(rgba(rows, x, y) == body
                 for y in range(ceil(bounds["y"]), floor(edge(bounds, "y")))
                 for x in range(ceil(bounds["x"]), floor(edge(bounds, "x"))))
    require(sum(mask) >= 24, "Too little painted icon area")
    return {**icon, "body": {"at": body_at, "pixel": body}, "mute_mark": {"at": mark_at, "pixel": mark},
            "ink_pixel_count": sum(mask), "mask_sha256": sha256(mask).hexdigest()}


def photograph(app, plan, title, frame, route, shapes, rtl, focused=None):
    shot = capture(app, plan, title)
    require(settle(app) == frame, "Frame changed during the transport comparison")
    width, height, rows = read_png((plan.out / shot["name"]).read_bytes())
    require((width, height) == VIEWPORTS["compact" if rtl else "wide"], "Wrong decoded screenshot size")
    background = node(frame, f"{route}-route")["style"]["background"]
    require(rgba(rows, 4, 4) == background
            and (max(background[:3]) <= 96 if rtl else min(background[:3]) >= 200),
            "Headless canvas does not have the requested theme")
    samples = {}
    for name, details in shapes.items():
        target = f"{route}-{name}"
        disabled = name == "disabled"
        samples[name] = {"paint": {
            part: paint_piece(frame, rows, target, part, rect, details["vertical"], disabled, background)
            for part, rect in {**details["pieces"], "handle": details["handle"]}.items()
        }}
        if route == CENTERED and details["range"]["value"] != 0.:
            rail = details["rail"]
            active = details["pieces"].get("active")
            if active:
                x = floor(active["x"] + active["width"] * 0.3)
                y = floor(center(rail, "y"))
                require(rgba(rows, x, y) == background, "The raster has no neutral cut")
                samples[name]["neutral_cut_pixel"] = {"at": [x, y], "rgba": background}
        if details.get("icon"):
            track = details["icon"]["side"]
            surface = samples[name]["paint"][track]["actual"]
            samples[name]["icon"] = painted_icon(rows, details, surface, disabled)
        elif route == INSET and name == "short":
            colors = {
                tuple(node(frame, f"{target}-inset-icon-{slot}-glyph")["fill_contour"]["color"])
                for slot in ("active", "inactive")
            }
            # This fixture has only one existing stop that shares a glyph ink.
            # Check the measured fill areas outside the stop for stray pixels.
            pieces = details["pieces"]
            for part in ("active", "inactive"):
                if part not in pieces:
                    continue
                rect = pieces[part]
                for y in range(ceil(rect["y"]), floor(edge(rect, "y"))):
                    for x in range(ceil(rect["x"]), floor(edge(rect, "x"))):
                        stop = pieces.get("stop")
                        inside_stop = stop and stop["x"] <= x < edge(stop, "x") and stop["y"] <= y < edge(stop, "y")
                        require(inside_stop or tuple(rgba(rows, x, y)) not in colors, "Short rail paints an icon")
        if not rtl:
            samples[name]["caption"] = ink_sample(frame, rows, f"{target}-caption")
        if focused == name:
            samples[name]["value_label"] = painted_label(frame, rows, target, details, rtl)
    return {**shot, "background": background, "samples": samples}


def verify_windowless(app, windows):
    count = onscreen_windows(app.proc.pid)
    windows.append(count)
    require(count == 0, "The owned Gallery process has a visible window")


def check_fixed(before, after):
    require(before["hit"] == after["hit"] and before["rail"] == after["rail"],
            "Value, focus or icon choice moved the hit target or the rail")


def step(app, route, sample, press, value):
    frame = key(app, press)
    focus(frame, f"{route}-{sample}")
    return frame, {"key": press, **inspect_value(frame, route, sample, value)}


def set_value(app, route, sample, requested, expected):
    target = f"{route}-{sample}"
    app.wire("Action.setValue", {"id": target, "value": requested})
    frame = settle(app)
    focus(frame, target)
    return frame, {"request": requested, **inspect_value(frame, route, sample, expected)}


def tab_to(app, route, sample):
    frame = key(app, "tab")
    focus(frame, f"{route}-{sample}")
    return frame


def centered_replay(app, plan, frame, rtl, windows):
    route = CENTERED
    result = {"initial": {target: inspect_value(frame, route, target, data[1])
                          for target, data in CENTERED_SAMPLES.items()}}
    if not rtl:
        rest = {target: shape(frame, route, target, data[1], rtl) for target, data in CENTERED_SAMPLES.items()}
        result["rest"] = {"shape": rest, "png": photograph(app, plan, CAPTURES[0], frame, route, rest, rtl)}
    frame = tab_to(app, route, "negative")
    initial = shape(frame, route, "negative", -50., rtl, engaged=True)
    signed = []
    for press, expected in (("up", -45.), ("left", -50.), ("right", -45.),
                            ("home", -100.), ("end", 100.)):
        frame, receipt = step(app, route, "negative", press, expected)
        signed.append(receipt)
        check_fixed(initial, shape(frame, route, "negative", expected, rtl, engaged=True))
    frame, absolute = set_value(app, route, "negative", -37.25, -37.25)
    checked = shape(frame, route, "negative", -37.25, rtl, engaged=True)
    check_fixed(initial, checked)
    result["signed"] = {"keys": signed, "set_value": absolute, "shape": checked}
    if not rtl:
        result["signed"]["png"] = photograph(app, plan, CAPTURES[1], frame, route, {"negative": checked},
                                              rtl, focused="negative")
    verify_windowless(app, windows)
    tab_to(app, route, "neutral")
    tab_to(app, route, "positive")
    frame = tab_to(app, route, "discrete")
    first = shape(frame, route, "discrete", -25., rtl, engaged=True)
    frame, tie = set_value(app, route, "discrete", -12.5, 0.)
    neutral = shape(frame, route, "discrete", 0., rtl, engaged=True)
    require("active" not in neutral["pieces"], "Neutral still paints an active span")
    frame, jumped = set_value(app, route, "discrete", 34., 25.)
    checked = shape(frame, route, "discrete", 25., rtl, engaged=True)
    check_fixed(first, checked)
    scroll = node(frame, f"{route}-scroll")["style"].get("scroll_y", 0.)
    require(not rtl or scroll > 0., "The compact scroller did not reveal the focused lattice")
    result["discrete"] = {"set_value": [tie, jumped], "neutral": neutral, "shape": checked, "scroll_y": scroll}
    if rtl:
        result["discrete"]["png"] = photograph(app, plan, CAPTURES[2], frame, route, {"discrete": checked},
                                                rtl, focused="discrete")
    app.wire("Action.setValue", {"id": f"{route}-disabled", "value": 75.}, expected_error="disabled")
    inspect_value(settle(app), route, "disabled", 50.)
    tab_to(app, route, "back")
    focus(key(app, "tab", reverse=True), f"{route}-discrete")
    return result


def inset_replay(app, plan, frame, rtl, windows):
    route = INSET
    result = {"initial": {target: inspect_value(frame, route, target, data[1])
                          for target, data in INSET_SAMPLES.items()}}
    if not rtl:
        rest = {target: shape(frame, route, target, data[1], rtl) for target, data in INSET_SAMPLES.items()}
        result["rest"] = {"shape": rest, "png": photograph(app, plan, CAPTURES[3], frame, route, rest, rtl)}
    frame = tab_to(app, route, "horizontal-medium")
    initial = shape(frame, route, "horizontal-medium", 65., rtl, engaged=True)
    result["horizontal"] = {"initial": initial}
    if rtl:
        result["horizontal"]["png"] = photograph(app, plan, CAPTURES[6], frame, route,
                                                  {"horizontal-medium": initial}, rtl, focused="horizontal-medium")
    frame, lowered = set_value(app, route, "horizontal-medium", 5., 5.)
    relocated = shape(frame, route, "horizontal-medium", 5., rtl, engaged=True)
    require(relocated["icon"]["side"] == "inactive" and relocated["icon"]["points"] == initial["icon"]["points"],
            "The 5% fallback did not retain the normal glyph in the inactive track")
    check_fixed(initial, relocated)
    if not rtl:
        result["horizontal"]["relocated_png"] = photograph(app, plan, CAPTURES[4], frame, route,
                                                            {"horizontal-medium": relocated}, rtl, focused="horizontal-medium")
    frame, minimum_key = step(app, route, "horizontal-medium", "home", 0.)
    minimum = shape(frame, route, "horizontal-medium", 0., rtl, engaged=True)
    require(minimum["icon"]["bounds"] == relocated["icon"]["bounds"]
            and minimum["icon"]["points"] != relocated["icon"]["points"], "The minimum failed to change glyph in place")
    if not rtl:
        result["horizontal"]["minimum_png"] = photograph(app, plan, CAPTURES[5], frame, route,
                                                          {"horizontal-medium": minimum}, rtl, focused="horizontal-medium")
        normal_paint = result["horizontal"]["relocated_png"]["samples"]["horizontal-medium"]["icon"]
        muted_paint = result["horizontal"]["minimum_png"]["samples"]["horizontal-medium"]["icon"]
        require(normal_paint["mask_sha256"] != muted_paint["mask_sha256"], "The actual minimum image reuses the normal icon")
    frame, max_key = step(app, route, "horizontal-medium", "end", 100.)
    restored = shape(frame, route, "horizontal-medium", 100., rtl, engaged=True)
    check_fixed(initial, restored)
    require(restored["icon"]["side"] == "active" and restored["icon"]["points"] == initial["icon"]["points"],
            "End did not restore the active normal glyph")
    result["horizontal"].update(set_value=lowered, keys=[minimum_key, max_key],
                                relocated=relocated, minimum=minimum, restored=restored)
    for name in ("horizontal-large", "horizontal-xl"):
        tab_to(app, route, name)
    frame = tab_to(app, route, "short")
    require(shape(frame, route, "short", 50., rtl, engaged=True)["icon"] is None, "Short track gained a glyph with focus")
    before = layout(frame, f"{route}-vertical-medium")["bounds"]
    frame = tab_to(app, route, "vertical-medium")
    upright = shape(frame, route, "vertical-medium", 65., rtl, engaged=True)
    scroll = node(frame, f"{route}-scroll")["style"].get("scroll_y", 0.)
    require(not rtl or (scroll > 0. and upright["hit"]["y"] < before["y"]),
            "The compact scroller did not reveal the vertical target")
    result["vertical"] = {"initial": upright, "scroll_y": scroll}
    if rtl:
        result["vertical"]["png"] = photograph(app, plan, CAPTURES[7], frame, route, {"vertical-medium": upright},
                                                rtl, focused="vertical-medium")
    frame, up = step(app, route, "vertical-medium", "up", 70.)
    check_fixed(upright, shape(frame, route, "vertical-medium", 70., rtl, engaged=True))
    frame, absolute = set_value(app, route, "vertical-medium", 5., 5.)
    relocated = shape(frame, route, "vertical-medium", 5., rtl, engaged=True)
    frame, zero = step(app, route, "vertical-medium", "home", 0.)
    muted = shape(frame, route, "vertical-medium", 0., rtl, engaged=True)
    require(relocated["icon"]["side"] == muted["icon"]["side"] == "inactive"
            and relocated["icon"]["bounds"] == muted["icon"]["bounds"]
            and relocated["icon"]["points"] != muted["icon"]["points"],
            "Vertical glyph did not move to the inactive track and switch at the minimum")
    result["vertical"].update(keys=[up, zero], set_value=absolute, relocated=relocated, minimum=muted)
    verify_windowless(app, windows)
    for name in ("vertical-large", "vertical-xl"):
        tab_to(app, route, name)
    app.wire("Action.setValue", {"id": f"{route}-disabled", "value": 20.}, expected_error="disabled")
    inspect_value(settle(app), route, "disabled", 65.)
    tab_to(app, route, "back")
    focus(key(app, "tab", reverse=True), f"{route}-vertical-xl")
    return result


def replay_profile(plan, route, profile):
    width, height = VIEWPORTS[profile]
    rtl = profile == "compact"
    windows = []
    show, back = f"{route}-show", f"{route}-back"
    with launch(plan, f"{route}-{profile}", width, height) as app:
        verify_windowless(app, windows)
        entries = focus_by_tab(app, show)
        focus(key(app, "enter"), back)
        for setting in ("theme", "rtl", "large-text"):
            target = f"{route}-{setting}"
            frame = activate(app, target) if rtl else key(app, "tab")
            focus(frame, target)
        verify_windowless(app, windows)
        run = centered_replay if route == CENTERED else inset_replay
        details = run(app, plan, frame, rtl, windows)
        closed = activate(app, back)
        focus(closed, show)
        require(find(closed, f"{route}-scroll")[1] is None, "The route is still mounted after Back")
        verify_windowless(app, windows)
    require(app.proc.returncode is not None, "The Gallery child did not exit")
    return {"viewport": [width, height], "rtl": rtl, "text_scale": 2 if rtl else 1,
            "opening_tabs": entries, "tests": details, "closed_focus": show,
            "disabled_value_rejected": True, "disabled_skipped_by_tab": True,
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
    required = {"Cargo.toml", "Cargo.lock", "crates/surface/src/layout.rs",
                "crates/surface-material/src/standard_slider/vertical.rs",
                "crates/surface-material/src/standard_slider/inset_icon.rs",
                "examples/material-gallery/src/vertical_centered_slider_demo.rs",
                "examples/material-gallery/src/slider_inset_demo.rs"}
    require(required <= entries.keys(), "Source seal omits the reviewed refinement routes")
    for name, checksum in entries.items():
        path = root / name
        require(path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root)
                and digest(path) == checksum, f"Wrong source bytes for {name}")
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == "0.1.57", f"Unexpected source Cargo version {version}")
    return {"manifest_sha256": expected, "files_verified": len(entries), "version": version}


def replay(paths, out):
    out.mkdir(mode=0o700)
    plan = SimpleNamespace(**paths, out=out)
    return {f"{route}-{profile}": replay_profile(plan, route, profile)
            for route in (CENTERED, INSET) for profile in VIEWPORTS}


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

    def fixture(route, sample, value, parts, rail_axis):
        vertical = route == CENTERED
        spec = (CENTERED_SAMPLES if vertical else INSET_SAMPLES)[sample]
        target = f"{route}-{sample}"
        info = {"stable_id": target, "role": "Slider", "name": spec[0], "slider_vertical": vertical,
                "range": {"min": -100. if vertical else 0., "max": 100., "step": 5., "value": value},
                "slider_value_text": value_label(value, vertical)}
        view = box(0, 0, 320, 420)
        nodes = [info]
        areas = [{"bounds": rail_axis, "clip": view}]
        for part, rect in parts.items():
            nodes.append({"stable_id": f"{target}-{part}"})
            areas.append({"bounds": rect, "clip": view})
            if part.startswith("inset-icon-"):
                nodes.append({"stable_id": f"{target}-{part}-glyph",
                              "fill_contour": {"color": [21, 30, 40, 255], "points": [[1, 1], [4, 1], [1, 4]]}})
                areas.append({"bounds": rect, "clip": rect})
        return {"dom": {"nodes": nodes}, "layout": {"nodes": areas}}

    vertical = {
        "rail": box(80, 112, 48, 216), "handle": box(82, 272, 44, 4), "focus": box(77, 267, 54, 14),
        "active": box(92, 222, 24, 44), "inactive-min": box(92, 282, 24, 46),
        "inactive-max": box(92, 112, 24, 106), "stop-min": box(102, 320, 4, 4),
        "stop-max": box(102, 116, 4, 4),
    }
    check = fixture(CENTERED, "negative", -50., vertical, box(80, 100, 48, 240))
    for rtl in False, True:
        require(shape(check, CENTERED, "negative", -50., rtl)["pieces"]["active"] == vertical["active"],
                "The vertical rail must always place the minimum at the bottom")

    horizontal = {
        "rail": box(32, 80, 216, 52), "handle": box(138, 80, 4, 52), "focus": box(133, 75, 14, 62),
        "active": box(32, 86, 100, 40), "inactive": box(148, 86, 100, 40), "stop": box(240, 104, 4, 4),
        "inset-icon-active": box(38, 94, 24, 24), "inset-icon-inactive": box(0, 0, 0, 0),
    }
    hframe = fixture(INSET, "horizontal-medium", 50., horizontal, box(20, 80, 240, 52))
    require(shape(hframe, INSET, "horizontal-medium", 50., False)["icon"]["side"] == "active",
            "A measured active track can hold the icon")

    def rejected(action, message):
        try:
            action()
        except RuntimeError as error:
            require(message in str(error), f"Unexpected negative control {error}")
        else:
            raise AssertionError(f"Expected rejection {message}")

    broken = deepcopy(check)
    node(broken, f"{CENTERED}-negative")["slider_vertical"] = False
    rejected(lambda: shape(broken, CENTERED, "negative", -50., False), "orientation or role")
    broken_gap = {**vertical, "active": box(92, 221, 24, 45)}
    wrong = fixture(CENTERED, "negative", -50., broken_gap, box(80, 100, 48, 240))
    rejected(lambda: shape(wrong, CENTERED, "negative", -50., False), "active logical end")
    overlapping = {**horizontal, "inset-icon-active": box(0, 0, 0, 0),
                   "inset-icon-inactive": box(217, 94, 24, 24)}
    wrong = fixture(INSET, "horizontal-medium", 50., overlapping, box(20, 80, 240, 52))
    rejected(lambda: shape(wrong, INSET, "horizontal-medium", 50., False), "Glyph overlaps")

    ink, track = [43, 21, 13, 255], [191, 176, 163, 255]
    icon = {"bounds": box(0, 0, 24, 24), "ink": ink, "minimum_glyph": True}
    rows = [bytearray(track * 24) for _ in range(24)]
    for y in range(7, 13):
        for x in range(6, 12):
            rows[y][4*x:4*(x+1)] = bytes(ink)
    rows[9][84:88] = bytes(ink)
    minimum = painted_icon(rows, {"icon": icon}, track, False)
    rejected(lambda: painted_icon(rows, {"icon": {**icon, "minimum_glyph": False}}, track, False),
             "Mute mark does not match")
    rows[9][84:88] = bytes(track)
    normal = painted_icon(rows, {"icon": {**icon, "minimum_glyph": False}}, track, False)
    require(normal["mask_sha256"] != minimum["mask_sha256"], "Synthetic masks should differ")
    print(json.dumps({"positive_controls": 5, "negative_controls": 4, "executables_launched": 0}, indent=2))

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
    report = {"schema": "surface.test.slider-refinements.native.v1", "source": sources,
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
    children = sum(profile["gallery_reaped"] for recurrence in recurrences for profile in recurrence.values())
    require(window_samples == 32 and children == 8, "Wrong number of process and window observations")
    report.update({"rendering": "headless", "reduce_motion": True,
                   "transports": ["Page.screenshot", "CLI", "MCP"],
                   "input_scope": ["Action.key", "Action.setValue", "Action.activate"],
                   "window_samples": window_samples, "owned_processes_reaped": children,
                   "replays": recurrences, "byte_identical_pngs": pngs})
    (out / "native-slider-refinements-report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    files = sorted(path for path in out.rglob("*") if path.is_file())
    manifest = out / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in files))
    print(json.dumps({"manifest_sha256": digest(manifest), "images_per_replay": len(pngs),
                      "window_samples": window_samples, "visible_windows": 0, "pngs": pngs}, indent=2))


if __name__ == "__main__":
    main()
