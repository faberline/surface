#!/usr/bin/env python3
"""Record and check the headless horizontal Navigation Bar."""

import argparse
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
from render_helpers import activate, capture, find, focus, key, node, settle, visible
from window_check import onscreen_windows

PREFIX = "navigation-bar-demo-"
SHOW = "navigation-bar-medium-demo-show"
BACK, PREVIEW, BAR, SCROLL, HEADING = (
    PREFIX + part for part in ("back", "preview", "bar", "scroll", "page-title")
)
ITEMS = tuple(PREFIX + part for part in ("home", "explore", "saved", "updates", "settings"))
HOME, EXPLORE, SAVED, UPDATES, SETTINGS = ITEMS
NAMES = ("首頁", "探索路線", "收藏的地點", "最新消息", "設定")
LABELS = ("首頁", "探索路線", "收藏", "最新消息", "設定")
CONTROLS = tuple(PREFIX + part for part in ("theme", "rtl", "large-text", "count-3", "count-4", "count-5"))
PROFILES = {
    "wide": (839, 900, False, SAVED, "wide-light-saved"),
    "medium": (720, 900, True, UPDATES, "medium-dark-rtl-200-updates"),
}
SAMPLES = {"ready", "opened", "before_capture", "after_capture", "returned"}
TRANSPORTS = ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"]
BINARIES = ("surface-material-gallery", "surface-devtools")
REPORT = "native-horizontal-nav-report.json"
SCHEMA = "surface.test.horizontal-flexible-nav.native.v1"
SHA = re.compile(r"[0-9a-f]{64}")
AFTER_SELECTED = {SAVED: UPDATES, UPDATES: SETTINGS}


def pins(root, manifest, expected):
    require(SHA.fullmatch(expected or "") and manifest.is_file() and digest(manifest) == expected,
            f"Reviewed manifest digest differs: {manifest}")
    entries = {}
    for line in manifest.read_text().splitlines():
        value, name = line.split("  ", 1)
        relative = Path(name)
        path = root / relative
        require(SHA.fullmatch(value) and not relative.is_absolute() and ".." not in relative.parts
                and relative.as_posix() == name and name not in entries and name != "."
                and path.is_file() and not path.is_symlink()
                and path.resolve().is_relative_to(root) and digest(path) == value,
                f"Invalid pinned file: {name}")
        entries[name] = value
    require(entries, "Empty manifest")
    return entries


def check_source(args):
    source = args.source.resolve(strict=True)
    entries = pins(source, args.source_manifest, args.source_sha)
    local = {path.relative_to(source).as_posix() for path in source.rglob("*") if path.is_file()}
    require(set(entries) == local, "Reviewed source inventory is incomplete")
    version = tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == "0.1.73", "Source is not Surface v0.1.73")
    return {"version": version, "manifest_sha256": args.source_sha, "file_count": len(entries)}


def check_inputs(args):
    source = check_source(args)
    manifest = args.binary_manifest.resolve(strict=True)
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


def color(value):
    require(isinstance(value, list) and len(value) == 4
            and all(type(channel) is int and 0 <= channel <= 255 for channel in value),
            f"Expected RGBA8, got {value}")
    return tuple(value)


def take_layout(frame, count):
    dom, measured = frame["dom"]["nodes"], frame["layout"]["nodes"]
    items = {}
    for item in ITEMS[:count]:
        indicator_id, label_id = item + "-indicator", item + "-label"
        indicator = node(frame, indicator_id)
        icon_index = indicator["first_child"]
        require(type(icon_index) is int and 0 <= icon_index < len(dom), f"No icon child: {item}")
        icon = dom[icon_index]
        icon_box = measured[icon_index]["bounds"]
        require(icon.get("fill_contour") and not icon.get("name") and not icon.get("stable_id")
                and same_box(measured[icon_index]["clip"], icon_box), f"Wrong passive icon: {item}")
        items[item] = {"hit": visible(frame, item, 48), "pill": visible(frame, indicator_id),
                       "label": visible(frame, label_id), "icon": icon_box,
                       "text": node(frame, label_id)["text"]}
    return {"preview": visible(frame, PREVIEW), "bar": visible(frame, BAR, 48),
            "scroll": visible(frame, SCROLL), "items": items}


