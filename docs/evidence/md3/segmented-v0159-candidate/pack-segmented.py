#!/usr/bin/env python3
"""Package exact Baseline Segmented Button evidence without starting a host."""

import argparse
import gzip
from hashlib import file_digest, sha1, sha256
import json
from math import ceil, floor
import os
from pathlib import Path, PurePosixPath
import re
from runpy import run_path
import shutil
import subprocess
import sys
import tomllib

from PIL import Image

STAGE = Path("/tmp/surface-md3-segmented-v0159.yzqEnSpq")
BASE = Path("/tmp/surface-md3-buttons-v0158.l0TJqeGW")
NATIVE_ROOT = Path("/tmp/surface-md3-segmented-native.O3aJrrno")
VERSION = "0.1.59"
SOURCE = "9051c16e996c9b310062682765a7d0bc54cf42dfbadbda4c5f63bd457b5d53b8"
TREE = "757179e260a331b4488c6be1b49410263097d675"
BASE_SOURCE = "b03623198811629a68a2dfe5620d5705c55ea7744157ff2ac732d174833eedbf"
BASE_TREE = "8ba487496a2816f3bff3363328ba0dc40a059c30"
PATCH = "c76e282e1b634e597705e5e9306c7dedd2e43dbb7bc6e4aaebb1f672bb8d7766"
PATCH_NAME = "segmented-v0159.patch"
NATIVE_HARNESS = "1b356ac97208abdca4775f8538e7b5e986c13219b8581aad36e58893e552cf49"
STOCK_HARNESS = "fa7ca061fc7624f4390fcbdc31c77dd6f3abaecb090ec73b1b68d85f11e306d5"
PIXELS = "5ae7f8c42274f34283cf484be877f6de1ed4eeb6e8c43abe0d87e99311e9f796"
STOCK_RUNNER = "d54232ba518fba53d4ab540de6230afc7911b97b0364c8ce574610e2697d5a0b"
FORMS_PIXELS = "13d2a3b233e36fbbf44410c3b2b4f213a6832e4495670ed54976fce2d6e62b95"
REFINEMENTS_REPORTS = ("material-vertical-centered-slider.json", "material-slider-inset.json")
NATIVE_REPORT = "native-segmented-report.json"
STOCK_REPORT = "verified-segmented-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
OBSERVATION = "headless-window-observation.json"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
RUST_LOGS = ("workspace-v0159-exact.log", "gpu-v0159-exact.log")
SUPPLEMENTAL = "gpu-v0159-supplemental.log"
INTEGRITY_LOGS = tuple(
    f"{group}-source-{point}.log"
    for group in ("workspace", "gpu")
    for point in ("before", "after")
)
NATIVE_IMAGES = {
    "wide-light-rest.png": (960, 880),
    "wide-light-selected-keyboard.png": (960, 880),
    "compact-dark-rtl-200.png": (320, 880),
    "scrolled-dark-rtl-200.png": (240, 880),
}
NATIVE_SEAL = "a2ba60d961887df4f113c7b8431eaf32a72c92ee35d87cd40da240e963646d6b"
BINARY_SEAL = "6a85c0ef5cca613e2b115e4ab0fb3e7f9d8d09fd6bc24463d5257f47bd0de5dd"
STOCK_PINS = (
    "browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256",
    "release-pin.json", "source-origins.json", "build-markers.json", "sealed-inputs.sha256",
    "inputs-before.json", "approved-runner.sha256",
)
RETAINED = (
    "browser-ime.json", "browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json",
    "browser-smoke.json", "browser-material-side-sheet.json", "bridge-web-result.json",
    "devtools-headless-result.json",
    "material-standard-slider.json", "material-slider-forms.json", "material-range-slider.json",
)
HEX = re.compile(r"[a-f0-9]{64}")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def read_json(path):
    return json.loads(path.read_text())


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True) + "\n")


def manifest(path):
    entries = {}
    for line in path.read_text().splitlines():
        sha, name = line.split("  ", 1)
        relative = PurePosixPath(name)
        require(HEX.fullmatch(sha) and not relative.is_absolute() and ".." not in relative.parts
                and relative.as_posix() == name and name not in entries and name != ".",
                f"Invalid entry in {path.name}: {name}")
        entries[name] = sha
    require(entries, f"Empty manifest: {path}")
    return entries


def verify_entries(root, entries):
    for name, sha in entries.items():
        path = root / name
        require(path.is_file() and not path.is_symlink()
                and path.resolve(strict=True).is_relative_to(root.resolve(strict=True))
                and digest(path) == sha, f"Different or missing manifest member: {path}")


def verify_seal(root, filename, expected, *, strict=False):
    path = root / filename
    require(HEX.fullmatch(expected or "") and digest(path) == expected, f"Wrong {path} seal")
    entries = manifest(path)
    verify_entries(root, entries)
    if strict:
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file() and p != path}
        require(actual == set(entries), f"Unsealed files in {root}")
    return entries


def object_id(kind, data):
    return sha1(f"{kind} {len(data)}\0".encode("ascii") + data).digest()


