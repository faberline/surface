#!/usr/bin/env python3
"""Package exact Slider Refinements evidence without starting a host."""

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

STAGE = Path("/tmp/surface-md3-slider-refinements-v0157.RxfdP7uB")
BASE = Path("/tmp/surface-md3-range-slider-v0156.DvRoswnb")
NATIVE_ROOT = Path("/tmp/surface-md3-slider-refinements-native.SBOymsiH")
VERSION = "0.1.57"
SOURCE = "5936ee7ecfdf584a2ca3acf7fa3297db09e86132b8a5dd60bd169391a90d44c1"
TREE = "72cf21a76d284e3a299aedff91b953eae232db07"
BASE_SOURCE = "de92395ee2ab7b8d7d49760adb2722d5b44ca270c2002deb6fefbecd809e8647"
BASE_TREE = "6e7c09a6eda249eb046babf3f2a4b392b0aaa99c"
PATCH = "aee0551b61d814069ba29052af7f00c601de8f6de371247f79fdcfa475870327"
PATCH_NAME = "slider-refinements-v0157.patch"
NATIVE_HARNESS = "0e4875535706f8e481018ac1c648989106a617f944a7a1eec926d77689963ba6"
STOCK_HARNESS = "e2bfd922b0219b83404c450929eb14afcd0700708dc801bb828bb20963baa8bc"
PIXELS = "2bc808755ceb6d0affa6cbb7102abb77b9bc245e08e4d5cf549f266e55fdb3a2"
FORMS_PIXELS = "13d2a3b233e36fbbf44410c3b2b4f213a6832e4495670ed54976fce2d6e62b95"
REFINEMENTS_REPORTS = ("material-vertical-centered-slider.json", "material-slider-inset.json")
NATIVE_REPORT = "native-slider-refinements-report.json"
STOCK_REPORT = "verified-slider-refinements-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
OBSERVATION = "headless-window-observation.json"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
RUST_LOGS = ("workspace-v0157-exact.log", "gpu-v0157-exact.log")
SUPPLEMENTAL = "gpu-v0157-supplemental.log"
INTEGRITY_LOGS = tuple(
    f"{group}-r1-source-{point}.log"
    for group in ("workspace", "gpu")
    for point in ("before", "after")
)
NATIVE_IMAGES = {
    "centered-wide-light-rest.png": (960, 1040),
    "centered-wide-light-signed-focus.png": (960, 1040),
    "centered-compact-dark-rtl-200.png": (320, 880),
    "inset-wide-light-rest.png": (960, 1040),
    "inset-wide-light-relocated.png": (960, 1040),
    "inset-wide-light-minimum.png": (960, 1040),
    "inset-compact-dark-rtl-200-horizontal.png": (320, 880),
    "inset-compact-dark-rtl-200-vertical.png": (320, 880),
}
RUNNER_SHA = "90afee69933e69308bdb1f106562c5e5e41c50fe58d3e909b97398e754a6796a"
NATIVE_SEAL = "43f72374785a2eec0932b2ff6c42ef152fa5567d82214b3345ff596d8ff3c51b"
BINARY_SEAL = "579a3d7f5ca6bcd78aabd0d649508cf4c82ec51d2355a4658c9a719b26fb8df3"
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
    prior, candidate = base / "source.sha256", stage / "source-r1.sha256"
    require(digest(prior) == BASE_SOURCE and digest(candidate) == SOURCE, "Wrong base or candidate source seal")
    before, after = manifest(prior), manifest(candidate)
    for root, entries, count, tree in ((base / "source", before, 646, BASE_TREE),
                                       (stage / "source", after, 652, TREE)):
        require(len(entries) == count, "Wrong source inventory count")
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
                  if p.is_file() and p.relative_to(root).parts[0] != ".git"}
        require(actual == set(entries), f"Source file inventory differs: {root}")
        verify_entries(root, entries)
        require(source_tree(root, entries) == tree, f"Wrong exact source tree: {root}")
    root = stage / "source"
    patch = stage / PATCH_NAME
    require(digest(patch) == PATCH and (stage / "slider-refinements-tree").read_text().strip() == TREE
            and tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == VERSION,
            "Wrong candidate patch, tree marker or workspace version")
    material = (root / "e2e/src/material.rs").read_text()
    registry = re.search(r"pub const STEPS: &\[&str\] = &\[(.*?)\];", material, re.S)
    required_steps = [str(step) for step in range(1, 316) if step != 23]
    require(registry and re.findall(r'"M(\d+)"', registry.group(1)) == required_steps,
            "The stock runner must require every Material step through M315 except failure-only M23")
    paths = {p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch.read_bytes(), re.M)}
    changed = {name for name in before.keys() | after.keys() if before.get(name) != after.get(name)}
    require(paths == changed, "Patch and source inventory differ")
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
        raw = materialize(path if path.is_file() else log_root / f"{name}.gz")
        lines = raw.decode().splitlines()
        require(len(lines) == len(expected) and set(lines) == expected,
                f"Source preflight or postflight check differs: {name}")
        source_logs[name] = sha256(raw).hexdigest()
    return source_logs


