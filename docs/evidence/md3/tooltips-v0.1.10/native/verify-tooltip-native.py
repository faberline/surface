#!/usr/bin/env python3
"""Replay Material 3 tooltips with native Metal and no visible window."""
import argparse
import json
from pathlib import Path
import time

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box

ID = "tooltip-demo-"
PLAIN = ID + "plain"
RICH = ID + "rich"
ACTION = ID + "action"
BODY = ID + "body-action"
SCREENSHOT = {"scale": 1, "max_side": 1200}


def locate(frame, stable_id):
    for index, node in enumerate(frame["dom"]["nodes"]):
        if node["stable_id"] == stable_id:
            return node, frame["layout"]["nodes"][index]["bounds"]
    return None, None


def until(app, ready, reason):
    deadline = time.monotonic() + 5
    while True:
        frame = app.dom()
        if ready(frame):
            return frame
        require(time.monotonic() < deadline, reason)
        time.sleep(0.03)


def ready(app, owner):
    def visible(frame):
        _, bounds = locate(frame, owner + "-popup")
        return bounds is not None and bounds["height"] > 0 and same_box(frame["layout"]["viewport"], bounds)
    return until(app, visible, f"{owner} popup was not placed in the viewport")


def dismissed(app, owner, focus):
    frame = until(app, lambda f: locate(f, owner + "-popup")[0] is None and focused(f) == focus,
                  f"{owner} popup or focus persisted after dismissal")
    for _ in range(3):
        require(locate(app.dom(), owner + "-popup")[0] is None, "Tooltip reopened without rearming")
    return frame


def capture(app, plan, name, frame, owner, parity=True):
    path = plan.out / f"{name}.png"
    client = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    data = path.read_bytes()
    if parity:
        raw = app.wire("Page.screenshot", SCREENSHOT)
        image, mcp_metadata, _ = app.mcp(SCREENSHOT)
        require(data == png_payload(raw) == image, "The three transports returned different PNGs")
        require(metadata(client) == metadata(raw) == mcp_metadata, "PNG metadata differed")
    width, height, rows = read_png(data)
    require((width, height) == (client["width"], client["height"]), "PNG metadata mismatch")
    node, popup = locate(frame, owner + "-popup")
    sample = rows[int(popup["y"] + popup["height"] / 2)]
    x = int(popup["x"] + 5)
    actual = list(sample[x * 4 : x * 4 + 3])
    expected = node["style"]["background"][:3]
    require(max(abs(a - b) for a, b in zip(actual, expected)) < 8,
            f"Popup paint {actual} differed from {expected}")
    return {"file": path.name, "sha256": digest(path), "bounds": popup, "paint": actual,
            "transport_parity": parity, **metadata(client)}


def check_side(frame, owner, above):
    _, anchor = locate(frame, owner)
    _, popup = locate(frame, owner + "-popup")
    gap = (anchor["y"] - popup["y"] - popup["height"]) if above else (popup["y"] - anchor["y"] - anchor["height"])
    require(3. <= gap <= 5., f"Wrong tooltip placement gap {gap}: {anchor}, {popup}")


def check_plain(app, plan, name, above, caret=True):
    focus_by_tab(app, PLAIN)
    frame = ready(app, PLAIN)
    popup, rect = locate(frame, PLAIN + "-popup")
    require(popup["role"] == "Tooltip" and popup["name"] == "複製目前所選的文字。",
            f"Wrong accessible plain tooltip {popup['role']} {popup['name']}")
    require(focused(frame) == PLAIN, "A plain tooltip moved focus")
    require(40. <= rect["width"] <= 200. and rect["height"] >= 24., "Wrong plain tooltip size")
    require((locate(frame, PLAIN + "-popup-caret")[0] is not None) == caret, "Wrong caret state")
    check_side(frame, PLAIN, above)
    receipt = capture(app, plan, name, frame, PLAIN)
    app.key("escape")
    dismissed(app, PLAIN, PLAIN)
    return receipt


def check_rich(app, plan, name, above):
    app.activate(RICH)
    frame = ready(app, RICH)
    popup, rect = locate(frame, RICH + "-popup")
    require(popup["role"] == "Dialog" and popup["name"] == "分享設定", "Wrong rich tooltip semantics")
    require(rect["width"] <= 320., "Rich tooltip exceeded maximum width")
    require(focused(frame) == RICH, "Rich tooltip stole owner focus")
    check_side(frame, RICH, above)
    receipt = capture(app, plan, name, frame, RICH)
    app.key("tab")
    require(focused(app.dom()) == ACTION, "Tab could not reach the rich action")
    app.key("escape")
    dismissed(app, RICH, RICH)
    app.activate(RICH)
    ready(app, RICH)
    app.activate(ACTION)
    dismissed(app, RICH, RICH)
    app.activate(RICH)
    ready(app, RICH)
    app.activate(BODY)
    end = dismissed(app, RICH, BODY)
    require("選項 1 次" in locate(end, ID + "status")[0]["text"]
            and "查看 1 次" in locate(end, ID + "status")[0]["text"],
            "The rich or the surrounding action was not invoked")
    return receipt


def profile(app, plan, compact):
    app.activate("section-2")
    app.activate(ID + "show")
    require(focused(app.dom()) == ID + "back", "Fixture did not focus Back")
    if compact:
        for key in ["theme", ID + "rtl", ID + "scale", ID + "top", ID + "start"]:
            app.activate(key)
    if compact:
        frame = app.dom()
        _, plain = locate(frame, PLAIN)
        _, rich = locate(frame, RICH)
        require(max(plain["x"] + plain["width"], rich["x"] + rich["width"]) >= 311.,
                "RTL logical start did not reach the physical right edge")
    prefix = "dark-rtl-200" if compact else "light"
    results = {
        "plain": check_plain(app, plan, prefix + "-plain", above=not compact),
        "rich": check_rich(app, plan, prefix + "-rich", above=not compact),
    }
    if compact:
        app.activate(ID + "caret")
        app.activate(ID + "end")
        frame = app.dom()
        require(min(locate(frame, key)[1]["x"] for key in (PLAIN, RICH)) <= 9.,
                "RTL logical end did not reach the physical left edge")
        results["plain_edge_no_caret"] = check_plain(app, plan, prefix + "-edge-no-caret", False, False)
    return results


def main():
    parser = argparse.ArgumentParser()
    for key in ("gallery", "cli", "out"):
        parser.add_argument("--" + key, type=Path, required=True)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True, exist_ok=False)
    results = {"source_manifest_sha256": plan.source}
    for label, w, h, compact in [("desktop", 960, 800, False), ("dark-rtl-200", 320, 580, True)]:
        with launch(plan, label, w, h) as app:
            results[label] = profile(app, plan, compact)
            print("PASS", label, flush=True)
    (plan.out / "native-tooltip-report.json").write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
