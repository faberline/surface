"""Verify Material divider geometry and actual PNG ink on headless Metal."""

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, focus, key, node, visible
from window_check import onscreen_windows


PREFIX = "divider-demo-"


def verify_ink(plan, shot, frame, suffix, box):
    _, _, rows = read_png((plan.out / shot["name"]).read_bytes())
    x, y, width, height = [round(box[field]) for field in ("x", "y", "width", "height")]
    color = bytes(node(frame, PREFIX + suffix)["style"]["background"])

    def pixel(column, row):
        return rows[row][4 * column:4 * column + 4]

    if suffix == "vertical":
        actual = [pixel(x, row) for row in range(y + 1, y + height - 1)]
        adjacent = [pixel(x - 1, y + height // 2), pixel(x + 1, y + height // 2)]
    else:
        actual = [pixel(column, y) for column in range(x + 1, x + width - 1)]
        adjacent = [pixel(x + width // 2, y - 1), pixel(x + width // 2, y + 1)]
    require(all(sample == color for sample in actual), f"{suffix} did not paint the outline color")
    require(all(sample != color for sample in adjacent), f"{suffix} painted outside its thickness")
    return {"bounds": box, "rgba": list(color), "interior_pixels": len(actual)}


def replay(plan, compact):
    profile = "compact-dark-rtl-200" if compact else "wide-light-ltr"
    viewport = (320, 640) if compact else (960, 640)
    windows = []
    with launch(plan, profile, *viewport) as app:
        def count_windows():
            count = onscreen_windows(app.proc.pid)
            require(count == 0, f"{profile} opened a window")
            windows.append(count)

        count_windows()
        activate(app, "section-3")
        focus_by_tab(app, PREFIX + "show")
        frame = key(app, "enter")
        focus(frame, PREFIX + "back")
        if compact:
            for suffix in ("theme", "rtl", "large-text"):
                frame = activate(app, PREFIX + suffix)
        focus_by_tab(app, PREFIX + "back")
        frame = app.dom()
        full = visible(frame, PREFIX + "full")
        inset = visible(frame, PREFIX + "inset")
        middle = visible(frame, PREFIX + "middle")
        vertical = visible(frame, PREFIX + "vertical")
        require((full["height"], inset["height"], middle["height"], vertical["width"]) == (1., 1., 1., 1.),
                "Divider thickness changed")
        require(inset["x"] - full["x"] == (0. if compact else 16.)
                and full["width"] - inset["width"] == 16., "Single inset does not mirror")
        require(middle["x"] - full["x"] == 16. and full["width"] - middle["width"] == 32.,
                "Middle inset margins changed")
        require(vertical["height"] == 64., "Vertical divider no longer fills its 64 dp row")
        shot = capture(app, plan, profile)
        ink = {
            suffix: verify_ink(plan, shot, frame, suffix, box)
            for suffix, box in (("full", full), ("inset", inset), ("middle", middle), ("vertical", vertical))
        }
        count_windows()
        frame = key(app, "enter")
        focus(frame, PREFIX + "show")
        count_windows()
    return {"profile": profile, "viewport": viewport, "ink": ink, "capture": shot, "visible_windows": windows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for argument in ("gallery", "cli", "out"):
        parser.add_argument("--" + argument, type=Path, required=True)
    args = parser.parse_args()
    plan = SimpleNamespace(
        gallery=args.gallery.resolve(strict=True),
        cli=args.cli.resolve(strict=True),
        out=args.out.resolve(),
    )
    plan.out.mkdir()
    pins = {name: digest(getattr(plan, name)) for name in ("gallery", "cli")}
    result = {
        "schema": "surface.test.material-divider-native.v1",
        "binaries": pins,
        "transport": ["Page.screenshot", "CLI", "MCP"],
        "profiles": [replay(plan, compact) for compact in (False, True)],
    }
    require(pins == {name: digest(getattr(plan, name)) for name in pins}, "Binaries changed")
    (plan.out / "native-divider-report.json").write_text(json.dumps(result, indent=2) + "\n")
    print("PASS four divider variants, inset mirroring, actual PNG colors, focus return and zero windows")


if __name__ == "__main__":
    main()