def verify_layout(layout, profile, count):
    width, height, rtl, _, _ = PROFILES[profile]
    preview, bar, scroll, items = (layout[part] for part in ("preview", "bar", "scroll", "items"))
    require(near(preview["x"], 0) and near(preview["width"], width) and near(bottom(preview), height)
            and near(bar["x"], 0) and near(bar["width"], width) and near(bottom(bar), height)
            and near(bottom(scroll), bar["y"]) and scroll["height"] > 0,
            f"{profile}: bar or scroll region lost its viewport edge")
    require(set(items) == set(ITEMS[:count]), f"{profile}: incorrect item inventory")
    tallest = max(item["label"]["height"] for item in items.values())
    require(near(bar["height"], max(64, max(24, tallest) + 40))
            and (tallest >= 63.9 if rtl else near(tallest, 16)),
            f"{profile}: tallest label and bar height disagree")
    start = (width - 112 * count) / 2
    physical = sorted((item["hit"] for item in items.values()), key=lambda hit: hit["x"])
    require(all(near(right(left), right_item["x"]) for left, right_item in zip(physical, physical[1:])),
            f"{profile}: adjacent hit cells have a gap or overlap: {physical}")
    for index, item in enumerate(ITEMS[:count]):
        hit, pill, icon, label = (items[item][part] for part in ("hit", "pill", "icon", "label"))
        col = count - 1 - index if rtl else index
        expected_x = start + 112 * col
        # Taffy snaps flow roots to logical pixels. The centered 839 dp bar
        # permits a half-pixel offset, while each 112 dp cell stays exact.
        require(abs(hit["x"] - expected_x) <= 0.51 and near(hit["width"], 112)
                and near(hit["y"], bar["y"]) and near(hit["height"], bar["height"])
                and same_box(bar, hit),
                f"{profile}/{count}/{item}: wrong 112 dp target or direction; "
                f"hit={hit!r}; bar={bar!r}; expected_x={expected_x}")
        require(items[item]["text"] == LABELS[index]
                and 0 < label["width"] <= 52.1 and label["height"] >= (31.9 if rtl else 15.9)
                and near(pill["x"] + pill["width"] / 2, hit["x"] + 56)
                and near(pill["y"] + pill["height"] / 2, hit["y"] + hit["height"] / 2)
                and near(pill["width"], 60 + label["width"])
                and near(pill["height"], max(24, label["height"]) + 16)
                and same_box(hit, pill) and same_box(pill, icon) and same_box(pill, label),
                f"{profile}/{item}: wrong measured pill or label")
        if rtl:
            edge, gap, opposite = right(pill) - right(icon), icon["x"] - right(label), label["x"] - pill["x"]
        else:
            edge, gap, opposite = icon["x"] - pill["x"], label["x"] - right(icon), right(pill) - right(label)
        require(near(icon["width"], 24) and near(icon["height"], 24)
                and near(icon["y"] + 12, pill["y"] + pill["height"] / 2)
                and near(label["y"] + label["height"] / 2, pill["y"] + pill["height"] / 2)
                and near(edge, 16) and near(gap, 4) and near(opposite, 16),
                f"{profile}/{item}: wrong logical side or icon gap")


def expected_semantics(count, current):
    return {
        "landmark": ["Navigation", "主要導覽"],
        "heading": ["Heading", LABELS[ITEMS.index(current)]],
        "destinations": {
            item: {"role": "Button", "name": NAMES[index], "current_page": item == current,
                   "disabled": item == EXPLORE}
            for index, item in enumerate(ITEMS[:count])
        },
    }


def verify_semantics(payload, count, current):
    require(payload == expected_semantics(count, current) and all(
        type(item["current_page"]) is bool and type(item["disabled"]) is bool
        for item in payload["destinations"].values()
    ), "Navigation landmark, current page, names, disabled state or heading differs")


def semantics(frame, count, current):
    bar, heading = node(frame, BAR), node(frame, HEADING)
    payload = {
        "landmark": [bar["role"], bar["name"]],
        "heading": [heading["role"], heading["name"]],
        "destinations": {
            item: {part: match[part] for part in ("role", "name", "current_page", "disabled")}
            for item in ITEMS if (match := find(frame, item)[1]) is not None
        },
    }
    verify_semantics(payload, count, current)
    return payload


