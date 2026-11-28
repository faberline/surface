#!/usr/bin/env python3
"""Verify the Expressive vertical choice menu in the native headless Gallery."""

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
SHOW, BACK, OPENER, PANEL, OUTSIDE = (
    PREFIX + part for part in ("choice-show", "back", "opener", "panel", "list")
)
VIEWPORT, CONTENT, THUMB = (PANEL + suffix for suffix in ("-viewport", "-content", "-scroll-thumb"))
CHOICE = PREFIX + "choice-"
GROUPS = {CHOICE + key: name for key, name in (
    ("sort", "排序方式"), ("density", "顯示密度"), ("fields", "在清單中顯示"),
)}
RECENT, NAME, OWNER, COMFORTABLE, COMPACT, KIND, SIZE, LOCKED = (
    CHOICE + key for key in ("recent", "name", "owner", "comfortable", "compact", "kind", "size", "locked")
)
LABELS = dict(zip((RECENT, NAME, OWNER, COMFORTABLE, COMPACT, KIND, SIZE, LOCKED),
                  ("最近修改", "名稱", "擁有者", "舒適", "緊湊", "檔案類型",
                   "檔案大小與可用空間的詳細資料", "鎖定狀態"), strict=True))
MEMBERS = dict(zip(GROUPS, ((RECENT, NAME, OWNER), (COMFORTABLE, COMPACT), (KIND, SIZE, LOCKED)), strict=True))
OWNERS = {entry: group for group, entries in MEMBERS.items() for entry in entries}
RADIOS = set(LABELS) - {KIND, SIZE, LOCKED}
DISABLED = {OWNER, LOCKED}
TITLE = "檔案動作"
INITIAL = frozenset((RECENT, COMFORTABLE, KIND, LOCKED))
AFTER_SORT = INITIAL - {RECENT} | {NAME}
AFTER_DENSITY = AFTER_SORT - {COMFORTABLE} | {COMPACT}
STANDARD_CHECKS = AFTER_DENSITY | {SIZE}
STANDARD_FINAL = STANDARD_CHECKS - {KIND}
VIBRANT_FINAL = INITIAL | {SIZE}
TAIL_FINAL = VIBRANT_FINAL - {KIND}


@dataclass(frozen=True)
class Profile:
    width: int
    height: int
    settings: tuple
    shots: tuple
    surface: tuple

    @property
    def compact(self):
        return self.width == 320


PROFILES = {
    "standard": Profile(960, 1120, (), ("standard-light-initial",), (248, 242, 250, 255)),
    "vibrant": Profile(960, 1120, ("vibrant",), ("vibrant-light-checks",), (255, 217, 227, 255)),
    "compact": Profile(320, 480, ("vibrant", "theme", "rtl", "bottom", "large-text"),
                       ("vibrant-dark-rtl-200-initial", "vibrant-dark-rtl-200-checked"), (99, 59, 72, 255)),
}
TRANSPORTS = ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"]
BINARIES = ("surface-material-gallery", "surface-devtools")
REPORT = "native-vertical-choice-menu-report.json"
SCHEMA = "surface.test.vertical-choice-menu.native.v1"
SHA = re.compile(r"[0-9a-f]{64}")
REQUIRED_SOURCE = {
    "examples/material-gallery/src/menu_demo.rs",
    "examples/material-gallery/src/menu_demo/choices.rs",
    "crates/surface-material/src/command_menu.rs",
    "crates/surface-material/src/command_menu/vertical.rs",
    "crates/surface/src/microdom.rs",
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
    source = args.source.resolve(strict=True)
    entries = pins(source, args.source_manifest, args.source_sha)
    local = {path.relative_to(source).as_posix() for path in source.rglob("*") if path.is_file()}
    require(set(entries) == local and REQUIRED_SOURCE <= entries.keys(), "Reviewed source inventory is incomplete")
    version = tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == args.source_version, "Source version differs from the reviewed input")
    return {"version": version, "manifest_sha256": args.source_sha, "file_count": len(entries)}


def check_inputs(args):
    source = check_source(args)
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


