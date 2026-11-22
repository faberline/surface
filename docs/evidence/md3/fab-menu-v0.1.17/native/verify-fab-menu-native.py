#!/usr/bin/env python3
"""Verify both FAB Menu presentations with the Surface headless Metal host."""

import argparse
import json
import math
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, launch, same_box
from window_check import onscreen_windows

BASE = "fab-menu-demo"
TRIGGER, FACE, LIST, MENU, SCOPE, OUTSIDE = (
    BASE + suffix for suffix in ("-trigger", "-trigger-face", "-list", "-menu", "-scope", "-outside")
)
ITEMS = [f"{BASE}-item-{index}" for index in range(6)]
SIZES = {"regular": 56, "medium": 80, "large": 96}
LABELS = ["筆記", "任務", "會議紀錄", "匯入專案參考資料", "提醒", "工作區捷徑"]
PALETTES = ("primary", "secondary", "tertiary")
CLOSE_NAME = "關閉動作選單"
OPEN_NAME = "新增項目"
SHOT = {"scale": 1, "max_side": 1200}


def locate(frame, key):
    for index, node in enumerate(frame["dom"]["nodes"]):
        if node.get("stable_id") == key:
            return index, node
    return None, None


def node(frame, key):
    result = locate(frame, key)[1]
    require(result is not None, f"Missing node {key}")
    return result


def bounds(frame, key):
    index = locate(frame, key)[0]
    require(index is not None, f"Missing layout {key}")
    return frame["layout"]["nodes"][index]["bounds"]


def end(rect, axis):
    return rect[axis] + rect["width" if axis == "x" else "height"]


def near(actual, expected, tolerance=0.6):
    return abs(actual - expected) <= tolerance


def settled(app):
    first = app.dom()
    for _ in range(14):
        current = app.dom()
        if first == current:
            return current
        first = current
    raise RuntimeError("Gallery did not settle")


def focus(app, key):
    frame = settled(app)
    require(focused(frame) == key, f"Expected {key} focus, got {focused(frame)}")
    return frame


def activate(app, suffix):
    app.activate(BASE + "-" + suffix)


def open_scene(app):
    app.activate("section-1")
    activate(app, "show")
    return focus(app, BASE + "-back")


def capture(app, plan, name, frame, paint=()):
    path = plan.out / (name + ".png")
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    raw = app.wire("Page.screenshot", SHOT)
    mcp, mcp_meta, _ = app.mcp(SHOT)
    encoded = path.read_bytes()
    require(encoded == png_payload(raw) == mcp, f"{name}: screenshot transport bytes differ")
    require(metadata(cli) == metadata(raw) == mcp_meta, f"{name}: screenshot metadata differ")
    width, height, rgba = read_png(encoded)
    require((width, height) == (cli["width"], cli["height"]), f"Wrong dimensions {name}")
    result = {"file": path.name, "sha256": digest(path), **metadata(cli), "transport_parity": True}
    for key in paint:
        rect = bounds(frame, key)
        x = math.floor(rect["x"] + 9.)
        y = math.floor(rect["y"] + rect["height"] / 2.)
        require(0 <= x < width and 0 <= y < height, f"Paint sample lies outside {name}")
        wanted = node(frame, key)["style"]["background"][:3]
        actual = tuple(rgba[y][x * 4:x * 4 + 3])
        require(max(abs(a - b) for a, b in zip(actual, wanted)) <= 10,
                f"{name}: {key} fill sample {actual} != {wanted}")
        result.setdefault("fills", {})[key] = actual
    return result


def check_native(frame, closed, count, rtl, short=False):
    root = frame["layout"]["viewport"]
    close, scroller = bounds(frame, FACE), bounds(frame, LIST)
    require(near(close["width"], 56.) and near(close["height"], 56.), f"Close token {close}")
    require(near(close["y"], closed["y"]), f"Trigger top moved {closed} -> {close}")
    require(near(close["x"], closed["x"]) if rtl else near(end(close, "x"), end(closed, "x")),
            f"Logical trailing anchor moved {closed} -> {close}")
    require(same_box(root, close) and same_box(root, scroller), "Menu or close outside viewport")
    require(end(scroller, "y") <= close["y"] + 0.5, f"List overlaps pinned close {close}")
    owner = node(frame, TRIGGER)
    require(owner["role"] == "Button" and owner["name"] == CLOSE_NAME,
            f"Unexpected native close semantics {owner.get('name')}")
    previous_end = float("-inf")
    ordered = []
    for index in range(count):
        identity = ITEMS[index]
        item = node(frame, identity)
        require(item["name"] == LABELS[index] and item["role"] == "Button",
                f"Missing native button semantics {identity}")
        require(item["disabled"] == (count == 6 and index == 2), f"Wrong disabled {identity}")
        rect = bounds(frame, identity)
        require(rect["height"] >= 47.5, f"Too small item {identity} {rect}")
        require(rect["y"] > previous_end, f"Item order or overlap failed at {identity}")
        previous_end = end(rect, "y")
        if not short:
            require(same_box(scroller, rect), f"Large viewport clipped {identity} {rect}")
        ordered.append(rect)
    return {"anchor": closed, "close": close, "clip": scroller, "item_rects": ordered}


