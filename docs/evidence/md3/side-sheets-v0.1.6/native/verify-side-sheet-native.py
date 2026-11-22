#!/usr/bin/env python3
"""Verify Material 3 side sheets through the native Surface Gallery."""

import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import time

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, launch, same_box


SHEET = "sheet-demo-panel"
CLOSE = SHEET + "-close"
CONTENT = SHEET + "-content"
HEADER = SHEET + "-header"
LAYER = SHEET + "-layer"
BODY = "sheet-demo-main"
MAIN_ACTION = "sheet-demo-main-action"
PANEL_ACTION = "sheet-demo-action"
DISABLED = "sheet-demo-disabled"
LAST = "sheet-demo-last"
OPEN = "sheet-demo-open"
SHOW = "side-sheet-demo-show"
SCREENSHOT = {"scale": 1, "max_side": 1200}

# Presentation, style and physical edge for the 960 dp left-to-right window.
DESKTOP_CASES = [
    ("coplanar", "docked", "end"),
    ("coplanar", "detached", "start"),
    ("overlay", "docked", "start"),
    ("overlay", "detached", "end"),
    ("modal", "docked", "end"),
    ("modal", "detached", "start"),
]


def locate(frame, stable_id):
    ids = [i for i, node in enumerate(frame["dom"]["nodes"]) if node["stable_id"] == stable_id]
    require(len(ids) == 1, f"Expected one node for {stable_id}, got {ids}")
    return frame["dom"]["nodes"][ids[0]], frame["layout"]["nodes"][ids[0]]


def bounds(frame, stable_id):
    return locate(frame, stable_id)[1]["bounds"]


def absent(frame, stable_id):
    return all(node["stable_id"] != stable_id for node in frame["dom"]["nodes"])


def await_frame(app, ready, message):
    timeout = time.monotonic() + 4
    while True:
        current = app.dom()
        if ready(current):
            return current
        require(time.monotonic() < timeout, message)
        time.sleep(.03)


def capture(app, destination, name, parity=False):
    path = destination / f"{name}.png"
    def cli_screenshot(output):
        return app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", output])

    attempts = None
    if parity:
        # Focus can change while separate clients connect. Compare only
        # frames bracketed by identical raw captures.
        with tempfile.TemporaryDirectory(prefix="side-sheet-frame-", dir=destination.parent) as work:
            for attempt in range(1, 7):
                before = app.wire("Page.screenshot", SCREENSHOT)
                candidate = Path(work) / f"{attempt}.png"
                cli = cli_screenshot(candidate)
                mcp_image, mcp_metadata, _ = app.mcp(SCREENSHOT)
                after = app.wire("Page.screenshot", SCREENSHOT)
                if png_payload(before) != png_payload(after):
                    time.sleep(.15)
                    continue
                require(candidate.read_bytes() == png_payload(after) == mcp_image,
                        "The three transports returned different bytes for the same frame")
                require(metadata(cli) == metadata(before) == metadata(after) == mcp_metadata,
                        "PNG metadata differs across transports")
                candidate.rename(path)
                attempts = attempt
                break
            else:
                raise RuntimeError("No stable frame was available for the three-client PNG check")
    else:
        cli = cli_screenshot(path)
    payload = path.read_bytes()
    width, height, rows = read_png(payload)
    viewport = app.dom()["layout"]["viewport"]
    require((width, height) == (viewport["width"], viewport["height"]), "PNG window size is wrong")
    result = {"file": path.name, "metadata": metadata(cli), "sha256": digest(path)}
    if parity:
        result["transport_parity"] = ["raw", "cli", "mcp"]
        result["same_frame_attempt"] = attempts
    return result, rows


def ax_read(plan, app, *command):
    if not plan.ax_reader:
        return {"trusted": False}
    timeout = time.monotonic() + 5
    while True:
        result = subprocess.run([plan.ax_reader, str(app.proc.pid), *command],
                                capture_output=True, check=True, timeout=15)
        state = json.loads(result.stdout)
        if (not state["trusted"] or state["objects"] or state.get("action_status") == 0
                or time.monotonic() >= timeout):
            return state
        app.dom()
        time.sleep(.1)