def open_menu(app):
    current = settle(app)
    absent(current, PANEL)
    require(node(current, OPENER).get("expanded") is False, "Expected a collapsed trigger")
    if current["interaction"]["focused"] != OPENER:
        focus_by_tab(app, OPENER)
    frame = key(app, "enter")
    focus(frame, RECENT)
    require(node(frame, OPENER).get("expanded") is True, "Trigger does not expose expanded state")
    return frame


def closed(frame, target):
    absent(frame, PANEL)
    absent(frame, THUMB)
    require(node(frame, OPENER).get("expanded") is False, "Trigger stayed expanded")
    focus(frame, target)
    return frame["interaction"]["focused"]




def expected_controls(profile):
    return {setting: setting in profile.settings for setting in ("vibrant", "rtl", "large-text", "bottom", "theme")}


def controls(frame, profile):
    values = {name: node(frame, PREFIX + name).get("checked" if name == "theme" else "selected")
              for name in expected_controls(profile)}
    absent(frame, PREFIX + "groups")
    require(values == expected_controls(profile) and all(type(value) is bool for value in values.values()),
            f"Unexpected choice fixture configuration: {values!r}")
    return values


def verify_semantics(payload, checked):
    require(payload["menu"] == ["Menu", TITLE]
            and payload["groups"] == {name: ["Group", label] for name, label in GROUPS.items()},
            f"Wrong named choice groups: {payload!r}")
    expected = {
        name: {"role": "MenuItem", "name": LABELS[name], "disabled": name in DISABLED,
               "owner": OWNERS[name], "checked": name in checked, "radio": name in RADIOS}
        for name in LABELS
    }
    require(payload["items"] == expected
            and all(type(row[key]) is bool for row in payload["items"].values()
                    for key in ("checked", "radio", "disabled"))
            and all(len(set(members) & checked) == 1 for members in list(MEMBERS.values())[:2]),
            f"Wrong checked state or group scope: {payload['items']!r}")


def semantics(frame, checked):
    panel_index, panel = find(frame, PANEL)
    view_index, view = find(frame, VIEWPORT)
    content_index, content = find(frame, CONTENT)
    require(panel_index is not None and view is not None and content is not None
            and view["parent"] == panel_index and content["parent"] == view_index,
            "Choice viewport or content left its menu")
    groups = {}
    for name in GROUPS:
        item = node(frame, name)
        require(item["parent"] == content_index, f"{name} left the menu content")
        groups[name] = [item["role"], item["name"]]
    items = {}
    for name in LABELS:
        item = node(frame, name)
        owner = OWNERS[name]
        require(item["parent"] == find(frame, owner)[0], f"{name} left its group")
        items[name] = {key: item[key] for key in ("role", "name", "disabled", "checked")}
        items[name].update(owner=owner, radio=item.get("menu_radio", False))
    payload = {"menu": [panel["role"], panel["name"]], "groups": groups, "items": items}
    verify_semantics(payload, checked)
    return payload


def observation(frame, target, checked=None):
    focus(frame, target)
    if checked is None:
        closed(frame, target)
        return {"focus": target, "expanded": False, "semantics": None}
    require(node(frame, OPENER).get("expanded") is True, "Open trigger is not expanded")
    return {"focus": target, "expanded": True, "semantics": semantics(frame, checked)}


def verify_observation(record, target, checked=None):
    require(record["focus"] == target and record["expanded"] is (checked is not None),
            f"Wrong observed focus or expanded state: {record!r}")
    if checked is None:
        require(record["semantics"] is None, "Closed menu still supplied row state")
    else:
        verify_semantics(record["semantics"], checked)


def geometry(frame, profile, target):
    data = {"trigger": visible(frame, OPENER), "panel": visible(frame, PANEL),
            "viewport": visible(frame, VIEWPORT), "contents": layout(frame, CONTENT)["bounds"],
            "groups": {name: layout(frame, name)["bounds"] for name in GROUPS},
            "items": {name: layout(frame, name) for name in LABELS},
            "thumb": visible(frame, THUMB) if find(frame, THUMB)[1] is not None else None}
    verify_geometry(data, profile, target)
    return data


