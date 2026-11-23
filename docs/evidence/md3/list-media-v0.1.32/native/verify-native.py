#!/usr/bin/env python3
"""Replay raster list media in the native headless Gallery."""

import argparse
import json
from pathlib import Path

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle
from window_check import onscreen_windows

DEMO = "list-media-demo"
LIST = "list-media-items"
SCROLL = "list-media-scroll"
DETAIL = "list-media-detail"
EARTH = "list-media-earth"
POSTER = "list-media-poster"
PROBE = "list-media-probe"
TEXT = "list-media-text"
DISABLED = "list-media-disabled"
ROWS = (EARTH, POSTER, PROBE, TEXT, DISABLED)
TITLES = dict(zip(ROWS, ("藍色彈珠", "紐西蘭上空", "色彩測試圖", "觀測筆記", "封存影像")))
MIN_TARGET = 47.5
REPORT_NAME = "native-list-media-report.json"
IMAGE_NAMES = {
    "media-baseline-light.png",
    "media-segmented-light.png",
    "media-standard-dark.png",
    "media-poster-detail.png",
    "media-compact-dark-rtl.png",
    "media-short-scroll.png",
}

GREEN = (19, 190, 82, 255)
BLUE = (27, 91, 238, 255)
YELLOW = (250, 205, 41, 255)
RED = (239, 23, 64, 128)
GUARDS = ((231, 32, 176, 255), (255, 167, 16, 255))


def bounds(frame, identifier):
    return layout(frame, identifier)["bounds"]


def intersection(*rects):
    left = max(rect["x"] for rect in rects)
    top = max(rect["y"] for rect in rects)
    right = min(rect["x"] + rect["width"] for rect in rects)
    bottom = min(rect["y"] + rect["height"] for rect in rects)
    return {"x": left, "y": top, "width": max(0., right-left), "height": max(0., bottom-top)}


def visible_part(frame, identifier, *, in_list=True):
    item = layout(frame, identifier)
    clips = [item["clip"], frame["layout"]["viewport"]]
    if in_list:
        scroll = layout(frame, SCROLL)
        clips += [scroll["bounds"], scroll["clip"]]
    return intersection(item["bounds"], *clips)


def full_box(frame, identifier, *, in_list=True):
    rect = bounds(frame, identifier)
    visible = visible_part(frame, identifier, in_list=in_list)
    require(min(rect["width"], rect["height"]) > 0. and same_box(visible, rect),
            f"{identifier} is clipped: {rect} in {visible}")
    return rect


def scene(app, *settings):
    activate(app, "section-2")
    activate(app, f"{DEMO}-show")
    for preset in settings:
        activate(app, f"{DEMO}-{preset}")
    frame = settle(app)
    require(node(frame, DEMO)["role"] == "Window", "Wrong Gallery scene")
    require(node(frame, SCROLL)["name"] == "影像清單捲動區", "Missing named media scroller")
    require(node(frame, LIST)["name"] == "太空影像", "Wrong name for the media list")
    return frame


def measure_media(frame, *, baseline, mode="passive", wide=True, rtl=False):
    role = {"passive": "ListItem", "action": "Button", "select": "Checkbox"}[mode]
    require(node(frame, LIST)["role"] == ("Group" if mode == "select" else "List"),
            "Wrong media list role")
    require(find(frame, f"{TEXT}-media")[0] is None, "Text-only row gained a media child")
    sizes = [(80., 80.), (114., 64.), (56., 56.), (80., 80.)] if wide else [
        (56., 56.), (100., 56.), (56., 56.), (56., 56.),
    ]
    result = {}
    for row_id in ROWS:
        row = node(frame, row_id)
        require(row["role"] == role, f"Unexpected native role for {row_id}")
        if mode != "passive":
            require(row["name"] == TITLES[row_id], f"Wrong accessible name {row_id}")
            require(row["disabled"] is (row_id == DISABLED), f"Wrong disabled state {row_id}")
            target = bounds(frame, row_id)
            require(min(target["width"], target["height"]) >= MIN_TARGET, f"Small hit {row_id}")
        if mode == "select":
            mark = bounds(frame, f"{row_id}-mark")
            require((mark["width"], mark["height"]) == (18., 18.), "Wrong checkbox indicator size")
    for row_id, (width, height) in zip((EARTH, POSTER, PROBE, DISABLED), sizes):
        image_id = f"{row_id}-media"
        image = node(frame, image_id)
        require(image["role"] == "Group" and not image["name"] and image.get("tab_index") is None,
                f"Unexpected control on passive {image_id}")
        row = bounds(frame, row_id)
        box = bounds(frame, image_id)
        label = bounds(frame, f"{row_id}-headline")
        require((box["width"], box["height"]) == (width, height), f"Wrong raster profile {image_id}: {box}")
        require(same_box(row, box) and same_box(row, label), f"Media slots left the row {row_id}")
        start = row["x"] + row["width"] - box["x"] - box["width"] if rtl else box["x"] - row["x"]
        gap = box["x"] - label["x"] - label["width"] if rtl else label["x"] - box["x"] - box["width"]
        want_start = (0. if mode == "passive" else 3.) if baseline and row_id == POSTER else 16.
        require(abs(start - want_start) <= .5 and abs(gap - (16. if baseline else 12.)) <= .5,
                f"Wrong logical leading placement {image_id}: {start=}, {gap=}")
        require(label["width"] > box["width"], f"{image_id} consumed the widest slot")
        result[row_id] = {"raster": box, "headline": label, "leading_inset": start, "copy_gap": gap}
    return result


