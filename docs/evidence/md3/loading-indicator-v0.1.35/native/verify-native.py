#!/usr/bin/env python3
"""Replay Material loading indicators in the native headless Gallery."""

import argparse
import json
from math import hypot
from pathlib import Path
import re

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle
from window_check import onscreen_windows

DEMO = "loading-demo"
SHOW = f"{DEMO}-show"
BACK = f"{DEMO}-back"
DEFAULT = "loading-default"
CONTAINED = "loading-contained"
NAMES = {DEFAULT: "載入資料", CONTAINED: "讀取快取"}
REPORT_NAME = "native-loading-indicator-report.json"
IMAGE_NAMES = {
    "loading-light-48.png",
    "loading-stopped-48.png",
    "loading-restarted-48.png",
    "loading-dark-rtl-24.png",
    "loading-dark-rtl-48.png",
    "loading-wide-240.png",
}


def rect(frame, stable_id):
    return layout(frame, stable_id)["bounds"]


def full_box(frame, stable_id):
    shown = layout(frame, stable_id)
    box = shown["bounds"]
    require(same_box(frame["layout"]["viewport"], box) and same_box(shown["clip"], box),
            f"Clipped {stable_id}: {shown}")
    return box


def follow(app, stroke, stable_id):
    frame = key(app, stroke)
    focus(frame, stable_id)
    return frame


def scene(app, *settings):
    activate(app, "section-1")
    focus_by_tab(app, SHOW)
    frame = follow(app, "enter", BACK)
    for preset in settings:
        frame = activate(app, f"{DEMO}-{preset}")
    require(node(frame, DEMO)["role"] == "Window" and node(frame, DEMO)["name"] == "載入指示器",
            "Unexpected Gallery scene")
    return frame


def tab_away(app):
    focus_by_tab(app, BACK)
    frame = follow(app, "enter", SHOW)
    require(find(frame, DEMO)[0] is None, "The loading scene stayed open")
    for stable_id in NAMES:
        require(find(frame, stable_id)[0] is None, "A progress node stayed on the home route")
    return SHOW


def luminance(rgba):
    channels = [channel / 255. for channel in rgba[:3]]
    linear = [v / 12.92 if v <= .04045 else ((v + .055) / 1.055) ** 2.4 for v in channels]
    return sum(weight * v for weight, v in zip((.2126, .7152, .0722), linear))


def contrast(foreground, background):
    dark, light = sorted((luminance(foreground), luminance(background)))
    return (light + .05) / (dark + .05)


def reduced_polygon(contour_source):
    # The reduced pose uses the start of pair 0 with Compose's +90° offset.
    first_pair = contour_source.split("\n    &[\n", 1)[1].split("\n    ],", 1)[0]
    samples = re.findall(r"\[(-?\d+), (-?\d+), -?\d+, -?\d+\]", first_pair)
    require(3 <= len(samples) <= 256, "Missing or invalid original contour samples")
    xs, ys = [tuple(map(int, axis)) for axis in zip(*samples)]
    center_x = (min(xs) + max(xs)) / 2.
    center_y = (min(ys) + max(ys)) / 2.
    return [((center_y - y) / 32768., (x - center_x) / 32768.) for x, y in zip(xs, ys)]


def measure(frame, stable_id, size, polygon):
    progress = node(frame, stable_id)
    require(progress["role"] == "Progress" and progress["name"] == NAMES[stable_id]
            and progress.get("range") is None and progress.get("tab_index") is None,
            f"Wrong indeterminate progress semantics: {stable_id}")
    outer = full_box(frame, stable_id)
    slot = full_box(frame, f"{stable_id}-slot")
    shape_id = f"{stable_id}-shape"
    active = full_box(frame, shape_id)
    require(outer == slot and (outer["width"], outer["height"]) == (size, size),
            f"Wrong indicator allocation: {stable_id} {outer}")
    want_active = size * 38. / 48.
    require(abs(active["height"] - want_active) <= .01 and abs(active["width"] - want_active) <= .01,
            f"Wrong active size for {stable_id}: {active}")
    for axis, length in (("x", "width"), ("y", "height")):
        inset = (outer[length] - active[length]) / 2.
        require(abs(active[axis] - outer[axis] - inset) <= .01, "Active outline lost its center")
    shape = node(frame, shape_id)
    require(shape["role"] == "Group" and not shape["name"] and shape.get("tab_index") is None,
            "Shape acquired a semantic target")
    require("fill_contour" not in shape, "Paint geometry leaked into the semantic snapshot")
    center = want_active * .5
    radius = max(hypot(x, y) * want_active for x, y in polygon)
    require(radius < center, f"Contour touched the edge of the active box: {radius}")
    outer_color = progress["style"]["background"]
    backing = node(frame, f"{stable_id}-panel")["style"]["background"]
    require(outer_color[3] == (255 if stable_id == CONTAINED else 0), "Wrong default/contained fill")
    return {"name": progress["name"], "role": progress["role"], "has_value": False,
            "outer": outer, "active": active, "contour_vertices": len(polygon),
            "contour_radius": radius, "container": outer_color, "backing": backing}


