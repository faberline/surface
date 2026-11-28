#!/usr/bin/env python3
"""Replay and read back the Gallery's fixed native forced-color preview."""

import argparse
import json
import os
from pathlib import Path
from runpy import run_path
import shlex
import sys
from tempfile import TemporaryDirectory
import tomllib
from types import SimpleNamespace

BASE = Path("/tmp/surface-md3-v0171-replay.CZtAvGj9/native-helpers")
BASE_SHA = "34291d67bd5e47502553bcbf0171b424b4bee2f9be0fa7e5f2873b7c235d7575"
sys.path.insert(0, str(BASE))
from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box
from render_helpers import activate, capture, find, focus, key, node, settle, visible
from window_check import onscreen_windows

PRIOR = run_path(str(BASE / "replay-boundaries.py"))["PRIOR"]
SPEC = "https://www.w3.org/TR/css-color-adjust-1/#forced-color-palettes"
SCHEMA = "surface.test.disabled-text-field-contrast.native.v2"
SCOPE = "Native headless Gallery fixed forced-colors preview, W3C CSS Color Adjustment Level 1 §5.2"
REPORT = "native-text-field-disabled-contrast-report.json"
PROFILES = {
    "wide-light": (960, 900, "light", False, 1),
    "wide-dark": (960, 900, "dark", False, 1),
    "compact-dark-rtl": (320, 900, "dark", True, 2),
}
COLORS = {
    "light": ((255, 255, 255, 255), (0, 0, 0, 255), (96, 0, 0, 255)),
    "dark": ((0, 0, 0, 255), (255, 255, 255, 255), (63, 242, 63, 255)),
}
FORMS = ("outlined", "filled")
PHASES = ("idle", "read-only-focused")
LOCKED = {
    "disabled": ("停用名稱", "已封存專案", True),
    "read-only": ("唯讀名稱", "既有專案", False),
}
TRANSPORTS = PRIOR["TRANSPORTS"]
SOURCE_PATHS = {
    "Cargo.toml", "crates/surface-material/src/outlined_text_field.rs",
    "crates/surface-platform/src/headless.rs", "examples/material-gallery/src/lib.rs",
    "examples/material-gallery/src/main.rs",
    "examples/material-gallery/src/outlined_text_field_demo.rs",
    "examples/material-gallery/src/filled_text_field_demo.rs",
}


def id_for(form, part):
    return f"{form}-text-field-demo-{part}"


def shot_name(profile, form, phase):
    return f"{profile}-{form}-{phase}.png"


def palette(mode):
    return dict(zip(("canvas", "canvas_text", "gray_text"), map(list, COLORS[mode]), strict=True))


def checked_base():
    files = PRIOR["pins"](BASE.resolve(), BASE / "SHA256SUMS", BASE_SHA)
    require({"replay-boundaries.py", "replay-placeholder.py", "native_runtime.py",
             "gallery_transport.py", "render_helpers.py", "window_check.py"} <= files.keys(),
            "Missing original native helpers")


def checked_inputs(args):
    require(args.source_root is not None and args.source_manifest is not None
            and args.binary_manifest is not None, "Exact source and native binary manifests are required")
    root = args.source_root.resolve(strict=True)
    files = PRIOR["pins"](root, args.source_manifest, args.source_manifest_sha)
    require(SOURCE_PATHS <= files.keys()
            and set(files) == {p.relative_to(root).as_posix() for p in root.rglob("*")
                               if p.is_file() and p.relative_to(root).parts[0] != ".git"},
            "The source seal must include every candidate path and the native preview")
    require(tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == "0.1.71",
            "Only the exact v0.1.71 source is accepted")
    native = (root / "examples/material-gallery/src/main.rs").read_text()
    require(all(token in native for token in ('"--headless"', '"--forced-colors"',
                                              "Color::rgb(96, 0, 0)", "Color::rgb(63, 242, 63)",
                                              "with_disabled_text(disabled)")),
            "The sealed Gallery is missing the reviewed fixed-palette headless route")
    binary_root = args.binary_manifest.resolve(strict=True).parent
    binaries = PRIOR["pins"](binary_root, args.binary_manifest, args.binary_manifest_sha)
    gallery, cli = (binary_root / name for name in ("surface-material-gallery", "surface-devtools"))
    require(set(binaries) == {gallery.name, cli.name}
            and all(os.access(binary, os.X_OK) for binary in (gallery, cli)),
            "Expected the two exact executable native binaries")
    return gallery, cli, {"source_version": "0.1.71", "source_files": len(files),
                          "source_manifest_sha256": args.source_manifest_sha,
                          "binary_manifest_sha256": args.binary_manifest_sha, "binaries": binaries}


