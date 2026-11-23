#!/usr/bin/env python3
"""Replay Material reorderable lists in the offscreen Gallery."""

import argparse
import json
from pathlib import Path

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle
from window_check import onscreen_windows

DEMO = "list-reorder-demo"
LIST = "list-reorder-items"
SCROLL = "list-reorder-scroll"
INITIAL_STATUS = "尚未調整順序"
ROWS = tuple(f"list-reorder-{row}" for row in (*"abcdefghi", "fixed", "j", "k"))
TITLES = dict(zip(ROWS, ("筆記", "山徑", "地圖", "車站", "行程", "相簿",
                         "文件", "旅館", "清單", "固定分隔", "票券", "聯絡人")))
A, B, C, D = ROWS[:4]
I, FIXED, J = ROWS[8:11]
TOLERANCE = .5
MIN_TARGET = 48. - TOLERANCE
REPORT_NAME = "native-list-reorder-report.json"
IMAGE_NAMES = {
    "reorder-segmented-light-before.png",
    "reorder-segmented-light-after.png",
    "reorder-standard-dark.png",
    "reorder-baseline-light.png",
    "reorder-compact-dark-rtl.png",
    "reorder-short-scroll.png",
}


def scene(app, *settings):
    activate(app, "section-2")
    activate(app, f"{DEMO}-show")
    for preset in settings:
        activate(app, f"{DEMO}-{preset}")
    frame = settle(app)
    require(node(frame, DEMO)["role"] == "Window", "Wrong Gallery scene")
    require(node(frame, SCROLL)["name"] == "可排序清單捲動區", "Wrong named scroller")
    source = node(frame, LIST)
    require(source["role"] == "List" and source["name"] == "旅程資料順序", "Wrong list semantics")
    assert_order(frame, ROWS)
    assert_status(frame, INITIAL_STATUS)
    require(all(find(frame, f"{LIST}-{kind}")[0] is None for kind in ("lift", "drop")),
            "Passive drag overlays were mounted at rest")
    return frame


def assert_status(frame, wanted):
    actual = node(frame, f"{DEMO}-status")["text"]
    require(actual == wanted, f"Expected Gallery status {wanted!r}. Got {actual!r}")


def bounds(frame, identifier):
    return layout(frame, identifier)["bounds"]


def follow(app, stroke, identifier):
    frame = key(app, stroke)
    focus(frame, identifier)
    return frame


def measured_order(frame):
    parent, _ = find(frame, LIST)
    items = [item["stable_id"].removesuffix("-item") for item in frame["dom"]["nodes"]
             if item.get("parent") == parent and item["role"] == "ListItem"]
    for before, after in zip(items, items[1:]):
        before_box = bounds(frame, f"{before}-item")
        after_box = bounds(frame, f"{after}-item")
        require(before_box["y"] + before_box["height"] <= after_box["y"] + TOLERANCE,
                "Visual and semantic row order differ")
    return items


def assert_order(frame, wanted):
    actual = measured_order(frame)
    require(actual == list(wanted), f"Wrong order: expected {wanted}; got {actual}")
    return actual


def target(frame, row, suffix=None, *, disabled=False):
    identifier = f"{row}-{suffix}" if suffix else row
    label = {"up": "上移", "down": "下移"}.get(suffix, "") + TITLES[row]
    source = node(frame, identifier)
    require(source["role"] == "Button" and source["name"] == label and source["disabled"] is disabled,
            f"Wrong button semantics for {identifier}: {source}")
    box = bounds(frame, identifier)
    require(min(box["width"], box["height"]) >= MIN_TARGET, f"Small button {identifier}: {box}")
    return box


def intersection(*rects):
    left = max(rect["x"] for rect in rects)
    top = max(rect["y"] for rect in rects)
    right = min(rect["x"] + rect["width"] for rect in rects)
    bottom = min(rect["y"] + rect["height"] for rect in rects)
    return {"x": left, "y": top, "width": max(0., right - left), "height": max(0., bottom - top)}


def fully_visible(frame, identifier, *, min_size=MIN_TARGET):
    measured = layout(frame, identifier)
    scroller = layout(frame, SCROLL)
    clip = intersection(frame["layout"]["viewport"], measured["clip"],
                        scroller["bounds"], scroller["clip"])
    box = measured["bounds"]
    require(min(box["width"], box["height"]) >= min_size and same_box(clip, box),
            f"Expected fully visible {identifier}. Got {box} in {clip}")
    return box


