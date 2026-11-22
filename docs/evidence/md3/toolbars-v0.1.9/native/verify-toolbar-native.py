#!/usr/bin/env python3
"""Replay the MD3 toolbar with Metal and without a visible window."""
import argparse
import json
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, launch, same_box

ID = "toolbar-demo-"
BAR = ID + "bar"
CORE = ID + "core"
LEAD = ID + "lead"
EXTRA = ID + "extra"
DISABLED = ID + "disabled"
OVERFLOW = ID + "overflow"
MENU = ID + "overflow-menu"
FAB = ID + "action-fab"
WORKSPACE = ID + "workspace"
SCREENSHOT = {"scale": 1, "max_side": 1200}


def locate(frame, key):
    for index, node in enumerate(frame["dom"]["nodes"]):
        if node["stable_id"] == key:
            return node, frame["layout"]["nodes"][index]["bounds"]
    return None, None


def bounds(frame, key):
    return locate(frame, key)[1]


def close_enough(first, second):
    return abs(first - second) < 1


def capture(app, plan, name, frame, parity=False):
    path = plan.out / f"{name}.png"
    receipt = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    data = path.read_bytes()
    if parity:
        wire = app.wire("Page.screenshot", SCREENSHOT)
        mcp, mcp_receipt, _ = app.mcp(SCREENSHOT)
        require(data == png_payload(wire) == mcp, "PNG bytes differ across screenshot clients")
        require(metadata(receipt) == metadata(wire) == mcp_receipt, "Screenshot metadata differs")
    w, h, rows = read_png(data)
    require((w, h) == (receipt["width"], receipt["height"]), "PNG metadata mismatch")
    root, bar = locate(frame, BAR)
    sampled = None
    if bar is not None:
        vertical = bar["height"] > bar["width"]
        x = int(bar["x"] + (3 if vertical else bar["width"] / 2))
        y = int(bar["y"] + (bar["height"] / 2 if vertical else 3))
        sampled = list(rows[y][4 * x : 4 * x + 3])
        expected = root["style"]["background"][:3]
        require(max(abs(a - b) for a, b in zip(sampled, expected)) < 7,
                f"Wrong palette pixel {sampled} versus {expected}")
    return {"file": path.name, "sha256": digest(path), **metadata(receipt),
            "transport_parity": parity, "palette_pixel": sampled}


def require_named_geometry(frame, kind, rtl):
    group, bar = locate(frame, BAR)
    workspace = bounds(frame, WORKSPACE)
    require(group["name"] == "編輯工具列" and group["role"] == "Group", "Toolbar name or role is missing")
    require(same_box(workspace, bar), f"Toolbar exceeds workspace: {bar} in {workspace}")
    if kind == "vertical":
        require(close_enough(bar["width"], 64.) and close_enough(
            bar["x"] - workspace["x"] if rtl else
            workspace["x"] + workspace["width"] - bar["x"] - bar["width"], 16.),
            f"Wrong vertical placement: {bar}, workspace {workspace}")
    else:
        require(close_enough(bar["height"], 64.), f"Wrong horizontal toolbar height: {bar}")
        require(close_enough(workspace["y"] + workspace["height"] - bar["y"] - bar["height"],
                             0. if kind == "docked" else 16.), f"Wrong bottom placement: {bar}")
        if kind == "docked":
            require(close_enough(bar["width"], workspace["width"]), "Docked bar does not span layout")
    for action in [LEAD, CORE, DISABLED, EXTRA, OVERFLOW]:
        target = bounds(frame, action)
        require(target is not None and target["width"] >= 48 and target["height"] >= 48
                and same_box(bar, target), f"Invalid target for {action}: {target}")
    require(locate(frame, DISABLED)[0]["disabled"], "Disabled action is enabled")
    lead = bounds(frame, LEAD)
    core = bounds(frame, CORE)
    require(lead["y"] < core["y"] if kind == "vertical" else (lead["x"] > core["x"]) == rtl,
            f"Wrong logical action order, leading {lead}, core {core}")
    return bar


def keyboard(app, kind, rtl):
    key = "down" if kind == "vertical" else "left" if rtl else "right"
    app.activate(LEAD)
    app.key(key)
    require(focused(app.dom()) == CORE, "Axis arrow did not move focus to the toggle")
    app.key(key)
    require(focused(app.dom()) == EXTRA, "Axis arrow did not skip disabled action")
    app.wire("Action.activate", {"id": DISABLED}, "disabled")


def overflow(app):
    app.activate(OVERFLOW)
    frame = app.dom()
    require(locate(frame, MENU)[0]["role"] == "Menu", "Overflow did not open an accessible menu")
    require(focused(frame) == ID + "overflow-action", "Menu did not focus the first action")
    require(same_box(frame["layout"]["viewport"], bounds(frame, MENU)), "Overflow menu is outside viewport")
    app.wire("Action.activate", {"id": ID + "body-action"}, "outside_modal")
    app.key("escape")
    require(locate(app.dom(), MENU)[0] is None and focused(app.dom()) == OVERFLOW,
            "Escape did not restore the overflow trigger")
    app.activate(OVERFLOW)
    app.activate(ID + "overflow-action")
    require(locate(app.dom(), MENU)[0] is None and focused(app.dom()) == OVERFLOW,
            "Menu item did not close and restore the trigger")


