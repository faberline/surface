#!/usr/bin/env python3
"""Replay pinned multiline Text Fields in the native headless Gallery."""

import argparse
import json
from math import ceil, floor
import os
from pathlib import Path
import re
import sys
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

PREFIX = "multiline-text-field-demo-"
SHOW, ROUTE, BACK, APPEARANCE, THEME, RTL, LARGE_TEXT, SCROLL = (
    PREFIX + part for part in ("show", "route", "back", "appearance", "theme", "rtl", "large-text", "scroll")
)
GROW, FIXED, SUBMIT, FEEDBACK, READ_ONLY, DISABLED = (
    PREFIX + part for part in ("grow", "fixed", "submit", "feedback", "read-only", "disabled")
)
CLEAR = GROW + "-trailing-action"
GROW_INITIAL = "第一行"
GROW_TWO = "甲甲甲\n甲"
FIXED_INITIAL = "第一行😀\n第二行 a\u0301\n第三行 ABC 中文文字\n第四行\n第五行\n第六行\n第七行\n第八行\n"
FIELD_NAMES = {GROW: "多行備註", FIXED: "詳細說明", READ_ONLY: "唯讀內容", DISABLED: "停用內容"}
READ_ONLY_VALUE = "已儲存的第一行\n已儲存的第二行\n"
DISABLED_VALUE = "停用的第一行\n停用的第二行"
SCOPES = {"wide": (960, 900), "compact": (320, 900)}
TRANSPORTS = ["Page.screenshot", "CLI", "MCP"]
SHA = re.compile(r"[a-f0-9]{64}")
COMPONENT_PATHS = {
    "crates/surface-app/src/editing_runtime.rs",
    "crates/surface-app/src/focus_scroll.rs",
    "crates/surface-material/src/outlined_text_field.rs",
    "crates/surface-wgpu/src/text.rs",
    "examples/material-gallery/src/lib.rs",
    "examples/material-gallery/src/multiline_text_field_demo.rs",
}


def pins(root, manifest, expected):
    require(SHA.fullmatch(expected) and digest(manifest) == expected, "Reviewed manifest digest differs")
    entries = {}
    for line in manifest.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        relative = Path(name)
        path = root / relative
        require(SHA.fullmatch(checksum) and not relative.is_absolute() and ".." not in relative.parts
                and name != "." and name == relative.as_posix() and name not in entries
                and path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root)
                and digest(path) == checksum, f"Invalid pinned file: {name}")
        entries[name] = checksum
    require(entries, "Reviewed manifest is empty")
    return entries


def right(rect):
    return rect["x"] + rect["width"]


def bottom(rect):
    return rect["y"] + rect["height"]


def near(a, b):
    return abs(a - b) <= .05


def inside(outer, inner):
    return same_box(outer, inner, tolerance=.05)


def style_color(frame, target, property_name):
    rgba = node(frame, target)["style"][property_name]
    require(len(rgba) == 4 and all(type(c) is int and 0 <= c <= 255 for c in rgba),
            f"Wrong authored {property_name} color for {target}")
    return tuple(rgba)


def editor(frame, field, value):
    semantic = node(frame, field)
    snapshot = frame["text_inputs"].get(field)
    require(sum(item.get("stable_id") == field for item in frame["dom"]["nodes"]) == 1
            and (semantic["role"], semantic["name"], semantic["text"]) == ("TextInput", FIELD_NAMES[field], value)
            and semantic["disabled"] is (field == DISABLED), f"Wrong stable editor: {field}")
    if field == DISABLED:
        require(snapshot is None, "Disabled example mounted an editing session")
        return {"value": value, "disabled": True}
    require(snapshot is not None and snapshot["value"] == snapshot["display_value"] == value
            and snapshot["composing"] is False and snapshot["preedit_range"] is None,
            f"Wrong committed native editing snapshot for {field}")
    offsets = [snapshot["selection_anchor"], snapshot["selection_cursor"]]
    encoded = value.encode()
    require(all(0 <= position <= len(encoded) for position in offsets), f"Selection offset escapes {field}")
    for position in offsets:
        encoded[:position].decode()
    require(semantic.get("described_by_text") == field + "-description", f"Editor lost its support: {field}")
    return {"value": value, "selection_utf8": offsets, "description": semantic["described_by_text"], "disabled": False}


