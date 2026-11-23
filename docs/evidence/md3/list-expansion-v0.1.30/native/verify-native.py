#!/usr/bin/env python3
"""Replay expandable Material lists in the offscreen Gallery."""

import argparse
import json
from pathlib import Path

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

DEMO = "list-expand-demo"
LIST = "list-expand-items"
SCROLL = "list-expand-scroll"
BEFORE = f"{DEMO}-before"
AFTER = f"{DEMO}-after"
HEADER = "list-expand-header"
BRANCH = f"{HEADER}-branch"
BODY = f"{HEADER}-expansion-content"
INDICATOR = f"{HEADER}-indicator"
CHILD_A = "list-expand-child-a"
CHILD_B = "list-expand-child-b"
DISABLED = "list-expand-child-disabled"
A_NAME = "步道地圖與無障礙路線"
B_NAME = "下載地圖"
CHILDREN = ((CHILD_A, A_NAME, False), (DISABLED, "暫停行程", True), (CHILD_B, B_NAME, False))
MIN_TARGET = 47.5
TOLERANCE = .5
REPORT_NAME = "native-list-expansion-report.json"
IMAGE_NAMES = {
    "expansion-segmented-light-closed.png",
    "expansion-segmented-light-open.png",
    "expansion-segmented-light-escape.png",
    "expansion-standard-dark.png",
    "expansion-baseline-light.png",
    "expansion-compact-dark-rtl.png",
    "expansion-short-outer-scroll.png",
}


def scene(app, *settings):
    activate(app, "section-2")
    activate(app, f"{DEMO}-show")
    for preset in settings:
        activate(app, f"{DEMO}-{preset}")
    frame = settle(app)
    require(node(frame, DEMO)["role"] == "Window", "Wrong Gallery scene")
    require(node(frame, SCROLL)["name"] == "可展開清單捲動區", "Wrong named scroller")
    status(frame, None)
    return frame


def status(frame, label):
    wanted = "尚未操作" if label is None else f"最近操作：{label}"
    actual = node(frame, f"{DEMO}-status")["text"]
    require(actual == wanted, f"Expected Gallery status {wanted!r}. Got {actual!r}")


def bounds(frame, identifier):
    return layout(frame, identifier)["bounds"]


def follow(app, stroke, identifier):
    frame = key(app, stroke)
    focus(frame, identifier)
    return frame


def enter_header(app):
    focus_by_tab(app, BEFORE)
    follow(app, "tab", "list-expand-before")
    return follow(app, "down", HEADER)


def has_ancestor(frame, identifier, ancestor):
    nodes = frame["dom"]["nodes"]
    ancestor_idx, _ = find(frame, ancestor)
    index = node(frame, identifier).get("parent")
    while index is not None:
        if index == ancestor_idx:
            return True
        index = nodes[index].get("parent")
    return False


def target(frame, identifier, name, *, disabled=False):
    source = node(frame, identifier)
    require(source["role"] == "Button" and source["name"] == name
            and source["disabled"] is disabled, f"Wrong button semantics for {identifier}")
    rect = bounds(frame, identifier)
    require(min(rect["width"], rect["height"]) >= MIN_TARGET, f"Small target {identifier}: {rect}")
    return rect


def disclosure(frame, expanded):
    group = node(frame, LIST)
    require(group["role"] == "List" and group["name"] == "旅行清單", "Wrong native list")
    branch = node(frame, BRANCH)
    require(branch["role"] == "ListItem" and branch.get("expanded") is None, "Wrong branch role")
    require(has_ancestor(frame, BRANCH, LIST) and has_ancestor(frame, HEADER, BRANCH),
            "Disclosure is outside its list item")
    source = node(frame, HEADER)
    require(source.get("disclosure") is True and source.get("expanded") is expanded,
            f"Expected explicit disclosure state {expanded}")
    indicator = node(frame, INDICATOR)
    require(indicator["role"] == "Group" and indicator.get("tab_index") is None,
            "Chevron should be a passive visual")
    require(same_box(target(frame, HEADER, "旅行行程"), bounds(frame, INDICATOR)),
            "Chevron left the disclosure control")
    return {"expanded": source["expanded"], "header_name": source["name"],
            "header_role": source["role"], "branch_role": branch["role"],
            "indicator": bounds(frame, INDICATOR)}


