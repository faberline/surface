"""Headless Metal proof for the painted Search hint and unchanged real value."""

import argparse
import json
from pathlib import Path
import runpy
from types import SimpleNamespace

from gallery_transport import require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, find, focus, key, node, settle, visible
from window_check import onscreen_windows

SUPPORT = runpy.run_path(str(Path(__file__).with_name("replay-search.py")))
query_ink = SUPPORT["query_ink"]
enter = SUPPORT["enter"]
check_open = SUPPORT["check_open"]
BASE, INPUT, OPENER, CLEAR, STATUS = (SUPPORT[name] for name in ("BASE", "INPUT", "OPENER", "CLEAR", "STATUS"))


def check_empty(frame):
    focus(frame, INPUT)
    text = frame["text_inputs"][INPUT]
    editor = node(frame, INPUT)
    require(text["value"] == text["display_value"] == editor["text"] == "", "Hint became an editor value")
    require(text["selection_anchor"] == text["selection_cursor"] == 0 and not text["composing"], "Wrong empty editor state")
    require(editor["name"] == "搜尋文件" and editor["role"] == "TextInput", "The accessible field label changed")
    require(find(frame, CLEAR)[1] is None, "An empty query has a Clear action")
    require(node(frame, STATUS)["name"] == "近期文件，共 12 筆", "Hint affected history")


def replay(plan, compact, comparison):
    width, height = (320, 400) if compact else (960, 800)
    profile = "compact-dark-rtl-200-divided" if compact else "docked-light-ltr-contained"
    windows = []

    def count(app):
        value = onscreen_windows(app.proc.pid)
        require(value == 0, "Gallery unexpectedly opened a visible window")
        windows.append(value)

    with launch(plan, profile, width, height) as app:
        count(app)
        enter(app, compact)
        if compact:
            activate(app, f"{BASE}-style")
        focus_by_tab(app, OPENER)
        opening = key(app, "enter")
        check_empty(opening)
        geometry = check_open(opening, width, height, compact, compact)
        require(geometry["editor"]["height"] == 48, "Placeholder resized the input hit target")
        hint_color = node(opening, "search-doc-design-supporting")["style"]["foreground"]
        require(hint_color != node(opening, INPUT)["style"]["foreground"], "Hint and input ink should be distinct")
        empty = capture(app, plan, f"{profile}-empty")
        glyphs = query_ink(plan.out / empty["name"], geometry["editor"], hint_color)
        center = geometry["editor"]["y"] + geometry["editor"]["height"] / 2
        require(abs(glyphs["mid_y"] - center) <= 4, "Hint glyphs were not centered")
        count(app)

        app.wire("Action.fill", {"id": INPUT, "value": "設計"})
        populated = settle(app)
        focus(populated, INPUT)
        require(populated["text_inputs"][INPUT]["value"] == "設計", "Typed value did not replace the hint")
        typed = capture(app, plan, f"{profile}-typed")
        require((comparison / f"{profile}.png").read_bytes() == (plan.out / typed["name"]).read_bytes(),
                "The nonempty render changed from the sealed Search proof")
        count(app)

        restored = activate(app, CLEAR)
        check_empty(restored)
        require(visible(restored, INPUT) == geometry["editor"], "Clear moved the empty editor")
        cleared = capture(app, plan, f"{profile}-cleared")
        require(cleared["sha256"] == empty["sha256"], "Clear did not restore the initial hint frame")
        focus(key(app, "escape"), OPENER)
        count(app)
    return {"profile": profile, "viewport": [width, height], "hint_color": hint_color,
            "hint_ink": glyphs, "empty_input": geometry["editor"], "visible_windows": windows,
            "captures": [empty, typed, cleared], "typed_matches_sealed_search": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ("gallery", "cli", "out", "comparison"):
        parser.add_argument(f"--{arg}", type=Path, required=True)
    args = parser.parse_args()
    gallery, cli = args.gallery.resolve(strict=True), args.cli.resolve(strict=True)
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use private /tmp evidence")
    out.mkdir(parents=True)
    binaries = {"gallery": digest(gallery), "cli": digest(cli)}
    plan = SimpleNamespace(gallery=gallery, cli=cli, out=out)
    profiles = [replay(plan, compact, args.comparison) for compact in (False, True)]
    require(binaries == {"gallery": digest(gallery), "cli": digest(cli)}, "Native binaries changed")
    report = {"schema": "surface.test.search-placeholder-native.v1", "binaries": binaries,
              "transport": ["Page.screenshot", "CLI", "MCP"], "profiles": profiles}
    (out / "native-blank-search-report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    print("PASS six screenshots, distinct centered hint ink, unchanged typed frames, and eight zero-window samples")


if __name__ == "__main__":
    main()