def open_toolbar(app, dark=False, rtl=False, large=False):
    app.activate("section-1")
    app.activate(ID + "show")
    require(focused(app.dom()) == ID + "back", "Opening fixture did not set focus to Back")
    for enabled, control in [(dark, "theme"), (rtl, ID + "rtl"), (large, ID + "scale")]:
        if enabled:
            app.activate(control)


def desktop(app, plan):
    open_toolbar(app)
    variants = []
    for kind in ["docked", "horizontal", "vertical"]:
        app.activate(ID + kind)
        for palette in ["standard", "vibrant"]:
            app.activate(ID + palette)
            frame = app.dom()
            geometry = require_named_geometry(frame, kind, False)
            variants.append({"kind": kind, "palette": palette, "bounds": geometry,
                             "capture": capture(app, plan, f"light-{kind}-{palette}", frame)})
        keyboard(app, kind, False)
    before = locate(app.dom(), CORE)[0]["checked"]
    app.activate(CORE)
    require(locate(app.dom(), CORE)[0]["checked"] is not before, "Toggle did not update checked state")
    overflow(app)
    app.activate(ID + "horizontal")
    app.activate(ID + "fab")
    paired = app.dom()
    toolbar = bounds(paired, BAR)
    fab = bounds(paired, FAB)
    require(fab is not None and close_enough(fab["x"] - toolbar["x"] - toolbar["width"], 8.),
            f"Adjacent FAB placement is wrong: {toolbar}, {fab}")
    pair = capture(app, plan, "light-horizontal-paired", paired, parity=True)
    app.activate(ID + "preview")
    collapsed = app.dom()
    require(bounds(collapsed, BAR) is None and bounds(collapsed, FAB) is not None,
            "Paired collapse did not keep only the FAB")
    fab_only = capture(app, plan, "light-horizontal-collapsed", collapsed)
    app.activate(ID + "preview")
    hidden = app.dom()
    require(bounds(hidden, BAR) is None and bounds(hidden, FAB) is None,
            "Hidden state left an interactive toolbar or FAB")
    app.activate(ID + "body-action")
    return {"variants": variants, "paired": pair, "paired_collapsed": fab_only}


def compact(app, plan):
    open_toolbar(app, dark=True, rtl=True, large=True)
    app.activate(ID + "vibrant")
    app.activate(ID + "horizontal")
    frame = app.dom()
    require_named_geometry(frame, "horizontal", True)
    keyboard(app, "horizontal", True)
    horizontal = capture(app, plan, "dark-rtl-200-horizontal", app.dom(), parity=True)
    app.activate(ID + "fab")
    paired = app.dom()
    workspace, surface, fab = (bounds(paired, key) for key in (WORKSPACE, BAR, FAB))
    require(same_box(workspace, surface) and same_box(workspace, fab),
            "The paired toolbar or FAB exceeds the compact workspace")
    separated = any(surface[p] + surface[m] <= fab[p] or fab[p] + fab[m] <= surface[p]
                    for p, m in (("x", "width"), ("y", "height")))
    require(separated, "The compact toolbar and FAB overlap")
    pair = capture(app, plan, "dark-rtl-200-paired", paired, parity=True)
    app.activate(ID + "fab")
    app.activate(ID + "vertical")
    frame = app.dom()
    require_named_geometry(frame, "vertical", True)
    heading = bounds(frame, ID + "row-title-0")
    require(heading["height"] > 36, "Compact report did not use large CJK text")
    keyboard(app, "vertical", True)
    vertical = capture(app, plan, "dark-rtl-200-vertical", app.dom(), parity=True)
    overflow(app)
    app.activate(ID + "preview")
    collapsed = app.dom()
    require(bounds(collapsed, CORE) is not None and bounds(collapsed, LEAD) is None
            and bounds(collapsed, OVERFLOW) is None, "Unpaired collapse did not show only core actions")
    shot = capture(app, plan, "dark-rtl-200-core-only", collapsed, parity=True)
    return {"horizontal": horizontal, "paired": pair, "vertical": vertical, "core_only": shot}


def main():
    parser = argparse.ArgumentParser()
    for key in ("gallery", "cli", "out"):
        parser.add_argument("--" + key, type=Path, required=True)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True, exist_ok=False)
    results = {"source_manifest_sha256": plan.source}
    for profile, w, h, verify in [
        ("desktop", 960, 800, desktop), ("dark-rtl-200", 320, 580, compact)
    ]:
        with launch(plan, profile, w, h) as app:
            results[profile] = verify(app, plan)
            print("PASS", profile, flush=True)
    (plan.out / "native-toolbar-report.json").write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
