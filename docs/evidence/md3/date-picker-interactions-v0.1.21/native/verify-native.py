#!/usr/bin/env python3
"""Verify the date interaction update on the native headless Gallery."""

import argparse
import json
import math
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, launch, same_box
from window_check import onscreen_windows

SINGLE = "date-picker-demo"
MODAL = SINGLE + "-modal"
DOCKED = "docked-date-picker-demo"
PICKER = DOCKED + "-picker"
TRIGGER = PICKER + "-calendar-trigger"
EDITOR = PICKER + "-input"
SAVED = "2024/02/28"
SHOT = {"scale": 1, "max_side": 1200}


def find(frame, target):
    return next(
        ((i, node) for i, node in enumerate(frame["dom"]["nodes"]) if node.get("stable_id") == target),
        (None, None),
    )


def node(frame, target):
    _, found = find(frame, target)
    require(found is not None, f"Missing DOM node {target}")
    return found


def layout(frame, target):
    index, _ = find(frame, target)
    require(index is not None, f"Missing layout node {target}")
    return frame["layout"]["nodes"][index]


def settle(app):
    previous = app.dom()
    for _ in range(24):
        current = app.dom()
        if previous == current:
            return current
        previous = current
    raise RuntimeError("The Gallery layout did not settle")


def activate(app, target):
    app.activate(target)
    return settle(app)


def key(app, name, *, reverse=False):
    app.wire("Action.key", {"key": name, "extend": reverse})
    return settle(app)


def focus(frame, target):
    require(focused(frame) == target, f"Expected focus {target}. Got {focused(frame)}")


def full_target(frame, target):
    measured = layout(frame, target)
    bounds = measured["bounds"]
    require(bounds["width"] >= 47.5 and bounds["height"] >= 47.5, f"Small target {target}: {bounds}")
    require(same_box(measured["clip"], bounds), f"Clipped target {target}: {measured}")
    return bounds


def require_saved(frame, fixture):
    text = node(frame, fixture + "-status").get("text")
    require(text == f"已選日期 {SAVED}", f"Unexpected saved date: {text}")


def scene(app, fixture):
    app.activate("section-2")
    frame = activate(app, fixture + "-show")
    require_saved(frame, fixture)
    return frame


def check_size(box, size):
    require(all(abs(box[key] - value) <= 0.5 for key, value in zip(("width", "height"), size)),
            f"Expected panel size {size}. Got {box}")


def capture(app, plan, frame, title, panel):
    path = plan.out / f"{title}.png"
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    api = app.wire("Page.screenshot", SHOT)
    mcp, mcp_metadata, _ = app.mcp(SHOT)
    encoded = path.read_bytes()
    require(encoded == png_payload(api) == mcp, f"PNG bytes differ for {title}")
    require(metadata(cli) == metadata(api) == mcp_metadata, f"PNG metadata differs for {title}")
    width, height, rows = read_png(encoded)
    require((width, height) == (cli["width"], cli["height"]), "Decoded PNG dimensions disagree")

    color = bytes(node(frame, panel)["style"]["background"])
    bounds = layout(frame, panel)["bounds"]
    left = max(0, math.ceil(bounds["x"]))
    right = min(width, math.floor(bounds["x"] + bounds["width"]))
    top = max(0, math.ceil(bounds["y"]))
    bottom = min(height, math.floor(bounds["y"] + bounds["height"]))
    matching = sum(rows[y][4*x:4*x+4] == color for y in range(top, bottom) for x in range(left, right))
    require(color[3] == 255 and matching >= 512, f"Expected painted panel of color {list(color)} in {title}")
    return {"name": path.name, "sha256": digest(path), **metadata(cli),
            "panel_rgba": list(color), "panel_pixels": matching}


def single_day(date):
    return MODAL + "-day-" + date


def close_single(app):
    frame = key(app, "escape")
    focus(frame, SINGLE + "-open")
    require(find(frame, MODAL)[1] is None, "Escape did not close the calendar")
    require_saved(frame, SINGLE)
    return focused(frame)


def single_desktop(app, plan):
    scene(app, SINGLE)
    start = activate(app, SINGLE + "-open")
    focus(start, single_day("2024-02-28"))
    require(node(start, single_day("2024-02-28"))["selected"] is True, "The saved date lacks selection")
    panel = layout(start, MODAL)["bounds"]
    check_size(panel, (360, 524))
    frame = key(app, "page_down")
    target = single_day("2024-03-28")
    focus(frame, target)
    rect = full_target(frame, target)
    grid = node(frame, MODAL + "-grid")
    require(grid.get("grid_role") == "Grid" and grid["name"] == "2024年3月", "PageDown did not show March")
    require(node(frame, target).get("selected") is False, "PageDown selected a new date")
    require_saved(frame, SINGLE)
    shot = capture(app, plan, frame, "single-light-march", MODAL)
    restored = close_single(app)
    return {"panel": panel, "month": grid["name"], "focused": target, "focus_bounds": rect,
            "restored_focus": restored, "saved": SAVED, "image": shot}


