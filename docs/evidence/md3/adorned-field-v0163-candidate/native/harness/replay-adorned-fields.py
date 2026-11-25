#!/usr/bin/env python3
"""Replay the pinned adorned text field in a native headless Gallery."""

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

PREFIX = "text-field-adornments-demo-"
SHOW, ROUTE, BACK, EDIT, READ_ONLY, DISABLED, FEEDBACK = (
    PREFIX + key for key in ("show", "route", "back", "editable", "read-only", "disabled", "feedback")
)
APPEARANCE, NARROW = (PREFIX + name for name in ("appearance", "narrow"))
SETTINGS = [PREFIX + name for name in ("theme", "rtl", "large-text", "error")]
NAMES = [
    (EDIT, "方案費用", "套用建議金額"),
    (READ_ONLY, "目前方案費用", "查看現有費率"),
    (DISABLED, "停用方案費用", "套用停用方案金額"),
]
HELP = "請輸入不含單位的金額。"
ERROR = "請輸入大於零的金額。"
SUBMITTED = "已送出欄位內的金額。"
APPLIED = "已套用 24 歐元的每月費用。"
REVIEWED = "目前費率為每月 18 歐元。"
SCOPES = {"wide": (960, 1120), "compact": (320, 1120)}
TRANSPORTS = ["Page.screenshot", "CLI", "MCP"]


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


def role_checks(frame, *, error):
    for field, label, command in NAMES:
        count = sum(item.get("stable_id") == field for item in frame["dom"]["nodes"])
        editor, button = node(frame, field), node(frame, action(field))
        require(count == 1 and (editor["role"], editor["name"]) == ("TextInput", label),
                f"Wrong single editor {field}")
        require((button["role"], button["name"]) == ("Button", command),
                f"Wrong independent action {field}")
        require(editor["disabled"] is (field == DISABLED)
                and button["disabled"] is (field == DISABLED),
                f"Wrong disabled state {field}")
    editor = node(frame, EDIT)
    descriptions = [EDIT + "-prefix-description", EDIT + "-suffix-description"]
    require(editor.get("additional_description_text_targets") == descriptions,
            "Ordered affix IDs differ in the authored editor")
    for target, word in zip(descriptions, ["歐元", "每月"]):
        metadata = node(frame, target)
        require(metadata["role"] == "Text" and metadata["text"] == word,
                f"Incorrect spoken affix {target}")
        visible(frame, target)
    helper = node(frame, EDIT + "-description")
    text = ERROR if error else HELP
    require(editor.get("described_by_text") == EDIT + "-description"
            and editor.get("invalid", False) is error
            and helper["role"] == "Text" and helper["text"] == text,
            "Wrong scoped support/validation authoring")
    status = node(frame, EDIT + "-validation-status")
    require(status["role"] == "Status" and status.get("assertive_live_status") is True
            and status["text"] == (text if error else ""),
            "Wrong assertive validation authoring")
    require(node(frame, EDIT + "-leading-icon")["role"] == "Group",
            "Leading art must have no action role")
    return {"affix_text": ["歐元", "每月"], "description": text, "invalid": error,
            "assertive_status": status["text"], "disabled_action": action(DISABLED)}


def geometry(frame, appearance, *, rtl, narrow):
    boundary = "indicator" if appearance == "filled" else "outline"
    face = visible(frame, EDIT + "-" + boundary)
    editor = visible(frame, EDIT)
    action_box = visible(frame, action(EDIT), 47.95)
    value = visible(frame, EDIT + "-value-box")
    leading = box(frame, EDIT, "leading-icon")
    trailing = visible(frame, EDIT + "-trailing-icon")
    require((face["height"] == 96. if rtl and appearance == "filled" else
             face["height"] >= 56.), "Field face has an incorrect scaled height")
    require(close(action_box["width"], 48.) and close(action_box["height"], 48.)
            and disjoint(editor, action_box), "Action and editor must have distinct hit boxes")
    require(close(trailing["width"], 24.) and close(trailing["height"], 24.)
            and close(trailing["x"], action_box["x"] + 12.)
            and close(trailing["y"], action_box["y"] + 12.), "Trailing icon is off-center")
    require(close(action_box["x"], face["x"]) if rtl else close(right(action_box), right(face)),
            "Action must sit on the logical trailing side")
    logical = {"face": face, "editor": editor, "action": action_box, "value": value,
               "leading": leading, "trailing": trailing}
    if narrow:
        require(close(face["width"], 112.) and face == editor and leading["width"] <= 0.05
                and action_box["y"] >= bottom(face) + 3.95,
                "Narrow profile must retain a full editor and relocate the action")
        require(value["width"] >= 48., "Narrow value lane cannot fit text and caret")
        logical["affixes"] = [
            visible(frame, EDIT + "-" + part + "-wrapped") for part in ("prefix", "suffix")
        ]
        require(all(rect["y"] >= bottom(face) for rect in logical["affixes"]),
                "Wrapped affixes entered the 112 dp editor")
    else:
        require(close(editor["width"] + 48., face["width"])
                and close(action_box["y"] + 24., face["y"] + face["height"] / 2.),
                "Inline geometry must leave a 48 dp trailing column")
        require(close(leading["width"], 24.) and close(leading["height"], 24.)
                and close(right(face) - right(leading) if rtl else leading["x"] - face["x"], 12.),
                "Leading icon must sit 12 dp inside logical start")
        prefix = visible(frame, EDIT + "-prefix-visual")
        suffix = visible(frame, EDIT + "-suffix-visual")
        require(close(prefix["y"], value["y"]) and close(suffix["y"], value["y"]),
                "Affix and value line boxes do not align")
        if rtl:
            require(close(value["x"] - right(suffix), 2.)
                    and close(prefix["x"] - right(value), 2.), "Affixes did not mirror in RTL")
        else:
            require(close(value["x"] - right(prefix), 2.)
                    and close(suffix["x"] - right(value), 2.), "LTR affix gaps differ")
        logical["affixes"] = [prefix, suffix]
    return logical


