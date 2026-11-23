#!/usr/bin/env python3
"""Replay Material 3 lists through the windowless native Gallery."""

import argparse
import json
from pathlib import Path

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

DEMO = "lists-demo"
BEFORE = f"{DEMO}-before"
AFTER = f"{DEMO}-after"
GROUP = "lists-items"
ROWS = [f"lists-{letter}" for letter in "abcdef"]
CORNERS = ("top_left", "top_right", "bottom_right", "bottom_left")


def open_scene(app, *settings):
    activate(app, "section-2")
    activate(app, f"{DEMO}-show")
    for setting in settings:
        activate(app, f"{DEMO}-{setting}")
    return settle(app)


def corners(frame, identifier):
    value = node(frame, identifier)["style"]["corners"]
    return tuple(value[name] for name in CORNERS)


def gap(frame, first, second):
    top, bottom = (layout(frame, identifier)["bounds"] for identifier in (first, second))
    return bottom["y"] - top["y"] - top["height"]


def selected(frame, identifier):
    for row in ROWS:
        item = node(frame, row)
        require(item["role"] == "Radio" and item.get("checked") is (row == identifier),
                f"Wrong native role or checked state on {row}")
    current = node(frame, GROUP)
    require(current["role"] == "RadioGroup" and current["name"] == "書籤", "Named radio group is missing")


def focus_pixels(plan, frame, shot, identifier):
    bounds = visible(frame, identifier, 48)
    focus_ink = node(frame, identifier)["focus_color"]
    _, _, pixels = read_png((plan.out / shot["name"]).read_bytes())
    x = round(bounds["x"] + bounds["width"] / 2)
    top = round(bounds["y"])
    colors = [list(pixels[y][x*4:x*4+4]) for y in range(top - 1, top + 5)]

    def same(pixel):
        return all(abs(channel - target) <= 3 for channel, target in zip(pixel, focus_ink))

    require([same(pixel) for pixel in colors] == [False, True, True, True, False, False],
            f"Expected three focus pixels from the edge, got {colors} for {focus_ink}")
    return {"rgba": focus_ink, "edge_samples": colors}


def segmented(app, plan):
    frame = open_scene(app)
    selected(frame, "lists-c")
    require(node(frame, "lists-b")["disabled"], "Disabled option is enabled")
    for identifier, height in (("lists-a", 56), ("lists-c", 72), ("lists-d", 88)):
        bounds = layout(frame, identifier)["bounds"]
        require(abs(bounds["height"] - height) <= 0.5, f"Wrong minimum height for {identifier}: {bounds}")
    require(corners(frame, "lists-a") == (16, 16, 4, 4), "Missing segmented leading corners")
    require(corners(frame, "lists-b") == (4, 4, 4, 4), "Missing segmented inside corners")
    require(corners(frame, "lists-f") == (4, 4, 16, 16), "Missing segmented trailing corners")
    require(gap(frame, "lists-a", "lists-b") == 2, "Wrong segmented gap")
    light = capture(app, plan, "segmented-light", frame=frame, sample_targets=("lists-a-headline",))
    focus_by_tab(app, BEFORE)
    focus(key(app, "tab"), "lists-c")
    frame = key(app, "up")
    focus(frame, "lists-a")
    selected(frame, "lists-c")
    outline = capture(app, plan, "segmented-focused", frame=frame)
    outline["focus"] = focus_pixels(plan, frame, outline, "lists-a")
    frame = key(app, "up")
    focus(frame, "lists-f")
    visible(frame, "lists-f", 48)
    selected(frame, "lists-c")
    frame = key(app, "enter")
    selected(frame, "lists-f")
    focus(frame, "lists-f")
    scrolled = capture(app, plan, "segmented-wrap-selected", frame=frame, sample_targets=("lists-f-headline",))
    focus(key(app, "tab"), AFTER)
    return {"viewport": [760, 620], "wrap_from_a_to_f": True, "transport_captures": [light, outline, scrolled]}


