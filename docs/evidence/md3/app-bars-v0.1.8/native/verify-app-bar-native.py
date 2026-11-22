#!/usr/bin/env python3
"""Replay the MD3 App Bar with native Metal and no visible window."""
import argparse
import json
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, launch, same_box

ID = "app-bar-demo-"
BAR = ID + "bar"
TITLE = BAR + "-title"
ENTRY = ID + "search-entry"
QUERY = ID + "search-input"
DIALOG = ID + "search-view"
SCREENSHOT = {"scale": 1, "max_side": 1200}


def locate(frame, stable_id):
    for index, node in enumerate(frame["dom"]["nodes"]):
        if node["stable_id"] == stable_id:
            return node, frame["layout"]["nodes"][index]["bounds"]
    return None, None


def bounds(frame, stable_id):
    return locate(frame, stable_id)[1]


def shot(app, plan, name, parity=False, paint=None):
    path = plan.out / f"{name}.png"
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    data = path.read_bytes()
    if parity:
        raw = app.wire("Page.screenshot", SCREENSHOT)
        mcp, mcp_meta, _ = app.mcp(SCREENSHOT)
        require(data == png_payload(raw) == mcp, "Screenshot PNG differs across clients")
        require(metadata(cli) == metadata(raw) == mcp_meta, "Screenshot metadata differs")
    w, h, rows = read_png(data)
    require((w, h) == (cli["width"], cli["height"]), "PNG metadata mismatch")
    if paint is not None:
        sample = rows[2][4:7]
        require(max(abs(a - b) for a, b in zip(sample, paint)) < 7,
                f"App bar background pixel disagrees with the DOM: {list(sample)} vs {paint}")
    return {"file": path.name, "sha256": digest(path), **metadata(cli), "transport_parity": parity}


def geometry(frame, rtl=False):
    bar = bounds(frame, BAR)
    lead = bounds(frame, ID + "leading")
    action = bounds(frame, ID + "action")
    disabled, inactive = locate(frame, ID + "disabled")
    require(disabled["disabled"], "Disabled command is enabled")
    require(all(same_box(bar, action_rect) and action_rect["width"] >= 44
                and action_rect["height"] >= 44 for action_rect in (lead, action, inactive)),
            f"App bar clipped an action target: {bar}, {lead}, {action}, {inactive}")
    require((action["x"] < lead["x"]) == rtl, "Start/end controls are not mirrored")
    return bar


def bar_shot(app, plan, frame, name, parity=False):
    return shot(app, plan, name, parity, locate(frame, BAR)[0]["style"]["background"][:3])


def open_gallery(app, dark=False, compact=False):
    app.activate("section-1")
    app.activate(ID + "show")
    require(focused(app.dom()) == ID + "back", "Opening the fixture did not focus Back")
    if dark:
        app.activate("theme")
    if compact:
        app.activate(ID + "rtl")
        app.activate(ID + "scale")


def search(app, plan, profile):
    app.activate(ID + "search")
    closed = app.dom()
    require(locate(closed, ENTRY)[0]["role"] == "Button", "Search must have a button trigger")
    entry_shot = bar_shot(app, plan, closed, profile + "-search-entry")
    app.activate(ENTRY)
    opened = app.dom()
    require(focused(opened) == QUERY, "Search did not focus the editable field")
    role, rect = locate(opened, DIALOG)
    require(role["name"] == "搜尋工作區" and role["role"] == "Dialog", "Search dialog has no accessible name")
    require(rect == opened["layout"]["viewport"], f"Search did not fill the viewport: {rect}")
    app.wire("Action.activate", {"id": ID + "action"}, "outside_modal")
    app.cli(["action", "fill", QUERY, "設計"])
    filtered = app.dom()
    require(locate(filtered, ID + "search-result")[0]["name"] == "設計紀錄", "Chinese query lost its match")
    require(locate(filtered, ID + "search-result-tests")[0] is None, "Search retained a non-match")
    search_shot = shot(app, plan, profile + "-search-filtered", parity=True)
    app.cli(["action", "fill", QUERY, "無匹配"])
    require(locate(app.dom(), ID + "search-empty")[0] is not None, "Missing search empty state")
    app.activate(ID + "search-clear")
    restored = app.dom()
    require(focused(restored) == QUERY, "Clear did not restore field focus")
    require(locate(restored, ID + "search-result-tests")[0] is not None, "Clear did not restore results")
    app.key("escape")
    require(focused(app.dom()) == ENTRY and locate(app.dom(), DIALOG)[0] is None,
            "Escape did not restore the search trigger")
    app.activate(ENTRY)
    app.activate(ID + "search-result")
    require(focused(app.dom()) == ENTRY and locate(app.dom(), DIALOG)[0] is None,
            "Result selection did not return to the search trigger")
    require("設計紀錄" in locate(app.dom(), ID + "status")[0]["text"], "Chosen result is absent")
    return {"entry": entry_shot, "filtered": search_shot, "selected": "設計紀錄"}


