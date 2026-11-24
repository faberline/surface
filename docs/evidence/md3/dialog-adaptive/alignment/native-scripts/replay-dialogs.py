"""Reproduce the Material Dialog Gallery journey with offscreen Metal."""

import argparse
import json
import math
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

SHOW = "dialog-demo-show"
BASIC = "dialog-demo-basic"
ALERT = "dialog-demo-alert"
EDIT = "dialog-demo-edit"
DISCARD = "dialog-demo-discard"
EDITOR = "dialog-demo-editor"
SAVED = "dialog-demo-saved"
INITIAL = "週末行程"
DRAFT = "秋日散步"
SECOND_DRAFT = "待捨棄"


def box_end(bounds, axis):
    return bounds[axis] + bounds["width" if axis == "x" else "height"]


def side_by_side(cancel, confirm, rtl):
    require(cancel["y"] == confirm["y"], "Wide Basic actions should share a row")
    require(box_end(confirm, "x") <= cancel["x"] if rtl else box_end(cancel, "x") <= confirm["x"],
            "Basic confirm is not at logical end")


def name_is(frame, target, name):
    require(node(frame, target)["name"] == name, f"Unexpected name for {target}")


def open_by_keyboard(app, panel):
    steps = focus_by_tab(app, f"{panel}-open")
    frame = key(app, "enter")
    require(find(frame, panel)[1] is not None, f"Keyboard did not open {panel}")
    return frame, len(steps)


def color_at(rows, x, y):
    return list(rows[y][4 * x:4 * x + 4])


