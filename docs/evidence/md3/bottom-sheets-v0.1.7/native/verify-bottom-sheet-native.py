#!/usr/bin/env python3
"""Check the bottom sheet on the native GPU without opening a window."""

import argparse
import json
from pathlib import Path
import time

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box

ID = "bottom-sheet-demo-"
PANEL = ID + "panel"
OPEN = ID + "open"
HANDLE = PANEL + "-handle"
CLOSE = PANEL + "-close"
HEADER = PANEL + "-header"
CONTENT = PANEL + "-content"
LAST = ID + "last"
MAIN = ID + "main-action"
DISABLED = ID + "disabled"
SCREENSHOT = {"scale": 1, "max_side": 1200}


def node(frame, name):
    for index, item in enumerate(frame["dom"]["nodes"]):
        if item["stable_id"] == name:
            return item, frame["layout"]["nodes"][index]
    return None, None


def bounds(frame, name):
    return node(frame, name)[1]["bounds"]


def until(app, ready, message):
    deadline = time.monotonic() + 5
    while True:
        frame = app.dom()
        if ready(frame):
            return frame
        require(time.monotonic() < deadline, message)
        time.sleep(0.03)


def capture(app, folder, name, parity=False):
    path = folder / f"{name}.png"
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    body = path.read_bytes()
    if parity:
        raw = app.wire("Page.screenshot", SCREENSHOT)
        mcp, mcp_meta, _ = app.mcp(SCREENSHOT)
        require(body == png_payload(raw) == mcp, "PNG bytes differ across transports")
        require(metadata(cli) == metadata(raw) == mcp_meta, "PNG metadata differs across transports")
    width, height, rows = read_png(body)
    require((width, height) == (cli["width"], cli["height"]), "PNG dimensions differ from metadata")
    return {"file": path.name, "sha256": digest(path), **metadata(cli), "three_client_parity": parity}, rows


def rgb(rows, x, y):
    return list(rows[int(y)][int(x) * 4 : int(x) * 4 + 3])


def near(actual, expected, tolerance):
    return max(abs(a - b) for a, b in zip(actual, expected)) <= tolerance


def check_paint(frame, shot, background, width):
    panel = bounds(frame, PANEL)
    color = node(frame, PANEL)[0]["style"]["background"][:3]
    sample = rgb(shot, panel["x"] + 8, panel["y"] + panel["height"] * 0.6)
    require(near(sample, color, 8), f"Incorrect panel paint {sample}, expected {color}")
    before = rgb(background, width - 16, 8)
    after = rgb(shot, width - 16, 8)
    scrim = node(frame, PANEL + "-layer")[0]["style"]["background"]
    expected = [round(c + (tint - c) * scrim[3] / 255) for c, tint in zip(before, scrim)]
    require(near(after, expected, 8), f"Incorrect scrim paint {after}, expected {expected}")
    return {"panel_rgb": sample, "background_rgb": before, "page_rgb": after, "scrim": scrim}


def check_geometry(frame, w, h, height, modal):
    sheet, geometry = node(frame, PANEL)
    rect = geometry["bounds"]
    require(sheet["name"] == "工作區篩選" and sheet["role"] == ("Dialog" if modal else "Group"),
            f"Wrong panel semantics {sheet['role']} {sheet['name']}")
    want_width = min(w, 640)
    require(rect == {"x": (w - want_width) / 2, "y": h - height,
                     "width": want_width, "height": height}, f"Wrong panel geometry {rect}")
    control = bounds(frame, HANDLE)
    require(control["width"] == control["height"] == 48 and same_box(rect, control),
            f"Wrong handle target {control}")
    require(bounds(frame, CONTENT)["height"] == height - 48, "Wrong content viewport height")
    require(node(frame, DISABLED)[0]["disabled"], "Disabled action became enabled")
    return rect