def rgba(rows, x, y):
    x, y = floor(x), floor(y)
    require(0 <= y < len(rows) and 0 <= x < len(rows[0]) // 4, "Pixel outside the original PNG")
    return list(rows[y][4 * x:4 * x + 4])


def contrast_pixels(rows, rect, background):
    left, top = max(0, ceil(rect["x"])), max(0, ceil(rect["y"]))
    end, stop = min(len(rows[0]) // 4, floor(right(rect))), min(len(rows), floor(bottom(rect)))
    return sum(
        max(abs(a - b) for a, b in zip(rows[y][4 * x:4 * x + 3], background[:3])) >= 20
        for y in range(top, stop) for x in range(left, end)
    )


def photo(app, plan, frame, name, *, appearance, rtl=False, narrow=False, editor_focused=False):
    drawn = geometry(frame, appearance, rtl=rtl, narrow=narrow)
    proof = capture(app, plan, name)
    require((proof["width"], proof["height"], proof["scale"]) == (*SCOPES[plan.mode], 1),
            "Screenshot dimensions or scale differ from the requested viewport")
    _, _, rows = read_png((plan.out / proof["name"]).read_bytes())
    face = drawn["face"]
    x, y = face["x"], face["y"]
    width, height = face["width"], face["height"]
    background = node(frame, ROUTE)["style"]["background"]
    fill = node(frame, EDIT + "-container")["style"]["background"] if appearance == "filled" else background
    stroke = [rgba(rows, x + width / 2, y + height - i - 0.5) for i in range(5)]
    thickness = 3 if editor_focused else 1
    require(stroke[0] != fill and stroke[:thickness] == [stroke[0]] * thickness
            and stroke[thickness:] == [fill] * (5 - thickness), f"{name}: wrong bottom edge {stroke}")
    if appearance == "filled":
        require(fill != background and rgba(rows, x + 0.5, y + 0.5) == background
                and rgba(rows, x + 6, y + 0.5) == fill
                and rgba(rows, x + 0.5, y + height - 0.5) == stroke[0],
                f"{name}: filled top or bottom corners are wrong")
    if appearance == "outlined" and editor_focused:
        label = visible(frame, EDIT + "-floating-label")
        require(rgba(rows, label["x"] - 2, y + 0.5) == background
                and rgba(rows, label["x"] - 6, y + 0.5) == stroke[0],
                f"{name}: absent transparent border notch")
    glyphs = {
        "trailing": contrast_pixels(rows, drawn["trailing"], background if narrow else fill)
    }
    if not narrow:
        glyphs["leading"] = contrast_pixels(rows, drawn["leading"], fill)
    require(all(count > 16 for count in glyphs.values()), f"{name}: invisible vector glyph {glyphs}")
    if narrow or editor_focused:
        glyphs["affixes"] = [
            contrast_pixels(rows, rect, background if narrow else fill) for rect in drawn["affixes"]
        ]
        require(all(count > 8 for count in glyphs["affixes"]), f"{name}: missing affix paint")
    return {**proof, "geometry": drawn, "paint": {"band": stroke, "glyph_pixels": glyphs}}


def field_snapshot(frame, value, feedback):
    require(node(frame, EDIT)["text"] == value and node(frame, FEEDBACK)["text"] == feedback,
            f"Expected raw value {value!r} and separate feedback {feedback!r}")
    return {"value": value, "feedback": feedback}


def replay(plan):
    width, height = SCOPES[plan.mode]
    observations = {}

    def window(app, step):
        count = onscreen_windows(app.proc.pid)
        observations[step] = count
        require(count == 0, f"{step}: owned Gallery window became visible")

    captures = []
    behavior = {}
    with launch(plan, plan.mode, width, height) as app:
        window(app, "ready")
        frame = activate(app, SHOW)
        focus(frame, BACK)
        window(app, "opened")
        if plan.mode == "wide":
            behavior["authored_relations"] = role_checks(frame, error=False)
            captures.append(photo(app, plan, frame, "wide-outlined-idle", appearance="outlined"))
            focus_by_tab(app, EDIT)
            frame = settle(app)
            captures.append(photo(app, plan, frame, "wide-outlined-focused", appearance="outlined",
                                  editor_focused=True))
            app.wire("Action.fill", {"id": EDIT, "value": "37"})
            behavior["typed"] = field_snapshot(settle(app), "37", "")
            frame = key(app, "enter")
            focus(frame, EDIT)
            behavior["submitted"] = field_snapshot(frame, "37", SUBMITTED)
            frame = key(app, "tab")
            focus(frame, action(EDIT))
            frame = key(app, "space")
            focus(frame, action(EDIT))
            behavior["suggestion"] = field_snapshot(frame, "24", APPLIED)
            frame = key(app, "tab")
            focus(frame, READ_ONLY)
            require(node(frame, READ_ONLY)["text"] == "18", "Read-only value differs")
            app.wire("Action.fill", {"id": READ_ONLY, "value": "99"}, "action_failed")
            app.wire("Action.fill", {"id": DISABLED, "value": "99"}, "disabled")
            app.wire("Action.activate", {"id": action(DISABLED)}, "disabled")
            frame = key(app, "tab")
            focus(frame, action(READ_ONLY))
            frame = key(app, "enter")
            require(node(frame, READ_ONLY)["text"] == "18"
                    and node(frame, DISABLED)["text"] == "30", "Locked values changed")
            behavior["reviewed"] = field_snapshot(frame, "24", REVIEWED)
            frame = key(app, "tab", reverse=True)
            focus(frame, READ_ONLY)
            window(app, "edited_submitted_and_acted")
            activate(app, APPEARANCE)
            focus_by_tab(app, EDIT)
            frame = settle(app)
            require(node(frame, EDIT)["text"] == "24", "Appearance replaced the committed value")
            captures.append(photo(app, plan, frame, "wide-filled-focused", appearance="filled",
                                  editor_focused=True))
        else:
            for setting in SETTINGS:
                activate(app, setting)
            focus_by_tab(app, EDIT)
            frame = settle(app)
            behavior["authored_relations"] = role_checks(frame, error=True)
            captures.append(photo(app, plan, frame, "compact-outlined-error", appearance="outlined",
                                  rtl=True, editor_focused=True))
            window(app, "outlined_error")
            activate(app, APPEARANCE)
            focus_by_tab(app, EDIT)
            frame = settle(app)
            captures.append(photo(app, plan, frame, "compact-filled-error", appearance="filled",
                                  rtl=True, editor_focused=True))
            activate(app, NARROW)
            focus_by_tab(app, EDIT)
            frame = key(app, "tab")
            focus(frame, action(EDIT))
            frame = key(app, "space")
            focus(frame, action(EDIT))
            behavior["relocated_suggestion"] = field_snapshot(frame, "24", APPLIED)
            captures.append(photo(app, plan, frame, "compact-narrow-action", appearance="filled",
                                  rtl=True, narrow=True))
            window(app, "relocated_action")
        window(app, "captures_saved")
        frame = activate(app, BACK)
        focus(frame, SHOW)
        require(find(frame, ROUTE)[1] is None, "Closed route still mounted")
        window(app, "closed")
    require(app.proc.returncode is not None, "Headless Gallery child was not reaped")
    return {"viewport": [width, height], "captures": captures, "behavior": behavior}, {
        "pid": app.proc.pid, "visible_windows": observations, "reaped": True,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--source-manifest", type=Path, required=True)
    parser.add_argument("--source-manifest-sha", required=True)
    parser.add_argument("--binary-manifest", type=Path, required=True)
    parser.add_argument("--binary-manifest-sha", required=True)
    parser.add_argument("--verify-inputs-only", action="store_true")
    parser.add_argument("--out", type=Path)
    parser.add_argument("--repeats", type=int, choices=(1, 2), default=2)
    args = parser.parse_args()
    root = args.source_root.resolve(strict=True)
    bundle = args.binary_manifest.resolve(strict=True).parent
    source = pins(root, args.source_manifest, args.source_manifest_sha)
    require({
        "crates/surface-app/src/accessibility.rs", "crates/surface-app/src/editing_runtime.rs",
        "crates/surface-app/src/editor_paint.rs", "crates/surface-material/src/outlined_text_field.rs",
        "crates/surface-material/src/text_field_adornments.rs",
        "examples/material-gallery/src/text_field_adornments_demo.rs",
    } <= source.keys(), "Source manifest does not contain the adorned fields")
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
    result = {**provenance, "schema": "surface.test.text-field-adornments.native.v1",
              "transports": TRANSPORTS, "replays": replays, "processes": processes,
              "semantics": "Styled Runtime DOM through native headless DevTools"}
    (out / "native-adorned-fields-report.json").write_text(json.dumps(result, indent=2, ensure_ascii=False))
    paths = sorted(path for path in out.rglob("*") if path.is_file())
    (out / "SHA256SUMS").write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in paths))
    print(f"{args.repeats} out of {args.repeats} headless replays passed at {out}")


if __name__ == "__main__":
    main()
