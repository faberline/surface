#!/usr/bin/env python3
"""Verify Material 3 Expressive Time Dial on the native headless Gallery."""

import argparse
import json
from pathlib import Path

from gallery_transport import require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import (
    activate, capture, field_description, find, finish, focus, key, node, period, settle, value,
    visible,
)
from window_check import onscreen_windows

DEMO = "time-dial-demo"
MODAL = "time-dial"
FACE = MODAL + "-dial-face"
OPEN = DEMO + "-open"


def saved(frame, time):
    expected = f"已儲存 {time}"
    status = node(frame, DEMO + "-saved")
    require((status.get("text") or status["name"]) == expected, f"Expected {expected}, got {status}")


def scene(app):
    app.activate("section-2")
    frame = activate(app, DEMO + "-show")
    saved(frame, "09:07")
    return frame


def dialog(app):
    frame = activate(app, OPEN)
    require(node(frame, MODAL)["role"] == "Dialog", "Expected named modal scope")
    return frame


def clock_face(frame):
    bounds = visible(frame, FACE, 255.5)
    require(abs(bounds["width"] - 256) <= 0.5 and abs(bounds["height"] - 256) <= 0.5,
            f"Expected full 256 dp clock face, got {bounds}")
    return bounds


def checked_value(frame, field, value):
    item = node(frame, f"{MODAL}-dial-{field}-{value}")
    require(item["role"] == "Radio" and item["checked"] is True and item["name"],
            f"Expected checked, named dial value {value}: {item}")
    return item["name"]


def resting_hand(frame, field, value):
    handle = visible(frame, MODAL + "-dial-handle", 47.5)
    chosen = visible(frame, f"{MODAL}-dial-{field}-{value}", 47.5)
    for axis, size in (("x", "width"), ("y", "height")):
        expected = chosen[axis] + chosen[size] / 2
        require(abs(handle[axis] + handle[size] / 2 - expected) <= 0.5,
                f"Reduce Motion did not snap the hand to {field} {value}")
    return handle


def closed(frame, time):
    focus(frame, OPEN)
    require(find(frame, MODAL)[1] is None, "Time Dial did not dismiss")
    saved(frame, time)


def portrait(app, plan):
    scene(app)
    frame = dialog(app)
    focus(frame, MODAL + "-hour-selector")
    shape = clock_face(frame)
    initial = checked_value(frame, "hour", 9)
    period(frame, MODAL, "am")
    frame = key(app, "enter")
    focus(frame, MODAL + "-dial-hour-12")
    checked_value(frame, "hour", 9)
    resting_hand(frame, "hour", 9)
    saved(frame, "09:07")
    before = capture(app, plan, "portrait-light-12h")
    app.wire("Action.activate", {"id": DEMO + "-outside"}, expected_error="outside_modal")
    frame = activate(app, MODAL + "-dial-hour-3")
    checked_value(frame, "minute", 7)
    if frame["interaction"]["focused"] != MODAL + "-minute-selector":
        focus_by_tab(app, MODAL + "-minute-selector")
        frame = settle(app)
    focus(frame, MODAL + "-minute-selector")
    frame = key(app, "enter")
    focus(frame, MODAL + "-dial-minute-0")
    checked_value(frame, "minute", 7)
    resting_hand(frame, "minute", 7)
    frame = activate(app, MODAL + "-dial-minute-35")
    selected = checked_value(frame, "minute", 35)
    saved(frame, "09:07")
    handle = resting_hand(frame, "minute", 35)
    after = capture(app, plan, "portrait-light-minute")
    require(before["sha256"] != after["sha256"], "Choosing an hour and minute did not change paint")
    closed(activate(app, MODAL + "-confirm"), "03:35")
    return {"face": shape, "entry": {"hour": 12, "minute": 0},
            "hour_initial_name": initial, "minute_final_name": selected, "snapped_hand": handle,
            "saved_time": "03:35", "outside_action_rejected": True, "images": [before, after]}


def landscape(app, plan):
    scene(app)
    for choice in ("theme", "rtl", "locale", "format"):
        activate(app, f"{DEMO}-{choice}")
    frame = dialog(app)
    focus(frame, MODAL + "-hour-selector")
    shape = clock_face(frame)
    hour = visible(frame, MODAL + "-hour-selector", 47.5)
    require(abs(shape["x"] - hour["x"]) > 100, "Wide dial did not place face beside selector")
    require(find(frame, MODAL + "-period")[1] is None, "24-hour dial has an AM/PM group")
    name = checked_value(frame, "hour", 9)
    frame = key(app, "enter")
    focus(frame, MODAL + "-dial-hour-0")
    checked_value(frame, "hour", 9)
    resting_hand(frame, "hour", 9)
    before = capture(app, plan, "horizontal-dark-rtl")
    activate(app, MODAL + "-dial-hour-12")
    frame = activate(app, MODAL + "-use-input")
    value(frame, MODAL, "hour", "１２")
    value(frame, MODAL, "minute", "０７")
    saved(frame, "09:07")
    after = capture(app, plan, "horizontal-dark-japanese-input", frame=frame,
                    sample_targets=(MODAL + "-hour-label", MODAL + "-minute-label"))
    frame = activate(app, MODAL + "-use-dial")
    clock_face(frame)
    closed(key(app, "escape"), "09:07")
    return {"face": shape, "entry_hour": 0, "hour_initial_name": name, "selector": hour, "retained": "12:07",
            "images": [before, after]}


