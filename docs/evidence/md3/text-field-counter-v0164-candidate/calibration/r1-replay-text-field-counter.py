#!/usr/bin/env python3
"""Replay the pinned Material text field counter in a native headless Gallery."""

import argparse
import json
from math import ceil, floor
import os
from pathlib import Path
import re
import sys
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

PREFIX = "text-field-counter-demo-"
SHOW, ROUTE, BACK, EDIT, READ_ONLY, DISABLED, FEEDBACK = (
    PREFIX + key for key in ("show", "route", "back", "editable", "read-only", "disabled", "feedback")
)
APPEARANCE, NARROW, ERROR = (PREFIX + name for name in ("appearance", "narrow", "error"))
SETTINGS = [PREFIX + name for name in ("theme", "rtl", "large-text")]
FIELD_NAMES = {EDIT: "標籤名稱", READ_ONLY: "唯讀標籤", DISABLED: "停用標籤"}
READ_ONLY_VALUE = "🇹🇼a\u0301"
DISABLED_VALUE = "停用標籤內容"
SEED = "中a\u0301👍🏽"
OVERFLOW = "中a\u0301👍🏽🇹🇼👩‍💻Z"
HELP = "請輸入 5 個字以內的標籤名稱。英文字母、中文或表情符號都可以使用。"
READ_ONLY_HELP = "已儲存的標籤。"
LIMIT_ERROR = "請輸入最多 5 個字。"
MANUAL_ERROR = "這個標籤已存在。"
SUBMITTED = "已送出標籤。"
CLEARED = "已清除標籤。"
SCOPES = {"wide": (960, 1120), "compact": (320, 1200)}
TRANSPORTS = ["Page.screenshot", "CLI", "MCP"]
LOCKED = {READ_ONLY: (READ_ONLY_VALUE, 2, READ_ONLY_HELP), DISABLED: (DISABLED_VALUE, 6, None)}

def pins(root, manifest, expected):
    require(re.fullmatch(r"[a-f0-9]{64}", expected) and digest(manifest) == expected,
            "Reviewed manifest digest differs")
    entries = {}
    for line in manifest.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        relative = Path(name)
        path = root / relative
        require(re.fullmatch(r"[a-f0-9]{64}", checksum) and relative.as_posix() == name
                and not relative.is_absolute() and ".." not in relative.parts
                and name not in entries and path.is_file() and not path.is_symlink()
                and path.resolve().is_relative_to(root) and digest(path) == checksum,
                f"Invalid manifest entry: {name}")
        entries[name] = checksum
    require(entries, "Reviewed manifest is empty")
    return entries


def action(field):
    return field + "-trailing-action"


def box(frame, field, part):
    return layout(frame, field + "-" + part)["bounds"]


def right(rect):
    return rect["x"] + rect["width"]


def bottom(rect):
    return rect["y"] + rect["height"]


def close(a, b):
    return abs(a - b) < 0.05


def disjoint(first, second):
    return (min(right(first), right(second)) - max(first["x"], second["x"]) < 0.05
            or min(bottom(first), bottom(second)) - max(first["y"], second["y"]) < 0.05)




def rgba(node_source, name):
    channels = node_source["style"][name]
    require(len(channels) == 4 and all(type(value) is int and 0 <= value <= 255 for value in channels),
            f"Invalid authored {name} paint")
    return channels


