#!/usr/bin/env python3
"""Verify the Expressive vertical command menu in the native headless Gallery."""

import argparse
from contextlib import contextmanager
from dataclasses import dataclass
import json
import math
import os
from pathlib import Path
import re
import shlex
import sys
from tempfile import TemporaryDirectory
import tomllib
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

PREFIX = "menu-demo-"
SHOW, BACK, OPENER, PANEL, STATUS, OUTSIDE = (
    PREFIX + part for part in ("vertical-show", "back", "opener", "panel", "last", "list")
)
VIEWPORT, CONTENT, THUMB = (PANEL + suffix for suffix in ("-viewport", "-content", "-scroll-thumb"))
GROUPS = tuple(PREFIX + part for part in ("file-group", "copy-group", "info-group"))
NAMES = ("檔案", "整理", "資訊")
COMMANDS = tuple(PREFIX + part for part in ("inbox", "archive", "find", "rename", "copy", "export", "details"))
LABELS = ("移到收件匣", "封存", "在清單中尋找", "重新命名目前選取的檔案", "建立副本", "匯出檔案", "顯示詳細資料")
FIRST, DISABLED, FIND, RENAME, _, _, LAST = COMMANDS
TITLE = "檔案動作"
RESULT = "已執行「顯示詳細資料」"


@dataclass(frozen=True)
class Profile:
    width: int
    height: int
    settings: tuple
    groups: int
    shots: tuple

    @property
    def compact(self):
        return self.width == 320


PROFILES = {
    "standard": Profile(960, 1120, (), 3, ("standard-light-grouped",)),
    "vibrant": Profile(960, 1120, ("vibrant", "groups"), 1, ("vibrant-light-single",)),
    "compact": Profile(320, 480, ("vibrant", "theme", "rtl", "bottom", "large-text"), 3,
                       ("vibrant-dark-rtl-200-top", "vibrant-dark-rtl-200-end")),
}
TRANSPORTS = ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"]
BINARIES = ("surface-material-gallery", "surface-devtools")
REPORT = "native-vertical-menu-report.json"
SCHEMA = "surface.test.vertical-command-menu.native.v1"
SHA = re.compile(r"[0-9a-f]{64}")
SOURCE_SHA = "8a2037d8b5fb11afcb226f5b27b41656d7c3dd428b09db9c4aaf2d711cacf2d0"
SOURCE_TREE = "8046e25a630284411e1ebf9f9d16d1004b02651b"
ORIGINAL_NATIVE_SHA = "28bef3519d0d035c1750a80c964b012ea1e40ec117c5d6491800e36a11bb4cbc"
REQUIRED_SOURCE = {
    "examples/material-gallery/src/menu_demo.rs",
    "crates/surface-material/src/command_menu.rs",
    "crates/surface-material/src/command_menu/vertical.rs",
}


def pins(root, manifest, expected):
    require(SHA.fullmatch(expected or "") and manifest.is_file() and not manifest.is_symlink()
            and digest(manifest) == expected, "Reviewed manifest digest differs")
    entries = {}
    for line in manifest.read_text().splitlines():
        value, name = line.split("  ", 1)
        relative = Path(name)
        path = root / relative
        require(SHA.fullmatch(value) and not relative.is_absolute() and ".." not in relative.parts
                and relative.as_posix() == name and name not in entries and name != "."
                and path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root)
                and digest(path) == value, f"Invalid pinned file: {name}")
        entries[name] = value
    require(entries, "Empty manifest")
    return entries


def check_source(args):
    require(args.source_sha == SOURCE_SHA, "Expected the exact corrected source hash")
    source = args.source.resolve(strict=True)
    entries = pins(source, args.source_manifest, args.source_sha)
    local = {path.relative_to(source).as_posix() for path in source.rglob("*") if path.is_file()}
    require(set(entries) == local and REQUIRED_SOURCE <= entries.keys(), "Reviewed source inventory is incomplete")
    version = tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == "0.1.75", "Source is not Surface v0.1.75")
    return {"version": version, "manifest_sha256": args.source_sha, "tree": SOURCE_TREE, "file_count": len(entries)}


