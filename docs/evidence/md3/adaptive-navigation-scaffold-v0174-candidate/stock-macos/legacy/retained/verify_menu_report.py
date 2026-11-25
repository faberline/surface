#!/usr/bin/env python3
"""Validate M267–M269 from actual Surface Chrome reports and decoded PNGs."""

import argparse
from collections import Counter
from hashlib import sha256
import json
import math
from pathlib import Path
import re

from PIL import Image, ImageChops

STEPS = ("M267", "M268", "M269")
STEM = "material-command-menu"
VIEWPORTS = {"wide": (960, 800), "compact": (320, 480), "compact-footer": (320, 480)}
ADAPTER_FIELDS = ("isFallbackAdapter", "vendor", "architecture")


def need(condition, explanation):
    if not condition:
        raise ValueError(explanation)


def read_json(path):
    return json.loads(path.read_text())


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def integer(value):
    return type(value) is int


def rect(value):
    finite = lambda n: type(n) in (int, float) and math.isfinite(n)
    need(isinstance(value, list) and len(value) == 4 and all(map(finite, value)),
         f"invalid rectangle: {value!r}")
    need(value[2] > 0 and value[3] > 0, f"empty rectangle: {value!r}")
    return value


def inside(outer, inner):
    ox, oy, ow, oh = outer
    x, y, w, h = inner
    left, top = max(ox, x), max(oy, y)
    overlap = (left, top, min(ox + ow, x + w) - left, min(oy + oh, y + h) - top)
    return all(abs(a - b) <= 0.75 for a, b in zip(overlap, inner))


def adapter_fields(value):
    need(isinstance(value, dict) and set(ADAPTER_FIELDS) <= value.keys(),
         "missing observed WebGPU adapter metadata")
    need(type(value["isFallbackAdapter"]) is bool, "adapter fallback status is not a boolean")
    need(all(isinstance(value[key], str) for key in ADAPTER_FIELDS[1:]),
         "adapter vendor and architecture must be observed strings")
    return {key: value[key] for key in ADAPTER_FIELDS}


def decode(path, viewport):
    with Image.open(path) as image:
        need(image.format == "PNG" and image.mode in ("RGB", "RGBA"), f"wrong PNG format: {path.name}")
        image.verify()
    with Image.open(path) as image:
        image.load()
        canvas_width, canvas_height = viewport
        need(image.size == (2 * canvas_width, 2 * (canvas_height + 44)),
             f"{path.name} has unexpected screenshot dimensions {image.size}")
        return image.convert("RGB")


def device_point(image, viewport, x, y):
    scale = image.width / viewport[0]
    header = image.height / scale - viewport[1]
    return (math.floor(scale * x), math.floor(scale * (y + header)))


def canvas_crop(image, viewport, bounds):
    x, y, width, height = bounds
    return image.crop((*device_point(image, viewport, x, y),
                       *device_point(image, viewport, x + width, y + height)))


def footer_geometry(compact, trigger):
    scroller = rect(compact["scroller"])
    before = rect(compact["status_before"])
    after = rect(compact["status_after"])
    clearance = trigger[1] - scroller[1] - scroller[3]
    need(inside((0, 0, *VIEWPORTS["compact"]), scroller), "footer scroller escaped the viewport")
    need(clearance >= 8, "the footer has less than 8 dp clearance")
    need(not inside(scroller, before), "the status was already fully visible before the wheel")
    need(inside(scroller, after), "wheel did not reveal all of the status")
    need(before[1] > after[1], "the wheel did not move the status")
    need(before[0] == after[0] and before[2:] == after[2:], "scroll changed status geometry")
    if "trigger_after" in compact:
        need(rect(compact["trigger_after"]) == trigger, "the footer trigger moved")
    return {"scroller": scroller, "status_before": before, "status_after": after,
            "clearance_dp": clearance, "scroll_dp": before[1] - after[1]}


