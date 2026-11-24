"""Verify the Material Checkbox Gallery using the offscreen native backend."""

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, node, visible
from window_check import onscreen_windows

PREFIX = "checkbox-demo-"
SHOW, BACK, THEME, RTL, LARGE, HELP = (
    PREFIX + part for part in ("show", "back", "theme", "rtl", "large-text", "error-hint")
)
IDS = tuple(PREFIX + part for part in ("all", "notes", "attachments", "locked", "consent"))
ALL, NOTES, ATTACHMENTS, LOCKED, CONSENT = IDS
NAMES = ("同步全部", "筆記", "附件", "已鎖定的專案", "允許在行動網路上同步大型附件")
INITIAL = ("mixed", "checked", "unchecked", "mixed", "unchecked")
TEXT = "請勾選後再繼續。"


def check_state(frame, expected, *, invalid=True):
    rows = [node(frame, stable_id) for stable_id in IDS]
    require(tuple(row["name"] for row in rows) == NAMES, "Checkbox names changed")
    for row, value in zip(rows, expected, strict=True):
        require(row["role"] == "Checkbox", "A row lost its Checkbox role")
        require(row["checked"] is (value != "unchecked"), f"Unexpected checked bit for {row['name']}")
        require(row["mixed"] is (value == "mixed"), f"Unexpected mixed bit for {row['name']}")
    require([row["disabled"] for row in rows] == [False, False, False, True, False],
            "The locked row was not kept disabled")
    require(rows[-1].get("invalid", False) is invalid, "The consent row lost its invalid state")
    require(rows[-1].get("described_by_text") == (HELP if invalid else None),
            "The consent description points to the wrong ID")
    if invalid:
        require(node(frame, HELP)["text"] == TEXT, "Visible error help is missing")
    else:
        require(find(frame, HELP)[1] is None, "Stale help remains after confirmation")


def geometry(frame, compact):
    rows = {}
    for stable_id in IDS:
        target = visible(frame, stable_id, 48)
        shape = visible(frame, stable_id + "-box")
        label = visible(frame, stable_id + "-label")
        ring = visible(frame, stable_id + "-focus-ring")
        require((shape["width"], shape["height"]) == (18, 18), "Wrong Checkbox box size")
        require((ring["width"], ring["height"]) == (50, 50), "Wrong focus ring bounds")
        require(all(same_box(target, bounds) for bounds in (shape, label, ring)),
                "The hit bounds or label clipped the Checkbox artwork")
        require(
            label["x"] + label["width"] <= shape["x"] if compact
            else shape["x"] + shape["width"] <= label["x"],
            "Checkbox artwork is not at logical start",
        )
        rows[stable_id] = {"target": target, "box": shape, "label": label, "ring": ring}
    require(len({row["box"]["x"] for row in rows.values()}) == 1, "Leading Checkbox boxes do not align")
    if compact:
        require(rows[CONSENT]["label"]["height"] >= 96, "Large Chinese label did not wrap")
    return rows


def sample_error(plan, filename, shape, color):
    w, h, pixels = read_png((plan.out / filename).read_bytes())
    x, y = round(shape["x"] + shape["width"] / 2), round(shape["y"] + 1)
    require(0 <= x < w and 0 <= y < h, "Sample is outside the capture")
    pixel = list(pixels[y][4 * x:4 * (x + 1)])
    require(pixel == color, f"Error-box edge {pixel} differs from error text {color}")
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
        check_state(frame, INITIAL)

        for stable_id in (THEME, RTL, LARGE):
            frame = activate(app, stable_id) if compact else key(app, "tab")
            focus(frame, stable_id)
            check_state(frame, INITIAL)

        frame = key(app, "tab")
        focus(frame, ALL)
        boxes = geometry(frame, compact)
        visible(frame, HELP)
        image = capture(app, plan, title, frame=frame, sample_targets=[CONSENT + "-label", HELP])
        image["error_pixel"] = sample_error(plan, image["name"], boxes[CONSENT]["box"],
                                            node(frame, HELP)["style"]["foreground"])
        windows.append(onscreen_windows(app.proc.pid))

        frame = key(app, "space")
        check_state(frame, ("checked", "checked", "checked", "mixed", "unchecked"))
        frame = key(app, "tab")
        focus(frame, NOTES)
        frame = key(app, "enter")
        check_state(frame, ("mixed", "unchecked", "checked", "mixed", "unchecked"))
        frame = key(app, "tab")
        focus(frame, ATTACHMENTS)
        frame = key(app, "enter")
        check_state(frame, ("unchecked", "unchecked", "unchecked", "mixed", "unchecked"))
        app.wire("Action.activate", {"id": LOCKED}, expected_error="disabled")
        frame = key(app, "tab")
        focus(frame, CONSENT)
        frame = key(app, "space")
        final = ("unchecked", "unchecked", "unchecked", "mixed", "checked")
        check_state(frame, final, invalid=False)
        focus(frame, CONSENT)
        focus(activate(app, BACK), SHOW)
        windows.append(onscreen_windows(app.proc.pid))
        require(windows == [0, 0, 0], "The Gallery showed an on-screen window")
    require(app.proc.returncode is not None, "Gallery process did not exit")
    return {"viewport": [width, height], "rtl": compact, "text_scale": 2 if compact else 1,
            "windows": windows, "before": INITIAL, "after": final, "geometry": boxes, "png": image}


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
        profile(plan, "compact-dark-rtl-200", 320, 720, True),
    ]
    require(hashes == {"gallery": digest(plan.gallery), "cli": digest(plan.cli)},
            "A pinned binary changed during the checks")
    report = {"schema": "surface.test.checkbox.native.v1", "binaries": hashes,
              "rendering": "headless", "transports": ["Page.screenshot", "CLI", "MCP"],
              "profiles": profiles}
    (out / "native-gallery-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print("Two native Checkbox profiles passed. API, CLI and MCP produced the same PNG bytes.")


if __name__ == "__main__":
    main()