def rust_info(workspace, gpu, supplemental):
    text, rust = tests(workspace)
    tail = text.rsplit("\ntest result:", 1)[-1]
    require(rust["passed"] == 1481 and rust["ignored"] == 19
            and tail.count("Finished `dev` profile") == 3
            and re.search(r"^provenance: .+ self-test cases passed\)", text, re.M)
            and "Doc-tests surface_material_gallery" in text
            and "range_slider_steps_are_required_by_the_material_case ... ok" in text
            and "vertical_centered_steps_are_required_by_the_material_case ... ok" in text
            and "compact_dark_rtl_scroll_reveals_the_active_vertical_glyph_and_large_focus_label ... ok" in text
            and "signed_discrete_drag_and_keys_use_y_and_the_same_authored_stops ... ok" in text,
            "The workspace lacks complete Slider Refinements, provenance or Wasm checks")
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


def native_shot(directory, shot, hashes):
    name = shot["name"]
    size = NATIVE_IMAGES[name]
    require(shot["sha256"] == hashes[name] and (shot["width"], shot["height"]) == size
            and shot["format"] == "png" and shot["scale"] == 1, f"Wrong native capture receipt: {name}")
    picture = open_png(directory / name, size)
    require(list(picture.getpixel((4, 4))) == shot["background"], f"Native background mismatch: {name}")
    for sample in shot["samples"].values():
        require(sample["paint"], f"Absent native painted parts: {name}")
        for pixel in sample["paint"].values():
            require(list(picture.getpixel(tuple(pixel["at"]))) == pixel["actual"], f"Wrong native track color: {name}")
        if cut := sample.get("neutral_cut_pixel"):
            require(list(picture.getpixel(tuple(cut["at"]))) == cut["rgba"] == shot["background"],
                    f"Wrong native neutral cut color: {name}")
        if caption := sample.get("caption"):
            native_ink(picture, caption)
        if icon := sample.get("icon"):
            for key in ("body", "mute_mark"):
                pixel = icon[key]
                require(list(picture.getpixel(tuple(pixel["at"]))) == pixel["pixel"],
                        f"Wrong native inset glyph pixel: {name}")
            match = tuple(icon["body"]["pixel"])
            mask = bytes(picture.getpixel(xy) == match for xy in scan_box(picture, icon["bounds"]))
            require(sum(mask) == icon["ink_pixel_count"] >= 24
                    and sha256(mask).hexdigest() == icon["mask_sha256"], f"Wrong native glyph contour: {name}")
        if label := sample.get("value_label"):
            badge, ring = label["label"], label["ring"]
            center = (floor(badge["x"] + badge["width"] / 2), floor(badge["y"] + 8))
            require(list(picture.getpixel(center)) == label["label_ink"], f"Wrong native label fill: {name}")
            ring_ink = tuple(label["ring_ink"])
            count = sum(picture.getpixel(xy) == ring_ink for xy in scan_box(picture, ring))
            require(count == label["ring_pixels"] >= 40, f"Wrong native focus ring: {name}")
            native_ink(picture, label["glyphs"])
    return name


