#!/usr/bin/env python3
"""Measure the FAB in two fixed native headless sessions, without live resize."""

import argparse
import json
from pathlib import Path
from runpy import run_path
from types import SimpleNamespace

from gallery_transport import require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, focus, key, layout
from window_check import onscreen_windows

SHOW = "dialog-demo-show-transform"
FAB = "dialog-demo-fab-open"


def profile(plan, width, height, check_fab):
    with launch(plan, f"fab-{width}", width, height) as app:
        windows = [onscreen_windows(app.proc.pid)]
        activate(app, "section-2")
        if width == 320:
            focus_by_tab(app, SHOW)
            frame = key(app, "enter")
        else:
            frame = activate(app, SHOW)
        focus(frame, "dialog-demo-back")
        frame = activate(app, "dialog-demo-rtl")
        viewport = frame["layout"]["viewport"]
        bounds = check_fab(frame, viewport, True)
        require(bounds == {"x": 24., "y": height - 80., "width": 56., "height": 56.},
                f"Incorrect native FAB bounds: {bounds}")
        clip = layout(frame, FAB)["clip"]
        require(clip == viewport, f"FAB must be fully visible: {clip}")
        windows.append(onscreen_windows(app.proc.pid))
        require(windows == [0, 0], f"Headless Gallery showed a window: {windows}")
    return {"viewport": viewport, "fab": bounds, "clip": clip, "windows": windows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ("gallery", "cli", "out"):
        parser.add_argument(f"--{field}", required=True, type=Path)
    args = parser.parse_args()
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use a private /tmp directory")
    out.mkdir()
    plan = SimpleNamespace(gallery=args.gallery.resolve(strict=True),
                           cli=args.cli.resolve(strict=True), out=out)
    check_fab = run_path(str(Path(__file__).parent / "native-scripts/replay-fab-dialog.py"))["check_fab"]
    binaries = {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}
    profiles = [profile(plan, width, height, check_fab)
                for width, height in ((960, 1800), (320, 400))]
    require(binaries == {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}, "Binaries changed")
    report = {"schema": "surface.test.native-fab-fixed-viewport.v1",
              "measurement": "two separately launched fixed headless viewports",
              "binaries": binaries, "profiles": profiles}
    (out / "native-fab-bounds.json").write_text(json.dumps(report, indent=2) + "\n")
    print("Native RTL FAB measured at y1720 and y320 in two fixed headless viewports.")


if __name__ == "__main__":
    main()