def check_inputs(args):
    source = check_source(args)
    require(args.binary_sha != ORIGINAL_NATIVE_SHA, "Use the corrected native build manifest")
    manifest = args.binary_manifest
    require(not manifest.is_symlink(), "Expected a regular binary manifest")
    manifest = manifest.resolve(strict=True)
    entries = pins(manifest.parent, manifest, args.binary_sha)
    require(set(entries) == set(BINARIES), "Expected exactly the Gallery and DevTools executable")
    gallery, cli = (manifest.parent / binary for binary in BINARIES)
    require(all(os.access(path, os.X_OK) for path in (gallery, cli)), "An input is not executable")
    return gallery, cli, {"source": source, "binaries": {"manifest_sha256": args.binary_sha, "files": entries}}


def console_args(width, height):
    return ["--headless", "--width", str(width), "--height", str(height)]


def guard_text(gallery, width, height):
    checks = [f'[ "${index}" = {shlex.quote(arg)} ]'
              for index, arg in enumerate(console_args(width, height), 1)]
    return ('#!/bin/sh\n[ "$#" -eq 5 ] && ' + " && ".join(checks)
            + ' || exit 71\n' + f'exec {shlex.quote(str(gallery))} "$@"\n')


@contextmanager
def diagnose_failure(directory, profile_name, app, stage, windows):
    try:
        yield
    except Exception as error:
        # The app is still connected here. Save the actual DOM snapshot before
        # the outer launch guard cleans up its child. Never save the session.
        record = {"scope": "failed headless native replay", "profile": profile_name, "stage": stage["name"],
                  "error_type": type(error).__name__, "error": str(error), "pid": app.proc.pid,
                  "window_samples_before_failure": dict(windows)}
        try:
            record["snapshot"] = app.wire("DOM.snapshot", {})
        except (OSError, RuntimeError, ValueError) as unavailable:
            record["snapshot_error"] = str(unavailable)
        path = directory / f"{profile_name}-failure.json"
        path.write_text(json.dumps(record, ensure_ascii=False, indent=2, sort_keys=True) + "\n")
        print(f"Original failure details: {path}", file=sys.stderr)
        raise


def right(rect):
    return rect["x"] + rect["width"]


def bottom(rect):
    return rect["y"] + rect["height"]


def near(a, b):
    return abs(a - b) <= 0.5


def absent(frame, target):
    require(find(frame, target)[1] is None, f"{target} is still mounted")


def expected_controls(profile):
    values = {name: name in profile.settings for name in ("groups", "vibrant", "rtl", "large-text", "bottom", "theme")}
    values["groups"] = profile.groups == 3
    return values


def controls(frame, profile):
    observed = {name: node(frame, PREFIX + name).get("selected") for name in expected_controls(profile) if name != "theme"}
    observed["theme"] = node(frame, PREFIX + "theme").get("checked")
    require(observed == expected_controls(profile) and all(type(value) is bool for value in observed.values()),
            f"Gallery controls do not match the requested profile: {observed!r}")
    return observed


def open_menu(app):
    current = settle(app)
    absent(current, PANEL)
    require(node(current, OPENER).get("expanded") is False, "Expected a collapsed trigger")
    if current["interaction"]["focused"] != OPENER:
        focus_by_tab(app, OPENER)
    frame = key(app, "enter")
    focus(frame, FIRST)
    require(node(frame, OPENER).get("expanded") is True, "Trigger does not expose expanded state")
    return frame


def closed(frame, target):
    absent(frame, PANEL)
    absent(frame, THUMB)
    require(node(frame, OPENER).get("expanded") is False, "Trigger stayed expanded")
    focus(frame, target)
    return frame["interaction"]["focused"]


def group_for(index, profile):
    if profile.groups == 1 or index < 3:
        return GROUPS[0]
    return GROUPS[1] if index < 6 else GROUPS[2]