def compact(app, plan):
    scene(app)
    for choice in ("theme", "rtl", "large-text"):
        activate(app, f"{DEMO}-{choice}")
    frame = dialog(app)
    focus(frame, MODAL + "-hour-input")
    require(find(frame, FACE)[1] is None, "Compact fallback still draws a clipped clock")
    reason = node(frame, MODAL + "-unfit")
    require("空間不足" in (reason.get("text") or reason["name"]), "Missing fallback reason")
    hour = visible(frame, MODAL + "-hour-input", 47.5)
    minute = visible(frame, MODAL + "-minute-input", 47.5)
    require(hour["x"] > minute["x"], "Input fallback does not mirror its field positions")
    ok = visible(frame, MODAL + "-confirm", 47.5)
    cancel = visible(frame, MODAL + "-cancel", 47.5)
    image = capture(app, plan, "compact-dark-rtl")
    frame = finish(app, MODAL, "hour", "13", "13")
    field_description(frame, MODAL, "hour", error=True)
    frame = activate(app, MODAL + "-confirm")
    require(find(frame, MODAL)[1] is not None, "Invalid fallback input closed the dialog")
    saved(frame, "09:07")
    closed(activate(app, MODAL + "-cancel"), "09:07")
    return {"viewport": [320, 300], "hour": hour, "minute": minute, "save": ok,
            "cancel": cancel, "saved_time": "09:07", "image": image}


def compact_japanese(app, plan):
    scene(app)
    for choice in ("theme", "rtl", "locale", "large-text"):
        activate(app, f"{DEMO}-{choice}")
    frame = dialog(app)
    focus(frame, MODAL + "-hour-input")
    require(find(frame, FACE)[1] is None, "Compact Japanese has a clipped dial")
    panel = visible(frame, MODAL)
    ok = visible(frame, MODAL + "-confirm", 47.5)
    cancel = visible(frame, MODAL + "-cancel", 47.5)
    body = visible(frame, MODAL + "-body")
    require(node(frame, MODAL + "-cancel")["name"] == "取り消し", "Wrong Cancel name")
    require(cancel["height"] >= 64, "The Japanese action did not wrap in the fixture")
    require(body["y"] + body["height"] <= ok["y"], "The body overlaps the footer")
    value(frame, MODAL, "hour", "９")
    shot = capture(app, plan, "compact-dark-rtl-japanese")
    closed(activate(app, MODAL + "-cancel"), "09:07")
    return {"panel":panel, "body":body, "save":ok, "cancel":cancel, "image":shot}


def magnified(app, plan):
    scene(app)
    for choice in ("theme", "rtl", "locale", "format", "large-text"):
        activate(app, f"{DEMO}-{choice}")
    frame = dialog(app)
    shape = clock_face(frame)
    labels = {}
    for choice in range(24):
        radio = node(frame, f"{MODAL}-dial-hour-{choice}")
        require(radio["role"] == "Radio" and "24 のうち" in radio["name"],
                f"Missing localized hour count for {choice}")
        index = radio.get("first_child")
        if index is not None:
            glyph = frame["dom"]["nodes"][index]
            require(glyph["style"]["font_size"] <= 16, f"Oversized dial hour {choice}")
            labels[choice] = frame["layout"]["nodes"][index]["bounds"]
    require(9 in labels and len(labels) >= 22, "too many numeric labels removed")
    for choice, axis in ((0, "y"), (3, "x"), (6, "y"), (9, "x")):
        if choice in labels and choice + 12 in labels:
            start, end = sorted((labels[choice], labels[choice + 12]), key=lambda box: box[axis])
            span = "width" if axis == "x" else "height"
            require(start[axis] + start[span] <= end[axis] + 0.5,
                    f"Radial labels overlap at {choice}: {start}, {end}")
    image = capture(app, plan, "magnified-dark-rtl-japanese")
    return {"viewport": [1120, 880], "face": shape, "visible_printed_hours": sorted(labels),
            "semantically_named_hours": 24, "image": image}


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
        "motion": "Native headless uses Reduce Motion. WebGPU and timed Runtime tests sample the spring.",
        "external_input_boundary": "Physical point delivery and composition use separate tests",
    }
    profiles = (("portrait", (390, 720), portrait), ("landscape", (844, 390), landscape),
                ("compact", (320, 300), compact), ("magnified", (1120, 880), magnified),
                ("compact_japanese", (320, 300), compact_japanese))
    for name, dimensions, check in profiles:
        with launch(plan, name, *dimensions) as app:
            before = onscreen_windows(app.proc.pid)
            result = check(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(before == 0 and after == 0, f"Unexpected visible window in {name}")
            result["onscreen_windows"] = {"before": before, "after": after}
            report[name] = result
            print(f"PASS {name} Time Dial and PNG transports", flush=True)
    (plan.out / "native-time-dial-motion-report.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