def counter(frame, count):
    visual = node(frame, GROW + "-counter")
    spoken = node(frame, GROW + "-counter-description")
    require(visual["text"] == f"{count}/12"
            and spoken["text"] == f"字數，已輸入 {count} 個字，上限 12 個字。"
            and not spoken.get("assertive_live_status")
            and node(frame, GROW).get("additional_description_text_targets")
            == [GROW + "-prefix-description", GROW + "-counter-description"],
            "Wrong committed multiline count or ordered description")
    return {"ratio": visual["text"], "description": spoken["text"]}


def measure(frame, field, appearance, row_count, *, rtl, text_scale):
    filled = appearance == "filled"
    face = visible(frame, field + ("-indicator" if filled else "-outline"))
    editor_bounds = visible(frame, field)
    body = visible(frame, field + "-value-box")
    outer = visible(frame, SCROLL)
    outer_offset = outer["y"] + 8 - layout(frame, GROW + "-field")["bounds"]["y"]
    support = layout(frame, field + "-description")
    description = support["bounds"]
    support_visible = inside(support["clip"], description)
    pitch = 24 * text_scale
    require(near(outer["height"], 480) and outer_offset >= -.05
            and near(body["height"], pitch * row_count) and inside(editor_bounds, body)
            and inside(face, editor_bounds) and inside(outer, face)
            and near(body["y"] - face["y"], 8 + 16 * text_scale if filled else 16)
            and near(bottom(face) - bottom(body), 8 if filled else 16)
            and near(description["y"] - bottom(face), 4),
            f"Measured {field} geometry differs from {appearance}, {row_count} rows, scale {text_scale}")
    require(support_visible or (field == FIXED and rtl),
            "Supporting text clips in a profile with room for the whole field")
    require((find(frame, field + "-container")[1] is not None) is filled,
            "Container and requested appearance disagree")
    proof = {"face": face, "editor": editor_bounds, "body": body, "description": description,
             "description_visible": support_visible, "description_clip": support["clip"],
             "caption": visible(frame, field + "-floating-label"), "line_height": pitch,
             "row_count": row_count, "scroller": {"bounds": outer, "offset_y": outer_offset}}
    if field == GROW:
        button = node(frame, CLEAR)
        hit = visible(frame, CLEAR, 47.95)
        prefix = visible(frame, GROW + "-prefix-visual")
        opposite = near(hit["x"], face["x"]) if rtl else near(right(hit), right(face))
        separate = near(right(hit), editor_bounds["x"]) if rtl else near(right(editor_bounds), hit["x"])
        gap = prefix["x"] - right(body) if rtl else body["x"] - right(prefix)
        require((button["role"], button["name"], button["disabled"]) == ("Button", "清除備註", False)
                and near(hit["width"], 48) and near(hit["height"], 48) and separate and opposite
                and node(frame, GROW + "-prefix-visual")["text"] == "#" and near(gap, 2),
                "Clear or passive prefix lost its separate logical placement")
        proof.update({"clear_hit": hit, "prefix": prefix})
    else:
        require(near(body["x"] - face["x"], 16) and near(right(face) - right(body), 16),
                "Fixed editor lost equal text insets")
    return proof


