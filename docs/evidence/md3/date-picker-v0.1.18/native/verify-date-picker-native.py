#!/usr/bin/env python3
"""Replay the single-date picker on the Surface headless Metal host."""

import argparse
import json
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, launch, same_box
from window_check import onscreen_windows

BASE = "date-picker-demo"
PICKER = BASE + "-modal"
INPUT = PICKER + "-input"
SHOT = {"scale": 1, "max_side": 1200}


def locate(frame, stable_id):
    for index, node in enumerate(frame["dom"]["nodes"]):
        if node.get("stable_id") == stable_id:
            return index, node
    raise RuntimeError(f"Missing node {stable_id}")


def node(frame, stable_id):
    return locate(frame, stable_id)[1]


def bounds(frame, stable_id):
    return frame["layout"]["nodes"][locate(frame, stable_id)[0]]["bounds"]


def part(name):
    return PICKER + "-" + name


def day(date):
    return part("day-" + date)


def settled(app):
    first = app.dom()
    for _ in range(18):
        last = app.dom()
        if first == last:
            return last
        first = last
    raise RuntimeError("The Gallery view did not settle")


def focus(app, target):
    frame = settled(app)
    require(focused(frame) == target, f"Expected {target}; found {focused(frame)}")
    return frame


def scene(app):
    app.activate("section-2")
    app.activate(BASE + "-show")


def open_picker(app, input_mode=False):
    app.activate(BASE + ("-input-open" if input_mode else "-open"))


def assert_size(rect, width, height):
    require(abs(rect["width"] - width) <= 0.7 and abs(rect["height"] - height) <= 0.7,
            f"Expected {width}x{height}; got {rect}")


def capture(app, plan, name, fill_sample=None):
    file = plan.out / (name + ".png")
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", file])
    api = app.wire("Page.screenshot", SHOT)
    mcp, mcp_meta, _ = app.mcp(SHOT)
    encoded = file.read_bytes()
    require(encoded == png_payload(api) == mcp, f"Different PNG bytes for {name}")
    require(metadata(cli) == metadata(api) == mcp_meta, f"Different PNG metadata for {name}")
    width, height, rows = read_png(encoded)
    require((width, height) == (cli["width"], cli["height"]), "PNG dimensions disagree")
    result = {"name": file.name, "sha256": digest(file), **metadata(cli)}
    if fill_sample is not None:
        frame, stable_id = fill_sample
        rect = bounds(frame, stable_id)
        x = int(rect["x"] + 5)
        y = int(rect["y"] + rect["height"] / 2)
        rgba = tuple(rows[y][4 * x:4 * x + 3])
        expected = node(frame, stable_id)["style"]["background"][:3]
        require(max(abs(actual - wanted) for actual, wanted in zip(rgba, expected)) <= 8,
                f"Selected day fill was {rgba} instead of {expected}")
        result["selected_fill"] = rgba
    return result


def key(app, name, extend=False):
    return app.wire("Action.key", {"key": name, "extend": extend})


def fill(app, value):
    return app.wire("Action.fill", {"id": INPUT, "value": value})


def check_date_nodes(frame):
    grid = node(frame, part("grid"))
    header = node(frame, part("weekday-0"))
    selected = node(frame, day("2024-02-28"))
    today = node(frame, day("2024-02-29"))
    unavailable = node(frame, day("2024-02-27"))
    require(grid["grid_role"] == "Grid" and header["grid_role"] == "ColumnHeader",
            "Calendar grid and weekday roles are absent")
    require(selected["grid_role"] == "GridCell" and selected["selected"] is True,
            "The applied date is not the selected cell")
    require(today["current_date"] is True and today["selected"] is False,
            "Today and selection are not independent")
    require(unavailable["disabled"], "Unavailable date accepts action")
    return {"grid": grid["name"], "weekday": header["name"],
            "selected": selected["name"], "today": today["name"], "disabled": unavailable["name"]}


