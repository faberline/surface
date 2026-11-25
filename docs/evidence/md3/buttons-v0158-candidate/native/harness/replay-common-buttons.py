#!/usr/bin/env python3
"""Calibrate exact Surface Common Buttons with native headless transports."""


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

PREFIX = "button-demo-"
VIEWPORTS = {"wide": (960, 1160), "compact": (320, 880)}
CAPTURES = (
    "wide-light-rest", "wide-light-selected-focus", "wide-light-square-toggle",
    "compact-dark-rtl-200-sizes", "compact-dark-rtl-200-wrapped-focus",
)
SIZES = {
    "size-xs": ("極小", 32, 14, 20, 20, 4), "size-s": ("小型", 40, 14, 20, 20, 8),
    "size-m": ("中型", 56, 16, 24, 24, 8), "size-l": ("大型", 96, 24, 32, 32, 12),
    "size-xl": ("特大", 136, 32, 40, 40, 16),
}
COMMANDS = {
    "command-elevated": "建立", "command-filled": "儲存", "command-tonal": "分享",
    "command-outlined": "詳細資料", "command-text": "略過",
}
TOGGLES = {
    "toggle-elevated": "提醒", "toggle-filled": "收藏",
    "toggle-tonal": "追蹤", "toggle-outlined": "固定",
}
DETAILS = {
    "leading": "新增相片", "trailing": "加入清單", "disabled": "停用操作",
    "disabled-toggle": "停用切換", "wrapped": "下載播放清單以便離線收聽",
}
NAMES = {**{name: size[0] for name, size in SIZES.items()}, **COMMANDS, **TOGGLES, **DETAILS}
ART = {
    **{name: (size[4], size[5], True) for name, size in SIZES.items()},
    "leading": (20, 4, True), "trailing": (20, 8, False),
    "disabled": (20, 8, True), "wrapped": (24, 8, True),
}
DISABLED = ("disabled", "disabled-toggle")
TAB_ORDER = ("theme", "rtl", "large-text", *SIZES, *COMMANDS, *TOGGLES, "leading", "trailing", "wrapped")
BINARIES = {"surface-material-gallery": "gallery", "surface-devtools": "cli"}
HEX = re.compile(r"[0-9a-f]{64}")


def close(actual, expected, message):
    require(isfinite(actual) and abs(actual - expected) <= .05, f"{message}: {actual}, expected {expected}")


def edge(rect, axis):
    return rect[axis] + rect["width" if axis == "x" else "height"]


def center(rect, axis):
    return (rect[axis] + edge(rect, axis)) / 2


def rounded_radius(authored, rect):
    corners = authored["style"]["corners"]
    require(len(set(corners.values())) == 1, "The fixture requires uniform corners")
    raw = corners["top_left"]
    require(isfinite(raw) and raw >= 0, "Invalid authored corner")
    return min(raw, rect["width"] / 2, rect["height"] / 2)


def separation(hit, face, ring):
    for axis, dimension in (("x", "width"), ("y", "height")):
        close(hit[dimension], max(face[dimension], 48), f"Target {dimension} minimum")
        close(center(hit, axis), center(face, axis), f"Face center along {axis}")
        close(ring[axis], face[axis] - 5, f"Focus outset along {axis}")
        close(ring[dimension], face[dimension] + 10, f"Focus extent {dimension}")


def state(frame, expected, recent=None):
    rows = {}
    for name, label in NAMES.items():
        target = PREFIX + name
        action = node(frame, target)
        value = expected.get(name)
        require(action["name"] == label and action["role"] == "Button"
                and action["checked"] is value and action["disabled"] is (name in DISABLED),
                f"Wrong named action or state for {target}")
        require(sum(item.get("stable_id") == target for item in frame["dom"]["nodes"]) == 1,
                f"Duplicate stable action ID for {target}")
        rows[name] = {"name": label, "checked": value, "disabled": action["disabled"]}
    status = node(frame, PREFIX + "status")["text"]
    require(status == ("尚未操作" if recent is None else f"最近操作：{NAMES[recent]}"),
            f"Unexpected Gallery status: {status}")
    return {"actions": rows, "status": status}


def passive(*nodes):
    return all(not item["name"] and item.get("tab_index") is None for item in nodes)