def pixel(rows, x, y):
    x, y = floor(x), floor(y)
    require(0 <= y < len(rows) and 0 <= x < len(rows[0]) // 4, "Pixel outside original PNG")
    return tuple(rows[y][4 * x:4 * x + 4])


def rgb_gap(first, second):
    return max(abs(a - b) for a, b in zip(first[:3], second[:3], strict=True))


def text_band(rows, box, paper, foreground, caret_ink):
    points, caret = [], []
    for y in range(ceil(box["y"]), floor(bottom(box))):
        for x in range(ceil(box["x"]), floor(right(box))):
            color = pixel(rows, x, y)
            if color == caret_ink:
                caret.append((x, y))
            elif color[3] == 255 and rgb_gap(color, paper) >= 35 and rgb_gap(color, foreground) <= 42:
                points.append((x, y))
    return {"ink_pixels": len(points), "ink": point_bounds(points), "caret_pixels": len(caret),
            "caret": point_bounds(caret)}


def point_bounds(points):
    if not points:
        return None
    xs, ys = zip(*points)
    return {"left": min(xs), "right": max(xs) + 1, "top": min(ys), "bottom": max(ys) + 1}


def body_paint(rows, measured, paper, ink, row_profile, *, rtl=False, equal_end=False, caret_ink=None):
    body, face = measured["body"], measured["face"]
    pitch = measured["line_height"]
    bands = []
    for index, occupied in enumerate(row_profile):
        box = {**body, "y": body["y"] + index * pitch, "height": pitch}
        row = text_band(rows, box, paper, ink, caret_ink)
        if occupied:
            require(row["ink_pixels"] >= 16, f"No readable editor text on line {index + 1}")
        else:
            require(row["ink_pixels"] == 0, f"Unexpected editor glyph pixels on blank line {index + 1}")
            if caret_ink is not None:
                caret = row["caret"]
                require(caret is not None and caret["right"] - caret["left"] <= 2
                        and caret["bottom"] - caret["top"] >= pitch - 1,
                        "Blank line lost its separate full-height caret")
        bands.append(row)
    gap = {**body, "y": bottom(body), "height": bottom(face) - bottom(body) - 3}
    require(gap["height"] >= 5 and all(
        pixel(rows, x, y) == paper for y in range(ceil(gap["y"]), floor(bottom(gap)))
        for x in range(ceil(gap["x"]), floor(right(gap)))
    ), "Text escaped the bottom of its value-box viewport")
    if equal_end:
        first, second = (band["ink"] for band in bands[:2])
        if rtl:
            require(right(body) - first["right"] <= 12 and right(body) - second["right"] <= 12
                    and abs(first["right"] - second["right"]) <= 3 and second["left"] - first["left"] >= 24,
                    "Shorter RTL line did not keep the same physical right ink edge")
        else:
            require(first["left"] - body["x"] <= 10 and second["left"] - body["x"] <= 10
                    and abs(first["left"] - second["left"]) <= 3 and first["right"] - second["right"] >= 12,
                    "Shorter LTR line did not keep the same physical left ink edge")
    return {"lines": bands, "clear_padding": gap, "foreground": list(ink), "paper": list(paper)}


def face_paint(frame, rows, field, measured, appearance, *, focused_editor, rtl):
    face = measured["face"]
    page = style_color(frame, ROUTE, "background")
    paper = style_color(frame, field + "-container", "background") if appearance == "filled" else page
    x = face["x"] + face["width"] / 2
    samples = [pixel(rows, x, bottom(face) - index - .5) for index in range(5)]
    stroke = 3 if focused_editor else 1
    require(samples[0] != paper and samples[:stroke] == [samples[0]] * stroke
            and samples[stroke:] == [paper] * (5 - stroke),
            "Original pixels do not match the 1 or 3 dp bottom stroke")
    corner, notch = None, None
    if appearance == "filled":
        corner = [pixel(rows, face["x"] + offset, face["y"] + .5) for offset in (.5, 6.5)]
        require(paper != page and corner == [page, paper]
                and pixel(rows, face["x"] + .5, bottom(face) - .5) == samples[0],
                "Filled container lost its top radius, background or square bottom")
    elif focused_editor:
        caption = measured["caption"]
        y = face["y"] + .5
        notch = [pixel(rows, point, y) for point in (caption["x"] - 2, right(caption) + 2)]
        edge = pixel(rows, face["x"] + 8 if rtl else right(face) - 8, y)
        require(notch == [paper, paper] and edge == samples[0],
                "Focused Outline lost its clear notch or matching top stroke")
    require((max(page[:3]) < 80) is rtl, "Wrong Gallery backdrop for this profile")
    return {"stroke_dp": stroke, "samples": samples, "paper": list(paper), "top_corner": corner, "notch": notch}


def photo(app, plan, frame, name, field, appearance, value, row_profile, *, focused_editor=True, equal_end=False):
    rtl = plan.mode == "compact"
    state = editor(frame, field, value)
    measured = measure(frame, field, appearance, len(row_profile), rtl=rtl, text_scale=2 if rtl else 1)
    focus(frame, field if focused_editor else BACK)
    shot = capture(app, plan, name)
    require((shot["width"], shot["height"], shot["scale"]) == (*SCOPES[plan.mode], 1), "Screenshot size changed")
    _, _, rows = read_png((plan.out / shot["name"]).read_bytes())
    face = face_paint(frame, rows, field, measured, appearance, focused_editor=focused_editor, rtl=rtl)
    caret_ink = face["samples"][0] if focused_editor else None
    body = body_paint(rows, measured, tuple(face["paper"]), style_color(frame, field, "foreground"), row_profile,
                      rtl=rtl, equal_end=equal_end, caret_ink=caret_ink)
    proof = {**shot, "field": field, "appearance": appearance, "rtl": rtl, "state": state,
             "geometry": measured, "face_paint": face, "text_paint": body}
    if field == GROW:
        proof["counter"] = counter(frame, len(value))
    return proof


def grow(app, *, rtl):
    steps = focus_by_tab(app, GROW)
    before = settle(app)
    focus(before, GROW)
    editor(before, GROW, GROW_INITIAL)
    scale = 2 if rtl else 1
    base = measure(before, GROW, "outlined", 1, rtl=rtl, text_scale=scale)
    frame = key(app, "enter")
    focus(frame, GROW)
    value = GROW_INITIAL + "\n"
    newline = editor(frame, GROW, value)
    require(newline["selection_utf8"] == [len(value.encode())] * 2 and node(frame, FEEDBACK)["text"] == "",
            "Enter did not add a blank text row without submitting")
    taller = measure(frame, GROW, "outlined", 2, rtl=rtl, text_scale=scale)
    require(near(taller["face"]["height"] - base["face"]["height"], taller["line_height"]),
            "Blank terminal row did not grow the face by one line")
    typed = counter(frame, len(value))
    app.wire("Action.fill", {"id": GROW, "value": GROW_TWO})
    filled = settle(app)
    focus(filled, GROW)
    editor(filled, GROW, GROW_TWO)
    frame = key(app, "enter")
    editor(frame, GROW, GROW_TWO + "\n")
    require(node(frame, FEEDBACK)["text"] == "", "Growing multiline field submitted during edit")
    return frame, {"focus_steps": steps, "newline": newline, "counter": typed,
                   "body_heights": [base["body"]["height"], taller["body"]["height"]]}


def fixed_page(app, *, top):
    offsets = []
    end = len(FIXED_INITIAL.encode())
    first_line = len(FIXED_INITIAL.split("\n", 1)[0].encode())
    for _ in range(8):
        frame = key(app, "page_up" if top else "page_down")
        focus(frame, FIXED)
        anchor, cursor = editor(frame, FIXED, FIXED_INITIAL)["selection_utf8"]
        require(anchor == cursor, "Unmodified paging must move a collapsed caret")
        offsets.append(cursor)
        reached = cursor <= first_line if top else cursor == end
        if reached:
            break
    else:
        raise RuntimeError(f"Fixed editor caret never reached the expected end: {offsets}")
    require(len(offsets) >= 2 and all((a > b if top else a < b) for a, b in zip(offsets, offsets[1:])),
            f"Fixed editor paging went in the wrong direction: {offsets}")
    return frame, offsets


def fixed_geometry(top, bottom_photo):
    first, last = top["geometry"], bottom_photo["geometry"]
    for part in ("face", "editor", "body"):
        require(near(first[part]["height"], last[part]["height"])
                and near(first[part]["width"], last[part]["width"]), "Fixed editor resized while revealing later rows")
    require(top["text_paint"]["lines"][2]["ink_pixels"] >= 16
            and bottom_photo["text_paint"]["lines"][2]["ink_pixels"] == 0
            and top["sha256"] != bottom_photo["sha256"], "Visible fixed text did not change after paging")


def submit_and_locked(app, fixed_capture):
    frame = key(app, "tab")
    focus(frame, SUBMIT)
    hit = visible(frame, SUBMIT)
    require(node(frame, SUBMIT)["name"] == "送出草稿" and node(frame, FEEDBACK)["text"] == "",
            "Submit must be an independent action with initially empty feedback")
    support = visible(frame, FIXED + "-description")
    require(node(frame, FIXED + "-description")["text"] == "內容會在固定高度內捲動。",
            "Fixed editor lost its authored support")
    initial = fixed_capture["geometry"]
    if not initial["description_visible"]:
        require(support["y"] < initial["description"]["y"],
                "Tab must reveal previously clipped support through the outer scroller")
    frame = key(app, "enter")
    focus(frame, SUBMIT)
    require(node(frame, FEEDBACK)["text"] == "草稿已送出。" and node(frame, FEEDBACK)["role"] == "Status",
            "Separate Submit did not report success")
    current = {field: editor(frame, field, value)
               for field, value in ((GROW, GROW_TWO + "\n"), (FIXED, FIXED_INITIAL))}
    frame = key(app, "tab")
    focus(frame, READ_ONLY)
    read_only = editor(frame, READ_ONLY, READ_ONLY_VALUE)
    app.wire("Action.fill", {"id": READ_ONLY, "value": "改"}, "action_failed")
    app.wire("Action.key", {"key": "enter"}, "action_failed")
    app.wire("Action.fill", {"id": DISABLED, "value": "改"}, "disabled")
    frame = settle(app)
    require(editor(frame, READ_ONLY, READ_ONLY_VALUE) == read_only, "Blocked edits changed read-only value or range")
    disabled = editor(frame, DISABLED, DISABLED_VALUE)
    next_focus = key(app, "tab")["interaction"]["focused"]
    require(next_focus != DISABLED, "Disabled multiline editor acquired Tab focus")
    return {"target": hit, "feedback": "草稿已送出。", "outer_revealed_support": support,
            "retained": current, "read_only": read_only, "disabled": disabled, "next_focus": next_focus}


def replay(plan):
    width, height = SCOPES[plan.mode]
    compact = plan.mode == "compact"
    observations, captures, behavior = {}, [], {}

    def window(app, phase):
        count = onscreen_windows(app.proc.pid)
        require(count == 0, f"{phase}: owned Gallery window became visible")
        observations[phase] = count

    with launch(plan, plan.mode, width, height) as app:
        window(app, "ready")
        frame = activate(app, SHOW)
        focus(frame, BACK)
        if compact:
            for setting in (THEME, RTL, LARGE_TEXT):
                activate(app, setting)
        else:
            captures.append(photo(app, plan, frame, "wide-outlined-rest", GROW, "outlined", GROW_INITIAL, (True,),
                                  focused_editor=False))
        window(app, "opened")

        frame, behavior["grow"] = grow(app, rtl=compact)
        captures.append(photo(app, plan, frame, f"{plan.mode}-outlined-grow", GROW, "outlined",
                              GROW_TWO + "\n", (True, True, False), equal_end=True))
        window(app, "grow_focused")
        activate(app, APPEARANCE)
        if compact:
            focus_by_tab(app, GROW)
            captures.append(photo(app, plan, settle(app), "compact-filled-grow", GROW, "filled",
                                  GROW_TWO + "\n", (True, True, False), equal_end=True))

        behavior["fixed_focus_steps"] = focus_by_tab(app, FIXED)
        frame = settle(app)
        before = editor(frame, FIXED, FIXED_INITIAL)
        require(before["selection_utf8"] == [len(FIXED_INITIAL.encode())] * 2, "Fixed seed cursor moved on entry")
        frame, behavior["fixed_up_utf8"] = fixed_page(app, top=True)
        if not compact:
            captures.append(photo(app, plan, frame, "wide-filled-fixed-top", FIXED, "filled",
                                  FIXED_INITIAL, (True, True, True)))
        frame, behavior["fixed_down_utf8"] = fixed_page(app, top=False)
        captures.append(photo(app, plan, frame, f"{plan.mode}-filled-fixed-bottom", FIXED, "filled",
                              FIXED_INITIAL, (True, True, False)))
        if not compact:
            fixed_geometry(captures[-2], captures[-1])
        window(app, "fixed_revealed")

        behavior["submit_and_locks"] = submit_and_locked(app, captures[-1])
        window(app, "submitted")
        behavior["clear_focus_steps"] = focus_by_tab(app, CLEAR)
        frame = key(app, "space")
        behavior["clear"] = editor(frame, GROW, "")
        behavior["empty_counter"] = counter(frame, 0)
        require(node(frame, CLEAR)["disabled"] and node(frame, FEEDBACK)["text"] == "草稿已送出。",
                "Clear must empty only Grow and keep the separate Submit feedback")
        editor(frame, FIXED, FIXED_INITIAL)
        frame = activate(app, BACK)
        focus(frame, SHOW)
        require(find(frame, ROUTE)[1] is None, "Closed multiline route still mounted")
        window(app, "closed")
    require(app.proc.returncode is not None, "Headless Gallery child was not reaped")
    return {"viewport": [width, height], "captures": captures, "behavior": behavior}, {
        "pid": app.proc.pid, "visible_windows": observations, "reaped": True,
    }


def self_test():
    def rect(x, y, w, h):
        return {"x": x, "y": y, "width": w, "height": h}

    def rejects(function, expected):
        try:
            function()
        except RuntimeError as error:
            require(expected in str(error), f"Wrong failing predicate: {error}")
            return
        raise RuntimeError(f"Negative control accepted: {expected}")

    paper, ink = (20, 18, 24, 255), (230, 225, 233, 255)
    measured = {"face": rect(0, 0, 160, 160), "body": rect(16, 4, 128, 144), "line_height": 48}

    def image(first, second, *, blank_ink=False, leak=False):
        def color(x, y):
            if (first <= x < 140 and 10 <= y < 24) or (second <= x < 140 and 57 <= y < 72):
                return ink
            if 110 <= x < 125 and ((blank_ink and 109 <= y < 112) or (leak and 149 <= y < 152)):
                return ink
            return paper
        return [bytes(channel for x in range(160) for channel in color(x, y)) for y in range(160)]

    good = image(48, 108)
    body_paint(good, measured, paper, ink, (True, True, False), rtl=True, equal_end=True)
    rejects(lambda: body_paint(good, measured, paper, ink, (True, True, False), equal_end=True), "LTR line")
    rejects(lambda: body_paint(image(48, 108, blank_ink=True), measured, paper, ink, (True, True, False)),
            "blank line")
    rejects(lambda: body_paint(image(48, 108, leak=True), measured, paper, ink, (True, True, False)),
            "escaped the bottom")
    print("PASS line placement, blank row and clip controls on synthetic pixels. No app started.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--source-manifest", type=Path)
    parser.add_argument("--source-manifest-sha")
    parser.add_argument("--binary-manifest", type=Path)
    parser.add_argument("--binary-manifest-sha")
    parser.add_argument("--verify-inputs-only", action="store_true")
    parser.add_argument("--out", type=Path)
    parser.add_argument("--repeats", type=int, choices=(1, 2), default=2)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    for parameter in ("source_root", "source_manifest", "source_manifest_sha", "binary_manifest", "binary_manifest_sha"):
        if getattr(args, parameter) is None:
            parser.error(f"--{parameter.replace('_', '-')} is required")
    root = args.source_root.resolve(strict=True)
    bundle = args.binary_manifest.resolve(strict=True).parent
    source = pins(root, args.source_manifest, args.source_manifest_sha)
    require(COMPONENT_PATHS <= source.keys(), "Source manifest does not contain the multiline Gallery product")
    binaries = pins(bundle, args.binary_manifest, args.binary_manifest_sha)
    gallery, cli = (bundle / name for name in ("surface-material-gallery", "surface-devtools"))
    require(set(binaries) == {gallery.name, cli.name} and all(os.access(path, os.X_OK) for path in (gallery, cli)),
            "Wrong native executable bundle")
    proof = {"source": {"manifest_sha256": args.source_manifest_sha, "files": len(source)},
             "binary_manifest_sha256": args.binary_manifest_sha, "binaries": binaries}
    if args.verify_inputs_only:
        require(args.out is None, "Preflight creates no evidence directory")
        print(json.dumps(proof, indent=2))
        return
    require(sys.platform == "darwin" and args.out is not None, "Replay requires macOS and a fresh --out")
    out = args.out.resolve()
    private = Path("/tmp").resolve()
    require(out != private and out.is_relative_to(private), "Output must use a new private /tmp directory")
    out.mkdir(mode=0o700)
    replays, processes = [], []
    for index in range(args.repeats):
        directory = out / f"replay-{index + 1}"
        directory.mkdir(mode=0o700)
        results = {mode: replay(SimpleNamespace(gallery=gallery, cli=cli, out=directory, mode=mode))
                   for mode in SCOPES}
        replays.append({mode: part[0] for mode, part in results.items()})
        processes.append({mode: part[1] for mode, part in results.items()})
    require(all(result == replays[0] for result in replays), "PNG bytes or measured geometry differ between repetitions")
    require(pins(root, args.source_manifest, args.source_manifest_sha) == source
            and pins(bundle, args.binary_manifest, args.binary_manifest_sha) == binaries,
            "Checked inputs changed during replay")
    result = {**proof, "schema": "surface.test.text-field-multiline.native.v1", "transports": TRANSPORTS,
              "replays": replays, "processes": processes,
              "semantics": "Native Runtime DOM and semantic DevTools keys on a headless Gallery"}
    (out / "native-multiline-text-field-report.json").write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    paths = sorted(path for path in out.rglob("*") if path.is_file())
    (out / "SHA256SUMS").write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in paths))
    print(f"{args.repeats} out of {args.repeats} headless replays passed at {out}")


if __name__ == "__main__":
    main()