def context(app, plan):
    app.activate(ID + "small")
    app.activate(ID + "context")
    selected = app.dom()
    require(locate(selected, TITLE)[0]["name"] == "已選取 1 項", "Contextual title is absent")
    require(locate(selected, ID + "leading")[0] is None, "Contextual controls did not replace page controls")
    selected_shot = bar_shot(app, plan, selected, "light-contextual")
    app.activate(ID + "context-close")
    require(focused(app.dom()) == ID + "context" and locate(app.dom(), TITLE)[0]["name"] == "工作區總覽",
            "Closing selection did not restore the original bar and focus")
    return selected_shot


def desktop(app, plan):
    open_gallery(app)
    variants = []
    for label, height, toggle_subtitle in [
        ("small", 64, False), ("center", 64, False),
        ("medium", 136, False), ("medium", 112, True),
        ("large", 120, False), ("large", 152, True),
    ]:
        if focused(app.dom()) != ID + label:
            app.activate(ID + label)
        if toggle_subtitle:
            app.activate(ID + "subtitle")
        frame = app.dom()
        bar = geometry(frame)
        require(bar["height"] == height, f"Wrong {label} bar height {bar}")
        title = bounds(frame, TITLE)
        require(same_box(bar, title), f"{label} title is outside the bar")
        if label == "center":
            require(abs(title["x"] + title["width"] / 2 - bar["width"] / 2) < 1,
                    "Center-aligned heading is off-center")
        variants.append({"kind": label, "bounds": bar,
                         "capture": bar_shot(app, plan, frame, f"light-{label}-{height}")})
    app.wire("Action.activate", {"id": ID + "disabled"}, "disabled")
    expanded = app.dom()
    app.activate(ID + "preview")
    folded = app.dom()
    require(bounds(folded, BAR)["height"] == 64, "Flexible large bar did not collapse to 64 dp")
    require(locate(expanded, BAR)[0]["style"]["background"] != locate(folded, BAR)[0]["style"]["background"],
            "Collapsed bar did not take the raised surface color")
    fold = bar_shot(app, plan, folded, "light-large-collapsed")
    return {"variants": variants, "collapsed": fold,
            "search": search(app, plan, "light"), "contextual": context(app, plan)}


def compact(app, plan):
    open_gallery(app, dark=True, compact=True)
    app.activate(ID + "center")
    centered = app.dom()
    rect = geometry(centered, rtl=True)
    heading = bounds(centered, TITLE)
    require(same_box(rect, heading) and heading["height"] > 32,
            f"200% CJK heading is clipped: {heading} vs {rect}")
    narrow = bar_shot(app, plan, centered, "dark-rtl-200-center", parity=True)
    app.activate(ID + "large")
    large = app.dom()
    bar = geometry(large, rtl=True)
    require(same_box(bar, bounds(large, TITLE)) and same_box(bar, bounds(large, BAR + "-subtitle")),
            "Narrow 200% large title/subtitle is clipped")
    expanded = bar_shot(app, plan, large, "dark-rtl-200-large", parity=True)
    return {"width": rect["width"], "center": narrow, "large": expanded,
            "search": search(app, plan, "dark-rtl-200")}


def main():
    parser = argparse.ArgumentParser()
    for key in ("gallery", "cli", "out"):
        parser.add_argument("--" + key, type=Path, required=True)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True, exist_ok=False)
    results = {}
    for profile, width, height, verify in [
        ("desktop", 960, 800, desktop), ("dark-rtl-200", 320, 580, compact)
    ]:
        with launch(plan, profile, width, height) as app:
            results[profile] = verify(app, plan)
            print("PASS", profile, flush=True)
    results["source_manifest_sha256"] = plan.source
    (plan.out / "native-app-bar-report.json").write_text(json.dumps(results, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