def native_matrix(app, plan):
    activate(app, "floating")
    profiles, images = [], []
    for theme in ("light", "dark"):
        if theme == "dark":
            activate(app, "theme")
        for palette in PALETTES:
            activate(app, "palette-" + palette)
            for size, side in SIZES.items():
                activate(app, "size-" + size)
                for count, slug in [(2, "two"), (6, "six")]:
                    activate(app, "count-" + slug)
                    closed = bounds(settled(app), FACE)
                    require(near(closed["width"], side) and near(closed["height"], side),
                            f"Closed size {size}: {closed}")
                    app.activate(TRIGGER)
                    current = focus(app, TRIGGER)
                    profile = check_native(current, closed, count, False)
                    profiles.append(dict(theme=theme, palette=palette, size=size, count=count, **profile))
                    if size == "medium" and count == 6:
                        app.key("tab")
                        unfocused_close = focus(app, ITEMS[0])
                        images.append(capture(app, plan, f"native-{theme}-{palette}", unfocused_close,
                                              (FACE, ITEMS[-1] + "-face")))
                    app.key("escape")
                    require(node(focus(app, TRIGGER), TRIGGER)["name"] == OPEN_NAME,
                            "Escape did not restore the named opener")
    activate(app, "theme")
    return {"profiles": profiles, "images": images}


def web_actions(app, plan):
    activate(app, "web")
    activate(app, "size-large")
    activate(app, "count-two")
    activate(app, "palette-primary")
    before = settled(app)
    closed = bounds(before, FACE)
    app.activate(TRIGGER)
    frame = focus(app, ITEMS[0])
    require(bounds(frame, FACE) == closed, "Web FAB moved while opening")
    first = node(frame, ITEMS[0])
    owner = node(frame, TRIGGER)
    require(first["role"] == "MenuItem" and owner["name"] == OPEN_NAME, "Wrong Web roles or trigger")
    require(node(frame, MENU)["role"] == "Menu" and owner.get("expanded") is True, "Missing expanded menu")
    popup = bounds(frame, MENU)
    require(near(closed["y"] - end(popup, "y"), 4.), f"Bottom Web menu did not anchor above: {popup}")
    bottom = capture(app, plan, "web-light-bottom", frame)
    app.key("down")
    focus(app, ITEMS[1])
    app.key("enter")
    frame = focus(app, TRIGGER)
    require(node(frame, BASE + "-status")["text"] == "已新增 任務", "Web command did not activate")
    app.activate(TRIGGER)
    focus(app, ITEMS[0])
    app.key("tab")
    focus(app, OUTSIDE)
    activate(app, "top")
    activate(app, "theme")
    closed_at_top = bounds(settled(app), FACE)
    app.activate(TRIGGER)
    frame = focus(app, ITEMS[0])
    popup_at_top = bounds(frame, MENU)
    require(near(popup_at_top["y"] - end(closed_at_top, "y"), 4.),
            f"Top Web menu did not anchor below: {popup_at_top}")
    top = capture(app, plan, "web-dark-top", frame)
    app.key("escape")
    focus(app, TRIGGER)
    activate(app, "all-disabled")
    app.activate(TRIGGER)
    frame = focus(app, TRIGGER)
    require(all(node(frame, key)["disabled"] for key in ITEMS[:2]), "All-disabled state is missing")
    app.key("escape")
    focus(app, TRIGGER)
    return {"bottom": bottom, "top": top, "bottom_menu": popup, "top_menu": popup_at_top,
            "tab_after": OUTSIDE, "disabled_close_focus": TRIGGER}


def short_compact(app, plan):
    activate(app, "floating")
    activate(app, "theme")
    activate(app, "rtl")
    activate(app, "size-large")
    activate(app, "count-six")
    activate(app, "large-text")
    closed = bounds(settled(app), FACE)
    app.activate(TRIGGER)
    current = focus(app, TRIGGER)
    layout = check_native(current, closed, 6, True, short=True)
    require(same_box(layout["clip"], layout["item_rects"][-1]),
            "Opening clipped the nearest action")
    keyboard = []
    for index in (0, 1, 3, 4, 5):
        app.key("tab")
        frame = focus(app, ITEMS[index])
        require(bounds(frame, FACE) == layout["close"], "Close moved while revealing a focused item")
        area = bounds(frame, LIST)
        item = bounds(frame, ITEMS[index])
        require(item["y"] >= area["y"] - 0.5 and end(item, "y") <= end(area, "y") + 0.5,
                f"Focus reveal clipped {ITEMS[index]}: {item} vs {area}")
        keyboard.append({"focus": ITEMS[index], "rect": item})
        if index == 3:
            label = bounds(frame, ITEMS[index] + "-label")
            require(same_box(item, label), f"200 percent label extends beyond the pill: {label}")
            wrapped = capture(app, plan, "native-dark-rtl-200-wrapped", frame)
    selected = capture(app, plan, "native-dark-rtl-200-scrolled", frame)
    app.key("space")
    after = focus(app, TRIGGER)
    require(node(after, BASE + "-status")["text"] == "已新增 工作區捷徑",
            "Focused compact action did not activate")
    return {"layout": layout, "traversal": keyboard, "selected": selected, "wrapped": wrapped}


def main():
    parser = argparse.ArgumentParser()
    for name in ("gallery", "cli", "out"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    plan.out.mkdir(exist_ok=False)
    result = {"source_manifest_sha256": plan.source, "gallery_sha256": digest(plan.gallery),
              "cli_sha256": digest(plan.cli), "headless_renderer": "WGPU native macOS"}
    for name, width, height in (("desktop", 960, 800), ("compact", 320, 360)):
        with launch(plan, name, width, height) as app:
            before = onscreen_windows(app.proc.pid)
            require(before == 0, f"{name} Gallery owns a visible window")
            open_scene(app)
            if name == "desktop":
                data = {"native": native_matrix(app, plan), "web": web_actions(app, plan)}
            else:
                data = short_compact(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(after == 0, f"{name} Gallery opened a window")
            result[name] = {**data, "onscreen_windows": {"before": before, "after": after}}
        print(f"PASS {name} geometry, focus, and screenshot transport", flush=True)
    (plan.out / "native-fab-menu-report.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