def launch_args(profile):
    width, height, mode, _, _ = PROFILES[profile]
    return ["--headless", "--width", str(width), "--height", str(height), "--forced-colors", mode]


def guard_program(gallery, profile):
    expected, mode = launch_args(profile)[:5], PROFILES[profile][2]
    checks = " ] && [ ".join(f'"${i}" = "{item}"' for i, item in enumerate(expected, 1))
    return ("#!/bin/sh\n"
            f'[ "$#" -eq 5 ] && [ {checks} ] || exit 71\n'
            f'exec {shlex.quote(str(gallery))} "$@" --forced-colors {mode}\n')


def read_semantics(frame, form, current_focus):
    focus(frame, current_focus)
    sem = {}
    for part, (name, value, disabled) in LOCKED.items():
        ident = id_for(form, part)
        target = node(frame, ident)
        require(sum(entry.get("stable_id") == ident for entry in frame["dom"]["nodes"]) == 1
                and (target["role"], target["name"], target["text"], target["disabled"])
                == ("TextInput", name, value, disabled),
                f"{ident} lost its single named TextInput, authored value, or disabled state")
        buffer = frame["text_inputs"].get(ident)
        if disabled:
            require(buffer is None, f"{ident} has an interactive text buffer")
        else:
            require(buffer is not None and buffer["value"] == buffer["display_value"] == value
                    and buffer["composing"] is False,
                    f"{ident} lost its intact read-only value")
        sem[part] = {"id": ident, "role": "TextInput", "name": name, "value": value,
                     "disabled": disabled, "has_editor": buffer is not None}
    return {"locked_fields": sem, "focus": current_focus}


def body_rect(face, scale, form):
    height = 24 * scale
    return {"x": face["x"] + 16, "width": face["width"] - 32, "height": height,
            "y": face["y"] + (8 + 16 * scale if form == "filled" else (face["height"] - height) / 2)}


def check_geometry(regions, profile, form):
    width, height, _, rtl, scale = PROFILES[profile]
    viewport = {"x": 0, "y": 0, "width": width, "height": height}
    for part, region in regions.items():
        face, box, label = (region[k] for k in ("face", "body", "label"))
        require(same_box(viewport, face) and face["width"] >= 48 and face["height"] >= 48
                and same_box(face, box) and same_box(viewport, label),
                f"{form}/{part} measured box clips")
        template = body_rect(face, scale, form)
        require(all(PRIOR["near"](box[k], template[k]) for k in template)
                and PRIOR["near"](label["height"], 16 * scale)
                and label["width"] >= 20 and label["width"] < face["width"] - 32,
                f"{form}/{part} measured glyph bounds or scale changed")
        label_edge = label["x"] - face["x"] if not rtl else PRIOR["right"](face) - PRIOR["right"](label)
        require(12 <= label_edge <= 21 and PRIOR["near"](
            label["y"] - face["y"], 8 if form == "filled" else -8 * scale),
            f"{form}/{part} floating label lost logical lead or boundary alignment")


def geometry(frame, profile, form):
    _, _, mode, rtl, scale = PROFILES[profile]
    settings = {part: node(frame, id_for(form, part)).get("selected")
                for part in ("theme", "rtl", "large-text", "error")}
    require(settings == {"theme": mode == "dark", "rtl": rtl, "large-text": scale == 2, "error": False},
            "Wrong appearance, text scale, or fixture directions")
    require(node(frame, id_for(form, "route"))["style"]["background"] == palette(mode)["canvas"],
            "Gallery's mounted theme does not use the pinned Canvas color")
    regions = {}
    for part in LOCKED:
        ident = id_for(form, part)
        face = visible(frame, ident, 48)
        boundary = visible(frame, ident + ("-indicator" if form == "filled" else "-outline"))
        require(all(PRIOR["near"](boundary[k], face[k]) for k in face)
                and node(frame, ident)["style"]["font_size"] == 16 * scale,
                f"{ident} hit/editor and boundary no longer share the measured face")
        body = (visible(frame, ident + "-disabled-value") if part == "disabled"
                else body_rect(face, scale, form))
        regions[part] = {"face": face, "body": body, "label": visible(frame, ident + "-floating-label")}
    check_geometry(regions, profile, form)
    return {"settings": settings, "regions": regions}


