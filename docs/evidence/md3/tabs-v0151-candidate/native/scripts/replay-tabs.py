"""Validate real Surface tabs and screenshots without creating a window."""

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, visible
from window_check import onscreen_windows

PREFIX = "tabs-demo-"
SHOW, BACK, THEME, RTL, LARGE, SCROLLABLE, AUTO, PRIMARY, SECONDARY = (
    PREFIX + part for part in ("show", "back", "theme", "rtl", "large", "scrollable", "automatic", "primary", "secondary")
)
OVERVIEW, LOCK, ACTIVITY, HISTORY = (
    PREFIX + part for part in ("overview", "lock", "activity", "history")
)
UNREAD = PREFIX + "unread"


def selected(frame, tab, visible_primary):
    bar = node(frame, PRIMARY)
    require(bar["role"] == "TabList" and bar["name"] == "工作區頁籤", "Incorrect named tablist")
    roles = [(id_, node(frame, id_)) for id_ in (OVERVIEW, LOCK, ACTIVITY)]
    if visible_primary:
        roles.append((HISTORY, node(frame, HISTORY)))
    for id_, item in roles:
        require(item["role"] == "Tab" and item["selected"] is (id_ == tab), f"Bad state of {id_}")
        require(item["disabled"] is (id_ == LOCK), f"Bad disabled state of {id_}")
        require(item.get("tab_controls") == id_ + "-panel", f"Bad tab panel target for {id_}")
        _, panel = find(frame, id_ + "-panel")
        require(bool(panel) is (id_ == tab), "Only the selected panel can have mounted content")
    require(node(frame, UNREAD)["selected"] is True, "Changing primary tabs changed secondary state")


def pixel(rows, x, y):
    offset = int(x) * 4
    return list(rows[int(y)][offset:offset + 4])


def inspect_png(plan, image, frame, tab, compact):
    width, height, rows = read_png((plan.out / image["name"]).read_bytes())
    strip = visible(frame, PRIMARY)
    target = visible(frame, tab, 48)
    label = visible(frame, tab + "-label")
    ink = node(frame, tab + "-label")["style"]["foreground"]
    divider = node(frame, PRIMARY + "-divider")["style"]["background"]
    x = target["x"] + target["width"] / 2
    y = strip["y"] + strip["height"]
    require(0 < x < width and 4 < y <= height, "Indicator sample outside frame")
    require(all(pixel(rows, x, y - depth) == ink for depth in (1, 2, 3)),
            "Primary indicator must have three full-height pixels at center")
    require(pixel(rows, x, y - 4) != ink, "Primary indicator grew above 3 dp")
    require(same_box(target, label), "Focused tab label clipped")
    if compact:
        require(label["height"] >= 40, "Large CJK label did not receive 200 percent line height")
    else:
        require(strip["height"] == 64, "Fixed primary bar has wrong height")
        lower = visible(frame, SECONDARY)
        require(lower["height"] == 48, "Fixed secondary bar has wrong height")
        locked = visible(frame, LOCK)
        divider_x = locked["x"] + locked["width"] / 2
        require(pixel(rows, divider_x, y - 1) == divider, "SurfaceVariant divider is missing")
        second = visible(frame, UNREAD)
        y2 = lower["y"] + lower["height"]
        mid = second["x"] + second["width"] / 2
        require(all(pixel(rows, mid, y2 - depth) == ink for depth in (1, 2)),
                "Secondary indicator must have two full-height pixels at center")
        require(pixel(rows, mid, y2 - 3) != ink, "Secondary indicator grew above 2 dp")
    return {"tab": target, "label": label, "strip": strip, "mark": [int(x), int(y - 2)], "rgba": ink}


def profile(plan, name, width, height, compact):
    with launch(plan, name, width, height) as app:
        windows = [onscreen_windows(app.proc.pid)]
        if compact:
            focus_by_tab(app, SHOW)
            frame = key(app, "enter")
        else:
            frame = activate(app, SHOW)
        focus(frame, BACK)
        selected(frame, OVERVIEW, False)
        if compact:
            for toggle in (THEME, RTL, LARGE, SCROLLABLE):
                focus(activate(app, toggle), toggle)
            focus(key(app, "tab"), AUTO)
        else:
            for target in (THEME, RTL, LARGE, SCROLLABLE, AUTO):
                focus(key(app, "tab"), target)
        focus(key(app, "tab"), OVERVIEW)
        arrow = "left" if compact else "right"
        frame = key(app, arrow)
        focus(frame, ACTIVITY)
        selected(frame, OVERVIEW, compact)
        focus(key(app, "down"), ACTIVITY)
        if compact:
            focus(key(app, "end"), HISTORY)
        primary = HISTORY if compact else ACTIVITY
        frame = key(app, "enter")
        focus(frame, primary)
        selected(frame, primary, compact)
        image = capture(app, plan, name, frame=frame, sample_targets=[primary + "-label"])
        geometry = inspect_png(plan, image, frame, primary, compact)
        windows.append(onscreen_windows(app.proc.pid))
        locked = layout(frame, LOCK)
        if same_box(locked["clip"], locked["bounds"]):
            app.wire("Action.activate", {"id": LOCK}, expected_error="disabled")
        focus(key(app, "tab"), primary + "-panel")
        focus(key(app, "tab"), UNREAD)
        focus(key(app, "tab"), PREFIX + "read-message")
        focus(key(app, "tab", reverse=True), UNREAD)
        focus(key(app, "tab", reverse=True), primary + "-panel")
        focus(key(app, "tab", reverse=True), primary)
        focus(activate(app, BACK), SHOW)
        windows.append(onscreen_windows(app.proc.pid))
        require(windows == [0, 0, 0], "Gallery had a visible window")
    require(app.proc.returncode is not None, "Gallery did not exit")
    return {
        "viewport": [width, height],
        "rtl": compact,
        "text_scale": 2 if compact else 1,
        "windows": windows,
        "initial": OVERVIEW,
        "physical_next": arrow,
        "active": primary,
        "geometry": geometry,
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
    binaries = {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}
    profiles = [
        profile(plan, "wide-light-fixed", 960, 800, False),
        profile(plan, "compact-dark-rtl-200", 320, 760, True),
    ]
    require(binaries == {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}, "A pinned executable changed")
    report = {
        "schema": "surface.test.tabs.native.v1",
        "binaries": binaries,
        "rendering": "headless",
        "transports": ["Page.screenshot", "CLI", "MCP"],
        "profiles": profiles,
    }
    (out / "native-gallery-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print("Fixed light and compact dark RTL tabs passed with matching PNGs and zero visible windows.")


if __name__ == "__main__":
    main()
