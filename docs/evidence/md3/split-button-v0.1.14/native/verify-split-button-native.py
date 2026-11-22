#!/usr/bin/env python3
"""Replay the Material 3 Split Button through the native offscreen renderer."""

import argparse
import json
import math
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box
from window_check import onscreen_windows

TRACK = "split-track"
MAIN, OPTIONS = TRACK + "-leading", TRACK + "-trailing"
FACES = (MAIN + "-face", OPTIONS + "-face")
LABEL, MENU = TRACK + "-label", TRACK + "-menu"
ITEMS = tuple(f"split-menu-item-{i}" for i in range(3))
SIZES = dict(zip(("xs", "s", "m", "l", "xl"), (32, 40, 56, 96, 136)))
APPEARANCES = ("elevated", "filled", "tonal", "outlined")
BG = {"light": (253, 247, 255), "dark": (20, 18, 24)}
SCREENSHOT = {"scale": 1, "max_side": 1200}


def find(frame, key):
    for index, source in enumerate(frame["dom"]["nodes"]):
        if source.get("stable_id") == key:
            return index, source
    return None, None


def node(frame, key):
    source = find(frame, key)[1]
    require(source is not None, f"Missing {key}")
    return source


def bounds(frame, key):
    index = find(frame, key)[0]
    require(index is not None, f"Missing layout for {key}")
    return frame["layout"]["nodes"][index]["bounds"]


def end(box, axis):
    return box[axis] + box["width" if axis == "x" else "height"]


def expect_focus(app, key):
    frame = app.dom()
    require(focused(frame) == key, f"Expected {key} focus, got {focused(frame)}")
    return frame


def settled(app):
    previous = app.dom()
    for _ in range(7):
        frame = app.dom()
        if frame == previous:
            return frame
        previous = frame
    raise RuntimeError("Layout did not settle across eight offscreen frames")


def open_demo(app):
    app.activate("section-1")
    app.activate("split-demo-show")
    return expect_focus(app, "split-demo-back")


def near(actual, expected, limit=3):
    return max(abs(a - b) for a, b in zip(actual, expected)) <= limit


def blended(rgba, background):
    r, g, b, a = rgba
    return tuple(round((a * channel + (255 - a) * bg) / 255)
                 for channel, bg in zip((r, g, b), background))


def rounded_corner_excess(box, corners, pixel_x, pixel_y):
    x, y = pixel_x + 0.5, pixel_y + 0.5
    left, right = x - box["x"], end(box, "x") - x
    top, bottom = y - box["y"], end(box, "y") - y
    excess = 0.
    radius_max = min(box["width"], box["height"]) / 2
    for name, dx, dy in (("top_left", left, top), ("top_right", right, top),
                         ("bottom_left", left, bottom), ("bottom_right", right, bottom)):
        radius = min(corners[name], radius_max)
        if dx < radius and dy < radius:
            excess = max(excess, math.hypot(radius - dx, radius - dy) - radius)
    return excess


