#!/usr/bin/env python3
"""Verify the adaptive navigation views on three fixed native headless windows."""

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

PREFIX = "adaptive-navigation-demo-"
SHOW, BACK, SHELL, LANDMARK, PAGE, TITLE, EDITOR = (
    PREFIX + part for part in ("show", "back", "scaffold", "bar", "page", "title", "draft")
)
SCROLL = LANDMARK + "-scroll"
CONTENT = SHELL + "-content"
ITEMS = tuple(PREFIX + part for part in ("home", "explore", "saved", "updates", "settings"))
HOME, EXPLORE, SAVED, UPDATES, SETTINGS = ITEMS
LABELS = ("首頁", "探索路線", "收藏", "最新消息", "設定")
CONTROLS = tuple(PREFIX + part for part in ("theme", "rtl", "large"))


@dataclass(frozen=True)
class Profile:
    width: int
    height: int
    mode: str
    image: str

    @property
    def rail(self):
        return self.mode == "rail"


PROFILES = {
    "compact": Profile(320, 620, "vertical", "compact-320-light-vertical-bar"),
    "medium": Profile(600, 620, "horizontal", "medium-light-horizontal-bar"),
    "rail": Profile(840, 300, "rail", "wide-dark-rtl-200-short-rail"),
}
SAMPLES = {"ready", "opened", "before_capture", "after_capture", "returned"}
TRANSPORTS = ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"]
BINARIES = ("surface-material-gallery", "surface-devtools")
REPORT = "native-adaptive-nav-report.json"
SCHEMA = "surface.test.adaptive-navigation-scaffold.native.v1"
SHA = re.compile(r"[0-9a-f]{64}")
REQUIRED_SOURCE = {
    "examples/material-gallery/src/adaptive_navigation_demo.rs",
    "crates/surface-material/src/adaptive_navigation_scaffold.rs",
    "crates/surface-material/src/wide_navigation_rail.rs",
    "crates/surface-material/src/flexible_navigation_bar.rs",
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
    require(version == "0.1.74", "Source is not Surface v0.1.74")
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


def right(rect):
    return rect["x"] + rect["width"]


def bottom(rect):
    return rect["y"] + rect["height"]


def near(a, b):
    return abs(a - b) <= 0.1


def equal_box(a, b):
    return a.keys() == b.keys() and all(near(a[field], b[field]) for field in a)


def color(value):
    require(isinstance(value, list) and len(value) == 4
            and all(type(channel) is int and 0 <= channel <= 255 for channel in value), "Expected RGBA8")
    return tuple(value)


def boxes(frame, profile):
    result = {name: visible(frame, target) for name, target in (
        ("shell", SHELL), ("landmark", LANDMARK), ("content", CONTENT), ("page", PAGE),
    )}
    if profile.rail:
        result["scroll"] = visible(frame, SCROLL)
    else:
        require(find(frame, SCROLL)[1] is None, "A bottom bar must not mount a rail scroller")
    verify_boxes(result, profile)
    return result


def verify_boxes(record, profile):
    shell, nav, page = (record[key] for key in ("shell", "landmark", "page"))
    require(equal_box(shell, {"x": 0, "y": 0, "width": profile.width, "height": profile.height})
            and equal_box(page, record["content"]), "Viewport, content or scaffold dimensions differ")
    if profile.rail:
        require(near(nav["width"], 96) and near(right(nav), profile.width)
                and near(nav["y"], 0) and near(nav["height"], profile.height)
                and near(page["x"], 0) and near(page["y"], 0)
                and near(page["height"], profile.height) and near(right(page), nav["x"])
                and equal_box(record["scroll"], nav), f"RTL leading rail or independent page differs: {record}")
    else:
        require(near(nav["x"], 0) and near(nav["width"], profile.width)
                and near(bottom(nav), profile.height) and nav["height"] >= 64
                and near(page["x"], 0) and near(page["y"], 0) and page["height"] > 0
                and near(page["width"], profile.width) and near(bottom(page), nav["y"]),
                f"Bottom bar and adjacent page geometry differs: {record}")


def expected_semantics(current):
    return {
        "landmarks": [[LANDMARK, "主要導覽"]],
        "marked_current_pages": [current],
        "title": ["Heading", LABELS[ITEMS.index(current)]],
        "destinations": {item: {"role": "Button", "name": name, "current_page": item == current,
                               "disabled": item == EXPLORE}
                         for item, name in zip(ITEMS, LABELS, strict=True)},
    }


def verify_semantics(observed, current):
    require(observed == expected_semantics(current)
            and all(type(item[part]) is bool for item in observed["destinations"].values()
                    for part in ("current_page", "disabled")),
            "Expected exactly one named navigation landmark, disabled destination, and current page")


def semantics(frame, current):
    dom = frame["dom"]["nodes"]
    heading = node(frame, TITLE)
    result = {
        "landmarks": [[entry.get("stable_id"), entry["name"]] for entry in dom if entry["role"] == "Navigation"],
        "marked_current_pages": [entry.get("stable_id") for entry in dom if entry.get("current_page") is True],
        "title": [heading["role"], heading["name"]],
        "destinations": {
            item: {key: node(frame, item)[key] for key in ("role", "name", "current_page", "disabled")}
            for item in ITEMS
        },
    }
    verify_semantics(result, current)
    return result


def indexed_child(dom, owner, description):
    index = owner.get("first_child")
    require(type(index) is int and 0 <= index < len(dom), f"Missing {description}")
    return index


def passive_group(part):
    return part["role"] == "Group" and not part.get("name") and not part.get("stable_id")


def parts(frame, item, profile):
    dom, measured = frame["dom"]["nodes"], frame["layout"]["nodes"]
    owner = node(frame, item)
    hit = visible(frame, item, 48)
    if profile.rail:
        # The rail's standard passive indicator and caption have no stable IDs.
        pill_index = indexed_child(dom, owner, "rail indicator")
        caption_index = dom[pill_index]["next_sibling"]
    else:
        pill_index, _ = find(frame, item + "-indicator")
        caption_index, _ = find(frame, item + "-label")
    indices = (pill_index, caption_index)
    require(all(type(index) is int and 0 <= index < len(dom) for index in indices), "Missing navigation artwork")
    pill, caption = (dom[index] for index in indices)
    icon_index = indexed_child(dom, pill, "passive navigation icon")
    art = [pill_index, caption_index, icon_index]
    icon_region = dom[icon_index]
    require(passive_group(icon_region), f"{item}: the icon region carries an independent role, name or ID")
    if profile.mode == "horizontal":
        contour_index = icon_index
    else:
        # VectorIcon::view adds one passive clip around its 24 dp contour.
        require(not icon_region.get("fill_contour"), f"{item}: expected the clipped vector wrapper")
        contour_index = indexed_child(dom, icon_region, "vector contour inside the clip")
        require(dom[contour_index].get("next_sibling") is None
                and equal_box(measured[icon_index]["bounds"], measured[contour_index]["bounds"]),
                f"{item}: vector contour does not occupy its expected clip")
        art.append(contour_index)
    contour = dom[contour_index]
    require(passive_group(contour) and contour.get("fill_contour") and contour.get("first_child") is None,
            f"{item}: the expected passive contour leaf is missing")
    require(caption.get("text") == LABELS[ITEMS.index(item)] and caption["role"] == "Text",
            f"{item}: visible label changed")
    for index in art:
        require(same_box(measured[index]["clip"], measured[index]["bounds"]), f"{item}: clipped artwork")
    return (
        {"hit": hit, "pill": measured[pill_index]["bounds"], "label": measured[caption_index]["bounds"],
         "icon": measured[icon_index]["bounds"], "text": caption["text"]},
        {"selected": pill["style"]["background"], "label": caption["style"]["foreground"],
         "icon": contour["fill_contour"]["color"], "focus": (pill if profile.rail else owner)["focus_color"]},
    )


def verify_parts(record, profile, item):
    hit, pill, icon, label = (record[part] for part in ("hit", "pill", "icon", "label"))
    require(hit["width"] >= 48 and hit["height"] >= 48
            and near(icon["width"], 24) and near(icon["height"], 24)
            and same_box(hit, pill) and same_box(hit, label) and same_box(pill, icon)
            and record["text"] == LABELS[ITEMS.index(item)] and label["width"] > 0, "Part or hit geometry differs")
    require(abs((pill["x"] + pill["width"] / 2) - (hit["x"] + hit["width"] / 2)) <= 0.51,
            "The visible pill is not centered on its target")
    if profile.mode == "horizontal":
        require(near(hit["width"], 112) and pill["width"] > 56 and near(pill["height"], 40)
                and near(label["height"], 16) and same_box(pill, label)
                and near(label["x"] - right(icon), 4) and near(icon["x"] - pill["x"], 16),
                "The medium variant lost its fixed target or leading horizontal icon")
    else:
        require(near(pill["width"], 56) and near(pill["height"], 32)
                and label["height"] >= (31.9 if profile.rail else 15.9)
                and near(label["y"] - bottom(pill), 4), "The vertical icon/label stack differs")
        if profile.rail:
            require(near(hit["width"], 96), "Rail destination does not fill the rail")


def rail_reveal(before, after, scroll):
    require(near(before["x"], after["x"]) and near(before["width"], after["width"])
            and near(before["height"], after["height"])
            and bottom(before) > bottom(scroll) + 10 and before["y"] > after["y"] + 10
            and same_box(scroll, after), f"Focus did not reveal the final rail destination: {before} -> {after}")


def paint(path, profile, navigation, selected, focused, palette):
    width, height, rgba = read_png(path.read_bytes())
    require((width, height) == (profile.width, profile.height), "Unexpected screenshot viewport")
    surface, fill, icon, label, ring = (color(palette[name]) for name in ("surface", "selected", "icon", "label", "focus"))
    require(all(value[3] == 255 for value in (surface, fill, icon, label, ring))
            and len({surface, fill, icon}) == 3 and ring != surface and label != surface,
            "Surface, selection, ink or focus colors do not contrast")

    def pixel(x, y):
        x, y = math.floor(x), math.floor(y)
        require(0 <= x < width and 0 <= y < height, "A paint probe is outside the image")
        return tuple(rgba[y][4*x:4*x+4])

    def ink_votes(item, ink, paper):
        channel = max(range(3), key=lambda index: abs(ink[index] - paper[index]))
        require(ink[channel] != paper[channel], "Ink has no difference from its background")
        return sum(sample[3] == 255 and abs(sample[channel] - ink[channel]) < abs(sample[channel] - paper[channel])
                   for y in range(math.ceil(item["y"]), math.floor(bottom(item)))
                   for x in range(math.ceil(item["x"]), math.floor(right(item)))
                   for sample in (pixel(x, y),))

    pill = selected["pill"]
    probes = [pixel(navigation["x"] + 3, navigation["y"] + 2),
              pixel(pill["x"] + 6, pill["y"] + pill["height"] / 2),
              pixel(pill["x"] + 2, pill["y"] + 2)]
    require(probes == [surface, fill, surface], f"Actual background, pill or corner pixels differ: {probes}")

    focus_rect = focused["pill" if profile.rail else "hit"]
    middle = math.floor(focus_rect["x"] + focus_rect["width"] / 2)
    ring_votes = sum(pixel(x, focus_rect["y"] + dy) == ring
                     for x in range(middle - 5, middle + 6) for dy in (0, 1, 2))
    labels = ink_votes(selected["label"], label, fill if profile.mode == "horizontal" else surface)
    icons = ink_votes(selected["icon"], icon, fill)
    require(ring_votes >= 8 and labels >= 12 and icons >= 12,
            f"Missing rendered focus, label or icon pixels: {ring_votes}, {labels}, {icons}")
    return {"surface_rgba": list(probes[0]), "pill_rgba": list(probes[1]), "corner_rgba": list(probes[2]),
            "focus_pixels": ring_votes, "label_pixels": labels, "icon_pixels": icons}


def expected_keys(profile):
    if profile.rail:
        return [("down", SAVED, HOME), ("space", SAVED, SAVED),
                ("down", UPDATES, SAVED), ("down", SETTINGS, SAVED)]
    return [("tab", SAVED, HOME), ("enter", SAVED, SAVED), ("tab", UPDATES, SAVED)]


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


def play(directory, profile_name, gallery, cli):
    profile = PROFILES[profile_name]
    windows = {}
    stage = {"name": "open"}

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
            activate(app, "section-3")
            frame = activate(app, SHOW)
            focus(frame, BACK)
            sample(app, "opened")
            stage["name"] = "profile-controls"
            if profile.rail:
                for setting in CONTROLS:
                    activate(app, setting)
            focus_by_tab(app, HOME)
            frame = settle(app)
            focus(frame, HOME)
            stage["name"] = "initial-layout"
            measured = boxes(frame, profile)
            initial = semantics(frame, HOME)
            editor_before = layout(frame, EDITOR)["bounds"]
            before = layout(frame, SETTINGS)["bounds"] if profile.rail else None
            history = []
            for action, target, current in expected_keys(profile):
                stage["name"] = f"key-{action}-{target}"
                frame = key(app, action)
                focus(frame, target)
                history.append({"key": action, "focus": frame["interaction"]["focused"],
                                "semantics": semantics(frame, current)})
            stage["name"] = "disabled-and-page"
            denied = app.wire("Action.activate", {"id": EXPLORE}, expected_error="disabled")
            frame = settle(app)
            _, focused_id, selected_id = expected_keys(profile)[-1]
            focus(frame, focused_id)
            require(boxes(frame, profile) == measured and layout(frame, EDITOR)["bounds"] == editor_before,
                    "Keyboard focus moved or replaced the independent page content")
            final = semantics(frame, selected_id)
            stage["name"] = "selected-artwork"
            selected, palette = parts(frame, selected_id, profile)
            verify_parts(selected, profile, selected_id)
            stage["name"] = "focused-artwork"
            focused, focus_ink = parts(frame, focused_id, profile)
            verify_parts(focused, profile, focused_id)
            palette["surface"] = node(frame, LANDMARK)["style"]["background"]
            palette["focus"] = focus_ink["focus"]
            reveal = None
            if profile.rail:
                stage["name"] = "rail-reveal"
                rail_reveal(before, focused["hit"], measured["scroll"])
                reveal = {"before": before, "after": focused["hit"]}
            stage["name"] = "screenshot-transports"
            sample(app, "before_capture")
            image = capture(app, plan, profile.image)
            sample(app, "after_capture")
            stage["name"] = "paint"
            pixels = paint(directory / image["name"], profile, measured["landmark"], selected, focused, palette)
            stage["name"] = "return-focus"
            focus_by_tab(app, BACK)
            returned = key(app, "enter")
            focus(returned, SHOW)
            require(find(returned, LANDMARK)[1] is None, "Navigation remains mounted after Back")
            sample(app, "returned")

    require(app.proc.returncode is not None, "The owned Gallery process was not reaped")
    return (
        {"mode": profile.mode, "layout": measured, "initial": initial, "keys": history,
         "selected": final, "capture_focus": focused_id, "disabled_activation": denied,
         "return_focus": returned["interaction"]["focused"], "selected_art": selected, "focused_art": focused,
         "rail_reveal": reveal, "page_editor": {"before": editor_before, "after": layout(frame, EDITOR)["bounds"]},
         "palette": palette, "pixels": pixels, "image": image},
        {"profile": profile_name, "pid": app.proc.pid, "reaped": True,
         "argv": console_args(profile.width, profile.height), "visible_windows": windows},
    )


def verify_play(record, profile, artifact, pinned_png):
    measured = record["layout"]
    verify_boxes(measured, profile)
    verify_semantics(record["initial"], HOME)
    events = expected_keys(profile)
    require(len(record["keys"]) == len(events), "Missing keyboard events")
    for event, (action, focus_id, current) in zip(record["keys"], events, strict=True):
        require(event["key"] == action and event["focus"] == focus_id, "Keyboard focus or disabled skip differs")
        verify_semantics(event["semantics"], current)
    _, focused_id, selected_id = events[-1]
    verify_semantics(record["selected"], selected_id)
    require(record["capture_focus"] == focused_id and focused_id != selected_id
            and record["mode"] == profile.mode and record["disabled_activation"] == "disabled"
            and record["return_focus"] == SHOW, "Focus, semantic activation or return differs")
    for art, ident in ((record["selected_art"], selected_id), (record["focused_art"], focused_id)):
        verify_parts(art, profile, ident)
        require(same_box(measured["landmark"], art["hit"]), "A destination target is not inside the navigation clip")
    require(record["page_editor"]["before"] == record["page_editor"]["after"],
            "Navigation moved the independent editor")
    reveal = record["rail_reveal"]
    if profile.rail:
        require(reveal["after"] == record["focused_art"]["hit"], "Rail reveal record differs from visible settings")
        rail_reveal(reveal["before"], reveal["after"], measured["scroll"])
    else:
        require(reveal is None, "A bar contains unexpected rail state")
    expected_image = {"name": profile.image + ".png", "width": profile.width, "height": profile.height,
                      "scale": 1, "format": "png", "sha256": pinned_png}
    require(record["image"] == expected_image, "Screenshot metadata or original digest differs")
    require(record["pixels"] == paint(artifact, profile, measured["landmark"], record["selected_art"],
                                       record["focused_art"], record["palette"]), "Original PNG and paint receipt differ")


def readback(root, evidence_sha, source_sha, binary_sha):
    root = root.resolve(strict=True)
    files = pins(root, root / "SHA256SUMS", evidence_sha)
    expected = {REPORT, *(f"repeat-{index}/{profile.image}.png"
                          for index in ("1", "2") for profile in PROFILES.values())}
    actual = {path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file()}
    require(set(files) == expected and actual == expected | {"SHA256SUMS"}, "Unexpected portable evidence inventory")
    report = json.loads((root / REPORT).read_text())
    require(report["schema"] == SCHEMA and report["transports"] == TRANSPORTS
            and report["source"]["manifest_sha256"] == source_sha and SHA.fullmatch(source_sha or "")
            and report["source"]["version"] == "0.1.74" and type(report["source"]["file_count"]) is int
            and report["source"]["file_count"] >= len(REQUIRED_SOURCE)
            and report["binaries"]["manifest_sha256"] == binary_sha and SHA.fullmatch(binary_sha or "")
            and set(report["binaries"]["files"]) == set(BINARIES)
            and all(SHA.fullmatch(value) for value in report["binaries"]["files"].values()),
            "Reviewed portable source or executable pins differ")
    replays = report["replays"]
    require(set(replays) == {"1", "2"} and replays["1"] == replays["2"] and set(replays["1"]) == set(PROFILES),
            "Repeated geometry, keys, pixels or PNG digests differ")
    require(replays["1"]["compact"]["palette"]["surface"] == replays["1"]["medium"]["palette"]["surface"]
            and replays["1"]["compact"]["palette"]["surface"] != replays["1"]["rail"]["palette"]["surface"],
            "Light bar and dark rail profiles disagree with their intended palettes")
    for index, records in replays.items():
        for name, record in records.items():
            profile = PROFILES[name]
            relative = f"repeat-{index}/{profile.image}.png"
            verify_play(record, profile, root / relative, files[relative])
    processes = report["processes"]
    require(len(processes) == 6 and len({entry["pid"] for entry in processes}) == 6
            and {(entry["replay"], entry["profile"]) for entry in processes}
            == {(index, profile) for index in replays for profile in PROFILES}, "Missing owned process or profile")
    for entry in processes:
        profile = PROFILES[entry["profile"]]
        require(type(entry["pid"]) is int and entry["pid"] > 0 and entry["reaped"] is True
                and entry["argv"] == console_args(profile.width, profile.height)
                and set(entry["visible_windows"]) == SAMPLES
                and all(type(count) is int and count == 0 for count in entry["visible_windows"].values()),
                "Owned Gallery process, exact headless args or window samples differ")
    return {"schema": SCHEMA, "evidence_sha256": evidence_sha, "source_sha256": source_sha,
            "binary_sha256": binary_sha, "scenes": {name: {"viewport": [profile.width, profile.height],
                                                        "placement": profile.mode}
                                                  for name, profile in PROFILES.items()},
            "original_pngs": len(expected) - 1, "replays": 2, "transports": TRANSPORTS,
            "window_samples": len(processes) * len(SAMPLES), "visible_windows": 0, "reaped": len(processes),
            "native_resize_covered": False}


def run(args):
    require(sys.platform == "darwin", "Native window proof requests macOS")
    gallery, cli, inputs = check_inputs(args)
    out = args.out.resolve()
    scratch = Path("/tmp").resolve()
    require(out.is_relative_to(scratch) and out != scratch, "A fresh evidence directory under /tmp is required")
    out.mkdir(mode=0o700)
    report = {"schema": SCHEMA, **inputs, "transports": TRANSPORTS, "replays": {}, "processes": []}
    paths = [REPORT]
    for index in ("1", "2"):
        directory = out / f"repeat-{index}"
        directory.mkdir()
        report["replays"][index] = {}
        for name in PROFILES:
            record, process = play(directory, name, gallery, cli)
            report["replays"][index][name] = record
            report["processes"].append({"replay": index, **process})
            paths.append(f"repeat-{index}/{record['image']['name']}")
    require((gallery, cli, inputs) == check_inputs(args), "Reviewed input bytes changed during the replay")
    (out / REPORT).write_text(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n")
    manifest = out / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(out / name)}  {name}\n" for name in sorted(paths)))
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
        sys.exit(f"Adaptive navigation native replay stopped. {error}")
