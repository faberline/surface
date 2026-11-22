#!/usr/bin/env python3
"""Replay Material 3 Button Groups through the native offscreen renderer."""

import argparse
import json
import math
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box
from window_check import onscreen_windows


PREFIX = "button-group-"
ITEMS = tuple(PREFIX + f"item-{index}" for index in range(5))
TRACK = PREFIX + "track"
OVERFLOW = TRACK + "-overflow"
MENU = TRACK + "-menu"
SIZES = ("xs", "s", "m", "l", "xl")
HEIGHT = dict(zip(SIZES, (32, 40, 56, 96, 136)))
GAP = dict(zip(SIZES, (18, 12, 8, 8, 8)))
SQUARE = dict(zip(SIZES, (12, 12, 16, 28, 28)))
OUTER = dict(zip(SIZES, (4, 8, 8, 16, 20)))
INNER = dict(zip(SIZES, (8, 8, 8, 16, 20)))
SCREENSHOT = {"scale": 1, "max_side": 1200}
PALETTE = {
    "light": ((253, 247, 255), (232, 222, 248), (98, 91, 113)),
    "dark": ((20, 18, 24), (74, 68, 88), (203, 194, 219)),
}
REPRESENTATIVE = {
    ("light", "standard", "round"): "xs",
    ("light", "standard", "square"): "s",
    ("light", "connected", "round"): "m",
    ("light", "connected", "square"): "l",
    ("dark", "standard", "round"): "xl",
    ("dark", "standard", "square"): "m",
    ("dark", "connected", "round"): "s",
    ("dark", "connected", "square"): "xl",
}


def find(frame, stable_id):
    for index, node in enumerate(frame["dom"]["nodes"]):
        if node.get("stable_id") == stable_id:
            return node, frame["layout"]["nodes"][index]
    return None, None


def node(frame, stable_id):
    source, _ = find(frame, stable_id)
    require(source is not None, f"Missing {stable_id}")
    return source


def bounds(frame, stable_id):
    _, geometry = find(frame, stable_id)
    require(geometry is not None, f"Missing layout for {stable_id}")
    return geometry["bounds"]


def selected(frame):
    return [index for index, key in enumerate(ITEMS) if (find(frame, key)[0] or {}).get("checked")]


def assert_selected(frame, indices):
    actual = selected(frame)
    require(actual == indices, f"Selection is {actual}; expected {indices}")
    return actual


def expect_focus(app, target):
    frame = app.dom()
    require(focused(frame) == target, f"Expected focus on {target}, got {focused(frame)}")
    return frame


def open_group(app):
    app.activate("section-1")
    app.activate(PREFIX + "demo-show")
    frame = expect_focus(app, PREFIX + "demo-back")
    assert_selected(frame, [1])
    return frame


def tab_enters_first(app):
    route = focus_by_tab(app, ITEMS[0])
    require(all(key not in ITEMS for key in route[:-1]), f"Tab entered on the wrong item: {route}")
    return route


def same_radius(actual, expected):
    return actual > 10000 if expected == "full" else abs(actual - expected) <= 0.01


def verify_palette(frame, theme):
    surface, idle, active = PALETTE[theme]
    require(tuple(node(frame, PREFIX + "demo-page")["style"]["background"][:3]) == surface,
            f"Wrong {theme} surface")
    require(tuple(node(frame, ITEMS[0] + "-face")["style"]["background"]) == (*idle, 255),
            f"Wrong {theme} idle color")
    require(tuple(node(frame, ITEMS[1] + "-face")["style"]["background"]) == (*active, 255),
            f"Wrong {theme} selected color")