def semantics(frame, profile):
    panel_index, panel = find(frame, PANEL)
    view_index, view = find(frame, VIEWPORT)
    content_index, content = find(frame, CONTENT)
    require(panel_index is not None and view is not None and content is not None
            and view["parent"] == panel_index and content["parent"] == view_index,
            "Popup viewport or content left its authored menu")
    groups = {}
    for name in GROUPS[:profile.groups]:
        item = node(frame, name)
        require(item["parent"] == content_index, f"{name} left its menu content")
        groups[name] = [item["role"], item["name"]]
    for name in GROUPS[profile.groups:]:
        absent(frame, name)
    commands = {}
    for index, name in enumerate(COMMANDS):
        target = node(frame, name)
        owner = group_for(index, profile)
        require(target["parent"] == find(frame, owner)[0], f"{name} is detached from {owner}")
        commands[name] = [target["role"], target["name"], target["disabled"], owner]
    payload = {"menu": [panel["role"], panel["name"]], "groups": groups, "commands": commands}
    verify_semantics(payload, profile)
    return payload


def verify_semantics(payload, profile):
    groups = dict(zip(GROUPS, (["Group", label] for label in NAMES), strict=True))
    if profile.groups == 1:
        groups = {GROUPS[0]: ["Group", ""]}
    commands = {name: ["MenuItem", label, name == DISABLED, group_for(index, profile)]
                for index, (name, label) in enumerate(zip(COMMANDS, LABELS, strict=True))}
    require(payload == {"menu": ["Menu", TITLE], "groups": groups, "commands": commands}
            and all(type(row[2]) is bool for row in payload["commands"].values()),
            f"Wrong menu, group or command semantics: {payload!r}")


def geometry(frame, profile, target):
    data = {"trigger": visible(frame, OPENER), "panel": visible(frame, PANEL),
            "viewport": visible(frame, VIEWPORT), "contents": layout(frame, CONTENT)["bounds"],
            "groups": {group: layout(frame, group)["bounds"] for group in GROUPS[:profile.groups]},
            "commands": {command: layout(frame, command) for command in COMMANDS},
            "thumb": visible(frame, THUMB) if find(frame, THUMB)[1] is not None else None}
    verify_geometry(data, profile, target)
    return data


def verify_geometry(data, profile, target):
    panel, trigger, viewport, contents = (data[part] for part in ("panel", "trigger", "viewport", "contents"))
    safe = {"x": 8, "y": 8, "width": profile.width - 16, "height": profile.height - 16}
    require(same_box(safe, panel) and near(panel["width"], 240) and panel["height"] >= 48
            and same_box(panel, viewport), f"Invalid popup or scroller bounds: {panel!r}, {viewport!r}")
    gap = trigger["y"] - bottom(panel) if profile.compact else panel["y"] - bottom(trigger)
    require(near(gap, 4), f"Popup lost its 4 dp trigger gap: {gap}")
    require(set(data["groups"]) == set(GROUPS[:profile.groups])
            and set(data["commands"]) == set(COMMANDS), "Group or command bounds are missing")
    group_boxes = [data["groups"][name] for name in GROUPS[:profile.groups]]
    group_gap = 0 if profile.compact else 2
    require(all(near(item["y"] - bottom(previous), group_gap)
                for previous, item in zip(group_boxes, group_boxes[1:])),
            f"Wrong {group_gap} dp inter-group spacing: {group_boxes!r}")
    for index, name in enumerate(COMMANDS):
        row = data["commands"][name]["bounds"]
        own = data["groups"][group_for(index, profile)]
        require(row["width"] >= 48 and row["height"] >= 48 and same_box(own, row),
                f"{name}: command hit box left its group or minimum extent: {row!r}")
        if name == target or not profile.compact:
            require(same_box(viewport, row) and same_box(data["commands"][name]["clip"], row),
                    f"{name}: expected a fully visible command: {data['commands'][name]!r}")
    thumb = data["thumb"]
    if profile.compact:
        require(data["commands"][RENAME]["bounds"]["height"] > 48
                and data["commands"][RENAME]["clip"] == viewport
                and contents["height"] > viewport["height"], "The compact viewport does not wrap and scroll")
        require(thumb is not None and same_box(panel, thumb) and near(thumb["width"], 4)
                and near(thumb["x"] - panel["x"], 2) and thumb["height"] >= 16,
                f"The RTL thumb lost its 4 dp width or left trailing edge: {thumb!r}")
    else:
        require(thumb is None and same_box(viewport, contents), "A fitting menu gained a thumb or lost content")


