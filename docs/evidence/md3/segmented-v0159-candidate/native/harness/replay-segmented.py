#!/usr/bin/env python3
"""Verify Baseline Segmented Button on the pinned headless native Gallery."""

import argparse
import json
from math import ceil, floor, isfinite
import os
from pathlib import Path
import re
from types import SimpleNamespace
import tomllib

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, find, focus, ink_sample, key, layout, node, settle, visible
from window_check import onscreen_windows

ROUTE = "segmented-demo-route"
SHOW = "segmented-demo-show"
BACK = "segmented-demo-back"
AFTER = "segmented-demo-after"
STATUS = "segmented-demo-status"
SETTINGS = ("segmented-demo-theme", "segmented-demo-rtl", "segmented-demo-large-text")
COMPARE, VIEW, FILTERS = "segments-comparison", "segments-view", "segments-filters"
LEFT, RIGHT = "segment-left-pane", "segment-right-pane"
LIST, TABLE, GRID = "segment-list-view", "segment-table-view", "segment-grid-view"
NEW, SAVED, LOCAL, SHARED, ARCHIVED = (
    "segment-new", "segment-saved", "segment-local", "segment-shared", "segment-archived",
)
GROUPS = {
    COMPARE: ("RadioGroup", "對照位置", "Radio", {LEFT: "左側", RIGHT: "右側"}),
    VIEW: ("RadioGroup", "檢視方式", "Radio", {LIST: "清單檢視", TABLE: "表格檢視", GRID: "格狀檢視"}),
    FILTERS: ("Group", "篩選條件", "Checkbox",
              {NEW: "最新", SAVED: "已儲存", LOCAL: "離線", SHARED: "已分享", ARCHIVED: "封存"}),
}
DISABLED = (TABLE, LOCAL)
VIEWPORTS = {"wide": (960, 880), "compact": (320, 880), "scrolled": (240, 880)}
CAPTURES = ("wide-light-rest", "wide-light-selected-keyboard", "compact-dark-rtl-200",
            "scrolled-dark-rtl-200")
BINARIES = {"surface-material-gallery": "gallery", "surface-devtools": "cli"}
HEX = re.compile(r"[0-9a-f]{64}")


def close(actual, expected, message):
    require(isfinite(actual) and abs(actual - expected) <= 0.06,
            f"{message}: {actual}, expected {expected}")


def edge(rect, axis):
    return rect[axis] + rect["width" if axis == "x" else "height"]


def center(rect, axis):
    return (rect[axis] + edge(rect, axis)) / 2


def check_dimensions(hit, face, ring):
    require(all(isfinite(value) for rect in (hit, face, ring) for value in rect.values()),
            "Nonfinite segment rectangle")
    require(hit["width"] >= 47.99 and hit["height"] >= 47.99, "Segment target is below 48 dp")
    for axis, dimension in (("x", "width"), ("y", "height")):
        close(center(hit, axis), center(face, axis), f"Face center along {axis}")
        close(ring[axis], face[axis] - 5, f"Focus outset along {axis}")
        close(ring[dimension], face[dimension] + 10, f"Focus extent {dimension}")
    close(hit["width"], face["width"], "Hit and face widths")
    close(hit["height"], face["height"] + 8, "Transparent vertical tap allowance")


