"""Verify Regular and Heavy Divider paint from the same headless Gallery."""

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, focused, launch
from render_helpers import activate, capture, focus, key, node, visible
from window_check import onscreen_windows


PREFIX = "divider-demo-"
WEIGHT = PREFIX + "weight"
DIVIDERS = ("full", "inset", "middle", "vertical")


def stripe(bounds, thickness, suffix, color, background, png):
    width, height, rows = png
    x, y, w, h = (bounds[axis] for axis in ("x", "y", "width", "height"))
    require(all(float(value).is_integer() for value in (x, y, w, h)),
            f"{suffix} is not aligned at 1× screenshot scale: {bounds}")
    x, y, w, h = map(int, (x, y, w, h))
    right, bottom = x + w, y + h
    require((w if suffix == "vertical" else h) == thickness and w > 0 and h > 0,
            f"{suffix} does not measure {thickness} dp: {bounds}")
    require(0 < x < right < width and 0 < y < bottom < height,
            f"{suffix} has no complete adjacent border: {bounds}")
    is_vertical = suffix == "vertical"
    start, stop = (y + 1, bottom - 1) if is_vertical else (y, bottom)
    for row in rows[start:stop]:
        require(row[4*x:4*right] == color*w, f"{suffix} has a gap in its complete ink rectangle")
    caps = []
    if is_vertical:
        for row in (rows[y], rows[bottom-1]):
            cap = [row[4*column:4*(column+1)] for column in range(x,right)]
            require(all(pixel != background and all(min(a,b) <= c <= max(a,b)
                        for a,b,c in zip(background,color,pixel)) for pixel in cap),
                    f"{suffix} cap does not fill its entire depth with outline ink")
            caps.append([list(pixel) for pixel in cap])
    outer_rows = [rows[edge][4*(x-1):4*(right+1)] for edge in (y-1, bottom)]
    adjacent = [edge[index:index+4] for edge in outer_rows for index in range(0, len(edge), 4)]
    for row in rows[y:bottom]:
        adjacent.extend([row[4*(x-1):4*x], row[4*right:4*(right+1)]])
    require(all(pixel == background for pixel in adjacent),
            f"{suffix} painted outside {bounds} at {thickness} dp")
    return {"bounds": bounds, "rgba": list(color), "background": list(background),
            "solid_pixels": w*(stop-start), "painted_cap_rows": caps,
            "physical_thickness_px": thickness, "adjacent_pixels": len(adjacent),
            "matching_adjacent_pixels": 0}


def geometry(frame, compact, thickness):
    boxes = {suffix: visible(frame, PREFIX + suffix) for suffix in DIVIDERS}
    full, inset, middle, vertical = boxes.values()
    require((full["height"], inset["height"], middle["height"], vertical["width"])
            == (thickness,) * 4, f"All four samples must use {thickness} dp: {boxes}")
    require(inset["x"] - full["x"] == (0. if compact else 16.)
            and full["width"] - inset["width"] == 16., "Single inset does not mirror")
    require(middle["x"] - full["x"] == 16. and full["width"] - middle["width"] == 32.,
            "Middle inset margins changed")
    require(vertical["height"] == 64., "Vertical divider no longer fills its 64 dp row")
    return boxes


def capture_weight(app, plan, frame, profile, viewport, compact, heavy):
    thickness = 8 if heavy else 1
    selection = node(frame, WEIGHT)
    require(selection["name"] == "粗線" and selection["selected"] is heavy,
            f"Wrong weight toggle for {thickness} dp: {selection['selected']}")
    focus(frame, WEIGHT)
    boxes = geometry(frame, compact, thickness)
    weight = "heavy" if heavy else "regular"
    shot = capture(app, plan, f"{profile}-{weight}", frame=frame)
    require((shot["width"], shot["height"], shot["scale"]) == (*viewport, 1.),
            f"{profile} did not capture native 1× pixels: {shot}")
    png = read_png((plan.out / shot["name"]).read_bytes())
    outline = bytes((73, 69, 78, 255) if compact else (202, 196, 207, 255))
    background = bytes(node(frame, "surface-page")["style"]["background"])
    require(png[2][0][:4] == background, "The captured Gallery background changed")
    ink = {}
    for suffix, box in boxes.items():
        authored = bytes(node(frame, PREFIX + suffix)["style"]["background"])
        require(authored == outline, f"{suffix} does not use the expected Outline Variant color")
        ink[suffix] = stripe(box, thickness, suffix, outline, background, png)
    return {"weight": weight, "thickness_dp": thickness, "selected": heavy,
            "focused": WEIGHT, "ink": ink, "capture": shot}, boxes


def replay(plan, compact):
    profile = "compact-dark-rtl-200" if compact else "wide-light-ltr"
    viewport = (320, 800) if compact else (960, 800)
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
        back = PREFIX + "back"
        if focused(frame) != back:
            focus_by_tab(app, back)
            frame = app.dom()
        focus(frame, back)
        tab_path = [back]
        for suffix in ("theme", "rtl", "large-text", "weight"):
            target = PREFIX + suffix
            frame = key(app, "tab")
            focus(frame, target)
            visible(frame, target, 48.)
            tab_path.append(target)
        regular, initial = capture_weight(app, plan, frame, profile, viewport, compact, False)
        count_windows()
        frame = key(app, "enter")
        heavy, _ = capture_weight(app, plan, frame, profile, viewport, compact, True)
        count_windows()
        frame = key(app, "enter")
        focus(frame, WEIGHT)
        require(node(frame, WEIGHT)["selected"] is False and geometry(frame, compact, 1) == initial,
                "Toggling back did not restore the same Regular geometry")
        frame = key(app, "tab")
        focus(frame, back)
        tab_path.append(back)
        frame = key(app, "enter")
        focus(frame, PREFIX + "show")
        count_windows()
    return {"profile": profile, "viewport": viewport, "weights": [regular, heavy],
            "tab_path": tab_path, "restored_focus": PREFIX + "show", "visible_windows": windows}


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
    require(plan.out.is_relative_to(Path("/tmp").resolve()), "Use a private /tmp output directory")
    plan.out.mkdir()
    pins = {name: digest(getattr(plan, name)) for name in ("gallery", "cli")}
    result = {
        "schema": "surface.test.material-divider-native.v2",
        "binaries": pins,
        "transport": ["Page.screenshot", "CLI", "MCP"],
        "profiles": [replay(plan, compact) for compact in (False, True)],
    }
    require(pins == {name: digest(getattr(plan, name)) for name in pins}, "Binaries changed")
    (plan.out / "native-divider-report.json").write_text(json.dumps(result, indent=2) + "\n")
    print("PASS two sealed headless profiles, both weights, four PNGs with all stripe pixels, zero windows")


if __name__ == "__main__":
    main()