def children(frame, expanded):
    if not expanded:
        for identifier in (BODY, CHILD_A, CHILD_B, DISABLED):
            require(find(frame, identifier)[0] is None, f"{identifier} remains mounted when closed")
        return {}
    body = bounds(frame, BODY)
    rows = {}
    edge = body["y"]
    for identifier, name, disabled in CHILDREN:
        rect = target(frame, identifier, name, disabled=disabled)
        require(has_ancestor(frame, identifier, BODY) and not has_ancestor(frame, identifier, HEADER),
                f"Nested button {identifier}")
        require(same_box(body, rect), f"Child target escaped expansion body: {identifier}")
        require(edge <= rect["y"] + TOLERANCE, "Child targets overlap")
        edge = rect["y"] + rect["height"]
        rows[identifier] = rect
    return rows


def pose(frame):
    branch = bounds(frame, BRANCH)
    header = bounds(frame, HEADER)
    following = bounds(frame, "list-expand-after")
    return {"branch": branch, "header": header, "following": following,
            "extent": branch["height"] - header["height"],
            "following_offset": following["y"] - branch["y"]}


def verify_inline(closed, opened, body):
    delta = opened["extent"] - closed["extent"]
    require(abs(closed["extent"]) <= TOLERANCE, f"Closed body has a visible extent: {closed}")
    require(delta > 100 and abs(delta - body["height"]) <= TOLERANCE,
            f"Open body did not reach its full natural height: {opened}, {body}")
    require(abs(opened["header"]["height"] - closed["header"]["height"]) <= TOLERANCE
            and abs(opened["following_offset"] - closed["following_offset"] - delta) <= TOLERANCE,
            f"Following row did not move with the inline expansion: {closed}, {opened}")


def verify_closed(before, after):
    require(abs(before["extent"] - after["extent"]) <= TOLERANCE
            and abs(before["following_offset"] - after["following_offset"]) <= TOLERANCE,
            f"Collapse did not restore the closed geometry: {before}, {after}")


def shot(app, plan, title, frame, *, ink=()):
    before = onscreen_windows(app.proc.pid)
    captured = capture(app, plan, title, frame=frame, sample_targets=ink)
    after = onscreen_windows(app.proc.pid)
    logical = frame["layout"]["viewport"]
    require((captured["width"], captured["height"]) == (logical["width"], logical["height"]),
            f"{title} PNG differs from the logical viewport")
    require(before == after == 0, f"{title} showed a Gallery window")
    captured["visible_windows"] = [before, after]
    return captured


def outline_sample(plan, frame, capture_result):
    rect = visible(frame, HEADER, MIN_TARGET)
    source = node(frame, HEADER)
    require(source.get("focus_outline_width") == 3, "Missing 3 dp focus outline")
    ink = source["focus_color"]
    width, height, rows = read_png((plan.out / capture_result["name"]).read_bytes())
    x = round(rect["x"] + rect["width"] / 2)
    y = round(rect["y"])
    require(0 <= x < width and 0 <= y <= height - 5, "Focus samples are outside the PNG")
    samples = [list(rows[i][4*x:4*x+4]) for i in range(y, y + 5)]
    matches = [max(abs(actual - wanted) for actual, wanted in zip(color, ink)) <= 3
               for color in samples]
    require(matches == [True, True, True, False, False], f"Wrong 3 dp inside focus stroke: {samples}")
    return {"samples": samples, "expected_focus_color": ink}


