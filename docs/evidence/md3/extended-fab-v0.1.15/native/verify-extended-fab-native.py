#!/usr/bin/env python3
"""Exercise Extended FAB layout and transport on the native headless GPU host."""

import argparse
import json
import math
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, launch, same_box
from window_check import onscreen_windows

ACTION = "extended-fab-action"
FACE, ICON, LABEL = (ACTION + "-" + suffix for suffix in ("face", "icon", "label"))
NAME = "新增專案筆記"
VISIBILITY = "extended-fab-toggle-visibility"
SIZES = {"small": (56, 16, 24), "medium": (80, 20, 28), "large": (96, 28, 36)}
PALETTES = ("tonal-primary", "tonal-secondary", "tonal-tertiary", "primary", "secondary", "tertiary")
BG = {"light": (253, 247, 255), "dark": (20, 18, 24)}
SHOT = {"scale": 1, "max_side": 1200}


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


def near(actual, expected, limit=0.75):
    return abs(actual - expected) <= limit


def color_near(actual, expected, limit=4):
    return max(abs(a - b) for a, b in zip(actual, expected)) <= limit


def stable(app):
    previous = app.dom()
    for _ in range(7):
        frame = app.dom()
        if frame == previous:
            return frame
        previous = frame
    raise RuntimeError("Measured Extended FAB layout did not settle")


def act(app, control):
    app.activate("extended-fab-" + control)
    return stable(app)


def expect_focus(app, key):
    frame = stable(app)
    require(focused(frame) == key, f"Expected focus on {key}, got {focused(frame)}")
    return frame


def open_demo(app):
    app.activate("section-1")
    app.activate("extended-fab-demo-show")
    return expect_focus(app, "extended-fab-back")


def assert_action(frame, disabled=False):
    button = node(frame, ACTION)
    require(button["role"] == "Button" and button["name"] == NAME
            and button["disabled"] == disabled, f"Incorrect stable action for disabled={disabled}")
    like_named = [entry for entry in frame["dom"]["nodes"]
                  if entry["role"] == "Button" and entry["name"] == NAME]
    require(len(like_named) == 1, "The Extended FAB must have one named button")


def layout_check(frame, size, *, rtl=False, centered=False, compact=False,
                 collapsed=False, text_only=False):
    assert_action(frame)
    face, hit = bounds(frame, FACE), bounds(frame, ACTION)
    viewport = frame["layout"]["viewport"]
    height, radius, icon_size = SIZES[size]
    require(same_box(viewport, hit) and same_box(hit, face)
            and min(hit["height"], hit["width"]) >= 48., f"Clipped or undersized action: {hit}")
    require(near(end(face, "y"), end(viewport, "y") - 16.), f"Bottom anchor moved: {face}")
    if centered:
        require(near(face["x"] + face["width"] / 2, viewport["width"] / 2),
                f"Face is not centered: {face}")
    elif rtl:
        require(near(face["x"], viewport["x"] + 16.), f"RTL leading edge moved: {face}")
    else:
        require(near(end(face, "x"), end(viewport, "x") - 16.), f"Trailing edge moved: {face}")
    require(face["height"] >= height - 0.75 and (compact or near(face["height"], height)),
            f"{size} has an invalid face height: {face}")
    require(face["width"] <= viewport["width"] - 31.,
            f"Face exceeds its 16 dp viewport margins: {face}")
    corners = node(frame, FACE)["style"]["corners"]
    require(all(near(corner, radius) for corner in corners.values()),
            f"{size} uses wrong radius: {corners}")
    icon = None
    if text_only:
        require(find(frame, ICON)[1] is None, "Icon is still rendered in text-only mode")
    else:
        icon = bounds(frame, ICON)
        require(same_box(face, icon) and near(icon["width"], icon_size)
                and near(icon["height"], icon_size), f"Incorrect icon size: {icon}")
    if collapsed:
        require(near(face["width"], height) and near(face["height"], height)
                and find(frame, LABEL)[1] is None, f"Collapsed {size} is not an icon-only square")
        label = None
    else:
        label = bounds(frame, LABEL)
        require(same_box(face, label) and node(frame, LABEL)["text"] == NAME,
                f"The full label must stay within the face: {label}, {face}")
        require(label["width"] > 0. and label["height"] > 0., "Label is empty")
        if icon is not None:
            if end(icon, "y") <= label["y"] + 0.75:
                require(compact, "Wide label and icon do not share the same row")
            elif rtl:
                require(end(label, "x") <= icon["x"] + 0.75, "Icon is on the wrong RTL side")
            else:
                require(end(icon, "x") <= label["x"] + 0.75, "Icon is on the wrong LTR side")
    return {"hit": hit, "face": face, "icon": icon, "label": label, "radius": radius}