def capture(app, plan, name, frame, theme, appearance=None, label_line_height=None):
    path = plan.out / (name + ".png")
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    raw = app.wire("Page.screenshot", SCREENSHOT)
    mcp, mcp_metadata, _ = app.mcp(SCREENSHOT)
    encoded = path.read_bytes()
    require(encoded == png_payload(raw) == mcp, f"{name}: raw, CLI and MCP bytes differ")
    require(metadata(cli) == metadata(raw) == mcp_metadata, f"{name}: transport metadata differs")
    width, height, rows = read_png(encoded)
    require((width, height) == (cli["width"], cli["height"]), f"{name}: PNG dimensions differ")

    def pixel(x, y):
        x, y = math.floor(x), math.floor(y)
        require(0 <= x < width and 0 <= y < height, f"Invalid {name} pixel ({x},{y})")
        return tuple(rows[y][x * 4:x * 4 + 3])

    background = BG[theme]
    require(near(pixel(2, height - 6), background), f"{name}: wrong page paint")
    receipt = {"file": path.name, "sha256": digest(path), **metadata(cli), "transport_parity": True}
    if appearance is not None:
        fills = []
        for key in FACES:
            box = bounds(frame, key)
            x, mid = box["x"], box["y"] + box["height"] / 2
            style = node(frame, key)["style"]
            fill = pixel(x + 6.5, mid)
            expected = blended(style["background"], background)
            require(near(fill, expected), f"{name} {key}: fill {fill} != {expected}")
            border = style["border_width"]
            require((border > 0) == (appearance == "outlined"), f"{name} {key}: wrong stroke token")
            stroke = None
            if border:
                stroke_color = blended(style["border_color"], background)
                candidates = [pixel(x + offset, mid) for offset in (0.25, 0.75, 1.25)]
                stroke = min(candidates, key=lambda p: sum(abs(a - b) for a, b in zip(p, stroke_color)))
                require(near(stroke, stroke_color, 16) and not near(stroke, fill, 8),
                        f"{name} {key}: expected visible outline, found {stroke} over {fill}")
            fills.append({"id": key, "fill_rgb": fill, "stroke_rgb": stroke})
        face = bounds(frame, FACES[0])
        below = pixel(face["x"] + face["width"] / 2, end(face, "y") + 0.75)
        receipt["paint"] = {"halves": fills, "below_rgb": below}
    if label_line_height is not None:
        label = bounds(frame, LABEL)
        ink = node(frame, LABEL)["style"]["foreground"][:3]
        face = bounds(frame, FACES[0])
        corners = node(frame, FACES[0])["style"]["corners"]
        canvas = pixel(2, height - 6)
        left, right = math.ceil(label["x"]), math.floor(end(label, "x"))
        top = label["y"]
        ink_pixels = []
        outside = []
        for line in range(2):
            start = math.ceil(top + line * label_line_height)
            stop = math.floor(top + (line + 1) * label_line_height)
            count = 0
            for y in range(start, stop):
                for x in range(left, right):
                    color = pixel(x, y)
                    if near(color, ink, 16) and not near(color, canvas, 4):
                        count += 1
                        excess = rounded_corner_excess(face, corners, x, y)
                        if excess > 1.:
                            outside.append((excess, x, y))
            require(count > 32, f"{name}: visible label line {line + 1} has only {count} pixels")
            ink_pixels.append(count)
        require(not outside, f"{name}: {len(outside)} ink pixels outside the rounded corners. "
                f"Furthest exceeds radius by {max(outside, default=(0., 0, 0))} (distance, x, y)")
        receipt["label_ink_pixels_per_line"] = ink_pixels
        receipt["label_ink_pixels_past_rounded_corners"] = 0
    return receipt


def layout_case(frame, size, rtl=False, scales_text=False):
    track = bounds(frame, TRACK)
    hits = [bounds(frame, key) for key in (MAIN, OPTIONS)]
    faces = [bounds(frame, key) for key in FACES]
    for hit, face in zip(hits, faces):
        require(same_box(track, hit) and same_box(hit, face)
                and min(hit["width"], hit["height"]) >= 47.5,
                f"{size}: small or clipped target {hit}, face {face}, track {track}")
        height = face["height"]
        require(height >= SIZES[size] - 0.5 and (scales_text or height <= SIZES[size] + 0.5),
                f"{size}: wrong face height {face}")
    left, right = hits[::-1] if rtl else hits
    require(end(left, "x") <= right["x"] + 0.5,
            f"{size}: overlapping targets or wrong direction: {left}, {right}")
    require(abs(faces[0]["y"] - faces[1]["y"]) <= 0.5
            and abs(faces[0]["height"] - faces[1]["height"]) <= 0.5,
            f"Faces do not share one row and height: {faces}")
    viewport = frame["layout"]["viewport"]
    require(same_box(viewport, track), f"Track overflows viewport: {track}")
    outer_gutters = [track["x"] - viewport["x"], end(viewport, "x") - end(track, "x")]
    require(min(outer_gutters) >= 15.5, f"Track loses the 16 dp viewport gutter: {outer_gutters}")
    left_face, right_face = faces[::-1] if rtl else faces
    face_gap = right_face["x"] - end(left_face, "x")
    require(abs(face_gap - 2) <= 0.5, f"Visual halves lost the 2 dp gap: {face_gap}")
    return {"hits": hits, "faces": faces, "track": track, "outer_gutters": outer_gutters,
            "face_gap": face_gap}


