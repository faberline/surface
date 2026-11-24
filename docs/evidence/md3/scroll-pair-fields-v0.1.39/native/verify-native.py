#!/usr/bin/env python3
"""Replay paired quantity and unit Scroll Fields through the offscreen Gallery."""

import argparse
from dataclasses import dataclass
import json
import math
from pathlib import Path

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, focused, launch
from render_helpers import activate, capture, find, focus, layout, node, key, settle, visible
from window_check import onscreen_windows

PAGE = "scroll-pair-demo"
SHOW = f"{PAGE}-show"
BACK = f"{PAGE}-back"
PAIR = f"{PAGE}-pair"
REPORT_NAME = "native-scroll-pair-report.json"
IMAGE_NAMES = {
    "scroll-pair-light-adjusted.png",
    "scroll-pair-light-unit-committed.png",
    "scroll-pair-rtl-quantity.png",
    "scroll-pair-rtl-unit.png",
    "scroll-pair-compact-editors.png",
    "scroll-pair-compact-committed.png",
}


@dataclass(frozen=True)
class Field:
    id: str
    label: str

    def part(self, suffix):
        return f"{self.id}-{suffix}"


QUANTITY = Field(f"{PAGE}-quantity", "數量")
UNIT = Field(f"{PAGE}-unit", "單位")
FIELDS = (QUANTITY, UNIT)


def scene(app, *settings):
    activate(app, "section-0")
    frame = activate(app, SHOW)
    window = node(frame, PAGE)
    require(window["role"] == "Window" and window["name"] == "數量與單位",
            "Gallery did not open the paired-field route")
    for setting in settings:
        frame = activate(app, f"{PAGE}-{setting}")
        if setting != "theme":
            require(node(frame, f"{PAGE}-{setting}").get("selected") is True,
                    f"Gallery did not enable {setting}")
    pair = node(frame, PAIR)
    require(pair["role"] == "Group" and pair["name"] == "容量",
            f"Expected the passive named 容量 group, received {pair['name']!r}")
    for field in FIELDS:
        require(node(frame, field.part("label"))["text"] == field.label,
                f"Missing separate visual label {field.label}")
    return frame


def keyboard_focus(app, target):
    path = [] if focused(settle(app)) == target else focus_by_tab(app, target)
    focus(settle(app), target)
    require(PAIR not in path, "Passive pair became a keyboard stop")
    return path


def tab_order(app, suffix):
    keyboard_focus(app, BACK)
    quantity = keyboard_focus(app, QUANTITY.part(suffix))
    require(UNIT.part(suffix) not in quantity, "Unit precedes Quantity in Tab order")
    return quantity


def back(app):
    path = keyboard_focus(app, BACK)
    frame = key(app, "enter")
    focus(frame, SHOW)
    require(find(frame, PAGE)[0] is None, "Back left the paired field route active")
    return {"tab_path": path, "restored": SHOW}


def adjustable(frame, field, *, minimum, maximum, step, value, center, value_text=None,
               disabled=False):
    control = node(frame, field.part("adjustable"))
    semantics = control.get("adjustable", {})
    require(control["name"] == field.label, f"{field.id} lost its semantic label")
    expected = {"min": minimum, "max": maximum, "step": step, "value": value}
    require(all(semantics.get(name) == expected_value for name, expected_value in expected.items()),
            f"{field.id} adjustable semantics mismatch: {semantics!r}")
    effective_value_text = semantics.get("value_text")
    if value_text is not None:
        require(effective_value_text == value_text,
                f"{field.id} lost its localized value description: {effective_value_text!r}")
    else:
        require(effective_value_text and str(value) in effective_value_text,
                f"{field.id} lost its numeric value description: {effective_value_text!r}")
    require(control.get("disabled", False) is disabled,
            f"{field.id} has the wrong disabled state")
    require(find(frame, field.part("editor"))[0] is None, f"{field.id} retained its editor")
    painted = node(frame, field.part("center"))["text"]
    require(painted == center, f"{field.id} painted center {painted!r}, expected {center!r}")
    return {"name": field.label, **semantics, "center": painted, "disabled": disabled}


def quantity(frame, value, *, disabled=False):
    return adjustable(frame, QUANTITY, minimum=-1800, maximum=1800, step=2, value=value,
                      center=str(value), value_text=str(value), disabled=disabled)


def unit(frame, value, text, *, disabled=False):
    return adjustable(frame, UNIT, minimum=1, maximum=3, step=1, value=value,
                      center=text, value_text=text, disabled=disabled)


def saved(frame, number, measure):
    text = node(frame, f"{PAGE}-saved")["text"]
    require(text == f"已選 {number} {measure}", f"Caller-owned saved pair disagrees: {text!r}")
    return text


