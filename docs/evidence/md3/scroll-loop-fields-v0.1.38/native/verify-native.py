#!/usr/bin/env python3
"""Replay both looping Scroll Fields through the native offscreen Gallery."""

import argparse
from dataclasses import dataclass
import json
import math
from pathlib import Path

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows


@dataclass(frozen=True)
class Fixture:
    page: str
    launcher: str
    title: str
    label: str

    def control(self, part):
        return f"{self.page}-{part}"

    def field(self, part):
        return f"{self.page}-value-{part}"


NUMERIC = Fixture("scroll-field-demo", "scroll-loop-numeric-show", "循環數字欄位", "數量")
CHOICE = Fixture("scroll-choice-demo", "scroll-loop-choice-show", "循環文字欄位", "地點")
REPORT_NAME = "native-scroll-loop-report.json"
IMAGE_NAMES = {
    "scroll-loop-numeric-light-minimum.png",
    "scroll-loop-numeric-light-maximum.png",
    "scroll-loop-choice-dark-station.png",
    "scroll-loop-choice-dark-reordered.png",
    "scroll-loop-choice-dark-compact.png",
    "scroll-loop-choice-dark-compact-station.png",
}


def scene(app, fixture, *settings):
    activate(app, "section-0")
    frame = activate(app, fixture.launcher)
    window = node(frame, fixture.page)
    require(window["role"] == "Window" and window["name"] == fixture.title,
            f"Gallery did not open {fixture.title}")
    for part in settings:
        frame = activate(app, fixture.control(part))
    for part in set(settings) - {"theme"}:
        require(node(frame, fixture.control(part)).get("selected") is True,
                f"{fixture.title} did not enable {part}")
    return frame


def keyboard_focus(app, target):
    path = [] if focused(settle(app)) == target else focus_by_tab(app, target)
    focus(settle(app), target)
    return path


def back(app, fixture):
    path = keyboard_focus(app, fixture.control("back"))
    frame = key(app, "enter")
    focus(frame, fixture.launcher)
    require(find(frame, fixture.page)[0] is None, f"Back left {fixture.title} active")
    require(node(frame, fixture.launcher)["name"] == fixture.title,
            "Back did not restore the corresponding looping launcher")
    return {"tab_path": path, "restored": fixture.launcher}


def saved(frame, fixture, value_text):
    descriptor = node(frame, fixture.control("saved"))["text"]
    require(descriptor == f"已選 {value_text}", f"Displayed selection disagrees: {descriptor!r}")
    return descriptor


def adjustable(frame, fixture, value, value_text, *, minimum, maximum, step, center, above, below):
    control = node(frame, fixture.field("adjustable"))
    semantics = {"min": minimum, "max": maximum, "step": step, "value": value,
                 "value_text": value_text}
    require(control.get("adjustable") == semantics and control["name"] == fixture.label,
            f"{fixture.title} semantics mismatch: {control.get('adjustable')!r}")
    require(not control.get("disabled", False), "The adjustable field is disabled")
    require(find(frame, fixture.field("editor"))[0] is None, "Scroll mode retained the editor")
    for part, expected in (("center", center), ("above", above), ("below", below)):
        text = node(frame, fixture.field(part))["text"]
        require(text == expected, f"{fixture.title} painted {part} {text!r}, expected {expected!r}")
    return {"name": fixture.label, **semantics,
            "painted": {"center": center, "above": above, "below": below},
            "saved": saved(frame, fixture, value_text)}


def editor(frame, fixture, text):
    control = node(frame, fixture.field("editor"))
    require(control["role"] == "TextInput" and control["name"] == fixture.label
            and control["text"] == text, f"Expected {fixture.label} editor with {text!r}")
    require(not control.get("invalid", False), "Valid editor unexpectedly shows an error")
    require(find(frame, fixture.field("adjustable"))[0] is None, "Input retained the adjustable")


def fill(app, fixture, text):
    reply = app.wire("Action.fill", {"id": fixture.field("editor"), "value": text})
    require(reply.get("changed"), f"Action.fill did not change the editor to {text!r}")
    frame = settle(app)
    focus(frame, fixture.field("editor"))
    editor(frame, fixture, text)
    return frame


