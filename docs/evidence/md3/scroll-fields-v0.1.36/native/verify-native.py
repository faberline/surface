#!/usr/bin/env python3
"""Replay the numeric Scroll Field through the native headless Gallery."""

import argparse
import json
import math
from pathlib import Path

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

DEMO = "scroll-field-demo"
SHOW = f"{DEMO}-show"
BACK = f"{DEMO}-back"
FIELD = f"{DEMO}-value"
ADJUSTABLE = f"{FIELD}-adjustable"
EDITOR = f"{FIELD}-editor"
FACE = f"{FIELD}-face"
CENTER = f"{FIELD}-center"
ERROR = f"{FIELD}-error"
SAVED = f"{DEMO}-saved"
REPORT_NAME = "native-scroll-field-report.json"
IMAGE_NAMES = {
    "scroll-field-light-adjusted.png",
    "scroll-field-light-error.png",
    "scroll-field-light-committed.png",
    "scroll-field-dark-compact.png",
    "scroll-field-dark-fallback.png",
    "scroll-field-dark-committed.png",
}


def scene(app, *settings):
    activate(app, "section-0")
    frame = activate(app, SHOW)
    require(node(frame, DEMO)["role"] == "Window", "Gallery did not open Scroll Field")
    for suffix in settings:
        frame = activate(app, f"{DEMO}-{suffix}")
    return frame


def keyboard_focus(app, target):
    path = [] if focused(settle(app)) == target else focus_by_tab(app, target)
    focus(settle(app), target)
    return path


def saved(frame, value):
    text = node(frame, SAVED)["text"]
    require(text == f"已選 {value} 件", f"Caller-owned saved value disagrees: {text!r}")
    return text


def adjustable(frame, value):
    control = node(frame, ADJUSTABLE)
    expected = {"min": 0, "max": 30, "step": 2, "value": value, "value_text": f"{value} 件"}
    require(control.get("adjustable") == expected,
            f"Adjustable semantics mismatch: {control.get('adjustable')!r}")
    require(control["name"] == "數量", "Adjustable field lost its localized name")
    require(not control.get("disabled", False), "Expected an enabled adjustable field")
    require(find(frame, EDITOR)[0] is None, "Scroll mode retained the editor")
    for stable_id, shown in ((f"{FIELD}-above", value + 2), (CENTER, value),
                             (f"{FIELD}-below", value - 2)):
        require(node(frame, stable_id)["text"] == str(shown),
                f"{stable_id} does not paint the expected value {shown}")
    saved(frame, value)
    return expected


def editor(frame, text, *, invalid=False):
    value = node(frame, EDITOR)
    require(value["role"] == "TextInput" and value["name"] == "數量" and value["text"] == text,
            f"Input field did not retain the expected raw value {text!r}")
    require(value.get("invalid", False) is invalid,
            f"Unexpected input invalid flag: {value.get('invalid')!r}")
    require(find(frame, ADJUSTABLE)[0] is None, "Input mode retained the adjustable control")
    return value


def fill(app, raw):
    response = app.wire("Action.fill", {"id": EDITOR, "value": raw})
    require(response.get("changed"), f"Action.fill did not change the editor to {raw!r}")
    frame = settle(app)
    focus(frame, EDITOR)
    editor(frame, raw)
    return frame


def back(app):
    path = keyboard_focus(app, BACK)
    frame = key(app, "enter")
    focus(frame, SHOW)
    require(find(frame, DEMO)[0] is None, "Back left the Scroll Field route active")
    node(frame, SHOW)
    return {"tab_path": path, "restored": SHOW}


def close_to(actual, expected, name, tolerance=0.51):
    require(abs(actual - expected) <= tolerance, f"{name}: got {actual}, expected {expected}")


def scroll_geometry(frame, *, compact):
    face = visible(frame, FACE)
    capture_bounds = visible(frame, ADJUSTABLE, 48.)
    above = visible(frame, f"{FIELD}-above-target", 48.)
    below = visible(frame, f"{FIELD}-below-target", 48.)
    close_to(above["height"], 48., "above target height")
    close_to(below["height"], 48., "below target height")
    require(same_box(capture_bounds, above) and same_box(capture_bounds, below),
            "Alternate targets exceed the adjustable capture owner")
    require(above["y"] < face["y"] and
            below["y"] + below["height"] > face["y"] + face["height"],
            "Alternate targets failed to extend beyond the painted face")
    require(face["width"] >= 120., "Medium field fell below the 120dp floor")
    close_to(face["height"], 224. if compact else 200., "fitted face height")
    center = visible(frame, CENTER)
    close_to(center["y"] + center["height"] / 2., face["y"] + face["height"] / 2.,
             "center glyph line stays vertically centered")
    ring = visible(frame, f"{FIELD}-focus")
    close_to(face["x"] - ring["x"], 5., "outer focus ring extent")
    return {"face": face, "capture": capture_bounds, "above_target": above,
            "below_target": below, "focus_ring": ring}