def authored_selection(frame, comparison=RIGHT, view=GRID, filters=()):
    checked = {comparison, view, *filters}
    result = {}
    for group, (role, group_name, item_role, items) in GROUPS.items():
        group_index, authored = find(frame, group)
        require(authored is not None and (authored["role"], authored["name"]) == (role, group_name),
                f"Wrong named group {group}")
        track_index, track = find(frame, f"{group}-track")
        require(track is not None and track["parent"] == group_index, f"Missing track for {group}")
        result[group] = []
        for target, name in items.items():
            index, item = find(frame, target)
            require(item is not None and item["role"] == item_role and item["name"] == name and
                    item["checked"] is (target in checked) and item["disabled"] is (target in DISABLED) and
                    item["parent"] == track_index and item.get("selected") is None and not item.get("mixed"),
                    f"Wrong stable {item_role} state, name or ownership for {target}")
            require(sum(x.get("stable_id") == target for x in frame["dom"]["nodes"]) == 1,
                    f"Repeated stable action {target}")
            for suffix in ("face", "focus"):
                child = node(frame, f"{target}-{suffix}")
                require(child["parent"] == index and child["role"] == "Group" and not child["name"] and
                        child.get("tab_index") is None, f"{target}-{suffix} is not passive")
            if target == GRID:
                require(find(frame, f"{target}-label")[1] is None, "Icon-only radio has a painted label")
                require(node(frame, f"{target}-icon").get("fill_contour"), "Missing icon-only radio artwork")
                require((find(frame, f"{target}-check")[1] is not None) is (target in checked),
                        "Icon-only radio check does not match authored selection")
            result[group].append({"id": target, "role": item_role, "name": name,
                                  "checked": target in checked, "disabled": target in DISABLED})
    status = node(frame, STATUS)
    filters_status = f"已選取 {len(filters)} 項篩選條件" if filters else "未選取篩選條件"
    require(status["role"] == "Status" and status["text"] == filters_status,
            "Selection status disagrees with authored filters")
    return {"groups": result, "status": filters_status}


def geometry(frame, target, *, fully_visible=False):
    suffixes = ("face", "focus")
    result = {suffix: layout(frame, f"{target}-{suffix}")["bounds"] for suffix in suffixes}
    result["hit"] = layout(frame, target)["bounds"]
    check_dimensions(result["hit"], result["face"], result["focus"])
    if fully_visible:
        visible(frame, target, 47.99)
        for suffix in suffixes:
            visible(frame, f"{target}-{suffix}")
    return result


def aligned_rows(frame, rtl, *, wide=False, all_visible=True):
    proof = {}
    for group, (_, _, _, items) in GROUPS.items():
        layouts = {target: geometry(frame, target, fully_visible=all_visible) for target in items}
        boxes = list(layouts.values())
        first = boxes[0]["face"]
        for n, parts in enumerate(boxes):
            face = parts["face"]
            close(face["width"], first["width"], f"Equal width for {group}")
            close(face["height"], first["height"], f"Equal height for {group}")
            close(face["y"], first["y"], f"One physical row for {group}")
            if n:
                previous = boxes[n - 1]["face"]
                close(edge(face, "x") if rtl else edge(previous, "x"),
                      previous["x"] if rtl else face["x"], f"Adjacent cells for {group}")
        if wide:
            close(first["height"], 40, f"Baseline face height for {group}")
        proof[group] = layouts
    return proof