def rounded_excess(face, corners, x, y):
    left, right = x + 0.5 - face["x"], end(face, "x") - x - 0.5
    top, bottom = y + 0.5 - face["y"], end(face, "y") - y - 0.5
    limit = min(face["width"], face["height"]) / 2.
    excess = 0.
    for corner, dx, dy in (("top_left", left, top), ("top_right", right, top),
                           ("bottom_left", left, bottom), ("bottom_right", right, bottom)):
        radius = min(corners[corner], limit)
        if dx < radius and dy < radius:
            excess = max(excess, math.hypot(radius - dx, radius - dy) - radius)
    return excess


def capture(app, plan, name, frame, theme, *, palette=False, scaled_label=False):
    path = plan.out / (name + ".png")
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    wire = app.wire("Page.screenshot", SHOT)
    mcp_png, mcp_meta, _ = app.mcp(SHOT)
    encoded = path.read_bytes()
    require(encoded == png_payload(wire) == mcp_png, f"{name}: screenshot bytes differ by transport")
    require(metadata(cli) == metadata(wire) == mcp_meta, f"{name}: metadata differs by transport")
    width, height, rows = read_png(encoded)
    require((width, height) == (cli["width"], cli["height"]), f"{name}: wrong PNG dimensions")

    def pixel(x, y):
        x, y = math.floor(x), math.floor(y)
        require(0 <= x < width and 0 <= y < height, f"{name}: pixel ({x},{y}) is out of range")
        return tuple(rows[y][x * 4:x * 4 + 3])

    canvas = pixel(2, height - 5)
    require(color_near(canvas, BG[theme]), f"{name}: wrong page background {canvas}")
    result = {"file": path.name, "sha256": digest(path), **metadata(cli), "transport_parity": True}
    face = bounds(frame, FACE)
    style = node(frame, FACE)["style"]
    x, y, w, h = (face[field] for field in ("x", "y", "width", "height"))
    if palette:
        expected = style["background"][:3]
        measured = pixel(x + w / 2., y + 4.)
        corner = pixel(x + 2., y + 2.)
        inset = min(style["corners"]["top_left"] / 2. + 1.5, 18.)
        inside = pixel(x + inset, y + inset)
        require(color_near(measured, expected, 5) and color_near(inside, expected, 5)
                and not color_near(corner, expected, 10),
                f"{name}: wrong fill or contour sample {measured}, {inside}, {corner}; wanted {expected}")
        ink = node(frame, LABEL)["style"]["foreground"]
        require(ink == node(frame, ICON)["fill_contour"]["color"], f"{name}: mismatched icon and label colors")
        shadow = [pixel(x + w / 2. + dx, y + h + dy)
                  for dy in (1, 2, 4, 6, 8, 12) for dx in (-20, 0, 20)]
        result["paint"] = {"fill": measured, "corner": corner, "inside": inside,
                           "ink": ink[:3], "shadow": shadow}
    if scaled_label:
        label = bounds(frame, LABEL)
        typography = node(frame, LABEL)["style"]
        font_line = 2 * (typography.get("line_height") or 1.5 * typography["font_size"])
        line_count = round(label["height"] / font_line)
        require(line_count >= 1 and near(label["height"], font_line * line_count),
                f"{name}: label height does not account for full lines: {label}, {font_line}")
        left, right = math.ceil(label["x"]), math.floor(end(label, "x"))
        ink = typography["foreground"][:3]
        line_ink = []
        violations = []
        for line in range(line_count):
            top = math.ceil(label["y"] + font_line * line)
            bottom = math.floor(label["y"] + font_line * (line + 1))
            count = 0
            for row in range(top, bottom):
                for column in range(left, right):
                    color = pixel(column, row)
                    if color_near(color, ink, 18) and not color_near(color, canvas, 4):
                        count += 1
                        delta = rounded_excess(face, style["corners"], column, row)
                        if delta > 1.:
                            violations.append((delta, column, row))
            require(count > 32, f"{name}: line {line + 1} has no visible text")
            line_ink.append(count)
        require(not violations, f"{name}: {len(violations)} ink pixels past the corners: "
                f"{max(violations, default=(0, 0, 0))}")
        result["label_line_height"] = font_line
        result["label_ink_per_line"] = line_ink
        result["label_ink_past_contour"] = 0
    return result


