#!/usr/bin/env python3
"""Verify Material list corner endpoints in the native headless Gallery."""

import argparse
import json
import math
from pathlib import Path

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle
from window_check import onscreen_windows

DEMO = "lists-demo"
LIST = "lists-items"
SCROLL = "lists-scroll"
A, B, C, D, E, F = ("lists-" + letter for letter in "abcdef")
D_NAME = "長文字會在窄畫面自動換行"
MEDIA = "list-media-demo"
MEDIA_LIST = "list-media-items"
MEDIA_SCROLL = "list-media-scroll"
EARTH = "list-media-earth"
POSTER = "list-media-poster"
PROBE = "list-media-probe"
REPORT_NAME = "native-list-ripple-report.json"
IMAGE_NAMES = {
    "list-ripple-segmented-light-rest.png",
    "list-ripple-segmented-light-focus.png",
    "list-ripple-standard-dark-focus.png",
    "list-ripple-baseline-light.png",
    "list-ripple-compact-dark-rtl.png",
    "list-ripple-short-scroll.png",
    "list-ripple-media-focus.png",
}
MIN_TARGET = 48.
COLOR_TOLERANCE = 3


def bounds(frame, stable_id):
    return layout(frame, stable_id)["bounds"]


def intersection(*rects):
    left = max(r["x"] for r in rects)
    top = max(r["y"] for r in rects)
    right = min(r["x"] + r["width"] for r in rects)
    bottom = min(r["y"] + r["height"] for r in rects)
    return {"x": left, "y": top, "width": max(0., right-left), "height": max(0., bottom-top)}


def visible_part(frame, stable_id, scroller=SCROLL):
    target = layout(frame, stable_id)
    scroll = layout(frame, scroller)
    return intersection(target["bounds"], target["clip"], scroll["bounds"], scroll["clip"],
                        frame["layout"]["viewport"])


def full_box(frame, stable_id, *, scroller=SCROLL, minimum=1.):
    rect = bounds(frame, stable_id)
    shown = visible_part(frame, stable_id, scroller)
    require(min(rect["width"], rect["height"]) >= minimum and same_box(shown, rect),
            f"{stable_id} is too small or clipped: {rect=} {shown=}")
    return rect


def follow(app, stroke, stable_id):
    frame = key(app, stroke)
    focus(frame, stable_id)
    return frame


def scene(app, *settings, demo=DEMO):
    activate(app, "section-2")
    activate(app, f"{demo}-show")
    for setting in settings:
        activate(app, f"{demo}-{setting}")
    frame = settle(app)
    label = "清單" if demo == DEMO else "影像清單"
    list_id, scroll_id = (LIST, SCROLL) if demo == DEMO else (MEDIA_LIST, MEDIA_SCROLL)
    name = "書籤" if demo == DEMO else "太空影像"
    require(node(frame, demo)["role"] == "Window" and node(frame, demo)["name"] == label,
            f"Wrong Gallery scene for {demo}")
    require(node(frame, list_id)["name"] == name and node(frame, scroll_id)["style"]["clip"],
            "Wrong named list or missing scroll clip")
    return frame


def assert_style(frame, demo, dark):
    rgba = node(frame, demo)["style"]["background"]
    require((sum(rgba[:3]) / 3. < 128.) is dark, f"Unexpected display theme: {rgba}")
    return rgba


def assert_status(frame, wanted):
    actual = node(frame, f"{DEMO}-status")["text"]
    require(actual == wanted, f"Expected status {wanted!r}. Got {actual!r}")


def assert_button(frame, stable_id, name, *, disabled=False):
    row = node(frame, stable_id)
    require(row["role"] == "Button" and row["name"] == name and row["disabled"] is disabled,
            f"Wrong native command semantics for {stable_id}")
    rect = bounds(frame, stable_id)
    require(min(rect["width"], rect["height"]) >= MIN_TARGET, f"{stable_id} is a small target")


def shot(app, plan, title, frame, *, labels=()):
    before = onscreen_windows(app.proc.pid)
    result = capture(app, plan, title, frame=frame, sample_targets=labels)
    after = onscreen_windows(app.proc.pid)
    viewport = frame["layout"]["viewport"]
    require((result["width"], result["height"]) == (viewport["width"], viewport["height"]),
            f"Logical capture size changed for {title}")
    require(before == after == 0, "A screenshot opened a visible Gallery window")
    result["visible_windows"] = [before, after]
    return result


