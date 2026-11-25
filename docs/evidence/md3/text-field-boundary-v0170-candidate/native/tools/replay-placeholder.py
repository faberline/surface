#!/usr/bin/env python3
"""Replay Material Text Field placeholder endpoints in a native headless Gallery."""

import argparse
import json
from math import ceil, floor
import os
from pathlib import Path
import re
import sys
import tomllib
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, node, settle, visible
from window_check import onscreen_windows

PREFIX = "text-field-placeholder-demo-"
SHOW, ROUTE, BACK, THEME, FILLED, RTL, LARGE, ABOVE, WITHOUT_HINT, SET, CLEAR, EDITOR = (
    PREFIX + part for part in ("show", "route", "back", "theme", "filled", "rtl", "large", "above",
                                "no-hint", "set", "clear", "editor")
)
SHORT_LABEL = "名稱"
LONG_LABEL = "輸入團隊成員在共用專案清單中容易辨認的完整名稱"
HINT = "請輸入完整名稱"
SEED = "範例"
SHA = re.compile(r"[a-f0-9]{64}")
SCOPES = {"wide": (960, 900), "compact": (320, 900)}
APPEARANCES = ("outlined", "filled")
ENDPOINTS = {
    "wide": ("idle", "focused", "blur", "focused-nohint", "idle-nohint"),
    "compact": ("above-idle", "above-seeded", "above-seeded-nohint", "above-clear", "above-idle-nohint"),
}
TRANSPORTS = ["Page.screenshot", "CLI", "MCP"]
REPORT = "native-text-field-placeholder-report.json"
SCHEMA = "surface.test.text-field-placeholder.native.v1"
SCOPE = "Native headless endpoints under Reduce Motion, Runtime semantic snapshot and DevTools input"
GATES = {
    "Cargo.toml",
    "crates/surface-app/src/editing_runtime.rs",
    "crates/surface-app/src/editor_paint.rs",
    "crates/surface-app/src/placeholder_motion.rs",
    "crates/surface-material/src/outlined_text_field.rs",
    "examples/material-gallery/src/lib.rs",
    "examples/material-gallery/src/text_field_placeholder_demo.rs",
}


def pins(root, manifest, expected):
    require(SHA.fullmatch(expected or "") and manifest.is_file() and digest(manifest) == expected,
            "Reviewed manifest digest differs")
    entries = {}
    for line in manifest.read_text().splitlines():
        value, name = line.split("  ", 1)
        rel = Path(name)
        path = root / rel
        require(SHA.fullmatch(value) and not rel.is_absolute() and ".." not in rel.parts
                and rel.as_posix() == name and name not in entries and name != "."
                and path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root)
                and digest(path) == value, f"Invalid pinned file: {name}")
        entries[name] = value
    require(entries, "Reviewed manifest is empty")
    return entries


def right(rect):
    return rect["x"] + rect["width"]


def bottom(rect):
    return rect["y"] + rect["height"]


def near(left, right):
    return abs(left - right) <= 0.05


def crop(rect, width, height):
    box = (ceil(rect["x"]), ceil(rect["y"]), floor(right(rect)), floor(bottom(rect)))
    require(0 <= box[0] < box[2] <= width and 0 <= box[1] < box[3] <= height,
            f"Crop is outside the actual screenshot: {rect}")
    return box


def pixel(rows, x, y):
    return tuple(rows[y][4 * x:4 * x + 4])


def patch_difference(edited, reference, face, body):
    width = len(reference[0]) // 4
    height = len(reference)
    require(len(edited) == height and len(edited[0]) == width * 4,
            "Endpoint/control viewport differs")
    left, top, stop_x, stop_y = crop(face, width, height)
    body_left, body_top, body_right, body_bottom = crop(body, width, height)
    changed = [(x, y) for y in range(top, stop_y) for x in range(left, stop_x)
               if pixel(edited, x, y) != pixel(reference, x, y)]
    require(all(body_left <= x < body_right and body_top <= y < body_bottom for x, y in changed),
            "The comparison also changed paint outside the editor line")
    if not changed:
        return {"pixels": 0, "ink_bounds": None, "max_rgb_delta": 0}
    xs, ys = zip(*changed)
    ink = [min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1]
    contrast = max(max(abs(a - b) for a, b in zip(pixel(edited, x, y)[:3], pixel(reference, x, y)[:3]))
                   for x, y in changed)
    return {"pixels": len(changed), "ink_bounds": ink, "max_rgb_delta": contrast}


