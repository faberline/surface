#!/usr/bin/env python3
"""Replay the nonmodal quantity and unit popup through the offscreen Gallery."""

import argparse
from dataclasses import dataclass
import json
import math
from pathlib import Path

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, focused, launch
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

PAGE = "scroll-popup-demo"
SHOW = f"{PAGE}-show"
BACK = f"{PAGE}-back"
OPENER = f"{PAGE}-opener"
OUTSIDE = f"{PAGE}-outside"
DIALOG = f"{PAGE}-dialog"
BODY = f"{PAGE}-body"
CLOSE = f"{PAGE}-close"
PAIR = f"{PAGE}-pair"
REPORT_NAME = "native-scroll-popup-report.json"
IMAGE_NAMES = {
    "scroll-popup-light-below.png",
    "scroll-popup-light-accepted.png",
    "scroll-popup-rtl-above.png",
    "scroll-popup-rtl-draft.png",
    "scroll-popup-compact-quantity.png",
    "scroll-popup-compact-unit.png",
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
    require(window["role"] == "Window", "Gallery did not open the popup demo route")
    for setting in settings:
        frame = activate(app, f"{PAGE}-{setting}")
        if setting != "theme":
            require(node(frame, f"{PAGE}-{setting}").get("selected") is True,
                    f"Gallery did not enable {setting}")
    require(find(frame, DIALOG)[0] is None, "Popup should start closed")
    return frame


def popup(frame):
    dialog = node(frame, DIALOG)
    require(dialog["role"] == "Dialog" and dialog["name"] == "編輯容量",
            "Open popup lost its dialog role or localized name")
    pair = node(frame, PAIR)
    require(pair["role"] == "Group", "The quantity and unit lost their shared group")
    require(node(frame, OPENER).get("expanded") is True, "Open popup lost the opener's expanded state")
    for field in FIELDS:
        require(node(frame, field.part("label"))["text"] == field.label,
                f"Missing separate visual label {field.label}")
    return frame


def keyboard_focus(app, target):
    path = [] if focused(settle(app)) == target else focus_by_tab(app, target)
    focus(settle(app), target)
    require(PAIR not in path and DIALOG not in path, "Passive pair or dialog became a Tab stop")
    return path


def open_popup(app, *, editing=False):
    path = keyboard_focus(app, OPENER)
    frame = popup(key(app, "enter"))
    expected = QUANTITY.part("editor" if editing else "adjustable")
    focus(frame, expected)
    visible(frame, expected, 48.)
    return frame, path


def close_popup(app):
    path = keyboard_focus(app, CLOSE)
    frame = key(app, "enter")
    require(find(frame, DIALOG)[0] is None, "Explicit Close left the popup mounted")
    focus(frame, OPENER)
    require(node(frame, OPENER).get("expanded") is False, "Closed opener still announces expanded")
    return frame, {"tab_path": path, "restored": OPENER}


def back(app):
    path = keyboard_focus(app, BACK)
    frame = key(app, "enter")
    focus(frame, SHOW)
    require(find(frame, PAGE)[0] is None, "Back left the popup route active")
    return {"tab_path": path, "restored": SHOW}


def adjustable(frame, field, *, minimum, maximum, step, value, center, value_text):
    control = node(frame, field.part("adjustable"))
    semantics = control.get("adjustable", {})
    require(control["name"] == field.label, f"{field.id} lost its semantic label")
    expected = {"min": minimum, "max": maximum, "step": step, "value": value}
    require(all(semantics.get(name) == expected_value for name, expected_value in expected.items()),
            f"{field.id} adjustable semantics mismatch: {semantics!r}")
    require(semantics.get("value_text") == value_text and not control.get("disabled", False),
            f"{field.id} is disabled or lost its value description")
    require(find(frame, field.part("editor"))[0] is None, f"{field.id} retained its editor")
    painted = node(frame, field.part("center"))["text"]
    require(painted == center, f"{field.id} painted center {painted!r}, expected {center!r}")
    return {"name": field.label, **semantics, "center": painted}


def quantity(frame, value):
    return adjustable(frame, QUANTITY, minimum=-1800, maximum=1800, step=2, value=value,
                      center=str(value), value_text=str(value))


def unit(frame, value, text):
    return adjustable(frame, UNIT, minimum=1, maximum=3, step=1, value=value,
                      center=text, value_text=text)


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


def placement(frame, *, above, rtl=False):
    opener = visible(frame, OPENER)
    dialog = visible(frame, DIALOG, 48.)
    body = visible(frame, BODY, 48.)
    close = visible(frame, CLOSE, 48.)
    close_node = node(frame, CLOSE)
    require(close_node["role"] == "Button" and close_node["name"] == "關閉",
            "Popup Close lost its localized button label")
    require(abs(close["height"] - 48.) <= 0.51, "The fixed Close target must be 48dp high")
    require(abs(close["y"] - bottom(body)) <= 0.51 and bottom(close) <= bottom(dialog) + 0.51,
            "Close should remain below the separately clipped popup body")
    gap = opener["y"] - bottom(dialog) if above else dialog["y"] - bottom(opener)
    require(abs(gap - 4.) <= 0.51, f"Popup misses its {('above' if above else 'below')} anchor by {gap}dp")
    viewport = frame["layout"]["viewport"]
    expected_x = right(opener) - dialog["width"] if rtl else opener["x"]
    expected_x = max(8., min(expected_x, viewport["width"] - 8. - dialog["width"]))
    require(abs(expected_x - dialog["x"]) <= 0.51, "Popup misses the opener's logical start edge")
    require(dialog["x"] >= 7.49 and dialog["y"] >= 7.49 and
            right(dialog) <= viewport["width"] - 7.49 and bottom(dialog) <= viewport["height"] - 7.49,
            "Popup breaches its 8dp viewport inset")
    return {"side": "above" if above else "below", "opener": opener, "dialog": dialog,
            "body": body, "close": close, "gap": gap}


def body_scroll(frame):
    return node(frame, BODY)["style"].get("scroll_y", 0.)


def outside_bounds(frame):
    bounds = visible(frame, OUTSIDE)
    viewport = frame["layout"]["viewport"]
    require(bounds["x"] >= viewport["x"] and bounds["y"] >= viewport["y"]
            and right(bounds) <= right(viewport) and bottom(bounds) <= bottom(viewport),
            "Compact Outside button extends beyond the viewport")
    return bounds


def geometry(frame, *, rtl=False):
    quantity_face, unit_face = (visible(frame, field.part("face"), 48.) for field in FIELDS)
    quantity_label, unit_label = (visible(frame, field.part("label")) for field in FIELDS)
    require(bottom(quantity_label) <= quantity_face["y"] and bottom(unit_label) <= unit_face["y"],
            "Separate labels must remain visible above their own faces")
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


def shot(app, plan, title, frame, *, fields=FIELDS):
    before = onscreen_windows(app.proc.pid)
    receipt = capture(app, plan, title, frame=frame)
    after = onscreen_windows(app.proc.pid)
    require(before == after == 0, f"{title} opened a visible window")
    viewport = frame["layout"]["viewport"]
    require((receipt["width"], receipt["height"]) == (viewport["width"], viewport["height"]),
            "PNG dimensions differ from the native viewport")
    _, _, rows = read_png((plan.out / receipt["name"]).read_bytes())
    paint = {}
    for field in fields:
        editing = find(frame, field.part("editor"))[0] is not None
        paint[field.label] = field_paint(frame, rows, field, editing=editing)
    receipt.update({"visible_windows": [before, after], "paint": paint,
                    "focus": focused(frame), "body_scroll": body_scroll(frame),
                    "close": visible(frame, CLOSE, 48.)})
    print(f"PASS {title}: identical PNG through API, CLI and MCP", flush=True)
    return receipt


def light(app, plan):
    frame = scene(app)
    require(luminance(node(frame, PAGE)["style"]["background"]) > 0.5, "Light profile opened dark")
    require(node(frame, OUTSIDE)["name"] == "外部 0 次", "Outside count should start at zero")
    frame, opener_path = open_popup(app)
    below = placement(frame, above=False)
    paired = geometry(frame)
    initial = {"quantity": quantity(frame, 1200), "unit": unit(frame, 1, "毫升"),
               "saved": saved(frame, 1200, "毫升")}
    first = shot(app, plan, "scroll-popup-light-below", frame)

    outside_path = keyboard_focus(app, OUTSIDE)
    frame = popup(key(app, "enter"))
    focus(frame, OUTSIDE)
    outside_label = node(frame, OUTSIDE)["name"]
    require(outside_label == "外部 1 次", "Outside keyboard action was not delivered")
    saved(frame, 1200, "毫升")

    qty_path = keyboard_focus(app, QUANTITY.part("adjustable"))
    frame = key(app, "up")
    focus(frame, QUANTITY.part("adjustable"))
    edited_quantity = {"quantity": quantity(frame, 1202), "unit": unit(frame, 1, "毫升"),
                       "saved": saved(frame, 1202, "毫升")}
    unit_path = keyboard_focus(app, UNIT.part("adjustable"))
    frame = key(app, "enter")
    focus(frame, UNIT.part("editor"))
    editor(frame, UNIT, "毫升")
    saved(fill(app, UNIT, "fl oz"), 1202, "毫升")
    frame = popup(key(app, "enter"))
    focus(frame, UNIT.part("adjustable"))
    accepted = {"quantity": quantity(frame, 1202), "unit": unit(frame, 3, "液量盎司"),
                "saved": saved(frame, 1202, "液量盎司")}
    require(placement(frame, above=False) == below and geometry(frame) == paired,
            "Independent accepted values moved the anchored panel or fields")
    second = shot(app, plan, "scroll-popup-light-accepted", frame)
    frame, closed = close_popup(app)
    saved(frame, 1202, "液量盎司")
    return {"placement": below, "geometry": paired, "initial": initial,
            "outside": {"count_label": outside_label, "tab_path": outside_path,
                        "dialog_retained": True, "focus": OUTSIDE},
            "after_quantity_up": edited_quantity, "accepted": accepted,
            "opener_tab_path": opener_path, "quantity_tab_path": qty_path,
            "unit_tab_path": unit_path, "close": closed, "back": back(app),
            "captures": [first, second]}


def rtl(app, plan):
    frame = scene(app, "theme", "rtl", "bottom")
    require(luminance(node(frame, PAGE)["style"]["background"]) < 0.2, "RTL profile opened light")
    frame, opener_path = open_popup(app)
    above = placement(frame, above=True, rtl=True)
    paired = geometry(frame, rtl=True)
    quantity(frame, 1200)
    unit(frame, 1, "毫升")
    first = shot(app, plan, "scroll-popup-rtl-above", frame)

    unit_path = keyboard_focus(app, UNIT.part("adjustable"))
    frame = key(app, "enter")
    focus(frame, UNIT.part("editor"))
    editor(frame, UNIT, "毫升")
    draft = fill(app, UNIT, "L")
    saved(draft, 1200, "毫升")
    require(placement(draft, above=True, rtl=True) == above, "Input changed the anchored panel")
    second = shot(app, plan, "scroll-popup-rtl-draft", draft)
    frame, closed = close_popup(app)
    saved(frame, 1200, "毫升")

    frame, reopen = open_popup(app)
    unit(frame, 1, "毫升")
    keyboard_focus(app, UNIT.part("adjustable"))
    frame = key(app, "enter")
    reset = editor(frame, UNIT, "毫升")
    saved(frame, 1200, "毫升")
    frame, _ = close_popup(app)
    saved(frame, 1200, "毫升")

    frame, _ = open_popup(app)
    hide = f"{PAGE}-hide-opener"
    hide_path = keyboard_focus(app, hide)
    frame = key(app, "enter")
    focus(frame, hide)
    require(find(frame, OPENER)[0] is None and find(frame, DIALOG)[0] is None,
            "Removing the anchor left an orphaned popup or opener")
    frame = key(app, "enter")
    focus(frame, hide)
    require(find(frame, DIALOG)[0] is None and not node(frame, OPENER).get("expanded", False),
            "Showing the anchor reopened the closed popup")
    return {"rtl": True, "placement": above, "geometry": paired, "opener_tab_path": opener_path,
            "unit_tab_path": unit_path, "unsubmitted_draft": "L", "reset_editor": reset,
            "close": closed, "reopen_tab_path": reopen,
            "hide_opener": {"focus": hide, "tab_path": hide_path, "collapsed_on_show": True},
            "back": back(app), "captures": [first, second]}


def compact(app, plan):
    frame = scene(app, "theme", "rtl", "large-text", "bottom")
    require(luminance(node(frame, PAGE)["style"]["background"]) < 0.2, "Compact profile opened light")
    frame, opener_path = open_popup(app, editing=True)
    bounds = placement(frame, above=True, rtl=True)
    quantity_target = QUANTITY.part("editor")
    unit_target = UNIT.part("editor")
    initial = {"quantity": editor(frame, QUANTITY, "1200"), "unit": editor(frame, UNIT, "毫升"),
               "saved": saved(frame, 1200, "毫升")}
    quantity_editor = visible(frame, quantity_target, 48.)
    initial_scroll = body_scroll(frame)
    start = shot(app, plan, "scroll-popup-compact-quantity", frame, fields=(QUANTITY,))

    draft = fill(app, QUANTITY, "１３００")
    saved(draft, 1200, "毫升")
    editor(draft, UNIT, "毫升")
    frame = key(app, "enter")
    focus(frame, quantity_target)
    after_quantity = {"quantity": editor(frame, QUANTITY, "1300"), "unit": editor(frame, UNIT, "毫升"),
                      "saved": saved(frame, 1300, "毫升")}
    unit_path = keyboard_focus(app, unit_target)
    frame = settle(app)
    reveal_scroll = body_scroll(frame)
    unit_editor = visible(frame, unit_target, 48.)
    require(reveal_scroll > initial_scroll, "Tab did not scroll the popup body to reveal Unit")
    require(placement(frame, above=True, rtl=True) == bounds, "Body scroll moved the anchored Close row")
    draft = fill(app, UNIT, "L")
    editor(draft, QUANTITY, "1300")
    saved(draft, 1300, "毫升")
    frame = key(app, "enter")
    focus(frame, unit_target)
    after_unit = {"quantity": editor(frame, QUANTITY, "1300"), "unit": editor(frame, UNIT, "公升"),
                  "saved": saved(frame, 1300, "公升")}
    require(placement(frame, above=True, rtl=True) == bounds, "Input moved the fixed Close row")
    accepted = shot(app, plan, "scroll-popup-compact-unit", frame, fields=(UNIT,))

    outside_path = keyboard_focus(app, OUTSIDE)
    frame = settle(app)
    require(node(frame, OUTSIDE)["name"] == "外部 0 次", "Compact Outside count should start at zero")
    before_outside = outside_bounds(frame)
    frame = popup(key(app, "enter"))
    focus(frame, OUTSIDE)
    outside_label = node(frame, OUTSIDE)["name"]
    require(outside_label == "外部 1 次", "Compact Outside keyboard action was not delivered")
    after_outside = outside_bounds(frame)
    editor(frame, QUANTITY, "1300")
    editor(frame, UNIT, "公升")
    saved(frame, 1300, "公升")

    frame, closed = close_popup(app)
    saved(frame, 1300, "公升")
    return {"rtl": True, "text_scale": 2, "placement": bounds, "initial": initial,
            "quantity_editor": quantity_editor, "unit_editor": unit_editor,
            "body_scroll_before": initial_scroll, "body_scroll_unit": reveal_scroll,
            "after_quantity_entry": after_quantity, "after_unit_entry": after_unit,
            "opener_tab_path": opener_path, "unit_tab_path": unit_path,
            "outside": {"count_label": outside_label, "tab_path": outside_path,
                        "before": before_outside, "after": after_outside,
                        "focus": OUTSIDE, "dialog_retained": True},
            "close": closed, "back": back(app), "captures": [start, accepted]}


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
    for replay, width, height in ((light, 960, 820), (rtl, 960, 820), (compact, 320, 400)):
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
