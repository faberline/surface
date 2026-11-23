#!/usr/bin/env python3
"""Verify Expressive Time Input on the native headless Gallery."""

import argparse
import json
import math
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, launch, same_box
from window_check import onscreen_windows

DEMO = "time-input-demo"
INLINE = "time-input-inline"
MODAL = "time-input-modal"
OPEN = DEMO + "-modal-open"
OUTSIDE = DEMO + "-outside"
SHOT = {"scale": 1, "max_side": 1200}


def find(frame, target):
    return next(
        ((i, node) for i, node in enumerate(frame["dom"]["nodes"]) if node.get("stable_id") == target),
        (None, None),
    )


def node(frame, target):
    _, match = find(frame, target)
    require(match is not None, f"Missing DOM node {target}")
    return match


def layout(frame, target):
    index, match = find(frame, target)
    require(match is not None and frame.get("layout") is not None, f"Missing layout for {target}")
    return frame["layout"]["nodes"][index]


def settle(app):
    prior = app.dom()
    for _ in range(24):
        current = app.dom()
        if prior == current:
            return current
        prior = current
    raise RuntimeError("The Gallery layout did not settle")


def activate(app, target):
    app.activate(target)
    return settle(app)


def key(app, name, *, reverse=False):
    app.wire("Action.key", {"key": name, "extend": reverse})
    return settle(app)


def focus(frame, target):
    require(focused(frame) == target, f"Expected focus {target}. Got {focused(frame)}")


def visible(frame, target, minimum=1.):
    measured = layout(frame, target)
    bounds = measured["bounds"]
    require(bounds["width"] >= minimum and bounds["height"] >= minimum, f"Small target {target}: {bounds}")
    require(same_box(measured["clip"], bounds), f"Clipped target {target}: {measured}")
    return bounds


def value(frame, owner, field, expected=None):
    target = f"{owner}-{field}-input"
    surface_value = node(frame, target).get("text")
    require(frame["text_inputs"][target]["value"] == surface_value, f"Editor value differs at {target}")
    if expected is not None:
        require(expected == surface_value, f"Expected {target} = {expected!r}. Got {surface_value!r}")
    return surface_value


def fill(app, owner, field, raw):
    target = f"{owner}-{field}-input"
    app.wire("Action.fill", {"id": target, "value": raw})
    frame = settle(app)
    focus(frame, target)
    value(frame, owner, field, raw)
    return frame


def finish(app, owner, field, raw, normalized):
    fill(app, owner, field, raw)
    frame = key(app, "enter")
    focus(frame, f"{owner}-{field}-input")
    value(frame, owner, field, normalized)
    return frame


def field_description(frame, owner, field, *, error=False):
    expected = f"{owner}-{field}-{'error' if error else 'hint'}"
    field_node = node(frame, f"{owner}-{field}-input")
    require(field_node["role"] == "TextInput", f"Wrong role for {owner}-{field}")
    require(field_node.get("described_by_text") == expected, f"Expected the {field} description {expected}")
    description = node(frame, expected)
    text = description.get("text") or description["name"]
    require(bool(text.strip()), f"Missing description text for {owner}-{field}")
    return text


def label(frame, owner, field, expected):
    item = node(frame, f"{owner}-{field}-label")
    require((item.get("text") or item["name"]) == expected, f"Wrong label for {owner}-{field}")


def saved(frame, kind, time):
    expected = f"已儲存 {time}"
    status = node(frame, f"{DEMO}-{kind}-value")
    require((status.get("text") or status["name"]) == expected, f"Expected {expected}, got {status}")


def scene(app):
    app.activate("section-2")
    frame = activate(app, DEMO + "-show")
    saved(frame, "inline", "00:05")
    saved(frame, "modal", "09:07")
    return frame


