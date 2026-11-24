"""Verify the staged Search route against the native headless Gallery."""

import argparse
import json
import math
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, find, focus, key, node, settle, visible
from window_check import onscreen_windows

BASE = "search-overlay-demo"
INPUT = f"{BASE}-input"
OPENER = f"{BASE}-opener"
BAR = f"{BASE}-bar"
DIALOG = f"{BASE}-dialog"
STATUS = f"{BASE}-status"
RESULTS = f"{BASE}-results"
BACK = f"{BASE}-back"
CLEAR = f"{BASE}-clear"


def enter(app, compact):
    activate(app, "section-1")
    activate(app, f"{BASE}-show")
    if compact:
        for mode in ("theme", "rtl", "large-text"):
            activate(app, f"{BASE}-{mode}")


def check_open(frame, width, height, divided, compact):
    focus(frame, INPUT)
    bar = visible(frame, BAR)
    panel = visible(frame, DIALOG)
    results = visible(frame, RESULTS)
    back = visible(frame, BACK)
    editor = visible(frame, INPUT)
    require((back["width"], back["height"]) == (48, 48), "Back target is too small")
    require(back["x"] > editor["x"] if compact else back["x"] < editor["x"],
            "The Back target was not placed at logical start")
    require(bar["height"] == (72 if divided and compact else 56), "Incorrect pinned bar height")
    require(results["y"] >= bar["y"] + bar["height"], "Results crossed the pinned input row")
    require((find(frame, f"{BASE}-divider")[1] is not None) == divided, "Wrong style divider")
    require(node(frame, STATUS)["role"] == "Status", "Result count lost its live status role")
    if compact:
        require(panel == {"x":0.0,"y":0.0,"width":width,"height":height}, "Full-screen did not cover the viewport")
    else:
        anchor = visible(frame, f"{BASE}-anchor")
        require(panel["y"] == anchor["y"] and 360 <= panel["width"] <= 720
                and 240 <= panel["height"] <= height * 2 / 3, "Docked panel did not fit its anchor")
    return {"panel": panel, "bar": bar, "editor": editor, "results": results, "back": back}


def query_ink(path, editor, foreground):
    _, _, rows = read_png(path.read_bytes())
    x0 = math.ceil(editor["x"] + 3)
    x1 = math.floor(editor["x"] + editor["width"] - 3)
    y0 = math.ceil(editor["y"] + 3)
    y1 = math.floor(editor["y"] + editor["height"] - 3)
    points = []
    for y in range(y0, y1):
        pixels = []
        for x in range(x0, x1):
            pixel = rows[y][4*x:4*x+4]
            if pixel[3] == foreground[3] and max(abs(a - b) for a, b in zip(pixel[:3], foreground[:3])) <= 24:
                pixels.append((x, y))
        if len(pixels) >= 2:
            # The focus outline is outside the sampled inset. Rows with only
            # the one-pixel caret cannot supply glyph evidence.
            points.extend(pixels)
    require(len(points) >= 16, f"No query ink inside {path.name}")
    xs, ys = zip(*points)
    require(max(xs) - min(xs) >= 8, "Query sample found a caret instead of glyphs")
    return {"pixels": len(points), "top": min(ys), "bottom": max(ys),
            "mid_y": (min(ys) + max(ys) + 1) / 2}


