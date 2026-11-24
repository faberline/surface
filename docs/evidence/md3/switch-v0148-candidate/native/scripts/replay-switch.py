"""Verify the stock Switch Gallery and PNG transports on the native offscreen backend."""

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, node, visible
from window_check import onscreen_windows

PREFIX = "switch-demo-"
SHOW, BACK, THEME, RTL, LARGE = (
    PREFIX + part for part in ("show", "back", "theme", "rtl", "large-text")
)
IDS = tuple(PREFIX + part for part in (
    "plain", "selected-icon", "both-icons", "disabled-off", "disabled-on"
))
PLAIN, SELECTED, BOTH, DISABLED_OFF, DISABLED_ON = IDS
NAMES = ("通知", "相機", "行動網路同步", "位置存取", "自動備份")
INITIAL = [False, True, False, False, True]


def check_state(frame, selected):
    switches = [node(frame, key) for key in IDS]
    require(
        tuple(entry["name"] for entry in switches) == NAMES
        and all(entry["role"] == "Switch" for entry in switches),
        "The native DOM lost a Switch role or its stable name",
    )
    require(
        [entry["checked"] for entry in switches] == selected
        and all(entry["disabled"] == (index >= 3) for index, entry in enumerate(switches)),
        "A switch lost its controlled state",
    )


def geometry(frame, selected, rtl):
    rows = {}
    for index, stable_id in enumerate(IDS):
        row = visible(frame, stable_id, 48)
        track = visible(frame, stable_id + "-track")
        handle = visible(frame, stable_id + "-handle")
        label = visible(frame, stable_id + "-label")
        icon = find(frame, stable_id + "-icon")[1]
        require((track["width"], track["height"]) == (52, 32), "Wrong track size")
        require(same_box(row, track) and same_box(row, label)
                and same_box(track, handle), "Switch row lost visible content")
        expected_diameter = 24 if selected[index] or index >= 2 else 16
        require((handle["width"], handle["height"]) == (expected_diameter,) * 2,
                f"Wrong handle size for {stable_id}")
        center = 36 if selected[index] != rtl else 16
        require(abs(handle["x"] + handle["width"] / 2 - track["x"] - center) <= .5,
                f"Wrong thumb side for {stable_id}")
        require((icon is not None) == (index >= 2 or (index == 1 and selected[index])),
                f"Wrong icon variant for {stable_id}")
        require(
            track["x"] + track["width"] <= label["x"] if rtl
            else label["x"] + label["width"] <= track["x"],
            f"Label overlaps artwork for {stable_id}",
        )
        rows[stable_id] = {"target": row, "track": track, "handle": handle, "label": label}
    return rows


def png_sample(plan, image, target, background, foreground):
    _, _, rows = read_png((plan.out / image["name"]).read_bytes())
    x, y = round(target["x"] + target["width"] / 2), round(target["y"] + target["height"] / 2)
    pixel = list(rows[y][4 * x:4 * (x + 1)])
    require(pixel[:3] == foreground[:3] and pixel[3] == 255 and pixel[:3] != background[:3],
            f"Center sample is not the expected opaque thumb: {pixel}")
    return {"point": [x, y], "rgba": pixel}


def profile(plan, title, width, height, compact):
    checked = INITIAL.copy()
    with launch(plan, title, width, height) as app:
        windows = [onscreen_windows(app.proc.pid)]
        if compact:
            focus_by_tab(app, SHOW)
            frame = key(app, "enter")
        else:
            frame = activate(app, SHOW)
        focus(frame, BACK)
        check_state(frame, checked)

        for control in (THEME, RTL, LARGE):
            frame = activate(app, control) if compact else key(app, "tab")
            focus(frame, control)
            check_state(frame, checked)
        frame = key(app, "tab")
        focus(frame, PLAIN)
        frame = key(app, "space")
        checked[0] = True
        check_state(frame, checked)
        frame = key(app, "tab")
        focus(frame, SELECTED)
        frame = key(app, "enter")
        checked[1] = False
        check_state(frame, checked)
        frame = key(app, "tab")
        focus(frame, BOTH)
        settled = geometry(frame, checked, compact)
        if compact:
            require(settled[BOTH]["label"]["height"] >= 64, "Large Chinese text did not wrap")
        image = capture(app, plan, title, frame=frame, sample_targets=[BOTH + "-label"])
        image["handle_pixel"] = png_sample(
            plan, image, settled[PLAIN]["handle"],
            node(frame, "switch-demo")["style"]["background"],
            node(frame, PLAIN + "-handle")["style"]["background"],
        )
        windows.append(onscreen_windows(app.proc.pid))
        for disabled in (DISABLED_OFF, DISABLED_ON):
            app.wire("Action.activate", {"id": disabled}, expected_error="disabled")
        frame = key(app, "tab")
        require(frame["interaction"]["focused"] not in (DISABLED_OFF, DISABLED_ON),
                "Tab focused a disabled switch")
        check_state(frame, checked)
        focus(activate(app, BACK), SHOW)
        windows.append(onscreen_windows(app.proc.pid))
        require(windows == [0, 0, 0], "The Gallery had a visible window")
    require(app.proc.returncode is not None, "The Gallery process survived")
    return {
        "viewport": [width, height], "rtl": compact, "text_scale": 2 if compact else 1,
        "checked": checked, "windows": windows, "geometry": settled, "png": image,
    }


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
    hashes = {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}
    profiles = [
        profile(plan, "wide-light", 960, 800, False),
        profile(plan, "compact-dark-rtl-200", 320, 640, True),
    ]
    require(hashes == {"gallery": digest(plan.gallery), "cli": digest(plan.cli)},
            "A native binary changed during the checks")
    report = {"schema": "surface.test.switch.native.v1", "binaries": hashes,
              "rendering": "headless", "transports": ["Page.screenshot", "CLI", "MCP"],
              "profiles": profiles}
    (out / "native-gallery-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print("Two stock native headless profiles passed. PNG bytes matched across all three transports.")


if __name__ == "__main__":
    main()