def native_info(root, expected, binaries_manifest):
    require(expected == NATIVE_SEAL and digest(binaries_manifest) == BINARY_SEAL,
            "Wrong R1 native proof or executable manifest seal")
    entries = verify_seal(root, "SHA256SUMS", expected, strict=True)
    report = read_json(root / NATIVE_REPORT)
    binaries = manifest(binaries_manifest)
    require(set(binaries) == {"surface-material-gallery", "surface-devtools"}
            and report["binaries"] == binaries and report["native_binary_manifest_sha256"] == BINARY_SEAL,
            "Native proof used different Gallery or DevTools executables")
    require(report["schema"] == "surface.test.slider-refinements.native.v1"
            and report["source"] == {"manifest_sha256": SOURCE, "files_verified": 652, "version": VERSION}
            and report["rendering"] == "headless" and report["reduce_motion"] is True
            and report["input_scope"] == ["Action.key", "Action.setValue", "Action.activate"]
            and report["transports"] == ["Page.screenshot", "CLI", "MCP"]
            and len(report["replays"]) == 2 and report["replays"][0] == report["replays"][1],
            "Native proof scope or repeated semantic receipts differ")
    pngs = report["byte_identical_pngs"]
    require(set(pngs) == set(NATIVE_IMAGES) and len(set(pngs.values())) == len(NATIVE_IMAGES),
            "Native PNG inventory differs or reused a capture across scenes")
    expected_files = {f"replay-{i}/{name}" for i in (1, 2) for name in NATIVE_IMAGES}
    require(set(entries) == expected_files | {NATIVE_REPORT}, "Native sealed recurrence files differ")
    routes = ("vertical-centered-slider-demo", "slider-inset-demo")
    samples, reaped = 0, 0
    for index, profiles in enumerate(report["replays"], 1):
        directory = root / f"replay-{index}"
        require(set(profiles) == {f"{route}-{width}" for route in routes for width in ("wide", "compact")},
                "Missing native replay profile")
        shots = []
        for route in routes:
            for mode, size, rtl, scale in (("wide", [960, 1040], False, 1), ("compact", [320, 880], True, 2)):
                profile = profiles[f"{route}-{mode}"]
                require(profile["viewport"] == size and profile["rtl"] is rtl and profile["text_scale"] == scale
                        and profile["gallery_reaped"] is True and profile["visible_windows"] == [0] * 4
                        and profile["disabled_skipped_by_tab"] is True and profile["disabled_value_rejected"] is True
                        and profile["closed_focus"] == f"{route}-show", "Native profile or guard differs")
                samples += len(profile["visible_windows"])
                reaped += 1
                for test in profile["tests"].values():
                    shots.extend(native_shot(directory, shot, pngs) for key, shot in test.items()
                                 if key == "png" or key.endswith("_png"))
        require(sorted(shots) == sorted(NATIVE_IMAGES)
                and all(entries[f"replay-{index}/{name}"] == sha for name, sha in pngs.items()),
                "Native captures do not match their two sets of original bytes")
    require(samples == report["window_samples"] == 32 and reaped == report["owned_processes_reaped"] == 8,
            "Native window sampling or process reaping differs")
    return {"seal": expected, "report_sha256": digest(root / NATIVE_REPORT), "runs": 2,
            "visible_windows": 0, "window_samples": samples, "owned_processes_reaped": reaped,
            "transports": report["transports"], "binaries": binaries, "captures": pngs}


def guard(window):
    require(window["stock_exit"] == 0 and window["visible_owned_windows"] == []
            and window["remaining_observed_processes"] == [] and window["problems"] == [],
            "Chrome guard saw a visible window, surviving process or failure")


