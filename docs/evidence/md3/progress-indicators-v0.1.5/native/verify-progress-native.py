#!/usr/bin/env python3
"""Check Material 3 progress indicators in a native Surface window."""

import argparse
import json
from pathlib import Path
import subprocess
import time

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, launch


INDICATORS = {
    "progress-linear-flat": "線形進度",
    "progress-linear-wavy": "波形線形進度",
    "progress-circular-flat": "圓形進度",
    "progress-circular-wavy": "波形圓形進度",
}
PROFILES = {
    "light-desktop": (960, 800, False, False),
    "dark-desktop": (960, 800, True, False),
    "dark-rtl-200-compact": (320, 680, True, True),
    "light-rtl-200-narrow": (260, 720, False, True),
}
SCREENSHOT = {"scale": 1, "max_side": 1200}


def capture(app, out, name, parity=False):
    path = out / f"{name}.png"
    result = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    data = path.read_bytes()
    width, height, pixels = read_png(data)
    viewport = app.dom()["layout"]["viewport"]
    require((width, height) == (viewport["width"], viewport["height"]), "Screenshot size differs from window")
    summary = {"file": path.name, "sha256": digest(path), "metadata": metadata(result)}
    if parity:
        wire = app.wire("Page.screenshot", SCREENSHOT)
        image, mcp, _ = app.mcp(SCREENSHOT)
        require(png_payload(wire) == data == image, "API, CLI and MCP PNG bytes differ")
        require(metadata(wire) == metadata(result) == mcp, "Transport metadata differs")
        summary["transport_parity"] = ["raw", "cli", "mcp"]
    return summary, pixels


def check_nodes(frame, value):
    geometry = {}
    for index, entry in enumerate(frame["dom"]["nodes"]):
        stable_id = entry["stable_id"]
        if stable_id not in INDICATORS:
            continue
        require(stable_id not in geometry and entry["role"] == "Progress"
                and entry["name"] == INDICATORS[stable_id] and entry["tab_index"] is None,
                f"Incorrect progress semantics for {stable_id}: {entry}")
        range_value = entry["range"]
        if value is None:
            require(range_value is None, f"Indeterminate role has a numeric value: {entry}")
        else:
            require(range_value is not None and
                    (range_value["min"], range_value["max"], range_value["value"]) == (0, 1, value),
                    f"Wrong progress range: {entry}")
        geometry[stable_id] = frame["layout"]["nodes"][index]["bounds"]
    require(set(geometry) == set(INDICATORS), "Expected all four indicators")
    return geometry


def rgb(rows, x, y):
    return list(rows[int(y)][int(x) * 4:int(x) * 4 + 3])


def center_x(box):
    return box["x"] + box["width"] / 2


