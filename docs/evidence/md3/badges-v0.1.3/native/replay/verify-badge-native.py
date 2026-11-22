#!/usr/bin/env python3
"""Verify Material Badges on a native Surface Gallery window."""

import argparse
import json
import math
from pathlib import Path
import stat
import subprocess

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, find_input, focus_by_tab, focused, launch, presses, same_box, wait_for_persisted_trace


PROFILES = {
    "light-wide": (960, 800, False, False),
    "dark-wide": (960, 800, True, False),
    "dark-rtl-200-compact": (320, 480, True, True),
}
INBOX = "badge-inbox"
BEFORE = "收件匣，999 則未讀"
AFTER = "收件匣，1000 則未讀"
SCALE = {"scale": 1, "max_side": 1200}


def index(snapshot, stable_id):
    matches = [i for i, n in enumerate(snapshot["dom"]["nodes"]) if n["stable_id"] == stable_id]
    require(len(matches) == 1, f"Expected exactly one node for {stable_id}")
    return matches[0]


def node(snapshot, i):
    return snapshot["dom"]["nodes"][i]


def geometry(snapshot, i):
    return snapshot["layout"]["nodes"][i]


def descendants(snapshot, i):
    child = node(snapshot, i)["first_child"]
    while child is not None:
        yield child
        yield from descendants(snapshot, child)
        child = node(snapshot, child)["next_sibling"]


def badge_faces(snapshot, parent, fill):
    return [i for i in descendants(snapshot, parent)
            if node(snapshot, i)["style"]["position"] == "Absolute"
            and node(snapshot, i)["style"]["background"] == fill]


def badge(snapshot, parent, fill, label):
    found = badge_faces(snapshot, parent, fill)
    require(len(found) == 1, f"Expected one anchored badge in {parent}: got {found}")
    i = found[0]
    text = [node(snapshot, child)["text"] for child in descendants(snapshot, i)
            if node(snapshot, child)["text"] is not None]
    require(text == ([label] if label else []), f"Wrong badge label: {text}")
    badge_node = node(snapshot, i)
    require(badge_node["role"] == "Group" and badge_node["stable_id"] is None
            and badge_node["tab_index"] is None, "Decoration must be anonymous and passive")
    return geometry(snapshot, i)


def relative(rect, outer):
    return [rect["x"] - outer["x"], rect["y"] - outer["y"], rect["width"], rect["height"]]