def geometry(frame, name, rtl, checked=False, fully_visible=False):
    target = PREFIX + name
    action_index, action = find(frame, target)
    face_index, face_node = find(frame, target + "-face")
    label_node, focus_node = node(frame, target + "-label"), node(frame, target + "-focus")
    require(face_node["role"] == focus_node["role"] == "Group"
            and face_node["parent"] == focus_node["parent"] == action_index and passive(face_node, focus_node)
            and label_node["role"] == "Text" and label_node["parent"] == face_index
            and label_node["text"] == action["name"] == NAMES[name], f"Wrong paint ownership for {target}")
    parts = ("face", "label", "focus")
    boxes = {suffix: layout(frame, f"{target}-{suffix}")["bounds"] for suffix in parts}
    hit = layout(frame, target)["bounds"]
    face, ring, label = boxes["face"], boxes["focus"], boxes["label"]
    separation(hit, face, ring)
    radius, ring_radius = rounded_radius(face_node, face), rounded_radius(focus_node, ring)
    square = (name in ("toggle-tonal", "toggle-outlined")) != checked
    if square:
        close(radius, 12, f"{target} square face")
        close(ring_radius, 17, f"{target} square ring")
    else:
        close(radius, min(face["height"], face["width"]) / 2, f"{target} rounded face")
        close(ring_radius, min(ring["height"], ring["width"]) / 2, f"{target} rounded ring")
    require(label["width"] > 0 and label["height"] > 0
            and face["x"] - .05 <= label["x"] and edge(label, "x") <= edge(face, "x") + .05
            and face["y"] - .05 <= label["y"] and edge(label, "y") <= edge(face, "y") + .05,
            f"Label lies outside its own face: {target}")
    result = {**boxes, "hit": hit, "face_radius": radius, "ring_radius": ring_radius,
              "font_size": label_node["style"]["font_size"], "line_height": label_node["style"]["line_height"]}
    if name in ART:
        size, gap, leading = ART[name]
        icon_node = node(frame, target + "-icon")
        icon = layout(frame, target + "-icon")["bounds"]
        require(icon_node["role"] == "Group" and icon_node["parent"] == face_index
                and passive(icon_node) and icon_node["fill_contour"]["color"] == label_node["style"]["foreground"],
                f"Wrong passive artwork for {target}")
        close(icon["width"], size, f"{target} icon width")
        close(icon["height"], size, f"{target} icon height")
        left = leading != rtl
        measured_gap = label["x"] - edge(icon, "x") if left else icon["x"] - edge(label, "x")
        close(measured_gap, gap, f"{target} directional art gap")
        close(center(icon, "y"), center(label, "y"), f"{target} content center")
        result.update({"icon": icon, "icon_side": "left" if left else "right"})
        parts += ("icon",)
    if fully_visible:
        visible(frame, target)
        for item in parts:
            visible(frame, f"{target}-{item}")
    return result


def leaf_size(frame, name, rtl):
    result = geometry(frame, name, rtl)
    _, painted_height, font, line, _, _ = SIZES[name]
    scale = 2 if rtl else 1
    close(result["font_size"], font * scale, f"{name} text scale")
    close(result["line_height"], line * scale, f"{name} line height")
    close(result["label"]["height"], line * scale, f"{name} measured text height")
    close(result["face"]["height"], painted_height + (line if rtl else 0), f"{name} painted height")
    return result