def sample_fill(frame, rows, panel, covers_window):
    paint = node(frame, panel)["style"]["background"]
    bounds = layout(frame, panel)["bounds"]
    center_y = round(bounds["y"] + bounds["height"] / 2.)
    if covers_window:
        points = [(1, 1), (len(rows[0]) // 4 - 2, len(rows) - 2)]
    else:
        points = [(math.ceil(bounds["x"] + 8), center_y),
                  (math.floor(box_end(bounds, "x") - 8), center_y)]
    for x, y in points:
        require(color_at(rows, x, y) == paint, f"{panel} canvas fill differs at {(x, y)}")
    return {"rgba": paint, "points": points}


def sample_action_captions(frame, rows, info):
    result = {}
    for target in info:
        index, button = find(frame, target)
        require(button is not None and button["role"] == "Button", f"Missing action {target}")
        # Basic Dialog authors each passive measured caption immediately after
        # its semantic button. The Full-screen caption is independently named.
        if target.endswith("-action"):
            index, label = find(frame, f"{target}-label")
        else:
            index += 1
            label = frame["dom"]["nodes"][index]
        caption = frame["layout"]["nodes"][index]
        bounds = caption["bounds"]
        require(label["text"] == button["name"] and same_box(caption["clip"], bounds)
                and same_box(info[target], bounds), f"Measured caption is clipped for {target}")
        foreground = label["style"]["foreground"]
        # Use a tight color tolerance so a dark or light background cannot
        # count as text. Measure the actual action ink in its measured slot.
        ys = range(max(0, math.ceil(bounds["y"])), min(len(rows), math.floor(box_end(bounds, "y"))))
        xs = range(max(0, math.ceil(bounds["x"])), min(len(rows[0]) // 4, math.floor(box_end(bounds, "x"))))
        count = sum(
            pixel[3] == 255 and max(abs(a - b) for a, b in zip(pixel[:3], foreground[:3])) <= 20
            for y in ys for x in xs for pixel in [color_at(rows, x, y)]
        )
        require(count >= 12, f"No action ink in {target} measured caption")
        result[target] = {"caption": label["text"], "bounds": bounds, "near_color_pixels": count}
    return result


def sample_aligned_lines(frame, rows, target, compact):
    typography = node(frame, target)["style"]
    measured = layout(frame, target)
    bounds, clip = measured["bounds"], measured["clip"]
    authored_alignment = node(frame, target).get("text_alignment")
    require(authored_alignment == ("Right" if compact else "Left"),
            f"Unexpected authored alignment for {target}")
    text_scale = 2 if compact else 1
    line_height = (typography["line_height"] or typography["font_size"] * 1.5) * text_scale
    # Full-width CJK punctuation can carry ink on only half its advance.
    tolerance = math.ceil(typography["font_size"] * text_scale / 2) if compact and (
        target.endswith("-supporting") or target.startswith(DISCARD)
    ) else 7
    foreground = typography["foreground"]
    x_lo = math.ceil(max(bounds["x"], clip["x"]))
    x_hi = math.floor(min(box_end(bounds, "x"), box_end(clip, "x")))
    bottom = min(box_end(bounds, "y"), box_end(clip, "y"))
    lines = []
    for line in range(math.ceil(bounds["height"] / line_height)):
        start = bounds["y"] + line * line_height
        first = math.ceil(max(start, clip["y"]))
        last = math.floor(min(start + line_height, bottom))
        if last <= first:
            continue
        columns = {
            x for x in range(x_lo, x_hi) for y in range(first, last)
            if max(abs(a - b) for a, b in zip(color_at(rows, x, y)[:3], foreground[:3])) <= 24
        }
        require(len(columns) >= 4, f"No text ink in {target} row {line}")
        left, right = min(columns), max(columns)
        gap = x_hi - 1 - right if compact else left - x_lo
        require(gap <= tolerance, f"{target} row {line} ink is not aligned at logical start: {gap}px")
        lines.append({"ink_left": left, "ink_right": right, "logical_gap": gap})
    require(lines, f"No visible lines for {target}")
    return {"alignment": authored_alignment, "bounds": bounds, "lines": lines}


def panel_capture(app, plan, profile, stage, panel, frame, title, actions, covers_window=False):
    shot = capture(app, plan, f"{profile}-{stage}", frame=frame, sample_targets=(title,))
    _, _, rows = read_png((plan.out / shot["name"]).read_bytes())
    shot["surface"] = sample_fill(frame, rows, panel, covers_window)
    shot["actions"] = sample_action_captions(frame, rows, actions)
    compact = profile.startswith("compact")
    shot["text_alignment"] = {title: sample_aligned_lines(frame, rows, title, compact)}
    if panel == ALERT:
        supporting = f"{ALERT}-supporting"
        shot["text_alignment"][supporting] = sample_aligned_lines(frame, rows, supporting, compact)
    if compact and panel == EDIT:
        first, second = shot["text_alignment"][title]["lines"]
        require(first["ink_right"] - first["ink_left"] > second["ink_right"] - second["ink_left"]
                and abs(first["ink_right"] - second["ink_right"]) <= 7,
                "Unequal Full-screen title lines did not align individually")
    return shot


def check_basic(frame, compact):
    name_is(frame, BASIC, "行程通知")
    name_is(frame, f"{BASIC}-heading", "行程通知")
    focus(frame, f"{BASIC}-confirm")
    body = visible(frame, f"{BASIC}-body")
    heading = visible(frame, f"{BASIC}-heading")
    cancel = visible(frame, f"{BASIC}-cancel", 48.)
    confirm = visible(frame, f"{BASIC}-confirm", 48.)
    require(box_end(heading, "y") <= body["y"] and box_end(body, "y") <= min(cancel["y"], confirm["y"]),
            "Basic title, scrolling body and actions must occupy separate regions")
    tail = layout(frame, f"{BASIC}-offline")
    tail_visible_height = max(0., min(box_end(tail["bounds"], "y"), box_end(tail["clip"], "y"))
                              - max(tail["bounds"]["y"], tail["clip"]["y"]))
    if compact:
        require(box_end(tail["bounds"], "y") > box_end(body, "y")
                and tail_visible_height < tail["bounds"]["height"], f"Compact Basic did not clip its scroll body: {tail}")
        require(box_end(confirm, "y") <= cancel["y"] and confirm["x"] == cancel["x"],
                "Compact RTL Basic must stack confirm above cancel at logical end")
    else:
        side_by_side(cancel, confirm, False)
    return {"panel": visible(frame, BASIC), "body": body, "heading": heading,
            "tail_bounds": tail["bounds"], "tail_clip": tail["clip"], "tail_visible_height": tail_visible_height,
            "actions": {f"{BASIC}-cancel": cancel, f"{BASIC}-confirm": confirm}}


def check_edit(frame, compact, viewport):
    name_is(frame, EDIT, "編輯行程")
    name_is(frame, EDITOR, "行程名稱")
    focus(frame, EDITOR)
    panel = visible(frame, EDIT)
    body = visible(frame, f"{EDIT}-body")
    field = visible(frame, EDITOR, 48.)
    require(same_box(body, field), "The editor is not fully visible in its scroll body")
    if compact:
        require(panel == viewport, "The compact editor does not fill the viewport")
        require(find(frame, f"{EDIT}-cancel")[1] is None, "Compact edit has a Basic action row")
        header = visible(frame, f"{EDIT}-header")
        title = visible(frame, f"{EDIT}-title")
        close = visible(frame, f"{EDIT}-close", 48.)
        action = visible(frame, f"{EDIT}-action", 48.)
        icon = visible(frame, f"{EDIT}-close-icon", 24.)
        require(box_end(header, "y") == body["y"] and same_box(header, close) and same_box(header, action),
                "Compact header controls are not pinned above the body")
        require(icon["width"] == icon["height"] == 24. and box_end(header, "x") - box_end(icon, "x") == 24.,
                "RTL close icon is not 24 dp from logical start")
        require(close["x"] > title["x"] > action["x"], "Compact header has incorrect RTL order")
        return {"panel": panel, "body": body, "field": field, "header": header, "close": close,
                "close_icon": icon, "actions": {f"{EDIT}-action": action}}, f"{EDIT}-title", f"{EDIT}-action"
    require(panel["width"] < viewport["width"] and panel["height"] < viewport["height"],
            "Wide edit did not use a centered Basic panel")
    heading = visible(frame, f"{EDIT}-heading")
    cancel = visible(frame, f"{EDIT}-cancel", 48.)
    confirm = visible(frame, f"{EDIT}-confirm", 48.)
    side_by_side(cancel, confirm, False)
    require(box_end(heading, "y") <= body["y"] and box_end(body, "y") <= confirm["y"],
            "Wide edit body is outside its heading and action row")
    return {"panel": panel, "body": body, "field": field,
            "actions": {f"{EDIT}-cancel": cancel, f"{EDIT}-confirm": confirm}}, f"{EDIT}-heading", f"{EDIT}-confirm"


def replay(plan, compact):
    profile = "compact-dark-rtl-200" if compact else "wide-light-ltr"
    width, height = (320, 400) if compact else (960, 800)
    viewport = {"x": 0., "y": 0., "width": width, "height": height}
    windows = []
    report = {"profile": profile, "viewport": viewport}

    def count(app):
        total = onscreen_windows(app.proc.pid)
        require(total == 0, "Headless Gallery opened a visible window")
        windows.append(total)

    with launch(plan, profile, width, height) as app:
        count(app)
        activate(app, "section-2")
        activate(app, SHOW)
        if compact:
            for option in ("theme", "rtl", "large-text"):
                activate(app, f"dialog-demo-{option}")
        name_is(settle(app), SAVED, f"已儲存：{INITIAL}")

        frame, tabs = open_by_keyboard(app, BASIC)
        basic = check_basic(frame, compact)
        shot = panel_capture(app, plan, profile, "basic", BASIC, frame, f"{BASIC}-heading", basic["actions"])
        report["basic"] = {"tab_steps": tabs, "geometry": basic, "capture": shot}
        count(app)
        frame = key(app, "escape")
        focus(frame, f"{BASIC}-open")
        require(find(frame, BASIC)[1] is None, "Escape did not dismiss Basic")

        frame, tabs = open_by_keyboard(app, ALERT)
        name_is(frame, ALERT, "連線中斷")
        require(node(frame, ALERT).get("alert_dialog") is True, "Alert semantic metadata is absent")
        focus(frame, f"{ALERT}-confirm")
        frame = key(app, "escape")
        require(find(frame, ALERT)[1] is not None, "Escape dismissed the action-only alert")
        require(find(frame, f"{ALERT}-cancel")[1] is None, "Action-only alert has a cancel button")
        focus(frame, f"{ALERT}-confirm")
        app.wire("Action.activate", {"id": f"{EDIT}-open"}, "outside_modal")
        action = {f"{ALERT}-confirm": visible(frame, f"{ALERT}-confirm", 48.)}
        shot = panel_capture(app, plan, profile, "alert", ALERT, frame, f"{ALERT}-heading", action)
        report["alert"] = {"tab_steps": tabs, "panel": visible(frame, ALERT), "capture": shot,
                           "escape_keeps_open": True, "stock_alert_metadata": True}
        count(app)
        frame = key(app, "enter")
        focus(frame, f"{ALERT}-open")
        require(find(frame, ALERT)[1] is None, "Explicit acknowledgement left alert mounted")

        frame, tabs = open_by_keyboard(app, EDIT)
        require(frame["text_inputs"][EDITOR]["value"] == INITIAL, "Edit did not start from saved name")
        edit, title, save = check_edit(frame, compact, viewport)
        shot = panel_capture(app, plan, profile, "edit", EDIT, frame, title, edit["actions"], compact)
        report["edit"] = {"tab_steps": tabs, "geometry": edit, "capture": shot}
        count(app)

        app.wire("Action.fill", {"id": EDITOR, "value": DRAFT})
        frame = settle(app)
        focus(frame, EDITOR)
        require(frame["text_inputs"][EDITOR]["value"] == DRAFT, "Edit did not retain changed text")
        name_is(frame, SAVED, f"已儲存：{INITIAL}")
        frame = key(app, "escape")
        focus(frame, f"{DISCARD}-cancel")
        name_is(frame, DISCARD, "捨棄變更？")
        require(frame["text_inputs"][EDITOR]["value"] == DRAFT, "Nested dialog discarded the draft")
        app.wire("Action.activate", {"id": save}, "outside_modal")
        actions = {f"{DISCARD}-{suffix}": visible(frame, f"{DISCARD}-{suffix}", 48.)
                   for suffix in ("cancel", "confirm")}
        shot = panel_capture(app, plan, profile, "discard", DISCARD, frame, f"{DISCARD}-heading", actions)
        report["discard_confirmation"] = {"geometry": visible(frame, DISCARD), "capture": shot,
                                           "underlying_save_blocked": True}
        count(app)

        frame = key(app, "enter")
        focus(frame, EDITOR)
        require(find(frame, DISCARD)[1] is None and frame["text_inputs"][EDITOR]["value"] == DRAFT,
                "Continue did not return to the previous draft")
        require(node(frame, SAVED)["name"] == f"已儲存：{INITIAL}", "Continue unexpectedly committed")
        focus_by_tab(app, save)
        frame = key(app, "enter")
        focus(frame, f"{EDIT}-open")
        name_is(frame, SAVED, f"已儲存：{DRAFT}")
        require(find(frame, EDIT)[1] is None and find(frame, DISCARD)[1] is None, "Save left modal mounted")
        app.wire("Action.activate", {"id": save}, "missing_node")

        frame = key(app, "enter")
        focus(frame, EDITOR)
        require(frame["text_inputs"][EDITOR]["value"] == DRAFT, "Saved name did not persist on reopen")
        app.wire("Action.fill", {"id": EDITOR, "value": SECOND_DRAFT})
        focus(key(app, "escape"), f"{DISCARD}-cancel")
        focus_by_tab(app, f"{DISCARD}-confirm")
        frame = key(app, "enter")
        focus(frame, f"{EDIT}-open")
        name_is(frame, SAVED, f"已儲存：{DRAFT}")
        require(find(frame, EDIT)[1] is None and find(frame, DISCARD)[1] is None, "Discard left modal mounted")
        count(app)
        report["transaction"] = {"draft_retained_on_continue": DRAFT, "saved_after_reopen": DRAFT,
                                 "discarded": SECOND_DRAFT, "save_returns_focus": f"{EDIT}-open",
                                 "discard_returns_focus": f"{EDIT}-open"}
    report["visible_windows"] = windows
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for argument in ("gallery", "cli", "out"):
        parser.add_argument(f"--{argument}", type=Path, required=True)
    args = parser.parse_args()
    gallery, cli = args.gallery.resolve(strict=True), args.cli.resolve(strict=True)
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use private /tmp evidence")
    out.mkdir(parents=True)
    pins = {"gallery": digest(gallery), "cli": digest(cli)}
    plan = SimpleNamespace(gallery=gallery, cli=cli, out=out)
    profiles = [replay(plan, compact) for compact in (False, True)]
    require(pins == {"gallery": digest(gallery), "cli": digest(cli)}, "Native binaries changed")
    report = {"schema": "surface.test.material-dialogs-native.v1", "binaries": pins,
              "transport": ["Page.screenshot", "CLI", "MCP"], "profiles": profiles}
    (out / "native-dialog-report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    print("PASS eight PNGs, matching screenshot transports, actual title/action ink, and twelve zero-window probes")


if __name__ == "__main__":
    main()