def action_count(frame, count):
    return node(frame, "extended-fab-status")["text"] == f"已新增 {count} 則筆記"


def actions(app, plan):
    act(app, "color-tertiary")
    app.key("tab")
    expect_focus(app, ACTION)
    act(app, "color-tonal-primary")
    app.activate(ACTION)
    require(action_count(expect_focus(app, ACTION), 1), "Semantic activation did not run once")
    app.key("enter")
    require(action_count(expect_focus(app, ACTION), 2), "Enter did not run the same action")
    app.key("space")
    require(action_count(expect_focus(app, ACTION), 3), "Space did not run the same action")
    frame = expect_focus(app, ACTION)
    ring = bounds(frame, ACTION + "-focus-ring")
    face = bounds(frame, FACE)
    require(near(face["x"] - ring["x"], 5.) and near(face["y"] - ring["y"], 5.)
            and node(frame, ACTION + "-focus-ring")["style"]["border_width"] == 3.,
            f"Incorrect outside focus geometry: {ring}, {face}")
    focus_shot = capture(app, plan, "desktop-focus", frame, "light")

    frame = act(app, "disable")
    assert_action(frame, disabled=True)
    app.wire("Action.activate", {"id": ACTION}, expected_error="disabled")
    require(action_count(stable(app), 3), "Disabled intent ran the primary action")
    act(app, "color-tertiary")
    app.key("tab")
    expect_focus(app, "extended-fab-back")
    act(app, "disable")
    act(app, "color-tonal-primary")

    act(app, "hide-on-press")
    app.activate(ACTION)
    frame = expect_focus(app, VISIBILITY)
    require(action_count(frame, 4), "Hide on press ran the action the wrong number of times")
    assert_action(frame, disabled=True)
    require(node(frame, FACE)["style"]["background"][3] == 0 and find(frame, LABEL)[1] is None,
            "Exit-only Extended FAB is not fully hidden under Reduce Motion")
    app.wire("Action.activate", {"id": ACTION}, expected_error="disabled")
    require(action_count(stable(app), 4), "Hidden action could be triggered")
    hidden_shot = capture(app, plan, "desktop-hidden", frame, "light")
    frame = act(app, "toggle-visibility")
    assert_action(frame)
    require(action_count(frame, 4), "Showing the action invoked it")
    return {"activations": 4, "focus_fallback": VISIBILITY,
            "focus_shot": focus_shot, "hide_shot": hidden_shot}