def verify_scroll(before, end):
    require(all(before[key] == end[key] for key in ("trigger", "panel", "viewport")),
            "Scrolling moved the panel, viewport or trigger")
    first, last = before["thumb"], end["thumb"]
    row_before, row_end = (snapshot["commands"][LAST]["bounds"] for snapshot in (before, end))
    view_height = before["viewport"]["height"]
    track = view_height - 8
    ideal_height = max(16, track * view_height / before["contents"]["height"])
    require(near(first["y"] - before["panel"]["y"], 4)
            and last["y"] > first["y"] and near(last["x"], first["x"])
            and near(last["width"], first["width"])
            and all(abs(thumb["height"] - ideal_height) <= 1 for thumb in (first, last))
            and not same_box(before["viewport"], row_before) and row_end["y"] < row_before["y"]
            and same_box(end["viewport"], row_end),
            f"End proof differs: viewport {end['viewport']!r}, initial thumb {first!r}, "
            f"final thumb {last!r}, initial last row {row_before!r}, final row {row_end!r}")
    translated = before["contents"]["y"] - end["contents"]["y"]
    require(translated > 0 and all(near(before["groups"][name]["y"] - end["groups"][name]["y"], translated)
                                   for name in GROUPS), "The inner content did not scroll as a unit")
    return translated


def rgba(value):
    require(isinstance(value, list) and len(value) == 4
            and all(type(component) is int and 0 <= component <= 255 for component in value),
            f"Expected RGBA8 paint, got {value!r}")
    return tuple(value)


def artwork(frame, measured, profile, selected):
    group = group_for(COMMANDS.index(selected), profile)
    paper = group if profile.groups == 3 and not profile.compact else PANEL
    return {"label": visible(frame, selected + "-label"),
            "ink": node(frame, selected + "-label")["style"]["foreground"],
            "ring": node(frame, selected)["focus_color"], "group": group, "focused": selected,
            "surface": node(frame, paper)["style"]["background"],
            "page": node(frame, "surface-page")["style"]["background"],
            "thumb": node(frame, THUMB)["style"]["background"] if measured["thumb"] else None,
            "icon": visible(frame, FIRST + "-leading") if selected == FIRST else None,
            "icon_ink": node(frame, FIRST + "-leading")["fill_contour"]["color"] if selected == FIRST else None}


