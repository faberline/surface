#!/usr/bin/env python3
"""Check Responsive Time Scroll in the native headless Metal Gallery."""

import argparse
import json
from pathlib import Path

from gallery_transport import require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import (
    activate, capture, field_description, find, finish, focus, key, node, period, settle,
    value, visible,
)
from window_check import onscreen_windows

DEMO = "time-scroll-demo"
INLINE = "time-scroll-inline"
MODAL = "time-scroll-modal"
OPEN = DEMO + "-open"


def setting(app, *choices):
    for name in choices:
        activate(app, f"{DEMO}-{name}")


def scene(app):
    app.activate("section-2")
    return activate(app, "show-time-scroll-demo")


def status(frame, expected, *, inline=False):
    name = f"{DEMO}-{'inline-' if inline else ''}saved"
    item = node(frame, name)
    scope = "頁面" if inline else "對話框"
    require((item.get("text") or item["name"]) == f"{scope}已儲存 {expected}", f"Wrong saved time: {item}")


def wheel(frame, owner, field, *, accepted, minimum, maximum, disabled=False):
    stable = f"{owner}-{field}-wheel"
    item = node(frame, stable)
    data = item.get("adjustable", {})
    require(item.get("role") == "Group" and item.get("name"), f"Unnamed or wrong wheel role: {item}")
    require(data.get("min") == minimum and data.get("max") == maximum and data.get("step") == 1,
            f"Wrong wheel range: {item}")
    require(data.get("value") == accepted and data.get("value_text"), f"Wrong wheel value: {item}")
    require(item.get("disabled", False) == disabled, f"Wrong availability: {item}")
    bounds = visible(frame, stable, 47.5)
    require(bounds["height"] >= 143.5, f"Wheel lacks three full targets: {bounds}")
    return {"bounds": bounds, "value_text": data["value_text"], "name": item["name"]}


def light(app, plan):
    frame = scene(app)
    hour = wheel(frame, INLINE, "hour", accepted=9, minimum=1, maximum=12)
    minute = wheel(frame, INLINE, "minute", accepted=7, minimum=0, maximum=59)
    period(frame, INLINE, "am")
    focus_by_tab(app, INLINE + "-hour-wheel")
    frame = key(app, "down")
    focus(frame, INLINE + "-hour-wheel")
    wheel(frame, INLINE, "hour", accepted=10, minimum=1, maximum=12)
    wheel(frame, INLINE, "minute", accepted=7, minimum=0, maximum=59)
    status(frame, "09:07", inline=True)
    frame = key(app, "right")
    focus(frame, INLINE + "-minute-wheel")
    frame = key(app, "up")
    wheel(frame, INLINE, "minute", accepted=6, minimum=0, maximum=59)
    inline = capture(app, plan, "inline-light-12h")
    frame = activate(app, DEMO + "-confirm-inline")
    status(frame, "10:06", inline=True)
    frame = activate(app, OPEN)
    require(node(frame, MODAL)["role"] == "Dialog", "Missing modal scope")
    focus(frame, MODAL + "-hour-wheel")
    wheel(frame, MODAL, "hour", accepted=9, minimum=1, maximum=12)
    wheel(frame, MODAL, "minute", accepted=7, minimum=0, maximum=59)
    app.wire("Action.activate", {"id": DEMO + "-outside"}, expected_error="outside_modal")
    modal = capture(app, plan, "modal-light-12h")
    frame = key(app, "escape")
    require(find(frame, MODAL)[1] is None, "Escape did not close modal")
    focus(frame, OPEN)
    status(frame, "09:07")
    return {"initial_wheels": [hour, minute], "saved_inline": "10:06", "modal_escape": True,
            "images": [inline, modal]}