def field_state(frame, field, text, count, description, *, invalid=False):
    editor = node(frame, field)
    require(sum(item.get("stable_id") == field for item in frame["dom"]["nodes"]) == 1
            and (editor["role"], editor["name"], editor["text"]) == ("TextInput", FIELD_NAMES[field], text)
            and editor["disabled"] is (field == DISABLED) and editor.get("invalid", False) is invalid,
            f"Wrong single controlled editor, value, or state on {field}")
    counter_id = field + "-counter-description"
    extras = ([field + "-prefix-description"] if field == EDIT else []) + [counter_id]
    require(editor.get("described_by_text") == (field + "-description" if description else None)
            and editor.get("additional_description_text_targets") == extras,
            f"Wrong ordered support, affix or count description targets on {field}")
    counter = node(frame, field + "-counter")
    spoken = node(frame, counter_id)
    label = f"字數，已輸入 {count} 個字，上限 5 個字。"
    require((counter["role"], counter["text"]) == ("Text", f"{count}/5")
            and (spoken["role"], spoken["text"]) == ("Text", label)
            and not counter.get("assertive_live_status") and not spoken.get("assertive_live_status"),
            "Count text or passive description semantics differ")
    if description:
        helper = node(frame, field + "-description")
        require((helper["role"], helper["text"]) == ("Text", description), "Wrong supporting or validation text")
    status = node(frame, field + "-validation-status")
    require(status["role"] == "Status" and status.get("assertive_live_status") is True
            and status["text"] == (description if invalid else ""), "Validation status must only describe invalid text")
    clear_state = None
    if field == EDIT:
        prefix = node(frame, field + "-prefix-description")
        clear = node(frame, action(field))
        require((prefix["role"], prefix["text"]) == ("Text", "井字號")
                and (clear["role"], clear["name"]) == ("Button", "清除標籤")
                and clear["disabled"] is (text == ""), "Wrong passive hash or independent named Clear action")
        clear_state = {"name": clear["name"], "disabled": clear["disabled"]}
    else:
        require(find(frame, action(field))[1] is None, f"Locked example acquired an action: {field}")
    return {"id": field, "value": text, "counter": counter["text"], "invalid": invalid,
            "description": description, "description_ids": extras, "spoken_count": spoken["text"],
            "assertive_status": status["text"], "clear": clear_state}


def shape(face, helper, count, *, rtl, stacked):
    outside = count["x"] - face["x"] if rtl else right(face) - right(count)
    require(close(outside, 16.), "Counter must sit at the 16 dp logical trailing inset")
    if helper is None:
        require(close(count["y"] - bottom(face), 4.), "Counter-only row must start 4 dp below the face")
        return
    start = right(face) - right(helper) if rtl else helper["x"] - face["x"]
    require(close(start, 16.) and close(helper["y"] - bottom(face), 4.) and disjoint(helper, count),
            "Supporting text must start below the face and stay separate from the count")
    if stacked:
        require(close(helper["width"], face["width"] - 32.) and close(count["y"] - bottom(helper), 4.),
                "The full-width supporting text must precede the count by 4 dp in a stacked row")
    else:
        gap = helper["x"] - right(count) if rtl else count["x"] - right(helper)
        require(close(count["y"], helper["y"]) and close(gap, 16.), "The mirrored inline columns need a 16 dp gap")


def geometry(frame, field, appearance, *, rtl, stacked, described):
    face = visible(frame, field + ("-indicator" if appearance == "filled" else "-outline"))
    editor = visible(frame, field)
    helper = visible(frame, field + "-description") if described else None
    count = visible(frame, field + "-counter")
    spoken = visible(frame, field + "-counter-description")
    require(count == spoken, "Visible count and spoken description must share the measured slot")
    require(close(count["height"], 32. if rtl else 16.), "Counter line does not use the text scale")
    shape(face, helper, count, rtl=rtl, stacked=stacked)
    proof = {"face": face, "editor": editor, "helper": helper, "count": count, "spoken": spoken}
    if field != EDIT:
        return proof
    hit = visible(frame, action(field), 47.95)
    trailing = visible(frame, field + "-trailing-icon")
    value = visible(frame, field + "-value-box")
    prefix_id = field + ("-prefix-wrapped" if stacked else "-prefix-visual")
    prefix = visible(frame, prefix_id)
    require(node(frame, prefix_id)["text"] == ("井字號" if stacked else "#"),
            "Wrong separate hash affix artwork")
    side = close(hit["x"], face["x"]) if rtl else close(right(hit), right(face))
    require(close(hit["height"], 48.) and close(hit["width"], 48.) and disjoint(editor, hit) and side
            and close(trailing["x"], hit["x"] + 12.) and close(trailing["y"], hit["y"] + 12.)
            and close(trailing["width"], 24.) and close(trailing["height"], 24.),
            "Clear must keep a separate 48 dp hit with its centered icon")
    if stacked:
        require(close(face["width"], 112.) and editor == face and hit["y"] >= bottom(count) + 3.95
                and value["width"] >= 48. and close(prefix["y"] - bottom(hit), 4.),
                "112 dp fallback lost its editor, relocated action, wrapped affix or count")
    else:
        affix_gap = prefix["x"] - right(value) if rtl else value["x"] - right(prefix)
        require(close(editor["width"] + 48., face["width"])
                and close(hit["y"] + 24., face["y"] + face["height"] / 2.)
                and close(prefix["y"], value["y"]) and close(affix_gap, 2.),
                "Clear or affix misses its inline slot")
    return {**proof, "action": hit, "trailing": trailing, "value": value, "prefix": prefix,
            "prefix_id": prefix_id}