def paint(path, profile, measured, art):
    width, height, rows = read_png(path.read_bytes())
    require((width, height) == (profile.width, profile.height), "The original frame must be 1×")
    surface, page, ink, ring = (rgba(art[key]) for key in ("surface", "page", "ink", "ring"))
    require(surface != page and surface != ink and all(value[3] == 255 for value in (surface, page, ink, ring)),
            f"Invalid opaque menu palette: {art!r}")

    def pixel(x, y):
        x, y = math.floor(x), math.floor(y)
        require(0 <= x < width and 0 <= y < height, f"Pixel ({x}, {y}) leaves the canvas")
        return tuple(rows[y][x * 4:(x + 1) * 4])

    selected = art["focused"]
    group = group_for(COMMANDS.index(selected), profile)
    hit = measured["commands"][selected]["bounds"]
    group_box = measured["groups"][group]
    require(art["group"] == group and same_box(hit, art["label"]), "The sampled label is outside its command")
    x = right(group_box) - 2 if profile.compact else group_box["x"] + 2
    middle_y = hit["y"] + hit["height"] / 2
    actual = pixel(x, middle_y)
    require(actual == surface and pixel(2, 2) == page,
            f"Wrong menu surface or surrounding page pixels: {actual!r} vs {surface!r}")
    middle = math.floor(hit["x"] + hit["width"] / 2)
    ring_pixels = sum(pixel(x, hit["y"] + y) == ring for x in range(middle - 5, middle + 6) for y in (0, 1, 2))
    require(ring_pixels >= 8, f"Focus stroke missing on {selected}: {ring_pixels} pixels")

    def count(rect, color):
        return sum(pixel(x, y) == color for y in range(math.ceil(rect["y"]), math.floor(bottom(rect)))
                   for x in range(math.ceil(rect["x"]), math.floor(right(rect))))

    labels = count(art["label"], ink)
    require(labels > 12, f"Too little actual command label ink on {selected}: {labels}")
    icon_pixels = None
    if selected == FIRST:
        icon = art["icon"]
        require(same_box(hit, icon) and near(icon["width"], 20) and near(icon["height"], 20),
                "Leading icon is no longer 20 dp inside its row")
        icon_pixels = count(icon, rgba(art["icon_ink"]))
        require(icon_pixels > 12, f"Too little actual leading icon ink: {icon_pixels}")
    else:
        require(art["icon"] is None and art["icon_ink"] is None, "Unexpected first-row icon sample")
    thumb = measured["thumb"]
    thumb_pixel = None
    if thumb is not None:
        thumb_ink = rgba(art["thumb"])
        thumb_pixel = pixel(thumb["x"] + thumb["width"] / 2, thumb["y"] + thumb["height"] / 2)
        require(thumb_pixel == thumb_ink and thumb_ink == ink,
                f"Vibrant thumb paint differs from its paired ink: {thumb_pixel!r} vs {thumb_ink!r}")
    else:
        require(art["thumb"] is None, "The non-scrolling palette has a thumb entry")
    return {"surface": list(actual), "focus_pixels": ring_pixels, "label_pixels": labels,
            "icon_pixels": icon_pixels, "thumb": None if thumb_pixel is None else list(thumb_pixel)}


def reject_disabled(app):
    actions = {}
    for name in ("enter", "space"):
        result = app.wire("Action.key", {"key": name})
        current = settle(app)
        focus(current, DISABLED)
        absent(current, STATUS)
        require(node(current, OPENER).get("expanded") is True and result["changed"] is False,
                f"{name} changed the disabled command state: {result!r}")
        actions[name] = {"focus": current["interaction"]["focused"], "changed": result["changed"]}
    actions["direct"] = app.wire("Action.activate", {"id": DISABLED}, expected_error="disabled")
    return actions


def window_phases(profile):
    return {"ready", "route", "menu", "closed", "returned"} | {
        f"{phase}/{shot}" for phase in ("before", "after") for shot in profile.shots
    }