def matrix_case(frame, theme, kind, shape, size):
    verify_palette(frame, theme)
    assert_selected(frame, [1])
    require(node(frame, TRACK)["role"] == "Group" and node(frame, TRACK)["name"] == "圖片工具",
            "Missing named semantic group")
    require(node(frame, ITEMS[2])["disabled"], "Disabled item is selectable")
    viewport = bounds(frame, TRACK + "-viewport")
    faces = []
    margins = []
    indices = range(4) if find(frame, ITEMS[4])[0] is None else range(5)
    for index in indices:
        item = ITEMS[index]
        actual = bounds(frame, item + "-face")
        hit = bounds(frame, item)
        require(actual["height"] == HEIGHT[size], f"{size} visual token height changed: {actual}")
        require(hit["height"] >= 48 and hit["width"] >= 48 and same_box(hit, actual),
                f"{item} touches outside its hit area: {hit}, {actual}")
        require(same_box(viewport, hit), f"{item} overflows the desktop group")
        faces.append(actual)
        margins.append((hit["width"] - actual["width"]) / 2)
    gaps = [right["x"] - left["x"] - left["width"] for left, right in zip(faces, faces[1:])]
    token = 2. if kind == "connected" else GAP[size]
    expected_gaps = [max(token, left + right) for left, right in zip(margins, margins[1:])]
    require(all(abs(gap - want) <= 1 for gap, want in zip(gaps, expected_gaps)),
            f"{kind} {size} face gaps {gaps} differ from {expected_gaps}")
    first_corners = node(frame, ITEMS[0] + "-face")["style"]["corners"]
    selected_corners = node(frame, ITEMS[1] + "-face")["style"]["corners"]
    if kind == "connected":
        left, right = ("full" if shape == "round" else OUTER[size]), INNER[size]
        selected_radius = "full"
    else:
        left = right = "full" if shape == "round" else SQUARE[size]
        selected_radius = SQUARE[size] if shape == "round" else "full"
    require(all(same_radius(first_corners[side], want) for side, want in (
        ("top_left", left), ("bottom_left", left), ("top_right", right), ("bottom_right", right)
    )), f"{kind} {size} {shape} first corners do not match: {first_corners}")
    require(all(same_radius(value, selected_radius) for value in selected_corners.values()),
            f"{kind} {size} {shape} selected corners do not match: {selected_corners}")
    command, _ = find(frame, ITEMS[4])
    if command:
        require(command.get("checked") is (False if kind == "connected" else None),
                "The last control has the wrong command/toggle semantics")
    else:
        require(kind == "standard" and find(frame, OVERFLOW)[0] is not None,
                "A command disappeared without overflow")
    return {"face_height": HEIGHT[size], "face_gaps": gaps, "visible_items": len(faces),
            "first_corners": first_corners, "selected_corners": selected_corners}


def near(actual, expected):
    return max(abs(a - b) for a, b in zip(actual, expected)) <= 1