def check_accesskit(plan, app, modal, require_disabled=True):
    ax = ax_read(plan, app)
    if not ax["trusted"]:
        return {"verified": False, "reason": "AppKit reader or Accessibility permission is missing"}
    buttons = {entry["name"]: entry for entry in ax["objects"] if entry["role"] == "AXButton"}
    for name in ("關閉側邊面板", "儲存設定"):
        require(name in buttons and buttons[name]["enabled"] and "AXPress" in buttons[name]["actions"],
                f"AppKit did not expose {name}: {ax}")
    disabled = buttons.get("解除封存")
    # A disabled, fully clipped control has no scroll-reveal action and
    # Mac's projection omits it until it enters the scroll viewport.
    require((disabled is not None or not require_disabled)
            and (disabled is None or not disabled["enabled"]), "Wrong disabled state in AppKit")
    require(("更新頁面" in buttons) != modal, f"Wrong AppKit interaction scope: {ax}")
    if modal:
        require(any(node["subrole"] == "AXDialog" and node["name"] == "工作區詳細資料"
                    for node in ax["objects"]), f"Named dialog missing from AppKit: {ax}")
        if ax["active"]:
            require(buttons["關閉側邊面板"]["focused"], "Modal Close is not the first AppKit focus")
    return {"verified": True, **ax}


def perform_ax(plan, app, command, effect):
    deadline = time.monotonic() + 8
    attempts = []
    while time.monotonic() < deadline:
        focus = ax_read(plan, app, "focus")
        require(focus["trusted"], "The native action needs Accessibility permission")
        if not focus.get("active"):
            time.sleep(.1)
            continue
        action = ax_read(plan, app, command)
        attempts.append({key: action.get(key) for key in ("active", "root_source", "action_status")})
        until = min(deadline, time.monotonic() + .8)
        while time.monotonic() < until:
            frame = app.dom()
            if effect(frame):
                require(action.get("action_status") == 0, f"Unexpected AppKit result: {action}")
                return {"command": command, "attempts": attempts}, frame
            time.sleep(.035)
    raise RuntimeError(f"AppKit {command} did not take effect in the focused Gallery: {attempts}")


def sample_rgb(rows, x, y):
    x, y = int(x), int(y)
    require(x >= 0 and y >= 0 and y < len(rows) and x * 4 < len(rows[y]), f"Sample outside frame {(x, y)}")
    return list(rows[y][x * 4:x * 4 + 3])


def distance(a, b):
    return max(abs(x - y) for x, y in zip(a, b))


def check_paint(frame, open_rows, closed_rows, panel, left):
    actual = sample_rgb(open_rows, panel["x"] + 6, panel["y"] + panel["height"] * .65)
    expected = locate(frame, SHEET)[0]["style"]["background"][:3]
    require(distance(actual, expected) <= 8, f"Wrong panel color: {actual}, expected {expected}")
    viewport = frame["layout"]["viewport"]
    sample_at = (viewport["width"] - 12 if left else 12, viewport["height"] - 12)
    before, after = sample_rgb(closed_rows, *sample_at), sample_rgb(open_rows, *sample_at)
    rgba = locate(frame, LAYER)[0]["style"]["background"]
    if rgba[3] == 0:
        require(distance(before, after) <= 3, f"Standard panel dimmed the page: {before}/{after}")
        return {"panel": actual, "body_before": before, "body_after": after}
    tint = [round(base + (ink - base) * rgba[3] / 255) for base, ink in zip(before, rgba)]
    require(rgba[3] > 0 and distance(after, tint) <= 8 and distance(after, before) > 5,
            f"Modal scrim has wrong paint: {before}/{after}, expected {tint}")
    return {"panel": actual, "body_before": before, "body_after": after, "scrim": rgba}