def footer_paint(opened, footer, panel, footer_info):
    viewport = VIEWPORTS["compact"]
    need(ImageChops.difference(canvas_crop(opened, viewport, panel),
                               canvas_crop(footer, viewport, panel)).getbbox() is not None,
         "the footer PNG did not replace the painted open menu")
    status = canvas_crop(footer, viewport, footer_info["status_after"])
    colors = Counter(status.get_flattened_data() if hasattr(status, "get_flattened_data") else status.getdata())
    backdrop = colors.most_common(1)[0][0]
    need(max(backdrop) < 112, f"status area does not have a dark backdrop: {backdrop}")
    bright = sum(count for rgb, count in colors.items() if min(rgb) - max(backdrop) >= 64)
    need(bright >= 24, "the revealed status has no readable light ink in its own bounds")
    return {"status_backdrop_rgb": backdrop, "contrasting_status_pixels": bright}


def validate_windows(path, platform, stock, pre_footer):
    if path is None:
        need(platform == "linux", "macOS requires the observed no-window receipt")
        return None
    data = read_json(path)
    if "observations" in data:
        need(data.get("stock_exit") == 0, "stock window observer failed")
        need(data.get("visible_owned_windows") == [] and data.get("remaining_observed_processes") == [],
             "visible window or leftover process in stock observation")
        need(isinstance(data.get("method"), str) and "CoreGraphics" in data["method"],
             "missing macOS stock visible-window observer method")
        series = data["observations"]
        browsers = series["chrome"]["samples"]
        native = series["native_gallery"]["samples"]
        need(integer(native) and native >= 0 and (not stock or native > 0),
             "stock replay did not sample the offscreen Gallery")
    else:
        need(not stock, "use the stock window receipt for a full stock run")
        need(data.get("exit") == 0, "focused window observer failed")
        need(data.get("visible_windows") == [] and data.get("remaining_chrome") == [],
             "visible window or leftover Chrome process in focused observation")
        browsers = data["samples"]
        native = None
    need(integer(browsers) and browsers > 0, "no actual Chrome window samples")
    if pre_footer:
        need(browsers == 294, "interim reference observation has changed")
    return {"chrome_process_samples": browsers, "native_process_samples": native,
            "visible_windows": 0, "remaining_observed_processes": 0, "receipt_sha256": digest(path)}


