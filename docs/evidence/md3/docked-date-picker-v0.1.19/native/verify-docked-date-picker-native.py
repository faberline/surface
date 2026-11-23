#!/usr/bin/env python3
"""Replay the docked date field and calendar in the headless Metal Gallery."""

import argparse
import json
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, launch, same_box
from window_check import onscreen_windows

BASE = "docked-date-picker-demo"
PICKER = BASE + "-picker"
INPUT = PICKER + "-input"
TRIGGER = PICKER + "-calendar-trigger"
SHOT = {"scale": 1, "max_side": 1200}


def part(suffix):
    return f"{PICKER}-{suffix}"


def control(suffix):
    return f"{BASE}-{suffix}"


def day(date):
    return part("day-" + date)


def locate(frame, stable_id):
    return next(((index, candidate) for index, candidate in enumerate(frame["dom"]["nodes"])
                 if candidate.get("stable_id") == stable_id), (None, None))


def node(frame, stable_id):
    _, result = locate(frame, stable_id)
    require(result is not None, f"Missing node {stable_id}")
    return result


def bounds(frame, stable_id):
    index, _ = locate(frame, stable_id)
    require(index is not None, f"Missing bounds for {stable_id}")
    return frame["layout"]["nodes"][index]["bounds"]


def settle(app):
    first = app.dom()
    for _ in range(24):
        last = app.dom()
        if first == last:
            return last
        first = last
    raise RuntimeError("The Gallery did not reach a stable frame")


def require_focus(frame, target):
    require(focused(frame) == target, f"Focus was {focused(frame)} instead of {target}")


def require_closed(frame):
    require(locate(frame, part("popup"))[1] is None, "The docked calendar is still open")
    require(node(frame, TRIGGER).get("expanded") is False, "Calendar disclosure is not closed")


def require_saved(frame, value):
    text = node(frame, control("status"))["text"]
    require(text == f"已選日期 {value}", f"Displayed saved date disagrees: {text}")


def key(app, code, extend=False):
    app.wire("Action.key", {"key": code, "extend": extend})
    return settle(app)


def fill(app, text):
    app.wire("Action.fill", {"id": INPUT, "value": text})
    frame = settle(app)
    require_focus(frame, INPUT)
    require(node(frame, INPUT)["text"] == text, f"Typing normalized {text!r} too early")
    return frame


def capture(app, plan, name):
    file = plan.out / f"{name}.png"
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", file])
    api = app.wire("Page.screenshot", SHOT)
    mcp, mcp_meta, _ = app.mcp(SHOT)
    encoded = file.read_bytes()
    require(encoded == png_payload(api) == mcp, f"PNG transport mismatch for {name}")
    require(metadata(cli) == metadata(api) == mcp_meta, f"PNG metadata mismatch for {name}")
    width, height, _ = read_png(encoded)
    require((width, height) == (cli["width"], cli["height"]), "PNG dimensions disagree")
    return {"name": file.name, "sha256": digest(file), **metadata(cli)}


def scene(app):
    app.activate("section-2")
    app.activate(control("show"))
    frame = settle(app)
    require_closed(frame)
    require_saved(frame, "2024/02/28")


def calendar(app):
    app.activate(TRIGGER)
    frame = settle(app)
    require(node(frame, TRIGGER).get("expanded") is True, "Calendar disclosure is not open")
    require(node(frame, INPUT)["role"] == "TextInput", "Open scope lost the editor")
    return frame


def placement(frame, *, above, full):
    field = bounds(frame, part("field"))
    panel = bounds(frame, part("popup"))
    gap = field["y"] - panel["y"] - panel["height"] if above else (
        panel["y"] - field["y"] - field["height"])
    require(abs(gap - 4.) <= 0.75, f"Popup should clear the field by 4 dp, got {gap}")
    require(same_box(frame["layout"]["viewport"], panel), f"Popup is outside the viewport: {panel}")
    require(same_box(frame["layout"]["viewport"], field), f"Date field is outside the viewport: {field}")
    if full:
        require(abs(panel["width"] - 360.) <= 0.75 and abs(panel["height"] - 456.) <= 0.75,
                f"Panel does not match the 360 × 456 dp reference: {panel}")
    return {"field": field, "popup": panel, "gap": gap}


