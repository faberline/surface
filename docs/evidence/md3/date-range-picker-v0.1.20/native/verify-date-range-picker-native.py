#!/usr/bin/env python3
"""Replay the modal date range picker through the native headless Gallery."""

import argparse
import json
import math
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, launch, same_box
from window_check import onscreen_windows

BASE = "date-range-picker-demo"
PICKER = BASE + "-modal"
SHOT = {"scale": 1, "max_side": 1200}
INITIAL = ("2024/02/24", "2024/03/05")


def control(suffix):
    return f"{BASE}-{suffix}"


def part(suffix):
    return f"{PICKER}-{suffix}"


def day(date):
    return part("day-" + date)


def locate(frame, stable_id):
    return next(
        ((index, item) for index, item in enumerate(frame["dom"]["nodes"])
         if item.get("stable_id") == stable_id),
        (None, None),
    )


def node(frame, stable_id):
    _, result = locate(frame, stable_id)
    require(result is not None, f"Missing node {stable_id}")
    return result


def computed(frame, stable_id):
    index, _ = locate(frame, stable_id)
    require(index is not None, f"Missing bounds for {stable_id}")
    return frame["layout"]["nodes"][index]


def bounds(frame, stable_id):
    return computed(frame, stable_id)["bounds"]


def settle(app):
    old = app.dom()
    for _ in range(24):
        current = app.dom()
        if old == current:
            return current
        old = current
    raise RuntimeError("The Gallery did not reach a stable frame")


def key(app, name, *, extend=False):
    app.wire("Action.key", {"key": name, "extend": extend})
    return settle(app)


def activate(app, stable_id):
    app.activate(stable_id)
    return settle(app)


def fill(app, field, text):
    stable_id = part(f"{field}-input")
    app.wire("Action.fill", {"id": stable_id, "value": text})
    frame = settle(app)
    require(focused(frame) == stable_id, f"Filling {field} did not focus it")
    require(node(frame, stable_id)["text"] == text, f"The editor changed {text!r} while typing")
    return frame


def require_focus(frame, target):
    require(focused(frame) == target, f"Focus was {focused(frame)} instead of {target}")


def require_saved(frame, start, end):
    actual = node(frame, control("status"))["text"]
    require(actual == f"已儲存 {start} 至 {end}", f"Saved range was {actual}")


def require_closed(frame):
    for suffix in ("body", "summary", "start-input", "end-input", "month-2024-02-grid"):
        require(locate(frame, part(suffix))[1] is None, f"Closed picker kept {suffix}")
    require(locate(frame, PICKER)[1] is None, "The range dialog is still mounted")


def require_revealed(frame, date):
    stable_id = day(date)
    require_focus(frame, stable_id)
    layout = computed(frame, stable_id)
    target = layout["bounds"]
    require(target["width"] >= 47.5 and target["height"] >= 47.5, f"Small day target {target}")
    require(same_box(layout["clip"], target), f"Focused date {date} was clipped: {layout}")
    return target


def visible_target(frame, suffix):
    layout = computed(frame, part(suffix))
    target = layout["bounds"]
    require(same_box(layout["clip"], target), f"{suffix} is clipped: {layout}")
    require(target["width"] >= 47.5 and target["height"] >= 47.5, f"{suffix} is too small: {target}")
    return target


def action_targets(frame, mode_switch):
    return {suffix: visible_target(frame, suffix) for suffix in ("cancel", "confirm", mode_switch)}


def grids(frame):
    prefix = part("month-")
    mounted = [item for item in frame["dom"]["nodes"]
               if (item.get("stable_id") or "").startswith(prefix)
               and item["stable_id"].endswith("-grid")]
    require(1 <= len(mounted) <= 5, f"Mounted {len(mounted)} calendar grids")
    require(all(item.get("grid_role") == "Grid" and item.get("grid_multiselectable") is True
                for item in mounted), "A month is missing its multi-selection grid semantics")
    return [item["stable_id"][len(prefix):-len("-grid")] for item in mounted]


