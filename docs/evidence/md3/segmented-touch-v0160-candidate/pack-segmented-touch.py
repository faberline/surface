#!/usr/bin/env python3
"""Package exact Segmented Button touch-panning evidence without starting a host."""

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

STAGE = Path("/tmp/surface-md3-segmented-touch-v0160.5syabA1L")
BASE = Path("/tmp/surface-md3-segmented-v0159.yzqEnSpq")
NATIVE_ROOT = Path("/tmp/surface-md3-segmented-touch-native.26fxPGhh")
FOCUSED_NAME = "development-3"
FOCUSED_RUNNER = "f76d3c468273580cb445c8de9289fb1694a2fba3d1d663e07e063cbb1f0d60e6"
FOCUSED_FILES = ("invocation.json", "headless-window-observation.json")
VERSION = "0.1.60"
SOURCE = "4874c29831d6a87a60c1e3b98071cc522d2ab9a933deab60b6f33091bcfd25b9"
TREE = "b9539a68d5993b8c29a9dc17da571b31078c6874"
BASE_SOURCE = "9051c16e996c9b310062682765a7d0bc54cf42dfbadbda4c5f63bd457b5d53b8"
BASE_TREE = "757179e260a331b4488c6be1b49410263097d675"
PATCH = "bb9a2a5adcf0e984608deda77fca5d364c02704433f2cfb06355ffb549cd46d1"
PATCH_NAME = "segmented-touch-v0160.patch"
NATIVE_HARNESS = "eeb30ca9286fdb147276fb5b98b1c9704f52d303299200404e585862c1779c32"
STOCK_HARNESS = "49c91fe1b3b4c7dc7abfc7a3fd80c76208c8ba6841b0ab36116f87e0a9c13adf"
PIXELS = "45b72cdf45e1acfed36c285e1de7fd8ceef0019aec215e3f017889640b7ad1a4"
STOCK_RUNNER = "6e0a690cf3157c7143a2b758d51d56f7387964d9e9fca676f7e73caeed7d2c41"
FORMS_PIXELS = "13d2a3b233e36fbbf44410c3b2b4f213a6832e4495670ed54976fce2d6e62b95"
REFINEMENTS_REPORTS = ("material-vertical-centered-slider.json", "material-slider-inset.json")
NATIVE_REPORT = "native-segmented-report.json"
STOCK_REPORT = "verified-segmented-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
OBSERVATION = "headless-window-observation.json"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
RUST_LOGS = ("workspace-v0160-exact.log", "gpu-v0160-exact.log")
SUPPLEMENTAL = "gpu-v0160-supplemental.log"
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
NATIVE_SEAL = "ff3975aa0ec8d5590b71cfe25c47eaebebf9e41c7d8449e8be0c0cb3e142f1f2"
BINARY_SEAL = "2a14b23928fc7c5bb00ba296e620e48a2f04320d464ae68478b483374b36e7bf"
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
    for root, entries, count, tree in ((base / "source", before, 660, BASE_TREE),
                                       (stage / "source", after, 662, TREE)):
        require(len(entries) == count, "Wrong source inventory count")
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
                  if p.is_file() and p.relative_to(root).parts[0] != ".git"}
        require(actual == set(entries), f"Source file inventory differs: {root}")
        verify_entries(root, entries)
        require(source_tree(root, entries) == tree, f"Wrong exact source tree: {root}")
    root = stage / "source"
    patch = stage / PATCH_NAME
    require(digest(patch) == PATCH and (stage / "segmented-touch-tree").read_text().strip() == TREE
            and tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == VERSION,
            "Wrong candidate patch, tree marker or workspace version")
    material = (root / "e2e/src/material.rs").read_text()
    registry = re.search(r"pub const STEPS: &\[&str\] = &\[(.*?)\];", material, re.S)
    required_steps = [str(step) for step in range(1, 325) if step != 23]
    require(registry and re.findall(r'"M(\d+)"', registry.group(1)) == required_steps,
            "The stock runner must require every Material step through M324 except failure-only M23")
    paths = {p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch.read_bytes(), re.M)}
    changed = {name for name in before.keys() | after.keys() if before.get(name) != after.get(name)}
    require(paths == changed == set((stage / "owned-segmented-touch.paths").read_text().splitlines()) and len(paths) == 16,
            "Patch, source inventory and the 16 reviewed paths differ")
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
    require(rust["passed"] == 1515 and rust["ignored"] == 19
            and tail.count("Finished `dev` profile") == 3
            and re.search(r"^provenance: .+ self-test cases passed\)", text, re.M)
            and "Doc-tests surface_material_gallery" in text
            and "range_slider_steps_are_required_by_the_material_case ... ok" in text
            and "segmented_steps_are_required_by_the_material_case ... ok" in text
            and "horizontal_touch_is_physical_in_both_directions_and_locks_its_axis ... ok" in text
            and "vertical_touch_started_in_the_inner_strip_moves_only_the_parent ... ok" in text
            and "cancel_and_matched_host_loss_restore_offset_and_retire_the_old_touch ... ok" in text
            and "icon_stacks_when_more_chinese_text_fits_on_each_line ... ok" in text
            and "compact_rtl_grows_one_row_and_paints_the_whole_focus_outline ... ok" in text
            and "narrow_rtl_scroll_keeps_whole_targets_and_reveals_keyboard_and_ax_choices ... ok" in text,
            "The workspace lacks complete Segmented touch, provenance or Wasm checks")
    return {"workspace": {**rust, "wasm_checks": 3}, **metal_info(gpu, supplemental)}