def desktop(app, plan):
    scene(app)
    frame = calendar(app)
    require_focus(frame, day("2024-02-28"))
    below = placement(frame, above=False, full=True)
    grid = node(frame, part("grid"))
    require(grid.get("grid_role") == "Grid", "Calendar grid is missing accessibility metadata")
    selected = node(frame, day("2024-02-28"))
    today = node(frame, day("2024-02-29"))
    disabled = node(frame, day("2024-02-27"))
    require(selected["selected"] and today["current_date"] and disabled["disabled"],
            "Selected, today, and unavailable dates must stay distinct")
    photos = [capture(app, plan, "calendar-light-below")]

    app.activate(part("month"))
    frame = settle(app)
    node(frame, part("month-list"))
    photos.append(capture(app, plan, "month-list-light"))
    app.activate(part("month-03"))
    frame = settle(app)
    require("2024年3月" == node(frame, part("grid"))["name"], "Choosing a month changed no dates")
    app.activate(part("year"))
    frame = settle(app)
    node(frame, part("year-list"))
    photos.append(capture(app, plan, "year-list-light"))
    app.activate(part("year-2025"))
    frame = settle(app)
    require("2025年3月" == node(frame, part("grid"))["name"], "Choosing a year changed no dates")
    require_saved(frame, "2024/02/28")
    frame = key(app, "escape")
    require_closed(frame)
    require_focus(frame, TRIGGER)

    frame = fill(app, "２０００／２／２９")
    require_closed(frame)
    frame = key(app, "enter")
    require_saved(frame, "2000/02/29")
    require_closed(frame)
    require_focus(frame, INPUT)
    require(node(frame, INPUT)["text"] == "2000/02/29", "Enter did not normalize the typed date")
    fill(app, "2024-2-28")
    frame = key(app, "tab")
    require_focus(frame, TRIGGER)
    require_saved(frame, "2024/02/28")
    fill(app, "2100/02/29")
    frame = key(app, "enter")
    error = node(frame, part("error"))["text"]
    require("這個日期不存在" in error, "Invalid leap day did not show the expected error")
    require(node(frame, INPUT).get("described_by_text") == part("error"),
            "The input does not describe the date error")
    require_saved(frame, "2024/02/28")
    photos.append(capture(app, plan, "input-error-light"))

    fill(app, "2024/2/29")
    frame = calendar(app)
    require_saved(frame, "2024/02/28")
    frame = key(app, "escape")
    require_closed(frame)
    require_saved(frame, "2024/02/28")

    app.activate(control("theme"))
    app.activate(control("bottom"))
    frame = calendar(app)
    above = placement(frame, above=True, full=True)
    photos.append(capture(app, plan, "calendar-dark-above"))
    app.activate(day("2024-02-29"))
    frame = settle(app)
    require_closed(frame)
    require_focus(frame, TRIGGER)
    require_saved(frame, "2024/02/29")
    require(node(frame, INPUT)["text"] == "2024/02/29", "Calendar commit did not update the same field")
    return {"below": below, "above": above, "grid_name": grid["name"], "error": error,
            "selected": selected["name"], "today": today["name"], "disabled": disabled["name"],
            "images": photos}


def compact(app, plan):
    scene(app)
    for command in ("rtl", "large-text", "theme"):
        app.activate(control(command))
    frame = calendar(app)
    require_focus(frame, day("2024-02-28"))
    position = placement(frame, above=True, full=False)
    summary = bounds(frame, control("summary"))
    require(position["field"]["y"] >= summary["y"] + summary["height"] + 23.,
            "The wrapped Gallery controls overlap the date field")
    frame = key(app, "left")
    require_focus(frame, day("2024-02-29"))
    target = bounds(frame, day("2024-02-29"))
    require(target["width"] >= 47.5 and target["height"] >= 47.5, "Date target shrank below 48 dp")
    require(same_box(bounds(frame, part("body")), target), "Focused date is clipped by the popup body")
    photos = [capture(app, plan, "calendar-dark-rtl-200")]
    frame = key(app, "end")
    require_focus(frame, day("2024-03-02"))
    edge = bounds(frame, day("2024-03-02"))
    require(same_box(bounds(frame, part("grid-scroll")), edge), "The last RTL weekday is clipped")
    frame = key(app, "escape")
    require_closed(frame)
    require_focus(frame, TRIGGER)
    require_saved(frame, "2024/02/28")
    return {"layout": position, "focused_day": target, "edge_day": edge, "images": photos}


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
    }
    for name, size, checks in (("desktop", (960, 800), desktop), ("compact", (320, 600), compact)):
        with launch(plan, name, *size) as app:
            before = onscreen_windows(app.proc.pid)
            data = checks(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(before == 0 and after == 0, f"Unexpected visible Gallery window in {name}")
            data["onscreen_windows"] = {"before": before, "after": after}
            report[name] = data
            print(f"PASS {name}: Docked Date Picker and PNG transport", flush=True)
    (plan.out / "native-docked-date-picker-report.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