def play(directory, profile_name, gallery, cli):
    profile = PROFILES[profile_name]
    stage, windows = {"name": "launch"}, {}

    def sample(app, phase):
        require(app.proc.poll() is None and phase not in windows, "Invalid owned process observation")
        windows[phase] = onscreen_windows(app.proc.pid)
        require(windows[phase] == 0, f"{profile_name}/{phase}: a native window was visible")

    with TemporaryDirectory(prefix=".argv-", dir=directory) as tmp:
        guard = Path(tmp) / "headless-gallery"
        guard.write_text(guard_text(gallery, profile.width, profile.height))
        guard.chmod(0o700)
        plan = SimpleNamespace(out=directory, gallery=guard, cli=cli)
        with (launch(plan, profile_name, profile.width, profile.height) as app,
              diagnose_failure(directory, profile_name, app, stage, windows)):
            sample(app, "ready")
            stage["name"] = "route"
            activate(app, "section-2")
            focus_by_tab(app, SHOW)
            focus(key(app, "enter"), BACK)
            sample(app, "route")
            for setting in profile.settings:
                activate(app, PREFIX + setting)
            configured = controls(settle(app), profile)
            stage["name"] = "initial-menu"
            frame = open_menu(app)
            sample(app, "menu")
            names = semantics(frame, profile)
            initial = geometry(frame, profile, FIRST)
            history = [frame["interaction"]["focused"]]
            shots = {}
            blocked, accepted, remounted, movement = None, None, None, None

            def shot(capture_frame, measured, selected, tag):
                stage["name"] = tag
                focus(capture_frame, selected)
                require(semantics(capture_frame, profile) == names, "A repaint changed menu owners")
                art = artwork(capture_frame, measured, profile, selected)
                sample(app, f"before/{tag}")
                image = capture(app, plan, tag)
                sample(app, f"after/{tag}")
                pixels = paint(directory / image["name"], profile, measured, art)
                shots[tag] = {"geometry": measured, "art": art, "image": image, "pixels": pixels}

            if profile_name == "standard":
                stage["name"] = "disabled-and-roving"
                frame = key(app, "down")
                focus(frame, DISABLED)
                history.append(frame["interaction"]["focused"])
                blocked = reject_disabled(app)
                for target in (FIND, RENAME):
                    frame = key(app, "down")
                    focus(frame, target)
                    history.append(frame["interaction"]["focused"])
                shot(frame, geometry(frame, profile, RENAME), RENAME, profile.shots[0])
                stage["name"] = "command-activation"
                last = key(app, "end")
                focus(last, LAST)
                history.append(last["interaction"]["focused"])
                frame = key(app, "enter")
                close_focus = closed(frame, OPENER)
                status = node(frame, STATUS)
                require(status["role"] == "Text" and status["text"] == RESULT,
                        "One Enter did not publish the selected command result")
                accepted = {"key": "enter", "command": LAST, "status": status["text"]}
            elif profile_name == "vibrant":
                shot(frame, initial, FIRST, profile.shots[0])
                stage["name"] = "tab-dismissal"
                frame = key(app, "tab")
                close_focus = closed(frame, OUTSIDE)
                absent(frame, STATUS)
            else:
                shot(frame, initial, FIRST, profile.shots[0])
                stage["name"] = "end-scroll"
                frame = key(app, "end")
                focus(frame, LAST)
                history.append(frame["interaction"]["focused"])
                # The focus reveal updates layout first. Its measurement
                # callback positions the thumb in the following frame.
                from time import monotonic, sleep
                deadline = monotonic() + 2
                while True:
                    frame = app.dom()
                    focus(frame, LAST)
                    try:
                        end = geometry(frame, profile, LAST)
                        movement = verify_scroll(initial, end)
                        break
                    except RuntimeError:
                        if monotonic() >= deadline:
                            raise
                    sleep(0.02)
                shot(frame, end, LAST, profile.shots[1])
                stage["name"] = "escape-and-remount"
                frame = key(app, "escape")
                close_focus = closed(frame, OPENER)
                absent(frame, STATUS)
                reopened = open_menu(app)
                remounted = geometry(reopened, profile, FIRST)
                frame = key(app, "escape")
                require(closed(frame, OPENER) == close_focus, "Reopened popup changed Escape focus")
                absent(frame, STATUS)
            sample(app, "closed")
            stage["name"] = "return-launcher"
            focus_by_tab(app, BACK)
            returned = key(app, "enter")
            focus(returned, SHOW)
            absent(returned, OPENER)
            absent(returned, PANEL)
            sample(app, "returned")

    require(app.proc.returncode is not None, "The owned Gallery process was not reaped")
    return ({"controls": configured, "semantics": names, "initial": initial, "focus_history": history,
             "dismiss_focus": close_focus, "returned_focus": returned["interaction"]["focused"],
             "disabled": blocked, "accepted": accepted, "remounted": remounted, "scrolled_dp": movement,
             "captures": shots},
            {"profile": profile_name, "pid": app.proc.pid, "reaped": True,
             "argv": console_args(profile.width, profile.height), "visible_windows": windows})