def leading_content(frame, form):
    main = node(frame, MAIN)
    label = find(frame, LABEL)[1]
    content_index = find(frame, TRACK + "-leading-content")[0]
    icons = [source for source in frame["dom"]["nodes"]
             if source.get("parent") == content_index and "fill_contour" in source]
    require(main["name"] == "匯出報告", f"{form}: main action lost its name")
    require((label is not None) == (form != "icon") and len(icons) == int(form != "text"),
            f"{form}: wrong leading content")
    if label is not None:
        require(same_box(bounds(frame, MAIN), bounds(frame, LABEL)), f"{form}: label overflows")
    return {"text": label["text"] if label is not None else None, "icons": len(icons),
            "main_width": bounds(frame, MAIN)["width"]}


def menu_state(frame, opened, flip=False):
    trigger = node(frame, OPTIONS)
    require(trigger["role"] == "Button" and trigger["name"] == "匯出選項"
            and trigger.get("expanded") is opened and "disclosure" not in trigger
            and "expanded" not in node(frame, MAIN),
            "DOM menu popup combination or independent main semantics changed")
    menu = find(frame, MENU)[1]
    require((menu is not None) == opened, "DOM popup mount state disagrees with the trigger")
    if not opened:
        return None
    require(menu["role"] == "Menu" and menu["name"] == "匯出格式", "Missing named menu")
    box = bounds(frame, MENU)
    track = bounds(frame, TRACK)
    require(same_box(frame["layout"]["viewport"], box), "Popup is outside the viewport")
    if flip:
        require(end(box, "y") <= track["y"] + 0.5, "Popup did not flip up")
    else:
        require(box["y"] >= end(track, "y") - 0.5, "Popup did not open below")
    for i, key in enumerate(ITEMS):
        item = node(frame, key)
        require(item["role"] == "MenuItem" and item["disabled"] == (i == 1)
                and same_box(box, bounds(frame, key)), f"Incorrect menu command {key}")
    return box


def keyboard(app, plan):
    app.activate("split-large-text")
    app.activate("split-large-text")
    require(focus_by_tab(app, MAIN) == [MAIN], "Tab did not enter on the primary action")
    require(focus_by_tab(app, OPTIONS) == [OPTIONS], "Both halves must be Tab stops")
    menu_state(expect_focus(app, OPTIONS), False)
    app.key("enter")
    below = menu_state(expect_focus(app, ITEMS[0]), True)
    app.wire("Action.activate", {"id": ITEMS[1]}, expected_error="disabled")
    app.key("down")
    expect_focus(app, ITEMS[2])
    app.key("up")
    expect_focus(app, ITEMS[0])
    app.key("down")
    shot = capture(app, plan, "light-keyboard-menu", expect_focus(app, ITEMS[2]), "light")
    app.key("enter")
    frame = expect_focus(app, OPTIONS)
    menu_state(frame, False)
    require(node(frame, "split-status")["text"] == "已執行 純文字", "Enter did not invoke command")
    for key, shift, target in (("escape", False, OPTIONS), ("tab", False, "split-disable-main"),
                               ("tab", True, MAIN)):
        if focused(app.dom()) != OPTIONS:
            focus_by_tab(app, OPTIONS)
        app.key("space")
        menu_state(expect_focus(app, ITEMS[0]), True)
        app.wire("Action.key", {"key": key, "extend": shift})
        frame = expect_focus(app, target)
        menu_state(frame, False)
        require(node(frame, "split-status")["text"] == "已執行 純文字",
                f"{key} unexpectedly invoked a command")
    return {"command_focus": OPTIONS, "escape_focus": OPTIONS, "tab_focus": "split-disable-main",
            "shift_tab_focus": MAIN, "lower_placement": below, "capture": shot}


def independent_disabled(app):
    def disable(key, other):
        app.activate("split-disable-" + ("main" if key == MAIN else "options"))
        frame = app.dom()
        require(node(frame, key)["disabled"] and not node(frame, other)["disabled"],
                f"Disabling {key} affected both controls")
        app.wire("Action.activate", {"id": key}, expected_error="disabled")

    disable(MAIN, OPTIONS)
    app.activate(OPTIONS)
    menu_state(expect_focus(app, ITEMS[0]), True)
    app.key("escape")
    menu_state(expect_focus(app, OPTIONS), False)
    app.activate("split-disable-main")

    disable(OPTIONS, MAIN)
    app.activate(MAIN)
    frame = expect_focus(app, MAIN)
    menu_state(frame, False)
    require(node(frame, "split-status")["text"] == "已執行 匯出報告", "Primary action did not run")
    app.activate("split-disable-options")
    return {"disabled_main_keeps_menu": True, "disabled_menu_keeps_primary": True}