def middle(rect):
    return rect["y"] + rect["height"] / 2.


def bottom(rect):
    return rect["y"] + rect["height"]


def close_to(actual, expected, title, tolerance=0.51):
    require(abs(actual - expected) <= tolerance, f"{title}: got {actual}, expected {expected}")


def scroll_geometry(frame, fixture):
    face = visible(frame, fixture.field("face"), 48.)
    owner = visible(frame, fixture.field("adjustable"), 48.)
    center = visible(frame, fixture.field("center"))
    above = visible(frame, fixture.field("above-target"), 48.)
    below = visible(frame, fixture.field("below-target"), 48.)
    require(same_box(owner, above) and same_box(owner, below) and bottom(above) < below["y"],
            "Neighbors must have separate targets contained by the owner")
    close_to(above["height"], 48., "above target height")
    close_to(below["height"], 48., "below target height")
    require(above["y"] < face["y"] and bottom(below) > bottom(face),
            "Neighbor targets must extend beyond the cropped face")
    close_to(face["height"], 200., "regular painted height")
    close_to(middle(center), middle(face), "center alignment")
    laid_out_above = layout(frame, fixture.field("above"))["bounds"]
    laid_out_below = layout(frame, fixture.field("below"))["bounds"]
    require(middle(laid_out_above) < middle(center) < middle(laid_out_below),
            "Two neighbor labels do not have separate positions")
    ring = visible(frame, fixture.field("focus"))
    close_to(face["x"] - ring["x"], 5., "outer focus ring extent")
    return {"face": face, "owner": owner, "center": center, "above_target": above,
            "below_target": below, "above_label": laid_out_above, "below_label": laid_out_below,
            "focus_ring": ring}


def luminance(color):
    linear = [(x / 255. / 12.92 if x / 255. <= 0.04045 else
               ((x / 255. + 0.055) / 1.055) ** 2.4) for x in color[:3]]
    return sum(x * weight for x, weight in zip(linear, (0.2126, 0.7152, 0.0722)))


def contrast(foreground, background):
    dark, light = sorted((luminance(foreground), luminance(background)))
    return (light + 0.05) / (dark + 0.05)


def pixel(rows, x, y):
    return tuple(rows[y][4 * x:4 * x + 4])


