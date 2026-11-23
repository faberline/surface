#!/usr/bin/env python3
"""Check Baseline time picker paint and controls in a windowless Metal Gallery."""

import argparse
import json
from pathlib import Path

from gallery_transport import require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import (
    activate, capture, field_description, find, finish, focus, key, node,
    value, visible,
)
from window_check import onscreen_windows

INPUT = "time-input-modal"
INPUT_DEMO = "time-input-demo"
DIAL = "time-dial"
DIAL_DEMO = "time-dial-demo"
INPUT_OPEN = INPUT_DEMO + "-modal-open"
DIAL_OPEN = DIAL_DEMO + "-open"


def scene(app, demo, *settings):
    app.activate("section-2")
    activate(app, demo.replace("-demo", "-baseline-demo") + "-show")
    for setting in settings:
        activate(app, demo + "-" + setting)
    return activate(app, INPUT_OPEN if demo == INPUT_DEMO else DIAL_OPEN)


def saved(frame, demo, time):
    stable = demo + ("-modal-value" if demo == INPUT_DEMO else "-saved")
    item = node(frame, stable)
    require((item.get("text") or item["name"]) == f"已儲存 {time}", f"Unexpected saved value: {item}")


def panel(frame, prefix, *, normal=True):
    item = node(frame, prefix)
    require(item["role"] == "Dialog" and item["name"], "Baseline dialog is unnamed")
    layers = item.get("box_shadows", [])
    require(len(layers) == 2, "Missing Material Level 3 ambient and key shadows")
    style = item["style"]
    if normal:
        require(style["padding"] == 24., f"Expected 24 dp normal panel padding: {style}")
    return {"fill": style["background"], "padding": style["padding"], "shadows": layers}


def contained(outer, inner):
    return (outer["x"] - 0.5 <= inner["x"] and outer["y"] - 0.5 <= inner["y"]
            and inner["x"] + inner["width"] <= outer["x"] + outer["width"] + 0.5
            and inner["y"] + inner["height"] <= outer["y"] + outer["height"] + 0.5)


def dimensions(bounds, expected):
    require(all(abs(bounds[axis] - wanted) <= 0.5
                for axis, wanted in zip(("width", "height"), expected)),
            f"Expected {expected} logical size, got {bounds}")
    return bounds


def period(frame, prefix, name, painted_size):
    require(node(frame, prefix + "-period")["role"] == "RadioGroup", "Missing period radio group")
    paint = dimensions(visible(frame, prefix + "-period-paint"), painted_size)
    am, pm = [visible(frame, prefix + "-" + period, 47.5) for period in ("am", "pm")]
    separated = any(
        a[axis] + a[measure] <= b[axis] + 0.5
        for axis, measure in (("x", "width"), ("y", "height"))
        for a, b in ((am, pm), (pm, am))
    )
    require(separated, f"AM and PM hit targets overlap: {am} {pm}")
    for key, hit in (("am", am), ("pm", pm)):
        radio = node(frame, prefix + "-" + key)
        require(radio["role"] == "Radio" and radio.get("checked") is (key == name),
                f"Wrong checked state in {key}: {radio}")
        fill = visible(frame, f"{prefix}-{key}-paint")
        require(contained(hit, fill), f"Painted period control outside its input target: {fill}, {hit}")
    return {"track": paint, "am": am, "pm": pm}


def editor(frame, prefix, part, focused, expected=None):
    identifier = f"{prefix}-{part}-input"
    item = node(frame, identifier)
    require(item["role"] == "TextInput", f"Wrong editor role: {item}")
    style = item["style"]
    if focused:
        focus(frame, identifier)
        require(style["border_width"] == 2, f"Focused editor missing its 2 dp border: {style}")
    if expected is not None:
        value(frame, prefix, part, expected)
    return {"background": style["background"], "border": style["border_color"],
            "border_width": style["border_width"], "bounds": visible(frame, identifier)}


def input_light(app, plan):
    frame = scene(app, INPUT_DEMO)
    shell = panel(frame, INPUT)
    am_pm = period(frame, INPUT, "am", (52., 72.))
    hour = editor(frame, INPUT, "hour", True, "9")
    minute = editor(frame, INPUT, "minute", False, "07")
    dimensions(hour["bounds"], (96., 72.))
    dimensions(minute["bounds"], (96., 72.))
    require(hour["background"] != minute["background"], "Focused/resting fill is the same")
    first = capture(app, plan, "input-light-hour-focused")
    frame = key(app, "tab")
    reversed_hour = editor(frame, INPUT, "hour", False, "9")
    reversed_minute = editor(frame, INPUT, "minute", True, "07")
    require(hour["background"] == reversed_minute["background"] and
            minute["background"] == reversed_hour["background"], "Both fills did not follow focus")
    hint = field_description(frame, INPUT, "minute")
    second = capture(app, plan, "input-light-minute-focused")
    frame = finish(app, INPUT, "minute", "34", "34")
    saved(frame, INPUT_DEMO, "09:07")
    frame = activate(app, INPUT + "-confirm")
    saved(frame, INPUT_DEMO, "09:34")
    focus(frame, INPUT_OPEN)
    return {"panel": shell, "period": am_pm, "focused_hour": hour, "focused_minute": reversed_minute,
            "description": hint, "saved": "09:34", "images": [first, second]}