def editor(frame, field, raw):
    control = node(frame, field.part("editor"))
    require(control["role"] == "TextInput" and control["name"] == field.label
            and control["text"] == raw, f"Expected {field.label} editor with {raw!r}")
    require(not control.get("invalid", False), f"Valid {field.label} editor shows an error")
    require(find(frame, field.part("adjustable"))[0] is None,
            f"{field.label} Input kept its adjustable control")
    return {"name": field.label, "raw": raw, "target": field.part("editor")}


def fill(app, field, text):
    target = field.part("editor")
    reply = app.wire("Action.fill", {"id": target, "value": text})
    require(reply.get("changed"), f"{target} fill did not change the draft")
    frame = settle(app)
    focus(frame, target)
    editor(frame, field, text)
    return frame


def right(rect):
    return rect["x"] + rect["width"]


def bottom(rect):
    return rect["y"] + rect["height"]


def geometry(frame, *, rtl=False, stacked=False):
    quantity_face, unit_face = (visible(frame, field.part("face"), 48.) for field in FIELDS)
    quantity_label, unit_label = (visible(frame, field.part("label")) for field in FIELDS)
    require(bottom(quantity_label) <= quantity_face["y"] and bottom(unit_label) <= unit_face["y"],
            "Separate labels must remain visible above their own faces")
    if stacked:
        require(bottom(quantity_face) <= unit_label["y"] < unit_face["y"],
                "Compact fields are not vertically stacked in quantity-first order")
    else:
        first, second = (unit_face, quantity_face) if rtl else (quantity_face, unit_face)
        require(right(first) <= second["x"] and first["y"] < bottom(second)
                and second["y"] < bottom(first), "Wide fields do not occupy the expected side-by-side order")
    return {"quantity_face": quantity_face, "unit_face": unit_face,
            "quantity_label": quantity_label, "unit_label": unit_label}


def luminance(color):
    linear = [(x / 255. / 12.92 if x / 255. <= 0.04045 else
               ((x / 255. + 0.055) / 1.055) ** 2.4) for x in color[:3]]
    return sum(x * weight for x, weight in zip(linear, (0.2126, 0.7152, 0.0722)))


def contrast(foreground, background):
    dark, light = sorted((luminance(foreground), luminance(background)))
    return (light + 0.05) / (dark + 0.05)


def pixel(rows, x, y):
    return tuple(rows[y][4 * x:4 * x + 4])


