#!/usr/bin/env python3
"""Exercise the Expressive FAB in the Surface headless Metal Gallery."""

import argparse
import json
import math
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box
from window_check import onscreen_windows

ACTION = "fab-demo-action"
FACE, ICON, RING = (ACTION + suffix for suffix in ("-face", "-icon", "-focus-ring"))
FALLBACK = "fab-demo-toggle-visibility"
TILT = "fab-demo-tooltip"
LABEL = "新增專案筆記"
SIZES = {"regular": (56, 16, 24), "medium": (80, 20, 28), "large": (96, 28, 36)}
PALETTES = ("tonal-primary", "tonal-secondary", "tonal-tertiary", "primary", "secondary", "tertiary")
SHOT = {"scale": 1, "max_side": 1200}


def find(frame, key):
    for idx, node in enumerate(frame["dom"]["nodes"]):
        if node.get("stable_id") == key:
            return idx, node
    return None, None


def node(frame, key):
    out = find(frame, key)[1]
    require(out is not None, f"Missing {key}")
    return out


def bounds(frame, key):
    idx = find(frame, key)[0]
    require(idx is not None, f"Missing layout for {key}")
    return frame["layout"]["nodes"][idx]["bounds"]


def settled(app):
    before = app.dom()
    for _ in range(12):
        after = app.dom()
        if before == after:
            return after
        before = after
    raise RuntimeError("Gallery still updates after twelve frames")


def close(actual, expected, eps=0.6):
    return abs(actual - expected) <= eps


def assert_focus(app, key):
    frame = settled(app)
    require(focused(frame) == key, f"Expected {key} focus, got {focused(frame)}")
    return frame


def assert_layout(frame, size, rtl):
    face = bounds(frame, FACE)
    icon = bounds(frame, ICON)
    hit = bounds(frame, ACTION)
    outer = frame["layout"]["viewport"]
    side, radius, glyph = SIZES[size]
    require(same_box(outer, hit) and same_box(hit, face), f"Clipped target {hit}")
    require(close(face["width"], side) and close(face["height"], side), f"Wrong {size} size {face}")
    require(close(icon["width"], glyph) and close(icon["height"], glyph), f"Wrong glyph size {icon}")
    for start, length in [("x", "width"), ("y", "height")]:
        require(close(icon[start] + icon[length] / 2, face[start] + face[length] / 2),
                f"Icon is not centered on {start}")
    x = face["x"] if rtl else outer["width"] - (face["x"] + side)
    require(close(x, 16.) and close(outer["height"] - (face["y"] + side), 16.),
            f"Lost trailing or bottom anchor: {face}")
    owner = node(frame, ACTION)
    require(owner["role"] == "Button" and owner["name"] == LABEL and not owner["disabled"],
            "Action lacks its stable name")
    authored = node(frame, FACE)["style"]
    corners = authored.get("corners")
    if corners:
        require(all(close(value, radius) for value in corners.values()), f"Wrong corner radii {corners}")
    else:
        require(close(authored.get("radius", 0), radius), f"Wrong radius {authored}")
    return {"face": face, "icon": icon, "hit": hit}


def capture(app, plan, name, frame, color_probe=False):
    path = plan.out / (name + ".png")
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    raw = app.wire("Page.screenshot", SHOT)
    mcp, meta, _ = app.mcp(SHOT)
    encoded = path.read_bytes()
    require(encoded == png_payload(raw) == mcp, f"{name}: raw, CLI and MCP bytes differ")
    require(metadata(cli) == metadata(raw) == meta, f"{name}: transport metadata differs")
    width, height, rows = read_png(encoded)
    require((width, height) == (cli["width"], cli["height"]), f"Invalid PNG dimensions {name}")
    result = {"file": path.name, "sha256": digest(path), **metadata(cli), "transport_parity": True}
    if color_probe:
        face = bounds(frame, FACE)
        icon = bounds(frame, ICON)
        def rgb(x, y):
            x, y = math.floor(x), math.floor(y)
            require(0 <= x < width and 0 <= y < height, f"Probe out of {name} at {x}, {y}")
            return tuple(rows[y][x * 4:x * 4 + 3])
        expect_fill = tuple(node(frame, FACE)["style"]["background"][:3])
        expect_ink = tuple(node(frame, ICON)["fill_contour"]["color"][:3])
        center_x = face["x"] + face["width"] / 2
        center_y = face["y"] + face["height"] / 2
        face_rgb = rgb(face["x"] + 5, center_y)
        glyph_rgb = rgb(icon["x"] + icon["width"] / 2, icon["y"] + icon["height"] / 2)
        require(max(abs(a-b) for a,b in zip(face_rgb,expect_fill)) <= 3,
                f"{name}: wrong face color {face_rgb} expected {expect_fill}")
        require(max(abs(a-b) for a,b in zip(glyph_rgb,expect_ink)) <= 3,
                f"{name}: wrong glyph color {glyph_rgb} expected {expect_ink}")
        result.update(face_rgb=face_rgb, glyph_rgb=glyph_rgb,
                      shadow_rgb=rgb(center_x, face["y"] + face["height"] + 5))
    return result