def desktop(app, plan):
    open_demo(app)
    matrix, pngs = [], []
    for theme in ("light", "dark"):
        if theme == "dark":
            act(app, "theme")
        fills = set()
        for palette in PALETTES:
            act(app, "color-" + palette)
            for size in SIZES:
                frame = act(app, "size-" + size)
                expanded = layout_check(frame, size)
                if size == "small":
                    image = capture(app, plan, f"{theme}-{palette}", frame, theme, palette=True)
                    fill = tuple(image["paint"]["fill"])
                    require(fill not in fills, f"{theme} palette {palette} repeats RGB {fill}")
                    fills.add(fill)
                    pngs.append(image)
                collapsed = layout_check(act(app, "toggle-expand"), size, collapsed=True)
                matrix.append({"theme": theme, "palette": palette, "size": size,
                               "expanded": expanded, "collapsed": collapsed})
                act(app, "toggle-expand")
    act(app, "theme")
    act(app, "color-tonal-primary")
    frame = act(app, "size-small")
    normal_layout = layout_check(frame, "small")
    normal_shadow = node(frame, FACE)["box_shadows"]
    frame = act(app, "lowered")
    require(layout_check(frame, "small") == normal_layout,
            "Lowered rest elevation changed the action geometry")
    require(node(frame, FACE)["box_shadows"] != normal_shadow,
            "Lowered elevation did not replace the authored shadow")
    lowered = capture(app, plan, "light-lowered", frame, "light", palette=True)
    dark = pngs[0]["paint"]["shadow"]
    light = lowered["paint"]["shadow"]
    require(any(sum(after) - sum(before) > 4 and all(a >= b for a, b in zip(after, before))
                for before, after in zip(dark, light)),
            f"Lowered rest elevation did not lighten the real shadow: {dark}, {light}")
    act(app, "lowered")
    center = layout_check(act(app, "center"), "small", centered=True)
    act(app, "center")
    journey = actions(app, plan)
    return {"matrix": matrix, "palette_images": pngs, "lowered": lowered,
            "centered_layout": center, "actions": journey}


def compact(app, plan):
    open_demo(app)
    for control in ("theme", "rtl", "size-large"):
        frame = act(app, control)
    original = bounds(frame, LABEL)["height"]
    frame = act(app, "large-text")
    expanded = layout_check(frame, "large", rtl=True, compact=True)
    require(expanded["label"]["height"] >= 1.9 * original,
            "Compact label did not scale to 200 percent")
    expanded_shot = capture(app, plan, "dark-rtl-200-large", frame, "dark", scaled_label=True)
    frame = act(app, "toggle-expand")
    collapsed = layout_check(frame, "large", rtl=True, compact=True, collapsed=True)
    collapsed_shot = capture(app, plan, "dark-rtl-200-collapsed", frame, "dark")
    frame = act(app, "text-only")
    text_only = layout_check(frame, "large", rtl=True, compact=True, text_only=True)
    text_only_shot = capture(app, plan, "dark-rtl-200-text-only", frame, "dark", scaled_label=True)
    return {"label_height_before_scale": original, "expanded": expanded, "collapsed": collapsed,
            "text_only": text_only, "expanded_image": expanded_shot, "collapsed_image": collapsed_shot,
            "text_only_image": text_only_shot}


def main():
    parser = argparse.ArgumentParser()
    for arg in ("gallery", "cli", "out"):
        parser.add_argument("--" + arg, type=Path, required=True)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    plan.out.mkdir(exist_ok=False)
    report = {"source_manifest_sha256": plan.source, "gallery_sha256": digest(plan.gallery),
              "cli_sha256": digest(plan.cli), "headless_renderer": "WGPU native macOS"}
    for name, width, run in (("desktop", 960, desktop), ("compact", 320, compact)):
        with launch(plan, name, width, 800) as app:
            before = onscreen_windows(app.proc.pid)
            require(before == 0, f"{name} owns a visible window")
            data = run(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(after == 0, f"{name} opened a visible window")
            report[name] = {**data, "onscreen_windows": {"before": before, "after": after}}
        print(f"PASS {name} Extended FAB layout, paint, actions and PNG transport", flush=True)
    path = plan.out / "native-extended-fab-report.json"
    path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
