#!/usr/bin/env python3
"""Replay or verify native Material text field boundary endpoints."""

import argparse
import json
import os
from pathlib import Path
from runpy import run_path
import sys
import tomllib
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, find, focus, key, layout, node, settle
from window_check import onscreen_windows

HERE = Path(__file__).resolve().parent
PRIOR = run_path(str(HERE / "replay-placeholder.py"))
SHOW, BACK, ROUTE, THEME, RTL, LARGE, ABOVE, FILLED, EDITOR, SET, CLEAR, SEED, SHORT_LABEL, LONG_LABEL = (
    PRIOR[name] for name in ("SHOW", "BACK", "ROUTE", "THEME", "RTL", "LARGE", "ABOVE", "FILLED",
                             "EDITOR", "SET", "CLEAR", "SEED", "SHORT_LABEL", "LONG_LABEL")
)
PROFILES = {"wide": ((960, 900), ("outlined", "filled")), "compact": ((320, 900), ("outlined",))}
STAGES = ("idle", "focused", "blur")
TRANSPORTS = PRIOR["TRANSPORTS"]
REPORT = "native-text-field-boundary-report.json"
SCHEMA = "surface.test.text-field-boundary.native.v1"
SCOPE = "Native headless Reduce Motion endpoints at 1x raster scale, with Runtime semantic snapshots"
SOURCE_PATHS = {
    "Cargo.toml", "crates/surface-app/src/editor_paint.rs", "crates/surface-app/src/lib.rs",
    "crates/surface-material/src/outlined_text_field.rs", "crates/surface-platform/src/headless.rs",
    "examples/material-gallery/src/lib.rs", "examples/material-gallery/src/text_field_placeholder_demo.rs",
}
PALETTES = {
    "wide": {
        "background": (253, 247, 255, 255), "filled_face": (230, 224, 233, 255),
        "outline": (122, 117, 127, 255), "on_surface_variant": (73, 69, 78, 255),
        "primary": (101, 85, 143, 255), "on_surface": (29, 27, 32, 255),
    },
    "compact": {
        "background": (20, 18, 24, 255), "outline": (148, 143, 153, 255),
        "primary": (207, 189, 254, 255), "on_surface": (230, 224, 233, 255),
    },
}
DEPTHS = tuple(range(-2, 5))


def name(scope, appearance, phase):
    return f"{scope}-{appearance}-{phase}"


def expected_options(scope, appearance):
    return {**dict.fromkeys((THEME, RTL, LARGE, ABOVE), scope == "compact"), FILLED: appearance == "filled"}


def geometry(frame, scope, appearance):
    compact = scope == "compact"
    measured = PRIOR["geometry"](frame, appearance, compact)
    selected = {target: node(frame, target).get("selected") for target in (THEME, RTL, LARGE, ABOVE, FILLED)}
    require(selected == expected_options(scope, appearance)
            and node(frame, EDITOR)["style"]["font_size"] == (32 if compact else 16),
            "Wrong appearance, theme, RTL or editor scale")
    measured["selected_options"] = selected
    if compact:
        bounds = layout(frame, EDITOR + "-floating-label")["bounds"]
        require(bounds["width"] == bounds["height"] == 0, "Long label unexpectedly occupies an outline notch")
        measured["floating_bounds"] = bounds
    return measured


def flat_points(face, width, height, scope):
    edges = [face["x"], face["y"], PRIOR["right"](face), PRIOR["bottom"](face)]
    x, y, right, bottom = rounded = list(map(round, edges))
    require(all(abs(a - b) <= 0.05 for a, b in zip(edges, rounded)),
            f"Expected integer native pixel edges for this fixture. Got {face}")
    require(2 <= x < right <= width - 2 and 2 <= y < bottom <= height - 2,
            "Exterior guard or face lies outside the actual PNG")
    require(right - x >= 48 and bottom - y >= 48, "Expected a complete editable face")
    xs, ys = [x + (right - x) * part // 4 for part in (1, 2, 3)], [
        y + (bottom - y) * part // 4 for part in (1, 2, 3)
    ]
    # The compact fallback must keep all of its straight top closed, including
    # the logical leading (right) segment where a short RTL label would float.
    top = range(x + 8, right - 8) if scope == "compact" else xs
    return {
        "top": ([(px, y) for px in top], (0, 1)),
        "bottom": ([(px, bottom - 1) for px in xs], (0, -1)),
        "left": ([(x, py) for py in ys], (1, 0)),
        "right": ([(right - 1, py) for py in ys], (-1, 0)),
    }