def capture(app, plan, name, frame, theme):
    file = plan.out / (name + ".png")
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", file])
    raw = app.wire("Page.screenshot", SCREENSHOT)
    mcp, mcp_metadata, _ = app.mcp(SCREENSHOT)
    encoded = file.read_bytes()
    require(encoded == png_payload(raw) == mcp, f"{name} raw, CLI and MCP PNG bytes differ")
    require(metadata(cli) == metadata(raw) == mcp_metadata, f"{name} metadata differs")
    width, height, rows = read_png(encoded)
    require((width, height) == (cli["width"], cli["height"]), f"{name} PNG dimensions differ")

    def pixel(x, y):
        require(0 <= x < width and 0 <= y < height, f"{name} pixel ({x},{y}) is out of bounds")
        return tuple(rows[y][x * 4:x * 4 + 3])

    surface, idle, active = PALETTE[theme]
    require(near(pixel(4, height - 8), surface), f"{name} did not paint the surface")
    outline_samples = []
    for index, paint in enumerate((idle, active)):
        face_id = ITEMS[index] + "-face"
        box = bounds(frame, face_id)
        style = node(frame, face_id)["style"]
        # At this row the face is solid and no label or icon is present.
        y = math.ceil(box["y"]) + 2
        left = box["x"]
        right = left + box["width"]
        middle = math.floor((left + right) / 2)
        fill = pixel(middle, y)
        require(near(fill, paint), f"{name} {face_id} paint {fill} differs from {paint}")
        contrast = [foreground - background for foreground, background in zip(paint, surface)]
        midpoint = sum(channel * channel for channel in contrast) / 2.
        row = [x for x in range(math.floor(left), math.ceil(right))
               if sum((channel - background) * axis for channel, background, axis
                      in zip(pixel(x, y), surface, contrast)) >= midpoint]
        require(row, f"{name} {face_id} has no filled pixels")
        # Measure the visible edges against the same circular corner geometry.
        # Allow two pixels for antialiasing and logical-to-physical rounding.
        inset_y = y + 0.5 - box["y"]
        def inset(radius):
            radius = min(radius, box["width"] / 2, box["height"] / 2)
            return radius - math.sqrt(radius * radius - (radius - inset_y) ** 2) if radius > inset_y else 0.
        want_left = left + inset(style["corners"]["top_left"])
        want_right = right - inset(style["corners"]["top_right"])
        require(abs(row[0] + 0.5 - want_left) <= 2 and abs(row[-1] + 0.5 - want_right) <= 2,
                f"{name} {face_id} contour pixels [{row[0]}, {row[-1]}] differ from [{want_left}, {want_right}]")
        outline_samples.append({"id": face_id, "y": y, "fills_rgb": fill, "first_last_x": [row[0], row[-1]],
                                "expected_edges": [round(want_left, 3), round(want_right, 3)]})
    return {"file": file.name, "sha256": digest(file), **metadata(cli), "transport_parity": True,
            "paint": outline_samples}


def keyboard_journey(app):
    for setting in ("standard", "round", "size-s"):
        app.activate(PREFIX + setting)
    frame = app.dom()
    require(node(frame, ITEMS[4]).get("checked") is None, "Standard command has toggle semantics")
    app.wire("Action.activate", {"id": ITEMS[2]}, "disabled")
    app.cli(["action", "activate", ITEMS[2]], want_status=1)
    entry = tab_enters_first(app)
    assert_selected(expect_focus(app, ITEMS[0]), [1])
    for index in (1, 3):
        app.key("right")
        assert_selected(expect_focus(app, ITEMS[index]), [1])
    app.key("tab")
    require(focused(app.dom()) not in ITEMS, "Tab did not leave the group")
    app.wire("Action.key", {"key": "tab", "extend": True})
    expect_focus(app, ITEMS[0])
    app.key("enter")
    pressed = assert_selected(expect_focus(app, ITEMS[0]), [0])
    app.key("space")
    cleared = assert_selected(expect_focus(app, ITEMS[0]), [])
    app.activate(PREFIX + "required")
    assert_selected(app.dom(), [0])
    tab_enters_first(app)
    app.key("space")
    required = assert_selected(expect_focus(app, ITEMS[0]), [0])
    app.activate(PREFIX + "multi")
    tab_enters_first(app)
    app.key("right")
    app.key("enter")
    multiple = assert_selected(expect_focus(app, ITEMS[1]), [0, 1])
    app.key("space")
    assert_selected(expect_focus(app, ITEMS[1]), [0])
    app.activate(PREFIX + "commands")
    tab_enters_first(app)
    app.key("end")
    expect_focus(app, ITEMS[4])
    app.key("enter")
    frame = expect_focus(app, ITEMS[4])
    require(all(node(frame, key).get("checked") is None for key in ITEMS), "Commands expose checked state")
    require(node(frame, PREFIX + "status")["text"] == "已執行 匯出", "Command did not run")
    app.activate(PREFIX + "demo-back")
    expect_focus(app, PREFIX + "demo-show")
    return {"tab_route": entry, "arrows": [1, 3], "shift_tab_entry": 0,
            "enter_selected": pressed, "space_selected": cleared, "required_selected": required,
            "multi_selected": multiple, "command": ITEMS[4], "disabled_rejected": ITEMS[2],
            "back_focus": PREFIX + "demo-show"}


