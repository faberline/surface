"""Verify headless Material chips and byte parity across three screenshot transports."""

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

PREFIX = "chip-demo-"
SHOW, BACK, THEME, RTL, LARGE, SINGLE = (
    PREFIX + name for name in ("show", "back", "theme", "rtl", "large", "single")
)
ACTIONS = tuple(PREFIX + name for name in (
    "assist", "assist-elevated", "suggest", "suggest-elevated", "disabled"
))
NEWS, IMAGES, FILTER, FILTER_MENU = (
    PREFIX + name for name in ("news", "images", "filter-type", "filter-type-menu")
)
CHEN, CHEN_REMOVE, MEI, LIN, LIN_REMOVE = (
    PREFIX + name for name in ("chen", "chen-remove", "mei", "lin", "lin-remove")
)


def parent(frame, target):
    _, child = find(frame, target)
    require(child is not None, f"Missing target {target}")
    return frame["dom"]["nodes"][child["parent"]]


def split(frame, body, tail, rtl):
    a, b = visible(frame, body, 48), visible(frame, tail, 48)
    face = visible(frame, body + "-face")
    first, second = (b, a) if rtl else (a, b)
    require(abs(first["x"] + first["width"] - second["x"]) < 0.6,
            "Two independent hit areas must touch without overlapping")
    require(face["width"] >= 88 and face["height"] >= 32, "The split face is too small")
    require(same_box(a, visible(frame, body + "-label")), "The label spills out of its action")
    for target in (body, tail):
        require(parent(frame, target).get("grid_role") == "GridCell",
                f"{target} does not have its own grid cell")
        require(node(frame, target)["role"] == "Button", "Its real action role was lost")
    return {"body": a, "trailing": b, "face": face}


def verify_controls(frame, rtl):
    for id_ in (PREFIX + "actions", PREFIX + "filters", PREFIX + "contacts"):
        require(node(frame, id_).get("grid_role") == "Grid", f"Expected named grid {id_}")
    require(node(frame, NEWS)["checked"] is True and node(frame, IMAGES)["checked"] is False,
            "Initial Filter selections differ")
    require(node(frame, ACTIONS[-1])["disabled"], "Disabled suggestion was enabled")
    require(parent(frame, MEI).get("grid_role") == "GridCell", "Remove-only lacks its cell")
    require(find(frame, MEI + "-remove")[1] is None, "Remove-only created a second action")
    if rtl:
        require(find(frame, FILTER_MENU)[1] is None, "Compact Filter must have only one action")
        require(node(frame, FILTER)["expanded"] is False, "Compact Filter is not a closed menu trigger")
    else:
        require(node(frame, FILTER_MENU)["expanded"] is False, "Missing independent Filter menu trigger")


def activate_menu(app, anchor):
    frame = activate(app, anchor)
    if find(frame, PREFIX + "documents")[1] is None:
        frame = settle(app)
    focus(frame, PREFIX + "documents")
    frame = key(app, "enter")
    focus(frame, anchor)
    require(find(frame, PREFIX + "documents")[1] is None, "Menu was not dismissed")
    return frame


def verify_pixels(plan, frame, image, compact):
    _, _, rows = read_png((plan.out / image["name"]).read_bytes())
    samples = {}
    choices = (NEWS,) if compact else (NEWS, ACTIONS[1], IMAGES)
    for key in choices:
        id_ = key + "-face"
        face = visible(frame, id_)
        x = int(face["x"] + 3)
        y = int(face["y"] + face["height"] / 2)
        ink = list(rows[y][4 * x:4 * (x + 1)])
        require(ink == node(frame, id_)["style"]["background"], f"Incorrect chip fill for {id_}")
        samples[id_] = {"point": [x, y], "rgba": ink}
    return samples


def profile(plan, name, width, height, compact):
    with launch(plan, name, width, height) as app:
        windows = [onscreen_windows(app.proc.pid)]
        focus_by_tab(app, SHOW)
        frame = key(app, "enter")
        focus(frame, BACK)
        if compact:
            for control in (THEME, RTL, LARGE):
                focus(activate(app, control), control)
            focus(key(app, "tab"), SINGLE)
        else:
            for control in (THEME, RTL, LARGE, SINGLE):
                focus(key(app, "tab"), control)
        focus(key(app, "tab"), ACTIONS[0])
        focus(key(app, "tab"), NEWS)
        frame = key(app, "tab")
        focus(frame, CHEN)
        verify_controls(frame, compact)
        direction = "left" if compact else "right"
        frame = key(app, direction)
        focus(frame, CHEN_REMOVE)
        geometry = split(frame, CHEN, CHEN_REMOVE, compact)
        if compact:
            focus(key(app, "left"), MEI)
            frame = key(app, "down")
            focus(frame, LIN_REMOVE)
            long_chip = layout(frame, LIN + "-label")["bounds"]
            require(long_chip["height"] >= 40, "Large label has no enlarged line box")
        else:
            for id_ in ACTIONS:
                require(visible(frame, id_ + "-face")["height"] == 32, "Default chip face is not 32 dp")
        text_target = LIN if compact else CHEN
        image = capture(app, plan, name, frame=frame, sample_targets=[text_target + "-label"])
        image["face_pixels"] = verify_pixels(plan, frame, image, compact)
        windows.append(onscreen_windows(app.proc.pid))

        if not compact:
            app.wire("Action.activate", {"id": ACTIONS[-1]}, expected_error="disabled")
        anchor = FILTER if compact else FILTER_MENU
        activate_menu(app, anchor)
        # Tab enters the recipients grid once, then the adjacent remove action
        # is only reached through the grid's own key routing.
        focus(key(app, "tab"), CHEN)
        focus(key(app, direction), CHEN_REMOVE)
        frame = key(app, "delete")
        require(all(find(frame, target)[1] is None for target in (CHEN, CHEN_REMOVE)),
                "Deleting a split chip left a stale action")
        focused = frame["interaction"]["focused"]
        require(focused in (MEI, LIN, LIN_REMOVE), f"Invalid focus after removal {focused!r}")
        focus(activate(app, BACK), SHOW)
        windows.append(onscreen_windows(app.proc.pid))
        require(windows == [0, 0, 0], "Gallery had a visible window")
    require(app.proc.returncode is not None, "Gallery did not exit")
    return {
        "viewport": [width, height],
        "rtl": compact,
        "text_scale": 2 if compact else 1,
        "windows": windows,
        "split": geometry,
        "menu_trigger": anchor,
        "focus_after_delete": focused,
        "png": image,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ("gallery", "cli", "out"):
        parser.add_argument(f"--{field}", required=True, type=Path)
    args = parser.parse_args()
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use a private /tmp directory")
    out.mkdir()
    plan = SimpleNamespace(gallery=args.gallery.resolve(strict=True), cli=args.cli.resolve(strict=True), out=out)
    pins = {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}
    profiles = [
        profile(plan, "wide-light-chips", 960, 800, False),
        profile(plan, "compact-dark-rtl-200", 320, 760, True),
    ]
    require(pins == {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}, "Pinned executable changed")
    report = {
        "schema": "surface.test.chips.native.v1",
        "binaries": pins,
        "rendering": "headless",
        "transports": ["Page.screenshot", "CLI", "MCP"],
        "profiles": profiles,
    }
    (out / "native-gallery-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print("Wide light and compact dark RTL chips passed with matching PNGs and zero visible windows.")


if __name__ == "__main__":
    main()
