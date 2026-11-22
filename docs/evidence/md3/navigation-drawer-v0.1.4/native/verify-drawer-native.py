#!/usr/bin/env python3
"""Replay Material navigation drawers in the native Surface Gallery."""

import argparse
import json
from pathlib import Path
import stat
import subprocess

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box


PROFILES = {
    "light-desktop": (960, 800, False, False),
    "dark-desktop": (960, 800, True, False),
    "dark-rtl-200-compact": (320, 480, True, True),
    "dark-rtl-200-short": (320, 240, True, True),
}
OPEN = "drawer-demo-open"
CLOSE = "drawer-demo-close"
PANEL = "drawer-demo"
STANDARD = "drawer-standard"
SHARED = "drawer-demo-shared"
TOGGLE = "drawer-demo-standard-toggle"
SCALE = {"scale": 1, "max_side": 1200}


def locate(frame, stable_id):
    matches = [(node, frame["layout"]["nodes"][i])
               for i, node in enumerate(frame["dom"]["nodes"])
               if node["stable_id"] == stable_id]
    require(len(matches) == 1, f"Expected one node for {stable_id}")
    return matches[0]


def bounds(frame, stable_id):
    return locate(frame, stable_id)[1]["bounds"]


def absent(frame, stable_id):
    return all(node["stable_id"] != stable_id for node in frame["dom"]["nodes"])


def capture(app, folder, name, parity=False):
    path = folder / f"{name}.png"
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    data = path.read_bytes()
    width, height, rows = read_png(data)
    viewport = app.dom()["layout"]["viewport"]
    require((width, height) == (viewport["width"], viewport["height"])
            and (cli["width"], cli["height"], cli["scale"]) == (width, height, 1),
            "PNG dimensions differ from the logical viewport")
    require(stat.S_IMODE(path.stat().st_mode) == 0o600, "CLI PNG should be private")
    receipt = {"file": path.name, "sha256": digest(path), "metadata": metadata(cli)}
    if parity:
        wire = app.wire("Page.screenshot", SCALE)
        mcp, mcp_meta, _ = app.mcp(SCALE)
        require(png_payload(wire) == data == mcp, "Raw, CLI and MCP PNG bytes differ")
        require(metadata(wire) == metadata(cli) == mcp_meta, "Transport metadata differs")
        receipt["transport_parity"] = ["raw", "cli", "mcp"]
    return receipt, rows


def rgb(rows, x, y):
    return list(rows[int(y)][int(x) * 4:int(x) * 4 + 3])


def painted(frame, stable_id, rows, point):
    authored = locate(frame, stable_id)[0]["style"]["background"][:3]
    observed = rgb(rows, *point)
    require(max(abs(a - b) for a, b in zip(authored, observed)) <= 8,
            f"Wrong fill for {stable_id} at {point}: {observed} vs {authored}")
    return {"observed": observed, "authored": authored}


def report_ax(plan, app, *labels):
    if not plan.ax_reader:
        return {"trusted": False}
    process = subprocess.run([plan.ax_reader, str(app.proc.pid), *labels],
                             capture_output=True, check=True, timeout=15)
    return json.loads(process.stdout)


def matches(ax, name):
    return [row for row in ax["matches"] if row["name"] == name]


def verify_modal_ax(plan, app, current):
    ax = report_ax(plan, app, "首頁", "收件匣", "封存", "關閉抽屜", "重新整理")
    if not ax["trusted"]:
        return {"verified": False, "reason": "No macOS Accessibility permission or reader"}
    active = [node for node in ax["matches"] if node.get("current") == "page"]
    require(len(active) == 1 and active[0]["name"] == current
            and active[0]["enabled"] and "AXPress" in active[0]["actions"],
            f"Expected the current page and its action in AppKit: {ax}")
    disabled = matches(ax, "封存")
    require(len(disabled) == 1 and not disabled[0]["enabled"] and "AXPress" not in disabled[0]["actions"],
            f"Expected a disabled AppKit control: {ax}")
    close = matches(ax, "關閉抽屜")
    require(len(close) == 1 and close[0]["focused"] and "AXPress" in close[0]["actions"]
            and not matches(ax, "重新整理"), "AppKit did not focus Close or hide the backdrop")
    return {"verified": True, **ax}


