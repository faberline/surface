#!/usr/bin/env python3
"""Measure the Outlined Text Field on the pinned headless Surface Gallery."""

import argparse
import json
from math import ceil, floor
import os
from pathlib import Path
import re
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

PREFIX = "outlined-text-field-demo-"
SHOW, ROUTE, BACK = (PREFIX + value for value in ("show", "route", "back"))
EDIT, READ_ONLY, DISABLED, LONG = (
    PREFIX + value for value in ("editable", "read-only", "disabled", "long-label")
)
SETTINGS = [PREFIX + part for part in ("theme", "rtl", "large-text", "error")]
STATUS = PREFIX + "status"
HELP = EDIT + "-description"
NAMES = {
    EDIT: "專案名稱", READ_ONLY: "唯讀名稱", DISABLED: "停用名稱",
    LONG: "輸入團隊成員在共用專案清單中容易辨認的完整名稱",
}
SCOPES = {"wide": (960, 960), "compact": (320, 960)}


def pins(root, manifest, expected):
    require(re.fullmatch(r"[a-f0-9]{64}", expected) and digest(manifest) == expected,
            "Reviewed manifest digest differs")
    entries = {}
    for line in manifest.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        relative = Path(name)
        path = root / name
        require(not relative.is_absolute() and ".." not in relative.parts and name not in entries
                and path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root)
                and digest(path) == checksum, f"Invalid manifest entry: {name}")
        entries[name] = checksum
    return entries


def role_checks(frame, *, error=False):
    for target, name in NAMES.items():
        field = node(frame, target)
        require((field["role"], field["name"]) == ("TextInput", name), f"Wrong editor {target}")
        require(field["disabled"] is (target == DISABLED), f"Wrong disabled state on {target}")
        require(sum(item.get("stable_id") == target for item in frame["dom"]["nodes"]) == 1,
                f"More than one stable editor named {target}")
    editor = node(frame, EDIT)
    require(editor.get("invalid", False) is error and editor.get("described_by_text") == HELP,
            f"Missing error/description on primary editor: {editor}")
    helper = node(frame, HELP)
    expected = "請輸入有效的專案名稱。" if error else "請輸入中文或英文名稱。"
    require(helper["role"] == "Text" and helper["text"] == expected, "Wrong description")
    status = node(frame, EDIT + "-validation-status")
    require(status["role"] == "Status" and status.get("assertive_live_status") is True
            and status["text"] == (expected if error else ""),
            "Wrong authored validation status")
    visible(frame, EDIT + "-validation-status")
    return {"invalid": error, "description": expected, "assertive_status": status["text"]}