def verify_geometry(data, profile, target):
    panel, trigger, viewport, contents = (data[part] for part in ("panel", "trigger", "viewport", "contents"))
    safe = {"x": 8, "y": 8, "width": profile.width - 16, "height": profile.height - 16}
    require(same_box(safe, panel) and panel["height"] >= 48 and near(panel["width"], 240)
            and same_box(panel, viewport) and same_box(viewport, panel), "Popup bounds differ")
    gap = trigger["y"] - bottom(panel) if profile.compact else panel["y"] - bottom(trigger)
    require(near(gap, 4), f"Popup lost its 4 dp trigger gap: {gap}")
    require(set(data["groups"]) == set(GROUPS) and set(data["items"]) == set(LABELS),
            "Group or item bounds are missing")
    groups = [data["groups"][group] for group in GROUPS]
    group_gap = 0 if profile.compact else 2
    require(all(near(after["y"] - bottom(before), group_gap)
                for before, after in zip(groups, groups[1:])), f"Wrong {group_gap} dp group gap")
    for name, measured in data["items"].items():
        row = measured["bounds"]
        require(row["width"] >= 48 and row["height"] >= 48 and same_box(data["groups"][OWNERS[name]], row),
                f"{name}: choice hit box left its group or minimum area")
        if name == target or not profile.compact:
            require(same_box(viewport, row) and same_box(measured["clip"], row),
                    f"{name}: expected a full target inside the viewport")
    thumb = data["thumb"]
    if profile.compact:
        require(data["items"][SIZE]["bounds"]["height"] > 48 and contents["height"] > viewport["height"],
                "The narrow menu lost wrapping or its scroll extent")
        require(thumb is not None and same_box(panel, thumb) and near(thumb["width"], 4)
                and near(thumb["x"] - panel["x"], 2) and thumb["height"] >= 16,
                "The narrow RTL scroll thumb lost its size or logical trailing edge")
    else:
        require(thumb is None and same_box(viewport, contents), "Fitting content gained a scrollbar")


def verify_scroll(before, end):
    require(all(before[key] == end[key] for key in ("trigger", "panel", "viewport")),
            "Scroll moved the panel, scroller or trigger")
    first, last = before["thumb"], end["thumb"]
    before_lock, end_lock = (entry["items"][LOCKED]["bounds"] for entry in (before, end))
    viewport = before["viewport"]
    ideal_height = max(16, (viewport["height"] - 8) * viewport["height"] / before["contents"]["height"])
    require(near(first["y"] - viewport["y"], 4) and last["y"] > first["y"]
            and near(last["x"], first["x"]) and near(first["width"], last["width"])
            and all(abs(thumb["height"] - ideal_height) <= 1 for thumb in (first, last))
            and not same_box(viewport, before_lock) and same_box(viewport, end_lock),
            f"End did not reveal the last row with a proportional thumb: {first!r}, {last!r}")
    translated = before["contents"]["y"] - end["contents"]["y"]
    require(translated > 0 and all(near(before["groups"][name]["y"] - end["groups"][name]["y"], translated)
                                   for name in GROUPS), "Menu groups did not scroll together")
    return translated


def rgba(values):
    require(isinstance(values, (tuple, list)) and len(values) == 4
            and all(type(value) is int and 0 <= value <= 255 for value in values), f"Bad RGBA color: {values!r}")
    return tuple(values)


def contrast(a, b):
    def linear(c):
        c /= 255
        return c / 12.92 if c <= .04045 else ((c + .055) / 1.055) ** 2.4

    def luminance(color):
        return sum(weight * linear(value) for weight, value in zip((.2126, .7152, .0722), color[:3], strict=True))

    dark, light = sorted((luminance(a), luminance(b)))
    return round((light + .05) / (dark + .05), 6)