def count_ink(rows, rect, color):
    rgb = bytes(color)
    require(len(rgb) == 4 and rgb[3] == 255, "Expected opaque token")
    left, right = max(0, ceil(rect["x"])), min(len(rows[0]) // 4, floor(edge(rect, "x")))
    top, bottom = max(0, ceil(rect["y"])), min(len(rows), floor(edge(rect, "y")))
    return sum(rows[y][4*x:4*x+4] == rgb for y in range(top, bottom) for x in range(left, right))


def ring_sides(face, ring):
    return {
        "top": {"x": center(ring, "x") - 6, "y": ring["y"], "width": 12,
                "height": face["y"] - ring["y"]},
        "bottom": {"x": center(ring, "x") - 6, "y": edge(face, "y"), "width": 12,
                   "height": edge(ring, "y") - edge(face, "y")},
        "left": {"x": ring["x"], "y": center(ring, "y") - 6,
                 "width": face["x"] - ring["x"], "height": 12},
        "right": {"x": edge(face, "x"), "y": center(ring, "y") - 6,
                  "width": edge(ring, "x") - edge(face, "x"), "height": 12},
    }


def focus_ink(frame, rows, target):
    parts = geometry(frame, target, fully_visible=True)
    ring = node(frame, f"{target}-focus")
    token = ring["focus_color"]
    require(ring.get("focus_outline_from_parent") is True and ring.get("focus_outline_width") == 3,
            f"Wrong authored 3 dp focus outline on {target}")
    pixels = {}
    for side, probe in ring_sides(parts["face"], parts["focus"]).items():
        count = count_ink(rows, probe, token)
        require(count >= 8, f"Missing painted focus {side} on {target}: {count} pixels")
        pixels[side] = {"bounds": probe, "pixels": count}
    return {"parts": parts, "rgba": token, "edges": pixels}


def photograph(app, plan, profile, name, frame, *, focused=None, labels=(), selected=()):
    shot = capture(app, plan, name)
    require(settle(app) == frame, "The same settled frame must serve all three capture transports")
    width, height, rows = read_png((plan.out / shot["name"]).read_bytes())
    require((width, height) == VIEWPORTS[profile], "Wrong native capture viewport")
    theme = node(frame, ROUTE)["style"]["background"]
    require(list(rows[4][16:20]) == theme and
            (min(theme[:3]) >= 200 if profile == "wide" else max(theme[:3]) <= 96),
            "Screenshot does not contain the expected visible theme")
    shot.update({"background": theme,
                 "label_ink": {target: ink_sample(frame, rows, f"{target}-label") for target in labels}})
    if focused:
        shot["focus"] = focus_ink(frame, rows, focused)
    shot["selected_ink"] = {}
    for target in selected:
        face = geometry(frame, target, fully_visible=True)["face"]
        token = node(frame, f"{target}-face")["style"]["background"]
        selection = count_ink(rows, face, token)
        require(selection >= 30, f"Missing selected fill on {target}")
        shot["selected_ink"][target] = {"rgba": token, "pixels": selection}
    return shot


def observe_window(app, checks):
    count = onscreen_windows(app.proc.pid)
    checks.append(count)
    require(count == 0, "A window from the owned headless Gallery became visible")


def move(app, command, target, *, reverse=False):
    frame = key(app, command, reverse=reverse)
    focus(frame, target)
    return frame

def wide_profile(app, plan, frame):
    initial = authored_selection(frame)
    rows = aligned_rows(frame, False, wide=True)
    rest = photograph(app, plan, "wide", CAPTURES[0], frame,
                      labels=(LEFT, RIGHT, LIST, NEW), selected=(RIGHT, GRID))
    focus_steps = []

    def expect_key(command, target, **selection):
        result = move(app, command, target)
        focus_steps.append({"key": command, "id": target, **authored_selection(result, **selection)})
        return result

    for target in SETTINGS:
        expect_key("tab", target)
    expect_key("tab", LEFT)
    expect_key("right", RIGHT)
    expect_key("enter", RIGHT)
    expect_key("tab", LIST)
    expect_key("right", GRID)
    expect_key("left", LIST)
    expect_key("space", LIST, view=LIST)
    expect_key("enter", LIST, view=LIST)
    expect_key("tab", NEW, view=LIST)
    expect_key("space", NEW, view=LIST, filters=(NEW,))
    expect_key("right", SAVED, view=LIST, filters=(NEW,))
    expect_key("right", SHARED, view=LIST, filters=(NEW,))
    selected = expect_key("enter", SHARED, view=LIST, filters=(NEW, SHARED))
    keyboard = photograph(app, plan, "wide", CAPTURES[1], selected, focused=SHARED,
                          labels=(LIST, NEW, SHARED), selected=(LIST, NEW))
    expect_key("space", SHARED, view=LIST, filters=(NEW,))
    expect_key("left", SAVED, view=LIST, filters=(NEW,))
    expect_key("left", NEW, view=LIST, filters=(NEW,))
    expect_key("enter", NEW, view=LIST)
    frame = expect_key("tab", AFTER, view=LIST)
    return frame, {"initial": initial, "geometry": rows, "focus_steps": focus_steps,
                   "captures": [rest, keyboard]}


def compact_profile(app, plan, profile, frame):
    for setting in SETTINGS:
        move(app, "tab", setting)
        frame = move(app, "space", setting)
        require(node(frame, setting)["selected"] is True, f"Preset did not turn on: {setting}")
    for target in (LEFT, LIST, NEW):
        frame = move(app, "tab", target)
        authored_selection(frame)
    start = layout(frame, f"{FILTERS}-track")["bounds"]
    viewport = layout(frame, FILTERS)["bounds"]
    first = geometry(frame, NEW, fully_visible=True)
    last = geometry(frame, ARCHIVED)
    require(first["hit"]["x"] > last["hit"]["x"], "Logical first must paint on the right in RTL")
    text = node(frame, f"{NEW}-label")["style"]
    close(text["font_size"], 28, "Compact label text at 200%")
    require(first["face"]["height"] > 40, "Large label must grow the baseline face")
    aligned = aligned_rows(frame, True, all_visible=profile == "compact")
    if profile == "compact":
        require(start["width"] <= viewport["width"], "320 dp profile must fit the full filter row")
        shot = photograph(app, plan, profile, CAPTURES[2], frame, focused=NEW,
                          labels=(NEW, ARCHIVED))
        return frame, {"geometry": aligned, "focus_steps": [LEFT, LIST, NEW], "track": start,
                       "selection": authored_selection(frame), "captures": [shot]}

    require(start["width"] > viewport["width"], "240 dp profile must require horizontal scroll")
    travel = []
    for target in (SAVED, SHARED, ARCHIVED):
        frame = move(app, "left", target)
        travel.append({"id": target, "parts": geometry(frame, target, fully_visible=True),
                       "track": layout(frame, f"{FILTERS}-track")["bounds"]})
        authored_selection(frame)
    stop = layout(frame, f"{FILTERS}-track")["bounds"]
    require(stop["x"] > start["x"], "Moving RTL focus must scroll to reveal the other edge")
    close(stop["width"], start["width"], "Scroll does not shrink hit targets")
    final = move(app, "space", ARCHIVED)
    selected = authored_selection(final, filters=(ARCHIVED,))
    shot = photograph(app, plan, profile, CAPTURES[3], final, focused=ARCHIVED, labels=(ARCHIVED,))
    return final, {"track_start": start, "track_end": stop, "viewport": viewport,
                   "first": first, "travel": travel, "selected": selected, "captures": [shot]}


def replay_profile(plan, profile):
    width, height = VIEWPORTS[profile]
    windows = []
    with launch(plan, profile, width, height) as app:
        observe_window(app, windows)
        opening = focus_by_tab(app, SHOW)
        frame = move(app, "enter", BACK)
        authored_selection(frame)
        observe_window(app, windows)
        if profile == "wide":
            frame, checked = wide_profile(app, plan, frame)
        else:
            frame, checked = compact_profile(app, plan, profile, frame)
        for target in DISABLED:
            app.wire("Action.activate", {"id": target}, "disabled")
            require(settle(app) == frame, f"Disabled activation changed the native frame: {target}")
        observe_window(app, windows)
        closed = activate(app, BACK)
        focus(closed, SHOW)
        require(find(closed, ROUTE)[1] is None, "Closed Segmented route still mounted")
        observe_window(app, windows)
    require(app.proc.returncode is not None, "Gallery child was not reaped")
    typed = {"viewport": [width, height], "rtl": profile != "wide",
             "text_scale": 1 if profile == "wide" else 2,
             "opening_tabs": opening, "disabled_rejections": list(DISABLED),
             "closed_focus": SHOW, **checked}
    process = {"pid": app.proc.pid, "exit_code": app.proc.returncode,
               "visible_windows": windows, "reaped": True}
    return typed, process


def manifest_entries(path, expected):
    require(HEX.fullmatch(expected) and digest(path) == expected, "Reviewed manifest digest differs")
    entries = {}
    for line in path.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        relative = Path(name)
        require(HEX.fullmatch(checksum) and not relative.is_absolute() and ".." not in relative.parts
                and relative.as_posix() == name and name not in entries, "Invalid manifest path or entry")
        entries[name] = checksum
    require(entries, "Manifest is empty")
    return entries


def binary_pins(manifest, expected):
    entries = manifest_entries(manifest, expected)
    require(entries.keys() == BINARIES.keys(), "Manifest must bind exactly the native Gallery and CLI")
    paths = {}
    for name, checksum in entries.items():
        path = manifest.parent / name
        require(path.is_file() and not path.is_symlink() and os.access(path, os.X_OK)
                and digest(path) == checksum, f"Wrong native executable bytes for {name}")
        paths[BINARIES[name]] = path
    return paths, entries


def source_pins(root, manifest, expected):
    entries = manifest_entries(manifest, expected)
    required = {"Cargo.toml", "Cargo.lock", "crates/surface-material/src/baseline_segmented_button.rs",
                "crates/surface-material/tests/baseline_segmented_button.rs",
                "examples/material-gallery/src/segmented_demo.rs", "examples/material-gallery/src/lib.rs"}
    require(required <= entries.keys(), "Source seal omits Baseline Segmented Button or its Gallery route")
    # Check both inventory and bytes so a stale snapshot cannot omit a new test or fixture path.
    paths = {path.relative_to(root).as_posix() for path in root.rglob("*")
             if path.is_file() and path.relative_to(root).parts[0] != ".git"}
    require(paths == set(entries), "Source file inventory differs from the reviewed manifest")
    for name, checksum in entries.items():
        path = root / name
        require(path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root)
                and digest(path) == checksum, f"Wrong source bytes for {name}")
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == "0.1.59", f"Unexpected source Cargo version {version}")
    return {"manifest_sha256": expected, "files_verified": len(entries), "version": version}