def verify_play(record, profile_name, directory, files):
    profile = PROFILES[profile_name]
    verify_semantics(record["semantics"], profile)
    require(record["controls"] == expected_controls(profile) and record["returned_focus"] == SHOW,
            "Profile controls or launcher restoration changed")
    verify_geometry(record["initial"], profile, FIRST)
    shots = record["captures"]
    require(set(shots) == set(profile.shots), "Wrong capture inventory")
    expected_targets = (RENAME,) if profile_name == "standard" else (FIRST, LAST) if profile.compact else (FIRST,)
    for tag, target in zip(profile.shots, expected_targets, strict=True):
        shot = shots[tag]
        path = directory / f"{tag}.png"
        require(shot["art"]["focused"] == target, f"Wrong capture focus for {tag}")
        verify_geometry(shot["geometry"], profile, target)
        require(shot["image"] == {"name": path.name, "format": "png", "width": profile.width,
                                  "height": profile.height, "scale": 1,
                                  "sha256": files[f"{directory.name}/{path.name}"]}
                and shot["pixels"] == paint(path, profile, shot["geometry"], shot["art"]),
                f"Original PNG receipt disagrees for {tag}")
    require(record["dismiss_focus"] == (OUTSIDE if profile_name == "vibrant" else OPENER),
            "Wrong dismissal focus")
    if profile_name == "standard":
        require(record["focus_history"] == [FIRST, DISABLED, FIND, RENAME, LAST]
                and record["disabled"] == {"enter": {"focus": DISABLED, "changed": False},
                                           "space": {"focus": DISABLED, "changed": False}, "direct": "disabled"}
                and record["accepted"] == {"key": "enter", "command": LAST, "status": RESULT}
                and record["scrolled_dp"] is None and record["remounted"] is None,
                "Standard roaming, disabled rejection or activation receipt differs")
    elif profile.compact:
        require(record["focus_history"] == [FIRST, LAST] and record["accepted"] is None and record["disabled"] is None
                and shots[profile.shots[0]]["geometry"] == record["initial"]
                and record["scrolled_dp"] == verify_scroll(record["initial"], shots[profile.shots[1]]["geometry"]),
                "Compact overflow, scroll or focus movement differs")
        verify_geometry(record["remounted"], profile, FIRST)
        require(record["remounted"]["panel"] == record["initial"]["panel"], "Popup remount moved its panel")
    else:
        require(record["focus_history"] == [FIRST]
                and all(record[key] is None for key in ("remounted", "scrolled_dp", "accepted", "disabled")),
                "Vibrant Tab dismissal receipt differs")