def artwork(frame, profile, focused, selected, unchecked):
    selected_node = node(frame, selected + "-leading")
    unchecked_node = node(frame, unchecked + "-leading")
    require(selected_node["role"] == "Group" and unchecked_node["role"] == "Group"
            and unchecked_node.get("fill_contour") is None, "A passive check slot gained an unchecked mark")
    return {
        "focus": focused, "selected": selected, "unchecked": unchecked,
        "ring": node(frame, focused)["focus_color"],
        "selected_face": visible(frame, selected + "-face"),
        "unchecked_face": visible(frame, unchecked + "-face"),
        "selected_fill": node(frame, selected + "-face")["style"]["background"],
        "unchecked_fill": node(frame, unchecked + "-face")["style"]["background"],
        "selected_label": visible(frame, selected + "-label"),
        "mark": visible(frame, selected + "-leading"),
        "mark_ink": selected_node["fill_contour"]["color"],
        "label_ink": node(frame, selected + "-label")["style"]["foreground"],
        "empty_mark": visible(frame, unchecked + "-leading"),
        "surface": node(frame, PANEL if profile.compact else OWNERS[selected])["style"]["background"],
        "page": node(frame, "surface-page")["style"]["background"],
        "thumb_ink": node(frame, THUMB)["style"]["background"] if profile.compact else None,
    }


def paint(path, profile, measured, art):
    width, height, rows = read_png(path.read_bytes())
    require((width, height) == (profile.width, profile.height), "Native PNG is not original 1×")

    def pixel(x, y):
        x, y = math.floor(x), math.floor(y)
        require(0 <= x < width and 0 <= y < height, f"Pixel leaves the canvas at {x}, {y}")
        return tuple(rows[y][x * 4:(x + 1) * 4])

    def count(rect, color):
        return sum(pixel(x, y) == color
                   for y in range(math.ceil(rect["y"]), math.floor(bottom(rect)))
                   for x in range(math.ceil(rect["x"]), math.floor(right(rect))))

    surface, fill, mark_ink, label_ink, page, ring = (
        rgba(art[key]) for key in ("surface", "selected_fill", "mark_ink", "label_ink", "page", "ring")
    )
    require(surface == profile.surface and all(color[3] == 255 for color in (fill, mark_ink, label_ink, page, ring))
            and surface != fill and mark_ink == label_ink and rgba(art["unchecked_fill"])[3] == 0,
            "Selected and unselected palette roles differ")
    require(art["focus"] != art["selected"], "The selected face must be sampled without a focus layer")
    item = measured["items"][art["selected"]]["bounds"]
    unchecked = measured["items"][art["unchecked"]]["bounds"]
    focus_bounds = measured["items"][art["focus"]]["bounds"]
    face, empty_face, mark, empty, label = (
        art[key] for key in ("selected_face", "unchecked_face", "mark", "empty_mark", "selected_label")
    )
    require(same_box(item, face) and same_box(unchecked, empty_face)
            and same_box(face, label) and face["height"] >= 44 and empty_face["height"] >= 44
            and all(same_box(parent, box) and near(box["width"], 20) and near(box["height"], 20)
                    and near(right(parent) - right(box) if profile.compact else box["x"] - parent["x"], 16)
                    for parent, box in ((face, mark), (empty_face, empty))),
            "Check and empty mark slots need stable 20 dp bounds and a 16 dp logical leading inset")

    def face_sample(box):
        trailing = box["x"] + 6 if profile.compact else right(box) - 6
        return pixel(trailing, box["y"] + box["height"] / 2)

    require(face_sample(face) == fill and face_sample(empty_face) == surface and pixel(2, 2) == page,
            "Real selected fill or unchecked surface pixels differ")
    marks, empty_marks, labels = count(mark, mark_ink), count(empty, mark_ink), count(label, label_ink)
    middle = math.floor(focus_bounds["x"] + focus_bounds["width"] / 2)
    ring_pixels = sum(pixel(x, focus_bounds["y"] + y) == ring
                      for x in range(middle - 5, middle + 6) for y in (0, 1, 2))
    require(marks > 12 and empty_marks == 0 and labels > 12 and ring_pixels >= 8,
            f"Missing mark, label, focus or blank slot: {marks}, {empty_marks}, {labels}, {ring_pixels}")
    mark_contrast = contrast(mark_ink, fill)
    require(mark_contrast >= 3, "The selected marker needs visible contrast")
    thumb = measured["thumb"]
    thumb_pixel = None
    if thumb is not None:
        thumb_pixel = pixel(thumb["x"] + thumb["width"] / 2, thumb["y"] + thumb["height"] / 2)
        require(thumb_pixel == rgba(art["thumb_ink"]), "Wrong scroll thumb pixels")
    else:
        require(art["thumb_ink"] is None, "Fitted menu included thumb ink")
    return {"selected": list(fill), "surface": list(surface), "focus_pixels": ring_pixels,
            "mark_pixels": marks, "empty_mark_pixels": empty_marks, "label_pixels": labels,
            "face_contrast": contrast(surface, fill), "mark_contrast": mark_contrast,
            "thumb": None if thumb_pixel is None else list(thumb_pixel)}