def luminance(color):
    linear = [(x / 255. / 12.92 if x / 255. <= 0.04045 else
               ((x / 255. + 0.055) / 1.055) ** 2.4) for x in color[:3]]
    return sum(x * weight for x, weight in zip(linear, (0.2126, 0.7152, 0.0722)))


def contrast(foreground, background):
    dark, light = sorted((luminance(foreground), luminance(background)))
    return (light + 0.05) / (dark + 0.05)


def pixel(rows, x, y):
    return tuple(rows[y][4 * x:4 * x + 4])


def pixels_for(frame, rows, target, *, rgb_tolerance=0):
    measured = layout(frame, target)
    bounds, clip = measured["bounds"], measured["clip"]
    color = tuple(node(frame, target)["style"]["foreground"])
    xmin = max(0, math.ceil(max(bounds["x"], clip["x"])))
    xmax = min(len(rows[0]) // 4, math.floor(min(bounds["x"] + bounds["width"], clip["x"] + clip["width"])))
    ymin = max(0, math.ceil(max(bounds["y"], clip["y"])))
    ymax = min(len(rows), math.floor(min(bounds["y"] + bounds["height"], clip["y"] + clip["height"])))
    def matches(sample):
        return sample[3] == color[3] and max(abs(x - y) for x, y in zip(sample[:3], color[:3])) <= rgb_tolerance

    points = [(x, y) for y in range(ymin, ymax) for x in range(xmin, xmax)
              if matches(pixel(rows, x, y))]
    require(len(points) >= 16, f"No readable {color!r} paint in {target}")
    xs, ys = zip(*points)
    ink = [min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1]
    require(ink[2] >= 3 and ink[3] >= 4, f"Too little visible ink in {target}: {ink}")
    return {"text": node(frame, target).get("text"), "ink_box": ink,
            "ink_pixels": len(points), "rgb_tolerance": rgb_tolerance, "rgba": list(color)}


def shot(app, plan, title, frame, *, editing=False, error=False):
    before = onscreen_windows(app.proc.pid)
    receipt = capture(app, plan, title, frame=frame)
    after = onscreen_windows(app.proc.pid)
    require(before == after == 0, f"{title} opened a visible window")
    viewport = frame["layout"]["viewport"]
    require((receipt["width"], receipt["height"]) == (viewport["width"], viewport["height"]),
            "PNG dimensions differ from the native viewport")
    _, _, rows = read_png((plan.out / receipt["name"]).read_bytes())
    face = visible(frame, FACE)
    face_node = node(frame, FACE)
    middle_y = math.floor(face["y"] + face["height"] / 2.)
    inner = pixel(rows, math.floor(face["x"] + 10.), middle_y)
    base = tuple(face_node["style"]["background"])
    alpha = 0 if editing else face_node["state_layer_alphas"][1]
    tint = face_node["style"]["state_layer"]
    expected = tuple(round((background * (255 - alpha) + foreground * alpha) / 255.)
                     for background, foreground in zip(base[:3], tint[:3])) + (255,)
    require(max(abs(a - b) for a, b in zip(inner, expected)) <= 1,
            f"Actual face color {inner!r} disagrees with its focused token {expected!r}")
    ink = pixels_for(frame, rows, EDITOR if editing else CENTER)
    ratio = contrast(ink["rgba"], inner)
    require(ratio >= 4.5, f"Active text contrast is only {ratio:.3f}:1")
    receipt["visible_windows"] = [before, after]
    receipt["paint"] = {"face_rgba": list(inner), "base_face_rgba": list(base),
                        "focus_layer_alpha": alpha, "active": ink,
                        "text_contrast": round(ratio, 4)}
    focus_color = tuple(node(frame, f"{FIELD}-focus")["focus_color"])
    focus_pixel = pixel(rows, math.floor(face["x"] - 4.), middle_y)
    gap = pixel(rows, math.floor(face["x"] - 1.), middle_y)
    require(focus_pixel == focus_color, f"Outer focus ring did not paint: {focus_pixel!r}")
    require(gap != focus_color and contrast(focus_color, gap) >= 3.,
            "Focus ring lost its outer gap or contrast")
    receipt["paint"]["focus_stroke_rgba"] = list(focus_pixel)
    receipt["paint"]["focus_gap_rgba"] = list(gap)
    if editing:
        stroke = tuple(node(frame, FACE)["style"]["border_color"])
        require(stroke == tuple(ink["rgba"]), "Editor text and outline use different inks")
        edge = pixel(rows, math.floor(face["x"] + 1.), middle_y)
        require(edge == stroke, f"Expected editor outline pixels, got {edge!r}")
        receipt["paint"]["editor_stroke_rgba"] = list(edge)
    else:
        for part in ("above", "below"):
            neighbor = pixels_for(frame, rows, f"{FIELD}-{part}")
            require(contrast(neighbor["rgba"], inner) >= 3., f"{part} loses contrast")
            receipt["paint"][part] = neighbor
    if error:
        visible(frame, ERROR)
        # The 12 dp CJK caption uses antialiased strokes, even at its darkest pixels.
        receipt["paint"]["error"] = pixels_for(frame, rows, ERROR, rgb_tolerance=64)
    print(f"PASS {title}: same PNG through API, CLI and MCP, text contrast {ratio:.2f}:1", flush=True)
    return receipt


def light(app, plan):
    frame = scene(app)
    require(luminance(node(frame, DEMO)["style"]["background"]) > 0.5, "Light profile opened dark")
    initial = adjustable(frame, 14)
    path = keyboard_focus(app, ADJUSTABLE)
    frame = key(app, "up")
    focus(frame, ADJUSTABLE)
    incremented = adjustable(frame, 16)
    geometry = scroll_geometry(frame, compact=False)
    adjusted = shot(app, plan, "scroll-field-light-adjusted", frame)

    frame = key(app, "enter")
    focus(frame, EDITOR)
    editor(frame, "16")
    filled = fill(app, "17")
    saved(filled, 16)
    frame = key(app, "enter")
    focus(frame, EDITOR)
    field = editor(frame, "17", invalid=True)
    error = node(frame, ERROR)["text"]
    require(error == "請輸入偶數" and field.get("described_by_text") == ERROR,
            f"Off-step value did not expose a localized, associated error: {error!r}")
    saved(frame, 16)
    invalid = shot(app, plan, "scroll-field-light-error", frame, editing=True, error=True)

    filled = fill(app, "２０")
    saved(filled, 16)
    frame = key(app, "enter")
    focus(frame, ADJUSTABLE)
    committed = adjustable(frame, 20)
    require(find(frame, ERROR)[0] is None, "Accepted replacement retained its previous error")
    normalized = shot(app, plan, "scroll-field-light-committed", frame)
    return {"initial": initial, "incremented": incremented, "committed": committed,
            "focus_path": path, "error": error, "invalid_editor": True, "geometry": geometry,
            "back": back(app), "captures": [adjusted, invalid, normalized]}


def compact(app, plan):
    frame = scene(app, "theme", "rtl", "large-text", "compact")
    require(luminance(node(frame, DEMO)["style"]["background"]) < 0.2, "Compact profile opened light")
    for setting in ("rtl", "large-text", "compact"):
        require(node(frame, f"{DEMO}-{setting}").get("selected") is True,
                f"Compact profile did not enable {setting}")
    path = keyboard_focus(app, ADJUSTABLE)
    frame = settle(app)
    compact_value = adjustable(frame, 14)
    geometry = scroll_geometry(frame, compact=True)
    fitted = shot(app, plan, "scroll-field-dark-compact", frame)

    frame = activate(app, f"{DEMO}-wide-range")
    editor(frame, "1200")
    saved(frame, 1200)
    editor_path = keyboard_focus(app, EDITOR)
    frame = settle(app)
    initial = shot(app, plan, "scroll-field-dark-fallback", frame, editing=True)
    filled = fill(app, "１３００")
    saved(filled, 1200)
    frame = key(app, "enter")
    focus(frame, EDITOR)
    editor(frame, "1300")
    committed = saved(frame, 1300)
    require(find(frame, ERROR)[0] is None, "Range fallback showed a false error")
    normalized = shot(app, plan, "scroll-field-dark-committed", frame, editing=True)
    return {"rtl": True, "text_scale": 2, "compact": compact_value, "geometry": geometry,
            "focus_path": path, "editor_focus_path": editor_path, "fallback_initial": 1200,
            "fallback_committed": 1300, "caller_saved": committed, "back": back(app),
            "captures": [fitted, initial, normalized]}


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
        "motion": "native headless Reduce Motion",
        "input": "native headless DevTools Action.key, Action.activate and Action.fill",
        "transports": ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"],
        "profiles": {},
    }
    for replay, width, height in ((light, 960, 760), (compact, 320, 900)):
        name = replay.__name__
        with launch(plan, name, width, height) as app:
            before = onscreen_windows(app.proc.pid)
            require(before == 0, f"{name} created a visible window")
            proof = replay(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(after == 0, f"{name} created a visible window")
            proof["visible_windows"] = [before, after]
            proof["viewport"] = [width, height]
        require(app.proc.poll() is not None, f"{name} did not terminate the headless Gallery")
        report["profiles"][name] = proof
        print(f"PASS {name} {width}x{height}. Zero visible windows.", flush=True)
    require({p.name for p in plan.out.iterdir()} == IMAGE_NAMES, "Native PNG set differs")
    (plan.out / REPORT_NAME).write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