def verify_boundary(rows, measured, scope, appearance, phase):
    width, height = len(rows[0]) // 4, len(rows)
    palette = PALETTES[scope]
    filled = appearance == "filled"
    backdrop = palette["background"]
    paper = palette["filled_face"] if filled else backdrop
    role = "primary" if phase == "focused" else "on_surface_variant" if filled else "outline"
    ink = palette[role]
    thickness = 3 if phase == "focused" else 1
    require(tuple(measured["paper"]) == paper and tuple(measured["background"]) == backdrop,
            "Actual mounted background palette does not match the release fixture")
    result = {}
    for edge, (anchors, inward) in flat_points(measured["face"], width, height, scope).items():
        painted = edge == "bottom" or not filled
        stroke = thickness if painted else 0
        wanted = [backdrop if d < 0 else ink if d < stroke else paper for d in DEPTHS]
        sample = {}
        representative = {0, len(anchors) // 2, len(anchors) - 1}
        for i, (x, y) in enumerate(anchors):
            actual = [PRIOR["pixel"](rows, x + d * inward[0], y + d * inward[1]) for d in DEPTHS]
            require(actual == wanted, f"{scope}/{appearance}/{phase} {edge} at ({x}, {y}) "
                                      f"expected {stroke} dp of {role}. Sample: {actual}")
            if i in representative:
                sample[f"{x},{y}"] = [list(rgba) for rgba in actual]
        result[edge] = {"stroke_dp": stroke, "probe_count": len(anchors), "inward": list(inward),
                        "depth_offsets": list(DEPTHS), "samples": sample}
    return {"color_role": role, "color_rgba": list(ink), "edges": result,
            "closed_flat_top": scope == "compact" and not filled}


def verify_value(rows, measured, scope):
    left, top, right, bottom = PRIOR["crop"](measured["line"], len(rows[0]) // 4, len(rows))
    color = PALETTES[scope]["on_surface"]
    xs = [x for y in range(top, bottom) for x in range(left, right) if PRIOR["pixel"](rows, x, y) == color]
    center = (left + right) // 2
    require(len(xs) >= 8 and (min(xs) >= center if scope == "compact" else max(xs) < center),
            "Committed value ink is missing or is on the wrong logical edge")
    record = {"ink_pixels": len(xs), "x_extent": [min(xs), max(xs) + 1]}
    if scope == "compact":
        count = PRIOR["label_ink"](rows, measured["above"], measured["background"])
        require(count >= 100, "The long, wrapped above-label has no visible ink")
        record["above_label_ink_pixels"] = count
    return record


def capture_phase(app, plan, frame, appearance, phase, window):
    scope = plan.scope
    current = EDITOR if phase == "focused" else CLEAR
    label = LONG_LABEL if scope == "compact" else SHORT_LABEL
    semantic = PRIOR["editor_state"](frame, SEED, label, current)
    measured = geometry(frame, scope, appearance)
    entry = name(scope, appearance, phase)
    window(app, f"before/{entry}")
    shot = capture(app, plan, entry)
    window(app, f"after/{entry}")
    require((shot["width"], shot["height"]) == PROFILES[scope][0] and shot["scale"] == 1,
            "Native capture geometry or raster scale differs")
    return {**shot, "layout": measured, "semantic": semantic}


def phase_pixels(directory, captures, scope, appearance):
    require(set(captures) == set(STAGES), "Missing or extra boundary phase")
    idle = captures["idle"]
    compact = scope == "compact"
    expected_label = LONG_LABEL if compact else SHORT_LABEL
    measured = idle["layout"]
    require(measured["selected_options"] == expected_options(scope, appearance)
            and measured["text_scale"] == (2 if compact else 1)
            and measured["direction"] == ("RTL" if compact else "LTR")
            and ((measured["above"] is not None
                  and measured["floating_bounds"]["width"] == measured["floating_bounds"]["height"] == 0)
                 if compact else measured["above"] is None), "Wrong native appearance, layout or fallback scope")
    require(idle["sha256"] == captures["blur"]["sha256"], "Blur did not restore the exact idle PNG")
    result = {}
    for phase in STAGES:
        shot = captures[phase]
        file = directory / shot["name"]
        semantic = shot["semantic"]
        require(shot["name"] == name(scope, appearance, phase) + ".png" and digest(file) == shot["sha256"]
                and shot["layout"] == idle["layout"] and semantic["editor"] == EDITOR
                and semantic["role"] == "TextInput" and semantic["label"] == expected_label
                and semantic["value"] == semantic["display_value"] == SEED and semantic["disabled"] is False
                and semantic["hint_in_semantic_nodes"] is False
                and semantic["focus"] == (EDITOR if phase == "focused" else CLEAR),
                f"Original capture, named field, value, focus or geometry changed for {phase}")
        width, height, rows = read_png(file.read_bytes())
        require((width, height) == PROFILES[scope][0] == (shot["width"], shot["height"])
                and shot["format"] == "png" and shot["scale"] == 1, "Wrong original native endpoint PNG")
        boundary = verify_boundary(rows, shot["layout"], scope, appearance, phase)
        result[phase] = {**boundary, "value_paint": verify_value(rows, shot["layout"], scope)}
    return result


def sample_names(scope):
    return {"ready", "opened", "closed"} | {
        f"{edge}/{name(scope, appearance, phase)}"
        for appearance in PROFILES[scope][1] for phase in STAGES for edge in ("before", "after")
    }


def play(plan):
    width, height = PROFILES[plan.scope][0]
    observations, appearances = {}, {}

    def window(app, phase):
        require(app.proc.poll() is None and phase not in observations, "Headless child exited or sample repeated")
        count = onscreen_windows(app.proc.pid)
        require(count == 0, f"{phase}: new onscreen Gallery window")
        observations[phase] = count

    with launch(plan, plan.scope, width, height) as app:
        window(app, "ready")
        focus(activate(app, SHOW), BACK)
        if plan.scope == "compact":
            for target in (THEME, RTL, LARGE, ABOVE):
                activate(app, target)
        window(app, "opened")
        for appearance in PROFILES[plan.scope][1]:
            if appearance == "filled":
                activate(app, FILLED)
            activate(app, SET)
            focus_by_tab(app, CLEAR)
            idle = settle(app)
            captures = {"idle": capture_phase(app, plan, idle, appearance, "idle", window)}
            focused = key(app, "tab")
            captures["focused"] = capture_phase(app, plan, focused, appearance, "focused", window)
            blurred = key(app, "tab", reverse=True)
            captures["blur"] = capture_phase(app, plan, blurred, appearance, "blur", window)
            appearances[appearance] = {"captures": captures,
                                       "pixels": phase_pixels(plan.out, captures, plan.scope, appearance)}
        frame = activate(app, BACK)
        focus(frame, SHOW)
        require(find(frame, ROUTE)[1] is None, "Gallery back left the field mounted")
        window(app, "closed")
    require(app.proc.returncode is not None and set(observations) == sample_names(plan.scope),
            "Owned Gallery was not reaped or a window sample is missing")
    return {"viewport": [width, height], "appearances": appearances, "restored_focus": SHOW}, {
        "pid": app.proc.pid, "reaped": True, "visible_windows": observations,
    }


def checked_source(source_root, source_manifest, source_sha):
    root = source_root.resolve(strict=True)
    files = PRIOR["pins"](root, source_manifest, source_sha)
    require(SOURCE_PATHS <= files.keys(), "Source manifest omits required boundary, Gallery or headless code")
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == "0.1.70", "Only the reviewed v0.1.70 candidate is valid")
    require("runtime.set_reduced_motion(true);" in (root / "crates/surface-platform/src/headless.rs").read_text()
            and ".animate_text_input_boundary(" in (root / "crates/surface-material/src/outlined_text_field.rs").read_text(),
            "Pinned source omits the production native Reduce Motion or boundary binding")
    require(set(files) == {p.relative_to(root).as_posix() for p in root.rglob("*")
                           if p.is_file() and p.relative_to(root).parts[0] != ".git"},
            "The exact source directory contains unsealed files")
    return {"version": version, "files": len(files), "manifest_sha256": source_sha}


def checked_inputs(args):
    require(all(value is not None for value in (args.source_root, args.source_manifest, args.binary_manifest)),
            "Provide the exact source root and both reviewed manifests")
    source = checked_source(args.source_root, args.source_manifest, args.source_manifest_sha)
    bundle = args.binary_manifest.resolve(strict=True).parent
    binaries = PRIOR["pins"](bundle, args.binary_manifest, args.binary_manifest_sha)
    gallery, devtools = (bundle / name for name in ("surface-material-gallery", "surface-devtools"))
    require(set(binaries) == {gallery.name, devtools.name}
            and all(os.access(p, os.X_OK) for p in (gallery, devtools)), "Wrong or nonexecutable native binary pair")
    return gallery, devtools, {"source": source, "binaries": binaries,
                               "binary_manifest_sha256": args.binary_manifest_sha}


def verify_stored(root, source_sha, binary_sha, evidence_sha):
    table = PRIOR["pins"](root, root / "SHA256SUMS", evidence_sha)
    report = json.loads((root / REPORT).read_text())
    require(PRIOR["SHA"].fullmatch(source_sha or "") and PRIOR["SHA"].fullmatch(binary_sha or "")
            and report["schema"] == SCHEMA and report["scope"] == SCOPE and report["transports"] == TRANSPORTS
            and report["source"]["version"] == "0.1.70" and report["source"]["manifest_sha256"] == source_sha
            and report["source"]["files"] >= len(SOURCE_PATHS) and report["binary_manifest_sha256"] == binary_sha
            and set(report["binaries"]) == {"surface-material-gallery", "surface-devtools"}
            and all(PRIOR["SHA"].fullmatch(sha) for sha in report["binaries"].values()),
            "Wrong stored exact source, binary or headless scope")
    replays, processes = report["replays"], report["processes"]
    require(len(replays) == len(processes) == 2 and replays[0] == replays[1],
            "PNG hashes and semantics differ between independent replays")
    originals, pids, samples = {REPORT}, set(), 0
    for index, (replay, launch) in enumerate(zip(replays, processes, strict=True), 1):
        require(set(replay) == set(launch) == set(PROFILES), "Wrong replay profile inventory")
        directory = root / f"replay-{index}"
        for scope, (viewport, appearances) in PROFILES.items():
            view, proc = replay[scope], launch[scope]
            observations = proc["visible_windows"]
            require(view["viewport"] == list(viewport) and view["restored_focus"] == SHOW
                    and set(view["appearances"]) == set(appearances)
                    and proc["reaped"] is True and type(proc["pid"]) is int and proc["pid"] > 0
                    and proc["pid"] not in pids and set(observations) == sample_names(scope)
                    and set(observations.values()) == {0}, "Wrong native viewport, focus, child lifetime or window samples")
            pids.add(proc["pid"])
            samples += len(observations)
            for appearance in appearances:
                scenes = view["appearances"][appearance]
                paints = phase_pixels(directory, scenes["captures"], scope, appearance)
                require(paints == scenes["pixels"], "Original native boundary pixels no longer match the receipt")
                for shot in scenes["captures"].values():
                    relative = f"replay-{index}/{shot['name']}"
                    require(relative not in originals and table[relative] == shot["sha256"], "Wrong capture seal")
                    originals.add(relative)
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file() and p != root / "SHA256SUMS"}
    require(actual == set(table) == originals and len(originals) == 19 and len(pids) == 4 and samples == 48,
            "Missing or extra native proof, process or sample")
    return {"scope": SCOPE, "source_manifest_sha256": source_sha, "binary_manifest_sha256": binary_sha,
            "replay_manifest_sha256": evidence_sha, "report_sha256": digest(root / REPORT),
            "repeats": 2, "original_pngs": 18, "observed_window_samples": samples, "visible_owned_windows": 0,
            "reaped_gallery_processes": len(pids), "transports": TRANSPORTS}