def sample(rows, x, y):
    x, y = floor(x), floor(y)
    require(0 <= y < len(rows) and 0 <= x < len(rows[0]) // 4, "Sample outside the original PNG")
    return list(rows[y][4*x:4*x+4])


def test_border(frame, rows, *, focused, rtl, field=EDIT, above=False):
    face = visible(frame, field, 55.99)
    outline = layout(frame, field + "-outline")["bounds"]
    require(outline == face, "Outline and editor hit geometry differ")
    x, y, w = (face[key] for key in ("x", "y", "width"))
    label = layout(frame, field + "-floating-label")["bounds"]
    if above:
        caption = visible(frame, field + "-above-label")
        require(label["width"] == 0 and caption["y"] + caption["height"] < y,
                "Long label did not move fully above the outline")
        gap_x = x + 18
    else:
        require(label["width"] > 0 and label["y"] < y < label["y"] + label["height"],
                "The measured float layout does not cross the border")
        gap_x = label["x"] + label["width"] + 2 if rtl else label["x"] - 2
    solid_x = x + 7 if rtl else x + w - 7
    band = [sample(rows, solid_x, y + n + 0.5) for n in range(5)]
    background = sample(rows, x + w - 6, y + 10)
    gap = sample(rows, gap_x, y + 0.5)
    thickness = 3 if focused else 1
    require(all(part == band[0] for part in band[:thickness])
            and band[0] != background and band[thickness] == background,
            f"Expected a {thickness} dp border from actual pixels, got {band}")
    require(gap == (background if focused and not above else band[0]),
            f"Measured outline gap has the wrong pixel: {gap}, band {band}")
    return {"face": face, "label": label, "band": band, "gap": gap, "background": background}


def photo(app, plan, frame, name, **parts):
    receipt = capture(app, plan, name)
    _, _, rows = read_png((plan.out / receipt["name"]).read_bytes())
    result = {**receipt, "outline": test_border(frame, rows, **parts)}
    if parts.get("field", EDIT) == EDIT:
        result["disabled_value"] = test_disabled_value(frame, rows, rtl=parts["rtl"])
    return result


def test_disabled_value(frame, rows, *, rtl):
    face = visible(frame, DISABLED, 55.99)
    box = visible(frame, DISABLED + "-disabled-value")
    x, y, width, height = (box[name] for name in ("x", "y", "width", "height"))
    require(abs(x - face["x"] - 16) < 0.1 and abs(width - face["width"] + 32) < 0.1,
            "Disabled paint does not use the editor's inset")
    require(abs((y + height / 2) - (face["y"] + face["height"] / 2)) < 0.1,
            "Disabled value does not share the editor's vertical center")
    background = sample(rows, face["x"] + 6, y + height / 2)
    ink = []
    for py in range(ceil(y), floor(y + height)):
        for px in range(ceil(x), floor(x + width)):
            rgba = sample(rows, px, py)
            if max(abs(a - b) for a, b in zip(rgba[:3], background[:3])) > 12:
                ink.append(px)
    require(len(ink) > 40, "Disabled value has no visible paint in the measured box")
    nearest = (x + width - max(ink)) if rtl else (min(ink) - x)
    require(nearest < 12, "Disabled value does not align to the logical start")
    return {"bounds": box, "ink_pixels": len(ink), "ink_x": [min(ink), max(ink)]}


def replay(plan, mode):
    width, height = SCOPES[mode]
    observations = []

    def window(app):
        count = onscreen_windows(app.proc.pid)
        observations.append(count)
        require(count == 0, "Owned headless Gallery window became visible")

    captures = []
    with launch(plan, mode, width, height) as app:
        window(app)
        focus_by_tab(app, SHOW)
        frame = key(app, "enter")
        focus(frame, BACK)
        window(app)
        if mode == "wide":
            role_checks(frame)
            captures.append(photo(app, plan, frame, "wide-idle", focused=False, rtl=False))
            original_face = layout(frame, EDIT)["bounds"]
            focus_by_tab(app, EDIT)
            frame = settle(app)
            require(layout(frame, EDIT)["bounds"] == original_face, "Focus moved the editor")
            captures.append(photo(app, plan, frame, "wide-focused", focused=True, rtl=False))
            app.wire("Action.fill", {"id": EDIT, "value": "測試專案"})
            frame = key(app, "enter")
            require(node(frame, EDIT)["text"] == "測試專案"
                    and node(frame, STATUS)["text"] == "送出次數：1", "Enter or edit was lost")
            app.wire("Action.fill", {"id": READ_ONLY, "value": "不可寫入"}, "action_failed")
            app.wire("Action.fill", {"id": DISABLED, "value": "不可寫入"}, "disabled")
            focus(settle(app), EDIT)
            frame = key(app, "tab")
            focus(frame, READ_ONLY)
            frame = key(app, "tab")
            focus(frame, LONG)
        else:
            for control in SETTINGS:
                activate(app, control)
            focus_by_tab(app, EDIT)
            frame = settle(app)
            role_checks(frame, error=True)
            captures.append(photo(app, plan, frame, "compact-error", focused=True, rtl=True))
            focus_by_tab(app, LONG)
            frame = settle(app)
            captures.append(photo(app, plan, frame, "compact-long-label", focused=True, rtl=True,
                                  field=LONG, above=True))
        window(app)
        frame = activate(app, BACK)
        focus(frame, SHOW)
        require(find(frame, ROUTE)[1] is None, "Closed form still mounted")
        window(app)
    require(app.proc.returncode is not None, "Headless Gallery child was not reaped")
    return {"viewport": [width, height], "captures": captures}, {
        "pid": app.proc.pid, "visible_windows": observations, "reaped": True,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--source-manifest", type=Path, required=True)
    parser.add_argument("--source-manifest-sha", required=True)
    parser.add_argument("--binary-manifest", type=Path, required=True)
    parser.add_argument("--binary-manifest-sha", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--repeats", type=int, choices=(1, 2), default=2)
    args = parser.parse_args()
    root = args.source_root.resolve(strict=True)
    source = pins(root, args.source_manifest, args.source_manifest_sha)
    require({"crates/surface-app/src/editor_paint.rs",
             "crates/surface-material/src/outlined_text_field.rs",
             "examples/material-gallery/src/outlined_text_field_demo.rs"} <= source.keys(),
            "Manifest omits part of the Outlined Text Field")
    binaries = pins(args.binary_manifest.resolve().parent,
                    args.binary_manifest, args.binary_manifest_sha)
    gallery, cli = (args.binary_manifest.resolve().parent / name
                    for name in ("surface-material-gallery", "surface-devtools"))
    require(binaries.keys() == {gallery.name, cli.name}
            and all(os.access(path, os.X_OK) for path in (gallery, cli)), "Wrong executable bundle")
    out = args.out.resolve()
    require(out != Path("/tmp").resolve() and out.is_relative_to(Path("/tmp").resolve()),
            "Output must use a new private /tmp directory")
    out.mkdir(mode=0o700)
    replays, processes = [], []
    for i in range(args.repeats):
        directory = out / f"replay-{i + 1}"
        directory.mkdir(mode=0o700)
        plan = SimpleNamespace(gallery=gallery, cli=cli, out=directory)
        results = {profile: replay(plan, profile) for profile in SCOPES}
        replays.append({key: part[0] for key, part in results.items()})
        processes.append({key: part[1] for key, part in results.items()})
    require(all(result == replays[0] for result in replays), "PNG bytes or measured geometry differ")
    require(pins(root, args.source_manifest, args.source_manifest_sha) == source
            and pins(args.binary_manifest.resolve().parent, args.binary_manifest,
                     args.binary_manifest_sha) == binaries, "Inputs changed during screenshot replay")
    result = {
        "source": {"manifest_sha256": args.source_manifest_sha, "files": len(source)},
        "binaries": binaries, "replays": replays, "processes": processes,
        "transports": ["Page.screenshot", "CLI", "MCP"],
    }
    (out / "native-outlined-report.json").write_text(json.dumps(result, indent=2, ensure_ascii=False))
    paths = sorted(path for path in out.rglob("*") if path.is_file())
    (out / "SHA256SUMS").write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n"
                                         for path in paths))
    print(f"{args.repeats} out of {args.repeats} headless replays passed at {out}")


if __name__ == "__main__":
    main()