def metal_info(gpu, supplemental):
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
    return {"metal": {**metal, "backend": "Metal"}, "supplemental": {**extra, "backend": "Metal"}}


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
    layout = shot.get("list_artwork")
    if name in ("wide-light-rest.png", "compact-dark-rtl-200.png"):
        stacked = name.startswith("compact")
        require(layout and layout["arrangement"] == ("stacked" if stacked else "leading"),
                "The exact native icon arrangement is missing")
        face, icon, label = (layout["geometry"][part] for part in ("face", "icon", "label"))
        require((icon["width"], icon["height"]) == (18, 18), "Native List icon size changed")
        for child in (icon, label):
            require(all(face[axis] <= child[axis] and child[axis] + child[unit] <= face[axis] + face[unit] + .06
                        for axis, unit in (("x", "width"), ("y", "height"))), "Native List art clips")
        if stacked:
            require(abs(label["y"] - icon["y"] - 18 - 8) < .06 and label["width"] >= 56
                    and label["height"] == 80
                    and abs((label["x"] + label["width"] / 2) - (icon["x"] + 9)) < .06,
                    "Native compact icon did not stack above the two-line label")
        else:
            require(abs(label["x"] - icon["x"] - 18 - 8) < .06
                    and abs((label["y"] + label["height"] / 2) - (icon["y"] + 9)) < .06,
                    "Native desktop leading icon or gap changed")
        ink = layout["icon_ink"]
        require(paint_count(picture, icon, ink["rgba"]) == ink["pixels"] >= 20
                and layout["label_ink"]["bounds"] == label, "Native compact icon or label ink differs")
        native_ink(picture, layout["label_ink"])
    else:
        require(layout is None, "Unexpected List artwork checkpoint")
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
            and report["source"] == {"manifest_sha256": SOURCE, "files_verified": 662, "version": VERSION}
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
            and steps == [step for step in range(1, 325) if step != 23] and ime == list(range(1, 21))
            and re.findall(r"^e2e result: (.*?); artifacts in ", text, re.M) == [f"5 passed; 0 failed; {total} checks"]
            and cases == stock["cases"] and stock["total_checks"] == total and cases["material"]["steps"] == 323
            and cases["ime"] == {"steps": 20, "checks": 77} and stock["page_errors"] == 0,
            "Stock run lacks all five suites or exact M324 and IME coverage")
    adapter = stock["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False and stock["guard_and_window_checks"] == "passed",
            "Stock browser does not prove actual Apple Metal and guard success")
    after = read_json(root / "inputs-after.json")
    require(after == invocation["inputs"] == read_json(pins / "inputs-before.json")
            and invocation["suite_arguments"] == list(SUITES) and invocation["injected_chrome_flags"] == []
            and after["source_manifest_sha256"] == SOURCE and after["source_files_checked_per_origin"] == 662
            and after["release"] == offline["release"] == read_json(pins / "release-pin.json")
            and after["release"] == {"version": VERSION, "last_material_step": 324, "source_sha256": SOURCE,
                                     "source_files": 662, "source_tree": TREE,
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
            and pixels["steps"] == ["M320", "M321", "M322", "M323", "M324"] and len(pixels["captures"]) == 7,
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


def focused_info(root, verify_pixels, *, check_live_inputs=False):
    invocation = read_json(root / "invocation.json")
    window = read_json(root / OBSERVATION)
    guard(window)
    argv = invocation["argv"]
    require(invocation["scope"] == "focused development pack" and invocation["runner_sha256"] == FOCUSED_RUNNER
            and len(argv) == 8 and argv[1:2] == ["--chrome"] and argv[3:4] == ["--web-root"]
            and argv[5:6] == ["--artifacts"] and argv[-1] == "material"
            and Path(argv[0]).parent.name == "focused-bin-r3" and Path(argv[4]).parent.name == "web-r2",
            "The focused run was not the sealed R3 feature-only command")
    expected_web = len(invocation["web_files"]) == 4 and set(invocation["web_files"]) >= {"index.html", "surface-web.json"}
    require(expected_web and sum(name.endswith(".wasm") for name in invocation["web_files"]) == 1,
            "Focused Gallery Wasm inventory changed")
    if check_live_inputs:
        binary, assets = Path(argv[0]), Path(argv[4]) / "material-gallery"
        require(digest(binary) == FOCUSED_RUNNER and {path.name for path in assets.iterdir()} == set(invocation["web_files"]),
                "Focused runner or archived Gallery assets changed")
        verify_entries(assets, invocation["web_files"])
    event, = window["guard_events"]
    ledger = [read_json(file) for file in (root / "guard-ledger").glob("*.json")]
    child = window["launch_observations"]["chrome-primary"]
    headless_flags = [flag for flag in event["argv"] if flag == "--headless" or flag.startswith("--headless=")]
    profile = PurePosixPath(event["profile"])
    require(ledger == [event] and event["role"] == "chrome-primary" and event["allowed"] is True
            and headless_flags == ["--headless=new"] and child["pids"] == [event["pid"]] and child["samples"] > 0
            and profile.is_relative_to(PurePosixPath(argv[-2]).parent / "tmp"),
            "The focused Chrome process was not observed with a private headless profile")
    log_path = root / "runner.log"
    log = materialize(log_path if log_path.is_file() else root / "runner.log.gz")
    text = log.decode()
    require(re.findall(r"^case (\S+) \.\.\.$", text, re.M) == ["material"]
            and re.findall(r"^  \[(M\d+)\]", text, re.M) == [f"M{step}" for step in range(320, 325)]
            and re.findall(r"^case material \.\.\. ok \((\d+) steps, (\d+) checks,", text, re.M) == [("5", "474")]
            and re.findall(r"^e2e result: (.*?); artifacts in ", text, re.M) == ["1 passed; 0 failed; 474 checks"],
            "Focused R3 log differs from the five actual steps and 474 checks")
    feature = verify_pixels(root / "artifacts", calibration=True)
    require(feature["scope"] == "focused development calibration" and len(feature["captures"]) == 7,
            "Focused R3 feature evidence is incomplete")
    return {**feature, "checks": 474, "suite": "material", "visible_windows": 0, "remaining_owned_processes": 0,
            "runner_log_sha256": sha256(log).hexdigest(), "window_receipt_sha256": digest(root / OBSERVATION),
            "runner_sha256": FOCUSED_RUNNER, "web_files": invocation["web_files"]}


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
    paths = [stage / name for name in ("workspace-v0160-passed", "gpu-v0160-passed",
             "native-v0160-passed", "stock-mac-v0160-passed", OFFLINE, *RUST_LOGS, SUPPLEMENTAL)]
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
    prose = f"""# Surface v0.1.60 Segmented Button touch evidence

The 662-file candidate matches tree `{TREE}`. The patch applies to v0.1.59 tree `{BASE_TREE}`.
The source manifest SHA-256 is `{SOURCE}`.

- The workspace gate passed {rust['workspace']['passed']:,} Rust tests and three Wasm checks. The GPU gate ran {rust['metal']['passed']} standard and {rust['supplemental']['passed']} supplemental offscreen tests on Apple M1 Max Metal.
- Two native headless replays each saved four PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. {native['window_samples']} samples found zero visible windows.
- The 320 dp native frame shows the 18 dp icon centered above a complete two-line label, with an 8 dp gap. The verifier rechecks the icon and label pixels against the original in each replay.
- The five stock macOS suites passed {stock['total_checks']:,} checks through M324. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. The package includes seven original stock Canvas PNGs and the typed Segmented Button report, including both touch-list directions, vertical handoff and trusted capture loss.
- The separate R3 focused run passed 474 checks in one case. It supplies seven development PNGs. Its runner, bundle and guard receipt remain under `focused`.
- Both stock IME cancellation reports and raw reports for Standard, Forms, Range, Vertical Centered and Inset Icon Sliders support the retained checks.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-segmented-touch.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Baseline Segmented Button pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.60.patch.gz > /tmp/surface-v0.1.60.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.60.patch
git apply --whitespace=error-all /tmp/surface-v0.1.60.patch
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
    copy(Path(__file__), out / "pack-segmented-touch.py")
    copy(stage / "source.sha256", out / "source.sha256")
    copy(args.base / "source.sha256", out / "segmented-base.sha256")
    copy(stage / "segmented-touch-tree", out / "segmented-touch-tree")
    copy(stage / "owned-segmented-touch.paths", out / "owned-segmented-touch.paths")
    zipped(stage / PATCH_NAME, out / "surface-v0.1.60.patch.gz")
    for name in (*RUST_LOGS, SUPPLEMENTAL, *proof["rust"]["source_checks"]):
        zipped(stage / name, out / "rust" / f"{name}.gz")
    shutil.copytree(native, out / "native" / "proof")
    harness = out / "native" / "harness"
    for name in ("harness.sha256", *manifest(args.native_harness / "harness.sha256")):
        copy(args.native_harness / name, harness / name)
    copy(binary_manifest, out / "native" / "native-binaries.sha256")
    copy(stage / "native-final-pins.json", out / "native" / "input-pins.json")
    copy(stage / "native-final-runtime.log", out / "native" / "terminal.log")
    focused = out / "focused"
    original = stage / FOCUSED_NAME
    for name in FOCUSED_FILES:
        copy(original / name, focused / name)
    for path in (original / "guard-ledger").glob("*.json"):
        copy(path, focused / "guard-ledger" / path.name)
    zipped(original / "runner.log", focused / "runner.log.gz")
    for name in ("material-segmented.json", *proof["focused"]["captures"]):
        copy(original / "artifacts" / name, focused / "artifacts" / name)
    copy(stage / "focused-bin-r3" / "runner.sha256", focused / "approved-runner.sha256")
    copy(stage / "focused-bin-r3" / "scope-selector.rs", focused / "scope-selector.rs")
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
    write_json(out / "verified-segmented-touch-v0160.json", proof)
    readme(out, proof)
    seal = manifest_package(out)
    verify_package(out, seal)
    return {"path": str(out), "manifest_sha256": seal, "files": len(manifest(out / "SHA256SUMS"))}


def verify_package(root, expected):
    verify_seal(root, "SHA256SUMS", expected, strict=True)
    report = read_json(root / "verified-segmented-touch-v0160.json")
    source = report["source"]
    patch = materialize(root / "surface-v0.1.60.patch.gz")
    patch_paths = sorted({p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch, re.M)})
    prior, current = manifest(root / "segmented-base.sha256"), manifest(root / "source.sha256")
    require(len(prior) == 660 and len(current) == 662
            and set(patch_paths) == set((root / "owned-segmented-touch.paths").read_text().splitlines()) and len(patch_paths) == 16
            and patch_paths == sorted(name for name in prior.keys() | current.keys()
                                      if prior.get(name) != current.get(name)), "Packaged patch paths differ")

    require(source == {"version": VERSION, "source_sha256": SOURCE, "source_files": 662, "tree": TREE,
                       "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
                       "changed_paths": patch_paths}
            and digest(root / "source.sha256") == SOURCE and digest(root / "segmented-base.sha256") == BASE_SOURCE
            and (root / "segmented-touch-tree").read_text().strip() == TREE
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
            and pins["source"] == {"manifest_sha256": SOURCE, "files_verified": 662, "version": VERSION}
            and terminal["manifest_sha256"] == report["native"]["seal"]
            and terminal["window_samples"] == 24 and terminal["images_per_replay"] == 4
            and terminal["owned_processes_reaped"] == 6
            and terminal["visible_windows"] == 0 and terminal["pngs"] == report["native"]["captures"],
            "Original native preflight or terminal receipt differs")
    stock = root / "stock-macos"
    offline = read_json(stock / OFFLINE)
    verify_seal(stock, "harness.sha256", STOCK_HARNESS)
    verify_seal(stock, "sealed-inputs.sha256", read_json(stock / "inputs-before.json")["app_bundle_seal_sha256"])
    focused = root / "focused"
    verify_pixels = pixels_program(stock / "verify_segmented_pixels.py")
    require((focused / "approved-runner.sha256").read_text().strip() == FOCUSED_RUNNER
            and focused_info(focused, verify_pixels) == report["focused"],
            "Packaged focused R3 results differ from original artifacts")
    require(web_info(stock, stock, stock / OFFLINE, report["stock_macos"]["receipt_sha256"],
                     report["stock_macos"]["runner_sha256"],
                     verify_pixels) == report["stock_macos"],
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
    action.add_argument("--check-preflight", action="store_true")
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
        require(HEX.fullmatch(args.native_seal or ""), "Supply the reviewed exact native manifest digest")
        require(all(HEX.fullmatch(value or "") for value in (
                    args.stock_offline_sha, args.stock_runner_sha)) or args.check_preflight,
                "Supply the reviewed native manifest, stock runner and offline stock receipt digests")
        require(Path((args.stage / "native-proof-dir").read_text().strip()).resolve() == args.native.resolve()
                and Path((args.stage / "stock-observer-dir").read_text().strip()).resolve() == args.stock.resolve(),
                "Stage native and stock pointers differ")
        verify_seal(args.native_harness, "harness.sha256", NATIVE_HARNESS)
        binary_manifest = args.stage / "native-v0160-binaries" / "SHA256SUMS"
        verify_entries(binary_manifest.parent, manifest(binary_manifest))
        native = native_info(args.native, args.native_seal, binary_manifest)
        verify_seal(args.stock, "harness.sha256", STOCK_HARNESS)
        require(digest(args.stock / "browser-source.sha256") == SOURCE
                and (args.stock / "approved-runner.sha256").read_text().strip() == STOCK_RUNNER,
                "Wrong final stock source or runner pins")
        verify_entries(args.stock / "bundle", manifest(args.stock / "browser-binary-sha256.txt"))
        focused = focused_info(args.stage / FOCUSED_NAME, pixels_program(args.stock / "verify_segmented_pixels.py"),
                               check_live_inputs=True)
        if args.check_preflight:
            rust = metal_info(args.stage / RUST_LOGS[1], args.stage / SUPPLEMENTAL)
            if (args.stage / "workspace-v0160-passed").is_file():
                rust = rust_info(*(args.stage / name for name in RUST_LOGS), supplemental=args.stage / SUPPLEMENTAL)
                rust["source_checks"] = check_build_source(args.stage, args.stage / "source.sha256")
            print(json.dumps({"status": "preflight only",
                              "source": checked_source, "native": native, "focused": focused,
                              "rust": rust, "full_stock": "not evaluated"},
                             indent=2, ensure_ascii=False, sort_keys=True))
            return
        required_files(args.stage, args.stock, args.native)
        rust = rust_info(*(args.stage / name for name in RUST_LOGS), supplemental=args.stage / SUPPLEMENTAL)
        rust["source_checks"] = check_build_source(args.stage, args.stage / "source.sha256")
        stock = verify_stock_input(args.stock, args.stage / OFFLINE, args.stock_offline_sha,
                                   args.stock_runner_sha, binary_manifest)
        result = {"status": "verified", "source": checked_source, "rust": rust, "native": native, "focused": focused,
                  "stock_macos": stock}
        if args.out:
            result = {**result, "output": export(args, result, binary_manifest)}
    print(json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, FileNotFoundError, subprocess.CalledProcessError, KeyError, TypeError) as error:
        print(f"Segmented touch proof stopped. {error}", file=sys.stderr)
        sys.exit(1)