def segmented(app, plan):
    scene(app)
    frame = enter_header(app)
    closed_info = disclosure(frame, False)
    children(frame, False)
    closed = pose(frame)
    blocked_in_closed = app.wire("Action.activate", {"id": CHILD_A}, expected_error="missing_node")
    closed_shot = shot(app, plan, "expansion-segmented-light-closed", frame, ink=(f"{HEADER}-headline",))
    closed_shot["focus_outline"] = outline_sample(plan, frame, closed_shot)
    frame = follow(app, "space", HEADER)
    open_info = disclosure(frame, True)
    opened = pose(frame)
    verify_inline(closed, opened, bounds(frame, BODY))
    rows = children(frame, True)
    status(frame, None)
    frame = follow(app, "down", CHILD_A)
    child_a = visible(frame, CHILD_A, MIN_TARGET)
    open_shot = shot(app, plan, "expansion-segmented-light-open", frame, ink=(f"{CHILD_A}-headline",))
    frame = follow(app, "down", CHILD_B)
    child_b = visible(frame, CHILD_B, MIN_TARGET)
    blocked_disabled = app.wire("Action.activate", {"id": DISABLED}, expected_error="disabled")
    frame = settle(app)
    focus(frame, CHILD_B)
    status(frame, None)
    frame = follow(app, "enter", CHILD_B)
    status(frame, B_NAME)
    frame = follow(app, "escape", HEADER)
    disclosure(frame, False)
    children(frame, False)
    status(frame, B_NAME)
    restored = pose(frame)
    verify_closed(closed, restored)
    escape_shot = shot(app, plan, "expansion-segmented-light-escape", frame)
    escape_shot["focus_outline"] = outline_sample(plan, frame, escape_shot)
    follow(app, "down", "list-expand-after")
    follow(app, "tab", AFTER)
    return {"closed": closed, "open": opened, "disclosures": [closed_info, open_info],
            "restored": restored, "child_bounds": rows,
            "fully_visible_child_targets": {CHILD_A: child_a, CHILD_B: child_b},
            "disabled_skip": [CHILD_A, CHILD_B],
            "rejected_actions": {CHILD_A: blocked_in_closed, DISABLED: blocked_disabled},
            "independent_command": CHILD_B, "external_escape_focus": [CHILD_B, HEADER],
            "transport_captures": [closed_shot, open_shot, escape_shot]}


def themed_style(app, plan, style, *, dark):
    scene(app, *(("theme", style) if dark else (style,)))
    frame = enter_header(app)
    color = node(frame, DEMO)["style"]["background"]
    require((sum(color[:3]) / 3 < 128) is dark, f"Unexpected theme for {style}: {color}")
    closed = pose(frame)
    disclosure(frame, False)
    children(frame, False)
    frame = follow(app, "enter", HEADER)
    disclosure(frame, True)
    opened = pose(frame)
    verify_inline(closed, opened, bounds(frame, BODY))
    boxes = children(frame, True)
    frame = follow(app, "down", CHILD_A)
    visible(frame, CHILD_A, MIN_TARGET)
    receipt = shot(app, plan, f"expansion-{style}-{'dark' if dark else 'light'}",
                   frame, ink=(f"{CHILD_A}-headline",))
    frame = follow(app, "escape", HEADER)
    children(frame, False)
    verify_closed(closed, pose(frame))
    return {"style": style, "dark": dark, "page_fill": color, "closed": closed,
            "open": opened, "child_bounds": boxes, "transport_captures": [receipt]}


def standard(app, plan):
    return themed_style(app, plan, "standard", dark=True)


def baseline(app, plan):
    return themed_style(app, plan, "baseline", dark=False)