def single_compact(app, plan):
    scene(app, SINGLE)
    for setting in ("theme", "rtl", "large-text"):
        app.activate(f"{SINGLE}-{setting}")
    frame = activate(app, SINGLE + "-open")
    focus(frame, single_day("2024-02-28"))
    initial = full_target(frame, single_day("2024-02-28"))
    panel = layout(frame, MODAL)["bounds"]
    check_size(panel, (320, 520))
    require(same_box(panel, frame["layout"]["viewport"]), "Compact calendar needs a full viewport")
    frame = key(app, "left")
    target = single_day("2024-02-29")
    focus(frame, target)
    date = full_target(frame, target)
    require(date["x"] < initial["x"], "RTL ArrowLeft did not move to the next day")
    require(node(frame, target).get("current_date") is True, "Today lacks its semantic marker")
    require(node(frame, single_day("2024-02-28"))["selected"] is True, "The saved selection changed")
    confirm = full_target(frame, MODAL + "-confirm")
    require_saved(frame, SINGLE)
    shot = capture(app, plan, frame, "single-dark-rtl-200", MODAL)
    restored = close_single(app)
    return {"panel": panel, "rtl": True, "text_scale": 2, "focused": target,
            "focus_bounds": date, "confirm_bounds": confirm, "restored_focus": restored,
            "saved": SAVED, "image": shot}


def tab_to(app, target):
    steps = []
    for _ in range(10):
        frame = key(app, "tab")
        steps.append(focused(frame))
        if focused(frame) == target:
            return frame, steps
    raise RuntimeError(f"Tab missed {target}: {steps}")


def docked_desktop(app, plan):
    scene(app, DOCKED)
    frame = activate(app, TRIGGER)
    focus(frame, PICKER + "-day-2024-02-28")
    require(node(frame, TRIGGER).get("expanded") is True, "The picker did not open")
    popup = PICKER + "-popup"
    dimensions = layout(frame, popup)["bounds"]
    check_size(dimensions, (360, 456))
    images = []
    lists = []
    for kind, current in (("month", "02"), ("year", "2024")):
        selector = f"{PICKER}-{kind}"
        frame, steps = tab_to(app, selector)
        selector_bounds = full_target(frame, selector)
        require(find(frame, f"{PICKER}-{kind}-list")[1] is None, f"Tab opened the {kind} list")
        frame = key(app, "enter")
        row = f"{PICKER}-{kind}-{current}"
        focus(frame, row)
        row_bounds = full_target(frame, row)
        require(node(frame, row).get("selected") is True, f"Expected the current {kind} option")
        node(frame, f"{PICKER}-{kind}-list")
        require_saved(frame, DOCKED)
        require(node(frame, EDITOR).get("text") == SAVED, "Opening a list changed the editor text")
        images.append(capture(app, plan, frame, f"docked-{kind}-list", popup))
        frame = key(app, "escape")
        focus(frame, selector)
        require(find(frame, f"{PICKER}-{kind}-list")[1] is None, f"Escape left the {kind} list open")
        require(node(frame, TRIGGER).get("expanded") is True, "Escape closed the calendar and list together")
        lists.append({"list": kind, "tab_path": steps, "selector_bounds": selector_bounds,
                      "focused_option": row, "option_bounds": row_bounds, "restored_focus": focused(frame)})

    frame = key(app, "escape")
    focus(frame, TRIGGER)
    require(node(frame, TRIGGER).get("expanded") is False, "Escape left the disclosure open")
    require(find(frame, popup)[1] is None, "The closed picker still has a popup")
    require_saved(frame, DOCKED)
    return {"popup": dimensions, "lists": lists, "saved": SAVED, "restored_focus": focused(frame), "images": images}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--gallery", type=Path, required=True)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--out", type=Path, required=True)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True)
    report = {
        "source_manifest_sha256": plan.source,
        "gallery_sha256": digest(plan.gallery),
        "cli_sha256": digest(plan.cli),
        "headless_renderer": "WGPU native macOS",
        "input_source": "DevTools semantic named keys only",
        "physical_swipe_or_shift_letters": "not exercised by native DevTools",
    }
    profiles = (
        ("single_desktop", (960, 800), single_desktop),
        ("single_compact", (320, 520), single_compact),
        ("docked_desktop", (960, 800), docked_desktop),
    )
    for name, size, check in profiles:
        with launch(plan, name, *size) as app:
            before = onscreen_windows(app.proc.pid)
            result = check(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(before == 0 and after == 0, f"Unexpected visible Gallery window in {name}")
            result["onscreen_windows"] = {"before": before, "after": after}
            report[name] = result
            print(f"PASS {name} date interactions and PNG transport", flush=True)
    light = report["single_desktop"]["image"]["panel_rgba"]
    dark = report["single_compact"]["image"]["panel_rgba"]
    require(light != dark, "The panel color stayed the same after changing the theme")
    (plan.out / "native-date-interaction-report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