def run_profile(plan, name, profile, native_ax):
    width, height, dark, compact = profile
    with launch(plan, name, width, height, compact) as (app, _):
        app.activate("section-2")
        app.activate("drawer-demo-show")
        require(focused(app.dom()) == "drawer-demo-back", "Preview did not receive focus")
        if dark:
            app.activate("theme")
        if compact:
            app.activate("drawer-demo-rtl")
            app.activate("drawer-demo-scale")

        app.activate(SHARED)
        require(focused(app.dom()) == SHARED, "Main control is not active beside the drawer")
        app.activate(STANDARD + "-home")
        app.key("down")
        require(focused(app.dom()) == STANDARD + "-inbox", "Focus should skip the disabled row")
        app.key("enter")
        frame = app.dom()
        require(locate(frame, STANDARD + "-inbox")[0]["current_page"],
                "Standard selection did not set the current page")
        panel = bounds(frame, STANDARD)
        content = bounds(frame, STANDARD + "-content")
        layout = bounds(frame, STANDARD + "-layout")
        require(content["width"] > 0 and panel["height"] >= 76,
                "The window did not leave space for both regions")
        if compact:
            require(abs(panel["width"] - layout["width"] * .6) <= 1
                    and abs(panel["x"] + panel["width"] - layout["x"] - layout["width"]) <= 1,
                    "RTL standard panel failed to use logical start")
        else:
            require(panel["width"] == 360 and panel["x"] < content["x"],
                    "Standard drawer should be 360 dp on the left")
        standard, rows = capture(app, plan.out, name + "-standard")
        standard["geometry"] = {"panel": panel, "content": content}
        standard["fill"] = painted(frame, STANDARD, rows, (panel["x"] + 5, panel["y"] + 7))

        app.activate("drawer-demo-dismissible")
        require(not absent(app.dom(), STANDARD), "Dismissible drawer should start open")
        app.activate(TOGGLE)
        frame = app.dom()
        require(absent(frame, STANDARD) and bounds(frame, STANDARD + "-content")["width"] == layout["width"]
                and focused(frame) == TOGGLE, "Closing the standard drawer did not restore shared space")
        app.activate(SHARED)
        if name == "light-desktop":
            collapsed, rows = capture(app, plan.out, name + "-dismissible-closed")
        else:
            collapsed, rows = None, None
        app.activate(TOGGLE)

        app.activate(OPEN)
        frame = app.dom()
        require(focused(frame) == CLOSE, "Opening the modal drawer must focus Close")
        require(locate(frame, PANEL)[0]["role"] == "Dialog", "Missing modal dialog role")
        require(locate(frame, PANEL + "-home")[0]["current_page"], "The initial page is not current")
        require(app.wire("Action.activate", {"id": "drawer-demo-background"}, "outside_modal") == "outside_modal",
                "Modal background activation was not blocked")
        require(app.wire("Action.activate", {"id": PANEL + "-archive"}, "disabled") == "disabled",
                "Disabled modal destination was reachable")
        ax = verify_modal_ax(plan, app, "首頁") if native_ax else {}
        if ax.get("verified"):
            pressed = report_ax(plan, app, "--press", "收件匣")
            require(pressed.get("press_status") == 0 and absent(app.dom(), PANEL)
                    and focused(app.dom()) == OPEN, f"AppKit press did not restore the opener: {pressed}")
            ax["selected_with_axpress_and_restored_opener"] = True
        else:
            app.key("tab")
            require(focused(app.dom()) == PANEL + "-home", "Tab did not reach the home destination")
            app.key("down")
            require(focused(app.dom()) == PANEL + "-inbox", "Focus should skip the disabled row")
            app.key("enter")
            require(absent(app.dom(), PANEL) and focused(app.dom()) == OPEN, "Enter did not restore the opener")

        app.activate(OPEN)
        frame = app.dom()
        require(locate(frame, PANEL + "-inbox")[0]["current_page"]
                and not locate(frame, PANEL + "-home")[0]["current_page"], "Modal selection did not persist")
        modal_geometry = bounds(frame, PANEL)
        require(modal_geometry["height"] == height
                and modal_geometry["width"] == min(width, 360) and modal_geometry["x"] == 0,
                f"Unexpected full-height modal panel: {modal_geometry}")
        header = bounds(frame, PANEL + "-header")
        if compact:
            close = bounds(frame, CLOSE)
            require(close["x"] > width - 65 and close["width"] == 48,
                    "The vector Close control is not at the logical start")
            app.key("tab")
            require(focused(app.dom()) == PANEL + "-inbox", "Tab did not reach the current page")
            app.key("down")
            frame = app.dom()
            require(focused(frame) == PANEL + "-reports", "Arrow Down did not focus the wrapped destination")
            report = bounds(frame, PANEL + "-reports")
            require(report["height"] > 68 and report["y"] >= header["y"] + header["height"] - 1
                    and bounds(frame, PANEL + "-header") == header,
                    f"Focus did not scroll the wrapped label past the fixed header: {report}")
        modal, rows = capture(app, plan.out, name + "-modal", parity=name.endswith("compact"))
        modal["geometry"] = modal_geometry
        modal["fill"] = painted(frame, PANEL, rows, (8, 12 if compact else height - 9))
        app.key("escape")
        require(absent(app.dom(), PANEL) and focused(app.dom()) == OPEN, "Escape did not restore the opener")
        return {"standard": standard, "dismissible_closed": collapsed, "modal": modal, "ax": ax}


def main():
    parser = argparse.ArgumentParser()
    for arg in ("gallery", "cli", "profile-launcher", "out"):
        parser.add_argument("--" + arg, type=Path, required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--ax-reader", type=Path)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True, exist_ok=False)
    results = {}
    for name, profile in PROFILES.items():
        results[name] = run_profile(plan, name, profile, native_ax=name == "light-desktop")
        print(f"PASS {name}", flush=True)
    report = {"source_manifest_sha256": plan.source, "profiles": results}
    (plan.out / "native-drawer-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