def prove_case(plan, app, name, mode, style, edge, rtl, width, height):
    compact = width < 600
    effective = "modal" if compact and mode == "coplanar" else mode
    modal = effective == "modal"
    left = (edge == "start") != rtl
    inset = 16 if style == "detached" else 0

    app.activate(f"sheet-demo-{mode}")
    app.activate(f"sheet-demo-{style}")
    app.activate(f"sheet-demo-edge-{edge}")
    start = app.dom()
    require(absent(start, SHEET), "Side sheet unexpectedly present before opening")
    body_before = bounds(start, BODY)
    background, closed_rows = capture(app, plan.out, name + "-closed")
    app.activate(OPEN)

    def ready(frame):
        if absent(frame, SHEET):
            return False
        panel = bounds(frame, SHEET)
        anchor = inset if left else width - inset - panel["width"]
        return abs(panel["x"] - anchor) < 1

    frame = await_frame(app, ready, f"{name}: side sheet did not reach its logical edge")
    panel = bounds(frame, SHEET)
    body = bounds(frame, BODY)
    require(panel["width"] == min(256, width - inset * 2)
            and panel["height"] == height - inset * 2 and panel["y"] == inset,
            f"Wrong geometry for {name}: {panel}")
    role = locate(frame, SHEET)[0]["role"]
    require(role == ("Dialog" if modal else "Group"), f"Wrong panel role for {name}: {role}")
    require((locate(frame, LAYER)[0]["style"]["background"][3] > 0) == modal,
            f"Wrong modal layer state for {name}")
    if effective == "coplanar":
        fits = (body["x"] + 1 >= panel["x"] + panel["width"] if left
                else body["x"] + body["width"] <= panel["x"] + 1)
        require(fits and body["width"] < body_before["width"], f"Coplanar sheet did not reflow body: {body}")
    else:
        require(body == body_before, f"{effective} unexpectedly shrank the page")
    require(locate(frame, DISABLED)[0]["disabled"], "Disabled panel control became enabled")

    if modal:
        require(focused(frame) == CLOSE, f"Modal entry focus was {focused(frame)}")
        app.wire("Action.activate", {"id": MAIN_ACTION}, "outside_modal")
    else:
        require(focused(frame) == OPEN, f"Standard sheet unexpectedly took focus: {focused(frame)}")
        app.activate(MAIN_ACTION)
        require(focused(app.dom()) == MAIN_ACTION, "Standard body did not receive focus")
        app.key("escape")
        require(not absent(app.dom(), SHEET), "Escape on main content closed a standard sheet")

    shot, rows = capture(app, plan.out, name + "-open")
    paint = check_paint(frame, rows, closed_rows, panel, left)
    reveal = None
    app.wire("Action.activate", {"id": DISABLED}, "disabled")
    if compact:
        long, long_layout = locate(frame, "sheet-demo-long")
        require(long["text"] and long_layout["bounds"]["height"] > 64, "Large Chinese text did not wrap")
        before = bounds(frame, HEADER)
        require(not same_box(bounds(frame, CONTENT), bounds(frame, LAST)), "Last item should need scroll")
        app.activate(LAST)
        frame = await_frame(app, lambda s: focused(s) == LAST and same_box(bounds(s, CONTENT), bounds(s, LAST)),
                            "Focused last control was clipped")
        require(bounds(frame, HEADER) == before, "Scrolling moved the sheet header")
        scroll_shot, _ = capture(app, plan.out, name + "-scrolled", parity=True)
        reveal = {"focus": focused(frame), "last": bounds(frame, LAST), "capture": scroll_shot}
    else:
        app.activate(PANEL_ACTION)
        require(focused(app.dom()) == PANEL_ACTION, "Panel action did not receive focus")

    app.key("escape")
    final = await_frame(app, lambda s: absent(s, SHEET), f"{name}: side sheet did not close")
    require(focused(final) == OPEN, f"Closing {name} did not restore opener: {focused(final)}")
    require(bounds(final, BODY) == body_before, "Body layout did not return")
    return {
        "presentation": mode, "effective": effective, "style": style, "edge": edge,
        "geometry": {"sheet": panel, "body": body}, "paint": paint, "opening": shot,
        "background": background, "reveal": reveal,
    }