def pixels(rows, rect, color, tolerance=8):
    x0 = max(0, math.ceil(rect["x"]))
    x1 = min(len(rows[0]) // 4, math.floor(rect["x"] + rect["width"]))
    y0 = max(0, math.ceil(rect["y"]))
    y1 = min(len(rows), math.floor(rect["y"] + rect["height"]))
    return sum(1 for row in rows[y0:y1] for x in range(x0, x1)
               if max(abs(a - b) for a, b in zip(row[x * 4:x * 4 + 4], color)) <= tolerance)


def capture(app, folder, name, viewport, parity=False):
    path = folder / f"{name}.png"
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", path])
    data = path.read_bytes()
    width, height, rows = read_png(data)
    require((width, height) == (viewport["width"], viewport["height"])
            and (cli["width"], cli["height"], cli["scale"]) == (width, height, 1),
            "PNG does not match the native logical viewport")
    require(stat.S_IMODE(path.stat().st_mode) == 0o600, "CLI PNG must be a private new file")
    receipt = {"file": path.name, "sha256": digest(path), "bytes": len(data), "metadata": metadata(cli)}
    if parity:
        wire = app.wire("Page.screenshot", SCALE)
        mcp, mcp_meta, _ = app.mcp(SCALE)
        require(png_payload(wire) == data == mcp, "Raw, CLI and MCP PNG bytes differ")
        require(metadata(wire) == metadata(cli) == mcp_meta, "Transport metadata differs")
        (folder / f"{name}-raw.png").write_bytes(png_payload(wire))
        (folder / f"{name}-mcp.png").write_bytes(mcp)
        receipt["transport_parity"] = ["raw", "cli", "mcp"]
    return receipt, rows


def painted(rows, g, fill, ink=None):
    box = g["bounds"]
    require(same_box(g["clip"], box), f"Badge was cut off: {g}")
    fill_count = pixels(rows, box, fill)
    require(fill_count >= 11, f"Badge fill did not paint: got {fill_count}")
    receipt = {"bounds": box, "clip": g["clip"], "fill_pixels": fill_count}
    if ink:
        ink_count = pixels(rows, box, ink, tolerance=42)
        require(ink_count >= 4, f"Badge foreground did not paint: got {ink_count}")
        receipt["foreground_pixels"] = ink_count
    return receipt


def glyph_ink(frame, root, rows):
    icons = [i for i in descendants(frame, root) if node(frame, i)["text"] == "信"]
    require(len(icons) == 1, "The sample is missing its bundled-font inbox glyph")
    i = icons[0]
    bounds = geometry(frame, i)["bounds"]
    ink = pixels(rows, bounds, node(frame, i)["style"]["foreground"], tolerance=40)
    require(ink >= 9, f"The inbox glyph did not paint: {ink} matching pixels")
    return {"bounds": bounds, "pixels": ink}


def inside_contour(points, x, y):
    inside = False
    for (ax, ay), (bx, by) in zip(points, points[1:] + points[:1]):
        if (ay > y) != (by > y) and x < (bx - ax) * (y - ay) / (by - ay) + ax:
            inside = not inside
    return inside


def vector_ink(frame, root, rows, enlarged):
    icons = [i for i in descendants(frame, root) if node(frame, i).get("fill_contour")]
    require(len(icons) == 1, "The badged destination is missing its vector icon")
    i = icons[0]
    contour = node(frame, i)["fill_contour"]
    bounds = geometry(frame, i)["bounds"]
    require((bounds["width"], bounds["height"]) == (24, 24), "Destination icon is not 24 dp")
    points = [[x + bounds["x"], y + bounds["y"]] for x, y in contour["points"]]
    ink = contour["color"]
    width, height = len(rows[0]) // 4, len(rows)
    expected = visible = 0
    for y in range(max(0, math.ceil(bounds["y"])), min(height, math.floor(bounds["y"] + 24))):
        for x in range(max(0, math.ceil(bounds["x"])), min(width, math.floor(bounds["x"] + 24))):
            if not inside_contour(points, x + .5, y + .5):
                continue
            expected += 1
            sample = rows[y][4 * x:4 * x + 4]
            if max(abs(a - b) for a, b in zip(sample, ink)) <= 22:
                visible += 1
    # The normal top-trailing badge covers the icon's upper corner. The
    # enlarged layout reserves clearance so the icon remains independent.
    minimum_fraction = .8 if enlarged else .6
    require(expected >= 24 and visible / expected >= minimum_fraction,
            f"Navigation badge obscures the destination icon: {visible} of {expected} ink pixels")
    return {"bounds": bounds, "pixels": visible, "contour_pixels": expected}


def snapshot_after(app, kind):
    app.activate("badge-show-" + kind)
    focus_by_tab(app, INBOX)
    return app.dom()


def check_ax(options, app):
    if not options.ax_reader:
        return {"verified": False, "reason": "No AppKit reader supplied"}
    result = subprocess.run([options.ax_reader, str(app.proc.pid), AFTER, BEFORE, "999+"],
                            capture_output=True, check=True, timeout=15)
    state = json.loads(result.stdout)
    if not state["trusted"]:
        return {"verified": False, "reason": "Caller lacks macOS accessibility permission"}
    relevant = state["matches"]
    require(state["windows"] and 1 <= len(relevant) <= 2
            and all(b["name"] == AFTER and b["role"] == "AXButton"
                    and b["enabled"] and "AXPress" in b["actions"] for b in relevant),
            f"AppKit exposed a stale count or extra action: {relevant}")
    return {"verified": True, "windows": state["windows"], "full_count_buttons": relevant}


def verify(options, app, trace, folder, compact):
    snapshot = snapshot_after(app, "dot")
    dot_id = index(snapshot, "badge-standalone-dot")
    label_id = index(snapshot, "badge-standalone-label")
    require(node(snapshot, dot_id)["name"] == "有新訊息"
            and node(snapshot, label_id)["name"] == "有新內容"
            and all(node(snapshot, i)["role"] == "Text" for i in (dot_id, label_id)),
            "Standalone badge names are missing")
    fill = node(snapshot, dot_id)["style"]["background"]
    ink = node(snapshot, dot_id)["style"]["foreground"]
    viewport = snapshot["layout"]["viewport"]
    anchor = None
    report = {"error": fill, "on_error": ink}

    def anchor_position(frame):
        nonlocal anchor
        owner = index(frame, INBOX)
        region = index(frame, "badge-anchor-surface")
        box = geometry(frame, owner)["bounds"]
        position = relative(box, geometry(frame, region)["bounds"])
        require(position[2:] == [48, 48] and (anchor is None or anchor == position),
                "Badge moved or resized the 48 dp anchor")
        anchor = position
        return node(frame, owner), box, region

    def anchor_capture(frame, key, label, name, selected, parity=False):
        owner, box, region = anchor_position(frame)
        require(owner["name"] == name and owner["selected"] is selected and focused(frame) == INBOX,
                f"{key} lost the original anchor, focus or full accessible name")
        part = badge(frame, region, fill, label)
        offset = relative(part["bounds"], box)
        if label:
            horizontal = offset[0] + offset[2] if compact else 48 - offset[0]
            require(abs(horizontal - 12) <= 1 and abs(offset[1] + offset[3] - 14) <= 1,
                    f"{key}: unexpected badge overlap {offset}")
            require(offset[3] >= (31 if compact else 16), f"{key}: badge did not fit text")
        else:
            require(offset == ([0, 0, 6, 6] if compact else [42, 0, 6, 6]),
                    f"Dot is not at the top trailing edge: {offset}")
        pic, rows = capture(app, folder, key, viewport, parity=parity)
        pic["badge"] = painted(rows, part, fill, ink if label else None)
        pic["offset"] = offset
        return pic, rows

    report["dot"], rows = anchor_capture(snapshot, "dot", None, "收件匣，有新訊息", False)
    report["dot"]["anchor_glyph"] = glyph_ink(snapshot, index(snapshot, INBOX), rows)
    if not compact:
        report["standalone"] = [painted(rows, geometry(snapshot, dot_id), fill),
                                painted(rows, geometry(snapshot, label_id), fill, ink)]
    report["count-999"], _ = anchor_capture(snapshot_after(app, "count"), "count-999", "999", BEFORE, False)
    app.key("enter")
    report["selected-count-1000"], _ = anchor_capture(
        app.dom(), "selected-count-1000", "999+", AFTER, True, parity=compact)
    report["appkit"] = check_ax(options, app)
    app.key("tab")
    tab_after = focused(app.dom())
    require(tab_after == "badge-nav-0", f"Badge added an extra Tab target: {tab_after}")
    app.key("escape")
    rows = wait_for_persisted_trace(app, trace, lambda trace_rows: find_input(trace_rows, "Cancel"))
    events = presses(rows, INBOX)
    require(len(events) == 1, f"Expected one native press message, got {len(events)}")
    report["press"] = events[0]["data"]["source"]
    report["tab_after"] = tab_after
    report["label"], _ = anchor_capture(snapshot_after(app, "label"), "label", "新",
                                         "收件匣，有新訊息", True)

    frame = snapshot_after(app, "hidden")
    owner, box, region = anchor_position(frame)
    require(owner["name"] == "收件匣" and owner["selected"] is True and not badge_faces(frame, region, fill),
            "Hidden badge retained a decoration or extra accessible name")
    report["hidden"], rows = capture(app, folder, "hidden", viewport)
    dx, dy, width, height = report["label"]["offset"]
    area = {"x": box["x"] + dx, "y": box["y"] + dy, "width": width, "height": height}
    require(pixels(rows, area, fill) == 0, "Hidden badge still painted its old pixels")

    app.activate("badge-nav-1")
    frame = app.dom()
    destination = index(frame, "badge-nav-1")
    g = badge(frame, destination, fill, "999+")
    require(node(frame, destination)["name"] == AFTER and node(frame, destination)["selected"] is True
            and same_box(geometry(frame, destination)["bounds"], g["bounds"]),
            "Selected destination lost its full count or owner clearance")
    report["navigation"], rows = capture(app, folder, "navigation", viewport)
    report["navigation"]["badge"] = painted(rows, g, fill, ink)
    report["navigation"]["destination_icon"] = vector_ink(frame, destination, rows, compact)
    app.wire("Action.activate", {"id": "badge-nav-2"}, expected_error="disabled")

    focus_by_tab(app, "badge-scroll-owner")
    frame = app.dom()
    scroll = index(frame, "badge-scroll")
    g = badge(frame, scroll, fill, "999+")
    require(same_box(geometry(frame, scroll)["bounds"], g["bounds"]),
            f"Badge is outside the scrolling viewport: {g}")
    report["scroll-clearance"], rows = capture(app, folder, "scroll-clearance", viewport)
    report["scroll-clearance"]["badge"] = painted(rows, g, fill, ink)
    report["scroll-clearance"]["anchor_glyph"] = glyph_ink(frame, index(frame, "badge-scroll-owner"), rows)
    report["anchor"] = anchor
    return report


def run(options):
    binaries = {"gallery": options.gallery, "cli": options.cli}
    if "dark-rtl-200-compact" in options.profiles:
        require(options.profile_launcher, "Compact profile needs the exact Gallery launcher")
        binaries["profile_launcher"] = options.profile_launcher
    require(all(path.is_file() for path in binaries.values()), "Candidate binary is missing")
    options.out.mkdir(mode=0o700, parents=True, exist_ok=False)
    receipt = {
        "source": options.source, "completed": False,
        "macos": subprocess.check_output(["sw_vers", "-productVersion"], text=True).strip(),
        "binary_sha256": {key: digest(path) for key, path in binaries.items()},
        "input": "Surface semantic protocol",
        "profiles": {},
    }
    output = options.out / "report.json"
    for name, (width, height, dark, compact) in PROFILES.items():
        if name not in options.profiles:
            continue
        print(f"Running {name}", flush=True)
        folder = options.out / name
        folder.mkdir()
        with launch(options, name, width, height, compact) as (app, trace):
            app.activate("section-3")
            if dark:
                app.activate("theme")
            if compact:
                app.activate("badge-direction")
                app.activate("badge-scale")
            receipt["profiles"][name] = verify(options, app, trace, folder, compact)
        output.write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + "\n")
    if {"light-wide", "dark-wide"} <= receipt["profiles"].keys():
        require(receipt["profiles"]["light-wide"]["error"] != receipt["profiles"]["dark-wide"]["error"],
                "Light and dark badge colors did not differ")
    receipt["completed"] = True
    output.write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + "\n")
    print(f"Wrote {output}", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gallery", type=lambda s: Path(s).resolve(), required=True)
    parser.add_argument("--cli", type=lambda s: Path(s).resolve(), required=True)
    parser.add_argument("--profile-launcher", type=lambda s: Path(s).resolve())
    parser.add_argument("--ax-reader", type=lambda s: Path(s).resolve())
    parser.add_argument("--source", required=True)
    parser.add_argument("--out", type=lambda s: Path(s).resolve(), required=True)
    parser.add_argument("--profiles", choices=PROFILES, nargs="+", default=list(PROFILES))
    run(parser.parse_args())