def palette(frame, current):
    face = node(frame, current + "-indicator")
    icon = frame["dom"]["nodes"][face["first_child"]]
    next_item = node(frame, AFTER_SELECTED[current])
    require(next_item.get("focus_outline_width") == 3, "Unexpected authored focus stroke")
    result = {"bar": node(frame, BAR)["style"]["background"],
              "fill": face["style"]["background"],
              "ink": node(frame, current + "-label")["style"]["foreground"],
              "focus": next_item["focus_color"]}
    require(color(icon["fill_contour"]["color"]) == color(result["ink"]),
            "Selected icon and label have different ink")
    return result


def paint(path, profile, layout, palette):
    width, height, _, current, _ = PROFILES[profile]
    image_w, image_h, rows = read_png(path.read_bytes())
    require((image_w, image_h) == (width, height), f"{path.name}: unexpected decoded size")
    bar, fill, ink, ring = (color(palette[part]) for part in ("bar", "fill", "ink", "focus"))
    require(len({bar, fill, ink}) == 3 and bar != ring
            and all(value[3] == 255 for value in (bar, fill, ink, ring)),
            "Expected distinct, opaque Material colors")

    def pixel(x, y):
        px, py = math.floor(x), math.floor(y)
        require(0 <= px < width and 0 <= py < height, "Pixel probe exceeds screenshot")
        return tuple(rows[py][4 * px:4 * px + 4])

    item = layout["items"][current]
    pill = item["pill"]
    x, y, mid = pill["x"], pill["y"], pill["y"] + pill["height"] / 2
    require(pixel(3, mid) == bar and pixel(x + 6, mid) == fill and pixel(x + 5, y + 2) == bar,
            f"{path.name}: bar, pill face or rounded corner does not paint; "
            f"actual={pixel(3, mid), pixel(x + 6, mid), pixel(x + 5, y + 2)}; "
            f"expected={bar, fill, bar}; pill={pill}")
    focus_hit = layout["items"][AFTER_SELECTED[current]]["hit"]
    focus_x, focus_y = focus_hit["x"] + focus_hit["width"] / 2, focus_hit["y"]
    stroke = tuple(pixel(focus_x, focus_y + offset) for offset in range(4))
    require(stroke == (ring, ring, ring, bar),
            f"{path.name}: expected a 3 dp focus stroke; actual={stroke}; expected={(ring, ring, ring, bar)}")

    def count(rect):
        channel = max(range(3), key=lambda part: abs(ink[part] - fill[part]))
        votes = 0
        for py in range(math.ceil(rect["y"]), math.floor(bottom(rect))):
            for px in range(math.ceil(rect["x"]), math.floor(right(rect))):
                sample = pixel(px, py)
                opacity = (sample[channel] - fill[channel]) / (ink[channel] - fill[channel])
                if (0.65 <= opacity <= 1 and sample[3] == 255
                        and all(abs(sample[i] - (fill[i] + opacity * (ink[i] - fill[i]))) <= 3
                                for i in range(3))):
                    votes += 1
        return votes

    icon_pixels, label_pixels = count(item["icon"]), count(item["label"])
    require(icon_pixels > 24 and label_pixels > 16, f"{path.name}: icon or label has too little visible ink")
    return {"icon_pixels": icon_pixels, "label_pixels": label_pixels, "rounded_corner_clear": True,
            "focus_stroke_dp": 3}


def tab_order(current):
    enabled = [item for item in ITEMS if item != EXPLORE]
    return [*CONTROLS, *enabled[:enabled.index(current) + 1]]


def console_args(width, height):
    return ["--headless", "--width", str(width), "--height", str(height)]


def guard_text(gallery, width, height):
    checks = [f'[ "${index}" = {shlex.quote(arg)} ]'
              for index, arg in enumerate(console_args(width, height), 1)]
    return ('#!/bin/sh\n[ "$#" -eq 5 ] && ' + " && ".join(checks)
            + ' || exit 71\n' + f'exec {shlex.quote(str(gallery))} "$@"\n')