def window_phases(profile):
    return {"ready", "route", "menu", "closed", "returned"} | {
        f"{phase}/{shot}" for phase in ("before", "after") for shot in profile.shots
    }


def move(app, target):
    current = settle(app)
    order = tuple(LABELS)
    start, finish = (order.index(item) for item in (current["interaction"]["focused"], target))
    for _ in range(abs(finish - start)):
        current = key(app, "down" if finish > start else "up")
    focus(current, target)
    return current


def action(app, name, expected):
    response = app.wire("Action.key", {"key": name})
    require(response.get("changed") is expected, f"Expected {name} changed={expected}, got {response}")
    return settle(app)


def expect_blocked(app, target, checked, *, disabled):
    for name in ("enter", "space"):
        observation(action(app, name, False), target, checked)
    direct = app.wire("Action.activate", {"id": target}, expected_error="disabled") if disabled else None
    return {"focus": target, "enter": False, "space": False, "direct": direct}


def slot_bounds(frame, target):
    return {part: layout(frame, target + part)["bounds"] for part in ("", "-face", "-label", "-leading")}


def watch_end(app, profile, original):
    # A focus reveal can finish one frame before the measured thumb moves.
    from time import monotonic, sleep

    deadline = monotonic() + 2
    while True:
        frame = app.dom()
        focus(frame, LOCKED)
        try:
            scrolled = geometry(frame, profile, LOCKED)
            return verify_scroll(original, scrolled), scrolled
        except RuntimeError:
            if monotonic() >= deadline:
                raise
        sleep(0.02)


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
            stage["name"] = "initial"
            open_frame = open_menu(app)
            sample(app, "menu")
            initial = geometry(open_frame, profile, RECENT)
            observations = {"open": observation(open_frame, RECENT, INITIAL)}
            blocked = {"checked_single": expect_blocked(app, RECENT, INITIAL, disabled=False)}
            shots = {}
            scroll = None

            def shot(frame, focused, selected, unchecked, targets, tag):
                stage["name"] = tag
                state = observation(frame, focused, targets)
                measured = geometry(frame, profile, focused)
                require(selected in targets and unchecked not in targets, "Wrong sample choices")
                art = artwork(frame, profile, focused, selected, unchecked)
                sample(app, f"before/{tag}")
                image = capture(app, plan, tag)
                sample(app, f"after/{tag}")
                shots[tag] = {"state": state, "geometry": measured, "art": art, "image": image,
                              "pixels": paint(directory / image["name"], profile, measured, art)}

            if profile_name != "vibrant":
                frame = move(app, NAME)
                shot(frame, NAME, RECENT, NAME, INITIAL, profile.shots[0])

            current = INITIAL
            if profile_name == "standard":
                stage["name"] = "single-choice"
                move(app, OWNER)
                blocked["disabled"] = expect_blocked(app, OWNER, current, disabled=True)
                move(app, NAME)
                observations["sort-close"] = observation(action(app, "enter", True), OPENER)
                reopened = open_menu(app)
                observations["sort-reopen"] = observation(reopened, RECENT, AFTER_SORT)
                move(app, COMPACT)
                observations["density-close"] = observation(action(app, "space", True), OPENER)
                reopened = open_menu(app)
                observations["density-reopen"] = observation(reopened, RECENT, AFTER_DENSITY)
                current = AFTER_DENSITY
            else:
                stage["name"] = "show-last-choice"
                frame = key(app, "end")
                focus(frame, LOCKED)
                if profile.compact:
                    shift, end = watch_end(app, profile, initial)
                    scroll = {"translated_dp": shift, "end": end}
                blocked["disabled"] = expect_blocked(app, LOCKED, current, disabled=True)

            stage["name"] = "multiple-choice"
            size_frame = move(app, SIZE)
            before = slot_bounds(size_frame, SIZE)
            frame = action(app, "enter", True)
            current = current | {SIZE}
            after = slot_bounds(frame, SIZE)
            require(before == after, "Choosing SIZE displaced its field or passive slot")
            observations["size-on"] = observation(frame, SIZE, current)
            move(app, KIND)
            frame = action(app, "space", True)
            current = current - {KIND}
            observations["kind-off"] = observation(frame, KIND, current)

            if profile_name != "standard":
                frame = key(app, "end")
                shot(frame, LOCKED, SIZE, KIND, current, profile.shots[-1])

            stage["name"] = "close-and-reopen"
            if profile_name == "standard":
                observations["tab-close"] = observation(key(app, "tab"), OUTSIDE)
                reopened = open_menu(app)
                observations["tab-reopen"] = observation(reopened, RECENT, current)
            observations["escape-close"] = observation(key(app, "escape"), OPENER)
            reopened = open_menu(app)
            observations["escape-reopen"] = observation(reopened, RECENT, current)
            observations["final-close"] = observation(key(app, "escape"), OPENER)
            sample(app, "closed")
            stage["name"] = "return-launcher"
            focus_by_tab(app, BACK)
            returned = key(app, "enter")
            focus(returned, SHOW)
            absent(returned, OPENER)
            absent(returned, PANEL)
            sample(app, "returned")

    require(app.proc.returncode is not None, "The owned Gallery process was not reaped")
    return ({"controls": configured, "initial": initial, "observations": observations,
             "returned_focus": returned["interaction"]["focused"], "blocked": blocked, "scroll": scroll,
             "size_slots": {"before": before, "after": after},
             "captures": shots},
            {"profile": profile_name, "pid": app.proc.pid, "reaped": True,
             "argv": console_args(profile.width, profile.height), "visible_windows": windows})


