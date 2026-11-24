"""Exercise Search motion in an authenticated offscreen build with an exact clock."""

import argparse
import json
from pathlib import Path
import runpy
from types import SimpleNamespace

from gallery_transport import require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

SUPPORT = runpy.run_path(str(Path(__file__).with_name("replay-search.py")))
enter = SUPPORT["enter"]
check_open = SUPPORT["check_open"]
query_ink = SUPPORT["query_ink"]
BASE, INPUT, OPENER, BAR, DIALOG, RESULTS = (SUPPORT[name] for name in
                                           ("BASE", "INPUT", "OPENER", "BAR", "DIALOG", "RESULTS"))
SHELL = f"{BASE}-closing-shell"
CLOSING_BAR = f"{BASE}-closing-bar"
CLOSING_RESULTS = f"{BASE}-closing-results-surface"


def advance(app, time):
    require(app.wire("Test.setClock", {"now_ms": time}) == {"now_ms": time}, "Clock was not set")
    return settle(app)


def replay(plan, compact, stock):
    width, height = (320, 400) if compact else (960, 800)
    profile = "compact-dark-rtl-200-divided" if compact else "docked-light-ltr-contained"
    windows = []
    frames = {}
    shots = {}

    def count(app):
        n = onscreen_windows(app.proc.pid)
        require(n == 0, "Headless Gallery opened an onscreen window")
        windows.append(n)

    def record(app, label, frame, time):
        stage = label.startswith("opening")
        bar = visible(frame, BAR if stage else CLOSING_BAR)
        results = layout(frame, RESULTS if stage else CLOSING_RESULTS)["bounds"]
        shell = visible(frame, DIALOG if stage else SHELL)
        layer = node(frame, f"{BASE}-{'layer' if stage else 'closing-layer'}")
        frames[label] = {"now_ms": time, "bar": bar, "results": results,
                         "shell": shell, "scrim_alpha": layer["style"]["background"][3]}
        shots[label] = capture(app, plan, f"{profile}-{label}")
        if stage:
            focus(frame, INPUT)
            text = frame["text_inputs"][INPUT]
            require(text["value"] == "", "Opening animation changed the editor value")
            editor = visible(frame, INPUT)
            ink = node(frame, "search-doc-design-supporting")["style"]["foreground"]
            frames[label]["hint_ink"] = query_ink(plan.out / shots[label]["name"], editor, ink)
        count(app)

    with launch(plan, profile, width, height) as app:
        count(app)
        enter(app, compact)
        if compact:
            activate(app, f"{BASE}-style")
        focus_by_tab(app, OPENER)
        rest = visible(settle(app), OPENER)
        app.wire("Test.setClock", {"now_ms": -1}, "invalid_params")
        start = key(app, "enter")
        record(app, "opening-start", start, 0)
        require(frames["opening-start"]["bar"] == rest, "Start did not use measured resting bar")
        require(frames["opening-start"]["results"]["height"] == 0, "Initial results already covered the page")

        middle = advance(app, 60)
        record(app, "opening-60", middle, 60)
        opened = advance(app, 1000)
        record(app, "opening-settled", opened, 1000)
        check_open(opened, width, height, compact, compact)
        st, mid, end = (frames[name] for name in ("opening-start", "opening-60", "opening-settled"))
        require(st["results"]["height"] < mid["results"]["height"] < end["results"]["height"],
                "The opening results did not grow beside the pinned bar")
        require(mid["results"]["y"] >= mid["bar"]["y"] + mid["bar"]["height"] - 0.5,
                "Results covered the input row")
        if not compact:
            require([st["scrim_alpha"], end["scrim_alpha"]] == [0, 82]
                    and 0 < mid["scrim_alpha"] < 82, "Docked scrim did not fade with results")
        require(shots["opening-settled"]["sha256"] == digest(stock / f"{profile}-empty.png"),
                "The final animation frame differs from the independently built snapped frame")

        closed = key(app, "escape")
        focus(closed, OPENER)
        require(find(closed, INPUT)[1] is None and find(closed, DIALOG)[1] is None,
                "Closing retained a live Search editor or modal")
        require(visible(closed, CLOSING_BAR) == end["bar"], "Closing did not start at the last opening pose")
        exit_frame = advance(app, 1060)
        focus(exit_frame, OPENER)
        record(app, "closing-60", exit_frame, 1060)
        exit_geom = frames["closing-60"]
        require(0 < exit_geom["results"]["height"] < end["results"]["height"],
                "Closing results did not shrink")
        if not compact:
            require(0 < exit_geom["scrim_alpha"] < 82, "Docked scrim did not fade during exit")
        final = advance(app, 2000)
        focus(final, OPENER)
        require(find(final, SHELL)[1] is None, "Closing shell remained after settling")
        require(len({shot["sha256"] for shot in shots.values()}) == len(shots), "Distinct frame poses made identical PNGs")
        count(app)

    return {"name": profile, "viewport": [width, height], "frames": frames,
            "pngs": shots, "visible_windows": windows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("gallery", "cli", "out", "stock"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    gallery, cli = args.gallery.resolve(strict=True), args.cli.resolve(strict=True)
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use private /tmp evidence")
    out.mkdir(parents=True)
    binaries = {"gallery": digest(gallery), "cli": digest(cli)}
    plan = SimpleNamespace(gallery=gallery, cli=cli, out=out)
    profiles = [replay(plan, compact, args.stock) for compact in (False, True)]
    require(binaries == {"gallery": digest(gallery), "cli": digest(cli)}, "Native binaries changed")
    report = {"schema": "surface.test.search-motion-native.v1", "diagnostic_clock": "Test.setClock",
              "binaries": binaries, "transport": ["Page.screenshot", "CLI", "MCP"], "profiles": profiles}
    (out / "native-search-motion-report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    print("PASS eight real motion PNGs, three matching transports, twelve zero-window samples")


if __name__ == "__main__":
    main()
