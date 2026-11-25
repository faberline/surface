#!/usr/bin/env python3
"""Package exact Outlined Text Field evidence without starting a host."""

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

STAGE = Path("/tmp/surface-md3-outlined-field-tests-r2.j9B9xWVq")
R1 = Path("/tmp/surface-md3-outlined-field-v0161.ByvT73oG")
R1_SOURCE = "3624a5770f973ad72ccf2b02689cbd24b543c01187a9396738025d4c9646fe40"
R1_TREE = "09a0450473b05b44626bdb6f163fe92ca6ce3eff"
TEST_DELTA = "crates/surface-material/tests/outlined_text_field.rs"
R1_RUST = "7357e418bf523138de9366b6088c6dd9b4529ac9ed4d8806457b7ae8ad9526ab"
R1_FOCUSED = "9db1280cfcf50bc39639d5bee20c18ffbd6b3530a142b7b7ddbe072b98779782"
R1_NATIVE = "a640e988ada60e9c415918f6f4e9f635c621681c378d98b944e9b669b7d470dc"
BASE = Path("/tmp/surface-md3-segmented-touch-v0160.5syabA1L")
NATIVE_ROOT = Path("/tmp/surface-md3-outlined-native.3OHyWk9M")
FOCUSED_NAME = "development-1"
FOCUSED_RUNNER = R1 / "focused-bin-final" / "surface-e2e"
FOCUSED_RUNNER_SHA = "32f4a59b26c42b82843ccf56bc7d275eefeac7e1b35f4e2208d9a6d514df83a3"
STOCK_GALLERY = PurePosixPath("web/release/material-gallery")
FOCUSED_FILES = ("invocation.json", "headless-window-observation.json")
VERSION = "0.1.61"
SOURCE = "3c79ae5ee15bf3b9b786d37452f1e0e5929ac0d31276ca7f6506897462dd9cc8"
TREE = "7c7881f78c3dc44129f52688a079b6874c9a4991"
BASE_SOURCE = "4874c29831d6a87a60c1e3b98071cc522d2ab9a933deab60b6f33091bcfd25b9"
BASE_TREE = "b9539a68d5993b8c29a9dc17da571b31078c6874"
PATCH = "9e2f31202208fc5ad24c3ae68525932cd22e631496f64ba7f7fa74eee64ff70f"
PATCH_NAME = "outlined-field-v0161.patch"
NATIVE_HARNESS = "15df1a73e5acee5f851d75d18fc6671cf2796cf59a59c3fb5efbb939b971be4a"
STOCK_HARNESS = "aecfc1b9d5f596d182de349d478bdaff24bb925a4f8b9f70ec6df1d345c46693"
PIXELS = "4e0d205a47db4e6ccaf4c5f70d7f74ff6e1434f7af97af27d35ae250423f67f7"
STOCK = Path("/tmp/surface-md3-outlined-field-stock-v0161-r2.pT4k0wSO")
FORMS_PIXELS = "13d2a3b233e36fbbf44410c3b2b4f213a6832e4495670ed54976fce2d6e62b95"
REFINEMENTS_REPORTS = ("material-vertical-centered-slider.json", "material-slider-inset.json")
NATIVE_REPORT = "native-outlined-report.json"
STOCK_REPORT = "verified-outlined-field-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
OBSERVATION = "headless-window-observation.json"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
RUST_LOGS = ("workspace-v0161-exact.log", "gpu-v0161-exact.log")
SUPPLEMENTAL = "gpu-v0161-supplemental.log"
INTEGRITY_LOGS = tuple(
    f"{group}-source-{point}.log"
    for group in ("workspace", "gpu")
    for point in ("before", "after")
)
NATIVE_IMAGES = {
    "wide-idle.png": (960, 960),
    "wide-focused.png": (960, 960),
    "compact-error.png": (320, 960),
    "compact-long-label.png": (320, 960),
}
STOCK_PINS = (
    "browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256",
    "release-pin.json", "source-origins.json", "build-markers.json", "sealed-inputs.sha256",
    "inputs-before.json", "approved-runner.sha256",
)
RETAINED = (
    "browser-ime.json", "browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json",
    "browser-smoke.json", "browser-material-side-sheet.json", "bridge-web-result.json",
    "devtools-headless-result.json",
    "material-standard-slider.json", "material-slider-forms.json", "material-range-slider.json", "material-segmented.json",
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
    for root, entries, count, tree in ((base / "source", before, 662, BASE_TREE),
                                       (stage / "source", after, 668, TREE)):
        require(len(entries) == count, "Wrong source inventory count")
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
                  if p.is_file() and p.relative_to(root).parts[0] != ".git"}
        require(actual == set(entries), f"Source file inventory differs: {root}")
        verify_entries(root, entries)
        require(source_tree(root, entries) == tree, f"Wrong exact source tree: {root}")
    root = stage / "source"
    patch = stage / PATCH_NAME
    require(digest(patch) == PATCH and (stage / "outlined-field-tree").read_text().strip() == TREE
            and tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == VERSION,
            "Wrong candidate patch, tree marker or workspace version")
    material = (root / "e2e/src/material.rs").read_text()
    registry = re.search(r"pub const STEPS: &\[&str\] = &\[(.*?)\];", material, re.S)
    required_steps = [str(step) for step in range(1, 329) if step != 23]
    require(registry and re.findall(r'"M(\d+)"', registry.group(1)) == required_steps,
            "The stock runner must require every Material step through M328 except failure-only M23")
    paths = {p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch.read_bytes(), re.M)}
    changed = {name for name in before.keys() | after.keys() if before.get(name) != after.get(name)}
    require(paths == changed == set((stage / "owned-outlined-field.paths").read_text().splitlines()) and len(paths) == 23,
            "Patch, source inventory and the 23 reviewed paths differ")
    for directory, options in ((base / "source", []), (root, ["--reverse"])):
        subprocess.run(["git", "-C", str(directory), "apply", "--check", "--whitespace=error-all",
                        *options, str(patch)], check=True, env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"})
    return {"version": VERSION, "source_sha256": SOURCE, "source_files": len(after), "tree": TREE,
            "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
            "changed_paths": sorted(paths)}


def materialize(log):
    return gzip.decompress(log.read_bytes()) if log.suffix == ".gz" else log.read_bytes()


def r1_delta(prior, current):
    require(digest(prior) == R1_SOURCE and digest(current) == SOURCE, "Wrong R1/R2 source seals")
    before, after = manifest(prior), manifest(current)
    require(before.keys() == after.keys() and len(after) == 668
            and [path for path in before if before[path] != after[path]] == [TEST_DELTA],
            "R2 differs from R1 outside the one reviewed Rust test")
    return {name: checksum for name, checksum in before.items() if name != TEST_DELTA}


def calibration_r1(prior, current, rust_log, focused_report, native_report):
    unchanged = r1_delta(prior, current)
    rust = materialize(rust_log)
    require(sha256(rust).hexdigest() == R1_RUST and
            b"focus_repaints_one_semantic_editor_and_cuts_a_transparent_three_dp_outline ... FAILED" in rust and
            b"test result: FAILED. 3 passed; 1 failed" in rust and
            b"left: [NodeId(2), NodeId(7), NodeId(8)]" in rust,
            "The original R1 failed workspace receipt differs")
    require(digest(focused_report) == R1_FOCUSED and digest(native_report) == R1_NATIVE,
            "The original R1 calibration report bytes changed")
    native = read_json(native_report)
    require(native["source"] == {"manifest_sha256": R1_SOURCE, "files": 668}
            and len(native["replays"]) == 2, "R1 native report source or scope differs")
    return {"scope": "R1 development calibration only", "source_sha256": R1_SOURCE, "tree": R1_TREE,
            "workspace": "failed", "workspace_log_sha256": R1_RUST,
            "focused_report_sha256": R1_FOCUSED, "native_report_sha256": R1_NATIVE,
            "test_only_delta": TEST_DELTA, "identical_files": len(unchanged)}


def check_r1_sources(stage):
    original = R1 / "source.sha256"
    final = stage / "source.sha256"
    unchanged = r1_delta(original, final)
    require(unchanged == manifest(stage / "unchanged.sha256"), "The recorded unchanged hashes differ")
    entries = manifest(original)
    verify_entries(R1 / "source", entries)
    require(source_tree(R1 / "source", entries) == R1_TREE, "Wrong original R1 tree")
    return calibration_r1(original, final, R1 / RUST_LOGS[0],
                          R1 / "development-2" / "artifacts" / "material-outlined-text-field.json",
                          NATIVE_ROOT / "exact-1" / NATIVE_REPORT)


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


def scoped_r2(log):
    text, receipt = tests(log)
    require(receipt["passed"] == 4 and receipt["ignored"] == 0
            and "focus_repaints_one_semantic_editor_and_cuts_a_transparent_three_dp_outline ... ok" in text,
            "R2 must pass the actual four scoped Outlined Text Field tests")
    return {"scope": "R2 scoped Rust integration", **receipt}


def rust_info(workspace, gpu, supplemental):
    text, rust = tests(workspace)
    tail = text.rsplit("\ntest result:", 1)[-1]
    require(rust["passed"] >= 1519 and rust["ignored"] == 19
            and tail.count("Finished `dev` profile") == 3
            and re.search(r"^provenance: .+ self-test cases passed\)", text, re.M)
            and "Doc-tests surface_material_gallery" in text
            and "range_slider_steps_are_required_by_the_material_case ... ok" in text
            and "segmented_steps_are_required_by_the_material_case ... ok" in text
            and "outlined_text_field_steps_are_required_by_the_material_case ... ok" in text
            and "horizontal_touch_is_physical_in_both_directions_and_locks_its_axis ... ok" in text
            and "vertical_touch_started_in_the_inner_strip_moves_only_the_parent ... ok" in text
            and "cancel_and_matched_host_loss_restore_offset_and_retire_the_old_touch ... ok" in text
            and "focus_repaints_one_semantic_editor_and_cuts_a_transparent_three_dp_outline ... ok" in text
            and "same_id_theme_scale_and_validation_rebuild_preserve_backward_selection_and_preedit ... ok" in text
            and "compact_rtl_long_label_wraps_above_a_closed_outline ... ok" in text
            and "read_only_selects_without_edits_and_disabled_leaves_the_tab_order ... ok" in text
            and "compact_rtl_grows_one_row_and_paints_the_whole_focus_outline ... ok" in text
            and "narrow_rtl_scroll_keeps_whole_targets_and_reveals_keyboard_and_ax_choices ... ok" in text,
            "The workspace lacks complete outlined field, provenance or Wasm checks")
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


def native_shot(directory, shot, mode):
    name = shot["name"]
    goes_wide = mode == "wide"
    rtl = not goes_wide
    above = name == "compact-long-label.png"
    focused = name != "wide-idle.png"
    size = NATIVE_IMAGES[name]
    path = directory / name
    require((shot["width"], shot["height"]) == size and shot["scale"] == 1
            and shot["format"] == "png" and digest(path) == shot["sha256"], f"Changed native capture: {name}")
    picture = open_png(path, size)
    proof = shot["outline"]
    face, label = proof["face"], proof["label"]
    x, y, width, height = (face[part] for part in ("x", "y", "width", "height"))
    require(height == (56 if goes_wide else 80), f"Wrong native field height: {name}")
    scan_box(picture, face)

    def pixel(px, py):
        return list(picture.getpixel((floor(px), floor(py))))

    solid_x = x + 7 if rtl else x + width - 7
    band = [pixel(solid_x, y + n + .5) for n in range(5)]
    background = pixel(x + width - 6, y + 10)
    thickness = 3 if focused else 1
    require(band == proof["band"] and background == proof["background"]
            and all(color == band[0] for color in band[:thickness])
            and band[0] != background and all(color == background for color in band[thickness:]),
            f"Native original does not paint an actual {thickness} dp outline: {name}")
    if above:
        require(label["width"] == 0 and label["y"] < y, "Long native label created a clipped notch")
        gap_x = x + 18
    else:
        require(label["width"] > 0 and label["y"] < y < label["y"] + label["height"],
                "Native floating label misses its outline")
        gap_x = label["x"] + label["width"] + 2 if rtl else label["x"] - 2
    gap = pixel(gap_x, y + .5)
    require(gap == proof["gap"] == (background if focused and not above else band[0]),
            f"Native notch or closed outline differs: {name}")
    disabled = shot.get("disabled_value")
    require((disabled is None) == above, "Missing native disabled value")
    if disabled:
        box = disabled["bounds"]
        paper = pixel(box["x"] - 10, box["y"] + box["height"] / 2)
        pixels = [px for px, py in scan_box(picture, box)
                  if max(abs(a - b) for a, b in zip(pixel(px, py)[:3], paper[:3])) > 12]
        require(len(pixels) == disabled["ink_pixels"] > 40
                and [min(pixels), max(pixels)] == disabled["ink_x"],
                f"Disabled native text does not match the original ink: {name}")
        starts = box["x"] + box["width"] - max(pixels) if rtl else min(pixels) - box["x"]
        require(starts < 12, f"Disabled native text does not align to logical start: {name}")
    return name, shot["sha256"]


def native_info(root, expected, binaries_manifest, binary_sha):
    require(HEX.fullmatch(expected or "") and HEX.fullmatch(binary_sha or "")
            and digest(binaries_manifest) == binary_sha,
            "Wrong exact native proof or executable manifest seal")
    entries = verify_seal(root, "SHA256SUMS", expected, strict=True)
    report = read_json(root / NATIVE_REPORT)
    binaries = manifest(binaries_manifest)
    require(set(binaries) == {"surface-material-gallery", "surface-devtools"}
            and report["binaries"] == binaries, "Native proof used different executables")
    require(report["source"] == {"manifest_sha256": SOURCE, "files": 668}
            and report["transports"] == ["Page.screenshot", "CLI", "MCP"]
            and len(report["replays"]) == len(report["processes"]) == 2
            and report["replays"][0] == report["replays"][1], "Wrong native inputs, scope or replay parity")
    expected_members = {f"replay-{i}/{name}" for i in (1, 2) for name in NATIVE_IMAGES} | {NATIVE_REPORT}
    require(set(entries) == expected_members, "Wrong native original image inventory")
    processes, captures = [], {}
    for index, (profiles, children) in enumerate(zip(report["replays"], report["processes"], strict=True), 1):
        require(set(profiles) == set(children) == {"wide", "compact"}, "Missing native profile")
        directory = root / f"replay-{index}"
        seen = {}
        for mode, profile in profiles.items():
            size = [960 if mode == "wide" else 320, 960]
            require(profile["viewport"] == size and len(profile["captures"]) == 2, "Wrong native viewport")
            for shot in profile["captures"]:
                name, checksum = native_shot(directory, shot, mode)
                require(name not in seen and entries[f"replay-{index}/{name}"] == checksum,
                        "Duplicate capture or changed original native bytes")
                seen[name] = checksum
            child = children[mode]
            require(child["pid"] > 0 and child["reaped"] is True and child["visible_windows"] == [0] * 4,
                    "Native replay contains visible windows or an unreaped process")
            processes.append(child)
        require(set(seen) == set(NATIVE_IMAGES), "Wrong native image inventory")
        if index == 1:
            captures = seen
        else:
            require(captures == seen, "Independent native capture bytes changed")
    require(len({item["pid"] for item in processes}) == 4, "Native replay process IDs were reused")
    return {"seal": expected, "report_sha256": digest(root / NATIVE_REPORT), "runs": 2,
            "binary_manifest_sha256": binary_sha,
            "visible_windows": 0, "window_samples": 16, "owned_processes_reaped": 4,
            "transports": report["transports"], "binaries": binaries, "captures": captures}


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
            and HEX.fullmatch(runner_sha or ""), "Wrong stock offline receipt or runner seal")
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
            and steps == [step for step in range(1, 329) if step != 23] and ime == list(range(1, 21))
            and re.findall(r"^e2e result: (.*?); artifacts in ", text, re.M) == [f"5 passed; 0 failed; {total} checks"]
            and cases == stock["cases"] and stock["total_checks"] == total and cases["material"]["steps"] == 327
            and cases["ime"] == {"steps": 20, "checks": 77} and stock["page_errors"] == 0,
            "Stock run lacks all five suites or exact M328 and IME coverage")
    adapter = stock["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False and stock["guard_and_window_checks"] == "passed",
            "Stock browser does not prove actual Apple Metal and guard success")
    after = read_json(root / "inputs-after.json")
    require(after == invocation["inputs"] == read_json(pins / "inputs-before.json")
            and invocation["suite_arguments"] == list(SUITES) and invocation["injected_chrome_flags"] == []
            and after["source_manifest_sha256"] == SOURCE and after["source_files_checked_per_origin"] == 668
            and after["release"] == offline["release"] == read_json(pins / "release-pin.json")
            and after["release"] == {"version": VERSION, "last_material_step": 328, "source_sha256": SOURCE,
                                     "source_files": 668, "source_tree": TREE,
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
    require(pixels == stock["outlined_text_field"] and len(pixels["pngs"]) == 5,
            "Independent stock outline pixel replay differs")
    return {"receipt_sha256": offline_sha, "total_checks": total, "cases": cases, "adapter": adapter,
            "browser": stock["browser"], "page_errors": 0, "visible_windows": 0, "surviving_processes": 0,
            "runner_sha256": runner_sha, "captures": pixels["pngs"],
            "synthetic_composition": pixels["synthetic_composition"]}


def pixels_program(path):
    require(digest(path) == PIXELS and digest(path.parent / "verify_forms_pixels.py") == FORMS_PIXELS,
            "The stock pixel verifier or its helper changed")
    sys.path.insert(0, str(path.parent))
    try:
        return run_path(str(path))["verify"]
    finally:
        sys.path.pop(0)


def focused_stock_web(focused, stock_binary_manifest):
    exact = {PurePosixPath(name).name: checksum
             for name, checksum in manifest(stock_binary_manifest).items()
             if PurePosixPath(name).parent == STOCK_GALLERY}
    require(len(exact) == 4 and focused["web_files"] == exact,
            "R2 focused replay did not use the same exact Gallery Wasm and assets as R2 stock")


def focused_info(root, verify_pixels, runner_sha, *, check_live_inputs=False):
    invocation = read_json(root / "invocation.json")
    window = read_json(root / OBSERVATION)
    guard(window)
    argv = invocation["argv"]
    require(invocation["scope"] == "focused development pack" and invocation["runner_sha256"] == runner_sha == FOCUSED_RUNNER_SHA
            and len(argv) == 8 and argv[1:2] == ["--chrome"] and argv[3:4] == ["--web-root"]
            and argv[5:6] == ["--artifacts"] and argv[-1] == "material"
            and Path(argv[0]).parts[-3:] == FOCUSED_RUNNER.parts[-3:]
            and Path(argv[4]).parts[-4:] == (STAGE.name, "web-final", "web", "release")
            and Path(argv[6]).parts[-3:] == (STAGE.name, FOCUSED_NAME, "artifacts"),
            "The focused run was not the sealed v0.1.61 feature-only command")
    expected_web = len(invocation["web_files"]) == 4 and set(invocation["web_files"]) >= {"index.html", "surface-web.json"}
    require(expected_web and sum(name.endswith(".wasm") for name in invocation["web_files"]) == 1,
            "Focused Gallery Wasm inventory changed")
    if check_live_inputs:
        binary, assets = Path(argv[0]), Path(argv[4]) / "material-gallery"
        require(binary.resolve() == FOCUSED_RUNNER.resolve() and digest(binary) == runner_sha
                and (binary.parent / "runner.sha256").read_text().strip() == runner_sha
                and {path.name for path in assets.iterdir()} == set(invocation["web_files"]),
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
            and re.findall(r"^  \[(M\d+)\]", text, re.M) == [f"M{step}" for step in range(325, 329)]
            and re.findall(r"^case material \.\.\. ok \((\d+) steps, (\d+) checks,", text, re.M) == [("4", "178")]
            and re.findall(r"^e2e result: (.*?); artifacts in ", text, re.M) == ["1 passed; 0 failed; 178 checks"],
            "Focused v0.1.61 log differs from the four actual steps and 178 checks")
    artifacts = root / "artifacts"
    raw = read_json(artifacts / "material-outlined-text-field.json")
    feature = verify_pixels(artifacts, {"browser": raw["browser"]})
    require(len(feature["pngs"]) == 5 and type(feature["synthetic_composition"]["insert_text_needed"]) is bool,
            "Focused outline PNGs or synthetic commit fallback receipt differ")
    return {**feature, "captures": feature["pngs"], "scope": "R2 focused browser only",
            "checks": 178, "suite": "material", "visible_windows": 0, "remaining_owned_processes": 0,
            "runner_log_sha256": sha256(log).hexdigest(), "window_receipt_sha256": digest(root / OBSERVATION),
            "runner_sha256": runner_sha, "runner_reused_from": "R1 focused source-identical E2E runner",
            "web_files": invocation["web_files"]}


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
                    pixels_program(stock / "verify_outlined_field.py"))


def required_files(stage, stock, native):
    paths = [stage / name for name in ("workspace-v0161-passed", "gpu-v0161-passed",
             "native-v0161-passed", "stock-mac-v0161-passed", OFFLINE, *RUST_LOGS, SUPPLEMENTAL)]
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
    prose = f"""# Surface v0.1.61 Outlined Text Field R2 evidence

The 668-file candidate matches tree `{TREE}`. The patch applies to v0.1.60 tree `{BASE_TREE}`.
The source manifest SHA-256 is `{SOURCE}`.
R2 changes only the Outlined Text Field integration test from R1. The
R1 workspace failed its stale expectation for the assertive status
child. The unchanged R1 focused and native reports are calibration
under `calibration-r1`. The R2 counts below come from separate exact runs.

- The workspace gate passed {rust['workspace']['passed']:,} Rust tests and three Wasm checks. The GPU gate ran {rust['metal']['passed']} standard and {rust['supplemental']['passed']} supplemental offscreen tests on Apple M1 Max Metal.
- Two native headless replays each saved four PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. {native['window_samples']} samples found zero visible windows.
- Native wide and compact frames verify the 1 dp and 3 dp outlines, transparent notch, closed long-label outline and disabled text ink against both original replays.
- The five stock macOS suites passed {stock['total_checks']:,} checks through M328. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. The package includes five original stock Canvas PNGs and the typed Outlined Text Field report.
- The separate R2 focused run passed 178 checks in one case. It supplies five scoped PNGs with fresh guard receipts and the same R2 Gallery Web bytes as stock. It used the pinned R1 focused runner because all E2E source bytes match. The first trusted Enter did not submit during CDP-simulated composition. The report records whether Chrome used an explicit synthetic text commit before the separate trusted Enter submitted.
- Both stock IME cancellation reports and raw reports for Standard, Forms, Range, Vertical Centered and Inset Icon Sliders support the retained checks.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-outlined-field.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Outlined Text Field pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.61.patch.gz > /tmp/surface-v0.1.61.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.61.patch
git apply --whitespace=error-all /tmp/surface-v0.1.61.patch
shasum -a 256 -c /path/to/evidence/source.sha256
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/cargo-target/surface-outlined-field-repro
export SURFACE_GPU_EVIDENCE=/tmp/surface-outlined-field-repro/gpu-offscreen.png
sh scripts/check.sh
sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --lib gpu_offscreen_raster_cache -- --ignored --nocapture
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
```

The native harness README records the replay commands. The stock invocation records the five case arguments and before/after input pins. See [stock reproduction](stock-macos/reproduction.md) for the guarded replay. The full offline stock helper requires the original artifact inventory, runtime bundle and predecessor tools.

The evidence covers the local Mac. The native run used keyboard and programmatic input. Chrome used trusted keyboard, pointer and touch events and checked browser accessibility semantics, measured outlines, linked error descriptions, disabled and read-only editors, and dark RTL at 200 percent text. Hosted Linux, live screen-reader speech and system accessibility settings remain open.
"""
    (out / "README.md").write_text(prose)


def export(args, proof, binary_manifest):
    out = args.out
    require(not out.exists() and not out.is_symlink()
            and out.parent.resolve(strict=True).is_relative_to(Path("/tmp").resolve()),
            "Use a fresh output directory under /tmp")
    out.mkdir(mode=0o700)
    stage, stock, native = args.stage, args.stock, args.native
    copy(Path(__file__), out / "pack-outlined-field.py")
    copy(stage / "source.sha256", out / "source.sha256")
    copy(args.base / "source.sha256", out / "segmented-touch-base.sha256")
    copy(stage / "outlined-field-tree", out / "outlined-field-tree")
    copy(stage / "owned-outlined-field.paths", out / "owned-outlined-field.paths")
    zipped(stage / PATCH_NAME, out / "surface-v0.1.61.patch.gz")
    calibration = out / "calibration-r1"
    copy(R1 / "source.sha256", calibration / "r1-source.sha256")
    zipped(R1 / RUST_LOGS[0], calibration / "workspace-failed.log.gz")
    copy(R1 / "development-2" / "artifacts" / "material-outlined-text-field.json", calibration / "focused-report.json")
    copy(NATIVE_ROOT / "exact-1" / NATIVE_REPORT, calibration / "native-report.json")
    zipped(stage / "focused-r2.log", out / "rust" / "focused-r2.log.gz")
    for name in (*RUST_LOGS, SUPPLEMENTAL, *proof["rust"]["source_checks"]):
        zipped(stage / name, out / "rust" / f"{name}.gz")
    shutil.copytree(native, out / "native" / "proof")
    harness = out / "native" / "harness"
    for name in ("harness.sha256", *manifest(args.native_harness / "harness.sha256")):
        copy(args.native_harness / name, harness / name)
    copy(binary_manifest, out / "native" / "native-binaries.sha256")
    copy(stage / "native-final-runtime.log", out / "native" / "terminal.log")
    focused = out / "focused"
    original = stage / FOCUSED_NAME
    for name in FOCUSED_FILES:
        copy(original / name, focused / name)
    for path in (original / "guard-ledger").glob("*.json"):
        copy(path, focused / "guard-ledger" / path.name)
    zipped(original / "runner.log", focused / "runner.log.gz")
    for name in ("material-outlined-text-field.json", *proof["focused"]["captures"]):
        copy(original / "artifacts" / name, focused / "artifacts" / name)
    focus_binary_dir = Path(read_json(original / "invocation.json")["argv"][0]).parent
    copy(focus_binary_dir / "runner.sha256", focused / "approved-runner.sha256")
    copy(focus_binary_dir / "scope-selector.rs", focused / "scope-selector.rs")
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
    for name in ("material-outlined-text-field.json", "material-buttons.json", *REFINEMENTS_REPORTS, *proof["stock_macos"]["captures"], *RETAINED):
        copy(stock / "run" / "artifacts" / name, target / "artifacts" / name)
    write_json(out / "verified-outlined-field-v0161.json", proof)
    readme(out, proof)
    seal = manifest_package(out)
    verify_package(out, seal)
    return {"path": str(out), "manifest_sha256": seal, "files": len(manifest(out / "SHA256SUMS"))}


def verify_package(root, expected):
    verify_seal(root, "SHA256SUMS", expected, strict=True)
    report = read_json(root / "verified-outlined-field-v0161.json")
    source = report["source"]
    patch = materialize(root / "surface-v0.1.61.patch.gz")
    patch_paths = sorted({p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch, re.M)})
    prior, current = manifest(root / "segmented-touch-base.sha256"), manifest(root / "source.sha256")
    require(len(prior) == 662 and len(current) == 668
            and set(patch_paths) == set((root / "owned-outlined-field.paths").read_text().splitlines()) and len(patch_paths) == 23
            and patch_paths == sorted(name for name in prior.keys() | current.keys()
                                      if prior.get(name) != current.get(name)), "Packaged patch paths differ")

    require(source == {"version": VERSION, "source_sha256": SOURCE, "source_files": 668, "tree": TREE,
                       "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
                       "changed_paths": patch_paths}
            and digest(root / "source.sha256") == SOURCE and digest(root / "segmented-touch-base.sha256") == BASE_SOURCE
            and (root / "outlined-field-tree").read_text().strip() == TREE
            and sha256(patch).hexdigest() == PATCH,
            "Packaged source pins differ")
    calibration = root / "calibration-r1"
    require(calibration_r1(calibration / "r1-source.sha256", root / "source.sha256",
                           calibration / "workspace-failed.log.gz", calibration / "focused-report.json",
                           calibration / "native-report.json") == report["calibration_r1"]
            and scoped_r2(root / "rust" / "focused-r2.log.gz") == report["scoped_rust"],
            "Packaged R1 calibration or R2 scoped checks differ")
    rust = rust_info(*(root / "rust" / f"{name}.gz" for name in RUST_LOGS),
                     supplemental=root / "rust" / f"{SUPPLEMENTAL}.gz")
    rust["source_checks"] = check_build_source(root / "rust", root / "source.sha256")
    require(rust == report["rust"], "Packaged Rust or Metal receipt differs")
    native = root / "native"
    verify_seal(native / "harness", "harness.sha256", NATIVE_HARNESS)
    require(native_info(native / "proof", report["native"]["seal"], native / "native-binaries.sha256",
                        report["native"]["binary_manifest_sha256"])
            == report["native"], "Packaged native receipt differs")
    terminal = (native / "terminal.log").read_text().splitlines()
    require(len(terminal) == 1 and terminal[0].startswith("2 out of 2 headless replays passed at ")
            and terminal[0].endswith("/exact-2"),
            "Original native terminal receipt differs")
    stock = root / "stock-macos"
    offline = read_json(stock / OFFLINE)
    verify_seal(stock, "harness.sha256", STOCK_HARNESS)
    verify_seal(stock, "sealed-inputs.sha256", read_json(stock / "inputs-before.json")["app_bundle_seal_sha256"])
    focused = root / "focused"
    verify_pixels = pixels_program(stock / "verify_outlined_field.py")
    focused_stock_web(report["focused"], stock / "browser-binary-sha256.txt")
    focus_sha = report["focused"]["runner_sha256"]
    require((focused / "approved-runner.sha256").read_text().strip() == focus_sha
            and focused_info(focused, verify_pixels, focus_sha) == report["focused"],
            "Packaged focused v0.1.61 results differ from original artifacts")
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
            and digest(stock / "artifacts/material-segmented.json") == offline["segmented"]["report_sha256"]
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
    parser.add_argument("--native-binaries-sha")
    parser.add_argument("--focused-runner-sha")
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
        checked_source = source_info(args.stage, args.base)
        prior = check_r1_sources(args.stage)
        require((args.stage / "test-r2-passed").is_file(), "Pending R2 scoped Rust completion marker")
        scoped = scoped_r2(args.stage / "focused-r2.log")
        if args.check_preflight:
            verify_seal(args.native_harness, "harness.sha256", NATIVE_HARNESS)
            verify_seal(args.stock or STOCK, "harness.sha256", STOCK_HARNESS)
            pixels_program((args.stock or STOCK) / "verify_outlined_field.py")
            print(json.dumps({"status": "R2 preflight only", "source": checked_source,
                              "calibration_r1": prior, "scoped_rust": scoped,
                              "exact_native": "not evaluated", "exact_focused_browser": "not evaluated",
                              "full_stock": "not evaluated", "full_workspace": "not evaluated"},
                             indent=2, ensure_ascii=False, sort_keys=True))
            return
        if args.native is None:
            native_pointer = args.stage / "native-proof-dir"
            require(native_pointer.is_file(), f"Pending exact native proof: {native_pointer}")
            args.native = Path(native_pointer.read_text().strip()).resolve()
        if args.stock is None:
            stock_pointer = args.stage / "stock-observer-dir"
            require(stock_pointer.is_file(), f"Pending stock observer: {stock_pointer}")
            args.stock = Path(stock_pointer.read_text().strip()).resolve()
        required_files(args.stage, args.stock, args.native)
        require(all(HEX.fullmatch(value or "") for value in (
                    args.native_seal, args.native_binaries_sha, args.focused_runner_sha,
                    args.stock_offline_sha, args.stock_runner_sha)),
                "Supply the reviewed R2 native, binary, focused, stock runner and stock offline digests")
        require(Path((args.stage / "native-proof-dir").read_text().strip()).resolve()
                == args.native.resolve() == (NATIVE_ROOT / "exact-2").resolve()
                and Path((args.stage / "stock-observer-dir").read_text().strip()).resolve()
                == args.stock.resolve() == STOCK.resolve(), "R2 native and stock pointers differ")
        verify_seal(args.native_harness, "harness.sha256", NATIVE_HARNESS)
        binary_manifest = args.stage / "native-v0161-binaries" / "SHA256SUMS"
        verify_entries(binary_manifest.parent, manifest(binary_manifest))
        native = native_info(args.native, args.native_seal, binary_manifest, args.native_binaries_sha)
        verify_seal(args.stock, "harness.sha256", STOCK_HARNESS)
        require(digest(args.stock / "browser-source.sha256") == SOURCE
                and (args.stock / "approved-runner.sha256").read_text().strip() == args.stock_runner_sha,
                "Wrong final stock source or runner pins")
        verify_entries(args.stock / "bundle", manifest(args.stock / "browser-binary-sha256.txt"))
        focused = focused_info(args.stage / FOCUSED_NAME, pixels_program(args.stock / "verify_outlined_field.py"),
                               args.focused_runner_sha, check_live_inputs=True)
        rust = rust_info(*(args.stage / name for name in RUST_LOGS), supplemental=args.stage / SUPPLEMENTAL)
        rust["source_checks"] = check_build_source(args.stage, args.stage / "source.sha256")
        focused_stock_web(focused, args.stock / "browser-binary-sha256.txt")
        stock = verify_stock_input(args.stock, args.stage / OFFLINE, args.stock_offline_sha,
                                   args.stock_runner_sha, binary_manifest)
        result = {"status": "verified", "source": checked_source, "rust": rust, "native": native, "focused": focused,
                  "stock_macos": stock, "calibration_r1": prior, "scoped_rust": scoped}
        if args.out:
            result = {**result, "output": export(args, result, binary_manifest)}
    print(json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, FileNotFoundError, subprocess.CalledProcessError, KeyError, TypeError) as error:
        print(f"Outlined field proof stopped. {error}", file=sys.stderr)
        sys.exit(1)
