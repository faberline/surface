"""Exercise the newer Radio Button and Group in the native headless Gallery."""

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, focus, key, node, visible
from window_check import onscreen_windows

PREFIX = "radio-demo-"
SHOW, BACK, THEME, RTL, LARGE, LAYOUT, CLEAR, GROUP = (
    PREFIX + part for part in ("show", "back", "theme", "rtl", "large-text", "layout", "clear", "frequency")
)
IDS = tuple(PREFIX + part for part in ("daily", "locked", "weekly", "monthly"))
DAILY, LOCKED, WEEKLY, MONTHLY = IDS
NAMES = ("每天", "即時通知", "每週一通知", "每月")


def verify_selection(frame, expected):
    group = node(frame, GROUP)
    require(group["role"] == "RadioGroup" and group["name"] == "通知頻率", "Radio group name changed")
    for stable_id, name in zip(IDS, NAMES, strict=True):
        row = node(frame, stable_id)
        require(row["role"] == "Radio" and row["name"] == name, f"Incorrect semantics for {stable_id}")
        require(row["checked"] is (stable_id == expected), f"Incorrect checked bit for {stable_id}")
        require(row["disabled"] is (stable_id == LOCKED), f"Incorrect disabled bit for {stable_id}")


def geometry(frame, rtl):
    rows = {}
    for stable_id in IDS:
        target = visible(frame, stable_id, 48)
        ring = visible(frame, stable_id + "-ring")
        focus_ring = visible(frame, stable_id + "-focus-ring")
        layer = visible(frame, stable_id + "-state-layer")
        label = visible(frame, stable_id + "-label")
        require((ring["width"], ring["height"]) == (20, 20), "The ring is not 20 dp")
        require((layer["width"], layer["height"]) == (40, 40), "The state layer is not 40 dp")
        require((focus_ring["width"], focus_ring["height"]) == (30, 30), "The focus circle is not 30 dp")
        require(all(same_box(target, part) for part in (ring, focus_ring, layer, label)),
                "The ring or label was clipped by its target")
        require(label["x"] + label["width"] <= ring["x"] if rtl else
                ring["x"] + ring["width"] <= label["x"], "Incorrect logical leading edge")
        rows[stable_id] = {"target": target, "ring": ring, "focus_ring": focus_ring,
                           "state_layer": layer, "label": label}
    require(len({row["ring"]["x"] for row in rows.values()}) == 1, "The vertical radio rings do not align")
    dot = visible(frame, WEEKLY + "-dot")
    require((dot["width"], dot["height"]) == (10, 10), "The selected dot is not 10 dp")
    rows[WEEKLY]["dot"] = dot
    if rtl:
        require(rows[WEEKLY]["label"]["height"] >= 96, "The large weekly label did not wrap")
    return rows


def sample_dot(plan, file, dot, color):
    _, _, pixels = read_png((plan.out / file).read_bytes())
    x, y = round(dot["x"] + dot["width"] / 2), round(dot["y"] + dot["height"] / 2)
    pixel = list(pixels[y][4 * x:4 * (x + 1)])
    require(pixel == color and pixel[3] == 255, "The selected dot pixel does not match its authored ink")
    return {"point": [x, y], "rgba": pixel}


def profile(plan, title, width, height, compact):
    with launch(plan, title, width, height) as app:
        windows = [onscreen_windows(app.proc.pid)]
        if compact:
            focus_by_tab(app, SHOW)
            frame = key(app, "enter")
        else:
            frame = activate(app, SHOW)
        focus(frame, BACK)
        verify_selection(frame, WEEKLY)

        for control in (THEME, RTL, LARGE):
            frame = activate(app, control) if compact else key(app, "tab")
            focus(frame, control)
            verify_selection(frame, WEEKLY)
        focus(key(app, "tab"), LAYOUT)
        frame = key(app, "tab")
        focus(frame, WEEKLY)
        verify_selection(frame, WEEKLY)
        boxes = geometry(frame, compact)
        image = capture(app, plan, title, frame=frame, sample_targets=[WEEKLY + "-label"])
        image["dot_pixel"] = sample_dot(
            plan, image["name"], boxes[WEEKLY]["dot"], node(frame, WEEKLY + "-dot")["style"]["background"]
        )
        windows.append(onscreen_windows(app.proc.pid))

        frame = key(app, "space")
        focus(frame, WEEKLY)
        verify_selection(frame, WEEKLY)
        physical_next = "left" if compact else "right"
        for arrow, target in ((physical_next, MONTHLY), (physical_next, DAILY), ("down", WEEKLY)):
            frame = key(app, arrow)
            focus(frame, target)
            verify_selection(frame, target)
        app.wire("Action.activate", {"id": LOCKED}, expected_error="disabled")
        focus(key(app, "tab"), CLEAR)
        frame = key(app, "tab", reverse=True)
        focus(frame, WEEKLY)
        verify_selection(frame, WEEKLY)
        focus(key(app, "tab"), CLEAR)
        frame = key(app, "space")
        verify_selection(frame, None)
        focus(frame, CLEAR)
        frame = key(app, "tab", reverse=True)
        focus(frame, MONTHLY)
        verify_selection(frame, None)
        focus(key(app, "tab", reverse=True), LAYOUT)
        frame = key(app, "tab")
        focus(frame, DAILY)
        verify_selection(frame, None)
        focus(activate(app, BACK), SHOW)
        windows.append(onscreen_windows(app.proc.pid))
        require(windows == [0, 0, 0], "Gallery had a visible window")
    require(app.proc.returncode is not None, "Gallery did not exit")
    return {"viewport": [width, height], "rtl": compact, "text_scale": 2 if compact else 1,
            "windows": windows, "checked_at_entry": WEEKLY, "physical_next": physical_next,
            "no_selection_forward": DAILY, "no_selection_reverse": MONTHLY, "geometry": boxes, "png": image}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ("gallery", "cli", "out"):
        parser.add_argument(f"--{field}", required=True, type=Path)
    args = parser.parse_args()
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use a private /tmp directory")
    out.mkdir()
    plan = SimpleNamespace(gallery=args.gallery.resolve(strict=True), cli=args.cli.resolve(strict=True), out=out)
    hashes = {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}
    profiles = [
        profile(plan, "wide-light", 960, 800, False),
        profile(plan, "compact-dark-rtl-200", 320, 720, True),
    ]
    require(hashes == {"gallery": digest(plan.gallery), "cli": digest(plan.cli)},
            "A pinned native executable changed")
    report = {"schema": "surface.test.radio.native.v1", "binaries": hashes,
              "rendering": "headless", "transports": ["Page.screenshot", "CLI", "MCP"],
              "profiles": profiles}
    (out / "native-gallery-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print("Both native Radio profiles passed. The three screenshot transports matched.")


if __name__ == "__main__":
    main()