def verify_play(record, profile_name, directory, files):
    profile = PROFILES[profile_name]
    require(record["controls"] == expected_controls(profile) and record["returned_focus"] == SHOW,
            "Profile settings or launcher focus differ")
    verify_geometry(record["initial"], profile, RECENT)
    standard = profile_name == "standard"
    expected_states = {
        "open": (RECENT, INITIAL),
        "size-on": (SIZE, STANDARD_CHECKS if standard else VIBRANT_FINAL),
        "kind-off": (KIND, STANDARD_FINAL if standard else TAIL_FINAL),
        "escape-close": (OPENER, None),
        "escape-reopen": (RECENT, STANDARD_FINAL if standard else TAIL_FINAL),
        "final-close": (OPENER, None),
    }
    if standard:
        expected_states.update({
            "sort-close": (OPENER, None), "sort-reopen": (RECENT, AFTER_SORT),
            "density-close": (OPENER, None), "density-reopen": (RECENT, AFTER_DENSITY),
            "tab-close": (OUTSIDE, None), "tab-reopen": (RECENT, STANDARD_FINAL),
        })
    observations = record["observations"]
    require(set(observations) == set(expected_states), "Choice transition receipt differs")
    for name, (target, checked) in expected_states.items():
        verify_observation(observations[name], target, checked)
    expected_blocked = {"checked_single": {"focus": RECENT, "enter": False, "space": False, "direct": None},
                        "disabled": {"focus": OWNER if standard else LOCKED, "enter": False,
                                     "space": False, "direct": "disabled"}}
    require(record["blocked"] == expected_blocked, "Disabled or checked single activation differs")
    slots = record["size_slots"]
    require(set(slots["before"]) == {"", "-face", "-label", "-leading"}
            and slots["before"] == slots["after"], "Checked artwork changed the row geometry")
    if profile.compact:
        scroll = record["scroll"]
        verify_geometry(scroll["end"], profile, LOCKED)
        require(verify_scroll(record["initial"], scroll["end"]) == scroll["translated_dp"],
                "Measured scroll receipt differs")
    else:
        require(record["scroll"] is None, "Wide profile supplied an overflow receipt")

    shots = record["captures"]
    require(set(shots) == set(profile.shots), "Wrong capture inventory")
    initial = (NAME, RECENT, NAME, INITIAL)
    tail = (LOCKED, SIZE, KIND, TAIL_FINAL)
    specs = ((initial, tail) if profile.compact else (initial,) if standard else (tail,))
    for tag, (focused, selected, unchecked, checked) in zip(profile.shots, specs, strict=True):
        shot = shots[tag]
        path = directory / f"{tag}.png"
        verify_observation(shot["state"], focused, checked)
        verify_geometry(shot["geometry"], profile, focused)
        require(tuple(shot["art"][key] for key in ("focus", "selected", "unchecked")) == (focused, selected, unchecked),
                f"Wrong {tag} paint sample targets")
        require(shot["image"] == {"name": path.name, "format": "png", "width": profile.width,
                                  "height": profile.height, "scale": 1,
                                  "sha256": files[f"{directory.name}/{path.name}"]}
                and shot["pixels"] == paint(path, profile, shot["geometry"], shot["art"]),
                f"Original PNG receipt disagrees for {tag}")