def standard(app, plan):
    frame = open_scene(app, "theme", "standard", "action")
    require(node(frame, GROUP)["role"] == "List", "Expected a semantic action list")
    require(node(frame, "lists-a-item")["role"] == "ListItem", "Expected an action inside a list item")
    require(node(frame, "lists-c")["role"] == "Button", "Action row has the wrong role")
    require(node(frame, "lists-c")["described_by_text"] == "lists-c-supporting", "Missing supporting description")
    require(corners(frame, "lists-a") == (4, 4, 4, 4) and gap(frame, "lists-a", "lists-b") == 0,
            "Standard list uses the wrong corners or spacing")
    focus_by_tab(app, BEFORE)
    focus(key(app, "tab"), "lists-a")
    frame = key(app, "down")
    focus(frame, "lists-c")
    frame = key(app, "space")
    require(node(frame, f"{DEMO}-status")["text"] == "最近操作：河", "Space did not run the named action")
    shot = capture(app, plan, "standard-dark-action", frame=frame, sample_targets=("lists-c-headline", "lists-c-supporting"))
    return {"viewport": [760, 620], "space_ran_the_action": True, "transport_captures": [shot]}


def baseline(app, plan):
    frame = open_scene(app, "baseline", "passive")
    require(node(frame, GROUP)["role"] == "List", "Expected a semantic passive list")
    require(node(frame, "lists-a")["role"] == "ListItem", "Passive row is not a list item")
    require(corners(frame, "lists-a") == (0, 0, 0, 0), "Baseline resting row is not square")
    require(layout(frame, "lists-a-leading")["bounds"]["width"] == 24., "Baseline icon is not 24dp")
    focus_by_tab(app, BEFORE)
    focus(key(app, "tab"), AFTER)
    shot = capture(app, plan, "baseline-light-passive", frame=frame, sample_targets=("lists-a-headline", "lists-a-trailing"))
    return {"viewport": [760, 620], "tab_skips_passive_rows": True, "transport_captures": [shot]}


def compact(app, plan):
    frame = open_scene(app, "theme", "rtl", "large-text")
    focus_by_tab(app, BEFORE)
    focus(key(app, "tab"), "lists-c")
    first = key(app, "up")
    focus(first, "lists-a")
    lead = visible(first, "lists-a-leading")
    tail = visible(first, "lists-a-trailing")
    require(tail["x"] + tail["width"] < lead["x"], "RTL leading and trailing slots did not mirror")
    focus(key(app, "down"), "lists-c")
    frame = key(app, "down")
    focus(frame, "lists-d")
    row = visible(frame, "lists-d", 48)
    text = visible(frame, "lists-d-headline")
    support = visible(frame, "lists-d-supporting")
    require(row["height"] > 88. and same_box(row, text) and same_box(row, support), "Magnified text did not fit its row")
    require(text["height"] > 48., "Expected the magnified headline to wrap")
    selected(frame, "lists-c")
    frame = key(app, "enter")
    selected(frame, "lists-d")
    status = visible(frame, f"{DEMO}-status")
    require(status["height"] >= 60. and status["x"] + status["width"] <= 320.,
            f"The long selected name must wrap in the footer: {status}")
    shot = capture(app, plan, "segmented-compact-dark-rtl", frame=frame,
                   sample_targets=("lists-d-headline", "lists-d-supporting", f"{DEMO}-status"))
    return {"viewport": [320, 780], "wrapped_heading": text, "row": row, "rtl_slots": [lead, tail],
            "wrapped_status": status, "transport_captures": [shot]}


def short(app, plan):
    open_scene(app, "theme", "rtl", "large-text")
    focus_by_tab(app, BEFORE)
    frame = key(app, "tab")
    focus(frame, "lists-c")
    bounds = visible(frame, "lists-c", 48)
    selected(frame, "lists-c")
    frame = key(app, "enter")
    require(node(frame, f"{DEMO}-status")["text"] == "已選取：河", "Compact selected row is unusable")
    shot = capture(app, plan, "segmented-short-dark-rtl", frame=frame, sample_targets=("lists-c-headline",))
    return {"viewport": [320, 240], "revealed_selected_row": bounds, "transport_captures": [shot]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--gallery", required=True, type=Path)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--source", required=True)
    parser.add_argument("--out", required=True, type=Path)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True)
    result = {"source_manifest_sha256": plan.source, "gallery_sha256": digest(plan.gallery),
              "transports": ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"],
              "profiles": {}}
    for replay, width, height in ((segmented, 760, 620), (standard, 760, 620),
                                  (baseline, 760, 620), (compact, 320, 780), (short, 320, 240)):
        name = replay.__name__
        with launch(plan, name, width, height) as app:
            before = onscreen_windows(app.proc.pid)
            proof = replay(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(before == after == 0, f"{name} opened a visible Gallery window")
            proof["visible_windows"] = [before, after]
        require(app.proc.poll() is not None, f"{name} left the Gallery running")
        result["profiles"][name] = proof
        print(f"PASS {name} {width}x{height}. No visible windows.", flush=True)
    (plan.out / "native-list-report.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