def crop(rows, box):
    x, y, width, height = (int(box[key]) for key in ("x", "y", "width", "height"))
    require(x >= 0 and y >= 0 and y + height <= len(rows) and
            x + width <= len(rows[0]) // 4, f"Progress paint is outside the frame: {box}")
    return b"".join(row[x * 4:(x + width) * 4] for row in rows[y:y + height])


def samples(geometry, rows, rtl):
    bar = geometry["progress-linear-flat"]
    x, y, width, height = (bar[key] for key in ("x", "y", "width", "height"))

    def at(offset, row):
        return rgb(rows, x + (width - 1 - offset if rtl else offset), row)

    ink = at(width * .1, y + height / 2)
    track = at(width * .6, y + height / 2)
    gap = at(width * .25 + 2, y + height / 2)
    backdrop = at(width * .25 + 2, y - 1)
    stop = at(width - 2, y + height / 2)
    require(ink != track and track != gap and ink != gap and gap == backdrop and stop == ink,
            f"Linear quarter is missing an active span, track, gap or stop: {ink}/{track}/{gap}/{backdrop}/{stop}")
    ring = geometry["progress-circular-flat"]
    top_right = rgb(rows, center_x(ring) + 12, ring["y"] + 8)
    bottom_left = rgb(rows, center_x(ring) - 12, ring["y"] + 32)
    require(top_right == ink and bottom_left == track, f"Circular quarter is wrong: {top_right}/{bottom_left}")
    wave_counts = {}
    for stable_id in ("progress-linear-wavy", "progress-circular-wavy"):
        box = geometry[stable_id]
        coordinates = [(column, row)
                       for row in range(int(box["y"]), int(box["y"] + box["height"]))
                       for column in range(int(box["x"]), int(box["x"] + box["width"]))]
        active = sum(rgb(rows, column, row) == ink for column, row in coordinates)
        require(active >= 15, f"Wavy path did not paint for {stable_id}: {active} ink pixels")
        wave_counts[stable_id] = active
    return {"active": ink, "track": track, "gap": gap, "stop": stop, "wave_pixels": wave_counts}


def read_ax(plan, app, *args):
    return json.loads(subprocess.run([plan.ax_reader, str(app.proc.pid), *args],
                                   capture_output=True, check=True, timeout=15).stdout)


def ax_read(plan, app, expected):
    if not plan.ax_reader:
        return {"verified": False}
    deadline = time.monotonic() + 5
    while True:
        state = read_ax(plan, app)
        if (not state["trusted"] or state["windows"]) or time.monotonic() >= deadline:
            break
        app.dom()
        time.sleep(.1)
    if not state["trusted"]:
        return {"verified": False, "reason": "Caller lacks macOS Accessibility permission"}
    nodes = state["progress_indicators"]
    require(len(nodes) == 4 and {n["name"] for n in nodes} == set(INDICATORS.values()),
            f"AppKit does not report all four progress indicators: {state}")
    for node in nodes:
        require(not node["focused"] and not node.get("value_settable")
                and "AXPress" not in node["actions"], f"Progress is interactive: {node}")
        if expected is not None:
            require((node.get("min"), node.get("max"), node.get("value")) == (0, 1, expected),
                    f"AppKit has wrong progress range: {node}")
        else:
            require(all(key not in node for key in ("value", "min", "max")),
                    f"AppKit kept stale progress range: {node}")
    return {"verified": True, **state}


def show_samples(plan, app, height, value):
    geometry = check_nodes(app.dom(), value)
    if all(box["y"] + box["height"] <= height for box in geometry.values()):
        return geometry, False
    require(plan.ax_reader is not None, "The short profile needs the AppKit scroll reader")
    deadline = time.monotonic() + 4
    while time.monotonic() < deadline:
        state = read_ax(plan, app, "scroll")
        require(state["trusted"], "AppKit scroll needs Accessibility permission")
        if state.get("scroll_status") == 0:
            break
        time.sleep(.1)
    else:
        raise RuntimeError("No AppKit scroll action announced by the short window")
    while True:
        geometry = check_nodes(app.dom(), value)
        if all(box["y"] >= 0 and box["y"] + box["height"] <= height for box in geometry.values()):
            return geometry, True
        require(time.monotonic() < deadline, f"AppKit did not reveal all samples: {geometry}")
        time.sleep(.1)


def verify_profile(plan, name, width, height, dark, compact):
    with launch(plan, name, width, height, True) as (app, _):
        app.activate("section-1")
        app.activate("progress-demo-show")
        require(focused(app.dom()) == "progress-demo-back", "Focus did not reach the preview")
        if dark:
            app.activate("theme")
        if compact:
            app.activate("progress-rtl")
            app.activate("progress-scale")
        initial, scrolled = show_samples(plan, app, height, .25)
        expected_width = min(240, width - 56)
        require(initial["progress-linear-flat"]["width"] == expected_width
                and initial["progress-linear-flat"]["height"] == 4
                and initial["progress-linear-wavy"]["height"] == 10
                and initial["progress-circular-flat"]["width"] == 40
                and initial["progress-circular-wavy"]["width"] == 48,
                f"Unexpected progress indicator size: {initial}")
        if compact:
            require(initial["progress-circular-flat"]["y"] > initial["progress-linear-wavy"]["y"],
                    "Compact cards did not stack")
        known_ax = ax_read(plan, app, .25) if name == "light-desktop" else None
        quarter, pixels = capture(app, plan.out, name + "-quarter", parity=name == "dark-rtl-200-compact")
        paint = samples(initial, pixels, compact)

        app.activate("progress-value-0")
        zero, _ = show_samples(plan, app, height, 0.)
        _, pixels = capture(app, plan.out, name + "-zero")
        ring = zero["progress-circular-flat"]
        require(rgb(pixels, center_x(ring), ring["y"] + 2) == paint["track"],
                "The empty ring has active paint")
        app.activate("progress-value-100")
        full, _ = show_samples(plan, app, height, 1.)
        complete, pixels = capture(app, plan.out, name + "-complete")
        ring = full["progress-circular-flat"]
        require(all(rgb(pixels, center_x(ring) + dx, ring["y"] + dy) == paint["active"]
                    for dx, dy in ((0, 2), (0, 38), (-18, 20), (18, 20))),
                "The complete ring has missing active paint")
        full_ax = ax_read(plan, app, 1.) if known_ax else None

        app.activate("progress-mode-unknown")
        unknown_ax = ax_read(plan, app, None) if known_ax else None
        app.activate("progress-motion")
        unknown_geometry, _ = show_samples(plan, app, height, None)
        unknown, before = capture(app, plan.out, name + "-unknown")
        time.sleep(.35)
        frozen, after = capture(app, plan.out, name + "-reduced-motion")
        require(check_nodes(app.dom(), None) == unknown_geometry and
                all(crop(before, box) == crop(after, box) for box in unknown_geometry.values()),
                "Reduce Motion indicator pixels are not static")
        tab_order = []
        for _ in range(18):
            app.key("tab")
            target = focused(app.dom())
            tab_order.append(target)
            require(target not in INDICATORS, f"Read-only progress received focus: {target}")
            if target == "progress-demo-back":
                break
        require("progress-demo-back" in tab_order, "Tab did not leave the progress samples")
        return {
            "geometry": initial, "scrolled_by_appkit": scrolled,
            "quarter": quarter, "full": complete, "unknown": unknown,
            "pixels": paint, "reduced_motion_png": frozen,
            "reduced_motion_indicator_pixels_static": True, "tab_order": tab_order,
            "ax": {"quarter": known_ax, "complete": full_ax, "unknown": unknown_ax},
        }


def main():
    parser = argparse.ArgumentParser()
    for key in ("gallery", "cli", "profile-launcher", "out"):
        parser.add_argument("--" + key, type=Path, required=True)
    parser.add_argument("--ax-reader", type=Path)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True, exist_ok=False)
    receipts = {}
    for name, profile in PROFILES.items():
        receipts[name] = verify_profile(plan, name, *profile)
        print(f"PASS {name}", flush=True)
    (plan.out / "native-progress-report.json").write_text(json.dumps({
        "source_manifest_sha256": plan.source,
        "profiles": receipts,
    }, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
