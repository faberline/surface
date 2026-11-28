#!/usr/bin/env python3
"""Verify a synthetic, distinct field palette on the native headless Gallery."""

import argparse
from collections import Counter
import json
import os
from pathlib import Path
from runpy import run_path
import shlex
import sys
from tempfile import TemporaryDirectory
import tomllib
from types import SimpleNamespace

BASE = Path("/tmp/surface-md3-v0172-replay.OgmvVGia/native-helpers")
BASE_SHA = "34291d67bd5e47502553bcbf0171b424b4bee2f9be0fa7e5f2873b7c235d7575"
sys.path.insert(0, str(BASE))
from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

prior = run_path(str(BASE / "replay-placeholder.py"))
SHOW, BACK, ROUTE, THEME, FILLED, RTL, LARGE, ABOVE, SET, CLEAR, EDITOR, SEED, SHORT, LONG = (
    prior[name] for name in ("SHOW", "BACK", "ROUTE", "THEME", "FILLED", "RTL", "LARGE", "ABOVE",
                             "SET", "CLEAR", "EDITOR", "SEED", "SHORT_LABEL", "LONG_LABEL")
)
PAGE = (248, 247, 250, 255)
PAGE_INK = (28, 23, 38, 255)
FIELD = (19, 45, 47, 255)
FIELD_INK = (249, 220, 126, 255)
HIGHLIGHT = (63, 24, 97, 255)
HIGHLIGHT_INK = (237, 239, 254, 255)
TRANSPORTS = prior["TRANSPORTS"]
SCENES = {
    "wide": (960, 900, {
        "outlined": ("idle", "focused", "selected"),
        "filled": ("idle", "focused", "selected"),
    }),
    "compact": (320, 900, {
        "outlined": ("focused",),
        "above": ("focused",),
    }),
}
REPORT = "native-field-pair-preview.json"


def check_input(args):
    prior["pins"](BASE.resolve(), BASE / "SHA256SUMS", BASE_SHA)
    source = args.source.resolve(strict=True)
    files = prior["pins"](source, args.source_manifest, args.source_sha)
    require(set(files) == {path.relative_to(source).as_posix()
                           for path in source.rglob("*") if path.is_file()}, "Incomplete source seal")
    require(tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]["version"]
            == "0.1.72", "Expected v0.1.72 source")
    manifest = args.binary_manifest.resolve(strict=True)
    binary_files = prior["pins"](manifest.parent, manifest, args.binary_sha)
    require(set(binary_files) == {"surface-material-gallery", "surface-devtools"},
            "Expected native Gallery and DevTools executable")
    gallery, cli = (manifest.parent / name for name in ("surface-material-gallery", "surface-devtools"))
    require(os.access(gallery, os.X_OK) and os.access(cli, os.X_OK), "Native inputs are not executable")
    return gallery, cli


def name(profile, scene, phase):
    return f"{profile}-{scene}-{phase}"


def console_args(width, height):
    return ["--headless", "--width", str(width), "--height", str(height)]


def guard_text(gallery, width, height):
    checks = [
        f'[ "${index}" = {shlex.quote(arg)} ]'
        for index, arg in enumerate(console_args(width, height), 1)
    ]
    return ("#!/bin/sh\n[ \"$#\" -eq 5 ] && " + " && ".join(checks) + " || exit 71\n"
            + f'exec {shlex.quote(str(gallery))} "$@" --forced-colors field-pair\n')


def geometry(frame, profile, scene):
    filled, above = scene == "filled", scene == "above"
    scale = 2 if profile == "compact" else 1
    face = visible(frame, EDITOR, 47.95)
    boundary = visible(frame, EDITOR + ("-indicator" if filled else "-outline"))
    container = visible(frame, EDITOR + "-container")
    require(boundary == face == container, "Field face, container or stroke moved")
    require(tuple(node(frame, ROUTE)["style"]["background"]) == PAGE,
            "Route does not carry the expected Canvas")
    if above:
        require(layout(frame, EDITOR + "-floating-label")["bounds"]["width"] == 0,
                "Long label must use the above-field fallback")
    label = visible(frame, EDITOR + ("-above-label" if above else "-floating-label"))
    line = {
        "x": face["x"] + 16,
        "y": face["y"] + (8 + 16 * scale if filled else (face["height"] - 24 * scale) / 2),
        "width": face["width"] - 32,
        "height": 24 * scale,
    }
    require(same_box(face, line) and label["height"] >= 16 * scale,
            "Wrong text scale or body geometry")
    if profile == "compact":
        require(abs(prior["right"](label) - (prior["right"](face) - 16)) <= .05,
                "Compact caption lost its logical start")
    return {"face": face, "label": label, "body": line}