def tooltip(app):
    frame = assert_focus(app, ACTION)
    tip = node(frame, TILT)
    require(tip["role"] == "Tooltip" and tip["name"] == LABEL, "Tooltip does not show the action label")
    require(same_box(frame["layout"]["viewport"], bounds(frame, TILT)), "Tooltip exceeds viewport")
    tip_box, ring = bounds(frame, TILT), bounds(frame, RING)
    require(same_box(frame["layout"]["viewport"], ring), "Focus ring exceeds viewport")
    require(tip_box["y"] + tip_box["height"] <= ring["y"], "Tooltip covers the focus ring")
    return frame


def open_scene(app):
    app.activate("section-1")
    app.activate("fab-demo-show")
    return assert_focus(app, "fab-demo-back")


def desktop(app, plan):
    open_scene(app)
    profiles, images = [], []
    for theme in ("light", "dark"):
        if theme == "dark":
            app.activate("fab-demo-theme")
        for palette in PALETTES:
            app.activate("fab-demo-color-" + palette)
            for size in SIZES:
                app.activate("fab-demo-size-" + size)
                frame = settled(app)
                profiles.append(dict(theme=theme, palette=palette, size=size,
                                     **assert_layout(frame, size, False)))
                if size == "medium":
                    images.append(capture(app, plan, f"{theme}-{palette}", frame, True))
    app.activate("fab-demo-theme")
    app.activate("fab-demo-color-tonal-primary")
    app.activate("fab-demo-size-medium")
    regular = assert_layout(settled(app), "medium", False)
    standard = images[0]["shadow_rgb"]
    app.activate("fab-demo-lowered")
    lowered_frame = settled(app)
    require(assert_layout(lowered_frame, "medium", False) == regular,
            "Lowering elevation changed the hit or ink geometry")
    lowered = capture(app, plan, "light-lowered", lowered_frame, True)
    require(sum(lowered["shadow_rgb"]) > sum(standard) + 5,
            f"Lowered shadow did not brighten: {standard} -> {lowered['shadow_rgb']}")
    app.activate("fab-demo-lowered")
    app.activate("fab-demo-size-regular")
    report = {"tab_steps": focus_by_tab(app, ACTION)}
    report["tooltip_capture"] = capture(app, plan, "light-keyboard-tooltip", tooltip(app))
    app.key("escape")
    require(find(assert_focus(app, ACTION), TILT)[1] is None, "Escape retains unwanted tooltip")
    app.key("enter")
    app.key("space")
    app.activate(ACTION)
    require(node(settled(app), "fab-demo-status")["text"] == "已新增 3 則筆記",
            "Keyboard and semantic action counts disagree")
    app.activate("fab-demo-disable")
    frame = settled(app)
    require(node(frame, ACTION)["disabled"], "Action did not disable")
    app.wire("Action.activate", {"id": ACTION}, expected_error="disabled")
    require(node(settled(app), "fab-demo-status")["text"] == "已新增 3 則筆記",
            "Disabled action changed the count")
    app.activate("fab-demo-disable")
    app.activate("fab-demo-hide-on-press")
    app.activate(ACTION)
    hidden = assert_focus(app, FALLBACK)
    require(node(hidden, "fab-demo-status")["text"] == "已新增 4 則筆記",
            "First action that hides did not execute once")
    report["hidden_capture"] = capture(app, plan, "light-hidden", hidden)
    report.update(activations=4, focus_after_hide=FALLBACK)
    return {"profiles": profiles, "palette_images": images, "lowered": lowered, "keyboard": report}


def compact(app, plan):
    open_scene(app)
    app.activate("fab-demo-theme")
    app.activate("fab-demo-rtl")
    app.activate("fab-demo-size-large")
    app.activate("fab-demo-large-text")
    frame = settled(app)
    layout = assert_layout(frame, "large", True)
    rest = capture(app, plan, "dark-rtl-200-rest", frame, True)
    steps = focus_by_tab(app, ACTION)
    frame = tooltip(app)
    focused_image = capture(app, plan, "dark-rtl-200-tooltip", frame)
    app.key("escape")
    require(find(assert_focus(app, ACTION), TILT)[1] is None, "Compact tooltip did not dismiss")
    return {"layout": layout, "tab_steps": steps, "tooltip": bounds(frame, TILT),
            "rest": rest, "focused": focused_image}


def main():
    parser = argparse.ArgumentParser()
    for arg in ("gallery", "cli", "out"):
        parser.add_argument("--" + arg, type=Path, required=True)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    plan.out.mkdir(exist_ok=False)
    result = {"source_manifest_sha256": plan.source, "gallery_sha256": digest(plan.gallery),
              "cli_sha256": digest(plan.cli), "headless_renderer": "WGPU native macOS"}
    for name, width, run in (("desktop", 960, desktop), ("compact", 320, compact)):
        with launch(plan, name, width, 800) as app:
            before = onscreen_windows(app.proc.pid)
            require(before == 0, f"{name} Gallery owns a visible window")
            data = run(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(after == 0, f"{name} Gallery opened a window")
            result[name] = {**data, "onscreen_windows": {"before": before, "after": after}}
        print(f"PASS {name} geometry, focus and screenshot transport", flush=True)
    (plan.out / "native-expressive-fab-report.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