def open_gallery(app, dark=False, rtl=False, enlarged=False):
    app.activate("section-2")
    app.activate(SHOW)
    require(focused(app.dom()) == "sheet-demo-back", "Focus did not enter the demo")
    if dark:
        app.activate("theme")
    if rtl:
        app.activate("sheet-demo-rtl")
    if enlarged:
        app.activate("sheet-demo-scale")


def verify_appkit(plan):
    if not plan.ax_reader:
        return {"verified": False, "reason": "AppKit reader omitted"}
    # Use fresh foreground windows. The native host drops actions that arrive
    # after the window loses focus. Keep the accessibility action run free of
    # screenshot calls so its result can be attributed to the action alone.
    with launch(plan, "modal AppKit action", 960, 800, True) as (app, _):
        open_gallery(app)
        app.activate("sheet-demo-modal")
        app.activate(OPEN)
        modal = check_accesskit(plan, app, True)
        if not modal.get("verified"):
            return modal
        close, final = perform_ax(plan, app, "close", lambda frame: absent(frame, SHEET))
        require(focused(final) == OPEN, "AppKit Close did not return focus")
        close["focus_after"] = focused(final)
    with launch(plan, "standard AppKit tree", 960, 800, True) as (app, _):
        open_gallery(app)
        app.activate(OPEN)
        standard = check_accesskit(plan, app, False)
    with launch(plan, "compact AppKit page scroll", 320, 480, True) as (app, _):
        open_gallery(app, dark=True, rtl=True, enlarged=True)
        app.activate("sheet-demo-detached")
        app.activate(OPEN)
        compact = check_accesskit(plan, app, True, require_disabled=False)
        frame = app.dom()
        before = bounds(frame, LAST)["y"]
        header = bounds(frame, HEADER)
        scroll, final = perform_ax(plan, app, "scroll", lambda current: bounds(current, LAST)["y"] < before - 2)
        require(bounds(final, HEADER) == header, "AppKit scroll displaced the header")
        scroll["last_before"] = before
        scroll["last_after"] = bounds(final, LAST)["y"]
    return {"verified": True, "standard": standard, "modal": modal, "compact": compact,
            "close": close, "scroll": scroll}


def main():
    parser = argparse.ArgumentParser()
    for key in ("gallery", "cli", "out"):
        parser.add_argument("--" + key, type=Path, required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--headless", action="store_true")
    mode.add_argument("--ax-reader", type=Path)
    parser.add_argument("--profile-launcher", type=Path)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    if plan.ax_reader and not plan.profile_launcher:
        parser.error("--ax-reader requires --profile-launcher for the compact AppKit case")
    plan.out.mkdir(parents=True, exist_ok=False)
    if plan.ax_reader:
        accessibility = verify_appkit(plan)
        (plan.out / "appkit-actions.json").write_text(json.dumps(accessibility, ensure_ascii=False, indent=2) + "\n")
        print("AppKit actions and tree checked", flush=True)
        return
    receipts = {}
    for name, dark, rtl, w, h, cases in [
        ("light-desktop", False, False, 960, 800, DESKTOP_CASES),
        ("dark-rtl-desktop", True, True, 960, 800, [
            ("overlay", "detached", "end"), ("modal", "docked", "start")]),
        ("dark-rtl-200-narrow", True, True, 320, 480, [("coplanar", "detached", "end")]),
    ]:
        with launch(plan, name, w, h, True) as (app, _):
            open_gallery(app, dark, rtl, w < 600)
            receipts[name] = []
            for mode, style, edge in cases:
                case = f"{name}-{mode}-{style}-{edge}"
                receipts[name].append(prove_case(plan, app, case, mode, style, edge, rtl, w, h))
                print(f"PASS {case}", flush=True)
    (plan.out / "native-side-sheet-report.json").write_text(json.dumps({
        "source_manifest_sha256": plan.source,
        "mode": "headless",
        "profiles": receipts,
    }, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