def web_info(root, pins, offline_file, offline_sha, verify_pixels):
    require(HEX.fullmatch(offline_sha or "") and digest(offline_file) == offline_sha, "Wrong stock offline receipt seal")
    offline = read_json(offline_file)
    stock = read_json(root / STOCK_REPORT)
    window = read_json(root / OBSERVATION)
    guard(window)
    launch = window["guard_events"]
    ledger = [read_json(file) for file in (root / "guard-ledger").glob("*.json")]
    require(len(launch) == 3 and {event["role"] for event in launch} == {"chrome-primary", "chrome-bridge", "native"}
            and sorted(launch, key=lambda event: event["pid"]) == sorted(ledger, key=lambda event: event["pid"])
            and all(event["allowed"] is True for event in launch)
            and len({event["profile"] for event in launch if event["profile"]}) == 2,
            "Exact stock process ledger differs")
    observations = window["launch_observations"]
    require(set(observations) == {"chrome-primary", "chrome-bridge", "native"}
            and all(group["samples"] > 0 and len(group["pids"]) == 1 for group in observations.values())
            and observations["chrome-bridge"]["detached_samples"] > 0, "A guarded child was not observed live")
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
            and steps == [step for step in range(1, 316) if step != 23] and ime == list(range(1, 21))
            and re.findall(r"^e2e result: (.*?); artifacts in ", text, re.M) == [f"5 passed; 0 failed; {total} checks"]
            and cases == stock["cases"] and stock["total_checks"] == total and cases["material"]["steps"] == 314
            and cases["ime"] == {"steps": 20, "checks": 77} and stock["page_errors"] == 0,
            "Stock run lacks all five suites or exact M315 and IME coverage")
    adapter = stock["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False and stock["guard_and_window_checks"] == "passed",
            "Stock browser does not prove actual Apple Metal and guard success")
    after = read_json(root / "inputs-after.json")
    invocation = read_json(root / "invocation.json")
    require(after == invocation["inputs"] == read_json(pins / "inputs-before.json")
            and invocation["suite_arguments"] == list(SUITES) and invocation["injected_chrome_flags"] == []
            and after["source_manifest_sha256"] == SOURCE and after["source_files_checked_per_origin"] == 652
            and after["release"] == offline["release"] == read_json(pins / "release-pin.json")
            and after["release"] == {"version": VERSION, "last_material_step": 315, "source_sha256": SOURCE,
                                     "source_files": 652, "source_tree": TREE,
                                     "wide_canvas_height": 800, "rtl_canvas_height": 1020}
            and digest(pins / "browser-source.sha256") == SOURCE
            and after["app_bundle_seal_sha256"] == digest(pins / "sealed-inputs.sha256")
            and after["stock_files"] == manifest(pins / "browser-binary-sha256.txt")
            and after["bundle_files_checked"] == len(after["stock_files"]) == 16
            and after["runner_sha256"] == (pins / "approved-runner.sha256").read_text().strip()
            == after["stock_files"]["debug/surface-e2e"] == RUNNER_SHA,
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
    require(pixels == stock["slider_refinements"] and len(pixels["captures"]) == 7,
            "Independent stock pixel replay differs")
    return {"receipt_sha256": offline_sha, "total_checks": total, "cases": cases, "adapter": adapter,
            "browser": stock["browser"], "page_errors": 0, "visible_windows": 0, "surviving_processes": 0,
            "runner_sha256": RUNNER_SHA, "captures": pixels["captures"]}


def pixels_program(path):
    require(digest(path) == PIXELS and digest(path.parent / "verify_forms_pixels.py") == FORMS_PIXELS,
            "The stock pixel verifier or its helper changed")
    sys.path.insert(0, str(path.parent))
    try:
        return run_path(str(path))["verify"]
    finally:
        sys.path.pop(0)


def verify_stock_input(stock, offline, offline_sha, binary_manifest):
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
    return web_info(stock / "run", stock, offline, offline_sha, pixels_program(stock / "verify_refinement_pixels.py"))


def required_files(stage, stock, native):
    paths = [stage / name for name in ("workspace-v0157-passed", "gpu-v0157-passed",
             "native-v0157-passed", "stock-mac-v0157-passed", OFFLINE, *RUST_LOGS, SUPPLEMENTAL, *INTEGRITY_LOGS)]
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
    prose = f"""# Surface v0.1.57 Slider Refinements evidence

The 652-file R1 candidate matches tree `{TREE}`. The patch applies to v0.1.56 tree `{BASE_TREE}`.
The source manifest SHA-256 is `{SOURCE}`.

- The workspace gate passed {rust['workspace']['passed']:,} Rust tests and three Wasm checks. The GPU gate ran {rust['metal']['passed']} standard and {rust['supplemental']['passed']} supplemental offscreen tests on Apple M1 Max Metal.
- Two native headless replays each saved eight PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. {native['window_samples']} samples found zero visible windows.
- The five stock macOS suites passed {stock['total_checks']:,} checks through M315. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. The package includes seven original Canvas PNGs and the two typed .57 reports.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-slider-refinements.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Slider Refinements pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.57.patch.gz > /tmp/surface-v0.1.57.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.57.patch
git apply --whitespace=error-all /tmp/surface-v0.1.57.patch
shasum -a 256 -c /path/to/evidence/source.sha256
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/cargo-target/surface-slider-refinements-repro
export SURFACE_GPU_EVIDENCE=/tmp/surface-slider-refinements-repro/gpu-offscreen.png
sh scripts/check.sh
sh scripts/check-gpu.sh
cap cargo test --locked -p surface-wgpu --lib gpu_offscreen_raster_cache -- --ignored --nocapture
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture
```

The native harness README records the replay commands. The stock invocation records the five case arguments and before/after input pins. See [stock reproduction](stock-macos/reproduction.md) for the guarded replay. The full offline stock helper requires the original artifact inventory, runtime bundle and predecessor tools.

The evidence covers the local Mac. The native run used keyboard and programmatic input. Chrome used trusted keyboard, pointer and touch events and checked browser accessibility semantics. Hosted Linux, live screen-reader speech and system accessibility settings remain open.
"""
    (out / "README.md").write_text(prose)


def export(args, proof, binary_manifest):
    out = args.out
    require(not out.exists() and not out.is_symlink()
            and out.parent.resolve(strict=True).is_relative_to(Path("/tmp").resolve()),
            "Use a fresh output directory under /tmp")
    out.mkdir(mode=0o700)
    stage, stock, native = args.stage, args.stock, args.native
    copy(Path(__file__), out / "pack-slider-refinements.py")
    copy(stage / "source-r1.sha256", out / "source.sha256")
    copy(args.base / "source.sha256", out / "range-slider-base.sha256")
    copy(stage / "slider-refinements-tree", out / "slider-refinements-tree")
    zipped(stage / PATCH_NAME, out / "surface-v0.1.57.patch.gz")
    for name in (*RUST_LOGS, SUPPLEMENTAL, *INTEGRITY_LOGS):
        zipped(stage / name, out / "rust" / f"{name}.gz")
    shutil.copytree(native, out / "native" / "proof")
    harness = out / "native" / "harness"
    for name in ("harness.sha256", *manifest(args.native_harness / "harness.sha256")):
        copy(args.native_harness / name, harness / name)
    copy(binary_manifest, out / "native" / "native-binaries.sha256")
    copy(stage / "native-r1-pins.json", out / "native" / "input-pins.json")
    copy(stage / "native-r1-runtime.log", out / "native" / "terminal.log")
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
    for name in (*REFINEMENTS_REPORTS, *proof["stock_macos"]["captures"], *RETAINED):
        copy(stock / "run" / "artifacts" / name, target / "artifacts" / name)
    write_json(out / "verified-slider-refinements-v0157.json", proof)
    readme(out, proof)
    seal = manifest_package(out)
    verify_package(out, seal)
    return {"path": str(out), "manifest_sha256": seal, "files": len(manifest(out / "SHA256SUMS"))}


def verify_package(root, expected):
    verify_seal(root, "SHA256SUMS", expected, strict=True)
    report = read_json(root / "verified-slider-refinements-v0157.json")
    source = report["source"]
    patch = materialize(root / "surface-v0.1.57.patch.gz")
    patch_paths = sorted({p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch, re.M)})
    prior, current = manifest(root / "range-slider-base.sha256"), manifest(root / "source.sha256")
    require(len(prior) == 646 and len(current) == 652
            and patch_paths == sorted(name for name in prior.keys() | current.keys()
                                      if prior.get(name) != current.get(name)), "Packaged patch paths differ")

    require(source == {"version": VERSION, "source_sha256": SOURCE, "source_files": 652, "tree": TREE,
                       "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
                       "changed_paths": patch_paths}
            and digest(root / "source.sha256") == SOURCE and digest(root / "range-slider-base.sha256") == BASE_SOURCE
            and (root / "slider-refinements-tree").read_text().strip() == TREE
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
    pins = read_json(native / "input-pins.json")
    terminal = read_json(native / "terminal.log")
    require(pins["status"] == "pins-verified" and pins["binaries"] == report["native"]["binaries"]
            and pins["native_binary_manifest_sha256"] == BINARY_SEAL
            and pins["source"] == {"manifest_sha256": SOURCE, "files_verified": 652, "version": VERSION}
            and terminal["manifest_sha256"] == report["native"]["seal"]
            and terminal["window_samples"] == 32 and terminal["images_per_replay"] == 8
            and terminal["visible_windows"] == 0 and terminal["pngs"] == report["native"]["captures"],
            "Original native preflight or terminal receipt differs")
    stock = root / "stock-macos"
    offline = read_json(stock / OFFLINE)
    verify_seal(stock, "harness.sha256", STOCK_HARNESS)
    verify_seal(stock, "sealed-inputs.sha256", read_json(stock / "inputs-before.json")["app_bundle_seal_sha256"])
    require(web_info(stock, stock, stock / OFFLINE, report["stock_macos"]["receipt_sha256"],
                     pixels_program(stock / "verify_refinement_pixels.py")) == report["stock_macos"],
            "Packaged stock receipt differs")
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
        require(HEX.fullmatch(args.native_seal or "") and HEX.fullmatch(args.stock_offline_sha or ""),
                "Supply the reviewed native manifest and offline stock receipt digests")
        require(Path((args.stage / "native-proof-dir").read_text().strip()).resolve() == args.native.resolve()
                and Path((args.stage / "stock-observer-dir").read_text().strip()).resolve() == args.stock.resolve(),
                "Stage native and stock pointers differ")
        verify_seal(args.native_harness, "harness.sha256", NATIVE_HARNESS)
        binary_manifest = args.stage / "native-v0157-r1-binaries" / "SHA256SUMS"
        verify_entries(binary_manifest.parent, manifest(binary_manifest))
        native = native_info(args.native, args.native_seal, binary_manifest)
        rust = rust_info(*(args.stage / name for name in RUST_LOGS), supplemental=args.stage / SUPPLEMENTAL)
        rust["source_checks"] = check_build_source(args.stage, args.stage / "source-r1.sha256")
        stock = verify_stock_input(args.stock, args.stage / OFFLINE, args.stock_offline_sha, binary_manifest)
        result = {"status": "verified", "source": checked_source, "rust": rust, "native": native,
                  "stock_macos": stock}
        if args.out:
            result = {**result, "output": export(args, result, binary_manifest)}
    print(json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, FileNotFoundError, subprocess.CalledProcessError, KeyError, TypeError) as error:
        print(f"Slider Refinements proof stopped. {error}", file=sys.stderr)
        sys.exit(1)
