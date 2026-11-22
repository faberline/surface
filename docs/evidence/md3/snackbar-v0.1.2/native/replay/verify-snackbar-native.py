#!/usr/bin/env python3
"""Replay native Snackbar behavior against Surface's production debug transports."""

import argparse
from contextlib import contextmanager
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat
import subprocess
import tempfile
import time

from gallery_transport import Gallery, metadata, png_payload, read_png, require

PROFILES = {
    "light-wide": (960, 800, False, False),
    "dark-wide": (960, 800, True, False),
    "dark-rtl-200-compact": (320, 480, True, True),
}
TEXT = "變更已儲存"
UNDO = "專案已移到封存"
LONG = "已將跨部門協作專案移到另一個工作區。你可以復原這項變更。"
ACTIVE_ID = re.compile(r"gallery-snackbar-(\d+)$")
SCALE = {"scale": 1, "max_side": 1200}


def digest(path):
    sha256 = hashlib.sha256()
    with path.open("rb") as handle:
        for data in iter(lambda: handle.read(1024 * 1024), b""):
            sha256.update(data)
    return sha256.hexdigest()


def read_trace(path):
    lines = path.read_bytes().split(b"\n")
    return [json.loads(line) for line in lines[:-1] if line]


def presses(trace, stable_id):
    return [
        row for row in trace
        if row["kind"] == "Message"
        and (row["data"].get("source") or {}).get("id") == stable_id
    ]


def wait_for_persisted_trace(app, path, ready):
    # The stock native host uses BufWriter. Read-only presentations advance
    # that buffer, without sending any additional application input.
    for _ in range(64):
        rows = read_trace(path)
        if ready(rows):
            return rows
        app.dom()
    raise RuntimeError("The native trace did not persist the expected boundary")


def find_input(rows, event):
    return [row for row in rows if row["kind"] == "Input" and row["data"]["event"] == event]


@contextmanager
def launch(plan, name, width, height, compact):
    command = [plan.profile_launcher, str(width), str(height)] if compact else [plan.gallery]
    with tempfile.TemporaryDirectory(dir="/tmp", prefix="surface-snackbar-session-") as tmp:
        private = Path(tmp)
        session = private / "session"
        trace = private / "trace.jsonl"
        env = os.environ.copy()
        env.pop("SURFACE_BRIDGE_SESSION", None)
        env.update({
            "SURFACE_TRACE": str(trace),
            "SURFACE_DEVTOOLS_SESSION": str(session),
            "SURFACE_REDUCED_MOTION": "1",
        })
        with (private / "gallery.log").open("wb") as log:
            proc = subprocess.Popen(command, env=env, stdout=log, stderr=subprocess.STDOUT)
            try:
                app = Gallery(plan, session, proc)
                deadline = time.monotonic() + 22
                while time.monotonic() < deadline:
                    require(proc.poll() is None, f"{name} Gallery exited before readiness")
                    if session.is_file():
                        # Never send a token from a session readable by other users.
                        require(stat.S_IMODE(session.stat().st_mode) == 0o600, "Session mode is not 0600")
                        try:
                            snapshot = app.dom()
                        except (OSError, ValueError, RuntimeError):
                            time.sleep(0.1)
                            continue
                        break
                    time.sleep(0.1)
                else:
                    raise RuntimeError(f"{name} Gallery did not become ready")
                viewport = snapshot["layout"]["viewport"]
                require((viewport["width"], viewport["height"]) == (width, height),
                        f"{name} logical viewport was {viewport}")
                caps = app.cli(["capabilities"])
                require("Page.screenshot" in caps["methods"], "Production screenshot capability is missing")
                yield app, trace
            finally:
                if proc.poll() is None:
                    proc.terminate()
                    try:
                        proc.wait(timeout=4)
                    except subprocess.TimeoutExpired:
                        proc.kill()
                        proc.wait(timeout=4)


def visible(snapshot, key):
    for i, entry in enumerate(snapshot["dom"]["nodes"]):
        if entry.get("stable_id") == key:
            return entry, snapshot["layout"]["nodes"][i]["bounds"]
    raise RuntimeError(f"{key} is missing from the Gallery snapshot")


def focused(snapshot):
    return snapshot["interaction"]["focused"]