def pixel(rows, x, y):
    x, y = floor(x), floor(y)
    require(0 <= y < len(rows) and 0 <= x < len(rows[0]) // 4, "Pixel outside original PNG")
    return tuple(rows[y][4 * x:4 * x + 4])


def sample_pixels(rows, box):
    return [pixel(rows, x, y) for y in range(ceil(box["y"]), floor(bottom(box)))
            for x in range(ceil(box["x"]), floor(right(box)))]


def composite(background, ink, *, linear):
    def channels(n):
        value = n / 255
        if not linear:
            return value
        return value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4

    def output(value):
        if linear:
            value = 12.92 * value if value <= 0.0031308 else 1.055 * value ** (1 / 2.4) - 0.055
        return floor(255 * value + 0.5)

    alpha = ink[3] / 255
    return tuple(output(channels(paper) * (1 - alpha) + channels(color) * alpha)
                 for paper, color in zip(background[:3], ink[:3], strict=True)) + (255,)


def color_distance(first, second):
    return max(abs(a - b) for a, b in zip(first[:3], second[:3], strict=True))


def painted_ink(rows, box, authored, background):
    samples = sample_pixels(rows, box)
    require(samples and background in samples, "Application text needs transparent space around its glyphs")
    if authored[3] == 255:
        target, tolerance = [tuple(authored)], 3
    else:
        target = [composite(background, authored, linear=policy) for policy in (False, True)]
        tolerance = 5
    ink = sum(any(color_distance(p, color) <= tolerance for color in target) for p in samples)
    require(ink >= 4 and all(color_distance(background, color) >= 14 for color in target),
            f"Text lacks original pixels at its authored color {authored}")
    contrast = sum(color_distance(p, background) >= 12 for p in samples)
    require(contrast >= 10, "Count or supporting text is not visibly rendered")
    return {"rgba": authored, "ink_pixels": ink, "ink_contrast_pixels": contrast,
            "candidate_composites": [list(p) for p in target]}


def focus_and_fill(app, value):
    focus_by_tab(app, EDIT)
    app.wire("Action.fill", {"id": EDIT, "value": value})
    frame = settle(app)
    focus(frame, EDIT)
    return frame


def field_paint(frame, rows, field, measured, *, under_limit, invalid):
    background = tuple(rgba(node(frame, ROUTE), "background"))
    ink = rgba(node(frame, field + "-counter"), "foreground")
    neutral = rgba(node(frame, READ_ONLY + "-counter"), "foreground")
    helper = rgba(node(frame, field + "-description"), "foreground") if measured["helper"] else None
    if field == DISABLED:
        require(ink[3] == 97 and ink != neutral, "Disabled over-limit count must keep muted on-surface ink")
    elif invalid and not under_limit:
        require(ink == helper and ink != neutral and ink[3] == 255,
                "Enabled over-limit count and validation must share opaque error ink")
    else:
        require(ink == neutral and ink[3] == 255, "Under-limit count must keep neutral ink")
        if helper is not None:
            require((helper != neutral) is invalid, "Support and manual validation inks are not distinct")
    result = {"counter": painted_ink(rows, measured["count"], ink, background), "backdrop": list(background)}
    if helper is not None:
        result["helper"] = painted_ink(rows, measured["helper"], helper, background)
    return result


def face_paint(frame, rows, measured, appearance, field_focused, invalid):
    face = measured["face"]
    x, y, w, h = (face[key] for key in ("x", "y", "width", "height"))
    page = tuple(rgba(node(frame, ROUTE), "background"))
    fill = tuple(rgba(node(frame, EDIT + "-container"), "background")) if appearance == "filled" else page
    edge = [pixel(rows, x + w / 2, y + h - step - 0.5) for step in range(5)]
    width = 3 if field_focused else 1
    require(edge[0] != fill and edge[:width] == [edge[0]] * width and edge[width:] == [fill] * (5 - width),
            "Actual border thickness differs from the current focus")
    if invalid:
        authored = rgba(node(frame, EDIT + "-description"), "foreground")
        require(edge[0] == tuple(authored), "Validation did not recolor the actual border")
    if appearance == "filled":
        require(fill != page and pixel(rows, x + .5, y + .5) == page
                and pixel(rows, x + 6, y + .5) == fill and pixel(rows, x + .5, y + h - .5) == edge[0],
                "Filled top corners, body, or square bottom do not paint correctly")
    return {"bottom_edge_dp": width, "samples": [list(value) for value in edge], "fill": list(fill)}


def photo(app, plan, frame, name, *, appearance, value, count, helper, rtl=False, stacked=False,
          invalid=False, focused_editor=False, include_locked=False):
    current = field_state(frame, EDIT, value, count, helper, invalid=invalid)
    measured = geometry(frame, EDIT, appearance, rtl=rtl, stacked=stacked, described=True)
    shot = capture(app, plan, name)
    require((shot["width"], shot["height"], shot["scale"]) == (*SCOPES[plan.mode], 1), "Screenshot size changed")
    _, _, rows = read_png((plan.out / shot["name"]).read_bytes())
    paint = field_paint(frame, rows, EDIT, measured, under_limit=count <= 5, invalid=invalid)
    face = face_paint(frame, rows, measured, appearance, focused_editor, invalid)
    prefix_background = paint["backdrop"] if stacked else face["fill"]
    prefix_ink = rgba(node(frame, measured["prefix_id"]), "foreground")
    paint["affix"] = painted_ink(rows, measured["prefix"], prefix_ink, tuple(prefix_background))
    peers = {}
    if include_locked:
        for field, (value_locked, count_locked, helper_locked) in LOCKED.items():
            state = field_state(frame, field, value_locked, count_locked, helper_locked)
            layout_proof = geometry(frame, field, appearance, rtl=rtl, stacked=False, described=helper_locked is not None)
            peers[field] = {"state": state, "geometry": layout_proof,
                            "paint": field_paint(frame, rows, field, layout_proof, under_limit=count_locked <= 5, invalid=False)}
    return {**shot, "appearance": appearance, "rtl": rtl, "stacked": stacked,
            "state": current, "geometry": measured, "paint": paint, "face_paint": face, "locked": peers}


def replay(plan):
    width, height = SCOPES[plan.mode]
    observations = {}

    def window(app, step):
        count = onscreen_windows(app.proc.pid)
        observations[step] = count
        require(count == 0, f"{step}: owned Gallery window became visible")

    captures, behavior = [], {}
    with launch(plan, plan.mode, width, height) as app:
        window(app, "ready")
        frame = activate(app, SHOW)
        focus(frame, BACK)
        window(app, "opened")
        if plan.mode == "wide":
            captures.append(photo(app, plan, frame, "wide-outlined-seed", appearance="outlined", value=SEED,
                                  count=3, helper=HELP, include_locked=True))
            frame = focus_and_fill(app, OVERFLOW)
            field_state(frame, EDIT, OVERFLOW, 6, LIMIT_ERROR, invalid=True)
            require(node(frame, FEEDBACK)["text"] == "", "Editing must not call Submit or Clear")
            frame = key(app, "enter")
            focus(frame, EDIT)
            require(node(frame, FEEDBACK)["text"] == SUBMITTED, "Caller Enter did not submit the retained soft-overflow value")
            behavior["soft_overflow_submit"] = field_state(frame, EDIT, OVERFLOW, 6, LIMIT_ERROR, invalid=True)
            behavior["submit_feedback"] = node(frame, FEEDBACK)["text"]
            captures.append(photo(app, plan, frame, "wide-outlined-overflow", appearance="outlined", value=OVERFLOW,
                                  count=6, helper=LIMIT_ERROR, invalid=True, focused_editor=True))
            window(app, "submitted_soft_overflow")

            frame = activate(app, ERROR)
            behavior["manual_priority_during_overflow"] = field_state(frame, EDIT, OVERFLOW, 6, MANUAL_ERROR, invalid=True)
            frame = focus_and_fill(app, SEED)
            captures.append(photo(app, plan, frame, "wide-outlined-manual-error", appearance="outlined", value=SEED,
                                  count=3, helper=MANUAL_ERROR, invalid=True, focused_editor=True))
            activate(app, ERROR)
            frame = focus_and_fill(app, OVERFLOW[:-1])
            behavior["exact_limit"] = field_state(frame, EDIT, OVERFLOW[:-1], 5, HELP)
            focus_and_fill(app, SEED)
            activate(app, APPEARANCE)
            focus_by_tab(app, EDIT)
            frame = settle(app)
            captures.append(photo(app, plan, frame, "wide-filled-seed", appearance="filled", value=SEED,
                                  count=3, helper=HELP, focused_editor=True, include_locked=True))

            frame = key(app, "tab")
            focus(frame, action(EDIT))
            frame = key(app, "space")
            behavior["clear"] = field_state(frame, EDIT, "", 0, HELP)
            behavior["clear_feedback"] = node(frame, FEEDBACK)["text"]
            require(behavior["clear_feedback"] == CLEARED, "Named Clear did not run its distinct callback")
            app.wire("Action.activate", {"id": action(EDIT)}, "disabled")
            focus_by_tab(app, READ_ONLY)
            app.wire("Action.fill", {"id": READ_ONLY, "value": "改"}, "action_failed")
            app.wire("Action.fill", {"id": DISABLED, "value": "改"}, "disabled")
            frame = settle(app)
            behavior["locked"] = {field: field_state(frame, field, value, count, help_text)
                                  for field, (value, count, help_text) in LOCKED.items()}
            require(node(frame, FEEDBACK)["text"] == CLEARED, "Blocked commands changed the feedback")
            window(app, "clear_and_locked_fields")
        else:
            for setting in SETTINGS:
                activate(app, setting)
            focus_by_tab(app, EDIT)
            frame = settle(app)
            captures.append(photo(app, plan, frame, "compact-outlined-rtl-200", appearance="outlined", value=SEED,
                                  count=3, helper=HELP, rtl=True, focused_editor=True))
            activate(app, APPEARANCE)
            frame = focus_and_fill(app, OVERFLOW)
            captures.append(photo(app, plan, frame, "compact-filled-overflow-rtl-200", appearance="filled", value=OVERFLOW,
                                  count=6, helper=LIMIT_ERROR, rtl=True, invalid=True, focused_editor=True))
            window(app, "compact_overflow")
            activate(app, NARROW)
            frame = focus_and_fill(app, SEED)
            captures.append(photo(app, plan, frame, "compact-filled-stacked-112", appearance="filled", value=SEED,
                                  count=3, helper=HELP, rtl=True, stacked=True, focused_editor=True))
            frame = key(app, "tab")
            focus(frame, action(EDIT))
            frame = key(app, "enter")
            behavior["relocated_clear"] = field_state(frame, EDIT, "", 0, HELP)
            behavior["clear_feedback"] = node(frame, FEEDBACK)["text"]
            require(behavior["clear_feedback"] == CLEARED, "Relocated Clear missed its actual keyboard action")
            window(app, "compact_relocated_clear")
        window(app, "captures_saved")
        frame = activate(app, BACK)
        focus(frame, SHOW)
        require(find(frame, ROUTE)[1] is None, "Closed route still mounted")
        window(app, "closed")
    require(app.proc.returncode is not None, "Headless Gallery child was not reaped")
    return {"viewport": [width, height], "captures": captures, "behavior": behavior}, {
        "pid": app.proc.pid, "visible_windows": observations, "reaped": True,
    }


def self_test():
    def rect(x, y, w, h):
        return {"x": x, "y": y, "width": w, "height": h}

    def rejected(action, text):
        try:
            action()
        except (ValueError, RuntimeError, AssertionError) as error:
            require(text in str(error), f"Wrong negative-control result: {error}")
            return
        require(False, f"Accepted bad geometry: {text}")

    face = rect(20, 20, 280, 96)
    normal, count = rect(36, 120, 180, 64), rect(232, 120, 52, 32)
    shape(face, normal, count, rtl=False, stacked=False)

    def mirror(part):
        return {**part, "x": face["x"] + right(face) - right(part)}

    shape(face, mirror(normal), mirror(count), rtl=True, stacked=False)
    narrow, narrow_help, narrow_count = rect(20, 20, 112, 96), rect(36, 120, 80, 96), rect(36, 220, 52, 32)
    shape(narrow, narrow_help, narrow_count, rtl=True, stacked=True)
    shape(face, None, count, rtl=False, stacked=False)
    rejected(lambda: shape(face, normal, {**count, "x": 227, "width": 57}, rtl=False, stacked=False), "16 dp gap")
    rejected(lambda: shape(narrow, narrow_help, {**narrow_count, "y": 216}, rtl=True, stacked=True),
             "4 dp in a stacked row")
    paper, on_surface = (253, 247, 255, 255), (29, 27, 32, 97)
    disabled = composite(paper, on_surface, linear=False)
    require(disabled == (168, 163, 170, 255), "Wrong disabled encoded blend")
    rows = [bytes(channel for x in range(16) for channel in (disabled if 2 <= x < 8 and 2 <= y < 6 else paper))
            for y in range(8)]
    require(painted_ink(rows, rect(0, 0, 16, 8), on_surface, paper)["ink_pixels"] == 24,
            "Synthetic disabled contrast control failed to recount its pixels")
    print("PASS geometry, negative spacing and synthetic alpha pixel checks. No app started.")


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
    parser.add_argument("--self-test", action="store_true", help="Check pure predicates without an app")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    for key in ("source_root", "source_manifest", "source_manifest_sha", "binary_manifest", "binary_manifest_sha"):
        if getattr(args, key) is None:
            parser.error(f"--{key.replace('_', '-')} is required")
    root = args.source_root.resolve(strict=True)
    bundle = args.binary_manifest.resolve(strict=True).parent
    source = pins(root, args.source_manifest, args.source_manifest_sha)
    require({
        "crates/surface-app/src/accessibility.rs", "crates/surface-app/src/editing_runtime.rs",
        "crates/surface-app/src/editor_paint.rs", "crates/surface-material/src/outlined_text_field.rs",
        "crates/surface-material/src/text_field_adornments.rs",
        "crates/surface-material/src/text_field_counter.rs",
        "examples/material-gallery/src/text_field_counter_demo.rs",
    } <= source.keys(), "Source manifest does not contain the text field counter route")
    binaries = pins(bundle, args.binary_manifest, args.binary_manifest_sha)
    gallery, cli = (bundle / name for name in ("surface-material-gallery", "surface-devtools"))
    require(binaries.keys() == {gallery.name, cli.name}
            and all(os.access(path, os.X_OK) for path in (gallery, cli)), "Wrong executable bundle")
    provenance = {"source": {"manifest_sha256": args.source_manifest_sha, "files": len(source)},
                  "binary_manifest_sha256": args.binary_manifest_sha, "binaries": binaries}
    if args.verify_inputs_only:
        require(args.out is None, "Preflight does not create an evidence directory")
        print(json.dumps(provenance, indent=2))
        return
    require(sys.platform == "darwin" and args.out is not None, "Replay requires macOS and a fresh --out")
    out = args.out.resolve()
    private_root = Path("/tmp").resolve()
    require(out != private_root and out.is_relative_to(private_root),
            "Output must use a new private /tmp directory")
    out.mkdir(mode=0o700)
    replays, processes = [], []
    for index in range(args.repeats):
        directory = out / f"replay-{index + 1}"
        directory.mkdir(mode=0o700)
        results = {mode: replay(SimpleNamespace(gallery=gallery, cli=cli, out=directory, mode=mode))
                   for mode in SCOPES}
        replays.append({mode: part[0] for mode, part in results.items()})
        processes.append({mode: part[1] for mode, part in results.items()})
    require(all(result == replays[0] for result in replays), "PNG bytes or measured geometry differ")
    require(pins(root, args.source_manifest, args.source_manifest_sha) == source
            and pins(bundle, args.binary_manifest, args.binary_manifest_sha) == binaries,
            "Checked inputs changed during replay")
    result = {**provenance, "schema": "surface.test.text-field-counter.native.v1",
              "transports": TRANSPORTS, "replays": replays, "processes": processes,
              "semantics": "Styled Runtime DOM through native headless DevTools"}
    (out / "native-text-field-counter-report.json").write_text(json.dumps(result, indent=2, ensure_ascii=False))
    paths = sorted(path for path in out.rglob("*") if path.is_file())
    (out / "SHA256SUMS").write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in paths))
    print(f"{args.repeats} out of {args.repeats} headless replays passed at {out}")


if __name__ == "__main__":
    main()