def png_rows(plan, shot_receipt):
    _, _, rows = read_png((plan.out / shot_receipt["name"]).read_bytes())
    return rows


def rgba(rows, x, y):
    require(0 <= y < len(rows) and 0 <= x < len(rows[0]) // 4, f"Outside PNG: ({x}, {y})")
    return tuple(rows[y][x * 4:x * 4 + 4])


def near(first, second):
    return max(abs(a - b) for a, b in zip(first, second)) <= COLOR_TOLERANCE


def over(foreground, background):
    alpha = foreground[3]
    return tuple(round((src * alpha + dst * (255 - alpha)) / 255)
                 for src, dst in zip(foreground[:3], background[:3])) + (255,)


def corner_distance(x, y, radius):
    # Signed distance to the top-left corner at a pixel's center. The other
    # corners use mirrored coordinates. Stay away from antialiased boundaries.
    qx = radius - (x + .5)
    qy = radius - (y + .5)
    return math.hypot(max(0., qx), max(0., qy)) + min(0., max(qx, qy)) - radius


def corner_colors(frame, stable_id, demo, focused):
    row = node(frame, stable_id)
    style = row["style"]
    backing = tuple(node(frame, demo)["style"]["background"])
    fill = over(style["background"], backing)
    if not focused:
        return {"exterior": backing, "interior": fill}
    require(row.get("focus_outline_width") == 3, "Missing 3 dp inside focus outline")
    ink = tuple(row["focus_color"])
    state = style["state_layer"]
    tint = over((*state[:3], state[3] * 26 // 255), fill)
    require(tint != fill, f"{stable_id} gives no observable focus state layer")
    return {"exterior": backing, "stroke": ink, "interior": tint}


def contour(plan, frame, receipt, stable_id, *, radius, focused,
            demo=DEMO, scroller=SCROLL):
    rect = full_box(frame, stable_id, scroller=scroller, minimum=MIN_TARGET)
    x, y, width, height = (round(rect[k]) for k in ("x", "y", "width", "height"))
    rows = png_rows(plan, receipt)
    colors = corner_colors(frame, stable_id, demo, focused)
    counts = dict.fromkeys(colors, 0)
    samples = {}
    for corner in ("top_left", "top_right"):
        for dy in range(12):
            for dx in range(12):
                distance = corner_distance(dx, dy, radius)
                if distance >= .85:
                    kind = "exterior"
                elif focused and -2.15 <= distance <= -.85:
                    kind = "stroke"
                elif distance <= (-3.85 if focused else -.85):
                    kind = "interior"
                else:
                    continue
                xx = x + dx if corner == "top_left" else x + width - 1 - dx
                actual = rgba(rows, xx, y + dy)
                require(near(actual, colors[kind]),
                        f"{stable_id} radius {radius} {corner} ({dx}, {dy}) {kind}: "
                        f"got {actual}, expected {colors[kind]}, distance {distance:.2f}")
                counts[kind] += 1
                samples.setdefault(kind, {"offset": [dx, dy], "rgba": actual})
    require(counts["interior"] >= 30, f"No painted interior for {stable_id}")
    if radius:
        require(counts["exterior"] > 0, "Rounded row never left its outer corner clear")
    if focused:
        center = x + (width * 3) // 4
        strip = [rgba(rows, center, y + index) for index in range(5)]
        mask = [near(pixel, colors["stroke"]) for pixel in strip]
        require(mask == [True, True, True, False, False], f"Wrong 3 dp focus thickness: {strip}")
        require(counts["stroke"] >= 6, "Missing arc stroke evidence")
    else:
        strip = None
    return {
        "bounds": rect, "expected_radius_dp": radius,
        "expected_outline_width_dp": 3 if focused else 0,
        "expected_rgba": colors, "classified_pixels": counts,
        "example_samples": samples, "flat_outline_samples": strip,
    }


def reach_d(app):
    focus_by_tab(app, A)
    follow(app, "down", C)
    return follow(app, "down", D)


def segmented(app, plan):
    frame = scene(app, "segmented", "action")
    assert_style(frame, DEMO, False)
    assert_button(frame, B, "海", disabled=True)
    assert_button(frame, D, D_NAME)
    assert_status(frame, "最近操作：無")
    before = shot(app, plan, "list-ripple-segmented-light-rest", frame)
    rest = contour(plan, frame, before, D, radius=4., focused=False)
    frame = reach_d(app)
    after = shot(app, plan, "list-ripple-segmented-light-focus", frame)
    focused = contour(plan, frame, after, D, radius=16., focused=True)
    require(rest["bounds"] == focused["bounds"], "Corner change moved the layout rectangle")
    rejected = app.wire("Action.activate", {"id": B}, expected_error="disabled")
    frame = follow(app, "enter", D)
    assert_status(frame, f"最近操作：{D_NAME}")
    return {"row": D, "disabled_skip": [A, C, D], "blocked": rejected,
            "rest_endpoint": rest, "focus_endpoint": focused, "captures": [before, after]}


def standard(app, plan):
    frame = scene(app, "standard", "theme", "select")
    assert_style(frame, DEMO, True)
    require(node(frame, LIST)["role"] == "RadioGroup" and node(frame, C)["checked"],
            "Single selection lost its native radio group or initial row")
    focus_by_tab(app, C)
    frame = follow(app, "up", A)
    require(node(frame, C)["checked"] and not node(frame, A)["checked"], "Arrows selected a row")
    result = shot(app, plan, "list-ripple-standard-dark-focus", frame)
    endpoint = contour(plan, frame, result, A, radius=16., focused=True)
    frame = follow(app, "space", A)
    require(node(frame, A)["checked"] and not node(frame, C)["checked"], "Selection did not move")
    assert_status(frame, "已選取：山")
    return {"focus_endpoint": endpoint, "old_selection": C, "new_selection": A, "captures": [result]}


def baseline(app, plan):
    frame = scene(app, "baseline", "passive")
    assert_style(frame, DEMO, False)
    require(node(frame, A)["role"] == "ListItem" and node(frame, A).get("tab_index") is None,
            "Passive row introduced a control")
    activate(app, f"{DEMO}-action")
    focus_by_tab(app, A)
    frame = settle(app)
    result = shot(app, plan, "list-ripple-baseline-light", frame)
    endpoint = contour(plan, frame, result, A, radius=0., focused=True)
    return {"passive_role": "ListItem", "focus_endpoint": endpoint, "captures": [result]}


def compact(app, plan):
    frame = scene(app, "theme", "rtl", "large-text", "multi")
    assert_style(frame, DEMO, True)
    require(node(frame, LIST)["role"] == "Group", "Wrong native multi-select role")
    require(node(frame, C)["checked"] and node(frame, F)["checked"] and not node(frame, D)["checked"],
            "Unexpected initial multiple selection")
    focus_by_tab(app, C)
    frame = follow(app, "down", D)
    require(node(frame, D)["role"] == "Checkbox" and node(frame, D)["name"] == D_NAME,
            "Wrapped row lost its accessible name or checkbox role")
    assert_status(frame, "已選取 2 項")
    row = full_box(frame, D, minimum=MIN_TARGET)
    headline = full_box(frame, f"{D}-headline")
    support = full_box(frame, f"{D}-supporting")
    leading = full_box(frame, f"{D}-leading")
    mark = full_box(frame, f"{D}-mark")
    require(headline["height"] > 48. and support["height"] > 40., "200% text did not wrap")
    require(mark["x"] < headline["x"] < leading["x"] and row["width"] == 296.,
            "The compact row lost its RTL slot order or allocated width")
    require((mark["width"], mark["height"]) == (18., 18.), "Wrong checkbox size")
    result = shot(app, plan, "list-ripple-compact-dark-rtl", frame, labels=(f"{D}-supporting",))
    endpoint = contour(plan, frame, result, D, radius=16., focused=True)
    frame = follow(app, "space", D)
    require(all(node(frame, row)["checked"] for row in (C, D, F)), "Old checkboxes changed")
    assert_status(frame, "已選取 3 項")
    return {"rtl": True, "text_scale": 2, "headline": headline, "supporting": support,
            "leading": leading, "mark": mark, "focus_endpoint": endpoint,
            "selection": [C, D, F], "captures": [result]}


def scroll_offsets(frame):
    return {stable_id: node(frame, stable_id)["style"]["scroll_y"] for stable_id in (DEMO, SCROLL)}


def short(app, plan):
    frame = scene(app, "rtl", "large-text", "action")
    start = scroll_offsets(frame)
    reach_d(app)
    frame = follow(app, "down", E)
    assert_button(frame, E, "島")
    end = scroll_offsets(frame)
    require(end[SCROLL] > start[SCROLL], "Keyboard did not scroll the short list")
    shown = visible_part(frame, E)
    require(min(shown["width"], shown["height"]) >= MIN_TARGET,
            f"Short row has no usable hit region on screen: {shown}")
    result = shot(app, plan, "list-ripple-short-scroll", frame)
    frame = follow(app, "enter", E)
    assert_status(frame, "最近操作：島")
    return {"rtl": True, "text_scale": 2, "target": E, "visible_target": shown,
            "scroll_y": {"initial": start, "focused": end}, "captures": [result]}


def media(app, plan):
    frame = scene(app, "action", demo=MEDIA)
    require(node(frame, MEDIA_LIST)["role"] == "List", "Wrong media list outer role")
    assert_button(frame, PROBE, "色彩測試圖")
    focus_by_tab(app, EARTH)
    follow(app, "down", POSTER)
    frame = follow(app, "down", PROBE)
    art_id = f"{PROBE}-media"
    art = full_box(frame, art_id, scroller=MEDIA_SCROLL)
    require((art["width"], art["height"]) == (56., 56.), "Wrong media size")
    icon = node(frame, art_id)
    require(icon["role"] == "Group" and icon["name"] == "" and icon.get("tab_index") is None,
            "Decorative art acquired a control")
    result = shot(app, plan, "list-ripple-media-focus", frame)
    endpoint = contour(plan, frame, result, PROBE, radius=16., focused=True,
                       demo=MEDIA, scroller=MEDIA_SCROLL)
    rows = png_rows(plan, result)
    x, y = round(art["x"]), round(art["y"])
    pixels = {
        "corner": rgba(rows, x + 1, y + 1),
        "green": rgba(rows, x + 14, y + 14),
        "blue": rgba(rows, x + 42, y + 14),
    }
    tint = endpoint["expected_rgba"]["interior"]
    require(near(pixels["corner"], tint), "Focused state filled the independent raster corner")
    require(near(pixels["green"], (19, 190, 82, 255)) and near(pixels["blue"], (27, 91, 238, 255)),
            "Focused state changed the real image pixels or crop")
    return {"raster_bounds": art, "raster_radius_dp": 8, "color_samples": pixels,
            "focus_endpoint": endpoint, "captures": [result]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--gallery", required=True, type=Path)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--source", required=True)
    parser.add_argument("--out", required=True, type=Path)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True)
    report = {
        "source_manifest_sha256": plan.source,
        "gallery_sha256": digest(plan.gallery),
        "cli_sha256": digest(plan.cli),
        "motion": "native headless Reduce Motion endpoints",
        "input": "native headless DevTools Action.key and Action.activate",
        "transports": ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"],
        "profiles": {},
    }
    profiles = ((segmented, 960, 760), (standard, 960, 760), (baseline, 960, 760),
                (compact, 320, 900), (short, 320, 240), (media, 960, 760))
    for replay, width, height in profiles:
        name = replay.__name__
        with launch(plan, name, width, height) as app:
            before = onscreen_windows(app.proc.pid)
            require(before == 0, f"{name} created a visible window")
            proof = replay(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(after == 0, f"{name} created a visible window")
            proof["visible_windows"] = [before, after]
            proof["viewport"] = [width, height]
        require(app.proc.poll() is not None, f"{name} did not close the Gallery")
        report["profiles"][name] = proof
        print(f"PASS {name} {width}x{height}. No visible windows.", flush=True)
    require({path.name for path in plan.out.iterdir()} == IMAGE_NAMES, "Native PNG set differs")
    (plan.out / REPORT_NAME).write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