def current(snapshot):
    active = [
        entry["stable_id"] for entry in snapshot["dom"]["nodes"]
        if ACTIVE_ID.fullmatch(entry.get("stable_id") or "")
    ]
    require(len(active) <= 1, "More than one notification is visible")
    return next(iter(active), None)


def focus_by_tab(app, target):
    steps = []
    for _ in range(36):
        app.key("tab")
        current_focus = focused(app.dom())
        steps.append(current_focus)
        if current_focus == target:
            return steps
    raise RuntimeError(f"Tab did not reach {target}: {steps!r}")


def notice(app, trigger, text):
    app.activate(trigger)
    snapshot = app.dom()
    require(focused(snapshot) == trigger, "Showing the Snackbar stole focus")
    key = current(snapshot)
    require(key, "No Snackbar appeared")
    status, _ = visible(snapshot, f"{key}-text")
    require(status["role"] == "Status" and status["name"] == text,
            "Status must expose the notification message alone")
    return key


def same_box(parent, child, tolerance=0.5):
    return all(child[k] >= parent[k] - tolerance
               and child[k] + child[d] <= parent[k] + parent[d] + tolerance
               for k, d in (("x", "width"), ("y", "height")))


def ink(rows, rect, target, stride=1, margin=1, limit=12):
    width, height = len(rows[0]) // 4, len(rows)
    x0 = max(0, math.ceil(rect["x"] + margin))
    x1 = min(width, math.floor(rect["x"] + rect["width"] - margin))
    y0 = max(0, math.ceil(rect["y"] + margin))
    y1 = min(height, math.floor(rect["y"] + rect["height"] - margin))
    matches = []
    total = 0
    for y in range(y0, y1, stride):
        row = rows[y]
        for x in range(x0, x1, stride):
            total += 1
            sample = row[x * 4:x * 4 + 4]
            if max(abs(a - b) for a, b in zip(sample, target)) <= limit:
                matches.append((x, y))
    bounds = None
    if matches:
        xs, ys = zip(*matches)
        bounds = [min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1]
    return {"hits": len(matches), "samples": total, "ink_bounds": bounds}


def probe_ax(plan, app, message=None, action=None, *, status_only=False):
    if plan.ax_reader is None:
        return {"verified": False, "reason": "AppKit reader was not supplied"}
    output = subprocess.run(
        [plan.ax_reader, str(app.proc.pid), "--read-status" if status_only else "--read"],
        capture_output=True, timeout=10, check=True,
    )
    result = json.loads(output.stdout)
    if not result["trusted"]:
        return {"verified": False, "reason": "AppKit process is not trusted"}
    require(result["windows"] > 0, "AppKit did not expose the Gallery window")
    statuses = result["statuses"]
    if message is None:
        require(not statuses, "AppKit retained a retired notification status")
    else:
        require([status["label"] for status in statuses] == [message],
                f"AppKit exposed the wrong status: {statuses!r}")
    if action:
        for name in [action, "關閉通知"]:
            buttons = [
                button for button in result["buttons"]
                if button["name"] == name and button["enabled"] and "AXPress" in button["actions"]
            ]
            require(len(buttons) == 1, f"AppKit did not expose one independent press for {name!r}")
        require("可復原通知" in {button["name"] for button in result["buttons"]},
                "AppKit exposed only the Snackbar scope")
    result["verified"] = True
    return result


@contextmanager
def observe_ax(plan, app):
    if plan.ax_reader is None:
        yield None
        return
    # The reader emits a ready receipt without handling any outside process.
    arguments = [plan.ax_reader, str(app.proc.pid), "--observe", "9"]
    proc = subprocess.Popen(arguments, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        import select
        available, _, _ = select.select([proc.stdout], [], [], 6)
        require(available, "AppKit observer did not emit its registration result")
        line = proc.stdout.readline()
        require(line, "AppKit observer stopped during registration")
        yield {"registration": json.loads(line), "process": proc}
    finally:
        if proc.poll() is None:
            proc.terminate()
        try:
            proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=2)


def collect_ax_observations(observer, expected_message):
    if observer is None:
        return {"verified": False, "reason": "AppKit observer was not supplied"}
    registration = observer["registration"]
    if not registration.get("registered"):
        return {"verified": False, "registration": registration}
    proc = observer["process"]
    output, _ = proc.communicate(timeout=12)
    events = [json.loads(line) for line in output.splitlines()]
    matching = [event for event in events if event.get("announcement") == expected_message]
    unexpected = [event for event in events if event.get("announcement") in {"復原", "關閉通知"}]
    return {
        "verified": len(matching) == 1 and not unexpected,
        "registration": registration,
        "events": events,
        "matching_message_count": len(matching),
        "unexpected_action_count": len(unexpected),
    }