def replay(plan, compact, baseline):
    width, height = (320, 400) if compact else (960, 800)
    profile = "compact-dark-rtl-200" if compact else "docked-light-ltr"
    windows = []

    def count_windows(app):
        count = onscreen_windows(app.proc.pid)
        require(count == 0, "The headless route created an onscreen window")
        windows.append(count)

    records = []
    with launch(plan, profile, width, height) as app:
        count_windows(app)
        enter(app, compact)
        for divided in (False, True):
            if divided:
                activate(app, f"{BASE}-style")
            focus_by_tab(app, OPENER)
            frame = key(app, "enter")
            if find(frame, CLEAR)[1] is not None:
                frame = activate(app, CLEAR)
            geometry = check_open(frame, width, height, divided, compact)
            require(node(frame, STATUS)["name"] == "近期文件，共 12 筆", "Empty query lost the history state")
            app.wire("Action.fill", {"id": INPUT, "value": "設計"})
            frame = settle(app)
            focus(frame, INPUT)
            require(frame["text_inputs"][INPUT]["value"] == "設計", "The editor did not accept the query")
            require(node(frame, STATUS)["name"] == "找到 4 筆文件", "Wrong filter count")
            require(node(frame, "search-doc-archive")["disabled"], "The archived result should be disabled")
            require((visible(frame, CLEAR)["width"], visible(frame, CLEAR)["height"]) == (48, 48),
                    "Clear target is too small")
            frame = key(app, "enter")
            focus(frame, INPUT)
            name = f"{profile}-{'divided' if divided else 'contained'}"
            shot = capture(app, plan, name, frame=frame, sample_targets=[INPUT, "search-doc-design-headline"])
            count_windows(app)

            editor = visible(frame, INPUT)
            foreground = node(frame, INPUT)["style"]["foreground"]
            moved = query_ink(plan.out / shot["name"], editor, foreground)
            require(editor["height"] == 48, "Centering resized the input hit target")
            require(abs(moved["mid_y"] - (editor["y"] + editor["height"] / 2)) <= 4,
                    "Query glyphs were not centered in the measured input")
            if baseline:
                before = query_ink(baseline / shot["name"], editor, foreground)
                movement = moved["mid_y"] - before["mid_y"]
                require(abs(movement - (0 if compact else 12)) <= 1,
                        "Unexpected query movement from the first native proof")

            focus(key(app, "down"), "search-doc-design")
            focus(key(app, "tab", reverse=True), CLEAR)
            focus(key(app, "tab", reverse=True), INPUT)

            focus(key(app, "tab"), CLEAR)
            focus(key(app, "tab"), "search-doc-design")
            focus(key(app, "down"), "search-doc-review")
            frame = key(app, "enter")
            focus(frame, OPENER)
            require(find(frame, DIALOG)[1] is None
                    and node(frame, f"{BASE}-selected")["text"] == "已開啟 設計審查",
                    "The caller did not open the selected stable result")
            records.append({"style": "divided" if divided else "contained", "geometry": geometry, "shot": shot,
                            "query_ink": moved, "previous_query_ink": before if baseline else None,
                            "first_result_after_editor_arrow": "search-doc-design", "filter": "設計",
                            "selected": "search-doc-review", "restored_focus": OPENER})
        count_windows(app)

    return {"profile": profile, "viewport": [width, height], "captures": records, "visible_windows": windows}


def main():
    parser = argparse.ArgumentParser()
    for arg in ("gallery", "cli", "out"):
        parser.add_argument(f"--{arg}", type=Path, required=True)
    parser.add_argument("--baseline", type=Path)
    args = parser.parse_args()
    gallery = args.gallery.resolve(strict=True)
    cli = args.cli.resolve(strict=True)
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use a private /tmp output directory")
    out.mkdir(parents=True)
    binaries = {"gallery": digest(gallery), "cli": digest(cli)}
    plan = SimpleNamespace(gallery=gallery, cli=cli, out=out)
    profiles = [replay(plan, compact, args.baseline) for compact in (False, True)]
    require(binaries == {"gallery": digest(gallery), "cli": digest(cli)}, "Binaries changed during replay")
    report = {"schema": "surface.test.search-overlay-native.v1", "binaries": binaries,
              "transport": ["Page.screenshot", "CLI", "MCP"], "profiles": profiles}
    (out / "native-search-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print("PASS four centered captures and editor ArrowDown on three transports; eight zero-window observations")


if __name__ == "__main__":
    main()