def replay(paths, out):
    out.mkdir(mode=0o700)
    plan = SimpleNamespace(**paths, out=out)
    completed = {name: replay_profile(plan, name) for name in VIEWPORTS}
    return ({name: result[0] for name, result in completed.items()},
            {name: result[1] for name, result in completed.items()})


def compare_images(first, second):
    originals = {f"{name}.png" for name in CAPTURES}
    require(all({path.name for path in directory.glob("*.png")} == originals for directory in (first, second)),
            "A recurrence lacks an original PNG")
    matched = {}
    for name in sorted(originals):
        left, right = first / name, second / name
        require(left.read_bytes() == right.read_bytes(), f"Replays differ in {name}")
        matched[name] = digest(left)
    require(len(set(matched.values())) == len(originals), "Identical images were reused across different scenes")
    return matched


def self_test():
    hit = {"x": 10, "y": 20, "width": 48, "height": 48}
    face = {"x": 10, "y": 24, "width": 48, "height": 40}
    ring = {"x": 5, "y": 19, "width": 58, "height": 50}
    check_dimensions(hit, face, ring)
    layout_controls = (
        ({**hit, "width": 47}, face, ring),
        (hit, {**face, "x": 12}, ring),
        (hit, face, {**ring, "width": 50}),
    )
    for arguments in layout_controls:
        try:
            check_dimensions(*arguments)
        except RuntimeError:
            continue
        raise RuntimeError("Geometry accepted an invalid fixture")
    ink = [46, 72, 118, 255]
    paper = [240, 242, 246, 255]
    sides = ring_sides(face, ring)
    for missing in (None, *sides):
        rows = [bytearray(80 * bytes(paper)) for _ in range(80)]
        for side, probe in sides.items():
            if side == missing:
                continue
            for y in range(ceil(probe["y"]), floor(edge(probe, "y"))):
                for x in range(ceil(probe["x"]), floor(edge(probe, "x"))):
                    rows[y][4*x:4*x+4] = bytes(ink)
        counts = {side: count_ink(rows, probe, ink) for side, probe in sides.items()}
        require([side for side, count in counts.items() if count < 8] == ([missing] if missing else []),
                "Four-edge focus pixel control differs")
    print(json.dumps({"synthetic_geometry_positive": 1, "synthetic_geometry_rejections": 3,
                      "synthetic_focus_positive": 1, "synthetic_missing_focus_edges": 4,
                      "application_started": False}, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("binary-manifest", "source-root", "source-manifest"):
        parser.add_argument(f"--{name}", type=Path)
    parser.add_argument("--binary-manifest-sha256")
    parser.add_argument("--source-manifest-sha256")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--self-test", action="store_true")
    mode.add_argument("--check-source-only", action="store_true")
    mode.add_argument("--check-only", action="store_true")
    mode.add_argument("--out", type=Path, help="fresh private output directory inside /tmp")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if not all((args.source_root, args.source_manifest, args.source_manifest_sha256)):
        parser.error("Supply the reviewed source directory, manifest and digest.")
    source_root = args.source_root.resolve(strict=True)
    source_manifest = args.source_manifest.resolve(strict=True)
    sources = source_pins(source_root, source_manifest, args.source_manifest_sha256)
    if args.check_source_only:
        print(json.dumps({"status": "source-pins-verified", "source": sources}, indent=2))
        return
    if not all((args.binary_manifest, args.binary_manifest_sha256)):
        parser.error("Supply the reviewed native binary manifest and digest.")
    binary_manifest = args.binary_manifest.resolve(strict=True)
    paths, binaries = binary_pins(binary_manifest, args.binary_manifest_sha256)
    report = {"schema": "surface.test.segmented.native.v1", "source": sources,
              "native_binary_manifest_sha256": args.binary_manifest_sha256, "binaries": binaries}
    if args.check_only:
        print(json.dumps({"status": "pins-verified", **report}, indent=2))
        return

    out = args.out
    require(not out.exists() and not out.is_symlink()
            and out.parent.resolve(strict=True).is_relative_to(Path("/tmp").resolve()),
            "Use a fresh private output directory under /tmp")
    out.mkdir(mode=0o700)
    runs = [out / name for name in ("replay-1", "replay-2")]
    first, second = [replay(paths, path) for path in runs]
    require(len(VIEWPORTS) == 3 and len(CAPTURES) == 4, "Unexpected replay plan size")
    require(binary_pins(binary_manifest, args.binary_manifest_sha256)[1] == binaries,
            "Native binary bytes changed during the replays")
    require(source_pins(source_root, source_manifest, args.source_manifest_sha256) == sources,
            "Source bytes changed during the replays")
    images = compare_images(*runs)
    require(first[0] == second[0], "Typed or painted observations differ between recurrences")
    profiles = [first[0], second[0]]
    processes = [first[1], second[1]]
    children = [item for trial in processes for item in trial.values()]
    windows = sum(len(item["visible_windows"]) for item in children)
    require(windows == 24 and len({item["pid"] for item in children}) == 6
            and all(item["visible_windows"] == [0] * 4 and item["reaped"] for item in children),
            "Owned process or zero-window observation inventory differs")
    report.update({"rendering": "headless", "reduce_motion": True,
                   "transports": ["Page.screenshot", "CLI", "MCP"],
                   "input_scope": ["Action.key", "Action.activate", "DOM.snapshot"],
                   "window_samples": windows, "owned_processes_reaped": len(children),
                   "replays": profiles, "owned_process_observations": processes,
                   "byte_identical_pngs": images})
    (out / "native-segmented-report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    files = sorted(path for path in out.rglob("*") if path.is_file())
    manifest = out / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in files))
    print(json.dumps({"manifest_sha256": digest(manifest), "images_per_replay": len(images),
                      "window_samples": windows, "visible_windows": 0,
                      "owned_processes_reaped": len(children), "pngs": images}, indent=2))


if __name__ == "__main__":
    main()