def save_screenshot(plan, app, folder, tag, parity=False):
    path = folder / f"{tag}.png"
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    raw = path.read_bytes()
    captured = {"png": path.name, "sha256": digest(path), **metadata(cli)}
    if parity:
        wire = app.wire("Page.screenshot", SCALE)
        mcp, mcp_meta, _ = app.mcp(SCALE)
        require(raw == png_payload(wire) == mcp, "Native CLI, protocol and MCP PNG bytes differ")
        require(metadata(cli) == metadata(wire) == metadata(mcp_meta), "PNG attachment metadata differs")
        (folder / f"{tag}-wire.png").write_bytes(raw)
        (folder / f"{tag}-mcp.png").write_bytes(mcp)
        captured["byte_parity"] = ["cli", "wire", "mcp"]
    width, height, rows = read_png(raw)
    require((width, height, 1.0) == (cli["width"], cli["height"], cli["scale"]),
            "PNG dimensions differ from protocol metadata")
    return captured, rows


def screenshot(plan, app, folder, tag, key, action=None, compact=False, parity=False):
    before = app.dom()
    require(current(before) == key, f"{tag} is not showing the expected Snackbar")
    shot, rows = save_screenshot(plan, app, folder, tag, parity)
    panel, pb = visible(before, key)
    body, bb = visible(before, f"{key}-text")
    viewport = before["layout"]["viewport"]
    require(same_box(viewport, pb) and same_box(pb, bb), "Snackbar or text clips to its viewport")
    require(abs(viewport["height"] - (pb["y"] + pb["height"]) - 16) <= 1,
            f"Snackbar bottom inset is not 16 dp: {pb}")
    require(pb["width"] <= 600 and min(pb["x"], viewport["width"] - pb["x"] - pb["width"]) >= 15.5,
            "Snackbar panel width or side margins are wrong")

    surface = panel["style"]["background"]
    message_ink = ink(rows, bb, body["style"]["foreground"], limit=24)
    background = ink(rows, pb, surface, stride=4, margin=4, limit=4)
    require(background["hits"] > background["samples"] * 0.60, "PNG is missing the inverse surface fill")
    require(message_ink["hits"] >= 18, "PNG is missing the status message glyphs")
    shot.update({
        "view_id": key,
        "message": body["name"],
        "surface": surface,
        "panel": pb,
        "status": bb,
        "fill_ink": background,
        "message_ink": message_ink,
    })
    if action is None:
        require(not any(entry.get("stable_id") in {f"{key}-action", f"{key}-dismiss"}
                        for entry in before["dom"]["nodes"]), "Text-only notice has a control")
        return shot

    act_node, act = visible(before, f"{key}-action")
    close_node, close = visible(before, f"{key}-dismiss")
    require(act_node["role"] == close_node["role"] == "Button", "Both controls must be buttons")
    require(act_node["name"] == action and close_node["name"] == "關閉通知",
            "Snackbar controls have the wrong accessible labels")
    require(all(same_box(pb, b) and min(b["width"], b["height"]) >= 47.5 for b in (act, close)),
            "Snackbar controls must remain inside the panel with 48 dp hit targets")
    require((max(act["x"], close["x"]) >= min(act["x"] + act["width"], close["x"] + close["width"]))
            or (max(act["y"], close["y"]) >= min(act["y"] + act["height"], close["y"] + close["height"])),
            "Snackbar controls overlap")
    action_ink = ink(rows, act, act_node["style"]["foreground"], margin=5, limit=24)
    icon_box = {"x": close["x"] + close["width"] / 2 - 12,
                "y": close["y"] + close["height"] / 2 - 12, "width": 24, "height": 24}
    close_ink = ink(rows, icon_box, close_node["style"]["foreground"], margin=1, limit=24)
    require(action_ink["hits"] > 12 and close_ink["hits"] > 12, "Action or close icon did not paint")
    require(all(9 <= size <= 17 for size in close_ink["ink_bounds"][2:]),
            f"Wrong close vector glyph size: {close_ink}")
    if compact:
        require(close["x"] < act["x"] and act["y"] >= bb["y"] + bb["height"],
                "Large RTL text needs a mirrored separate control row")
        scale_button, _ = visible(before, "snackbar-scale")
        require(bb["height"] >= 80 and scale_button["name"] == "一般文字",
                "Compact message did not wrap at 200% CJK typography")
    else:
        require(bb["x"] < act["x"] < close["x"] and act["y"] == close["y"] == pb["y"],
                "Wide actions are not inline and ordered")
    shot.update({"action": act, "close": close, "action_ink": action_ink, "close_ink": close_ink})
    return shot