def readback(root, evidence_sha, source_sha, binary_sha):
    require(source_sha == SOURCE_SHA, "Expected the exact corrected source hash")
    require(binary_sha != ORIGINAL_NATIVE_SHA, "Use the corrected native build manifest")
    files = pins(root, root / "SHA256SUMS", evidence_sha)
    repeats = ("1", "2")
    expected = {REPORT} | {f"repeat-{n}/{tag}.png" for n in repeats for profile in PROFILES.values() for tag in profile.shots}
    actual = {path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file()}
    require(set(files) == expected and actual == expected | {"SHA256SUMS"}, "Unexpected evidence inventory")
    report = json.loads((root / REPORT).read_text())
    require(report["schema"] == SCHEMA and report["transports"] == TRANSPORTS
            and report["source"]["manifest_sha256"] == source_sha and report["source"]["version"] == "0.1.75"
            and report["source"]["tree"] == SOURCE_TREE
            and type(report["source"]["file_count"]) is int and report["source"]["file_count"] == 698
            and report["binaries"]["manifest_sha256"] == binary_sha
            and set(report["binaries"]["files"]) == set(BINARIES)
            and all(SHA.fullmatch(value) for value in report["binaries"]["files"].values()),
            "Source or executable pin differs")
    replays = report["replays"]
    require(set(replays) == set(repeats) and replays["1"] == replays["2"]
            and set(replays["1"]) == set(PROFILES), "The repeats differ in geometry, semantics or PNG bytes")
    require(len({tuple(record["captures"][PROFILES[name].shots[0]]["pixels"]["surface"])
                 for name, record in replays["1"].items()}) == 3, "All three menu surface profiles need distinct paint")
    for n, records in replays.items():
        for profile_name, record in records.items():
            verify_play(record, profile_name, root / f"repeat-{n}", files)
    processes = report["processes"]
    required = {(n, name) for n in repeats for name in PROFILES}
    require(len(processes) == len(required) and len({item["pid"] for item in processes}) == len(required)
            and {(item["repeat"], item["profile"]) for item in processes} == required,
            "Missing replay or owned processes")
    for item in processes:
        profile = PROFILES[item["profile"]]
        require(type(item["pid"]) is int and item["pid"] > 0 and item["reaped"] is True
                and item["argv"] == console_args(profile.width, profile.height)
                and set(item["visible_windows"]) == window_phases(profile)
                and all(type(value) is int and value == 0 for value in item["visible_windows"].values()),
                "Owned native process or window observation differs")
    return {"schema": SCHEMA, "source_sha256": source_sha, "source_tree": SOURCE_TREE, "binary_sha256": binary_sha, "evidence_sha256": evidence_sha,
            "original_pngs": len(expected) - 1, "repeats": len(repeats), "transports": TRANSPORTS,
            "window_samples": sum(len(item["visible_windows"]) for item in processes),
            "visible_windows": 0, "reaped": len(processes)}


def run(args):
    require(sys.platform == "darwin", "Native window verification needs macOS")
    gallery, cli, inputs = check_inputs(args)
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()) and out != Path("/tmp").resolve(), "Use a fresh private /tmp directory")
    out.mkdir(mode=0o700)
    report = {"schema": SCHEMA, **inputs, "transports": TRANSPORTS, "replays": {}, "processes": []}
    for repeat in ("1", "2"):
        directory = out / f"repeat-{repeat}"
        directory.mkdir()
        records = {}
        for profile_name in PROFILES:
            record, process = play(directory, profile_name, gallery, cli)
            records[profile_name] = record
            report["processes"].append({"repeat": repeat, **process})
        report["replays"][repeat] = records
    require(check_inputs(args) == (gallery, cli, inputs), "Source or executable pins changed during the run")
    (out / REPORT).write_text(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n")
    manifest = out / "SHA256SUMS"
    members = sorted(path for path in out.rglob("*") if path.is_file())
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in members))
    return readback(out, digest(manifest), args.source_sha, args.binary_sha)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check-source", action="store_true")
    mode.add_argument("--check-inputs", action="store_true")
    mode.add_argument("--run", action="store_true")
    mode.add_argument("--readback", type=Path)
    for option in ("source", "source-manifest", "binary-manifest", "out"):
        parser.add_argument(f"--{option}", type=Path)
    for option in ("source-sha", "binary-sha", "evidence-sha"):
        parser.add_argument(f"--{option}")
    args = parser.parse_args()
    required = ["source_sha"]
    if not args.readback:
        required.extend(("source", "source_manifest"))
    if args.run or args.check_inputs:
        required.extend(("binary_sha", "binary_manifest"))
    if args.run:
        required.append("out")
    if args.readback:
        required.extend(("binary_sha", "evidence_sha"))
    for option in required:
        if getattr(args, option) is None:
            parser.error(f"--{option.replace('_', '-')} is required for this mode")
    if args.check_source:
        result = check_source(args)
    elif args.check_inputs:
        result = check_inputs(args)[2]
    elif args.run:
        result = run(args)
    else:
        result = readback(args.readback, args.evidence_sha, args.source_sha, args.binary_sha)
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit(f"Corrected vertical menu native replay stopped. {error}")