def play(directory, profile, gallery, cli):
    width, height, rtl, current, image_name = PROFILES[profile]
    windows = {}
    layouts, initial, visited = {}, {}, []

    def sample(app, name):
        require(app.proc.poll() is None and name not in windows, "Invalid window observation")
        windows[name] = onscreen_windows(app.proc.pid)
        require(windows[name] == 0, f"{name}: a native window was visible")

    with TemporaryDirectory(prefix=".argv-", dir=directory) as tmp:
        guard = Path(tmp) / "headless-gallery"
        guard.write_text(guard_text(gallery, width, height))
        guard.chmod(0o700)
        plan = SimpleNamespace(out=directory, gallery=guard, cli=cli)
        with launch(plan, profile, width, height) as app:
            sample(app, "ready")
            activate(app, "section-3")
            focus_by_tab(app, SHOW)
            focus(key(app, "enter"), BACK)
            sample(app, "opened")
            if rtl:
                for setting in CONTROLS[:3]:
                    activate(app, setting)
            for count in (3, 4, 5):
                frame = activate(app, f"{PREFIX}count-{count}")
                layout = take_layout(frame, count)
                verify_layout(layout, profile, count)
                layouts[str(count)] = layout
                initial[str(count)] = semantics(frame, count, HOME)
            if settle(app)["interaction"]["focused"] != BACK:
                focus_by_tab(app, BACK)
            for target in tab_order(current):
                frame = key(app, "tab")
                focus(frame, target)
                state = semantics(frame, 5, HOME)
                visited.append({"focus": frame["interaction"]["focused"], "current_page": state})
            frame = key(app, "enter")
            focus(frame, current)
            final = semantics(frame, 5, current)
            after = take_layout(frame, 5)
            require(after == layouts["5"], "Selection changed bar or destination geometry")
            colors = palette(frame, current)
            denied = app.wire("Action.activate", {"id": EXPLORE}, expected_error="disabled")
            require(denied == "disabled", "Disabled destination was activated")
            focus(settle(app), current)
            semantics(settle(app), 5, current)
            # The selected pill has an intentional tint while it owns focus.
            # Read its base paint while the adjacent cell paints real focus.
            capture_frame = key(app, "tab")
            focus(capture_frame, AFTER_SELECTED[current])
            require(semantics(capture_frame, 5, current) == final and take_layout(capture_frame, 5) == after,
                    "Moving focus changed the selected page or bar layout")
            sample(app, "before_capture")
            image = capture(app, plan, image_name)
            sample(app, "after_capture")
            pixels = paint(directory / image["name"], profile, after, colors)
            focus_by_tab(app, BACK)
            returned = key(app, "enter")
            focus(returned, SHOW)
            require(find(returned, BAR)[1] is None, "The bar remained after Back")
            sample(app, "returned")

    require(app.proc.returncode is not None, "Owned Gallery process was not reaped")
    return (
        {"layouts": layouts, "initial": initial, "tabs": visited, "selected": final,
         "focus_after_enter": frame["interaction"]["focused"], "selected_layout": after,
         "focus_at_capture": capture_frame["interaction"]["focused"],
         "disabled_activation": denied, "return_focus": returned["interaction"]["focused"],
         "palette": colors, "pixels": pixels, "image": image},
        {"profile": profile, "pid": app.proc.pid, "reaped": True,
         "argv": console_args(width, height), "visible_windows": windows},
    )


def verify_play(record, profile, artifact, pinned_png):
    width, height, _, current, image_name = PROFILES[profile]
    for part in ("layouts", "initial"):
        require(set(record[part]) == {"3", "4", "5"}, f"Incomplete count proofs: {part}")
    for count in (3, 4, 5):
        verify_layout(record["layouts"][str(count)], profile, count)
        verify_semantics(record["initial"][str(count)], count, HOME)
    require([entry["focus"] for entry in record["tabs"]] == tab_order(current),
            "Tab traversal or disabled skip differs")
    for entry in record["tabs"]:
        verify_semantics(entry["current_page"], 5, HOME)
    verify_semantics(record["selected"], 5, current)
    require(record["focus_after_enter"] == current and record["disabled_activation"] == "disabled"
            and record["focus_at_capture"] == AFTER_SELECTED[current]
            and record["return_focus"] == SHOW and record["selected_layout"] == record["layouts"]["5"],
            "Keyboard selection or launcher focus restoration differs")
    require(record["image"] == {"name": f"{image_name}.png", "width": width, "height": height,
                                "scale": 1, "format": "png", "sha256": pinned_png},
            f"{artifact.name}: incorrect screenshot receipt")
    require(record["pixels"] == paint(artifact, profile, record["selected_layout"], record["palette"]),
            f"{artifact.name}: pixels differ from report")