def colors(rows, rect):
    x, y, end_x, end_y = prior["crop"](rect, len(rows[0]) // 4, len(rows))
    return Counter(prior["pixel"](rows, px, py) for py in range(y, end_y) for px in range(x, end_x))


def blended_caption(row_colors, ink, paper):
    channel = max(range(3), key=lambda i: abs(ink[i] - paper[i]))
    votes = 0
    for color, count in row_colors.items():
        coverage = (color[channel] - paper[channel]) / (ink[channel] - paper[channel])
        if (0.65 <= coverage <= 1.0 and color[3] == 255
                and all(abs(c - (b + coverage * (i - b))) <= 2
                        for c, i, b in zip(color[:3], ink[:3], paper[:3]))):
            votes += count
    return votes


def verify_pixels(path, scene, phase, measured, expected_size):
    width, height, rows = read_png(path.read_bytes())
    require((width, height) == expected_size, "PNG dimensions differ")
    face, label, body = (measured[part] for part in ("face", "label", "body"))
    x, y = round(face["x"] + face["width"] / 2), round(face["y"])
    end = round(prior["bottom"](face))
    pixel = lambda px, py: prior["pixel"](rows, px, py)
    require(pixel(x, y - 2) == PAGE and pixel(x, y + 5) == FIELD
            and pixel(x, end + 1) == PAGE, f"{path.name} field and canvas backgrounds differ")
    stroke = 1 if phase == "idle" else 3
    require(all(pixel(x, end - 1 - i) == (FIELD_INK if i < stroke else FIELD)
                for i in range(5)), f"{path.name} bottom stroke does not measure {stroke} dp")
    label_colors = colors(rows, label)
    if scene == "above":
        require(pixel(x, y) == FIELD_INK, f"{path.name} long outside label opens the contour")
        active = blended_caption(label_colors, PAGE_INK, PAGE)
        require(label_colors[FIELD] == 0 and label_colors[FIELD_INK] == 0,
                f"{path.name} above label leaks Field colors")
    else:
        active = blended_caption(label_colors, FIELD_INK, FIELD)
        if scene == "outlined":
            left, right = round(label["x"]), round(prior["right"](label))
            edge_x, guard_x = (left - 2, left - 7) if label["x"] < x else (right + 1, right + 6)
            require(pixel(edge_x, y - 2) == FIELD and pixel(guard_x, y - 2) == PAGE,
                    f"{path.name} backing does not cover just the straddling caption")
    require(active >= 10, f"{path.name} caption does not use its paired foreground")
    body_colors = colors(rows, body)
    require(body_colors[FIELD] > 30 and body_colors[FIELD_INK] >= 5 and body_colors[PAGE_INK] == 0,
            f"{path.name} editor loses Field/FieldText")
    if phase == "selected":
        require(body_colors[HIGHLIGHT] >= 12 and body_colors[HIGHLIGHT_INK] >= 5,
                f"{path.name} selected text loses its independent colors")
    else:
        require(body_colors[HIGHLIGHT] == body_colors[HIGHLIGHT_INK] == 0,
                f"{path.name} paints selection while collapsed")
    return {"field": body_colors[FIELD], "field_text": body_colors[FIELD_INK],
            "highlight": body_colors[HIGHLIGHT], "highlight_text": body_colors[HIGHLIGHT_INK],
            "caption": active, "bottom_stroke_dp": stroke}


def play(directory, profile, gallery, cli):
    width, height, scenes = SCENES[profile]
    records, windows = {}, {}

    def sample(app, label):
        require(app.proc.poll() is None and label not in windows, "Invalid window sample")
        windows[label] = onscreen_windows(app.proc.pid)
        require(windows[label] == 0, f"{label}: a Gallery window is visible")

    with TemporaryDirectory(prefix=".argv-", dir=directory) as tmp:
        guard = Path(tmp) / "headless-gallery"
        guard.write_text(guard_text(gallery, width, height))
        guard.chmod(0o700)
        plan = SimpleNamespace(out=directory, gallery=guard, cli=cli)
        with launch(plan, profile, width, height) as app:
            sample(app, "ready")
            focus(activate(app, SHOW), BACK)
            if profile == "compact":
                for setting in (THEME, RTL, LARGE):
                    activate(app, setting)
            sample(app, "opened")
            for scene, phases in scenes.items():
                if scene in ("filled", "above"):
                    activate(app, FILLED if scene == "filled" else ABOVE)
                activate(app, SET)
                focus_by_tab(app, CLEAR)
                for phase in phases:
                    if phase == "idle":
                        frame = settle(app)
                    elif phase == "focused":
                        focus(key(app, "tab"), EDITOR)
                        frame = key(app, "end")
                    else:
                        frame = key(app, "left", reverse=True)
                    label = LONG if scene == "above" else SHORT
                    focused = CLEAR if phase == "idle" else EDITOR
                    semantics = prior["editor_state"](frame, SEED, label, focused)
                    if phase == "selected":
                        require(semantics["selection_utf8"] == [6, 3], "Did not select one of two CJK graphemes")
                    measured = geometry(frame, profile, scene)
                    shot = name(profile, scene, phase)
                    sample(app, f"before/{shot}")
                    record = capture(app, plan, shot)
                    sample(app, f"after/{shot}")
                    record.update(geometry=measured, semantic=semantics, transports=TRANSPORTS)
                    record["pixels"] = verify_pixels(directory / record["name"], scene, phase, measured,
                                                      (width, height))
                    records[shot] = record
            focus(activate(app, BACK), SHOW)
            sample(app, "closed")
    require(app.proc.returncode is not None, "Owned native Gallery did not exit")
    return records, {"pid": app.proc.pid, "reaped": True, "visible_windows": windows,
                     "argv": console_args(width, height) + ["--forced-colors", "field-pair"]}


def readback(root, evidence_sha):
    report = json.loads((root / REPORT).read_text())
    files = prior["pins"](root, root / "SHA256SUMS", evidence_sha)
    expected = {REPORT}
    names = set()
    for profile, (width, height, scenes) in SCENES.items():
        for scene, phases in scenes.items():
            for phase in phases:
                shot = name(profile, scene, phase)
                names.add(shot)
                pair = [report["captures"][repeat][shot] for repeat in ("1", "2")]
                require(pair[0] == pair[1], f"{shot} does not repeat exactly")
                for repeat, record in zip(("1", "2"), pair, strict=True):
                    relative = f"replay-{repeat}/{shot}.png"
                    expected.add(relative)
                    path = root / relative
                    require(record["name"] == path.name and record["sha256"] == files[relative]
                            and record["transports"] == TRANSPORTS
                            and record["pixels"] == verify_pixels(path, scene, phase,
                                                                 record["geometry"], (width, height)),
                            f"{relative} evidence differs")
    require(set(files) == expected and set(report["captures"]) == {"1", "2"}
            and all(set(records) == names for records in report["captures"].values())
            and len(report["processes"]) == 4
            and len({item["pid"] for item in report["processes"]}) == 4
            and all(item["reaped"] and item["visible_windows"]
                    and not any(item["visible_windows"].values()) for item in report["processes"]),
            "Unexpected capture inventory or process observation")
    return {"scope": report["scope"], "source": report["source_sha256"],
            "binaries": report["binaries_sha256"], "original_pngs": len(expected) - 1,
            "evidence_manifest_sha256": evidence_sha, "transports": TRANSPORTS,
            "window_samples": sum(len(item["visible_windows"]) for item in report["processes"]),
            "visible_windows": 0, "reaped": len(report["processes"])}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path)
    parser.add_argument("--source-manifest", type=Path)
    parser.add_argument("--source-sha")
    parser.add_argument("--binary-manifest", type=Path)
    parser.add_argument("--binary-sha")
    parser.add_argument("--run", type=Path)
    parser.add_argument("--readback", type=Path)
    parser.add_argument("--evidence-sha")
    args = parser.parse_args()
    if args.readback:
        print(json.dumps(readback(args.readback.resolve(strict=True), args.evidence_sha), indent=2))
        return
    gallery, cli = check_input(args)
    if not args.run:
        print("Source and native binaries verified.")
        return
    root = args.run.resolve()
    root.mkdir(mode=0o700)
    report = {"scope": "Native synthetic fixed-palette preview", "source_sha256": args.source_sha,
              "binaries_sha256": args.binary_sha, "captures": {}, "processes": []}
    for repeat in ("1", "2"):
        directory = root / f"replay-{repeat}"
        directory.mkdir(mode=0o700)
        records = {}
        for profile in SCENES:
            receipts, process = play(directory, profile, gallery, cli)
            records.update(receipts)
            report["processes"].append(process)
        report["captures"][repeat] = records
    (root / REPORT).write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    manifest = root / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(root).as_posix()}\n"
                                for path in sorted([root / REPORT, *root.glob("replay-*/*.png")])))
    print(json.dumps(readback(root, digest(manifest)), indent=2))


if __name__ == "__main__":
    main()