def compact(app, plan):
    scene(app, "theme", "rtl", "large-text")
    frame = enter_header(app)
    closed = pose(frame)
    support = bounds(frame, f"{HEADER}-supporting")
    indicator = bounds(frame, INDICATOR)
    require(support["height"] > 48 and same_box(closed["header"], support),
            f"Supporting text did not wrap inside its disclosure: {support}")
    require(indicator["x"] + indicator["width"] <= support["x"] + TOLERANCE,
            "Indicator did not move to the logical end in RTL")
    frame = follow(app, "space", HEADER)
    verify_inline(closed, pose(frame), bounds(frame, BODY))
    frame = follow(app, "down", CHILD_A)
    hit = visible(frame, CHILD_A, MIN_TARGET)
    headline = visible(frame, f"{CHILD_A}-headline")
    require(headline["height"] > 48 and same_box(hit, headline),
            f"Child headline did not wrap: {headline}")
    receipt = shot(app, plan, "expansion-compact-dark-rtl", frame, ink=(f"{CHILD_A}-headline",))
    frame = activate(app, CHILD_A)
    focus(frame, CHILD_A)
    status(frame, A_NAME)
    frame = follow(app, "escape", HEADER)
    children(frame, False)
    verify_closed(closed, pose(frame))
    return {"rtl": True, "text_scale": 2, "indicator": indicator, "header_support": support,
            "full_child_target": hit, "full_child_headline": headline,
            "independent_command": CHILD_A, "transport_captures": [receipt]}


def intersection(*rects):
    left = max(rect["x"] for rect in rects)
    top = max(rect["y"] for rect in rects)
    right = min(rect["x"] + rect["width"] for rect in rects)
    bottom = min(rect["y"] + rect["height"] for rect in rects)
    return {"x": left, "y": top, "width": max(0, right-left), "height": max(0, bottom-top)}


def scroll_offsets(frame):
    return {identifier: node(frame, identifier)["style"]["scroll_y"] for identifier in (DEMO, SCROLL)}


def short(app, plan):
    initial = scene(app, "theme", "rtl", "large-text")
    initial_offsets = scroll_offsets(initial)
    enter_header(app)
    follow(app, "space", HEADER)
    follow(app, "down", CHILD_A)
    frame = follow(app, "down", CHILD_B)
    hit = target(frame, CHILD_B, B_NAME)
    viewport = frame["layout"]["viewport"]
    scroller = layout(frame, SCROLL)
    require(119.5 <= scroller["bounds"]["height"] <= 236.5,
            f"Unexpected authored scroll height: {scroller}")
    absolute_clip = intersection(viewport, scroller["bounds"], scroller["clip"])
    visible_hit = intersection(hit, layout(frame, CHILD_B)["clip"], absolute_clip)
    require(min(visible_hit["width"], visible_hit["height"]) >= MIN_TARGET,
            f"Short child did not reveal a usable target: {visible_hit} from {hit}")
    focused_offsets = scroll_offsets(frame)
    require(focused_offsets[SCROLL] > initial_offsets[SCROLL], "The child list did not scroll")
    status(frame, None)
    receipt = shot(app, plan, "expansion-short-outer-scroll", frame)
    frame = follow(app, "enter", CHILD_B)
    status(frame, B_NAME)
    frame = follow(app, "tab", AFTER)
    footer = visible(frame, AFTER, MIN_TARGET)
    footer_offsets = scroll_offsets(frame)
    require(max(focused_offsets[DEMO], footer_offsets[DEMO]) > initial_offsets[DEMO],
            "No measured outer page scroll while revealing the action or footer")
    require(same_box(viewport, footer), "After-list Tab target is outside the short viewport")
    return {"rtl": True, "text_scale": 2, "scroller_bounds": scroller["bounds"],
            "absolute_scroller_clip": absolute_clip, "visible_child_hit": visible_hit,
            "visible_footer_target": footer, "enabled_command": CHILD_B,
            "scroll_y": {"initial": initial_offsets, "child": focused_offsets, "footer": footer_offsets},
            "transport_captures": [receipt]}


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