def desktop(app, plan):
    scene(app)
    open_picker(app)
    frame = focus(app, day("2024-02-28"))
    rect = bounds(frame, PICKER)
    assert_size(rect, 360., 524.)
    semantic = check_date_nodes(frame)
    key(app, "right")
    frame = focus(app, day("2024-02-29"))
    images = [capture(app, plan, "calendar-light", (frame, day("2024-02-28") + "-face"))]
    key(app, "enter")
    frame = focus(app, day("2024-02-29"))
    require(node(frame, day("2024-02-29"))["selected"], "Enter did not select the draft")
    key(app, "right")
    focus(app, day("2024-03-02"))
    key(app, "page_down", extend=True)
    focus(app, day("2025-03-02"))
    key(app, "escape")
    frame = focus(app, BASE + "-open")
    require("2024/02/28" in node(frame, BASE + "-status")["text"], "Escape committed the draft")
    app.activate(BASE + "-theme")
    open_picker(app)
    focus(app, day("2024-02-28"))
    images.append(capture(app, plan, "calendar-dark"))
    app.activate(part("month"))
    years = bounds(settled(app), part("years"))
    require(years["height"] > 40., "Year controls do not have a viewport")
    images.append(capture(app, plan, "years-dark"))
    app.activate(part("year-prev"))
    app.activate(part("year-1900"))
    require("1900年2月" in node(settled(app), part("grid"))["name"], "Choosing a year did not update the calendar")
    key(app, "escape")
    focus(app, BASE + "-open")
    open_picker(app, input_mode=True)
    frame = focus(app, INPUT)
    input_rect = bounds(frame, PICKER)
    assert_size(input_rect, 328., 512.)
    require(node(frame, INPUT)["described_by_text"] == part("hint"), "Format helper isn't associated")
    fill(app, "2024-2-9")
    key(app, "tab", extend=True)
    frame = settled(app)
    require(node(frame, INPUT)["text"] == "2024/02/09", "Blur did not normalize the input")
    fill(app, "2024/02/30")
    key(app, "enter")
    frame = focus(app, INPUT)
    error = node(frame, part("error"))["text"]
    require("這個日期不存在" in error and node(frame, INPUT)["described_by_text"] == part("error"),
            "The invalid date error did not reach the input")
    images.append(capture(app, plan, "input-invalid-dark"))
    fill(app, "２０２４/２/２９")
    key(app, "enter")
    frame = focus(app, BASE + "-input-open")
    require("2024/02/29" in node(frame, BASE + "-status")["text"], "Enter did not commit the valid date")
    return {"calendar": rect, "input": input_rect, "years": years, "semantics": semantic,
            "invalid": error, "images": images}


def compact(app, plan):
    scene(app)
    for setting in ("theme", "rtl", "large-text"):
        app.activate(BASE + "-" + setting)
    open_picker(app)
    frame = focus(app, day("2024-02-28"))
    rect, cell = bounds(frame, PICKER), bounds(frame, day("2024-02-28"))
    assert_size(rect, 320., 520.)
    require(cell["width"] >= 47.5 and cell["height"] >= 47.5, f"Day target below 48dp: {cell}")
    require(same_box(rect, bounds(frame, part("confirm"))), "Pinned Save is outside the viewport")
    key(app, "left")
    focus(app, day("2024-02-29"))
    images = [capture(app, plan, "calendar-dark-rtl-200")]
    edge_dates = {}
    for name, date in (("end", "2024-03-02"), ("home", "2024-02-25")):
        key(app, name)
        frame = focus(app, day(date))
        rect_at_edge = bounds(frame, day(date))
        require(same_box(bounds(frame, part("body")), rect_at_edge)
                and same_box(bounds(frame, part("grid-scroll")), rect_at_edge),
                f"{name} left the focused edge day partly hidden")
        edge_dates[name] = {"date": date, "bounds": rect_at_edge}
    app.activate(part("mode"))
    frame = settled(app)
    require(same_box(rect, bounds(frame, INPUT)), "Compact input is outside the viewport")
    images.append(capture(app, plan, "input-dark-rtl-200"))
    app.activate(part("cancel"))
    focus(app, BASE + "-open")
    return {"calendar": rect, "cell": cell, "edge_dates": edge_dates, "images": images}


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
    for name, size, checks in (("desktop", (960, 760), desktop),
                               ("compact", (320, 520), compact)):
        with launch(plan, name, *size) as app:
            before = onscreen_windows(app.proc.pid)
            data = checks(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(before == 0 and after == 0, f"Unexpected visible Gallery window {name}")
            data["onscreen_windows"] = {"before": before, "after": after}
            report[name] = data
            print(f"PASS {name}: Date Picker and PNG transport", flush=True)
    (plan.out / "native-date-picker-report.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
