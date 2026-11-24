#!/usr/bin/env python3
"""Replay the Gallery's transactional Scroll Field dialog through DevTools."""

import argparse
from dataclasses import dataclass
import json
import math
from pathlib import Path

from gallery_transport import png_payload, read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box
from render_helpers import SHOT, activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

PAGE = "scroll-modal-demo"
SHOW = f"{PAGE}-show"
BACK = f"{PAGE}-back"
OPENER = f"{PAGE}-opener"
OUTSIDE = f"{PAGE}-outside"
DIALOG = f"{PAGE}-dialog"
LAYER = f"{PAGE}-layer"
BODY = f"{PAGE}-body"
PAIR = f"{PAGE}-pair"
CANCEL = f"{PAGE}-cancel"
SAVE = f"{PAGE}-save"
SAVED = f"{PAGE}-saved"
STATUS = f"{PAGE}-status"
REPORT_NAME = "native-scroll-modal-report.json"
IMAGE_NAMES = {
    "scroll-modal-light-open.png",
    "scroll-modal-light-unit-focus.png",
    "scroll-modal-light-saved.png",
    "scroll-modal-rtl-quantity-error.png",
    "scroll-modal-rtl-errors.png",
    "scroll-modal-rtl-valid-peer.png",
    "scroll-modal-compact-quantity.png",
    "scroll-modal-compact-unit.png",
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


def right(box):
    return box["x"] + box["width"]


def bottom(box):
    return box["y"] + box["height"]


def in_dialog(frame, target):
    if not isinstance(target, str) or not target:
        return False
    dialog, _ = find(frame, DIALOG)
    index, _ = find(frame, target)
    nodes = frame["dom"]["nodes"]
    while index is not None and index != dialog:
        index = nodes[index].get("parent")
    return dialog is not None and index == dialog and target != DIALOG


def scene(app, *settings):
    activate(app, "section-0")
    frame = activate(app, SHOW)
    require(node(frame, PAGE)["role"] == "Window", "Gallery did not open the dialog fixture")
    for setting in settings:
        target = f"{PAGE}-{setting}"
        frame = activate(app, target)
        state = node(frame, target)
        flag = "checked" if setting == "theme" else "selected"
        require(state.get(flag) is True, f"Gallery did not enable {setting}")
    require(find(frame, DIALOG)[0] is None, "Dialog opened before invoking the opener")
    return frame


def keyboard_focus(app, target):
    path = [] if focused(settle(app)) == target else focus_by_tab(app, target)
    focus(settle(app), target)
    return path


def saved(frame, number, label):
    expected = f"已儲存 {number} {label}"
    value = node(frame, SAVED)["text"]
    require(value == expected, f"Expected the whole accepted pair {expected!r}, got {value!r}")
    return value


def outside_count(frame, count):
    expected = f"外部 {count} 次"
    value = node(frame, OUTSIDE)["name"]
    require(value == expected, f"Expected {expected!r}, got {value!r}")
    return value


def shell(frame):
    dialog = node(frame, DIALOG)
    body = node(frame, BODY)
    require(dialog["role"] == "Dialog" and dialog["name"] == "編輯容量",
            "The modal lost its named dialog semantics")
    require(body["name"] == "容量欄位" and body["style"].get("clip") is True,
            "The body lost its accessible name or scroll viewport")
    require(node(frame, PAIR)["name"] == "容量" and node(frame, PAIR)["role"] == "Group",
            "The two fields lost their shared semantic group")
    require(node(frame, OPENER).get("expanded") is True, "The opener must report expanded")
    require(in_dialog(frame, BODY), "The scroll body is outside its modal scope")
    for field in FIELDS:
        require(in_dialog(frame, field.id) and node(frame, field.part("label"))["text"] == field.label,
                f"Missing independent field and label for {field.label}")


def geometry(frame):
    viewport = frame["layout"]["viewport"]
    panel = visible(frame, DIALOG, 48.)
    body = visible(frame, BODY)
    require(same_box(viewport, panel) and same_box(panel, body), "Dialog or body is clipped")
    require(abs((panel["x"] + panel["width"] / 2.) - (viewport["x"] + viewport["width"] / 2.)) <= 1.
            and abs((panel["y"] + panel["height"] / 2.) - (viewport["y"] + viewport["height"] / 2.)) <= 1.,
            "Dialog is not centered in the viewport")
    footer = {}
    for target, name in ((CANCEL, "取消"), (SAVE, "儲存")):
        action = node(frame, target)
        bounds = visible(frame, target, 48.)
        require(in_dialog(frame, target) and action["role"] == "Button" and action["name"] == name,
                f"Footer action {name} lost its role, name, or scope")
        require(same_box(panel, bounds) and bounds["y"] >= bottom(body) - 0.51,
                f"Footer action {name} intersects its scroller or leaves the panel")
        footer[target] = bounds
    first, second = footer.values()
    require(right(first) <= second["x"] + 0.51 or right(second) <= first["x"] + 0.51
            or bottom(first) <= second["y"] + 0.51 or bottom(second) <= first["y"] + 0.51,
            "Cancel and Save hit targets overlap")
    return {"dialog": panel, "body": body, "footer": footer}


def body_scroll(frame):
    return node(frame, BODY)["style"].get("scroll_y", 0.)


def open_dialog(app, *, editing=False):
    path = keyboard_focus(app, OPENER)
    frame = key(app, "enter")
    shell(frame)
    target = QUANTITY.part("editor" if editing else "adjustable")
    focus(frame, target)
    visible(frame, target, 48.)
    return frame, path


def assert_closed(frame):
    require(all(find(frame, target)[0] is None for target in (DIALOG, LAYER, BODY, CANCEL, SAVE)),
            "Closing left the dialog or scrim mounted")
    require(node(frame, OPENER).get("expanded") is False, "Closed opener still reports expanded")
    focus(frame, OPENER)
    return frame


def cancel(app):
    path = keyboard_focus(app, CANCEL)
    frame = assert_closed(key(app, "enter"))
    return frame, {"tab_path": path, "restored_focus": focused(frame)}


def back(app):
    keyboard_focus(app, BACK)
    frame = key(app, "enter")
    require(find(frame, PAGE)[0] is None, "Back did not leave the demo")
    focus(frame, SHOW)
    return focused(frame)


def tab_wrap(app, *, editing, pinned_geometry=None):
    start = QUANTITY.part("editor" if editing else "adjustable")
    focus(settle(app), start)

    def traverse(reverse):
        stops = []
        for _ in range(16):
            frame = key(app, "tab", reverse=reverse)
            target = focused(frame)
            require(in_dialog(frame, target), f"Tab left the modal to {target!r}")
            visible(frame, target, 48.)
            if pinned_geometry is not None:
                require(geometry(frame) == pinned_geometry, f"The footer moved while focusing {target}")
            if target == start:
                return stops + [target]
            require(target not in stops, f"Tab loop failed to return to {start}: {stops!r}")
            stops.append(target)
        raise RuntimeError(f"Tab did not wrap within 16 steps: {stops!r}")

    forward = traverse(reverse=False)
    backward = traverse(reverse=True)
    require(backward == [*reversed(forward[:-1]), start], "Tab and Shift+Tab did not wrap in reverse order")
    second = UNIT.part("editor" if editing else "adjustable")
    require({second, CANCEL, SAVE}.issubset(forward), "Tab skipped the Unit field or a footer action")
    require(forward.index(second) < min(forward.index(CANCEL), forward.index(SAVE)),
            "Tab visited the footer before the second field")
    return {"started_at": start, "tab": forward, "shift_tab": backward}


def reject_background(app, frame, count):
    reply = app.wire("Action.activate", {"id": OUTSIDE}, expected_error="outside_modal")
    after = settle(app)
    require(frame == after, "A background activation changed the live dialog or focus")
    return {"rejection": reply, "count_label": outside_count(after, count), "focus": focused(after)}


def adjustable(frame, field, *, minimum, maximum, step, value, text):
    target = node(frame, field.part("adjustable"))
    expected = {"min": minimum, "max": maximum, "step": step, "value": value, "value_text": text}
    detail = target.get("adjustable", {})
    require(target["name"] == field.label and all(detail.get(k) == v for k, v in expected.items()),
            f"Incorrect adjustable semantics for {field.label}: {detail!r}")
    require(node(frame, field.part("center"))["text"] == text, f"Incorrect wheel text for {field.label}")
    return detail


def wheels(frame, number, ordinal, label):
    quantity = adjustable(frame, QUANTITY, minimum=-1800, maximum=1800, step=2,
                          value=number, text=str(number))
    unit = adjustable(frame, UNIT, minimum=1, maximum=3, step=1, value=ordinal, text=label)
    return {"quantity": quantity, "unit": unit}


def editor(frame, field, raw, *, error=None):
    target = field.part("editor")
    control = node(frame, target)
    details = frame["text_inputs"][target]
    require(control["role"] == "TextInput" and control["name"] == field.label and control["text"] == raw
            and details["value"] == details["display_value"] == raw and not details["composing"],
            f"The {field.label} editor did not keep draft {raw!r}")
    require(control.get("invalid", False) == (error is not None), f"Invalid state mismatch for {field.label}")
    if error is not None:
        error_id = field.part("error")
        require(control.get("described_by_text") == error_id and node(frame, error_id)["text"] == error,
                f"The {field.label} error is missing or has no association")
    return {"name": field.label, "raw": raw, "error": error, "target": target}


def enter_editor(app, field, raw):
    target = field.part("editor")
    if find(settle(app), target)[0] is None:
        keyboard_focus(app, field.part("adjustable"))
        frame = key(app, "enter")
    else:
        keyboard_focus(app, target)
        frame = settle(app)
    focus(frame, target)
    visible(frame, target, 48.)
    editor(frame, field, raw)
    return frame


def fill(app, field, text, *, error=None):
    target = field.part("editor")
    reply = app.wire("Action.fill", {"id": target, "value": text})
    require(reply.get("changed"), f"Fill made no change for {target}")
    frame = settle(app)
    focus(frame, target)
    editor(frame, field, text, error=error)
    return frame


def pixel(rows, x, y):
    return tuple(rows[y][4 * x:4 * x + 4])


def blended(under, over):
    opacity = over[3] / 255.
    return tuple(round(back * (1. - opacity) + front * opacity)
                 for back, front in zip(under[:3], over[:3])) + (255,)


def scrim_pixels(frame, closed_rows, open_rows):
    layer = visible(frame, LAYER, 48.)
    viewport = frame["layout"]["viewport"]
    require(same_box(layer, viewport) and same_box(viewport, layer), "Backdrop does not cover the viewport")
    color = node(frame, LAYER)["style"]["background"]
    require(0 < color[3] < 255, "Backdrop loses its translucent scrim token")
    width, height = len(open_rows[0]) // 4, len(open_rows)
    samples = []
    for x, y in ((1, 1), (width - 2, 1), (1, height - 2), (width - 2, height - 2)):
        before = pixel(closed_rows, x, y)
        after = pixel(open_rows, x, y)
        expected = blended(before, color)
        require(before != after and max(abs(a - b) for a, b in zip(after, expected)) <= 2,
                f"Backdrop pixel {(x, y)} {after!r} does not match the scrim blend {expected!r}")
        samples.append({"xy": [x, y], "closed": before, "modal": after})
    return {"layer": layer, "token": color, "pixels": samples}


def readable_ink(frame, rows, target):
    computed = layout(frame, target)
    bounds, clip = computed["bounds"], computed["clip"]
    style = node(frame, target)["style"]
    rgba = tuple(style["foreground"])
    left = max(0, math.ceil(max(bounds["x"], clip["x"])))
    top = max(0, math.ceil(max(bounds["y"], clip["y"])))
    end_x = min(len(rows[0]) // 4, math.floor(min(right(bounds), right(clip))))
    end_y = min(len(rows), math.floor(min(bottom(bounds), bottom(clip))))
    points = []
    for y in range(top, end_y):
        for x in range(left, end_x):
            sample = pixel(rows, x, y)
            if sample[3] == rgba[3] and max(abs(a - b) for a, b in zip(sample[:3], rgba[:3])) <= 32:
                points.append((x, y))
    require(len(points) >= 16, f"No foreground ink at {target}")
    xs, ys = zip(*points)
    ink = [min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1]
    require(ink[2] >= 4 and ink[3] >= 6, f"Too little foreground ink at {target}: {ink!r}")
    return {"ink_px": len(points), "bounds_px": ink, "rgba": list(rgba), "rgb_tolerance": 32}


def field_ring(frame, rows, field):
    bounds = layout(frame, field.part("face"))["bounds"]
    ring_id = field.part("focus")
    ring = visible(frame, ring_id)
    color = tuple(node(frame, ring_id)["focus_color"])
    y = math.floor(bounds["y"] + bounds["height"] / 2.)
    x = math.floor(bounds["x"] - 4.)
    require(0 <= x < len(rows[0]) // 4 and 0 <= y < len(rows), "Focused field ring sample leaves the viewport")
    stroke = pixel(rows, x, y)
    require(stroke == color and stroke != pixel(rows, x + 3, y),
            f"Focused {field.label} has no visible separated ring at the measured midpoint")
    return {"bounds": ring, "xy": [x, y], "rgba": stroke}


def shot(app, plan, title, frame, *, targets, focused_field=None, backdrop=None):
    before = onscreen_windows(app.proc.pid)
    receipt = capture(app, plan, title, frame=frame)
    after = onscreen_windows(app.proc.pid)
    require(before == after == 0, f"{title} opened a visible window")
    viewport = frame["layout"]["viewport"]
    require((receipt["width"], receipt["height"]) == (viewport["width"], viewport["height"]),
            "PNG dimensions differ from the native logical viewport")
    _, _, rows = read_png((plan.out / receipt["name"]).read_bytes())
    receipt.update({"visible_windows": [before, after], "focus": focused(frame),
                    "ink": {target: readable_ink(frame, rows, target) for target in targets}})
    if focused_field is not None:
        require(focused(frame) in (focused_field.part("editor"), focused_field.part("adjustable")),
                f"Expected focus on {focused_field.label} during capture")
        receipt["focus_ring"] = field_ring(frame, rows, focused_field)
    if backdrop is not None:
        receipt["scrim"] = scrim_pixels(frame, backdrop, rows)
    print(f"PASS {title}: identical PNG and metadata through API, CLI and MCP", flush=True)
    return receipt


def light(app, plan):
    frame = scene(app)
    outside_count(frame, 0)
    keyboard_focus(app, OUTSIDE)
    frame = key(app, "enter")
    outside_count(frame, 1)
    saved(frame, 1200, "毫升")
    _, _, closed = read_png(png_payload(app.wire("Page.screenshot", SHOT)))
    frame, open_path = open_dialog(app)
    initial = wheels(frame, 1200, 1, "毫升")
    panel = geometry(frame)
    outside_rejected = reject_background(app, frame, 1)
    start = shot(app, plan, "scroll-modal-light-open", frame,
                 targets=[QUANTITY.part("center")], focused_field=QUANTITY, backdrop=closed)
    start_scroll = body_scroll(frame)
    tabs = tab_wrap(app, editing=False)
    unit_path = keyboard_focus(app, UNIT.part("adjustable"))
    frame = settle(app)
    visible(frame, UNIT.part("adjustable"), 48.)
    require(body_scroll(frame) > start_scroll, "Tab did not reveal the stacked Unit wheel")
    require(geometry(frame) == panel, "Body focus reveal moved the pinned footer")
    unit_focus = shot(app, plan, "scroll-modal-light-unit-focus", frame,
                      targets=[UNIT.part("center")], focused_field=UNIT)

    enter_editor(app, QUANTITY, "1200")
    fill(app, QUANTITY, "1204")
    frame = key(app, "enter")
    wheels(frame, 1204, 1, "毫升")
    saved(frame, 1200, "毫升")
    enter_editor(app, UNIT, "毫升")
    fill(app, UNIT, "L")
    frame = key(app, "enter")
    working = wheels(frame, 1204, 2, "公升")
    saved(frame, 1200, "毫升")
    frame, cancellation = cancel(app)
    discarded = saved(frame, 1200, "毫升")
    outside_count(frame, 1)

    frame, _ = open_dialog(app)
    wheels(frame, 1200, 1, "毫升")
    enter_editor(app, QUANTITY, "1200")
    fill(app, QUANTITY, "１２０６")
    enter_editor(app, UNIT, "毫升")
    frame = fill(app, UNIT, "fl oz")
    saved(frame, 1200, "毫升")
    keyboard_focus(app, SAVE)
    closed = assert_closed(key(app, "enter"))
    accepted = saved(closed, 1206, "液量盎司")
    final = shot(app, plan, "scroll-modal-light-saved", closed, targets=[SAVED])
    frame, _ = open_dialog(app)
    reopened = wheels(frame, 1206, 3, "液量盎司")
    frame = key(app, "up")
    abandoned_by_escape = wheels(frame, 1208, 3, "液量盎司")
    saved(frame, 1206, "液量盎司")
    # The opening wheel owns focus, so one semantic Escape reaches the modal
    # rather than retiring composition or cancelling an active editor.
    closed = assert_closed(key(app, "escape"))
    saved(closed, 1206, "液量盎司")
    outside_count(closed, 1)
    return {"initial": initial, "geometry": panel, "opener_tab_path": open_path, "tab_wrap": tabs,
            "background_activation": outside_rejected, "working_child_confirmations": working,
            "accepted_after_cancel": discarded, "cancel": cancellation, "accepted_after_save": accepted,
            "reopened": reopened, "working_before_escape": abandoned_by_escape,
            "escape_focus": focused(closed), "back_focus": back(app),
            "unit_reveal_tab_path": unit_path, "captures": [start, unit_focus, final]}


def rtl(app, plan):
    scene(app, "theme", "rtl")
    frame, _ = open_dialog(app)
    panel = geometry(frame)
    quantity_box, unit_box = (layout(frame, field.id)["bounds"] for field in FIELDS)
    require(bottom(quantity_box) <= unit_box["y"] + 0.51
            and abs(right(quantity_box) - right(unit_box)) <= 0.51,
            "Wide RTL fields must stack in authored order and align at logical start")
    enter_editor(app, QUANTITY, "1200")
    fill(app, QUANTITY, "1203")
    enter_editor(app, UNIT, "毫升")
    frame = fill(app, UNIT, "無")
    saved(frame, 1200, "毫升")
    keyboard_focus(app, SAVE)
    frame = key(app, "enter")
    shell(frame)
    focus(frame, SAVE)
    editor(frame, QUANTITY, "1203", error="請輸入偶數")
    editor(frame, UNIT, "無", error="找不到單位")
    require(node(frame, STATUS)["role"] == "Status" and node(frame, STATUS)["text"] == "請修正數量和單位",
            "Failed Save did not publish a live status for both errors")
    saved(frame, 1200, "毫升")
    keyboard_focus(app, QUANTITY.part("editor"))
    frame = settle(app)
    visible(frame, QUANTITY.part("error"))
    quantity_error = shot(app, plan, "scroll-modal-rtl-quantity-error", frame,
                          targets=[QUANTITY.part("error")], focused_field=QUANTITY)
    keyboard_focus(app, UNIT.part("editor"))
    frame = settle(app)
    visible(frame, UNIT.part("error"))
    first = shot(app, plan, "scroll-modal-rtl-errors", frame,
                 targets=[UNIT.part("error")], focused_field=UNIT)

    keyboard_focus(app, QUANTITY.part("editor"))
    fill(app, QUANTITY, "１２０６")
    keyboard_focus(app, SAVE)
    frame = key(app, "enter")
    shell(frame)
    quantity = editor(frame, QUANTITY, "１２０６")
    unit = editor(frame, UNIT, "無", error="找不到單位")
    focus(frame, SAVE)
    require(node(frame, STATUS)["text"] == "請修正單位", "Status did not clear the fixed quantity error")
    saved(frame, 1200, "毫升")
    keyboard_focus(app, QUANTITY.part("editor"))
    frame = settle(app)
    second = shot(app, plan, "scroll-modal-rtl-valid-peer", frame,
                  targets=[QUANTITY.part("editor")], focused_field=QUANTITY)
    frame, cancelled = cancel(app)
    reset = saved(frame, 1200, "毫升")
    frame, _ = open_dialog(app)
    wheels(frame, 1200, 1, "毫升")
    frame, _ = cancel(app)
    return {"geometry": panel, "rtl_order": {"quantity": quantity_box, "unit": unit_box},
            "valid_peer_after_failed_save": quantity, "remaining_error": unit,
            "cancel": cancelled, "accepted_after_cancel": reset, "back_focus": back(app),
            "captures": [quantity_error, first, second]}


def compact(app, plan):
    scene(app, "theme", "rtl", "large-text")
    frame, _ = open_dialog(app, editing=True)
    panel = geometry(frame)
    cancel_hit, save_hit = (panel["footer"][target] for target in (CANCEL, SAVE))
    require(right(save_hit) <= cancel_hit["x"] + 0.51 and abs(save_hit["y"] - cancel_hit["y"]) <= 0.51,
            "Compact RTL footer did not mirror its actions on the same row")
    first_target, second_target = (field.part("editor") for field in FIELDS)
    editor(frame, QUANTITY, "1200")
    editor(frame, UNIT, "毫升")
    require(bottom(layout(frame, first_target)["bounds"]) <= layout(frame, second_target)["bounds"]["y"],
            "Compact fields did not stack in logical order")
    tabs = tab_wrap(app, editing=True, pinned_geometry=panel)
    frame = settle(app)
    initial_scroll = body_scroll(frame)
    quantity = visible(frame, first_target, 48.)
    first = shot(app, plan, "scroll-modal-compact-quantity", frame,
                 targets=[first_target], focused_field=QUANTITY)
    fill(app, QUANTITY, "１３００")
    path = keyboard_focus(app, second_target)
    frame = settle(app)
    unit_scroll = body_scroll(frame)
    unit = visible(frame, second_target, 48.)
    require(unit_scroll > initial_scroll, "Tab did not reveal the second field by scrolling")
    require(geometry(frame) == panel, "Body scroll moved the dialog or a pinned footer target")
    frame = fill(app, UNIT, "L")
    editor(frame, QUANTITY, "１３００")
    saved(frame, 1200, "毫升")
    second = shot(app, plan, "scroll-modal-compact-unit", frame,
                  targets=[second_target], focused_field=UNIT)
    require(geometry(frame) == panel, "Typing changed the compact footer geometry")
    keyboard_focus(app, SAVE)
    accepted = assert_closed(key(app, "enter"))
    status = saved(accepted, 1300, "公升")
    frame, _ = open_dialog(app, editing=True)
    editor(frame, QUANTITY, "1300")
    editor(frame, UNIT, "公升")
    frame, cancelled = cancel(app)
    saved(frame, 1300, "公升")
    return {"rtl": True, "text_scale": 2, "geometry": panel, "tab_wrap": tabs,
            "quantity_editor": quantity, "unit_editor": unit, "unit_tab_path": path,
            "body_scroll": [initial_scroll, unit_scroll], "accepted_after_save": status,
            "cancel": cancelled, "back_focus": back(app), "captures": [first, second]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
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
        "input": "semantic DevTools Action.key, Action.activate, Action.fill",
        "transports": ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"],
        "limits": ["no native pointer events or real OS input", "no real IME or VoiceOver",
                   "no caller callback count in native DOM", "no projected browser accessibility tree"],
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
        require(app.proc.poll() is not None, f"{name} left a headless Gallery process running")
        report["profiles"][name] = proof
        print(f"PASS {name} {width}x{height}. Zero visible windows.", flush=True)
    require({path.name for path in plan.out.iterdir()} == IMAGE_NAMES, "Native PNG set differs")
    (plan.out / REPORT_NAME).write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
