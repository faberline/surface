"""Verify stock native Gallery FAB and dialog endpoints with no visible windows."""

import argparse
import json
import math
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

SHOW = "dialog-demo-show-transform"
ORDINARY_SHOW = "dialog-demo-show"
BACK = "dialog-demo-back"
FAB = "dialog-demo-fab-open"
SCROLLER = "dialog-demo-fab-preview-scroll"
REGULAR = "dialog-demo-edit-open"
ORDINARY_LAUNCHERS = ("dialog-demo-basic-open", "dialog-demo-alert-open", REGULAR)
SURROUNDING = ("dialog-demo-title", BACK, "dialog-demo-theme", "dialog-demo-rtl",
               "dialog-demo-large-text", "dialog-demo-saved")
EDIT = "dialog-demo-edit"
DISCARD = "dialog-demo-discard"
EDITOR = "dialog-demo-editor"
SAVED = "dialog-demo-saved"
INITIAL = "週末行程"
DRAFT = "秋日散步"
ABANDONED = "待捨棄"

PROFILES = [
    ("compact-dark-rtl-200", 320, 400, ("theme", "rtl", "large-text"), True, True),
    ("compact-light-ltr", 480, 640, (), True, False),
    ("wide-light-basic", 960, 800, (), False, False),
]


def far(bounds, axis):
    return bounds[axis] + bounds[{"x": "width", "y": "height"}[axis]]


def paint_at(rows, x, y):
    return list(rows[y][4 * x:4 * x + 4])


def rgba_close(actual, expected, tolerance=20):
    return actual[3] == expected[3] and max(
        abs(a - b) for a, b in zip(actual[:3], expected[:3])
    ) <= tolerance


def assert_name(frame, target, expected):
    received = node(frame, target)["name"]
    require(received == expected, f"Expected {target} name {expected!r}, got {received!r}")


def absent(frame, *targets):
    for target in targets:
        require(find(frame, target)[1] is None, f"{target} remained mounted")


def check_fab(frame, viewport, rtl):
    button = node(frame, FAB)
    require(button["role"] == "Button" and button["name"] == "新增行程",
            "The FAB lost its accessible button name")
    bounds = visible(frame, FAB, 48.)
    require(bounds["width"] == bounds["height"] == 56., f"Invalid regular FAB size: {bounds}")
    require(far(viewport, "y") - far(bounds, "y") == 24., "FAB bottom margin is not 24 dp")
    logical_margin = bounds["x"] if rtl else far(viewport, "x") - far(bounds, "x")
    require(logical_margin == 24., f"Incorrect FAB logical-end margin: {logical_margin}")
    require(visible(frame, "dialog-demo-fab-anchor") == bounds, "FAB is not seated in its anchor")
    return bounds


def check_clearance(frame, fab):
    absent(frame, *ORDINARY_LAUNCHERS)
    targets = {target: visible(frame, target) for target in SURROUNDING}
    scroller = visible(frame, SCROLLER)
    require(fab["y"] - far(scroller, "y") == 8., "Preview scroller has no 8 dp FAB clearance")
    for target, bounds in targets.items():
        require(same_box(scroller, bounds), f"{target} is clipped by the preview scroller")
        overlap_x = min(far(fab, "x"), far(bounds, "x")) - max(fab["x"], bounds["x"])
        overlap_y = min(far(fab, "y"), far(bounds, "y")) - max(fab["y"], bounds["y"])
        require(overlap_x <= 0. or overlap_y <= 0.,
                f"FAB covers {target}: {(overlap_x, overlap_y)} dp overlap")
    return {"scroller": scroller, "targets": targets}