def desktop(app, plan):
    menu_state(open_demo(app), False)
    variants = []
    matrix = []
    for theme in ("light", "dark"):
        if theme == "dark":
            app.activate("split-theme")
        for appearance in APPEARANCES:
            app.activate(f"split-appearance-{appearance}")
            for size in SIZES:
                app.activate(f"split-size-{size}")
                frame = app.dom()
                entry = layout_case(frame, size)
                matrix.append({"theme": theme, "appearance": appearance, "size": size, **entry})
                if size == "m":
                    variants.append(capture(app, plan, f"{theme}-{appearance}", frame,
                                            theme, appearance))
        elevated, filled = [item["paint"]["below_rgb"] for item in variants[-4:-2]]
        require(not near(elevated, filled, 1) and all(a <= b for a, b in zip(elevated, filled)),
                f"{theme}: elevated shadow did not darken the nearby surface: {elevated}, {filled}")
    app.activate("split-theme")
    app.activate("split-appearance-filled")
    app.activate("split-size-m")
    contents = {}
    for form in ("icon", "text", "both"):
        app.activate(f"split-leading-{form}")
        contents[form] = leading_content(app.dom(), form)
    require(contents["both"]["main_width"] > contents["icon"]["main_width"] + 4.,
            "Text and icon do not expand the primary face")

    keys = keyboard(app, plan)
    disabled = independent_disabled(app)
    app.activate("split-anchor-bottom")
    app.activate(OPTIONS)
    frame = expect_focus(app, ITEMS[0])
    upper = menu_state(frame, True, flip=True)
    shot = capture(app, plan, "light-flipped-menu", frame, "light")
    app.key("escape")
    menu_state(expect_focus(app, OPTIONS), False)
    return {"matrix": matrix, "paint": variants, "leading_content": contents,
            "keyboard": keys, "disabled": disabled, "upper_placement": upper, "upper_capture": shot}


def compact(app, plan):
    open_demo(app)
    for control in ("theme", "rtl", "size-xl"):
        app.activate(f"split-{control}")
    previous_height = bounds(settled(app), LABEL)["height"]
    app.activate("split-large-text")
    frame = settled(app)
    layout = layout_case(frame, "xl", rtl=True, scales_text=True)
    content = leading_content(frame, "both")
    label = bounds(frame, LABEL)
    require(label["height"] >= 1.75 * previous_height,
            f"Text did not grow to 200 percent: {previous_height}, {label}")
    label_style = node(frame, LABEL)["style"]
    line_height = 2 * (label_style.get("line_height") or 1.5 * label_style["font_size"])
    require(content["text"] == "匯出報告" and abs(label["height"] - 2 * line_height) <= 0.75,
            f"Expected two lines for the four-glyph label at 200 percent: {label}, {line_height}")
    shot = capture(app, plan, "dark-rtl-200-xl", frame, "dark", "filled", line_height)
    app.activate(OPTIONS)
    frame = expect_focus(app, ITEMS[0])
    popup = menu_state(frame, True)
    open_shot = capture(app, plan, "dark-rtl-200-xl-open", frame, "dark",
                        label_line_height=line_height)
    app.key("down")
    expect_focus(app, ITEMS[2])
    app.wire("Action.key", {"key": "tab", "extend": True})
    menu_state(expect_focus(app, MAIN), False)
    return {"layout": layout, "label": label, "label_before_scaling": previous_height,
            "label_line_height": line_height, "label_lines": 2,
            "content": content, "popup": popup, "closed_capture": shot, "open_capture": open_shot}


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
            before = onscreen_windows(app.proc.pid)
            require(before == 0, f"{name} Gallery owns a visible window")
            data = run(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(after == 0, f"{name} replay opened a window")
            report[name] = {**data, "onscreen_windows": {"before": before, "after": after}}
        print(f"PASS {name} layout, paint, keyboard and screenshot transport", flush=True)
    path = plan.out / "native-split-button-report.json"
    path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