def shot(app, plan, name, frame):
    before = onscreen_windows(app.proc.pid)
    receipt = capture(app, plan, name, frame=frame)
    after = onscreen_windows(app.proc.pid)
    viewport = frame["layout"]["viewport"]
    require((receipt["width"], receipt["height"]) == (viewport["width"], viewport["height"]),
            "PNG logical dimensions differ")
    require(before == after == 0, f"{name} opened a visible window")
    receipt["visible_windows"] = [before, after]
    return receipt


def rows(plan, receipt):
    _, _, rgba = read_png((plan.out / receipt["name"]).read_bytes())
    return rgba


def pixel(scanlines, x, y):
    require(0 <= x < len(scanlines[0]) // 4 and 0 <= y < len(scanlines),
            f"Sample outside the PNG at {(x, y)}")
    return list(scanlines[y][4*x:4*x+4])


def close_enough(a, b):
    return max(abs(x - y) for x, y in zip(a, b)) <= 3


def classify(point, edges):
    """Classify an interior or exterior sample at least one pixel from the edge."""
    px, py = point
    inside = False
    for (ax, ay), (bx, by) in edges:
        dx, dy = bx - ax, by - ay
        extent = dx * dx + dy * dy
        if extent > 0.:
            t = min(1., max(0., ((px - ax) * dx + (py - ay) * dy) / extent))
            if (px - ax - t * dx) ** 2 + (py - ay - t * dy) ** 2 < 1.2 ** 2:
                return None
        if (ay > py) != (by > py):
            cut = ax + (py - ay) * dx / dy
            if px < cut:
                inside = not inside
    return inside