def input_dark(app, plan):
    frame = scene(app, INPUT_DEMO, "theme", "rtl", "locale", "format")
    shell = panel(frame, INPUT)
    require(find(frame, INPUT + "-period")[1] is None, "24-hour input has a period group")
    hour = editor(frame, INPUT, "hour", True, "０９")
    minute = editor(frame, INPUT, "minute", False, "０７")
    require(hour["bounds"]["x"] > minute["bounds"]["x"], "RTL editors are out of order")
    shot = capture(app, plan, "input-dark-japanese-24h")
    frame = activate(app, INPUT + "-cancel")
    saved(frame, INPUT_DEMO, "09:07")
    focus(frame, INPUT_OPEN)
    return {"panel": shell, "hour": hour, "minute": minute, "image": shot}


def dial(app, plan, clock, orientation, title):
    settings = []
    if orientation == "horizontal":
        settings += ["theme", "rtl"]
    if clock == 24:
        settings.append("format")
    frame = scene(app, DIAL_DEMO, *settings)
    shell = panel(frame, DIAL)
    face = dimensions(visible(frame, DIAL + "-dial-face"), (256., 256.))
    hour = dimensions(visible(frame, DIAL + "-hour-selector"),
                      (114. if clock == 24 and orientation == "vertical" else 96., 80.))
    minute = dimensions(visible(frame, DIAL + "-minute-selector"),
                        (114. if clock == 24 and orientation == "vertical" else 96., 80.))
    if orientation == "vertical":
        require(hour["y"] + hour["height"] < face["y"], "Portrait dial layout moved beside the face")
    else:
        require(face["x"] + face["width"] < minute["x"], "RTL landscape face is outside its mirrored slot")
    if clock == 12:
        group = period(frame, DIAL, "am", (52., 80.) if orientation == "vertical" else (216., 38.))
    else:
        require(find(frame, DIAL + "-period")[1] is None, "24-hour Dial has a period group")
        group = None
    focus(frame, DIAL + "-hour-selector")
    frame = key(app, "enter")
    focus(frame, DIAL + ("-dial-hour-12" if clock == 12 else "-dial-hour-0"))
    require(node(frame, DIAL + "-dial-hour-9").get("checked"), "Enter accepted a different hour")
    shot = capture(app, plan, title)
    frame = key(app, "escape")
    focus(frame, DIAL_OPEN)
    saved(frame, DIAL_DEMO, "09:07")
    return {"panel": shell, "face": face, "hour": hour, "minute": minute, "period": group,
            "image": shot}


def compact(app, plan):
    frame = scene(app, DIAL_DEMO, "theme", "rtl", "locale", "large-text")
    shell = panel(frame, DIAL, normal=False)
    require(find(frame, DIAL + "-dial-face")[1] is None, "Compact dialog kept a clipped face")
    require(node(frame, DIAL + "-use-dial").get("disabled"), "Unfit Dial action can open")
    focus(frame, DIAL + "-hour-input")
    require(node(frame, DIAL + "-unfit").get("text"), "Missing localized fallback reason")
    actions = [visible(frame, DIAL + "-" + part, 47.5) for part in ("confirm", "cancel")]
    shot = capture(app, plan, "dial-compact-dark-rtl-japanese-input")
    frame = activate(app, DIAL + "-cancel")
    focus(frame, DIAL_OPEN)
    saved(frame, DIAL_DEMO, "09:07")
    return {"panel": shell, "actions": actions, "image": shot}


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
    }
    profiles = (
        ("input_light", (560, 720), input_light),
        ("input_dark", (620, 680), input_dark),
        ("dial_portrait", (390, 720), lambda a, p: dial(a, p, 12, "vertical", "dial-light-portrait-12h")),
        ("dial_portrait_24", (390, 720), lambda a, p: dial(a, p, 24, "vertical", "dial-light-portrait-24h")),
        ("dial_landscape", (844, 390), lambda a, p: dial(a, p, 12, "horizontal", "dial-dark-landscape-rtl-12h")),
        ("compact", (320, 240), compact),
    )
    for name, dimensions, verify in profiles:
        with launch(plan, name, *dimensions) as app:
            before = onscreen_windows(app.proc.pid)
            result = verify(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(before == 0 and after == 0, f"Unexpected visible window in {name}")
            result["onscreen_windows"] = {"before": before, "after": after}
            report[name] = result
            print(f"PASS {name} Baseline time picker and PNG transports", flush=True)
    (plan.out / "native-time-baseline-report.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