def measure_row(frame, row, *, up_disabled=False, down_disabled=False):
    shell = bounds(frame, f"{row}-item")
    grip_id = f"{row}-grip"
    grip = node(frame, grip_id)
    require(grip["role"] == "Group" and grip["name"] == "" and grip.get("tab_index") is None
            and grip["disabled"] is (up_disabled and down_disabled),
            f"Grip should be an unnamed drag-only target: {grip}")
    grip_bounds = bounds(frame, grip_id)
    require(min(grip_bounds["width"], grip_bounds["height"]) >= MIN_TARGET, "Small drag grip")
    rects = {"row": shell, row: target(frame, row),
             f"{row}-up": target(frame, row, "up", disabled=up_disabled),
             f"{row}-down": target(frame, row, "down", disabled=down_disabled), grip_id: grip_bounds}
    for identifier, rect in rects.items():
        require(same_box(shell, rect), f"{identifier} escaped its stable ListItem")
    hit_rects = list(rects.values())[1:]
    for index, first in enumerate(hit_rects):
        for second in hit_rects[index + 1:]:
            overlap = intersection(first, second)
            require(min(overlap["width"], overlap["height"]) <= TOLERANCE, "Row hit targets overlap")
    return rects


def shot(app, plan, title, frame, *, words=()):
    before = onscreen_windows(app.proc.pid)
    captured = capture(app, plan, title, frame=frame, sample_targets=words)
    after = onscreen_windows(app.proc.pid)
    logical = frame["layout"]["viewport"]
    require((captured["width"], captured["height"]) == (logical["width"], logical["height"]),
            f"{title} PNG differs from the logical viewport")
    require(before == after == 0, f"{title} showed a Gallery window")
    captured["visible_windows"] = [before, after]
    return captured


def focus_outline(plan, frame, capture_result, identifier):
    rect = fully_visible(frame, identifier)
    source = node(frame, identifier)
    require(source.get("focus_outline_width") == 3, "Missing 3 dp inside outline")
    ink = source["focus_color"]
    width, height, rows = read_png((plan.out / capture_result["name"]).read_bytes())
    x = round(rect["x"] + rect["width"] / 2)
    y = round(rect["y"])
    require(0 <= x < width and 0 <= y <= height - 5, "Focus samples outside the PNG")
    samples = [list(rows[i][4 * x:4 * x + 4]) for i in range(y, y + 5)]
    matches = [max(abs(actual - wanted) for actual, wanted in zip(color, ink)) <= 3
               for color in samples]
    require(matches == [True, True, True, False, False], f"Wrong 3 dp focus stroke: {samples}")
    return {"samples": samples, "rgba": ink}


def segmented(app, plan):
    scene(app)
    focus_by_tab(app, A)
    follow(app, "down", f"{A}-down")
    follow(app, "down", B)
    frame = follow(app, "down", f"{B}-up")
    poses = measure_row(frame, B)
    a = bounds(frame, f"{A}-item")
    gap = poses["row"]["y"] - a["y"] - a["height"]
    require(abs(gap - 2.) <= TOLERANCE, f"Wrong Segmented gap: {gap}")
    before = shot(app, plan, "reorder-segmented-light-before", frame, words=(f"{B}-headline",))
    before["focus_outline"] = focus_outline(plan, frame, before, f"{B}-up")

    frame = follow(app, "enter", f"{B}-down")
    order = assert_order(frame, (B, A, *ROWS[2:]))
    assert_status(frame, "已將山徑移到第 1 項")
    moved = measure_row(frame, B, up_disabled=True)
    require(moved["row"]["y"] < poses["row"]["y"], "Moved row did not move upwards")
    after = shot(app, plan, "reorder-segmented-light-after", frame, words=(f"{B}-headline",))
    after["focus_outline"] = focus_outline(plan, frame, after, f"{B}-down")

    blocked = {}
    for identifier in (f"{FIXED}-up", f"{FIXED}-down", f"{I}-down", f"{J}-up", f"{B}-up"):
        blocked[identifier] = app.wire("Action.activate", {"id": identifier}, expected_error="disabled")
    frame = settle(app)
    assert_order(frame, order)
    assert_status(frame, "已將山徑移到第 1 項")
    focus(frame, f"{B}-down")
    follow(app, "up", B)
    frame = follow(app, "enter", B)
    assert_status(frame, "已開啟山徑")
    assert_order(frame, order)
    return {"row_bounds_before": poses, "row_bounds_after": moved, "gap_dp": gap,
            "initial_order": list(ROWS), "new_order": order, "blocked": blocked,
            "move_focus_fallback": [f"{B}-up", f"{B}-down"], "independent_primary": B,
            "transport_captures": [before, after]}


def themed_style(app, plan, style, *, dark):
    scene(app, *((style, "theme") if dark else (style,)))
    focus_by_tab(app, A)
    frame = follow(app, "down", f"{A}-down")
    geometry = measure_row(frame, A, up_disabled=True)
    fill = node(frame, DEMO)["style"]["background"]
    require((sum(fill[:3]) / 3 < 128) is dark, f"Unexpected display theme for {style}")
    icon = bounds(frame, f"{A}-down-icon")
    want_icon = 24. if style == "baseline" else 20.
    require(abs(icon["width"] - want_icon) <= TOLERANCE and abs(icon["height"] - want_icon) <= TOLERANCE,
            f"Wrong icon dimensions for {style}: {icon}")
    receipt = shot(app, plan, f"reorder-{style}-{'dark' if dark else 'light'}",
                   frame, words=(f"{A}-headline",))
    frame = follow(app, "space", f"{A}-down")
    order = assert_order(frame, (B, A, *ROWS[2:]))
    assert_status(frame, "已將筆記移到第 2 項")
    return {"style": style, "dark": dark, "page_fill": fill, "row_geometry": geometry,
            "move_icon": icon, "changed_order": order, "transport_captures": [receipt]}