def ink_sample(frame, rows, target):
    bounds = visible(frame, target)
    color = bytes(node(frame, target)["style"]["foreground"])
    left = max(0, math.ceil(bounds["x"]))
    right = min(len(rows[0]) // 4, math.floor(bounds["x"] + bounds["width"]))
    top = max(0, math.ceil(bounds["y"]))
    bottom = min(len(rows), math.floor(bounds["y"] + bounds["height"]))
    tolerance = 64

    def near_ink(pixel):
        return pixel[3] == color[3] and max(abs(a - b) for a, b in zip(pixel[:3], color[:3])) <= tolerance

    points = [(x, y) for y in range(top, bottom) for x in range(left, right)
              if near_ink(rows[y][4*x:4*x+4])]
    require(len(points) >= 16, f"No readable ink in {target}")
    xs, ys = zip(*points)
    ink = [min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1]
    require(ink[2] >= 4 and ink[3] >= 6, f"Too little visible ink in {target}: {ink}")
    return {"bounds": bounds, "ink_bounds": ink, "rgba": list(color),
            "rgb_tolerance": tolerance, "ink_pixels": len(points)}


def capture(app, plan, title, *, frame=None, sample_targets=()):
    path = plan.out / f"{title}.png"
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    api = app.wire("Page.screenshot", SHOT)
    mcp, mcp_metadata, _ = app.mcp(SHOT)
    encoded = path.read_bytes()
    require(encoded == png_payload(api) == mcp, f"PNG bytes differ for {title}")
    require(metadata(cli) == metadata(api) == mcp_metadata, f"PNG metadata differs for {title}")
    width, height, rows = read_png(encoded)
    require((width, height) == (cli["width"], cli["height"]), "Decoded PNG dimensions disagree")
    result = {"name": path.name, "sha256": digest(path), **metadata(cli)}
    if sample_targets:
        result["ink"] = {target: ink_sample(frame, rows, target) for target in sample_targets}
    return result


def tab_to(app, target, *, reverse=False):
    steps = []
    for _ in range(8):
        frame = key(app, "tab", reverse=reverse)
        steps.append(focused(frame))
        if focused(frame) == target:
            return frame, steps
    raise RuntimeError(f"Tab missed {target}: {steps}")


def period(frame, owner, selected):
    require(node(frame, owner + "-period")["role"] == "RadioGroup", "Expected one period radio group")
    for choice, name in (("am", "上午"), ("pm", "下午")):
        radio = node(frame, f"{owner}-{choice}")
        require(radio["role"] == "Radio" and radio["name"] == name, f"Wrong period semantics for {choice}")
        require(radio.get("checked") is (choice == selected), f"Incorrect checked state for {choice}")


def inline(app, plan):
    frame = scene(app)
    label(frame, INLINE, "hour", "小時")
    label(frame, INLINE, "minute", "分鐘")
    period(frame, INLINE, "am")
    value(frame, INLINE, "hour", "12")
    value(frame, INLINE, "minute", "05")
    hints = {field: field_description(frame, INLINE, field) for field in ("hour", "minute")}
    fill(app, INLINE, "hour", "１２")
    frame = key(app, "tab")
    focus(frame, INLINE + "-minute-input")
    value(frame, INLINE, "hour", "12")
    frame = finish(app, INLINE, "minute", "５", "05")
    period(frame, INLINE, "am")
    saved(frame, "inline", "00:05")
    image = capture(app, plan, "inline-light-midnight")

    frame, path = tab_to(app, INLINE + "-am")
    focus(frame, INLINE + "-am")
    am = visible(frame, INLINE + "-am")
    pm = visible(frame, INLINE + "-pm")
    dx, dy = pm["x"] - am["x"], pm["y"] - am["y"]
    arrow = ("right" if dx > 0 else "left") if abs(dx) >= abs(dy) else ("down" if dy > 0 else "up")
    frame = key(app, arrow)
    focus(frame, INLINE + "-pm")
    frame = key(app, "space")
    period(frame, INLINE, "pm")
    value(frame, INLINE, "hour", "12")
    value(frame, INLINE, "minute", "05")
    saved(frame, "inline", "00:05")
    frame = activate(app, DEMO + "-inline-submit")
    saved(frame, "inline", "12:05")
    saved(frame, "modal", "09:07")
    return {"roundtrip_digits": ["１２", "５"], "hints": hints, "period_tab_path": path, "period_arrow": arrow,
            "midnight": "00:05", "noon_after_submit": "12:05", "image": image}


def open_modal(app):
    frame = activate(app, OPEN)
    focus(frame, MODAL + "-hour-input")
    require(node(frame, MODAL)["role"] == "Dialog", "The modal Time Input is not a dialog")
    return frame


def closed(frame, time):
    focus(frame, OPEN)
    require(find(frame, MODAL)[1] is None, "The Time Input dialog is still open")
    saved(frame, "modal", time)


def wide_modal(app, plan):
    scene(app)
    activate(app, DEMO + "-format")
    frame = open_modal(app)
    panel = visible(frame, MODAL)
    require(find(frame, MODAL + "-period")[1] is None, "The 24-hour modal has a period group")
    for suffix in ("-am", "-pm"):
        require(find(frame, MODAL + suffix)[1] is None, f"The 24-hour modal has {suffix}")
    value(frame, MODAL, "hour", "09")
    value(frame, MODAL, "minute", "07")
    require(field_description(frame, MODAL, "hour") != field_description(frame, MODAL, "minute"),
            "Hour and Minute have the same hint")
    app.wire("Action.activate", {"id": OUTSIDE}, expected_error="outside_modal")
    focus(settle(app), MODAL + "-hour-input")

    frame = finish(app, MODAL, "hour", "24", "24")
    hour_error = field_description(frame, MODAL, "hour", error=True)
    value(frame, MODAL, "minute", "07")
    field_description(frame, MODAL, "minute")
    frame = finish(app, MODAL, "minute", "60", "60")
    minute_error = field_description(frame, MODAL, "minute", error=True)
    require(field_description(frame, MODAL, "hour", error=True) == hour_error,
            "Editing Minute changed the Hour error")
    frame = finish(app, MODAL, "minute", "５９", "59")
    value(frame, MODAL, "hour", "24")
    require(field_description(frame, MODAL, "hour", error=True) == hour_error,
            "Fixing Minute changed the Hour error")
    field_description(frame, MODAL, "minute")
    saved(frame, "modal", "09:07")
    invalid_image = capture(app, plan, "modal-light-independent-error")
    _, invalid_path = tab_to(app, MODAL + "-confirm")
    frame = key(app, "enter")
    focus(frame, MODAL + "-confirm")
    saved(frame, "modal", "09:07")
    field_description(frame, MODAL, "hour", error=True)

    frame = finish(app, MODAL, "hour", "２３", "23")
    value(frame, MODAL, "minute", "59")
    field_description(frame, MODAL, "hour")
    field_description(frame, MODAL, "minute")
    saved(frame, "modal", "09:07")
    valid_image = capture(app, plan, "modal-light-valid")
    _, confirm_path = tab_to(app, MODAL + "-confirm")
    frame = key(app, "enter")
    closed(frame, "23:59")

    open_modal(app)
    frame = finish(app, MODAL, "hour", "8", "08")
    saved(frame, "modal", "23:59")
    _, cancel_path = tab_to(app, MODAL + "-cancel")
    frame = key(app, "enter")
    closed(frame, "23:59")

    activate(app, DEMO + "-locale")
    frame = open_modal(app)
    require(node(frame, MODAL)["name"] == "時刻を入力", "Wrong localized dialog name")
    require(node(frame, MODAL + "-title")["text"] == "時刻を入力", "Wrong localized dialog title")
    label(frame, MODAL, "hour", "時")
    label(frame, MODAL, "minute", "分")
    require(field_description(frame, MODAL, "hour") == "０–２３", "Wrong localized Hour help")
    require(field_description(frame, MODAL, "minute") == "０–５９", "Wrong localized Minute help")
    value(frame, MODAL, "hour", "２３")
    value(frame, MODAL, "minute", "５９")
    finish(app, MODAL, "hour", "9", "０９")
    frame = finish(app, MODAL, "minute", "７", "０７")
    value(frame, MODAL, "hour", "０９")
    saved(frame, "modal", "23:59")
    japanese = capture(
        app, plan, "modal-light-japanese", frame=frame,
        sample_targets=tuple(f"{MODAL}-{name}" for name in (
            "title", "hour-label", "minute-label", "hour-input", "minute-input", "hour-hint", "minute-hint",
        )),
    )
    frame = key(app, "escape")
    closed(frame, "23:59")
    return {"panel": panel, "initial": "09:07", "saved": "23:59",
            "hour_error": hour_error, "minute_error": minute_error,
            "confirm_tab_path": confirm_path, "invalid_confirm_tab_path": invalid_path,
            "cancel_tab_path": cancel_path, "outside_semantic_action_rejected": True,
            "japanese_normalized_draft": ["０９", "０７"], "focus_restored": OPEN,
            "images": [invalid_image, valid_image, japanese]}


def compact(app, plan):
    scene(app)
    for suffix in ("theme", "rtl", "large-text"):
        app.activate(f"{DEMO}-{suffix}")
    frame = open_modal(app)
    panel = visible(frame, MODAL)
    require(node(frame, MODAL)["name"] == "選擇時間", "Short dialog lost its accessible name")
    hour = visible(frame, MODAL + "-hour-input", 47.5)
    hour_hint = field_description(frame, MODAL, "hour")
    hour_help_bounds = visible(frame, MODAL + "-hour-hint")
    frame = key(app, "tab")
    focus(frame, MODAL + "-minute-input")
    minute = visible(frame, MODAL + "-minute-input", 47.5)
    minute_hint = field_description(frame, MODAL, "minute")
    minute_help_bounds = visible(frame, MODAL + "-minute-hint")
    require(abs(hour["width"] - minute["width"]) <= 0.5, "Hour and Minute have different widths")
    require(minute["x"] < hour["x"], "RTL did not place Minute on the left")
    period(frame, MODAL, "am")
    confirm = visible(frame, MODAL + "-confirm", 47.5)
    cancel = visible(frame, MODAL + "-cancel", 47.5)
    image = capture(app, plan, "modal-dark-rtl-200")
    frame = key(app, "tab", reverse=True)
    focus(frame, MODAL + "-hour-input")
    visible(frame, MODAL + "-hour-input", 47.5)
    frame = key(app, "escape")
    closed(frame, "09:07")
    return {"viewport": [320, 240], "rtl": True, "text_scale": 2, "panel": panel,
            "hour": hour, "minute": minute, "hour_hint": hour_hint, "minute_hint": minute_hint,
            "hour_hint_bounds": hour_help_bounds, "minute_hint_bounds": minute_help_bounds,
            "cancel": cancel, "confirm": confirm, "focus_restored": OPEN, "image": image}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--gallery", type=Path, required=True)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--out", type=Path, required=True)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True)
    report = {
        "source_manifest_sha256": plan.source,
        "gallery_sha256": digest(plan.gallery),
        "cli_sha256": digest(plan.cli),
        "headless_renderer": "WGPU native macOS",
        "input_source": "DevTools semantic key, action and fill",
        "external_input_boundary": "Hardware IME and scrim pointer events are outside this replay",
    }
    profiles = (("inline", (960, 800), inline), ("wide_modal", (960, 800), wide_modal),
                ("compact", (320, 240), compact))
    for name, dimensions, check in profiles:
        with launch(plan, name, *dimensions) as app:
            before = onscreen_windows(app.proc.pid)
            result = check(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(before == 0 and after == 0, f"Unexpected visible window in {name}")
            result["onscreen_windows"] = {"before": before, "after": after}
            report[name] = result
            print(f"PASS {name} Time Input and PNG transport", flush=True)
    (plan.out / "native-time-input-report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