def source_tree(root, entries):
    """Calculate the exact Git tree without writing repository objects."""
    folders = {}
    for name, checksum in entries.items():
        *parents, filename = PurePosixPath(name).parts
        folder = folders
        for parent in parents:
            folder = folder.setdefault(parent, {})
        path = root / name
        data = path.read_bytes()
        require(sha256(data).hexdigest() == checksum, f"Source changed during tree verification: {name}")
        mode = b"100755" if path.stat().st_mode & 0o100 else b"100644"
        folder[filename] = mode, object_id("blob", data)

    def git_tree(folder):
        def git_name(item):
            name, child = item
            return (name + ("/" if isinstance(child, dict) else "")).encode()

        data = []
        for name, child in sorted(folder.items(), key=git_name):
            mode, oid = (b"40000", git_tree(child)) if isinstance(child, dict) else child
            data.append(mode + b" " + name.encode() + b"\0" + oid)
        return object_id("tree", b"".join(data))

    return git_tree(folders).hex()


def source_info(stage, base):
    prior, candidate = base / "source.sha256", stage / "source.sha256"
    require(digest(prior) == BASE_SOURCE and digest(candidate) == SOURCE, "Wrong base or candidate source seal")
    before, after = manifest(prior), manifest(candidate)
    for root, entries, count, tree in ((base / "source", before, 656, BASE_TREE),
                                       (stage / "source", after, 660, TREE)):
        require(len(entries) == count, "Wrong source inventory count")
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
                  if p.is_file() and p.relative_to(root).parts[0] != ".git"}
        require(actual == set(entries), f"Source file inventory differs: {root}")
        verify_entries(root, entries)
        require(source_tree(root, entries) == tree, f"Wrong exact source tree: {root}")
    root = stage / "source"
    patch = stage / PATCH_NAME
    require(digest(patch) == PATCH and (stage / "segmented-tree").read_text().strip() == TREE
            and tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == VERSION,
            "Wrong candidate patch, tree marker or workspace version")
    material = (root / "e2e/src/material.rs").read_text()
    registry = re.search(r"pub const STEPS: &\[&str\] = &\[(.*?)\];", material, re.S)
    required_steps = [str(step) for step in range(1, 324) if step != 23]
    require(registry and re.findall(r'"M(\d+)"', registry.group(1)) == required_steps,
            "The stock runner must require every Material step through M323 except failure-only M23")
    paths = {p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch.read_bytes(), re.M)}
    changed = {name for name in before.keys() | after.keys() if before.get(name) != after.get(name)}
    require(paths == changed == set((stage / "owned-segmented.paths").read_text().splitlines()) and len(paths) == 11,
            "Patch, source inventory and the 11 reviewed paths differ")
    for directory, options in ((base / "source", []), (root, ["--reverse"])):
        subprocess.run(["git", "-C", str(directory), "apply", "--check", "--whitespace=error-all",
                        *options, str(patch)], check=True, env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"})
    return {"version": VERSION, "source_sha256": SOURCE, "source_files": len(after), "tree": TREE,
            "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
            "changed_paths": sorted(paths)}


def materialize(log):
    return gzip.decompress(log.read_bytes()) if log.suffix == ".gz" else log.read_bytes()


def tests(log):
    raw = materialize(log)
    text = raw.decode()
    rows = re.findall(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", text, re.M)
    require(rows and all(result == "ok" and failed == "0" for result, _, failed, _ in rows),
            f"Missing or failed tests in {log.name}")
    return text, {"passed": sum(int(passed) for _, passed, _, _ in rows),
                  "ignored": sum(int(ignored) for _, _, _, ignored in rows),
                  "log_sha256": sha256(raw).hexdigest()}


def check_build_source(log_root, source_manifest):
    expected = {f"{name}: OK" for name in manifest(source_manifest)}
    source_logs = {}
    for name in INTEGRITY_LOGS:
        path = log_root / name
        if not path.is_file():
            path = log_root / f"{name}.gz"
        if not path.is_file():
            continue
        raw = materialize(path)
        lines = raw.decode().splitlines()
        require(len(lines) == len(expected) and set(lines) == expected,
                f"Source preflight or postflight check differs: {name}")
        source_logs[name] = sha256(raw).hexdigest()
    return source_logs


def rust_info(workspace, gpu, supplemental):
    text, rust = tests(workspace)
    tail = text.rsplit("\ntest result:", 1)[-1]
    require(rust["passed"] == 1502 and rust["ignored"] == 19
            and tail.count("Finished `dev` profile") == 3
            and re.search(r"^provenance: .+ self-test cases passed\)", text, re.M)
            and "Doc-tests surface_material_gallery" in text
            and "range_slider_steps_are_required_by_the_material_case ... ok" in text
            and "segmented_steps_are_required_by_the_material_case ... ok" in text
            and "compact_rtl_grows_one_row_and_paints_the_whole_focus_outline ... ok" in text
            and "narrow_rtl_scroll_keeps_whole_targets_and_reveals_keyboard_and_ax_choices ... ok" in text,
            "The workspace lacks complete Baseline Segmented Button, provenance or Wasm checks")
    metal_text, metal = tests(gpu)
    require(metal["passed"] == 16 and metal["ignored"] == 0
            and "name: \"Apple M1 Max\"" in metal_text and "backend: Metal" in metal_text,
            "Missing the 16 actual offscreen Metal tests")
    extra_text, extra = tests(supplemental)
    require(extra["passed"] == 2 and extra["ignored"] == 0
            and 'name: "Apple M1 Max"' in extra_text and "backend: Metal" in extra_text
            and "gpu_offscreen_raster_cache_reuses_shared_pixels_and_evicts_by_count_and_bytes" in extra_text
            and "paired_selection_matches_single_draw_references_at_fractional_clip_edges" in extra_text,
            "The marked supplemental Metal run is incomplete")
    return {"workspace": {**rust, "wasm_checks": 3}, "metal": {**metal, "backend": "Metal"},
            "supplemental": {**extra, "backend": "Metal"}}


def open_png(path, expected):
    with Image.open(path) as picture:
        require(picture.format == "PNG" and picture.size == expected, f"Wrong PNG dimensions: {path}")
        picture.load()
        return picture.convert("RGBA")


def scan_box(picture, rect):
    left = max(0, ceil(rect["x"]))
    top = max(0, ceil(rect["y"]))
    right = min(picture.width, floor(rect["x"] + rect["width"]))
    bottom = min(picture.height, floor(rect["y"] + rect["height"]))
    require(left < right and top < bottom, "Empty native paint sample")
    return ((x, y) for y in range(top, bottom) for x in range(left, right))


def native_ink(picture, item):
    ink, tolerance = item["rgba"], item["rgb_tolerance"]
    points = []
    for xy in scan_box(picture, item["bounds"]):
        rgba = picture.getpixel(xy)
        if rgba[3] == ink[3] and max(abs(a - b) for a, b in zip(rgba[:3], ink[:3])) <= tolerance:
            points.append(xy)
    require(len(points) == item["ink_pixels"] and len(points) >= 16 and tolerance == 64,
            "Native caption or label ink count differs")
    xs, ys = zip(*points)
    require([min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1] == item["ink_bounds"],
            "Native caption or label ink bounds differ")


def paint_count(picture, rect, color):
    return sum(list(picture.getpixel(xy)) == color for xy in scan_box(picture, rect))


def check_dimensions(parts):
    hit, face, ring = (parts[key] for key in ("hit", "face", "focus"))
    require(hit["width"] >= 47.99 and hit["height"] >= 47.99
            and abs(hit["width"] - face["width"]) <= .06 and abs(hit["height"] - face["height"] - 8) <= .06,
            "Native slot lost its touch target or paint slack")
    for axis, side in (("x", "width"), ("y", "height")):
        require(abs(hit[axis] + hit[side] / 2 - face[axis] - face[side] / 2) <= .06
                and abs(ring[axis] - face[axis] + 5) <= .06 and abs(ring[side] - face[side] - 10) <= .06,
                "Native face or focus outline no longer has the authored bounds")


def native_shot(directory, shot, hashes, profile):
    name = shot["name"]
    size = NATIVE_IMAGES[name]
    require(shot["sha256"] == hashes[name] and (shot["width"], shot["height"]) == size
            and shot["format"] == "png" and shot["scale"] == 1, f"Wrong native capture receipt: {name}")
    picture = open_png(directory / name, size)
    background = shot["background"]
    require(list(picture.getpixel((4, 4))) == background, f"Native background mismatch: {name}")
    require(bool(shot.get("focus")) == (name != "wide-light-rest.png") and shot["label_ink"],
            "Native focus or glyph capture proof is incomplete")
    for ink in shot["label_ink"].values():
        native_ink(picture, ink)
    if focus := shot.get("focus"):
        check_dimensions(focus["parts"])
        require(set(focus["edges"]) == {"top", "bottom", "left", "right"}, "Native focus needs all four visible edges")
        require(all(paint_count(picture, part["bounds"], focus["rgba"]) == part["pixels"] >= 8
                    for part in focus["edges"].values()), "One exterior focus edge has different original pixels")
    for target, evidence in shot["selected_ink"].items():
        parts = next(row[target] for row in profile["geometry"].values() if target in row)
        require(paint_count(picture, parts["face"], evidence["rgba"]) == evidence["pixels"] >= 30,
                f"Actual selected fill is absent: {target}")
    return name


def selection(proof, comparison, view, filters):
    groups = proof["groups"]
    wanted = {"segments-comparison": [comparison], "segments-view": [view], "segments-filters": filters}
    require(set(groups) == set(wanted), "Native named group inventory differs")
    require(all([choice["id"] for choice in groups[group] if choice["checked"]] == asserted
                for group, asserted in wanted.items()), "Native manual selection state differs")
    for group, choices in groups.items():
        role = "Checkbox" if group == "segments-filters" else "Radio"
        require(all(choice["role"] == role and choice["name"]
                    and choice["disabled"] is (choice["id"] in ("segment-table-view", "segment-local"))
                    for choice in choices), "Native named choices or disabled semantics differ")
    require(proof["status"] == (f"已選取 {len(filters)} 項篩選條件" if filters else "未選取篩選條件"),
            "Native group status differs")


def native_profile(name, profile):
    width = {"wide": 960, "compact": 320, "scrolled": 240}[name]
    rtl = name != "wide"
    if name == "scrolled":
        # The original report stores the scroll port rectangle in this field.
        # The physical 240×880 viewport is checked on its PNG.
        require(profile["viewport"]["width"] == width - 32, "Wrong native narrow scroll port")
    else:
        require(profile["viewport"] == [width, 880], "Wrong native viewport")
    require(profile["rtl"] is rtl
            and profile["text_scale"] == (2 if rtl else 1)
            and profile["opening_tabs"][-1] == profile["closed_focus"] == "segmented-demo-show"
            and profile["disabled_rejections"] == ["segment-table-view", "segment-local"],
            f"Wrong native route profile: {name}")
    right, grid, view = "segment-right-pane", "segment-grid-view", "segment-list-view"
    if name == "wide":
        selection(profile["initial"], right, grid, [])
        steps = profile["focus_steps"]
        require([(part["key"], part["id"]) for part in steps] == [
            ("tab", "segmented-demo-theme"), ("tab", "segmented-demo-rtl"),
            ("tab", "segmented-demo-large-text"), ("tab", "segment-left-pane"),
            ("right", right), ("enter", right), ("tab", view), ("right", grid), ("left", view),
            ("space", view), ("enter", view), ("tab", "segment-new"), ("space", "segment-new"),
            ("right", "segment-saved"), ("right", "segment-shared"), ("enter", "segment-shared"),
            ("space", "segment-shared"), ("left", "segment-saved"), ("left", "segment-new"),
            ("enter", "segment-new"), ("tab", "segmented-demo-after")], "Wrong native keyboard sequence")
        selection(steps[8], right, grid, [])
        selection(steps[9], right, view, [])
        selection(steps[15], right, view, ["segment-new", "segment-shared"])
        selection(steps[-1], right, view, [])
    if name == "compact":
        require(profile["focus_steps"] == ["segment-left-pane", view, "segment-new"],
                "Native compact entry changed logical order")
        selection(profile["selection"], right, grid, [])
    if name == "scrolled":
        selection(profile["selected"], right, grid, ["segment-archived"])
        require([part["id"] for part in profile["travel"]] == ["segment-saved", "segment-shared", "segment-archived"],
                "Wrong native mirrored arrow reveal")
        start, end = profile["track_start"], profile["track_end"]
        require(start["x"] < end["x"] and start["width"] > profile["viewport"]["width"]
                and abs(start["width"] - end["width"]) <= .06,
                "Native narrow track shrank or did not scroll")
        check_dimensions(profile["first"])
        for part in profile["travel"]:
            check_dimensions(part["parts"])
    else:
        rows = profile["geometry"]
        require({name: len(parts) for name, parts in rows.items()} ==
                {"segments-comparison": 2, "segments-view": 3, "segments-filters": 5}, "Native row inventory differs")
        for row in rows.values():
            faces = [parts["face"] for parts in row.values()]
            require(all(abs(face["width"] - faces[0]["width"]) <= .06
                        and abs(face["height"] - faces[0]["height"]) <= .06 for face in faces),
                    "Native segments do not share a face size")
            require(faces[0]["height"] > 40 if rtl else abs(faces[0]["height"] - 40) <= .06,
                    "Wrong native painted size for scale")
            for part in row.values():
                check_dimensions(part)


def native_info(root, expected, binaries_manifest):
    require(expected == NATIVE_SEAL and digest(binaries_manifest) == BINARY_SEAL,
            "Wrong exact native proof or executable manifest seal")
    entries = verify_seal(root, "SHA256SUMS", expected, strict=True)
    report = read_json(root / NATIVE_REPORT)
    binaries = manifest(binaries_manifest)
    require(set(binaries) == {"surface-material-gallery", "surface-devtools"}
            and report["binaries"] == binaries and report["native_binary_manifest_sha256"] == BINARY_SEAL,
            "Native proof used different Gallery or DevTools executables")
    require(report["schema"] == "surface.test.segmented.native.v1"
            and report["source"] == {"manifest_sha256": SOURCE, "files_verified": 660, "version": VERSION}
            and report["rendering"] == "headless" and report["reduce_motion"] is True
            and report["input_scope"] == ["Action.key", "Action.activate", "DOM.snapshot"]
            and report["transports"] == ["Page.screenshot", "CLI", "MCP"]
            and len(report["replays"]) == 2 and report["replays"][0] == report["replays"][1],
            "Native proof scope or repeated semantic receipts differ")
    pngs = report["byte_identical_pngs"]
    expected_files = {f"replay-{i}/{name}" for i in (1, 2) for name in NATIVE_IMAGES}
    require(set(pngs) == set(NATIVE_IMAGES) and len(set(pngs.values())) == len(NATIVE_IMAGES)
            and set(entries) == expected_files | {NATIVE_REPORT}, "Native PNG inventory or recurrence differs")

    names = {"wide", "compact", "scrolled"}
    for index, profiles in enumerate(report["replays"], 1):
        directory = root / f"replay-{index}"
        require(set(profiles) == names, "Missing native replay profile")
        shots = []
        for name, profile in profiles.items():
            native_profile(name, profile)
            shots.extend(native_shot(directory, shot, pngs, profile) for shot in profile["captures"])
        require(sorted(shots) == sorted(NATIVE_IMAGES)
                and all(entries[f"replay-{index}/{name}"] == sha for name, sha in pngs.items()),
                "Native captures do not match their two sets of original bytes")

    observations = report["owned_process_observations"]
    require(len(observations) == 2 and all(set(observation) == names for observation in observations),
            "Wrong native observation inventory")
    processes = [process for replay in observations for process in replay.values()]
    require(len({process["pid"] for process in processes}) == report["owned_processes_reaped"] == 6
            and all(process["pid"] > 0 and process["reaped"] is True and process["exit_code"] in (0, -15)
                    and process["visible_windows"] == [0] * 4 for process in processes)
            and sum(len(process["visible_windows"]) for process in processes) == report["window_samples"] == 24,
            "Native window sampling or process reaping differs")
    return {"seal": expected, "report_sha256": digest(root / NATIVE_REPORT), "runs": 2,
            "visible_windows": 0, "window_samples": 24, "owned_processes_reaped": 6,
            "transports": report["transports"], "binaries": binaries, "captures": pngs}


def guard(window):
    require(window["stock_exit"] == 0 and window["visible_owned_windows"] == []
            and window["remaining_observed_processes"] == [] and window["problems"] == [],
            "Chrome guard saw a visible window, surviving process or failure")


def check_launches(root, window, invocation):
    launch = window["guard_events"]
    ledger = [read_json(file) for file in (root / "guard-ledger").glob("*.json")]
    roles = {"chrome-primary", "chrome-bridge", "native"}
    require(len(launch) == 3 and {event["role"] for event in launch} == roles
            and len({event["pid"] for event in launch}) == 3
            and sorted(launch, key=lambda event: event["pid"]) == sorted(ledger, key=lambda event: event["pid"])
            and all(event["allowed"] is True for event in launch)
            and len({event["profile"] for event in launch if event["profile"]}) == 2,
            "Exact stock process ledger differs")
    observations = window["launch_observations"]
    require(set(observations) == roles and observations["chrome-bridge"]["detached_samples"] > 0
            and all(observations[event["role"]]["samples"] > 0
                    and observations[event["role"]]["pids"] == [event["pid"]] for event in launch),
            "A guarded child was not observed live")
    argv = invocation["argv"]
    require(argv.count("--artifacts") == 1 and argv[-len(SUITES):] == list(SUITES),
            "Raw stock command differs")
    profile_parent = PurePosixPath(argv[argv.index("--artifacts") + 1]).parent / "tmp"
    for event in launch:
        if event["role"] == "native":
            require(event["profile"] is None and event["argv"] == ["--headless", "--width", "720", "--height", "640"],
                    "Raw native headless flags differ")
            continue
        flags = [arg for arg in event["argv"] if arg == "--headless" or arg.startswith("--headless=")]
        profiles = [arg.removeprefix("--user-data-dir=") for arg in event["argv"] if arg.startswith("--user-data-dir=")]
        profile = PurePosixPath(event["profile"])
        require(flags == ["--headless=new"] and profiles == [event["profile"]]
                and profile.is_absolute() and ".." not in profile.parts
                and profile.is_relative_to(profile_parent) and profile != profile_parent,
                "Raw Chrome headless flags or private profile differ")


def web_info(root, pins, offline_file, offline_sha, runner_sha, verify_pixels):
    require(HEX.fullmatch(offline_sha or "") and digest(offline_file) == offline_sha
            and runner_sha == STOCK_RUNNER, "Wrong stock offline receipt or runner seal")
    offline = read_json(offline_file)
    stock = read_json(root / STOCK_REPORT)
    window = read_json(root / OBSERVATION)
    guard(window)
    invocation = read_json(root / "invocation.json")
    check_launches(root, window, invocation)
    for key, value in stock.items():
        require(offline[key] == value, f"Stock offline receipt differs at {key}")
    log_path = root / "runner.log.gz"
    log = materialize(log_path if log_path.exists() else root / "runner.log")
    require(offline["runner_log_sha256"] == sha256(log).hexdigest()
            and offline["window_receipt_sha256"] == digest(root / OBSERVATION), "Stock log or stock guard digest differs")
    text = log.decode()
    case_starts = re.findall(r"^case (\S+) \.\.\.$", text, re.M)
    case_ok = re.findall(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks,", text, re.M)
    cases = {name: {"steps": int(steps), "checks": int(checks)} for name, steps, checks in case_ok}
    steps = list(map(int, re.findall(r"^  \[M(\d+)\]", text, re.M)))
    ime = list(map(int, re.findall(r"^  \[I(\d+)\]", text, re.M)))
    total = sum(case["checks"] for case in cases.values())
    require(tuple(case_starts) == tuple(name for name, _, _ in case_ok) == SUITES
            and steps == [step for step in range(1, 324) if step != 23] and ime == list(range(1, 21))
            and re.findall(r"^e2e result: (.*?); artifacts in ", text, re.M) == [f"5 passed; 0 failed; {total} checks"]
            and cases == stock["cases"] and stock["total_checks"] == total and cases["material"]["steps"] == 322
            and cases["ime"] == {"steps": 20, "checks": 77} and stock["page_errors"] == 0,
            "Stock run lacks all five suites or exact M323 and IME coverage")
    adapter = stock["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False and stock["guard_and_window_checks"] == "passed",
            "Stock browser does not prove actual Apple Metal and guard success")
    after = read_json(root / "inputs-after.json")
    require(after == invocation["inputs"] == read_json(pins / "inputs-before.json")
            and invocation["suite_arguments"] == list(SUITES) and invocation["injected_chrome_flags"] == []
            and after["source_manifest_sha256"] == SOURCE and after["source_files_checked_per_origin"] == 660
            and after["release"] == offline["release"] == read_json(pins / "release-pin.json")
            and after["release"] == {"version": VERSION, "last_material_step": 323, "source_sha256": SOURCE,
                                     "source_files": 660, "source_tree": TREE,
                                     "wide_canvas_height": 800, "rtl_canvas_height": 1020}
            and digest(pins / "browser-source.sha256") == SOURCE
            and after["app_bundle_seal_sha256"] == digest(pins / "sealed-inputs.sha256")
            and after["stock_files"] == manifest(pins / "browser-binary-sha256.txt")
            and after["bundle_files_checked"] == len(after["stock_files"]) == 16
            and after["runner_sha256"] == (pins / "approved-runner.sha256").read_text().strip()
            == after["stock_files"]["debug/surface-e2e"] == runner_sha,
            "Stock proof does not match final source, invocation or sealed inputs")
    artifacts = root / "artifacts"
    smoke, ime_report, side, bridge, native = (read_json(artifacts / name) for name in (
        "browser-smoke.json", "browser-ime.json", "browser-material-side-sheet.json",
        "bridge-web-result.json", "devtools-headless-result.json"))
    require(smoke["errors"] == ime_report["errors"] == side["errors"] == []
            and smoke["browser"] == side["browser"] == stock["browser"]
            and side["adapter"] == {name: smoke["adapter"][name] for name in adapter} == adapter
            and ime_report["committed"] == "繁體中文、简体中文"
            and bridge["webgpu_ready"] is True and bridge["owned_browser_exited"] is True
            and (native["width"], native["height"], native["scale"]) == (720, 640, 1)
            and native["action_rendered"] is True and native["screenshot_transports_match"] is True,
            "Raw stock smoke, IME, bridge or DevTools report differs")
    pixels = verify_pixels(artifacts, {"browser": stock["browser"], "adapter": adapter})
    require(pixels == stock["segmented"] and pixels["scope"] == "stock-feature"
            and pixels["steps"] == ["M320", "M321", "M322", "M323"] and len(pixels["captures"]) == 5,
            "Independent stock pixel replay differs")
    return {"receipt_sha256": offline_sha, "total_checks": total, "cases": cases, "adapter": adapter,
            "browser": stock["browser"], "page_errors": 0, "visible_windows": 0, "surviving_processes": 0,
            "runner_sha256": runner_sha, "captures": pixels["captures"]}


def pixels_program(path):
    require(digest(path) == PIXELS and digest(path.parent / "verify_forms_pixels.py") == FORMS_PIXELS,
            "The stock pixel verifier or its helper changed")
    sys.path.insert(0, str(path.parent))
    try:
        return run_path(str(path))["verify"]
    finally:
        sys.path.pop(0)


def verify_stock_input(stock, offline, offline_sha, runner_sha, binary_manifest):
    verify_seal(stock, "harness.sha256", STOCK_HARNESS)
    require(digest(stock / "native-binaries.sha256") == digest(binary_manifest),
            "Stock and native captured different executable pins")
    before = read_json(stock / "inputs-before.json")
    verify_seal(stock, "sealed-inputs.sha256", before["app_bundle_seal_sha256"])
    verify_entries(stock / "bundle", manifest(stock / "browser-binary-sha256.txt"))
    baked = subprocess.run([sys.executable, "-E", "-B", str(stock / "verify-stock-receipt.py"),
                            str(stock / "run")], capture_output=True, text=True, check=False)
    require(baked.returncode == 0, f"Stock offline replay failed:\n{baked.stderr}")
    require(digest(offline) == offline_sha and json.loads(baked.stdout) == read_json(offline),
            "Controller stock receipt must match a fresh no-launch replay")
    return web_info(stock / "run", stock, offline, offline_sha, runner_sha,
                    pixels_program(stock / "verify_segmented_pixels.py"))


def required_files(stage, stock, native):
    paths = [stage / name for name in ("workspace-v0159-passed", "gpu-v0159-passed",
             "native-v0159-passed", "stock-mac-v0159-passed", OFFLINE, *RUST_LOGS, SUPPLEMENTAL)]
    paths += [stock / "run" / name for name in (STOCK_REPORT, OBSERVATION, "inputs-after.json", "runner.log")]
    paths += [native / name for name in ("SHA256SUMS", NATIVE_REPORT)]
    pending = [str(path) for path in paths if not path.is_file()]
    require(not pending, "Pending exact evidence:\n" + "\n".join(pending))


def copy(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)


def zipped(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(gzip.compress(source.read_bytes(), mtime=0))


def manifest_package(path):
    files = sorted(p for p in path.rglob("*") if p.is_file())
    (path / "SHA256SUMS").write_text("".join(f"{digest(p)}  {p.relative_to(path)}\n" for p in files))
    return digest(path / "SHA256SUMS")


def readme(out, proof):
    stock, native, rust = proof["stock_macos"], proof["native"], proof["rust"]
    prose = f"""# Surface v0.1.59 Baseline Segmented Button evidence

The 660-file candidate matches tree `{TREE}`. The patch applies to v0.1.58 tree `{BASE_TREE}`.
The source manifest SHA-256 is `{SOURCE}`.

- The workspace gate passed {rust['workspace']['passed']:,} Rust tests and three Wasm checks. The GPU gate ran {rust['metal']['passed']} standard and {rust['supplemental']['passed']} supplemental offscreen tests on Apple M1 Max Metal.
- Two native headless replays each saved four PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. {native['window_samples']} samples found zero visible windows.
- The five stock macOS suites passed {stock['total_checks']:,} checks through M323. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. The package includes five original Canvas PNGs and the typed Baseline Segmented Button report.
- Both stock IME cancellation reports and raw reports for Standard, Forms, Range, Vertical Centered and Inset Icon Sliders support the retained checks.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-segmented.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Baseline Segmented Button pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.59.patch.gz > /tmp/surface-v0.1.59.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.59.patch
git apply --whitespace=error-all /tmp/surface-v0.1.59.patch
shasum -a 256 -c /path/to/evidence/source.sha256
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/cargo-target/surface-segmented-repro
export SURFACE_GPU_EVIDENCE=/tmp/surface-segmented-repro/gpu-offscreen.png
sh scripts/check.sh
sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --lib gpu_offscreen_raster_cache -- --ignored --nocapture
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
```

The native harness README records the replay commands. The stock invocation records the five case arguments and before/after input pins. See [stock reproduction](stock-macos/reproduction.md) for the guarded replay. The full offline stock helper requires the original artifact inventory, runtime bundle and predecessor tools.

The evidence covers the local Mac. The native run used keyboard and programmatic input. Chrome used trusted keyboard, pointer and touch events and checked browser accessibility semantics, each seam, touch padding, and the two RTL focus paths. Hosted Linux, live screen-reader speech and system accessibility settings remain open.
"""
    (out / "README.md").write_text(prose)


def export(args, proof, binary_manifest):
    out = args.out
    require(not out.exists() and not out.is_symlink()
            and out.parent.resolve(strict=True).is_relative_to(Path("/tmp").resolve()),
            "Use a fresh output directory under /tmp")
    out.mkdir(mode=0o700)
    stage, stock, native = args.stage, args.stock, args.native
    copy(Path(__file__), out / "pack-segmented.py")
    copy(stage / "source.sha256", out / "source.sha256")
    copy(args.base / "source.sha256", out / "buttons-base.sha256")
    copy(stage / "segmented-tree", out / "segmented-tree")
    copy(stage / "owned-segmented.paths", out / "owned-segmented.paths")
    zipped(stage / PATCH_NAME, out / "surface-v0.1.59.patch.gz")
    for name in (*RUST_LOGS, SUPPLEMENTAL, *proof["rust"]["source_checks"]):
        zipped(stage / name, out / "rust" / f"{name}.gz")
    shutil.copytree(native, out / "native" / "proof")
    harness = out / "native" / "harness"
    for name in ("harness.sha256", *manifest(args.native_harness / "harness.sha256")):
        copy(args.native_harness / name, harness / name)
    copy(binary_manifest, out / "native" / "native-binaries.sha256")
    copy(stage / "native-final-pins.json", out / "native" / "input-pins.json")
    copy(stage / "native-final-runtime.log", out / "native" / "terminal.log")
    target = out / "stock-macos"
    for name in (*STOCK_PINS, "harness.sha256", *manifest(stock / "harness.sha256")):
        copy(stock / name, target / name)
    for name in (STOCK_REPORT, OBSERVATION, "invocation.json", "inputs-after.json"):
        copy(stock / "run" / name, target / name)
    for path in (stock / "run" / "guard-ledger").glob("*.json"):
        copy(path, target / "guard-ledger" / path.name)
    zipped(stock / "run" / "runner.log", target / "runner.log.gz")
    copy(stage / OFFLINE, target / OFFLINE)
    copy(stock / "README.md", target / "reproduction.md")
    for name in ("material-segmented.json", "material-buttons.json", *REFINEMENTS_REPORTS, *proof["stock_macos"]["captures"], *RETAINED):
        copy(stock / "run" / "artifacts" / name, target / "artifacts" / name)
    write_json(out / "verified-segmented-v0159.json", proof)
    readme(out, proof)
    seal = manifest_package(out)
    verify_package(out, seal)
    return {"path": str(out), "manifest_sha256": seal, "files": len(manifest(out / "SHA256SUMS"))}


def verify_package(root, expected):
    verify_seal(root, "SHA256SUMS", expected, strict=True)
    report = read_json(root / "verified-segmented-v0159.json")
    source = report["source"]
    patch = materialize(root / "surface-v0.1.59.patch.gz")
    patch_paths = sorted({p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch, re.M)})
    prior, current = manifest(root / "buttons-base.sha256"), manifest(root / "source.sha256")
    require(len(prior) == 656 and len(current) == 660
            and set(patch_paths) == set((root / "owned-segmented.paths").read_text().splitlines()) and len(patch_paths) == 11
            and patch_paths == sorted(name for name in prior.keys() | current.keys()
                                      if prior.get(name) != current.get(name)), "Packaged patch paths differ")

    require(source == {"version": VERSION, "source_sha256": SOURCE, "source_files": 660, "tree": TREE,
                       "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
                       "changed_paths": patch_paths}
            and digest(root / "source.sha256") == SOURCE and digest(root / "buttons-base.sha256") == BASE_SOURCE
            and (root / "segmented-tree").read_text().strip() == TREE
            and sha256(patch).hexdigest() == PATCH,
            "Packaged source pins differ")
    rust = rust_info(*(root / "rust" / f"{name}.gz" for name in RUST_LOGS),
                     supplemental=root / "rust" / f"{SUPPLEMENTAL}.gz")
    rust["source_checks"] = check_build_source(root / "rust", root / "source.sha256")
    require(rust == report["rust"], "Packaged Rust or Metal receipt differs")
    native = root / "native"
    verify_seal(native / "harness", "harness.sha256", NATIVE_HARNESS)
    require(native_info(native / "proof", report["native"]["seal"], native / "native-binaries.sha256")
            == report["native"], "Packaged native receipt differs")
    pins_text = (native / "input-pins.json").read_text().strip()
    decoder = json.JSONDecoder()
    selftest, last = decoder.raw_decode(pins_text)
    pins = json.loads(pins_text[last:].strip())
    require(selftest["application_started"] is False and selftest["synthetic_missing_focus_edges"] == 4,
            "Original no-launch native guard receipt differs")
    terminal = read_json(native / "terminal.log")
    require(pins["status"] == "pins-verified" and pins["binaries"] == report["native"]["binaries"]
            and pins["native_binary_manifest_sha256"] == BINARY_SEAL
            and pins["source"] == {"manifest_sha256": SOURCE, "files_verified": 660, "version": VERSION}
            and terminal["manifest_sha256"] == report["native"]["seal"]
            and terminal["window_samples"] == 24 and terminal["images_per_replay"] == 4
            and terminal["owned_processes_reaped"] == 6
            and terminal["visible_windows"] == 0 and terminal["pngs"] == report["native"]["captures"],
            "Original native preflight or terminal receipt differs")
    stock = root / "stock-macos"
    offline = read_json(stock / OFFLINE)
    verify_seal(stock, "harness.sha256", STOCK_HARNESS)
    verify_seal(stock, "sealed-inputs.sha256", read_json(stock / "inputs-before.json")["app_bundle_seal_sha256"])
    require(web_info(stock, stock, stock / OFFLINE, report["stock_macos"]["receipt_sha256"],
                     report["stock_macos"]["runner_sha256"],
                     pixels_program(stock / "verify_segmented_pixels.py")) == report["stock_macos"],
            "Packaged stock receipt differs")
    require(digest(stock / "artifacts/material-buttons.json") == offline["common_buttons"]["report_sha256"],
            "Packaged earlier Common Buttons report differs")
    for name in REFINEMENTS_REPORTS:
        require(digest(stock / "artifacts" / name) == offline["slider_refinements"]["report_sha256"][name],
                f"Packaged earlier Slider Refinements report differs: {name}")
    ime = offline["retained"]["retained"]["ime_cancellation"]
    for name, key in (("browser-ime-cancel-redraw.json", "select_all_after_cancel_redraw"),
                      ("browser-ime-cancel-selected-redraw.json", "backward_cancel_before_redraw")):
        require(read_json(stock / "artifacts" / name) == ime[key], f"Packaged I11 {name} differs")
    require(offline["retained"]["checks"] == report["stock_macos"]["total_checks"]
            and offline["retained"]["window_observation"]["visible_windows"] == 0
            and offline["retained"]["window_observation"]["remaining_owned_processes"] == 0
            and digest(stock / "artifacts/material-standard-slider.json") == offline["sliders_report_sha256"]
            and digest(stock / "artifacts/material-slider-forms.json") == offline["slider_forms"]["report_sha256"]
            and digest(stock / "artifacts/material-range-slider.json") == offline["range_slider"]["report_sha256"]
            and digest(stock / "native-binaries.sha256") == digest(native / "native-binaries.sha256"),
            "Packaged predecessor report or native executable pins differ")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", type=Path, default=STAGE)
    parser.add_argument("--base", type=Path, default=BASE)
    parser.add_argument("--stock", type=Path)
    parser.add_argument("--native", type=Path)
    parser.add_argument("--native-harness", type=Path, default=NATIVE_ROOT)
    parser.add_argument("--native-seal")
    parser.add_argument("--stock-offline-sha")
    parser.add_argument("--stock-runner-sha")
    parser.add_argument("--source-sha")
    parser.add_argument("--tree")
    parser.add_argument("--package-sha")
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--check-only", action="store_true")
    action.add_argument("--out", type=Path)
    action.add_argument("--verify-package", type=Path)
    args = parser.parse_args()
    if args.verify_package:
        result = verify_package(args.verify_package, args.package_sha)
    else:
        require(args.source_sha == SOURCE and args.tree == TREE, "Supply the reviewed exact source SHA and tree")
        if args.native is None:
            native_pointer = args.stage / "native-proof-dir"
            require(native_pointer.is_file(), f"Pending exact native proof: {native_pointer}")
            args.native = Path(native_pointer.read_text().strip()).resolve()
        if args.stock is None:
            stock_pointer = args.stage / "stock-observer-dir"
            require(stock_pointer.is_file(), f"Pending stock observer: {stock_pointer}")
            args.stock = Path(stock_pointer.read_text().strip()).resolve()
        checked_source = source_info(args.stage, args.base)
        required_files(args.stage, args.stock, args.native)
        require(all(HEX.fullmatch(value or "") for value in (
                    args.native_seal, args.stock_offline_sha, args.stock_runner_sha)),
                "Supply the reviewed native manifest, stock runner and offline stock receipt digests")
        require(Path((args.stage / "native-proof-dir").read_text().strip()).resolve() == args.native.resolve()
                and Path((args.stage / "stock-observer-dir").read_text().strip()).resolve() == args.stock.resolve(),
                "Stage native and stock pointers differ")
        verify_seal(args.native_harness, "harness.sha256", NATIVE_HARNESS)
        binary_manifest = args.stage / "native-v0159-binaries" / "SHA256SUMS"
        verify_entries(binary_manifest.parent, manifest(binary_manifest))
        native = native_info(args.native, args.native_seal, binary_manifest)
        rust = rust_info(*(args.stage / name for name in RUST_LOGS), supplemental=args.stage / SUPPLEMENTAL)
        rust["source_checks"] = check_build_source(args.stage, args.stage / "source.sha256")
        stock = verify_stock_input(args.stock, args.stage / OFFLINE, args.stock_offline_sha,
                                   args.stock_runner_sha, binary_manifest)
        result = {"status": "verified", "source": checked_source, "rust": rust, "native": native,
                  "stock_macos": stock}
        if args.out:
            result = {**result, "output": export(args, result, binary_manifest)}
    print(json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, FileNotFoundError, subprocess.CalledProcessError, KeyError, TypeError) as error:
        print(f"Baseline Segmented Button proof stopped. {error}", file=sys.stderr)
        sys.exit(1)