def standard(app, plan):
    return themed_style(app, plan, "standard", dark=True)


def baseline(app, plan):
    return themed_style(app, plan, "baseline", dark=False)


def reach_third_row(app):
    focus_by_tab(app, A)
    for identifier in (f"{A}-down", B, f"{B}-up", f"{B}-down"):
        follow(app, "down", identifier)
    return follow(app, "down", C)


def compact(app, plan):
    scene(app, "theme", "rtl", "large-text")
    frame = reach_third_row(app)
    boxes = measure_row(frame, C)
    primary = fully_visible(frame, C)
    words = fully_visible(frame, f"{C}-supporting", min_size=1.)
    require(words["height"] > 40. and same_box(primary, words), "Scaled supporting text did not wrap")
    require(boxes[f"{C}-up"]["x"] > boxes[f"{C}-down"]["x"] > boxes[f"{C}-grip"]["x"],
            "Move and drag targets did not mirror in RTL")
    require(primary["y"] + primary["height"] <= boxes[f"{C}-grip"]["y"] + TOLERANCE,
            "Compact move and drag targets did not wrap below the primary")
    frame = follow(app, "down", f"{C}-up")
    hit = fully_visible(frame, f"{C}-up")
    require(abs(hit["width"] - 48.) <= TOLERANCE and abs(hit["height"] - 48.) <= TOLERANCE,
            "Compact move target is not 48 by 48 dp")
    receipt = shot(app, plan, "reorder-compact-dark-rtl", frame, words=(f"{C}-supporting",))
    receipt["focus_outline"] = focus_outline(plan, frame, receipt, f"{C}-up")
    frame = follow(app, "enter", f"{C}-up")
    order = assert_order(frame, (A, C, B, *ROWS[3:]))
    assert_status(frame, "已將地圖移到第 2 項")
    return {"rtl": True, "text_scale": 2, "primary": primary, "move_target": hit,
            "supporting_text": words, "row_geometry": boxes, "order": order, "transport_captures": [receipt]}


def scroll_offsets(frame):
    return {identifier: node(frame, identifier)["style"]["scroll_y"] for identifier in (DEMO, SCROLL)}


def short(app, plan):
    initial = scene(app, "theme", "rtl", "large-text")
    before = scroll_offsets(initial)
    reach_third_row(app)
    follow(app, "down", f"{C}-up")
    frame = follow(app, "down", f"{C}-down")
    after = scroll_offsets(frame)
    area = bounds(frame, SCROLL)
    require(111.5 <= area["height"] <= 272.5, f"Unexpected scroller height: {area}")
    require(after[SCROLL] > before[SCROLL], "Focus did not scroll the named list")
    assert_status(frame, INITIAL_STATUS)
    hit = fully_visible(frame, f"{C}-down")
    receipt = shot(app, plan, "reorder-short-scroll", frame)
    receipt["focus_outline"] = focus_outline(plan, frame, receipt, f"{C}-down")
    frame = follow(app, "enter", f"{C}-down")
    order = assert_order(frame, (A, B, D, C, *ROWS[4:]))
    assert_status(frame, "已將地圖移到第 4 項")
    return {"rtl": True, "text_scale": 2, "scroller_bounds": area, "full_move_target": hit,
            "scroll_y": {"before": before, "focused": after, "moved": scroll_offsets(frame)},
            "order": order, "move": f"{C}-down", "transport_captures": [receipt]}


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
    profiles = ((segmented, 760, 640), (standard, 760, 640),
                (baseline, 760, 640), (compact, 320, 900), (short, 320, 240))
    for replay, width, height in profiles:
        profile = replay.__name__
        with launch(plan, profile, width, height) as app:
            before = onscreen_windows(app.proc.pid)
            require(before == 0, f"{profile} opened a visible window before replay")
            proof = replay(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(after == 0, f"{profile} opened a visible window during replay")
            proof["visible_windows"] = [before, after]
            proof["viewport"] = [width, height]
        require(app.proc.poll() is not None, f"{profile} did not close the Gallery")
        report["profiles"][profile] = proof
        print(f"PASS {profile} {width}x{height}. No visible windows.", flush=True)
    require({path.name for path in plan.out.iterdir()} == IMAGE_NAMES, "Native PNG set differs")
    (plan.out / REPORT_NAME).write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