def validate(attempt, *, platform, log=None, window_receipt=None, expected_metadata=None,
             scope="focused", pre_footer=False, stock_material_steps=268):
    """Return independent results. `attempt` holds the PNGs and exact Menu report."""
    attempt = Path(attempt)
    log_path = Path(log) if log is not None else attempt / "runner.log"
    output = log_path.read_text()
    stock = scope == "stock"
    material_steps = tuple(re.findall(r"^\s*\[(M\d+)\]", output, re.M))
    need(tuple(step for step in material_steps if step in STEPS) == STEPS,
         "required Menu steps are missing, repeated or out of sequence")
    counts = re.findall(r"case material\s+\.\.\.\s+ok \(([0-9]+) steps, ([0-9]+) checks", output)
    need(len(counts) == 1, "missing successful Material case result")
    step_count, check_count = map(int, counts[0])
    need(len(material_steps) == step_count and step_count == (stock_material_steps if stock else 3),
         "runner step count differs from the selected case scope")
    need(check_count >= (117 if pre_footer else 122), "too few actual Material checks")
    browser_names = set(re.findall(r"\bbrowser: (Chrome/[0-9][0-9.]+)\b", output))
    report_path = attempt / f"browser-{STEM}.json"
    report = read_json(report_path)
    need(report["errors"] == [] and not {"failure", "state_at_failure"} & report.keys(),
         "Menu browser run reported a failure or page error")
    browser = report["browser"]
    need(browser_names == {browser}, "browser version differs from the actual runner log")
    adapter = None if pre_footer else adapter_fields(report.get("adapter"))
    if expected_metadata is not None:
        need(expected_metadata["browser"] == browser and adapter_fields(expected_metadata["adapter"]) == adapter,
             "browser or adapter differs from the independently observed metadata")

    keyboard = report["keyboard"]
    need(keyboard["disabled_focus"] is True, "disabled focus was not exercised")
    wide_panel, wide_trigger = rect(keyboard["panel"]), rect(keyboard["trigger"])
    need(inside((0, 0, *VIEWPORTS["wide"]), wide_panel), "wide popup escaped the Canvas")
    need(wide_panel[2] == 240 and wide_panel[1] >= wide_trigger[1] + wide_trigger[3],
         "wide menu placement or width is wrong")
    need(report["dismissal"] == {"tab_forward": "menu-demo-list", "tab_backward": "menu-demo-bottom",
                                 "outside_action_suppressed": True},
         "Tab or click-through result was wrong")
    compact = report["compact"]
    panel, trigger, last = (rect(compact[key]) for key in ("panel", "trigger", "last"))
    need(inside((0, 0, *VIEWPORTS["compact"]), trigger) and trigger[2] >= 48 and trigger[3] >= 48,
         "the compact trigger was clipped or smaller than its target")
    need(inside((8, 8, 304, 464), panel) and panel[1] + panel[3] <= trigger[1],
         "compact menu did not stay in the safe viewport and above the trigger")
    need(inside(panel, last) and last[3] >= 48 and compact["wrapped_height"] > 48,
         "the last command was clipped or the large label did not wrap")
    state = compact["state"]
    need(state["visible"] is True and state["rtl"] is True and state["bottom_anchor"] is True
         and state["text_scale"] == 2 and state["open"] is False and state["outside_actions"] == 0,
         "final state did not preserve the compact RTL, large text and dismissal settings")
    need(state["last"] == "移到收件匣", "expected earlier command selection was lost")

    shots = {name: attempt / f"{STEM}-{name}.png" for name in VIEWPORTS if not pre_footer or name != "compact-footer"}
    need(set(attempt.glob(f"{STEM}-*.png")) == set(shots.values()), "Menu PNG set is incomplete or contains a failure")
    images = {name: decode(path, VIEWPORTS[name]) for name, path in shots.items()}
    wx, wy, _, wh = wide_panel
    wide = images["wide"]
    sample = lambda x, y: list(wide.getpixel(device_point(wide, VIEWPORTS["wide"], x, y)))
    center = sample(wx + 3, wy + wh / 2)
    corner = sample(wx, wy)
    need(center != corner and center == keyboard["center_rgb"] and corner == keyboard["corner_rgb"],
         "decoded wide panel pixels do not match the reported Canvas paint")
    if pre_footer:
        need(not {"scroller", "status_before", "status_after"} & compact.keys(),
             "pre-footer mode only accepts the older schema")
        footer = None
    else:
        footer = footer_geometry(compact, trigger)
        footer.update(footer_paint(images["compact"], images["compact-footer"], panel, footer))
    if window_receipt is None:
        name = "headless-window-observation.json" if stock else "headless-observation.json"
        observation = attempt / name
        window_receipt = observation if observation.is_file() else None
    windows = validate_windows(Path(window_receipt) if window_receipt else None, platform, stock, pre_footer)
    return {"mode": "interim pre-footer" if pre_footer else "final Menu footer required",
            "platform": platform, "case_scope": scope, "menu_steps": STEPS,
            "material_steps": step_count, "material_checks": check_count, "page_errors": 0,
            "report_sha256": digest(report_path), "runner_log_sha256": digest(log_path),
            "browser": browser, "adapter": adapter, "window_observation": windows,
            "wide_canvas_samples": {"center": center, "corner": corner}, "footer": footer,
            "pngs": {name: {"size": images[name].size, "sha256": digest(path)} for name, path in shots.items()}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("attempt", type=Path)
    parser.add_argument("--platform", required=True, choices=["macos", "linux"])
    parser.add_argument("--scope", choices=["focused", "stock"], default="focused")
    parser.add_argument("--log", type=Path)
    parser.add_argument("--window-receipt", type=Path)
    parser.add_argument("--expected-metadata", type=Path, help="Optional same-run JSON with browser and adapter")
    parser.add_argument("--pre-footer", action="store_true", help="Only for the sealed interim local focused result")
    args = parser.parse_args()
    need(not args.pre_footer or (args.platform, args.scope) == ("macos", "focused"),
         "pre-footer mode is limited to the interim local three-step result")
    try:
        print(json.dumps(validate(args.attempt, platform=args.platform, log=args.log,
                                  window_receipt=args.window_receipt, scope=args.scope,
                                  pre_footer=args.pre_footer, expected_metadata=(
                                      read_json(args.expected_metadata) if args.expected_metadata else None)), indent=2))
    except (ValueError, KeyError, OSError, SyntaxError) as error:
        parser.exit(1, f"Menu verification failed: {error}\n")


if __name__ == "__main__":
    main()