def desktop(app, plan):
    open_group(app)
    matrix, captures = {}, {}
    for theme in ("light", "dark"):
        if theme == "dark":
            app.activate(PREFIX + "theme")
        for kind in ("standard", "connected"):
            app.activate(PREFIX + kind)
            for shape in ("round", "square"):
                app.activate(PREFIX + shape)
                for size in SIZES:
                    app.activate(PREFIX + "size-" + size)
                    name = f"{theme}-{kind}-{shape}-{size}"
                    frame = app.dom()
                    matrix[name] = matrix_case(frame, theme, kind, shape, size)
                    if REPRESENTATIVE[(theme, kind, shape)] == size:
                        captures[name] = capture(app, plan, name, frame, theme)
    return {"layout_matrix": matrix, "captures": captures, "keyboard": keyboard_journey(app)}


def compact(app, plan):
    open_group(app)
    for setting in ("theme", "direction", "large-text"):
        app.activate(PREFIX + setting)
    frame = app.dom()
    verify_palette(frame, "dark")
    assert_selected(frame, [1])
    view = bounds(frame, TRACK + "-viewport")
    require(view["width"] <= 288.5, "The compact group overruns the page")
    regular = bounds(frame, ITEMS[0] + "-face")
    require(regular["height"] >= 80, "The 200% label did not grow the face")
    order = [bounds(frame, key)["x"] for key in (*ITEMS[:4], OVERFLOW)]
    require(all(a > b for a, b in zip(order, order[1:])), f"Expected visual RTL order: {order}")
    for key in (*ITEMS[:4], OVERFLOW):
        target = bounds(frame, key)
        require(same_box(view, target) and target["width"] >= 48 and target["height"] >= 48,
                f"Compact {key} is clipped or too small: {target}")
    require(find(frame, ITEMS[4])[0] is None, "The compact command should travel through overflow")
    shot = capture(app, plan, "dark-rtl-200", frame, "dark")
    tab = tab_enters_first(app)
    expect_focus(app, ITEMS[0])
    for index in (1, 3):
        app.key("left")
        assert_selected(expect_focus(app, ITEMS[index]), [1])
    app.key("left")
    expect_focus(app, OVERFLOW)
    app.key("enter")
    frame = expect_focus(app, ITEMS[4])
    require(node(frame, MENU)["role"] == "Menu", "The command popup has no Menu role")
    app.key("escape")
    require(find(expect_focus(app, OVERFLOW), MENU)[0] is None, "Escape left the menu open")
    app.key("enter")
    expect_focus(app, ITEMS[4])
    app.key("enter")
    frame = expect_focus(app, OVERFLOW)
    require(node(frame, PREFIX + "status")["text"] == "已執行 匯出", "Overflow command did not run")
    require(find(frame, MENU)[0] is None, "Activating the command left the menu open")
    return {"capture": shot, "rtl_x": order, "face_height": regular["height"],
            "tab_route": tab, "rtl_arrows": [1, 3], "overflow_focus": ITEMS[4],
            "escape_focus": OVERFLOW, "command_focus": OVERFLOW}


def main():
    parser = argparse.ArgumentParser()
    for name in ("gallery", "cli", "out"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    plan.out.mkdir(exist_ok=False)
    report = {"source_manifest_sha256": plan.source, "gallery_sha256": digest(plan.gallery),
              "cli_sha256": digest(plan.cli), "headless_renderer": "WGPU native macOS"}
    for name, width, run in (("desktop", 960, desktop), ("compact", 320, compact)):
        with launch(plan, name, width, 800) as app:
            require(onscreen_windows(app.proc.pid) == 0, f"{name} Gallery owns a visible window")
            report[name] = run(app, plan)
            report[name]["onscreen_windows"] = onscreen_windows(app.proc.pid)
            require(report[name]["onscreen_windows"] == 0, f"{name} replay opened a window")
        print(f"PASS {name} layout, paint, keyboard and screenshot transport", flush=True)
    (plan.out / "native-button-group-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