def ink(frame, rows, target, *, minimum_width):
    computed = layout(frame, target)
    box, clip = computed["bounds"], computed["clip"]
    color = tuple(node(frame, target)["style"]["foreground"])
    x0 = max(0, math.ceil(max(box["x"], clip["x"])))
    x1 = min(len(rows[0]) // 4, math.floor(min(right(box), right(clip))))
    y0 = max(0, math.ceil(max(box["y"], clip["y"])))
    y1 = min(len(rows), math.floor(min(bottom(box), bottom(clip))))

    def matches(sample):
        return sample[3] == color[3] and max(abs(a - b) for a, b in zip(sample[:3], color[:3])) <= 64

    points = [(x, y) for y in range(y0, y1) for x in range(x0, x1) if matches(pixel(rows, x, y))]
    require(len(points) >= 16, f"No readable {color!r} ink in {target}")
    xs, ys = zip(*points)
    span = [min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1]
    require(span[2] >= minimum_width and span[3] >= 6, f"Too little text ink in {target}: {span!r}")
    return {"text": node(frame, target)["text"], "ink_box": span, "ink_pixels": len(points),
            "rgb_tolerance": 64, "rgba": list(color)}


def field_paint(frame, rows, field, *, editing):
    field_face = field.part("face")
    box = visible(frame, field_face)
    face = node(frame, field_face)
    y = math.floor(box["y"] + box["height"] / 2.)
    actual = pixel(rows, math.floor(box["x"] + 10.), y)
    base = tuple(face["style"]["background"])
    rendered_control = field.part("editor" if editing else "adjustable")
    is_focused = focused(frame) == rendered_control
    tint = face["style"]["state_layer"]
    alpha = face["state_layer_alphas"][1] if is_focused and not editing else 0
    expected = tuple(round((bg * (255 - alpha) + fg * alpha) / 255.)
                     for bg, fg in zip(base[:3], tint[:3])) + (255,)
    require(max(abs(a - b) for a, b in zip(actual, expected)) <= 1,
            f"{field.id} face pixels {actual!r} disagree with token {expected!r}")
    target = field.part("editor" if editing else "center")
    text = node(frame, target)["text"]
    glyphs = sum(not ch.isspace() for ch in text)
    active = ink(frame, rows, target, minimum_width=8 * glyphs)
    ratio = contrast(active["rgba"], actual)
    require(ratio >= 4.5, f"{field.id} active text contrast only {ratio:.3f}:1")
    label = ink(frame, rows, field.part("label"), minimum_width=8)
    report = {"face_rgba": list(actual), "base_face_rgba": list(base), "active": active,
              "label": label, "focus_layer_alpha": alpha, "text_contrast": round(ratio, 4),
              "focused": is_focused}
    if is_focused:
        expected_focus = tuple(node(frame, field.part("focus"))["focus_color"])
        stroke = pixel(rows, math.floor(box["x"] - 4.), y)
        gap = pixel(rows, math.floor(box["x"] - 1.), y)
        require(stroke == expected_focus and gap != stroke and contrast(stroke, gap) >= 3.,
                f"{field.id} focused ring lost its stroke, contrast or gap")
        report.update({"focus_stroke_rgba": list(stroke), "focus_gap_rgba": list(gap)})
    if editing:
        stroke = pixel(rows, math.floor(box["x"] + 1.), y)
        require(stroke == tuple(face["style"]["border_color"]) == tuple(active["rgba"]),
                f"{field.id} editor outline and glyph inks disagree")
        report["editor_stroke_rgba"] = list(stroke)
    return report


def shot(app, plan, title, frame, *, editing=False):
    before = onscreen_windows(app.proc.pid)
    receipt = capture(app, plan, title, frame=frame)
    after = onscreen_windows(app.proc.pid)
    require(before == after == 0, f"{title} opened a visible window")
    viewport = frame["layout"]["viewport"]
    require((receipt["width"], receipt["height"]) == (viewport["width"], viewport["height"]),
            "PNG dimensions differ from the native viewport")
    _, _, rows = read_png((plan.out / receipt["name"]).read_bytes())
    paint = {field.label: field_paint(frame, rows, field, editing=editing) for field in FIELDS}
    receipt.update({"visible_windows": [before, after], "paint": paint})
    print(f"PASS {title}: identical PNG through API, CLI and MCP", flush=True)
    return receipt


def light(app, plan):
    frame = scene(app)
    require(luminance(node(frame, PAGE)["style"]["background"]) > 0.5, "Light profile opened dark")
    initial = {"quantity": quantity(frame, 1200), "unit": unit(frame, 1, "毫升"),
               "saved": saved(frame, 1200, "毫升")}
    tab_quantity = tab_order(app, "adjustable")
    frame = key(app, "up")
    focus(frame, QUANTITY.part("adjustable"))
    after_quantity = {"quantity": quantity(frame, 1202), "unit": unit(frame, 1, "毫升"),
                      "saved": saved(frame, 1202, "毫升")}
    layout_once = geometry(frame)
    adjusted = shot(app, plan, "scroll-pair-light-adjusted", frame)

    tab_unit = keyboard_focus(app, UNIT.part("adjustable"))
    frame = key(app, "enter")
    focus(frame, UNIT.part("editor"))
    editor(frame, UNIT, "毫升")
    quantity(frame, 1202)
    draft = fill(app, UNIT, "fl oz")
    saved(draft, 1202, "毫升")
    frame = key(app, "enter")
    focus(frame, UNIT.part("adjustable"))
    after_unit = {"quantity": quantity(frame, 1202), "unit": unit(frame, 3, "液量盎司"),
                  "saved": saved(frame, 1202, "液量盎司"), "submitted_alias": "fl oz"}
    require(geometry(frame) == layout_once, "Unit entry moved the paired-field layout")
    committed = shot(app, plan, "scroll-pair-light-unit-committed", frame)
    return {"initial": initial, "after_quantity_up": after_quantity, "after_unit_entry": after_unit,
            "geometry": layout_once, "tab_to_quantity": tab_quantity, "tab_to_unit": tab_unit,
            "back": back(app), "captures": [adjusted, committed]}


def disabled_pair(app):
    frame = activate(app, f"{PAGE}-unit-disabled")
    quantity(frame, 1200)
    unit(frame, 2, "公升", disabled=True)
    unit_rejected = app.wire("Action.activate", {"id": UNIT.part("adjustable")},
                            expected_error="disabled")
    keyboard_focus(app, BACK)
    keyboard_focus(app, QUANTITY.part("adjustable"))
    frame = key(app, "up")
    quantity(frame, 1202)
    unit(frame, 2, "公升", disabled=True)
    saved(frame, 1202, "公升")
    tab_disabled_unit = keyboard_focus(app, BACK)
    require(UNIT.part("adjustable") not in tab_disabled_unit, "Disabled Unit remained in Tab order")

    activate(app, f"{PAGE}-unit-disabled")
    frame = activate(app, f"{PAGE}-quantity-disabled")
    quantity(frame, 1202, disabled=True)
    unit(frame, 2, "公升")
    quantity_rejected = app.wire("Action.activate", {"id": QUANTITY.part("adjustable")},
                                expected_error="disabled")
    keyboard_focus(app, BACK)
    tab_disabled_quantity = keyboard_focus(app, UNIT.part("adjustable"))
    require(QUANTITY.part("adjustable") not in tab_disabled_quantity,
            "Disabled Quantity remained in Tab order")
    frame = key(app, "up")
    return {"unit_activation": unit_rejected, "quantity_activation": quantity_rejected,
            "skip_disabled_unit": tab_disabled_unit, "skip_disabled_quantity": tab_disabled_quantity,
            "quantity_after_unit_up": quantity(frame, 1202, disabled=True),
            "unit_after_up": unit(frame, 3, "液量盎司"), "saved": saved(frame, 1202, "液量盎司")}


def rtl(app, plan):
    frame = scene(app, "theme", "rtl")
    require(luminance(node(frame, PAGE)["style"]["background"]) < 0.2, "RTL profile opened light")
    tab_quantity = tab_order(app, "adjustable")
    frame = settle(app)
    initial = {"quantity": quantity(frame, 1200), "unit": unit(frame, 1, "毫升"),
               "saved": saved(frame, 1200, "毫升")}
    placement = geometry(frame, rtl=True)
    first = shot(app, plan, "scroll-pair-rtl-quantity", frame)
    tab_unit = keyboard_focus(app, UNIT.part("adjustable"))
    frame = key(app, "up")
    focus(frame, UNIT.part("adjustable"))
    after_unit = {"quantity": quantity(frame, 1200), "unit": unit(frame, 2, "公升"),
                  "saved": saved(frame, 1200, "公升")}
    require(geometry(frame, rtl=True) == placement, "Unit adjustment changed the mirrored layout")
    second = shot(app, plan, "scroll-pair-rtl-unit", frame)
    return {"rtl": True, "geometry": placement, "initial": initial, "after_unit_up": after_unit,
            "tab_to_quantity": tab_quantity, "tab_to_unit": tab_unit,
            "independent_disabled": disabled_pair(app), "back": back(app),
            "captures": [first, second]}


def compact(app, plan):
    frame = scene(app, "theme", "rtl", "large-text")
    require(luminance(node(frame, PAGE)["style"]["background"]) < 0.2, "Compact profile opened light")
    editor(frame, QUANTITY, "1200")
    editor(frame, UNIT, "毫升")
    tab_quantity = tab_order(app, "editor")
    frame = settle(app)
    stacked = geometry(frame, rtl=True, stacked=True)
    initial = {"quantity": editor(frame, QUANTITY, "1200"), "unit": editor(frame, UNIT, "毫升"),
               "saved": saved(frame, 1200, "毫升")}
    start = shot(app, plan, "scroll-pair-compact-editors", frame, editing=True)

    draft = fill(app, QUANTITY, "１３００")
    editor(draft, UNIT, "毫升")
    saved(draft, 1200, "毫升")
    frame = key(app, "enter")
    focus(frame, QUANTITY.part("editor"))
    after_quantity = {"quantity": editor(frame, QUANTITY, "1300"), "unit": editor(frame, UNIT, "毫升"),
                      "saved": saved(frame, 1300, "毫升")}
    tab_unit = keyboard_focus(app, UNIT.part("editor"))
    draft = fill(app, UNIT, "L")
    editor(draft, QUANTITY, "1300")
    saved(draft, 1300, "毫升")
    frame = key(app, "enter")
    focus(frame, UNIT.part("editor"))
    after_unit = {"quantity": editor(frame, QUANTITY, "1300"), "unit": editor(frame, UNIT, "公升"),
                  "saved": saved(frame, 1300, "公升")}
    require(geometry(frame, rtl=True, stacked=True) == stacked,
            "Accepted editor values changed the stacked layout")
    accepted = shot(app, plan, "scroll-pair-compact-committed", frame, editing=True)
    return {"rtl": True, "text_scale": 2, "geometry": stacked, "initial": initial,
            "after_quantity_entry": after_quantity, "after_unit_entry": after_unit,
            "tab_to_quantity": tab_quantity, "tab_to_unit": tab_unit, "back": back(app),
            "captures": [start, accepted]}


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
        "input": "headless DevTools Action.key, Action.activate and Action.fill",
        "transports": ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"],
        "profiles": {},
    }
    for replay, width, height in ((light, 960, 820), (rtl, 960, 820), (compact, 320, 820)):
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