def verify_timeout(plan, app, trace, folder):
    opened_at = time.monotonic()
    open_id = notice(app, "snackbar-show-text", TEXT)
    try:
        probe = probe_ax(plan, app, TEXT, status_only=True)
    except RuntimeError as error:
        elapsed = round(time.monotonic() - opened_at, 3)
        raise RuntimeError(f"Text notice AppKit probe failed at {elapsed} s: {error}") from error
    captured = screenshot(plan, app, folder, "text-only", open_id)
    time.sleep(max(0, opened_at + 5.2 - time.monotonic()))

    # Read the persisted prefix without making an app request. The expiry
    # frame can still be buffered because it is the last frame before idle.
    before_query = read_trace(trace)
    final = app.dom()
    app.key("home")  # Semantic Edit on the focused button marks the end of the idle window.
    rows = wait_for_persisted_trace(app, trace, lambda records: find_input(records, "Edit"))
    shows = presses(rows, "snackbar-show-text")
    require(len(shows) == 1, "Trace did not record the text notice trigger once")
    start = shows[0]["t_ms"]
    markers = find_input(rows, "Edit")
    require(len(markers) == 1 and markers[0]["t_ms"] - start >= 4700,
            "Failed to mark the end of the idle window")
    marker = markers[0]
    idle_rebuilds = [
        row for row in rows if row["kind"] == "Rebuild" and 3500 <= row["t_ms"] - start <= 4500
    ]
    require(len(idle_rebuilds) == 1, "The native host did not autonomously rebuild after the short timeout")
    expiry = idle_rebuilds[0]
    require(marker["t_ms"] - expiry["t_ms"] >= 200,
            "The expiry rebuild did not precede the post-idle request")
    require(not any(start + 100 < row["t_ms"] <= expiry["t_ms"] for row in rows
                    if row["kind"] == "Input"), "An input drove the timed dismiss")
    require(current(final) is None and focused(final) == "snackbar-show-text",
            "Timed notice did not clear while keeping focus")
    return {
        "shot": captured, "appkit": probe,
        "show_ms": start, "rebuild_ms": expiry["t_ms"],
        "delay_ms": expiry["t_ms"] - start, "post_idle_marker_ms": marker["t_ms"],
        "rebuild_persisted_before_first_post_idle_query": expiry["seq"] in {
            row["seq"] for row in before_query
        },
        "focus": focused(final),
    }