def blend_paper(paper, ink, coverage):
    return tuple(round(base + coverage * (color - base)) for base, color in zip(paper, ink, strict=True))


def caption_coverage(pixel, ink, paper, channel):
    if pixel[3] != 255:
        return 0.
    coverage = (pixel[channel] - paper[channel]) / (ink[channel] - paper[channel])
    if not .60 <= coverage <= 1. or any(
        abs(sample - (base + coverage * (color - base))) > 1.01
        for sample, base, color in zip(pixel[:3], paper[:3], ink[:3], strict=True)
    ):
        return 0.
    return coverage


def pixel_mask(rows, rect, target, forbidden, rtl, name, *, paper=None):
    x0, y0, x1, y1 = PRIOR["crop"](rect, len(rows[0]) // 4, len(rows))
    points, exact, wrong, strong = [], 0, 0, 0
    channel = max(range(3), key=lambda i: abs(target[i] - paper[i])) if paper is not None else None
    for y in range(y0, y1):
        for x in range(x0, x1):
            pixel = PRIOR["pixel"](rows, x, y)
            if pixel == target:
                exact += 1
            elif pixel == forbidden:
                wrong += 1
            if paper is not None:
                coverage = caption_coverage(pixel, target, paper, channel)
                if coverage > 0.:
                    points.append((x, y))
                    strong += coverage >= .85
            elif pixel == target:
                points.append((x, y))
    if paper is None:
        require(exact >= 4, f"{name} has too few exact {target} pixels. Got {exact}")
    else:
        require(len(points) >= 8 and strong >= 4,
                f"{name} has too few pixels blended from {paper} toward {target}: {len(points)}, {strong} strong")
    xs, ys = zip(*points)
    extent = [min(xs), min(ys), max(xs) + 1, max(ys) + 1]
    require(extent[2] - extent[0] >= 8 and extent[3] - extent[1] >= 4,
            f"{name} expected color occupies too small a glyph region")
    leading_margin = x1 - extent[2] if rtl else extent[0] - x0
    require(rect["width"] < 160 or 0 <= leading_margin <= 16,
            f"{name} painted on the wrong logical side")
    # The Outlined label crop can include the end of a boundary at the text
    # inset. In the body crop only glyphs could use the other palette ink.
    require(wrong == 0 or name == "disabled-label", f"{name} has {wrong} pixels of the other text role")
    result = {"exact_pixels": exact, "extent": extent, "leading_margin": leading_margin,
              "other_role_pixels": wrong, "rgba": list(target)}
    if paper is not None:
        result.update(paper_rgba=list(paper), caption_fit_pixels=len(points), caption_strong_pixels=strong)
    return result


def verify_paint(rows, regions, profile, form, phase):
    _, _, mode, rtl, _ = PROFILES[profile]
    canvas, active, gray = COLORS[mode]
    check_geometry(regions, profile, form)
    colors = {"disabled": (gray, active), "read-only": (active, gray)}
    result = {}
    for part, boxes in regions.items():
        ink, other = colors[part]
        label_paper = canvas
        if part == "disabled" and form == "filled":
            face = boxes["face"]
            label = boxes["label"]
            x = int(face["x"] + face["width"] * (.25 if rtl else .75))
            y = int(label["y"] + label["height"] / 2)
            label_paper = PRIOR["pixel"](rows, x, y)
            prescribed = blend_paper(canvas, active, 10 / 255)
            require(label_paper[3] == 255 and all(abs(a - b) <= 1 for a, b in zip(label_paper, prescribed, strict=True)),
                    f"Filled disabled caption backdrop {label_paper} differs from its neutral face {prescribed}")
        result[part] = {
            "label": pixel_mask(rows, boxes["label"], ink, other, rtl, f"{part}-label", paper=label_paper),
            "body": pixel_mask(rows, boxes["body"], ink, other, rtl, f"{part}-body"),
        }
    face = regions["read-only"]["face"]
    x = int(face["x"] + face["width"] * (.25 if rtl else .75))
    stop = round(PRIOR["bottom"](face))
    require(abs(PRIOR["bottom"](face) - stop) <= .05, "Read-only edge is outside the integral fixture grid")
    actual = [PRIOR["pixel"](rows, x, stop - 1 - depth) for depth in (-2, 0, 1, 2, 3)]
    stripe = 3 if phase == "read-only-focused" else 1
    expected = [canvas, active, active, active, canvas] if stripe == 3 else [canvas, active, canvas, canvas, canvas]
    require(actual == expected, f"Read-only field lost {stripe} dp CanvasText boundary. Got {actual}")
    result["read_only_boundary"] = {"x": x, "bottom": stop, "depths": [-2, 0, 1, 2, 3],
                                    "stroke_dp": stripe, "samples": [list(p) for p in actual]}
    return result


def expected_semantics(form, phase):
    part = "editable" if phase == "idle" else "read-only"
    return {
        "focus": id_for(form, part),
        "locked_fields": {
            key: {"id": id_for(form, key), "role": "TextInput", "name": name, "value": value,
                  "disabled": disabled, "has_editor": not disabled}
            for key, (name, value, disabled) in LOCKED.items()
        },
    }


def settings_for(profile):
    _, _, mode, rtl, scale = PROFILES[profile]
    return {"theme": mode == "dark", "rtl": rtl, "large-text": scale == 2, "error": False}


def sample_names(profile):
    shots = {f"{edge}/{shot_name(profile, form, phase)}"
             for edge in ("before", "after") for form in FORMS for phase in PHASES}
    return {"ready"} | {f"{state}/{form}" for state in ("opened", "closed") for form in FORMS} | shots


def check_shot(directory, record, profile, form, phase):
    width, height = PROFILES[profile][:2]
    path = directory / shot_name(profile, form, phase)
    require(record["name"] == path.name and digest(path) == record["sha256"]
            and (record["width"], record["height"], record["scale"], record["format"]) == (width, height, 1, "png")
            and record["geometry"]["settings"] == settings_for(profile)
            and set(record["geometry"]["regions"]) == set(LOCKED)
            and record["semantic"] == expected_semantics(form, phase)
            and record["verified_transports"] == TRANSPORTS,
            f"{path.name} original, fixture, semantic or transport receipt differs")
    decoded_width, decoded_height, rows = read_png(path.read_bytes())
    require((decoded_width, decoded_height) == (width, height), f"{path.name} has the wrong actual size")
    return verify_paint(rows, record["geometry"]["regions"], profile, form, phase)


def play(plan, profile, gallery):
    width, height, mode, rtl, scale = PROFILES[profile]
    observations, forms = {}, {}

    def sample(app, key):
        require(app.proc.poll() is None and key not in observations, "Missing owned live Gallery or duplicate sample")
        count = onscreen_windows(app.proc.pid)
        require(count == 0, f"{key}: Gallery owns visible windows")
        observations[key] = count

    with TemporaryDirectory(prefix=".argv-", dir=plan.out) as temp:
        guard = Path(temp) / "headless-gallery"
        guard.write_text(guard_program(gallery, profile))
        guard.chmod(0o700)
        plan.gallery = guard
        with launch(plan, profile, width, height) as app:
            sample(app, "ready")
            for index, form in enumerate(FORMS):
                show, route, back = (id_for(form, part) for part in ("show", "route", "back"))
                focus(activate(app, show), back)
                # Gallery dark is global. Set it only on the first route.
                if mode == "dark" and index == 0:
                    activate(app, id_for(form, "theme"))
                if rtl:
                    activate(app, id_for(form, "rtl"))
                if scale == 2:
                    activate(app, id_for(form, "large-text"))
                sample(app, f"opened/{form}")
                focus_by_tab(app, id_for(form, "editable"))
                idle = settle(app)
                focus(readonly := key(app, "tab"), id_for(form, "read-only"))
                writes = {
                    part: app.wire("Action.fill", {"id": id_for(form, part), "value": "change"}, error)
                    for part, error in (("read-only", "action_failed"), ("disabled", "disabled"))
                }
                after_writes = settle(app)
                require(read_semantics(after_writes, form, id_for(form, "read-only"))
                        == read_semantics(readonly, form, id_for(form, "read-only")),
                        "Rejected fills changed focus or locked value")
                # Capture the Tab destination without scrolling to the long field first.
                left = key(app, "tab", reverse=True)
                focus(left, id_for(form, "editable"))
                shots = {}
                for phase, part in zip(PHASES, ("editable", "read-only"), strict=True):
                    frame = settle(app) if phase == "idle" else key(app, "tab")
                    current_focus = id_for(form, part)
                    semantic = read_semantics(frame, form, current_focus)
                    measured = geometry(frame, profile, form)
                    require(measured == geometry(idle, profile, form),
                            "Keyboard focus changed the locked field geometry")
                    filename = shot_name(profile, form, phase)
                    sample(app, f"before/{filename}")
                    record = capture(app, plan, filename.removesuffix(".png"))
                    sample(app, f"after/{filename}")
                    record.update(geometry=measured, semantic=semantic,
                                  verified_transports=TRANSPORTS)
                    record["pixels"] = check_shot(plan.out, record, profile, form, phase)
                    shots[phase] = record
                next_focus = focused(key(app, "tab"))
                require(next_focus == id_for(form, "long-label"), "Tab did not skip the disabled field")
                closed = activate(app, back)
                focus(closed, show)
                require(find(closed, route)[1] is None, "Back left a stale field route")
                sample(app, f"closed/{form}")
                forms[form] = {"captures": shots, "rejected_fills": writes,
                               "tab_after_read_only": next_focus, "restored_focus": show}
    require(app.proc.returncode is not None and set(observations) == sample_names(profile),
            "A Gallery process was not reaped or missed its window observations")
    return {"viewport": [width, height], "preview": mode, "palette": palette(mode), "forms": forms}, {
        "pid": app.proc.pid, "reaped": True, "visible_windows": observations,
        "argv_without_binary": launch_args(profile),
    }


def verify_stored(root, source_sha, binary_sha, evidence_sha):
    table = PRIOR["pins"](root, root / "SHA256SUMS", evidence_sha)
    report = json.loads((root / REPORT).read_text())
    require(report["schema"] == SCHEMA and report["scope"] == SCOPE and report["w3c_palette"] == SPEC
            and report["helpers_manifest_sha256"] == BASE_SHA and report["transports"] == TRANSPORTS
            and report["observer_sha256"] == digest(BASE / "window_check.py")
            and report["source_version"] == "0.1.71" and report["source_files"] >= len(SOURCE_PATHS)
            and PRIOR["SHA"].fullmatch(source_sha or "") and report["source_manifest_sha256"] == source_sha
            and PRIOR["SHA"].fullmatch(binary_sha or "") and report["binary_manifest_sha256"] == binary_sha
            and set(report["binaries"]) == {"surface-material-gallery", "surface-devtools"}
            and all(PRIOR["SHA"].fullmatch(value) for value in report["binaries"].values()),
            "Wrong exact source, binaries, helpers or W3C fixed preview scope")
    replays, processes = report["replays"], report["processes"]
    require(len(replays) == len(processes) == 2 and replays[0] == replays[1],
            "The two native replays differ in original PNG or Gallery state")
    expected_paths, pids, samples = {REPORT}, set(), 0
    for index, (replay, observed) in enumerate(zip(replays, processes, strict=True), 1):
        directory = root / f"replay-{index}"
        require(set(replay) == set(observed) == set(PROFILES), "Missing or extra native profiles")
        for profile, (width, height, mode, _, _) in PROFILES.items():
            view, process = replay[profile], observed[profile]
            windows = process["visible_windows"]
            require(view["viewport"] == [width, height] and view["preview"] == mode and view["palette"] == palette(mode)
                    and set(view["forms"]) == set(FORMS) and process["reaped"] is True
                    and type(process["pid"]) is int and process["pid"] > 0 and process["pid"] not in pids
                    and set(windows) == sample_names(profile) and set(windows.values()) == {0}
                    and process["argv_without_binary"] == launch_args(profile),
                    "Incorrect viewport, fixture, native lifetime or visible window observations")
            pids.add(process["pid"])
            samples += len(windows)
            for form, entry in view["forms"].items():
                shots = entry["captures"]
                require(set(shots) == set(PHASES)
                        and shots["idle"]["geometry"] == shots["read-only-focused"]["geometry"]
                        and entry["rejected_fills"] == {"read-only": "action_failed", "disabled": "disabled"}
                        and entry["tab_after_read_only"] == id_for(form, "long-label")
                        and entry["restored_focus"] == id_for(form, "show"),
                        f"Wrong {form} focus or author-owned value history")
                for phase, shot in shots.items():
                    relative = f"replay-{index}/{shot_name(profile, form, phase)}"
                    require(relative not in expected_paths and table.get(relative) == shot["sha256"]
                            and check_shot(directory, shot, profile, form, phase) == shot["pixels"],
                            f"{relative} failed exact independent PNG readback")
                    expected_paths.add(relative)
    actual_paths = {path.relative_to(root).as_posix() for path in root.rglob("*")
                    if path.is_file() and path != root / "SHA256SUMS"}
    require(set(table) == actual_paths == expected_paths and len(expected_paths) == 25
            and len(pids) == 6 and samples == 78, "Native artifact or window inventory differs")
    return {"scope": SCOPE, "w3c_palette": SPEC, "source_manifest_sha256": source_sha,
            "binary_manifest_sha256": binary_sha, "native_helpers_manifest_sha256": BASE_SHA,
            "observer_sha256": report["observer_sha256"],
            "replay_manifest_sha256": evidence_sha, "report_sha256": digest(root / REPORT),
            "repeats": 2, "original_pngs": 24, "observed_window_samples": samples,
            "visible_owned_windows": 0, "reaped_gallery_processes": len(pids), "transports": TRANSPORTS}


def run(args, inputs):
    gallery, cli, proof = inputs
    require(sys.platform == "darwin" and args.out is not None, "Replay requires macOS and a fresh --out")
    out, temporary = args.out.resolve(), Path("/tmp").resolve()
    require(out != temporary and out.is_relative_to(temporary) and out.parent.is_dir() and not out.exists(),
            "Replay output must be a fresh child of /tmp")
    old_umask = os.umask(0o077)
    try:
        out.mkdir(mode=0o700)
        replays, processes = [], []
        for index in (1, 2):
            directory = out / f"replay-{index}"
            directory.mkdir(mode=0o700)
            result = {profile: play(SimpleNamespace(cli=cli, out=directory), profile, gallery)
                      for profile in PROFILES}
            replays.append({profile: pair[0] for profile, pair in result.items()})
            processes.append({profile: pair[1] for profile, pair in result.items()})
        require(checked_inputs(args) == inputs, "Exact source or native binary bytes changed during replay")
        report = {**proof, "schema": SCHEMA, "scope": SCOPE, "w3c_palette": SPEC,
                  "helpers_manifest_sha256": BASE_SHA, "transports": TRANSPORTS,
                  "observer_sha256": digest(BASE / "window_check.py"),
                  "replays": replays, "processes": processes}
        (out / REPORT).write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
        originals = sorted(path for path in out.rglob("*") if path.is_file())
        manifest = out / "SHA256SUMS"
        manifest.write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in originals))
    finally:
        os.umask(old_umask)
    return verify_stored(out, args.source_manifest_sha, args.binary_manifest_sha, digest(manifest))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--verify-inputs-only", action="store_true")
    mode.add_argument("--verify-stored", type=Path, metavar="REPLAY")
    mode.add_argument("--run", action="store_true")
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--source-manifest", type=Path)
    parser.add_argument("--source-manifest-sha", required=True)
    parser.add_argument("--binary-manifest", type=Path)
    parser.add_argument("--binary-manifest-sha", required=True)
    parser.add_argument("--evidence-manifest-sha")
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    checked_base()
    if args.verify_stored is not None:
        require(args.out is None, "Offline readback does not write output")
        result = verify_stored(args.verify_stored.resolve(strict=True), args.source_manifest_sha,
                               args.binary_manifest_sha, args.evidence_manifest_sha)
    else:
        inputs = checked_inputs(args)
        if args.verify_inputs_only:
            require(args.out is None, "Input verification does not write replay output")
            result = inputs[2]
        else:
            result = run(args, inputs)
    print(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit(f"Native forced-color replay stopped. {error}")