def capture_fab(app, plan, profile, stage, frame, bounds, sample_targets=("dialog-demo-title", SAVED)):
    shot = capture(app, plan, f"{profile}-{stage}", frame=frame,
                   sample_targets=sample_targets)
    status = node(frame, FAB)
    index, _ = find(frame, FAB)
    face, icon = frame["dom"]["nodes"][index + 1:index + 3]
    require(face["role"] == "Group" and icon["text"] == "+",
            f"Missing regular FAB paint under {status['name']}")
    expected = face["style"]["background"]
    icon_ink = icon["style"]["foreground"]
    _, _, rows = read_png((plan.out / shot["name"]).read_bytes())
    point = (round(bounds["x"] + 9.), round(bounds["y"] + 28.))
    actual = paint_at(rows, *point)
    require(actual == expected, f"FAB body color differs at {point}: {actual} vs {expected}")
    icon_box = frame["layout"]["nodes"][index + 2]["bounds"]
    samples = sum(
        rgba_close(paint_at(rows, x, y), icon_ink, 30)
        for y in range(math.ceil(icon_box["y"]), math.floor(far(icon_box, "y")))
        for x in range(math.ceil(icon_box["x"]), math.floor(far(icon_box, "x")))
    )
    require(samples >= 6, f"FAB icon did not paint visibly ({samples} pixels)")
    shot["fab_paint"] = {"body_point": point, "rgba": actual, "icon_ink_pixels": samples}
    return shot


def check_editor(frame, viewport, fullscreen, rtl):
    focus(frame, EDITOR)
    require(node(frame, EDIT)["role"] == "Dialog", "Editor panel lost its dialog role")
    assert_name(frame, EDIT, "編輯行程")
    assert_name(frame, EDITOR, "行程名稱")
    bounds = visible(frame, EDIT)
    editor = visible(frame, EDITOR, 48.)
    body = visible(frame, f"{EDIT}-body")
    require(same_box(body, editor), "Editable text field is clipped by its body")
    if fullscreen:
        require(bounds == viewport, f"Full-screen panel has wrong bounds: {bounds}")
        absent(frame, f"{EDIT}-cancel", f"{EDIT}-confirm")
        title_id = f"{EDIT}-title"
        close_id = f"{EDIT}-close"
        save_id = f"{EDIT}-action"
        header = visible(frame, f"{EDIT}-header")
        title_bounds = visible(frame, title_id)
        close = visible(frame, close_id, 48.)
        save = visible(frame, save_id, 48.)
        close_icon = visible(frame, f"{close_id}-icon", 24.)
        require(far(header, "y") == body["y"] and same_box(header, close) and same_box(header, save),
                "Close and Save must stay within the pinned header")
        require(close_icon["width"] == close_icon["height"] == 24., "Wrong Close icon size")
        logical_start = (far(header, "x") - far(close_icon, "x")
                         if rtl else close_icon["x"] - header["x"])
        require(logical_start == 24., "Close icon is not 24 dp from logical start")
        ordered = close["x"] > title_bounds["x"] > save["x"] if rtl else close["x"] < title_bounds["x"] < save["x"]
        require(ordered, "Header controls are out of logical order")
        extra = {"header": header, "close_icon": close_icon}
    else:
        require(bounds["width"] == 400. and bounds["height"] < viewport["height"],
                f"Wide fallback is not a centered Basic panel: {bounds}")
        require(abs(2. * bounds["x"] + bounds["width"] - viewport["width"]) <= 1.
                and abs(2. * bounds["y"] + bounds["height"] - viewport["height"]) <= 1.,
                f"Basic fallback is not centered: {bounds}")
        absent(frame, f"{EDIT}-header", f"{EDIT}-action")
        title_id = f"{EDIT}-heading"
        close_id = f"{EDIT}-cancel"
        save_id = f"{EDIT}-confirm"
        title_bounds = visible(frame, title_id)
        close = visible(frame, close_id, 48.)
        save = visible(frame, save_id, 48.)
        require(close["y"] == save["y"] and far(close, "x") <= save["x"],
                "Wide Basic actions are not aligned in a row")
        require(far(title_bounds, "y") <= body["y"] and far(body, "y") <= save["y"],
                "Wide Basic title, body and actions overlap")
        extra = {}
    assert_name(frame, close_id, "關閉")
    assert_name(frame, save_id, "儲存")
    return {
        "presentation": "full-screen" if fullscreen else "basic",
        "panel": bounds, "editor": editor, "body": body, "heading": title_bounds,
        "close": close, "save": save, **extra,
    }, title_id, close_id, save_id