def label_ink(rows, bounds, paper):
    left, top, stop_x, stop_y = crop(bounds, len(rows[0]) // 4, len(rows))
    return sum(max(abs(a - b) for a, b in zip(pixel(rows, x, y)[:3], paper[:3])) >= 25
               for y in range(top, stop_y) for x in range(left, stop_x))


def style_background(frame, target):
    color = node(frame, target)["style"]["background"]
    require(len(color) == 4 and color[3] == 255 and all(type(v) is int and 0 <= v <= 255 for v in color),
            f"Invalid opaque background in {target}")
    return color


def editor_state(frame, value, label, current_focus):
    focus(frame, current_focus)
    semantic = node(frame, EDITOR)
    editor = frame["text_inputs"].get(EDITOR)
    require(sum(item["role"] == "TextInput" for item in frame["dom"]["nodes"]) == 1
            and semantic["role"] == "TextInput" and semantic["name"] == label
            and semantic["text"] == value and semantic["disabled"] is False
            and editor is not None and editor["value"] == editor["display_value"] == value
            and not editor["composing"] and editor["preedit_range"] is None,
            "The hint changed the native editor semantic node or value")
    require(not any(HINT in (item.get("name"), item.get("text")) for item in frame["dom"]["nodes"]),
            "The visual hint acquired an authored semantic node")
    selection = [editor["selection_anchor"], editor["selection_cursor"]]
    for offset in selection:
        encoded = value.encode()
        require(type(offset) is int and 0 <= offset <= len(encoded), "Selection is outside the committed value")
        encoded[:offset].decode()
    return {"editor": EDITOR, "label": label, "role": "TextInput", "value": value,
            "display_value": editor["display_value"], "focus": current_focus,
            "selection_utf8": selection, "disabled": False, "hint_in_semantic_nodes": False}


def geometry(frame, appearance, compact):
    filled = appearance == "filled"
    scale = 2 if compact else 1
    label_height = 16 * scale
    line_height = 24 * scale
    face = visible(frame, EDITOR, 47.95)
    boundary = visible(frame, EDITOR + ("-indicator" if filled else "-outline"))
    require(same_box(face, boundary, tolerance=.05) and same_box(boundary, face, tolerance=.05)
            and (find(frame, EDITOR + "-container")[1] is not None) is filled,
            "Single editor target no longer matches the painted face")
    if compact:
        above = visible(frame, EDITOR + "-above-label")
        require(above["height"] >= 2 * label_height and bottom(above) + 3.5 <= face["y"]
                and near(right(above), right(face) - 16), "The complete long label must wrap above the editor")
    else:
        above = None
    y = face["y"] + (8 + label_height if filled else (face["height"] - line_height) / 2)
    body = {"x": face["x"] + 16, "y": y, "width": face["width"] - 32, "height": line_height}
    require(same_box(face, body, tolerance=.05), "Measured editor text line clips or lost its scale")
    return {"face": face, "line": body, "above": above, "text_scale": scale,
            "direction": "RTL" if compact else "LTR",
            "paper": style_background(frame, EDITOR + "-container" if filled else ROUTE),
            "background": style_background(frame, ROUTE)}


def same_face_measure(a, b):
    return all(a[key] == b[key] for key in ("face", "line", "above", "text_scale", "direction", "paper", "background"))


def photo(app, plan, frame, appearance, stage, value, current_focus, window):
    compact = plan.scope == "compact"
    measured = geometry(frame, appearance, compact)
    semantic = editor_state(frame, value, LONG_LABEL if compact else SHORT_LABEL, current_focus)
    title = f"{plan.scope}-{appearance}-{stage}"
    window(app, f"before/{title}")
    shot = capture(app, plan, title)
    window(app, f"after/{title}")
    require((shot["width"], shot["height"]) == SCOPES[plan.scope], "PNG did not render the requested viewport")
    return {**shot, "layout": measured, "semantic": semantic}


def verify_one_appearance(root, shots, scope, appearance):
    require(tuple(shots) == ENDPOINTS[scope], "Wrong endpoint image order or inventory")
    decoded = {}
    wide = scope == "wide"
    expected_meta = (1, *SCOPES[scope], "png")
    first = shots[ENDPOINTS[scope][0]]["layout"]
    expected_label = SHORT_LABEL if wide else LONG_LABEL
    for name, record in shots.items():
        expected_name = f"{scope}-{appearance}-{name}.png"
        path = root / expected_name
        require(record["name"] == expected_name
                and (record["scale"], record["width"], record["height"], record["format"]) == expected_meta
                and record["sha256"] == digest(path), "Wrong saved original PNG or metadata")
        width, height, rows = read_png(path.read_bytes())
        require((width, height) == SCOPES[scope] and same_face_measure(first, record["layout"]),
                "Geometry moved across the visible and hidden hint controls")
        semantic = record["semantic"]
        value = SEED if not wide and "seeded" in name else ""
        focused = wide and name in ("focused", "focused-nohint")
        target = EDITOR if focused else SET if value else CLEAR
        selection = semantic["selection_utf8"]
        require(semantic == {"editor": EDITOR, "label": expected_label, "role": "TextInput",
                             "value": value, "display_value": value, "focus": target,
                             "selection_utf8": selection, "disabled": False, "hint_in_semantic_nodes": False}
                and isinstance(selection, list) and len(selection) == 2
                and (selection == [0, 0] if not value else all(offset in (0, 3, 6) for offset in selection)),
                "Stored focus or committed text differs")
        decoded[name] = rows
    face, body = first["face"], first["line"]
    require(first["text_scale"] == (1 if wide else 2) and first["direction"] == ("LTR" if wide else "RTL")
            and body["height"] == (24 if wide else 48) and same_box(face, body, tolerance=.05),
            "Wrong viewport, scale, or value rect")
    paper_probe = (floor(face["x"] + 8), floor(face["y"] + face["height"] / 2))
    require(all(pixel(rows, *paper_probe) == tuple(first["paper"]) for rows in decoded.values()),
            "Actual face pixels differ from the resolved background")

    def delta(stage, control):
        return patch_difference(decoded[stage], decoded[control], face, body)

    if wide:
        pairs = {"idle_no_hint": delta("idle", "idle-nohint"),
                 "focused_hint": delta("focused", "focused-nohint"),
                 "blur_no_hint": delta("blur", "idle-nohint")}
        visible_keys = ("focused_hint",)
        absent_keys = ("idle_no_hint", "blur_no_hint")
        require(first["above"] is None and first["background"][0] > 180, "Wide light fixture differs")
    else:
        pairs = {"idle_hint": delta("above-idle", "above-idle-nohint"),
                 "clear_hint": delta("above-clear", "above-idle-nohint"),
                 "seeded_no_hint": delta("above-seeded", "above-seeded-nohint"),
                 "seeded_value": delta("above-seeded-nohint", "above-idle-nohint")}
        visible_keys = ("idle_hint", "clear_hint", "seeded_value")
        absent_keys = ("seeded_no_hint",)
        require(pairs["idle_hint"] == pairs["clear_hint"]
                and delta("above-idle", "above-clear")["pixels"] == 0
                and first["above"]["height"] >= 64 and first["background"][0] < 70
                and label_ink(decoded["above-idle"], first["above"], first["background"]) >= 64,
                "Compact dark fallback or exact clear endpoint differs")
    for check in absent_keys:
        require(pairs[check]["pixels"] == 0, f"{scope}/{appearance}/{check}: unexpected hint pixels")
    for check in visible_keys:
        result = pairs[check]
        require(result["pixels"] >= 18 and result["max_rgb_delta"] >= 25,
                f"{scope}/{appearance}/{check}: missing visible text")
        x, _, w, h = result["ink_bounds"]
        logical_inset = x - body["x"] if wide else right(body) - (x + w)
        require(0 <= logical_inset <= 12 and h >= (6 if wide else 12) and w >= 12,
                f"{scope}/{appearance}/{check}: glyph ink lost the logical text anchor")
    return pairs


def wide_case(app, plan, appearance, window):
    shots = {}

    def shot(stage, frame, target):
        shots[stage] = photo(app, plan, frame, appearance, stage, "", target, window)

    shot("idle", activate(app, CLEAR), CLEAR)
    shot("focused", key(app, "tab"), EDITOR)
    shot("blur", key(app, "tab", reverse=True), CLEAR)
    activate(app, WITHOUT_HINT)
    focus_by_tab(app, EDITOR)
    shot("focused-nohint", settle(app), EDITOR)
    shot("idle-nohint", key(app, "tab", reverse=True), CLEAR)
    activate(app, WITHOUT_HINT)
    return shots


def compact_case(app, plan, appearance, window):
    shots = {}

    def shot(stage, frame, value, target):
        shots[stage] = photo(app, plan, frame, appearance, stage, value, target, window)

    shot("above-idle", activate(app, CLEAR), "", CLEAR)
    shot("above-seeded", activate(app, SET), SEED, SET)
    activate(app, WITHOUT_HINT)
    focus_by_tab(app, SET)
    shot("above-seeded-nohint", settle(app), SEED, SET)
    activate(app, WITHOUT_HINT)
    shot("above-clear", activate(app, CLEAR), "", CLEAR)
    activate(app, WITHOUT_HINT)
    focus_by_tab(app, CLEAR)
    shot("above-idle-nohint", settle(app), "", CLEAR)
    activate(app, WITHOUT_HINT)
    return shots


def phases(scope):
    moments = {"ready", "opened", "closed"}
    for appearance in APPEARANCES:
        for stage in ENDPOINTS[scope]:
            for edge in ("before", "after"):
                moments.add(f"{edge}/{scope}-{appearance}-{stage}")
    return moments


def replay(plan):
    width, height = SCOPES[plan.scope]
    observations, results = {}, {}

    def window(app, phase):
        require(app.proc.poll() is None and phase not in observations,
                f"{phase}: child exited early or phase repeated")
        count = onscreen_windows(app.proc.pid)
        require(count == 0, f"{phase}: native Gallery opened an onscreen window")
        observations[phase] = count

    with launch(plan, plan.scope, width, height) as app:
        window(app, "ready")
        frame = activate(app, SHOW)
        focus(frame, BACK)
        if plan.scope == "compact":
            for target in (THEME, RTL, LARGE, ABOVE):
                activate(app, target)
        window(app, "opened")
        trace = wide_case if plan.scope == "wide" else compact_case
        for appearance in APPEARANCES:
            if appearance == "filled":
                activate(app, FILLED)
            captures = trace(app, plan, appearance, window)
            results[appearance] = {"captures": captures,
                                   "pixel_checks": verify_one_appearance(plan.out, captures, plan.scope, appearance)}
        frame = activate(app, BACK)
        focus(frame, SHOW)
        require(find(frame, ROUTE)[1] is None, "The route is still mounted after Back")
        window(app, "closed")
    require(app.proc.returncode is not None and observations.keys() == phases(plan.scope),
            "The headless Gallery was not reaped or a window sample is missing")
    return {"viewport": [width, height], "appearances": results, "restored_focus": SHOW}, {
        "pid": app.proc.pid, "visible_windows": observations, "reaped": True,
    }


def checked_inputs(args):
    root = args.source_root.resolve(strict=True)
    bundle = args.binary_manifest.resolve(strict=True).parent
    source = pins(root, args.source_manifest, args.source_manifest_sha)
    require(GATES <= source.keys(), "Source manifest omits the Material placeholder implementation or demo")
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == "0.1.69", "The checked source must be v0.1.69")
    binaries = pins(bundle, args.binary_manifest, args.binary_manifest_sha)
    gallery, cli = (bundle / name for name in ("surface-material-gallery", "surface-devtools"))
    require(set(binaries) == {gallery.name, cli.name} and all(os.access(item, os.X_OK) for item in (gallery, cli)),
            "Wrong or nonexecutable native binary pair")
    return gallery, cli, {"source": {"manifest_sha256": args.source_manifest_sha, "files": len(source),
                                      "version": version},
                          "binary_manifest_sha256": args.binary_manifest_sha, "binaries": binaries}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--source-manifest", type=Path, required=True)
    parser.add_argument("--source-manifest-sha", required=True)
    parser.add_argument("--binary-manifest", type=Path, required=True)
    parser.add_argument("--binary-manifest-sha", required=True)
    parser.add_argument("--verify-inputs-only", action="store_true")
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    gallery, cli, proof = checked_inputs(args)
    if args.verify_inputs_only:
        require(args.out is None, "Preflight creates no evidence folder")
        print(json.dumps(proof, indent=2))
        return
    require(sys.platform == "darwin" and args.out is not None, "Replay requires macOS and a fresh --out")
    out = args.out.resolve()
    scratch = Path("/tmp").resolve()
    require(out != scratch and out.is_relative_to(scratch), "Output must use a fresh private /tmp path")
    old_umask = os.umask(0o077)
    try:
        out.mkdir(mode=0o700)
        replays, processes = [], []
        for index in (1, 2):
            directory = out / f"replay-{index}"
            directory.mkdir(mode=0o700)
            results = {scope: replay(SimpleNamespace(gallery=gallery, cli=cli, out=directory, scope=scope))
                       for scope in SCOPES}
            replays.append({scope: result[0] for scope, result in results.items()})
            processes.append({scope: result[1] for scope, result in results.items()})
        require(replays[0] == replays[1], "Original PNGs, viewport or semantic evidence changed between repeats")
        require(checked_inputs(args) == (gallery, cli, proof), "Source or binaries changed during the native replay")
        report = {**proof, "schema": SCHEMA, "transports": TRANSPORTS,
                  "replays": replays, "processes": processes, "scope": SCOPE}
        (out / REPORT).write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
        files = sorted(file for file in out.rglob("*") if file.is_file())
        (out / "SHA256SUMS").write_text("".join(f"{digest(file)}  {file.relative_to(out)}\n" for file in files))
    finally:
        os.umask(old_umask)
    print(f"2 out of 2 headless placeholder replays passed at {out}")


if __name__ == "__main__":
    try:
        main()
    except (KeyError, ValueError, OSError, RuntimeError) as error:
        sys.exit(f"Native placeholder replay stopped. {error}")