def scene(app):
    app.activate("section-2")
    frame = activate(app, control("show"))
    require_closed(frame)
    require_saved(frame, *INITIAL)


def capture(app, plan, name):
    file = plan.out / f"{name}.png"
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", file])
    api = app.wire("Page.screenshot", SHOT)
    mcp, mcp_metadata, _ = app.mcp(SHOT)
    encoded = file.read_bytes()
    require(encoded == png_payload(api) == mcp, f"PNG bytes differ for {name}")
    require(metadata(cli) == metadata(api) == mcp_metadata, f"PNG metadata differs for {name}")
    width, height, rows = read_png(encoded)
    require((width, height) == (cli["width"], cli["height"]), "PNG dimensions disagree")
    return {"name": file.name, "sha256": digest(file), **metadata(cli)}, rows


def pixels_in_band(rows, layout, color):
    box, clip = layout["bounds"], layout["clip"]
    start_x = max(0, math.ceil(max(box["x"], clip["x"])))
    start_y = max(0, math.ceil(max(box["y"], clip["y"])))
    stop_x = min(len(rows[0]) // 4, math.floor(min(box["x"] + box["width"], clip["x"] + clip["width"])))
    stop_y = min(len(rows), math.floor(min(box["y"] + box["height"], clip["y"] + clip["height"])))
    color = bytes(color)
    return sum(rows[y][x*4:x*4+4] == color
               for y in range(start_y, stop_y) for x in range(start_x, stop_x))


def band_receipt(frame, rows, month):
    prefix = part(f"month-{month}-week-")
    band_ids = [item["stable_id"] for item in frame["dom"]["nodes"]
                if (item.get("stable_id") or "").startswith(prefix)
                and item["stable_id"].endswith("-band")]
    require(band_ids, f"Missing {month} range bands")
    matches = []
    for stable_id in band_ids:
        layout = computed(frame, stable_id)
        rect = layout["bounds"]
        require(abs(rect["height"] - 40.) <= 0.5 and rect["width"] > 0., f"Invalid range band {rect}")
        color = node(frame, stable_id)["style"]["background"]
        require(color[3] == 255, f"Expected an opaque range band color: {color}")
        samples = pixels_in_band(rows, layout, color)
        if samples:
            matches.append({"id": stable_id, "bounds": rect, "rgba": color, "pixels": samples})
    require(matches, f"No {month} band paint in the rendered PNG")
    return matches


def heading_ink(frame, rows, month):
    stable_id = part(f"month-{month}-heading")
    index, title = locate(frame, stable_id)
    require(title and title["role"] == "Heading" and title["name"] == "2024年2月",
            "Wrong visible month heading")
    caption = title if title.get("text") else next(
        (item for item in frame["dom"]["nodes"]
         if item.get("parent") == index and item.get("text") == title["name"]), None,
    )
    require(caption, "The month heading has no caption to paint")
    color = bytes(caption["style"]["foreground"])
    rect = bounds(frame, stable_id)
    width = len(rows[0]) // 4
    top = max(0, math.ceil(rect["y"]))
    bottom = min(len(rows), math.floor(rect["y"] + rect["height"]))
    ink = [x for y in range(top, bottom) for x in range(width) if rows[y][4*x:4*x+4] == color]
    require(ink, "The visible month title has no solid ink")
    left = min(ink)
    right = width - 1 - max(ink)
    require(min(left, right) >= 2, f"Month title paint reaches a viewport edge: {left}, {right}")
    return {"text": title["name"], "left_clearance": left, "right_clearance": right, "pixels": len(ink)}


def summary(frame):
    status = node(frame, part("summary"))
    require(status["role"] == "Status", "The selection summary is not a status")
    return status.get("text") or status["name"]


def desktop_calendar(app, plan):
    scene(app)
    frame = activate(app, control("open"))
    require_revealed(frame, "2024-02-24")
    panel = bounds(frame, PICKER)
    viewport = frame["layout"]["viewport"]
    require(same_box(panel, viewport) and same_box(viewport, panel), "Calendar did not fill the viewport")
    actions = bounds(frame, part("actions"))
    require("2024年2月24日" in summary(frame) and "2024年3月5日" in summary(frame),
            "Summary does not name both range endpoints")
    app.wire("Action.activate", {"id": control("outside")}, expected_error="outside_modal")

    frame = key(app, "page_down")
    require_revealed(frame, "2024-03-24")
    mounted = grids(frame)
    require("2024-02" in mounted and "2024-03" in mounted, f"Range months disappeared: {mounted}")
    start = node(frame, day("2024-02-24"))
    end = node(frame, day("2024-03-05"))
    interior = node(frame, day("2024-02-29"))
    unavailable = node(frame, day("2024-03-01"))
    require(start.get("selected") is True and start["name"].startswith("開始日期"), "Start state is missing")
    require(end.get("selected") is True and end["name"].startswith("結束日期"), "End state is missing")
    require(interior.get("current_date") is True and interior.get("selected") is False
            and interior["name"].startswith("範圍內"), "Today and the range interior need distinct states")
    require(unavailable["disabled"] and unavailable.get("selected") is None,
            "An unavailable in-range date must remain unavailable as an endpoint")
    photo, rows = capture(app, plan, "calendar-light-crossing")
    band_paint = [*band_receipt(frame, rows, "2024-02"), *band_receipt(frame, rows, "2024-03")]

    frame = activate(app, day("2024-02-29"))
    require_focus(frame, day("2024-02-29"))
    require(node(frame, part("confirm"))["disabled"], "A partial range can be saved")
    require("尚未選擇" in summary(frame), "A partial range does not say the end is pending")
    frame = key(app, "right")
    require_revealed(frame, "2024-03-02")
    frame = key(app, "enter")
    require(not node(frame, part("confirm"))["disabled"], "A complete draft cannot be saved")
    draft_summary = summary(frame)
    require("2024年2月29日" in draft_summary and "2024年3月2日" in draft_summary,
            "The keyboard draft did not select the across-month endpoints")
    pages = []
    for date in ("2024-04-02", "2024-05-02", "2024-06-02"):
        frame = key(app, "page_down")
        target = require_revealed(frame, date)
        require(summary(frame) == draft_summary, "Browsing changed the range draft")
        require(bounds(frame, part("actions")) == actions, "Browsing moved the calendar actions")
        feed = node(frame, part("body"))["style"]
        pages.append({"focused": date, "mounted_months": grids(frame), "target": target,
                      "scroll_y": feed["scroll_y"]})
    frame = key(app, "escape")
    require_closed(frame)
    require_focus(frame, control("open"))
    require_saved(frame, *INITIAL)
    app.wire("Action.activate", {"id": day("2024-06-02")}, expected_error="missing_node")
    return {"initial_months": mounted, "pages": pages, "fixed_actions": actions, "initial_band_paint": band_paint,
            "draft_summary": draft_summary, "image": photo}


def desktop_input(app, plan):
    frame = activate(app, control("input-open"))
    require_focus(frame, part("start-input"))
    panel = bounds(frame, PICKER)
    require(abs(panel["width"] - 328.) <= 0.75 and abs(panel["height"] - 512.) <= 0.75,
            f"Wrong desktop input reference bounds: {panel}")
    for field in ("start", "end"):
        expected = part(f"{field}-hint")
        require(node(frame, part(f"{field}-input")).get("described_by_text") == expected,
                f"{field} is missing its independent format help")
        require("YYYY/MM/DD" in node(frame, expected)["text"], f"Missing {field} date format")

    fill(app, "start", "2024/3/7")
    frame = key(app, "tab")
    require_focus(frame, part("end-input"))
    require(node(frame, part("start-input"))["text"] == "2024/03/07", "Blur did not format Start")
    fill(app, "end", "2024/3/6")
    frame = key(app, "enter")
    pair_error = node(frame, part("end-error"))["text"]
    require("結束日期不可早於開始日期" in pair_error, "Reversed range has no associated error")
    require(node(frame, part("end-input")).get("described_by_text") == part("end-error"),
            "End does not describe the reversed-range error")
    require(node(frame, part("confirm"))["disabled"], "Reversed range can be saved")
    require_saved(frame, *INITIAL)
    invalid, _ = capture(app, plan, "input-light-reversed")

    fill(app, "start", "2100/2/29")
    frame = key(app, "enter")
    field_error = node(frame, part("start-error"))["text"]
    require("這個日期不存在" in field_error, "Invalid leap day error did not belong to Start")
    require(node(frame, part("start-input")).get("described_by_text") == part("start-error"),
            "Start does not describe its invalid-date error")
    require(node(frame, part("confirm"))["disabled"], "Invalid Start can be saved")

    fill(app, "start", "２０２４／２／２８")
    frame = key(app, "enter")
    require_focus(frame, part("start-input"))
    require(node(frame, part("start-input"))["text"] == "2024/02/28", "Start did not normalize")
    fill(app, "end", "２０２４／３／５")
    frame = key(app, "enter")
    require_focus(frame, part("end-input"))
    require(node(frame, part("end-input"))["text"] == "2024/03/05", "End did not normalize")
    require(not node(frame, part("confirm"))["disabled"], "A valid typed range cannot be saved")
    require_saved(frame, *INITIAL)
    valid, _ = capture(app, plan, "input-light-valid")
    frame = activate(app, part("confirm"))
    require_closed(frame)
    require_focus(frame, control("input-open"))
    require_saved(frame, "2024/02/28", "2024/03/05")
    return {"panel": panel, "pair_error": pair_error, "field_error": field_error,
            "saved": ["2024/02/28", "2024/03/05"], "images": [invalid, valid]}


def compact(app, plan):
    scene(app)
    for control_name in ("theme", "rtl", "large-text"):
        app.activate(control(control_name))
    frame = activate(app, control("open"))
    initial = require_revealed(frame, "2024-02-24")
    initial_x = node(frame, part("body"))["style"]["scroll_x"]
    panel = bounds(frame, PICKER)
    viewport = frame["layout"]["viewport"]
    require(same_box(panel, viewport) and same_box(viewport, panel), "Compact calendar is not full screen")
    frame = key(app, "left")
    target = require_revealed(frame, "2024-02-25")
    revealed_x = node(frame, part("body"))["style"]["scroll_x"]
    require(target["x"] > initial["x"], "RTL navigation did not move to the rightmost weekday")
    require(abs((revealed_x - initial_x) - 16.) <= 0.5,
            f"The 336 dp calendar did not pan across 320 dp of viewport: {initial_x}, {revealed_x}")
    action_targets(frame, "mode")
    photo, rows = capture(app, plan, "calendar-dark-rtl-200")
    bands = band_receipt(frame, rows, "2024-02")
    heading = heading_ink(frame, rows, "2024-02")
    frame = key(app, "page_down")
    next_target = require_revealed(frame, "2024-03-25")
    mounted = grids(frame)
    frame = key(app, "escape")
    require_focus(frame, control("open"))
    require_closed(frame)
    require_saved(frame, *INITIAL)
    return {"panel": panel, "first": initial, "mirrored": target, "later": next_target,
            "horizontal_pan": [initial_x, revealed_x], "mounted_months": mounted,
            "band_paint": bands, "heading_ink": heading, "image": photo}


def short_profile(app, plan):
    scene(app)
    for suffix in ("theme", "rtl", "large-text"):
        app.activate(control(suffix))
    frame = activate(app, control("open"))
    first = require_revealed(frame, "2024-02-24")
    calendar_actions = action_targets(frame, "mode")
    calendar_body = bounds(frame, part("body"))
    require(calendar_body["height"] >= 48., f"Short calendar body is too small: {calendar_body}")

    frame = key(app, "tab", extend=True)
    require_focus(frame, part("next-month"))
    next_button = visible_target(frame, "next-month")
    status = computed(frame, part("summary"))
    status_box = status["bounds"]
    require(status_box["width"] > 0. and status_box["height"] > 0.
            and same_box(status["clip"], status_box), "The short calendar status is clipped")
    range_summary = summary(frame)
    browsed = []
    for gesture, month in (("enter", "2024-03"), ("space", "2024-04")):
        frame = key(app, gesture)
        require_focus(frame, part("next-month"))
        require(visible_target(frame, "next-month") == next_button, "Browsing moved Next Month")
        require(action_targets(frame, "mode") == calendar_actions, "Browsing moved the actions")
        require(bounds(frame, part("summary")) == status_box and summary(frame) == range_summary,
                "Browsing moved or changed the short calendar status")
        month_box = bounds(frame, part(f"month-{month}"))
        require(abs(month_box["y"] - calendar_body["y"]) <= 0.5,
                f"Browsing did not align {month} to the top of the calendar body: {month_box}")
        browsed.append({"key": gesture, "month": month, "month_top": month_box["y"]})
    frame = key(app, "escape")
    require_closed(frame)
    require_focus(frame, control("open"))
    require_saved(frame, *INITIAL)

    frame = activate(app, control("open"))
    require(require_revealed(frame, "2024-02-24") == first, "Reopening changed the initial date reveal")
    frame = key(app, "left")
    second = require_revealed(frame, "2024-02-25")
    require(second["x"] > first["x"], "RTL navigation did not reach the next date")
    require(action_targets(frame, "mode") == calendar_actions, "Date navigation moved the actions")
    calendar_image, _ = capture(app, plan, "calendar-dark-rtl-200-short")
    frame = key(app, "escape")
    require_closed(frame)
    require_focus(frame, control("open"))

    frame = activate(app, control("input-open"))
    require_focus(frame, part("start-input"))
    start = visible_target(frame, "start-input")
    start_scroll = node(frame, part("body"))["style"]["scroll_y"]
    input_actions = action_targets(frame, "calendar-mode")
    input_body = bounds(frame, part("body"))
    frame = key(app, "tab")
    require_focus(frame, part("end-input"))
    end = visible_target(frame, "end-input")
    end_scroll = node(frame, part("body"))["style"]["scroll_y"]
    require(action_targets(frame, "calendar-mode") == input_actions, "Focusing End moved the actions")
    require(end_scroll > start_scroll, "Tab did not scroll the short input body to End")
    input_image, _ = capture(app, plan, "input-dark-rtl-200-short")
    frame = key(app, "tab", extend=True)
    require_focus(frame, part("start-input"))
    visible_target(frame, "start-input")
    frame = key(app, "escape")
    require_closed(frame)
    require_focus(frame, control("input-open"))
    require_saved(frame, *INITIAL)
    return {
        "viewport": [320, 240], "rtl": True, "text_scale": 2,
        "calendar": {"body": calendar_body, "actions": calendar_actions, "first": first,
                     "second": second, "next_button": next_button, "keyboard_browse": browsed,
                     "status": {"bounds": status_box, "range": range_summary}, "image": calendar_image},
        "input": {"body": input_body, "actions": input_actions, "start": start, "end": end,
                  "scroll": [start_scroll, end_scroll], "image": input_image},
    }


def desktop(app, plan):
    return {"calendar": desktop_calendar(app, plan), "input": desktop_input(app, plan)}


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
        "headless_renderer": "WGPU native macOS",
        "calendar_navigation": "DevTools keyboard actions and viewport focus reveal",
    }
    profiles = (
        ("desktop", (960, 840), desktop),
        ("compact", (320, 720), compact),
        ("short", (320, 240), short_profile),
    )
    for name, size, check in profiles:
        with launch(plan, name, *size) as app:
            before = onscreen_windows(app.proc.pid)
            result = check(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(before == 0 and after == 0, f"Unexpected onscreen Gallery window in {name}")
            result["onscreen_windows"] = {"before": before, "after": after}
            report[name] = result
            print(f"PASS {name} Date Range Picker and PNG transport", flush=True)
    light = report["desktop"]["calendar"]["initial_band_paint"][0]["rgba"]
    dark = report["compact"]["band_paint"][0]["rgba"]
    require(light != dark, "The band palette did not change in dark theme")
    (plan.out / "native-date-range-picker-report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