def sample_panel(app, plan, profile, stage, frame, panel_id, title_id, fullscreen):
    shot = capture(app, plan, f"{profile}-{stage}", frame=frame, sample_targets=(title_id,))
    width, height, rows = read_png((plan.out / shot["name"]).read_bytes())
    require((width, height) == (frame["layout"]["viewport"]["width"],
                                frame["layout"]["viewport"]["height"]),
            "Stock capture used unexpected viewport dimensions")
    bounds = layout(frame, panel_id)["bounds"]
    if fullscreen:
        points = [(1, 1), (width - 2, height - 2)]
    else:
        center_y = round(bounds["y"] + bounds["height"] / 2.)
        points = [(math.ceil(bounds["x"] + 8.), center_y),
                  (math.floor(far(bounds, "x") - 8.), center_y)]
    expected = node(frame, panel_id)["style"]["background"]
    require(all(paint_at(rows, x, y) == expected for x, y in points),
            f"Panel canvas fill differs for {panel_id} at {points}")
    shot["panel_paint"] = {"points": points, "rgba": expected}
    return shot


def replay(plan, profile, width, height, options, fullscreen, rtl):
    viewport = {"x": 0., "y": 0., "width": width, "height": height}
    samples = []
    report = {"profile": profile, "viewport": viewport, "options": options}

    def window_sample(app, stage):
        count = onscreen_windows(app.proc.pid)
        require(count == 0, f"Headless Gallery opened {count} windows at {stage}")
        samples.append({"stage": stage, "visible": count})

    with launch(plan, profile, width, height) as app:
        window_sample(app, "startup")
        activate(app, "section-2")
        activated = activate(app, SHOW)
        focus(activated, BACK)
        for choice in options:
            activate(app, f"dialog-demo-{choice}")
        tabs = focus_by_tab(app, FAB)
        closed = settle(app)
        focus(closed, FAB)
        absent(closed, *ORDINARY_LAUNCHERS, EDIT, DISCARD)
        assert_name(closed, SAVED, f"已儲存：{INITIAL}")
        fab_bounds = check_fab(closed, viewport, rtl)
        report["closed"] = {"tab_focus_path": tabs, "fab": fab_bounds,
                            "clearance_targets": check_clearance(closed, fab_bounds),
                            "capture": capture_fab(app, plan, profile, "fab", closed, fab_bounds)}
        window_sample(app, "fab")

        opened = key(app, "enter")
        geometry, title, close, save = check_editor(opened, viewport, fullscreen, rtl)
        require(opened["text_inputs"][EDITOR]["value"] == INITIAL, "Edit did not start from the saved name")
        require(layout(opened, FAB)["bounds"] == fab_bounds, "Open moved the measured FAB anchor")
        app.wire("Action.activate", {"id": FAB}, "outside_modal")
        app.wire("Action.activate", {"id": "dialog-demo-theme"}, "outside_modal")
        report["opened"] = {
            "geometry": geometry, "focus": EDITOR, "background_activation_blocked": True,
            "capture": sample_panel(app, plan, profile, "edit", opened, EDIT, title, fullscreen),
        }
        window_sample(app, "open")
        cancelled = key(app, "escape")
        focus(cancelled, FAB)
        absent(cancelled, EDIT, DISCARD)
        require(check_fab(cancelled, viewport, rtl) == fab_bounds, "Escape moved the FAB anchor")
        assert_name(cancelled, SAVED, f"已儲存：{INITIAL}")

        focus(key(app, "enter"), EDITOR)
        app.wire("Action.fill", {"id": EDITOR, "value": DRAFT})
        draft = settle(app)
        require(draft["text_inputs"][EDITOR]["value"] == DRAFT, "Edit did not retain the draft")
        assert_name(draft, SAVED, f"已儲存：{INITIAL}")
        discard = key(app, "escape")
        focus(discard, f"{DISCARD}-cancel")
        assert_name(discard, DISCARD, "捨棄變更？")
        require(discard["text_inputs"][EDITOR]["value"] == DRAFT, "Nested discard lost the draft")
        require(layout(discard, EDIT)["bounds"] == geometry["panel"], "Nested discard moved the editor panel")
        app.wire("Action.activate", {"id": save}, "outside_modal")
        app.wire("Action.fill", {"id": EDITOR, "value": ABANDONED}, "outside_modal")
        report["discard_prompt"] = {
            "panel": visible(discard, DISCARD), "focus": f"{DISCARD}-cancel",
            "cancel": visible(discard, f"{DISCARD}-cancel", 48.),
            "confirm": visible(discard, f"{DISCARD}-confirm", 48.),
            "underlying_save_blocked": True, "underlying_edit_blocked": True,
            "capture": sample_panel(app, plan, profile, "discard", discard,
                                    DISCARD, f"{DISCARD}-heading", False),
        }
        window_sample(app, "nested")

        resumed = key(app, "enter")
        focus(resumed, EDITOR)
        absent(resumed, DISCARD)
        require(resumed["text_inputs"][EDITOR]["value"] == DRAFT, "Continue did not restore the draft")
        assert_name(resumed, SAVED, f"已儲存：{INITIAL}")
        save_tabs = focus_by_tab(app, save)
        saved = key(app, "enter")
        focus(saved, FAB)
        absent(saved, EDIT, DISCARD)
        assert_name(saved, SAVED, f"已儲存：{DRAFT}")
        app.wire("Action.activate", {"id": save}, "missing_node")

        reopened = key(app, "enter")
        focus(reopened, EDITOR)
        require(reopened["text_inputs"][EDITOR]["value"] == DRAFT, "Reopen lost the saved value")
        app.wire("Action.fill", {"id": EDITOR, "value": ABANDONED})
        focus(activate(app, close), f"{DISCARD}-cancel")
        from_close = key(app, "enter")
        focus(from_close, close)
        absent(from_close, DISCARD)
        require(from_close["text_inputs"][EDITOR]["value"] == ABANDONED,
                "Continue from the Close button did not retain the draft")
        focus(activate(app, close), f"{DISCARD}-cancel")
        focus_by_tab(app, f"{DISCARD}-confirm")
        restored = key(app, "enter")
        focus(restored, FAB)
        absent(restored, EDIT, DISCARD)
        assert_name(restored, SAVED, f"已儲存：{DRAFT}")
        restored_bounds = check_fab(restored, viewport, rtl)
        require(restored_bounds == fab_bounds, "Closing moved the FAB anchor")
        report["restored"] = {
            "fab": restored_bounds, "focus": FAB, "discarded": ABANDONED,
            "saved": DRAFT, "save_tab_focus_path": save_tabs,
            "continue_from_escape_focus": EDITOR, "continue_from_close_focus": close,
            "clearance_targets": check_clearance(restored, restored_bounds),
            "capture": capture_fab(app, plan, profile, "restored", restored, restored_bounds),
        }
        window_sample(app, "restored")
        focus(activate(app, BACK), SHOW)
        absent(settle(app), FAB, EDIT, DISCARD)
        ordinary = activate(app, ORDINARY_SHOW)
        focus(ordinary, BACK)
        for launcher in ORDINARY_LAUNCHERS:
            visible(ordinary, launcher, 48.)
        absent(ordinary, FAB, EDIT, DISCARD)
        focus(activate(app, BACK), ORDINARY_SHOW)
        report["ordinary_route_unchanged"] = True
        window_sample(app, "back")
    report["visible_windows"] = samples
    return report