def readback(root, evidence_sha, source_sha, binary_sha):
    files = pins(root, root / "SHA256SUMS", evidence_sha)
    expected = {REPORT}
    for replay in ("1", "2"):
        expected.update(f"repeat-{replay}/{profile[-1]}.png" for profile in PROFILES.values())
    actual = {path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file()}
    require(set(files) == expected and actual == expected | {"SHA256SUMS"},
            "Unexpected portable evidence file inventory")
    report = json.loads((root / REPORT).read_text())
    require(report["schema"] == SCHEMA and report["transports"] == TRANSPORTS
            and report["source"]["manifest_sha256"] == source_sha
            and report["source"]["version"] == "0.1.73"
            and type(report["source"]["file_count"]) is int and report["source"]["file_count"] > 0
            and report["binaries"]["manifest_sha256"] == binary_sha
            and set(report["binaries"]["files"]) == set(BINARIES)
            and all(SHA.fullmatch(value) for value in report["binaries"]["files"].values()),
            "Portable source or executable pins differ")
    records = report["replays"]
    require(set(records) == {"1", "2"} and records["1"] == records["2"]
            and set(records["1"]) == set(PROFILES),
            "Repeated geometry, interaction or PNG receipts differ")
    require(records["1"]["wide"]["palette"] != records["1"]["medium"]["palette"],
            "Both native profiles report the same palette")
    for replay, profiles in records.items():
        for profile, record in profiles.items():
            path = f"repeat-{replay}/{PROFILES[profile][-1]}.png"
            require(path in files, f"Missing sealed image {path}")
            verify_play(record, profile, root / path, files[path])
    processes = report["processes"]
    require(len(processes) == 4 and len({entry["pid"] for entry in processes}) == 4
            and {(entry["replay"], entry["profile"]) for entry in processes}
            == {(replay, profile) for replay in records for profile in PROFILES},
            "Missing owned process or replay")
    for entry in processes:
        width, height, *_ = PROFILES[entry["profile"]]
        require(type(entry["pid"]) is int and entry["pid"] > 0 and entry["reaped"] is True
                and entry["argv"] == console_args(width, height)
                and set(entry["visible_windows"]) == SAMPLES
                and all(type(value) is int and value == 0 for value in entry["visible_windows"].values()),
                "An owned process or its window observations are invalid")
    return {"schema": SCHEMA, "evidence_sha256": evidence_sha, "source_sha256": source_sha,
            "binary_sha256": binary_sha, "original_pngs": 4, "replays": 2,
            "transports": TRANSPORTS, "window_samples": len(processes) * len(SAMPLES),
            "visible_windows": 0, "reaped": 4}


def run(args):
    require(sys.platform == "darwin", "Native process and window verification requires macOS")
    gallery, cli, inputs = check_inputs(args)
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()) and out != Path("/tmp").resolve(),
            "A fresh evidence directory under /tmp is required")
    out.mkdir(mode=0o700)
    report = {"schema": SCHEMA, **inputs, "transports": TRANSPORTS, "replays": {}, "processes": []}
    paths = [REPORT]
    for replay in ("1", "2"):
        directory = out / f"repeat-{replay}"
        directory.mkdir()
        report["replays"][replay] = {}
        for profile in PROFILES:
            record, process = play(directory, profile, gallery, cli)
            report["replays"][replay][profile] = record
            report["processes"].append({"replay": replay, **process})
            paths.append(f"repeat-{replay}/{record['image']['name']}")
    gallery_after, cli_after, inputs_after = check_inputs(args)
    require((gallery, cli, inputs) == (gallery_after, cli_after, inputs_after),
            "Source or executable pins changed during replay")
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
    for name in required:
        if not getattr(args, name):
            parser.error(f"--{name.replace('_', '-')} is required")
    for name in ("source_sha", "binary_sha", "evidence_sha"):
        value = getattr(args, name)
        if value is not None and not SHA.fullmatch(value):
            parser.error(f"--{name.replace('_', '-')} needs a 64-digit lowercase SHA-256")
    try:
        if args.run:
            result = run(args)
        elif args.readback:
            result = readback(args.readback.resolve(strict=True), args.evidence_sha,
                              args.source_sha, args.binary_sha)
        elif args.check_inputs:
            _, _, result = check_inputs(args)
        else:
            result = check_source(args)
    except (OSError, ValueError, TypeError, KeyError, RuntimeError) as error:
        parser.exit(1, f"{type(error).__name__}: {error}\n")
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