def rgba(rows, x, y):
    require(0 <= y < len(rows) and 0 <= x < len(rows[y]) // 4, f"Pixel is outside PNG: {(x, y)}")
    return list(rows[y][4 * x:4 * x + 4])


def box_points(rect):
    return ((x, y) for y in range(ceil(rect["y"]), floor(edge(rect, "y")))
            for x in range(ceil(rect["x"]), floor(edge(rect, "x"))))


def count_color(rows, rect, color):
    return sum(rgba(rows, *point) == color for point in box_points(rect))


def near(left, right, tolerance=2):
    return max(abs(a - b) for a, b in zip(left, right)) <= tolerance


def sample_paint(frame, rows, name, boxes, background, focused):
    target = PREFIX + name
    face_node = node(frame, target + "-face")
    label_node = node(frame, target + "-label")
    focus_node = node(frame, target + "-focus")
    fill = face_node["style"]["background"]
    face, ring = boxes["face"], boxes["focus"]
    result = {"fill_token": fill}
    at = (floor(face["x"] + min(8, face["width"] / 4)), floor(center(face, "y")))
    painted = rgba(rows, *at)
    if fill[3] == 255:
        if focused:
            # The focused face composites its low-opacity state layer over the fill.
            require(not near(painted, background) and near(painted, fill, 52),
                    f"Wrong focused face color for {target}: {painted}, fill {fill}")
        else:
            require(near(painted, fill), f"Wrong flat face color for {target}: {painted}, expected {fill}")
        result["body"] = {"at": at, "rgba": painted}
    elif name in DISABLED:
        require(fill[3] == 26 and painted[3] == 255 and not near(painted, background),
                f"Missing disabled face fill for {target}")
        result["body"] = {"at": at, "rgba": painted}
    elif face_node["style"]["border_width"]:
        ink = face_node["style"]["border_color"]
        y = face["y"]
        x = floor(center(face, "x"))
        samples = [{"at": [cx, cy], "rgba": rgba(rows, cx, cy)}
                   for cy in range(floor(y), ceil(y) + 2) for cx in range(x - 2, x + 3)]
        # A one-dp stroke can straddle two raster rows at a fractional origin.
        require(near(painted, background) and any(not near(sample["rgba"], background) for sample in samples),
                f"Outlined command lost its visible stroke or transparent face: {target}")
        result["outline"] = {"ink": ink, "samples": samples}
    else:
        require(near(painted, background), f"Text command has an unexpected fill: {target}")
    if name not in DISABLED:
        result["label_ink"] = ink_sample(frame, rows, target + "-label")
    if name in ART and name not in DISABLED:
        icon = boxes["icon"]
        mark = (floor(center(icon, "x")), floor(center(icon, "y")))
        ink = label_node["style"]["foreground"]
        pixels = count_color(rows, icon, ink)
        require(near(rgba(rows, *mark), ink) and pixels >= 20, f"Missing vector artwork for {target}")
        result["icon_ink"] = {"at": mark, "rgba": rgba(rows, *mark), "pixels": pixels}
    if name in SIZES and boxes["hit"]["height"] > face["height"]:
        gap = (floor(center(face, "x")), ceil(boxes["hit"]["y"]) + 1)
        require(near(rgba(rows, *gap), background), f"Short target padding paints for {target}")
        result["unpainted_hit_gap"] = {"at": gap, "rgba": rgba(rows, *gap)}
    if focused:
        ink = focus_node["focus_color"]
        top = {"x": center(ring, "x") - 10, "y": ring["y"], "width": 20, "height": 4}
        count = count_color(rows, top, ink)
        require(count >= 20, f"Native focus outline did not paint outside the face: {target}")
        result["ring_ink"] = {"bounds": top, "rgba": ink, "pixels": count}
    if name in ("toggle-filled", "toggle-tonal"):
        corner = {"x": ceil(face["x"] + 5), "y": ceil(face["y"] + 3), "width": 3, "height": 2}
        count = sum(near(rgba(rows, *point), painted) for point in box_points(corner))
        if boxes["face_radius"] == 12:
            require(count >= 4, f"Square toggle contour is absent: {target}, {count} interior pixels")
        else:
            require(count <= 2, f"Round toggle contour is absent: {target}, {count} interior pixels")
        result["corner"] = {"bounds": corner, "body_color_pixels": count}
    return result


def photograph(app, plan, title, frame, specimens, checked, rtl, focused=None):
    shot = capture(app, plan, title)
    require(settle(app) == frame, "Frame changed across the three transports")
    width, height, rows = read_png((plan.out / shot["name"]).read_bytes())
    require((width, height) == VIEWPORTS["compact" if rtl else "wide"], "Wrong native PNG size")
    background = node(frame, PREFIX + "route")["style"]["background"]
    require(rgba(rows, 4, 4) == background and (max(background[:3]) <= 96 if rtl else min(background[:3]) >= 200),
            "Wrong visible native theme")
    tested = {}
    for name in specimens:
        boxes = geometry(frame, name, rtl, checked.get(name, False), fully_visible=True)
        tested[name] = {"boxes": boxes, "paint": sample_paint(frame, rows, name, boxes, background, name == focused)}
    return {**shot, "background": background, "samples": tested}


def window_sample(app, checks):
    count = onscreen_windows(app.proc.pid)
    checks.append(count)
    require(count == 0, "A window from the owned headless Gallery became visible")


def next_focus(app, item, *, reverse=False):
    frame = key(app, "tab", reverse=reverse)
    focus(frame, PREFIX + item)
    return frame


def accepted(app, item, button, checked, toggle=None):
    frame = key(app, button)
    focus(frame, PREFIX + item)
    if toggle is not None:
        checked[item] = toggle
    return frame, {"input": button, "id": PREFIX + item, **state(frame, checked, item)}


def replay_profile(plan, profile):
    width, height = VIEWPORTS[profile]
    rtl = profile == "compact"
    windows = []
    checked = {**dict.fromkeys(TOGGLES, False), "disabled-toggle": True}
    captures, actions = {}, []
    with launch(plan, profile, width, height) as app:
        window_sample(app, windows)
        opening = focus_by_tab(app, PREFIX + "show")
        frame = key(app, "enter")
        focus(frame, PREFIX + "back")
        initial = state(frame, checked)
        for item in TAB_ORDER[:3]:
            frame = next_focus(app, item)
            if rtl:
                frame = key(app, "space")
                focus(frame, PREFIX + item)
        start = frame
        window_sample(app, windows)
        size_boxes = {name: leaf_size(frame, name, rtl) for name in SIZES}
        xs, small = size_boxes["size-xs"]["hit"], size_boxes["size-s"]["hit"]
        require(edge(small, "x") <= xs["x"] if rtl else edge(xs, "x") <= small["x"],
                "The first visible action row did not mirror")
        if rtl:
            captures["sizes"] = photograph(app, plan, "compact-dark-rtl-200-sizes", frame,
                                           ("size-xs", "size-s"), checked, rtl)
        else:
            captures["rest"] = photograph(app, plan, "wide-light-rest", frame, NAMES, checked, rtl)
        art = {}
        for item in TAB_ORDER[3:]:
            frame = next_focus(app, item)
            if item in ("command-filled", "command-text"):
                frame, proof = accepted(app, item, "enter" if item == "command-filled" else "space", checked)
                actions.append(proof)
            elif item == "toggle-filled":
                before = geometry(frame, item, rtl, fully_visible=True)
                frame, proof = accepted(app, item, "space", checked, True)
                actions.append(proof)
                after = geometry(frame, item, rtl, True, fully_visible=True)
                require(all(before[piece] == after[piece] for piece in ("hit", "face", "focus", "label")),
                        "The controlled toggle moved its own footprint")
                if not rtl:
                    captures["selected"] = photograph(app, plan, "wide-light-selected-focus", frame,
                                                       (item,), checked, rtl, item)
                frame, proof = accepted(app, item, "enter", checked, False)
                actions.append(proof)
            elif item == "toggle-tonal":
                frame, proof = accepted(app, item, "enter", checked, True)
                actions.append(proof)
                if not rtl:
                    captures["square_toggle"] = photograph(app, plan, "wide-light-square-toggle", frame,
                                                           (item,), checked, rtl, item)
            elif item in ("leading", "trailing"):
                art[item] = geometry(frame, item, rtl, fully_visible=True)
        reverse_frame = next_focus(app, "trailing", reverse=True)
        state(reverse_frame, checked, "toggle-tonal")
        frame = next_focus(app, "wrapped")
        for item in DISABLED:
            app.wire("Action.activate", {"id": PREFIX + item}, "disabled")
            require(settle(app) == frame, f"Disabled activation mutated the frame: {item}")
        wrapped = geometry(frame, "wrapped", rtl, fully_visible=True)
        if rtl:
            require(wrapped["label"]["height"] >= 2 * wrapped["line_height"] and wrapped["hit"]["width"] <= 276.05
                    and layout(frame, PREFIX + "sizes")["bounds"]["y"] < layout(start, PREFIX + "sizes")["bounds"]["y"],
                    "Compact long label did not wrap and scroll into view")
            captures["wrapped"] = photograph(app, plan, "compact-dark-rtl-200-wrapped-focus",
                                              frame, ("wrapped",), checked, rtl, "wrapped")
        window_sample(app, windows)
        closed = activate(app, PREFIX + "back")
        focus(closed, PREFIX + "show")
        require(find(closed, PREFIX + "scroll")[1] is None, "The retired route remains mounted")
        window_sample(app, windows)
    require(app.proc.returncode is not None, "The Gallery child did not exit")
    typed = {"viewport": [width, height], "rtl": rtl, "text_scale": 2 if rtl else 1, "opening_tabs": opening,
            "tab_order": [PREFIX + item for item in TAB_ORDER], "initial": initial, "accepted": actions,
            "captures": captures, "sizes": size_boxes, "artwork": art, "wrapped": wrapped,
            "disabled_rejections": [PREFIX + item for item in DISABLED], "closed_focus": PREFIX + "show",
            "visible_windows": windows, "gallery_reaped": True}
    process = {"pid": app.proc.pid, "exit_code": app.proc.returncode, "reaped": True, "visible_windows": windows}
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
    required = {"Cargo.toml", "Cargo.lock", "crates/surface-material/src/button.rs",
                "crates/surface-material/tests/material_button.rs", "examples/material-gallery/src/button_demo.rs",
                "examples/material-gallery/src/lib.rs"}
    require(required <= entries.keys(), "Source seal omits Common Buttons or its Gallery route")
    # Check both inventory and bytes so a stale snapshot cannot omit a new test or fixture path.
    paths = {path.relative_to(root).as_posix() for path in root.rglob("*")
             if path.is_file() and path.relative_to(root).parts[0] != ".git"}
    require(paths == set(entries), "Source file inventory differs from the reviewed manifest")
    for name, checksum in entries.items():
        path = root / name
        require(path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root)
                and digest(path) == checksum, f"Wrong source bytes for {name}")
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == "0.1.58", f"Unexpected source Cargo version {version}")
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
    face = {"x": 10, "y": 28, "width": 48, "height": 32}
    ring = {"x": 5, "y": 23, "width": 58, "height": 42}
    separation(hit, face, ring)
    layout_controls = (
        ({**hit, "height": 32}, face, ring),
        (hit, {**face, "x": 12}, ring),
        (hit, face, {**ring, "width": 50}),
    )
    for arguments in layout_controls:
        try:
            separation(*arguments)
        except RuntimeError:
            continue
        raise RuntimeError("Target separation accepted an invalid fixture")
    rect = {"x": 1, "y": 1, "width": 2, "height": 2}
    ink = [46, 72, 118, 255]
    paper = [240, 242, 246, 255]
    rows = [bytes(4 * paper), bytes(paper + ink + ink + paper),
            bytes(paper + ink + paper + paper), bytes(4 * paper)]
    require(count_color(rows, rect, ink) == 3 and count_color(rows, rect, paper) == 1,
            "Pixel decoder control differs")
    print(json.dumps({"synthetic_geometry_positive": 1, "synthetic_geometry_rejections": 3,
                      "synthetic_pixel_positive": 1, "application_started": False}, indent=2))


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
    report = {"schema": "surface.test.common-buttons.native.v1", "source": sources,
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
    require(windows == 16 and len({item["pid"] for item in children}) == 4
            and all(item["visible_windows"] == [0] * 4 and item["reaped"] for item in children),
            "Owned process or zero-window observation inventory differs")
    report.update({"rendering": "headless", "reduce_motion": True,
                   "transports": ["Page.screenshot", "CLI", "MCP"],
                   "input_scope": ["Action.key", "Action.activate", "DOM.snapshot"],
                   "window_samples": windows, "owned_processes_reaped": len(children),
                   "replays": profiles, "owned_process_observations": processes,
                   "byte_identical_pngs": images})
    (out / "native-common-buttons-report.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    files = sorted(path for path in out.rglob("*") if path.is_file())
    manifest = out / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in files))
    print(json.dumps({"manifest_sha256": digest(manifest), "images_per_replay": len(images),
                      "window_samples": windows, "visible_windows": 0,
                      "owned_processes_reaped": len(children), "pngs": images}, indent=2))


if __name__ == "__main__":
    main()