def dark(app, plan):
    scene(app)
    setting(app, "theme", "direction", "locale", "scale")
    frame = activate(app, OPEN)
    period_targets = [visible(frame, f"{MODAL}-{name}", 47.5) for name in ("am", "pm")]
    require(all(target["height"] <= 60 for target in period_targets),
            f"Japanese period labels wrap in the spacious dialog: {period_targets}")
    activate(app, MODAL + "-cancel")
    setting(app, "clock")
    frame = activate(app, OPEN)
    focus(frame, MODAL + "-hour-wheel")
    hour = wheel(frame, MODAL, "hour", accepted=9, minimum=0, maximum=23)
    minute = wheel(frame, MODAL, "minute", accepted=7, minimum=0, maximum=59)
    require(hour["bounds"]["x"] > minute["bounds"]["x"], "RTL did not mirror the two wheels")
    require(find(frame, MODAL + "-period")[1] is None, "24h should omit period")
    require(hour["value_text"] == "０９" and minute["value_text"] == "０７", "Missing full-width values")
    frame = key(app, "left")
    focus(frame, MODAL + "-minute-wheel")
    frame = key(app, "down")
    wheel(frame, MODAL, "minute", accepted=8, minimum=0, maximum=59)
    image = capture(app, plan, "modal-dark-rtl-japanese-24h")
    frame = activate(app, MODAL + "-use-input")
    focus(frame, MODAL + "-minute-input")
    value(frame, MODAL, "minute", "０８")
    hint = field_description(frame, MODAL, "minute")
    frame = finish(app, MODAL, "minute", "35", "３５")
    status(frame, "09:07")
    direct = capture(app, plan, "modal-dark-rtl-direct-input")
    frame = activate(app, MODAL + "-use-scroll")
    focus(frame, MODAL + "-minute-wheel")
    wheel(frame, MODAL, "minute", accepted=35, minimum=0, maximum=59)
    frame = activate(app, MODAL + "-confirm")
    focus(frame, OPEN)
    status(frame, "09:35")
    return {"initial_wheels": [hour, minute], "wide_period_targets": period_targets,
            "input_hint": hint, "saved_modal": "09:35",
            "images": [image, direct]}


def compact(app, plan):
    scene(app)
    setting(app, "theme", "direction", "locale", "scale")
    frame = activate(app, OPEN)
    focus(frame, MODAL + "-hour-input")
    require(find(frame, MODAL + "-hour-wheel")[1] is None, "The compact picker kept the wheel")
    require(node(frame, MODAL + "-use-scroll").get("disabled"), "The compact toggle can open an unfit wheel")
    require(node(frame, MODAL + "-unfit").get("text"), "Missing fallback reason")
    actions = [visible(frame, MODAL + "-" + action, 47.5) for action in ("cancel", "confirm")]
    image = capture(app, plan, "compact-dark-rtl-japanese-input")
    frame = activate(app, MODAL + "-cancel")
    focus(frame, OPEN)
    status(frame, "09:07")
    return {"actions": actions, "image": image}


def disabled(app, plan):
    scene(app)
    setting(app, "disabled")
    frame = activate(app, OPEN)
    focus(frame, MODAL + "-cancel")
    hour = wheel(frame, MODAL, "hour", accepted=9, minimum=1, maximum=12, disabled=True)
    minute = wheel(frame, MODAL, "minute", accepted=7, minimum=0, maximum=59, disabled=True)
    require(node(frame, MODAL + "-confirm").get("disabled"), "Disabled dialog can save")
    image = capture(app, plan, "disabled-light")
    frame = activate(app, MODAL + "-cancel")
    focus(frame, OPEN)
    return {"wheels": [hour, minute], "image": image}


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
        ("light", (560, 720), light), ("dark", (960, 760), dark),
        ("compact", (320, 240), compact), ("disabled", (480, 620), disabled),
    )
    for name, dimensions, check in profiles:
        with launch(plan, name, *dimensions) as app:
            before = onscreen_windows(app.proc.pid)
            result = check(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(before == 0 and after == 0, f"Unexpected visible window in {name}")
            result["onscreen_windows"] = {"before": before, "after": after}
            report[name] = result
            print(f"PASS {name} Time Scroll and PNG transports", flush=True)
    (plan.out / "native-time-scroll-report.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
