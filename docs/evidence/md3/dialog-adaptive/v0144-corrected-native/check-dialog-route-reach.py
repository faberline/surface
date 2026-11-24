"""Verify both Dialog entry points with the pinned Gallery in headless mode."""

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

ROUTES = ("dialog-demo-show", "dialog-demo-show-transform")
CONTROLS = ("dialog-demo-basic-open", "dialog-demo-fab-open")
PREVIOUS = ("date-picker-demo-show", "docked-date-picker-demo-show",
            "date-range-picker-demo-show", "show-time-scroll-demo",
            "time-input-baseline-demo-show", "time-dial-baseline-demo-show",
            "list-media-demo-show")
BACK = "dialog-demo-back"


def wide(plan):
    with launch(plan, "normalized", 960, 1800) as app:
        ready = onscreen_windows(app.proc.pid)
        frame = activate(app, "section-2")
        routes = {route: visible(frame, route, 48.) for route in ROUTES}
        previous = {route: visible(frame, route, 48.) for route in PREVIOUS}
        first, second = routes.values()
        require(first["y"] == second["y"], f"Launchers do not share a row: {routes}")
        require(first["x"] + first["width"] <= second["x"], f"Launchers overlap: {routes}")
        require(previous[PREVIOUS[-1]]["y"] + previous[PREVIOUS[-1]]["height"] < first["y"],
                "A new launcher precedes the last existing fixture")
        windows = [ready, onscreen_windows(app.proc.pid)]
        require(windows == [0, 0], f"Normalized Gallery has visible windows: {windows}")
    return {"viewport": [960, 1800], "launchers": routes, "previous": previous, "windows": windows}


def compact(plan):
    with launch(plan, "compact", 320, 400) as app:
        windows = [onscreen_windows(app.proc.pid)]
        frame = activate(app, "section-2")
        require(node(frame, "surface-page")["style"]["clip"], "The page must use a real root clip")
        initial = {route: layout(frame, route) for route in ROUTES}
        for route, area in initial.items():
            require(area["bounds"]["y"] >= area["clip"]["y"] + area["clip"]["height"],
                    f"{route} must start below the compact viewport: {area}")
        first_tabs = focus_by_tab(app, ROUTES[0])
        controls = {}
        for index, (route, control) in enumerate(zip(ROUTES, CONTROLS)):
            frame = key(app, "tab") if index else settle(app)
            focus(frame, route)
            revealed = visible(frame, route, 48.)
            require(revealed["y"] < initial[route]["bounds"]["y"], f"{route} did not scroll")
            shot = capture(app, plan, f"compact-revealed-{index + 1}", frame=frame)
            windows.append(onscreen_windows(app.proc.pid))
            opened = key(app, "enter")
            focus(opened, BACK)
            visible(opened, control, 48.)
            require(find(opened, CONTROLS[1 - index])[1] is None, f"{route} opened the wrong preview")
            windows.append(onscreen_windows(app.proc.pid))
            restored = activate(app, BACK)
            focus(restored, route)
            after_back = visible(restored, route, 48.)
            windows.append(onscreen_windows(app.proc.pid))
            controls[route] = {"before": initial[route], "revealed": revealed,
                               "tab_path": [route] if index else first_tabs, "opened": control,
                               "back_focus": route, "after_back": after_back, "shot": shot}
        require(not any(windows), f"Compact Gallery has visible windows: {windows}")
    return {"viewport": [320, 400], "launchers": controls, "windows": windows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ("gallery", "cli", "out"):
        parser.add_argument(f"--{field}", required=True, type=Path)
    args = parser.parse_args()
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use a private /tmp directory")
    out.mkdir()
    plan = SimpleNamespace(gallery=args.gallery.resolve(strict=True), cli=args.cli.resolve(strict=True), out=out)
    binaries = {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}
    profiles = [wide(plan), compact(plan)]
    require(binaries == {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}, "Binaries changed")
    (out / "route-reach.json").write_text(json.dumps({"binaries": binaries, "profiles": profiles}, indent=2) + "\n")
    print("Both launchers are visible at 960×1800. At 320×400, Tab reveals them, Enter opens each, and Back restores focus.")


if __name__ == "__main__":
    main()