def check_case(plan, app, profile, w, h, modal, skip_peek):
    title = f"{profile}-{'modal' if modal else 'standard'}{'-direct' if skip_peek else ''}"
    app.activate(ID + ("modal" if modal else "standard"))
    if skip_peek:
        app.activate(ID + "skip-peek")
    start = app.dom()
    body = bounds(start, PANEL + "-body")
    require(node(start, PANEL)[0] is None, "Panel existed before opening")
    closed, closed_rows = capture(app, plan.out, title + "-closed")
    app.activate(OPEN)
    peek = 246 if w < 600 else 206
    expanded = min(h - 24, 624)
    initial = expanded if skip_peek else peek
    frame = until(app, lambda frame: node(frame, PANEL)[1] is not None
                  and bounds(frame, PANEL)["height"] == initial, "Panel did not reach its first anchor")
    first = check_geometry(frame, w, h, initial, modal)
    require(bounds(frame, PANEL + "-body") == body, "Panel changed page geometry")
    if modal:
        require(focused(frame) == HANDLE, f"Wrong modal entry focus {focused(frame)}")
        app.wire("Action.activate", {"id": MAIN}, "outside_modal")
    else:
        require(focused(frame) == OPEN, f"Wrong standard entry focus {focused(frame)}")
        app.activate(MAIN)
        app.key("escape")
        require(node(app.dom(), PANEL)[0] is not None, "Main page Escape dismissed standard panel")
    if not modal:
        focus_by_tab(app, HANDLE)
    first_png, rows = capture(app, plan.out, title + ("-expanded" if skip_peek else "-peek"))
    paint = check_paint(frame, rows, closed_rows, w)
    app.wire("Action.activate", {"id": DISABLED}, "disabled")
    if not skip_peek:
        app.key("enter")
        frame = until(app, lambda frame: bounds(frame, PANEL)["height"] == expanded,
                      "Enter on the handle did not expand")
        require(focused(frame) == HANDLE, "Handle lost focus while expanding")
        expanded_rect = check_geometry(frame, w, h, expanded, modal)
        expanded_png, _ = capture(app, plan.out, title + "-expanded", parity=w < 600 and modal)
    else:
        expanded_rect = first
        expanded_png = first_png

    scrolled = None
    if w < 600:
        header = bounds(frame, HEADER)
        description = bounds(frame, ID + "description")
        require(description["height"] >= 80, f"Large Chinese paragraph did not wrap {description}")
        require(not same_box(bounds(frame, CONTENT), bounds(frame, LAST)), "Final action did not need scroll")
        app.activate(LAST)
        frame = until(app, lambda frame: focused(frame) == LAST and same_box(bounds(frame, CONTENT), bounds(frame, LAST)),
                      "Last action did not scroll into the viewport")
        require(bounds(frame, HEADER) == header, "Scrolling moved the header")
        scrolled, _ = capture(app, plan.out, title + "-scrolled", parity=modal)

    if modal and skip_peek:
        app.activate(CLOSE)
    else:
        app.key("escape")
    closed_frame = until(app, lambda frame: node(frame, PANEL)[0] is None, "The panel did not close")
    require(focused(closed_frame) == OPEN, f"Opener focus was not restored: {focused(closed_frame)}")
    require(bounds(closed_frame, PANEL + "-body") == body, "Page geometry did not return")
    return {"name": title, "first": first, "expanded": expanded_rect, "paint": paint,
            "background": closed, "opening": first_png, "largest": expanded_png, "scrolled": scrolled}


def main():
    parser = argparse.ArgumentParser()
    for key in ("gallery", "cli", "out"):
        parser.add_argument("--" + key, type=Path, required=True)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True, exist_ok=False)
    cases = []
    for profile, w, h in [("light", 960, 800), ("dark-200-narrow", 320, 480)]:
        for modal, skip_peek in [(False, False), (True, False), (True, True)]:
            with launch(plan, profile, w, h) as app:
                app.activate("section-2")
                app.activate(ID + "show")
                if w < 600:
                    app.activate("theme")
                    app.activate(ID + "scale")
                result = check_case(plan, app, profile, w, h, modal, skip_peek)
                cases.append(result)
                print("PASS", result["name"], flush=True)
    report = {"mode": "headless", "source_manifest_sha256": plan.source, "cases": cases}
    (plan.out / "native-bottom-sheet-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