def replay_short(plan):
    profile = "short-dark-rtl-200"
    viewport = {"x": 0., "y": 0., "width": 320, "height": 240}
    lower_control = "dialog-demo-large-text"
    windows = []

    def sample_window(app, stage):
        count = onscreen_windows(app.proc.pid)
        require(count == 0, f"Headless Gallery opened a visible window at {stage}")
        windows.append({"stage": stage, "visible": count})

    with launch(plan, profile, 320, 240) as app:
        sample_window(app, "startup")
        activate(app, "section-2")
        # Set the shared preferences before mounting the preview. Toggling
        # them inside the short scroller would reveal its last control first.
        activate(app, ORDINARY_SHOW)
        for option in ("theme", "rtl", "large-text"):
            activate(app, f"dialog-demo-{option}")
        focus(activate(app, BACK), ORDINARY_SHOW)
        initial = activate(app, SHOW)
        focus(initial, BACK)
        absent(initial, *ORDINARY_LAUNCHERS)
        fab = check_fab(initial, viewport, True)
        scroller = visible(initial, SCROLLER)
        require(fab["y"] - far(scroller, "y") == 8., "Short preview scroller reaches the FAB")
        initial_saved = layout(initial, SAVED)["bounds"]
        initial_control = layout(initial, lower_control)["bounds"]
        require(far(initial_saved, "y") > far(scroller, "y"), "Short preview did not overflow")
        require(far(initial_control, "y") > far(scroller, "y"),
                "Expected the lower setting to start below the viewport clip")
        tab_scroll = focus_by_tab(app, lower_control)
        revealed = settle(app)
        focus(revealed, lower_control)
        revealed_bounds = visible(revealed, lower_control, 48.)
        require(same_box(scroller, revealed_bounds), "Tab failed to reveal the lower control")
        require(layout(revealed, SAVED)["bounds"]["y"] < initial_saved["y"],
                "Tab reached the lower control without scrolling its preview")
        require(check_fab(revealed, viewport, True) == fab, "Scrolling moved the FAB")
        sample_window(app, "tab-reveal")
        focus_by_tab(app, FAB)
        preview = settle(app)
        focus(preview, FAB)
        fab_shot = capture_fab(app, plan, profile, "fab-after-tab", preview, fab, sample_targets=())
        opened = key(app, "enter")
        focus(opened, EDITOR)
        require(visible(opened, EDIT) == viewport, "Short editor did not fill the viewport")
        assert_name(opened, EDIT, "編輯行程")
        require(opened["text_inputs"][EDITOR]["value"] == INITIAL, "Short editor lost its value")
        edit_shot = sample_panel(app, plan, profile, "edit", opened, EDIT, f"{EDIT}-title", True)
        sample_window(app, "edit")
        restored = key(app, "escape")
        focus(restored, FAB)
        absent(restored, EDIT, DISCARD)
        require(check_fab(restored, viewport, True) == fab, "Closing the short editor moved the FAB")
        sample_window(app, "restored")
    return {
        "profile": profile, "viewport": viewport, "scroller": scroller, "fab": fab,
        "before_focus": BACK, "before_lower_control": initial_control,
        "tab_scroll_path": tab_scroll, "after_lower_control": revealed_bounds,
        "saved_y_before": initial_saved["y"], "saved_y_after": layout(revealed, SAVED)["bounds"]["y"],
        "captures": [fab_shot, edit_shot], "visible_windows": windows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ("gallery", "cli", "out"):
        parser.add_argument(f"--{flag}", required=True, type=Path)
    args = parser.parse_args()
    gallery, cli, out = (args.gallery.resolve(strict=True), args.cli.resolve(strict=True),
                         args.out.resolve())
    require(out.is_relative_to(Path("/tmp").resolve()), "Use a private /tmp evidence directory")
    out.mkdir()
    pins = {"gallery": digest(gallery), "cli": digest(cli)}
    plan = SimpleNamespace(gallery=gallery, cli=cli, out=out)
    profiles = [replay(plan, *config) for config in PROFILES]
    short = replay_short(plan)
    require(pins == {"gallery": digest(gallery), "cli": digest(cli)}, "Native binaries changed")
    report = {
        "schema": "surface.test.full-screen-dialog-fab-native.v1",
        "binaries": pins, "motion_mode": "stock headless Reduce Motion",
        "transport": ["Page.screenshot", "CLI", "MCP"], "profiles": profiles,
        "short_viewport": short,
    }
    (out / "native-full-screen-dialog-report.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    )
    print("PASS 14 PNGs, all three transports, 22 zero-window samples, three FAB journeys, and short Tab clearance")


if __name__ == "__main__":
    main()