def ink_sample(frame, rows, target, *, tolerance):
    computed = layout(frame, target)
    rect, clip = computed["bounds"], computed["clip"]
    target_node = node(frame, target)
    color = tuple(target_node["style"]["foreground"])
    x0 = max(0, math.ceil(max(rect["x"], clip["x"])))
    x1 = min(len(rows[0]) // 4, math.floor(min(rect["x"] + rect["width"], clip["x"] + clip["width"])))
    y0 = max(0, math.ceil(max(rect["y"], clip["y"])))
    y1 = min(len(rows), math.floor(min(bottom(rect), bottom(clip))))

    def matches(sample):
        return sample[3] == color[3] and max(abs(a - b) for a, b in zip(sample[:3], color[:3])) <= tolerance

    ink = [(x, y) for y in range(y0, y1) for x in range(x0, x1) if matches(pixel(rows, x, y))]
    require(len(ink) >= 16, f"No readable {color!r} paint in {target}")
    xs, ys = zip(*ink)
    span = [min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1]
    require(span[2] >= 3 and span[3] >= 4, f"{target} has too little readable ink: {span}")
    return {"text": target_node["text"], "ink_box": span, "ink_pixels": len(ink),
            "rgb_tolerance": tolerance, "rgba": list(color)}


def shot(app, plan, fixture, title, frame, *, editing=False):
    before = onscreen_windows(app.proc.pid)
    receipt = capture(app, plan, title, frame=frame)
    after = onscreen_windows(app.proc.pid)
    require(before == after == 0, f"{title} opened a visible window")
    viewport = frame["layout"]["viewport"]
    require((receipt["width"], receipt["height"]) == (viewport["width"], viewport["height"]),
            "PNG dimensions differ from the native viewport")

    _, _, rows = read_png((plan.out / receipt["name"]).read_bytes())
    face = visible(frame, fixture.field("face"))
    face_node = node(frame, fixture.field("face"))
    y = math.floor(middle(face))
    inner = pixel(rows, math.floor(face["x"] + 10.), y)
    base = tuple(face_node["style"]["background"])
    alpha = 0 if editing else face_node["state_layer_alphas"][1]
    tint = face_node["style"]["state_layer"]
    expected = tuple(round((bg * (255 - alpha) + fg * alpha) / 255.)
                     for bg, fg in zip(base[:3], tint[:3])) + (255,)
    require(max(abs(a - b) for a, b in zip(inner, expected)) <= 1,
            f"Face pixels {inner!r} disagree with the focused token {expected!r}")

    # Drawn CJK strokes use antialiasing, including in the heart of a glyph.
    active = ink_sample(frame, rows, fixture.field("editor" if editing else "center"), tolerance=64)
    glyphs = sum(not ch.isspace() for ch in active["text"])
    require(active["ink_box"][2] >= 8 * glyphs and active["ink_box"][3] >= 16,
            f"Too little active ink for {active['text']!r}")
    ratio = contrast(active["rgba"], inner)
    require(ratio >= 4.5, f"Active text contrast is only {ratio:.3f}:1")

    focus_color = tuple(node(frame, fixture.field("focus"))["focus_color"])
    outside_stroke = pixel(rows, math.floor(face["x"] - 4.), y)
    gap = pixel(rows, math.floor(face["x"] - 1.), y)
    require(outside_stroke == focus_color and gap != focus_color and contrast(focus_color, gap) >= 3.,
            "The focused field lost its outer ring, contrast or gap")
    paint = {"face_rgba": list(inner), "base_face_rgba": list(base), "active": active,
             "focus_layer_alpha": alpha, "text_contrast": round(ratio, 4),
             "focus_stroke_rgba": list(outside_stroke), "focus_gap_rgba": list(gap)}
    if editing:
        outline = pixel(rows, math.floor(face["x"] + 1.), y)
        require(outline == tuple(face_node["style"]["border_color"]) == tuple(active["rgba"]),
                "Editor outline and glyph inks disagree")
        paint["editor_stroke_rgba"] = list(outline)
    else:
        for part in ("above", "below"):
            neighbor = ink_sample(frame, rows, fixture.field(part), tolerance=48)
            require(contrast(neighbor["rgba"], inner) >= 3., f"{part} loses neighbor contrast")
            paint[part] = neighbor
        require(paint["above"]["ink_box"][1] + paint["above"]["ink_box"][3]
                < paint["below"]["ink_box"][1], "The two painted neighbor regions overlap")
    receipt.update({"visible_windows": [before, after], "paint": paint})
    print(f"PASS {title}: same PNG through API, CLI and MCP, text contrast {ratio:.2f}:1", flush=True)
    return receipt


def numeric(app, plan):
    fixture = NUMERIC
    frame = scene(app, fixture)
    require(luminance(node(frame, fixture.page)["style"]["background"]) > 0.5,
            "Numeric light profile opened dark")

    def verify(current, value, above, below):
        return adjustable(current, fixture, value, f"{value} 件", minimum=2, maximum=10, step=2,
                          center=str(value), above=str(above), below=str(below))

    initial = verify(frame, 10, 2, 8)
    target = fixture.field("adjustable")
    path = keyboard_focus(app, target)
    frame = key(app, "up")
    focus(frame, target)
    forward = verify(frame, 2, 4, 10)
    minimum_geometry = scroll_geometry(frame, fixture)
    low = shot(app, plan, fixture, "scroll-loop-numeric-light-minimum", frame)
    frame = key(app, "down")
    focus(frame, target)
    reverse = verify(frame, 10, 2, 8)
    maximum_geometry = scroll_geometry(frame, fixture)
    high = shot(app, plan, fixture, "scroll-loop-numeric-light-maximum", frame)
    return {"initial": initial, "up_wrap": forward, "down_wrap": reverse,
            "minimum_geometry": minimum_geometry, "maximum_geometry": maximum_geometry,
            "focus_path": path, "back": back(app, fixture), "captures": [low, high]}


def choice(app, plan):
    fixture = CHOICE
    frame = scene(app, fixture, "theme", "rtl")
    require(luminance(node(frame, fixture.page)["style"]["background"]) < 0.2,
            "Choice profile opened light")

    def verify(current, index, selected, adjacent):
        return adjustable(current, fixture, index, selected, minimum=1, maximum=2, step=1,
                          center=selected, above=adjacent, below=adjacent)

    initial = verify(frame, 1, "板橋", "台北 車站")
    target = fixture.field("adjustable")
    path = keyboard_focus(app, target)
    frame = key(app, "down")
    focus(frame, target)
    reverse = verify(frame, 2, "台北 車站", "板橋")
    station_geometry = scroll_geometry(frame, fixture)
    station = shot(app, plan, fixture, "scroll-loop-choice-dark-station", frame)
    frame = key(app, "up")
    focus(frame, target)
    forward = verify(frame, 1, "板橋", "台北 車站")
    frame = activate(app, fixture.control("reorder"))
    require(node(frame, fixture.control("reorder")).get("selected") is True, "Reorder stayed off")
    reorder_focus = keyboard_focus(app, target)
    frame = settle(app)
    reordered = verify(frame, 2, "板橋", "台北 車站")
    reordered_geometry = scroll_geometry(frame, fixture)
    reversal = shot(app, plan, fixture, "scroll-loop-choice-dark-reordered", frame)
    return {"rtl": True, "initial": initial, "down_wrap": reverse, "up_wrap": forward,
            "reordered": reordered, "preserved_authored_id": "banqiao", "focus_path": path,
            "reorder_focus_path": reorder_focus, "station_geometry": station_geometry,
            "reordered_geometry": reordered_geometry, "back": back(app, fixture),
            "captures": [station, reversal]}


def compact(app, plan):
    fixture = CHOICE
    frame = scene(app, fixture, "theme", "rtl", "large-text", "compact")
    require(luminance(node(frame, fixture.page)["style"]["background"]) < 0.2,
            "Compact choice profile opened light")
    target = fixture.field("editor")
    editor(frame, fixture, "板橋")
    saved(frame, fixture, "板橋")
    initial_focus = keyboard_focus(app, target)
    frame = settle(app)
    geometry = {"face": visible(frame, fixture.field("face"), 48.), "editor": visible(frame, target, 48.)}
    initial = shot(app, plan, fixture, "scroll-loop-choice-dark-compact", frame, editing=True)

    draft = fill(app, fixture, "台北車站")
    saved(draft, fixture, "板橋")
    frame = key(app, "enter")
    focus(frame, target)
    editor(frame, fixture, "台北")
    committed = saved(frame, fixture, "台北 車站")
    require(find(frame, fixture.field("error"))[0] is None, "A valid alias left an error")
    selected = shot(app, plan, fixture, "scroll-loop-choice-dark-compact-station", frame,
                    editing=True)
    return {"rtl": True, "text_scale": 2, "fallback_initial": "板橋", "submitted_alias": "台北車站",
            "normalized_editor": "台北", "saved_after_enter": committed,
            "authored_selected_id": "taipei-station", "retained_focus": target,
            "geometry": geometry, "focus_path": initial_focus, "back": back(app, fixture),
            "captures": [initial, selected]}


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
        "motion": "native headless Reduce Motion canonical endpoints",
        "input": "headless DevTools Action.key, Action.activate and Action.fill",
        "transports": ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"],
        "profiles": {},
    }
    for replay, width, height in ((numeric, 960, 820), (choice, 960, 820), (compact, 320, 820)):
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
    require({path.name for path in plan.out.iterdir()} == IMAGE_NAMES, "Native PNG set differs")
    (plan.out / REPORT_NAME).write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
