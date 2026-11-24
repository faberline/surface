"""Capture the two stock animation end states from an existing Gallery pin."""

import argparse
import json
from pathlib import Path
import runpy
from types import SimpleNamespace

from combined_editor import combined_editor
from gallery_transport import require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, key, node
from window_check import onscreen_windows

support = runpy.run_path(str(Path(__file__).with_name("replay-search.py")))
BASE, INPUT, OPENER, enter, check_open, query_ink = (support[k] for k in (
    "BASE", "INPUT", "OPENER", "enter", "check_open", "query_ink"))


def replay(plan, compact):
    name = "compact-dark-rtl-200-divided" if compact else "docked-light-ltr-contained"
    width, height = (320, 400) if compact else (960, 800)
    with launch(plan, name, width, height) as app:
        app.wire("Test.setClock", {"now_ms": 1000}, "unknown_method")
        enter(app, compact)
        if compact:
            activate(app, f"{BASE}-style")
        focus_by_tab(app, OPENER)
        frame = key(app, "enter")
        geometry = check_open(frame, width, height, compact, compact)
        shot = capture(app, plan, f"{name}-empty")
        path = plan.out / shot["name"]
        hint_color = node(frame, "search-doc-design-supporting")["style"]["foreground"]
        hint = query_ink(path, geometry["editor"], hint_color)
        painted = combined_editor(path, geometry["editor"], node(frame, INPUT)["style"]["foreground"], hint, compact)
        count = onscreen_windows(app.proc.pid)
        require(count == 0, "Stock Gallery opened a visible window")
        return {"profile":name, "geometry":geometry, "editor_evidence":painted,
                "shot":shot, "visible_window_count":count}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("gallery", "cli", "out"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use private /tmp evidence")
    out.mkdir(parents=True)
    plan = SimpleNamespace(gallery=args.gallery.resolve(strict=True), cli=args.cli.resolve(strict=True), out=out)
    binaries = {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}
    profiles = [replay(plan, compact) for compact in (False, True)]
    require(binaries == {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}, "Stock binary changed")
    report = {"schema":"surface.test.search-combined-stock-native.v1","binaries":binaries,
              "transport":["Page.screenshot","CLI","MCP"],"test_clock":"unknown_method","profiles":profiles}
    (out / "stock-search-settled-report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    print("PASS two stock Search PNGs, independent transport matching, zero visible windows")


if __name__ == "__main__":
    main()