def run(args, inputs):
    gallery, devtools, proof = inputs
    require(sys.platform == "darwin" and args.out is not None, "Authorized replay requires macOS and --out")
    out = args.out.resolve()
    tmp = Path("/tmp").resolve()
    require(out != tmp and out.is_relative_to(tmp) and out.parent.is_dir() and not out.exists(),
            "Use a fresh private output under /tmp")
    old_umask = os.umask(0o077)
    try:
        out.mkdir(mode=0o700)
        replays, processes = [], []
        for index in (1, 2):
            directory = out / f"replay-{index}"
            directory.mkdir(mode=0o700)
            captures = {scope: play(SimpleNamespace(gallery=gallery, cli=devtools, out=directory, scope=scope))
                        for scope in PROFILES}
            replays.append({scope: pair[0] for scope, pair in captures.items()})
            processes.append({scope: pair[1] for scope, pair in captures.items()})
        require(replays[0] == replays[1], "The two replays differ in original bytes or editor state")
        require(checked_inputs(args) == inputs, "Source or native binaries changed during replay")
        report = {**proof, "schema": SCHEMA, "scope": SCOPE, "transports": TRANSPORTS,
                  "replays": replays, "processes": processes}
        (out / REPORT).write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
        originals = sorted(p for p in out.rglob("*") if p.is_file())
        receipt = out / "SHA256SUMS"
        receipt.write_text("".join(f"{digest(path)}  {path.relative_to(out)}\n" for path in originals))
    finally:
        os.umask(old_umask)
    return verify_stored(out, args.source_manifest_sha, args.binary_manifest_sha, digest(receipt))


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
    if args.verify_stored:
        require(args.out is None, "Offline readback never writes --out")
        result = verify_stored(args.verify_stored.resolve(strict=True), args.source_manifest_sha,
                               args.binary_manifest_sha, args.evidence_manifest_sha)
    else:
        inputs = checked_inputs(args)
        if args.verify_inputs_only:
            require(args.out is None, "Input verification never writes --out")
            result = inputs[2]
        else:
            result = run(args, inputs)
    print(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit(f"Native boundary replay stopped. {error}")