def verify_actions(plan, app, trace, folder, compact):
    trigger = "snackbar-show-long" if compact else "snackbar-show-undo"
    message = LONG if compact else UNDO
    action = "復原工作區移動" if compact else "復原"
    probe_ax(plan, app)  # Ask AppKit for the tree before the first announcement.
    with observe_ax(plan, app) as observer:
        first = notice(app, trigger, message)
        screenshot_record = screenshot(plan, app, folder, "visible", first, action, compact, parity=compact)
        ax = probe_ax(plan, app, message, action)
        observed = collect_ax_observations(observer, message)
    # A focused background control must remain available while the notice is visible.
    background = app.wire("DOM.query", {"id": "section-2"})["matches"]
    require(len(background) == 1 and background[0]["in_scope"], "Snackbar became modal")
    app.activate("section-2")
    middle = app.dom()
    require(current(middle) == first and focused(middle) == "section-2",
            "Snackbar blocked background focus or cleared while moving focus")

    app.activate(trigger)
    require(current(app.dom()) == first, "The second notification replaced the active item")
    targets = [f"{first}-action"]
    tabs_to_action = focus_by_tab(app, targets[0])
    app.key("enter")
    next_frame = app.dom()
    second = current(next_frame)
    require(second is not None, "Undo also removed the queued notification")
    require(int(ACTIVE_ID.fullmatch(second)[1]) == int(ACTIVE_ID.fullmatch(first)[1]) + 1,
            f"Undo did not advance exactly one queue entry: {first} to {second}")
    require(focused(next_frame) == trigger, "Undo did not restore opener focus")
    app.wire("Action.activate", {"id": targets[0]}, expected_error="missing_node")
    require(current(app.dom()) == second, "A retired Undo mutated the queue")

    targets.append(f"{second}-dismiss")
    tabs_to_close = focus_by_tab(app, targets[1])
    app.key("enter")
    closed = app.dom()
    require(current(closed) is None and focused(closed) == trigger, "Close did not clear and restore focus")
    app.wire("Action.activate", {"id": targets[1]}, expected_error="missing_node")
    closed_shot, _ = save_screenshot(plan, app, folder, "closed")
    app.key("escape")  # Mark after both actions. The closed Gallery has no modal.
    persisted = wait_for_persisted_trace(app, trace, lambda rows: find_input(rows, "Cancel"))
    message_receipts = []
    for stable_id in targets:
        trace_records = presses(persisted, stable_id)
        require(len(trace_records) == 1, f"Expected one message from {stable_id}; got {trace_records}")
        message_receipts.append(trace_records[0]["data"]["source"])
    return {
        "visible": screenshot_record, "closed": closed_shot,
        "first": first, "second": second, "background_focus": focused(middle),
        "action_tabs": tabs_to_action, "close_tabs": tabs_to_close,
        "restored_focus": focused(closed), "press_trace": message_receipts,
        "native_accessibility": ax, "announcements": observed,
    }


def run(plan):
    require(plan.gallery.is_file() and plan.cli.is_file(), "Gallery and CLI must be exact candidate binaries")
    if "dark-rtl-200-compact" in plan.profiles:
        require(plan.profile_launcher is not None and plan.profile_launcher.is_file(),
                "The compact profile needs the two-argument Gallery launcher")
    plan.out.mkdir(mode=0o700, parents=True, exist_ok=False)
    report = {
        "source": plan.source,
        "completed": False,
        "binary_sha256": {"gallery": digest(plan.gallery), "cli": digest(plan.cli)},
        "appkit_os": subprocess.check_output(["sw_vers", "-productVersion"], text=True).strip(),
        "input": "Surface Action.key/Action.activate via loopback, no synthetic OS events",
        "profiles": {},
    }
    if plan.profile_launcher:
        report["binary_sha256"]["profile_launcher"] = digest(plan.profile_launcher)
    for name, (width, height, dark, compact) in PROFILES.items():
        if name not in plan.profiles:
            continue
        folder = plan.out / name
        folder.mkdir()
        print(f"Running {name}", flush=True)
        with launch(plan, name, width, height, compact) as (app, trace):
            app.activate("section-2")
            if dark:
                app.activate("theme")
            if compact:
                app.activate("snackbar-direction")
                app.activate("snackbar-scale")
            evidence = verify_actions(plan, app, trace, folder, compact)
            if name == "light-wide":
                evidence["timeout"] = verify_timeout(plan, app, trace, folder)
            report["profiles"][name] = evidence
        (plan.out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    if {"light-wide", "dark-wide"} <= report["profiles"].keys():
        light = report["profiles"]["light-wide"]["visible"]["surface"]
        dark = report["profiles"]["dark-wide"]["visible"]["surface"]
        require(light != dark, "Light and dark inverse fills did not differ")
    report["completed"] = True
    (plan.out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(f"Wrote {plan.out / 'report.json'}", flush=True)


def args():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gallery", type=Path, required=True)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--profile-launcher", type=Path)
    parser.add_argument("--ax-reader", type=Path)
    parser.add_argument("--source", required=True, help="Exact candidate commit or source-manifest digest")
    parser.add_argument("--out", type=Path, required=True, help="New output directory")
    parser.add_argument("--profiles", choices=PROFILES, nargs="+", default=list(PROFILES))
    parsed = parser.parse_args()
    for key in ("gallery", "cli", "profile_launcher", "ax_reader", "out"):
        value = getattr(parsed, key)
        if value is not None:
            setattr(parsed, key, value.resolve())
    return parsed


if __name__ == "__main__":
    run(args())