def readback(root, evidence_sha, source_sha, binary_sha, source_version):
    files = pins(root, root / "SHA256SUMS", evidence_sha)
    repeats = ("1", "2")
    expected = {REPORT} | {f"repeat-{n}/{tag}.png" for n in repeats for profile in PROFILES.values() for tag in profile.shots}
    actual = {path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file()}
    require(set(files) == expected and actual == expected | {"SHA256SUMS"}, "Unexpected evidence inventory")
    report = json.loads((root / REPORT).read_text())
    require(report["schema"] == SCHEMA and report["transports"] == TRANSPORTS
            and report["source"]["manifest_sha256"] == source_sha and report["source"]["version"] == source_version
            and type(report["source"]["file_count"]) is int and report["source"]["file_count"] >= len(REQUIRED_SOURCE)
            and report["binaries"]["manifest_sha256"] == binary_sha
            and set(report["binaries"]["files"]) == set(BINARIES)
            and all(SHA.fullmatch(value) for value in report["binaries"]["files"].values()),
            "Source or executable pin differs")
    replays = report["replays"]
    require(set(replays) == set(repeats) and replays["1"] == replays["2"]
            and set(replays["1"]) == set(PROFILES), "The repeats differ in geometry, semantics or PNG bytes")
    captures = replays["1"]
    standard = captures["standard"]["captures"][PROFILES["standard"].shots[0]]["pixels"]
    vibrant = captures["vibrant"]["captures"][PROFILES["vibrant"].shots[0]]["pixels"]
    compact = [item["pixels"] for item in captures["compact"]["captures"].values()]
    require(tuple(standard["selected"]) == PROFILES["vibrant"].surface
            and len({tuple(standard["selected"]), tuple(vibrant["selected"]),
                     *(tuple(item["selected"]) for item in compact)}) == 3,
            "Expected the published Standard selected fill and distinct Vibrant light/dark fills")
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
    return {"schema": SCHEMA, "source_sha256": source_sha, "binary_sha256": binary_sha, "evidence_sha256": evidence_sha,
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
    return readback(out, digest(manifest), args.source_sha, args.binary_sha, args.source_version)


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
    parser.add_argument("--source-version", choices=("0.1.75", "0.1.76"), default="0.1.76")
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
        result = readback(args.readback, args.evidence_sha, args.source_sha, args.binary_sha, args.source_version)
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit(f"Vertical choice menu native replay stopped. {error}")