def shot(app, plan, title, frame, *, labels=()):
    before = onscreen_windows(app.proc.pid)
    receipt = capture(app, plan, title, frame=frame, sample_targets=labels)
    after = onscreen_windows(app.proc.pid)
    viewport = frame["layout"]["viewport"]
    require((receipt["width"], receipt["height"]) == (viewport["width"], viewport["height"]),
            f"Wrong logical PNG size for {title}")
    require(before == after == 0, "A screenshot opened a visible Gallery window")
    receipt["visible_windows"] = [before, after]
    return receipt


def pixel(rows, x, y):
    require(0 <= x < len(rows[0]) // 4 and 0 <= y < len(rows), f"Sample outside PNG ({x}, {y})")
    return tuple(rows[y][4*x:4*x+4])


def distance(first, second):
    return max(abs(a - b) for a, b in zip(first, second))


def decode(plan, capture_result):
    _, _, rows = read_png((plan.out / capture_result["name"]).read_bytes())
    return rows


def raster_pixels(plan, frame, receipt, *, square):
    rows = decode(plan, receipt)
    rect = full_box(frame, f"{PROBE}-media")
    x, y, width, height = (round(rect[key]) for key in ("x", "y", "width", "height"))
    quarter, three_quarters = width // 4, 3 * width // 4
    upper, lower = height // 4, 3 * height // 4
    backing = pixel(rows, x + width + 4, y + height // 2)
    expected_alpha = tuple(round((foreground * 128 + background * 127) / 255)
                           for foreground, background in zip(RED[:3], backing[:3])) + (255,)
    actual = (pixel(rows, x + quarter, y + upper), pixel(rows, x + three_quarters, y + upper),
              pixel(rows, x + quarter, y + lower), pixel(rows, x + three_quarters, y + lower))
    expected = (GREEN, BLUE, expected_alpha, YELLOW)
    require(all(distance(a, b) <= 3 for a, b in zip(actual, expected)),
            f"Crop, orientation, or alpha mismatch: {actual}; expected {expected}")
    # Stay outside the 8 dp radius, past the crop boundary's filtered texels.
    corner = pixel(rows, x + 1, y + 1)
    require(distance(corner, GREEN if square else backing) <= 3,
            f"Corner mask does not match this list style: {corner}")
    for yy in range(y + 3, y + height - 3):
        for xx in range(x + 3, x + width - 3):
            require(all(distance(pixel(rows, xx, yy), guard) > 12 for guard in GUARDS),
                    f"An outside crop guard band leaked into ({xx}, {yy})")
    detail_colors = {}
    for row in (EARTH, POSTER):
        box = full_box(frame, f"{row}-media")
        px, py, pw, ph = (round(box[key]) for key in ("x", "y", "width", "height"))
        samples = {pixel(rows, px + i, py + j)
                   for i in range(pw // 4, 3 * pw // 4, 2)
                   for j in range(ph // 4, 3 * ph // 4, 2)}
        require(len(samples) > 30, f"Expected details from the actual photo/poster in {row}")
        detail_colors[row] = len(samples)
    return {"crop_guard": "outside bands excluded", "square": square, "corner": corner,
            "rgba_quadrants": actual, "expected_rgba_quadrants": expected, "backing": backing,
            "sampled_photo_colors": detail_colors}


def follow(app, stroke, identifier):
    frame = key(app, stroke)
    focus(frame, identifier)
    return frame


def passive_profile(app, plan, style, *, dark):
    settings = (style, "theme") if dark else (style,)
    frame = scene(app, *settings)
    color = node(frame, DEMO)["style"]["background"]
    require((sum(color[:3]) / 3 < 128) is dark, f"Unexpected display theme for {style}")
    baseline = style == "baseline"
    geometry = measure_media(frame, baseline=baseline)
    frame = settle(app)
    name = f"media-{style}-{'dark' if dark else 'light'}"
    capture_result = shot(app, plan, name, frame)
    pixels = raster_pixels(plan, frame, capture_result, square=baseline)
    return {"style": style, "dark": dark, "media_geometry": geometry, "raster_pixels": pixels,
            "captures": [capture_result]}


def baseline(app, plan):
    return passive_profile(app, plan, "baseline", dark=False)


def segmented(app, plan):
    return passive_profile(app, plan, "segmented", dark=False)


def standard(app, plan):
    receipt = passive_profile(app, plan, "standard", dark=True)
    frame = activate(app, f"{DEMO}-action")
    geometry = measure_media(frame, baseline=False, mode="action")
    target = focus_by_tab(app, EARTH)
    frame = follow(app, "down", POSTER)
    full_box(frame, POSTER)
    frame = key(app, "enter")
    detail = node(frame, DETAIL)
    require(detail["role"] == "Dialog" and detail["name"] == TITLES[POSTER], "Wrong detail modal")
    hero = full_box(frame, f"{DETAIL}-image", in_list=False)
    require(hero["width"] >= 100. and hero["height"] >= 48., "Detail lacks an image")
    require("ISS042-E-170757" in node(frame, f"{DETAIL}-text")["text"], "Wrong detail content")
    receipt["captures"].append(shot(app, plan, "media-poster-detail", frame))
    frame = follow(app, "escape", POSTER)
    require(find(frame, DETAIL)[0] is None, "Escape did not close the detail")
    blocked = app.wire("Action.activate", {"id": DISABLED}, expected_error="disabled")
    receipt["action"] = {"geometry": geometry, "tab_path": target, "opened": TITLES[POSTER],
                         "detail_raster": hero, "restored_focus": POSTER, "disabled_reply": blocked}
    return receipt


def checked(frame, expected):
    controls = [row for row in ROWS if node(frame, row).get("checked") is True]
    require(controls == list(expected), f"Wrong checked rows: {controls}")
    require(node(frame, f"{DEMO}-status")["text"] == f"已選 {len(expected)} 項", "Wrong selection count")
    return controls


def compact(app, plan):
    frame = scene(app, "theme", "rtl", "large-text", "select")
    geometry = measure_media(frame, baseline=False, mode="select", wide=False, rtl=True)
    checked(frame, ())
    focus_by_tab(app, EARTH)
    frame = follow(app, "space", EARTH)
    checked(frame, (EARTH,))
    follow(app, "down", POSTER)
    frame = follow(app, "enter", POSTER)
    selected = checked(frame, (EARTH, POSTER))
    rect = full_box(frame, POSTER)
    artwork = full_box(frame, f"{POSTER}-media")
    support = full_box(frame, f"{POSTER}-supporting")
    mark = full_box(frame, f"{POSTER}-mark")
    require(support["height"] > 40. and same_box(rect, support), "Scaled copy did not wrap")
    require(mark["x"] + mark["width"] < support["x"] < artwork["x"], "Wrong RTL slot order")
    capture_result = shot(app, plan, "media-compact-dark-rtl", frame, labels=(f"{POSTER}-supporting",))
    return {"rtl": True, "text_scale": 2, "media_geometry": geometry, "target": rect, "media": artwork,
            "supporting_copy": support, "mark": mark, "selected": selected, "captures": [capture_result]}


def scroll_offsets(frame):
    return {identifier: node(frame, identifier)["style"]["scroll_y"] for identifier in (DEMO, SCROLL)}


def short(app, plan):
    frame = scene(app, "rtl", "large-text", "select")
    before = scroll_offsets(frame)
    focus_by_tab(app, EARTH)
    follow(app, "down", POSTER)
    frame = follow(app, "down", PROBE)
    after = scroll_offsets(frame)
    require(after[SCROLL] > before[SCROLL], "Short focus did not scroll the named list")
    target = visible_part(frame, PROBE)
    require(min(target["width"], target["height"]) >= MIN_TARGET,
            f"Focused short row has no usable on-screen target: {target}")
    art = visible_part(frame, f"{PROBE}-media")
    require(min(art["width"], art["height"]) >= 8., "Short row did not reveal real image area")
    frame = follow(app, "space", PROBE)
    selected = checked(frame, (PROBE,))
    capture_result = shot(app, plan, "media-short-scroll", frame)
    return {"rtl": True, "text_scale": 2, "scroll_y": {"before": before, "focused": after},
            "visible_target": target, "visible_art": art, "selected": selected,
            "captures": [capture_result]}


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
    profiles = ((baseline, 960, 760), (segmented, 960, 760), (standard, 960, 760),
                (compact, 320, 900), (short, 320, 240))
    for replay, width, height in profiles:
        name = replay.__name__
        with launch(plan, name, width, height) as app:
            before = onscreen_windows(app.proc.pid)
            require(before == 0, f"{name} opened a visible window before replay")
            proof = replay(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(after == 0, f"{name} opened a visible window during replay")
            proof["visible_windows"] = [before, after]
            proof["viewport"] = [width, height]
        require(app.proc.poll() is not None, f"{name} did not close the Gallery")
        report["profiles"][name] = proof
        print(f"PASS {name} {width}x{height}. No visible windows.", flush=True)
    require({path.name for path in plan.out.iterdir()} == IMAGE_NAMES, "Native PNG set differs")
    (plan.out / REPORT_NAME).write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