def check_pixels(scanlines, stable_id, result, polygon):
    x, y, side, _ = (round(result["outer"][k]) for k in ("x", "y", "width", "height"))
    active = result["active"]
    contained = stable_id == CONTAINED
    background = result["container"] if contained else result["backing"]
    center = pixel(scanlines, x + side // 2, y + side // 2)
    rim = pixel(scanlines, x + side // 2, y + round(side * .08))
    corner = pixel(scanlines, x + 1, y + 1)
    ratio = contrast(center, background)
    require(center[3] == 255 and ratio >= 3., f"Missing visible ink for {stable_id}: {center}")
    require(close_enough(rim, background) and close_enough(corner, result["backing"]),
            f"Wrong round container or passive background for {stable_id}: {rim=} {corner=}")

    local = [((x + .5) * active["width"], (y + .5) * active["height"]) for x, y in polygon]
    edges = list(zip(local, local[1:] + local[:1]))
    classified = {"interior": 0, "exterior": 0}
    for column in range(1, 16):
        xx = round(active["x"] + active["width"] * column / 16.)
        for row in range(1, 16):
            yy = round(active["y"] + active["height"] * row / 16.)
            inside = classify((xx + .5 - active["x"], yy + .5 - active["y"]), edges)
            if inside is None:
                continue
            outside = background
            if contained and not inside:
                distance = hypot(xx + .5 - x - side / 2., yy + .5 - y - side / 2.)
                if abs(distance - side / 2.) < 1.:
                    continue
                if distance > side / 2.:
                    outside = result["backing"]
            expected = center if inside else outside
            require(close_enough(pixel(scanlines, xx, yy), expected),
                    f"{stable_id} contour did not reach the PNG at {(xx, yy)}")
            classified["interior" if inside else "exterior"] += 1
    require(min(classified.values()) >= 12, f"Insufficient contour samples for {stable_id}: {classified}")
    result["ink"] = center
    result["contrast"] = round(ratio, 4)
    result["pixels"] = {"center": center, "rim": rim, "corner": corner, "classified": classified}


def active_shot(app, plan, title, frame, size, *, rtl=False):
    proof = {stable_id: measure(frame, stable_id, size, plan.polygon) for stable_id in NAMES}
    first, second = (proof[stable_id]["outer"] for stable_id in NAMES)
    require(first["y"] == second["y"] and (first["x"] > second["x"]) is rtl,
            f"Unexpected direction for two indicator panels: {first}, {second}")
    receipt = shot(app, plan, title, frame)
    scanlines = rows(plan, receipt)
    for stable_id, item in proof.items():
        check_pixels(scanlines, stable_id, item, plan.polygon)
    return receipt, proof


def slot_crop(scanlines, box):
    x, y, width, height = (round(box[k]) for k in ("x", "y", "width", "height"))
    return [scanlines[index][4*x:4*(x+width)] for index in range(y, y + height)]


def light(app, plan):
    running = scene(app)
    require(sum(node(running, DEMO)["style"]["background"][:3]) / 3. > 128.,
            "Light profile opened dark")
    first, enabled = active_shot(app, plan, "loading-light-48", running, 48.)
    path = focus_by_tab(app, f"{DEMO}-stop")
    require(all(stable_id not in path for stable_id in NAMES), "Progress became a Tab stop")
    stopped = follow(app, "enter", f"{DEMO}-stop")
    after_stop = shot(app, plan, "loading-stopped-48", stopped)
    stopped_pixels = rows(plan, after_stop)
    slots = {}
    for stable_id in NAMES:
        require(find(stopped, stable_id)[0] is None and find(stopped, f"{stable_id}-shape")[0] is None,
                f"Stop left visible or semantic progress for {stable_id}")
        slot = full_box(stopped, f"{stable_id}-slot")
        require(slot == enabled[stable_id]["outer"], "Stopping changed the allocated slot")
        flat = bytes(enabled[stable_id]["backing"]) * round(slot["width"])
        require(all(span == flat for span in slot_crop(stopped_pixels, slot)),
                f"Stop left pixels in {stable_id}")
        slots[stable_id] = slot
    follow(app, "tab", f"{DEMO}-reset")
    follow(app, "tab", BACK)
    focus_by_tab(app, f"{DEMO}-run")
    restarted = follow(app, "enter", f"{DEMO}-run")
    after_start, restarted_proof = active_shot(app, plan, "loading-restarted-48", restarted, 48.)
    restored_pixels = rows(plan, after_start)
    first_pixels = rows(plan, first)
    for stable_id, slot in slots.items():
        require(restarted_proof[stable_id] == enabled[stable_id], "Start changed the reduced endpoint")
        require(slot_crop(first_pixels, slot) == slot_crop(restored_pixels, slot),
                "Start did not restore the same reduced pixels")
    returned_focus = tab_away(app)
    return {"startup": enabled, "stopped_slots": slots, "focus_path": path,
            "stop_focus": f"{DEMO}-stop", "restart_focus": f"{DEMO}-run",
            "back_focus": returned_focus, "captures": [first, after_stop, after_start]}


def compact(app, plan):
    frame = scene(app, "theme", "rtl", "large-text", "small")
    require(sum(node(frame, DEMO)["style"]["background"][:3]) / 3. < 128.,
            "Compact profile opened light")
    small, small_proof = active_shot(app, plan, "loading-dark-rtl-24", frame, 24., rtl=True)
    bigger = activate(app, f"{DEMO}-regular")
    regular, big_proof = active_shot(app, plan, "loading-dark-rtl-48", bigger, 48., rtl=True)
    return {"rtl": True, "text_scale": 2, "24_dp": small_proof, "48_dp": big_proof,
            "captures": [small, regular]}


def wide(app, plan):
    frame = scene(app, "large")
    receipt, proof = active_shot(app, plan, "loading-wide-240", frame, 240.)
    return {"size": 240, "indicators": proof, "captures": [receipt]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--gallery", required=True, type=Path)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--source", required=True)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--contour-data", required=True, type=Path)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True)
    plan.polygon = reduced_polygon(plan.contour_data.read_text())
    report = {
        "source_manifest_sha256": plan.source,
        "gallery_sha256": digest(plan.gallery),
        "cli_sha256": digest(plan.cli),
        "contour_data_sha256": digest(plan.contour_data),
        "motion": "native headless Reduce Motion endpoint",
        "input": "native headless DevTools Action.key and Action.activate",
        "transports": ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"],
        "profiles": {},
    }
    for replay, width, height in ((light, 960, 760), (compact, 320, 900), (wide, 960, 760)):
        name = replay.__name__
        with launch(plan, name, width, height) as app:
            before = onscreen_windows(app.proc.pid)
            require(before == 0, f"{name} created a visible window")
            proof = replay(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(after == 0, f"{name} created a visible window")
            proof["visible_windows"] = [before, after]
            proof["viewport"] = [width, height]
        require(app.proc.poll() is not None, f"{name} did not close the Gallery")
        report["profiles"][name] = proof
        print(f"PASS {name} {width}x{height}. No visible windows.", flush=True)
    require({path.name for path in plan.out.iterdir()} == IMAGE_NAMES, "Native PNG set differs")
    (plan.out / REPORT_NAME).write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
