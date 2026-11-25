#!/usr/bin/env python3
"""Export and recheck exact Filled Text Field proof without launching hosts."""

import argparse
import gzip
from hashlib import file_digest, sha1, sha256
import json
from math import ceil, floor
import os
from pathlib import Path, PurePosixPath
import re
from contextlib import contextmanager
from runpy import run_path
import shutil
import subprocess
import sys
import tomllib

from PIL import Image

STAGE = Path("/tmp/surface-md3-filled-field-v0162.GlMkNcej")
BASE = Path("/tmp/surface-md3-outlined-field-tests-r2.j9B9xWVq")
STOCK = Path("/tmp/surface-md3-filled-field-stock-v0162.X1DV91RH")
NATIVE = Path("/tmp/surface-md3-filled-native.eY7wWYUD")
VERSION = "0.1.62"
SOURCE = "78ae815e1c6407c91c65b13219e07b2c6f2298eb5218530670fad26f70a3dabe"
TREE = "7266ce0588d5bc2d83a4151ca2e13c780153e279"
BASE_SOURCE = "3c79ae5ee15bf3b9b786d37452f1e0e5929ac0d31276ca7f6506897462dd9cc8"
BASE_TREE = "7c7881f78c3dc44129f52688a079b6874c9a4991"
PATCH = "e7382f4038b3b97752127232dc518ec860ae3b49cc17d9d1b76e00da6187947b"
PATCH_NAME = "filled-v0162.patch"
STOCK_HARNESS = "2aa51779fc292ce34766a377fdbbf1b6faa88cdc6963cecfc514965d9ffdd569"
NATIVE_HELPERS = "1e65797c8f3f03976191d19a02b54de9fe58904ee958b02a1bd13175c5724fbe"
NATIVE_REPLAY = "842e5f4791c89e8da1c3b932dd169eb4ef874502d88587b6993d522495024bc1"
NATIVE_SEAL = "ba2f3c56c77623cc570731a3f1b2e91c20696add11fdb0403cbcf87dfe8ee723"
FOCUS_HARNESS = "19cbe74e05985136fa1a077f910f9e06bae6f59a4a606d0dff230a65e48ead0c"
FOCUS_DIRECTORY = "development-2"
FOCUS_RUNNER = "f0891525736b3d054bfc16426714a97933fc4e673a2c7e31a4f8c1bc003776a5"
STOCK_REPORT = "verified-filled-field-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
OBSERVATION = "headless-window-observation.json"
NATIVE_REPORT = "native-filled-report.json"
FILLED_REPORT = "material-filled-text-field.json"
OUTLINE_REPORT = "material-outlined-text-field.json"
FINAL_REPORT = "verified-filled-field-v0162.json"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
RUST_LOGS = ("workspace-v0162-exact.log", "gpu-v0162-exact.log", "gpu-v0162-supplemental.log")
NATIVE_IMAGES = {"wide-idle.png": (960, 1080), "wide-focused.png": (960, 1080),
                 "compact-error.png": (320, 1080), "compact-long-label.png": (320, 1080)}
STOCK_PINS = ("browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256",
              "release-pin.json", "source-origins.json", "build-markers.json", "sealed-inputs.sha256",
              "inputs-before.json", "approved-runner.sha256")
RETAINED = ("browser-ime.json", "browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json",
            "browser-smoke.json", "browser-material-side-sheet.json", "bridge-web-result.json",
            "devtools-headless-result.json", "material-standard-slider.json", "material-slider-forms.json",
            "material-range-slider.json", "material-segmented.json", "material-buttons.json",
            "material-vertical-centered-slider.json", "material-slider-inset.json")
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


def source_info(stage, base):
    prior, candidate = base / "source.sha256", stage / "source.sha256"
    require(digest(prior) == BASE_SOURCE and digest(candidate) == SOURCE, "Wrong base or candidate source seal")
    before, after = manifest(prior), manifest(candidate)
    for root, entries, count, tree in ((base / "source", before, 668, BASE_TREE),
                                       (stage / "source", after, 670, TREE)):
        require(len(entries) == count, "Wrong source inventory count")
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
                  if p.is_file() and p.relative_to(root).parts[0] != ".git"}
        require(actual == set(entries), f"Source file inventory differs: {root}")
        verify_entries(root, entries)
        require(source_tree(root, entries) == tree, f"Wrong exact source tree: {root}")
    root = stage / "source"
    patch = stage / PATCH_NAME
    require(digest(patch) == PATCH and (stage / "source-tree").read_text().strip() == TREE
            and tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == VERSION,
            "Wrong candidate patch, tree marker or workspace version")
    material = (root / "e2e/src/material.rs").read_text()
    registry = re.search(r"pub const STEPS: &\[&str\] = &\[(.*?)\];", material, re.S)
    required_steps = [str(step) for step in range(1, 333) if step != 23]
    require(registry and re.findall(r'"M(\d+)"', registry.group(1)) == required_steps,
            "The stock runner must require every Material step through M332 except failure-only M23")
    paths = {p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch.read_bytes(), re.M)}
    changed = {name for name in before.keys() | after.keys() if before.get(name) != after.get(name)}
    require(paths == changed and len(paths) == 15,
            "Patch, source inventory and the 15 reviewed paths differ")
    for directory, options in ((base / "source", []), (root, ["--reverse"])):
        subprocess.run(["git", "-C", str(directory), "apply", "--check", "--whitespace=error-all",
                        *options, str(patch)], check=True, env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"})
    return {"version": VERSION, "source_sha256": SOURCE, "source_files": len(after), "tree": TREE,
            "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
            "changed_paths": sorted(paths)}


def require_paths(root, names):
    missing = [str(root / name) for name in names if not (root / name).is_file()]
    require(not missing, "Await exact evidence:\n" + "\n".join(missing))


@contextmanager
def helper_paths(root):
    sys.path.insert(0, str(root))
    try:
        yield
    finally:
        sys.path.pop(0)


def native_pixels(directory, shot, mode):
    name = shot["name"]
    width, height = NATIVE_IMAGES[name]
    path = directory / name
    require((shot["width"], shot["height"], shot["scale"], shot["format"]) == (width, height, 1, "png")
            and digest(path) == shot["sha256"], f"Changed native image: {name}")
    picture = open_png(path, (width, height))
    proof = shot["indicator"]
    face, label = proof["face"], proof["label"]
    x, y, w, h = (face[key] for key in ("x", "y", "width", "height"))
    rtl = mode == "compact"
    long = name == "compact-long-label.png"
    require(h == (96 if rtl else 56) and 0 <= x < x + w <= width and 0 <= y < y + h <= height,
            f"Wrong exact native face bounds: {name}")
    if long:
        require(label["x"] == x + 16 and label["width"] == w - 32
                and label["height"] >= 64 and label["y"] + label["height"] + 4 == y,
                "The long native caption did not wrap fully above the Filled field")
    else:
        inset = x + w - label["x"] - label["width"] if rtl else label["x"] - x
        require(inset == 16 and label["y"] == y + 8 and label["height"] == (32 if rtl else 16),
                "The native caption does not fit the upper text lane")

    def pixel(px, py):
        return list(picture.getpixel((floor(px), floor(py))))

    band = [pixel(x + w / 2, y + h - depth - 0.5) for depth in range(5)]
    fill, paper = pixel(x + 6, y + 0.5), pixel(x - 2, y + 6)
    thick = 1 if name == "wide-idle.png" else 3
    require(band == proof["band"] == [band[0]] * thick + [fill] * (5 - thick)
            and fill == proof["fill"] and paper == proof["background"]
            and len({tuple(fill), tuple(paper), tuple(band[0])}) == 3, f"Native bottom indicator differs: {name}")
    corners = [pixel(px, py) for py in (y + 0.5, y + h - 0.5) for px in (x + 0.5, x + w - 0.5)]
    require(corners == proof["corners"] == [paper, paper, band[0], band[0]],
            f"Native rounded top or square bottom pixels differ: {name}")
    if not long:
        disabled = shot["disabled_value"]
        box = disabled["bounds"]
        paper = pixel(box["x"] - 10, box["y"] + box["height"] / 2)
        pixels = [px for px, py in scan_box(picture, box)
                  if max(abs(a - b) for a, b in zip(pixel(px, py)[:3], paper[:3], strict=True)) > 12]
        require(len(pixels) == disabled["ink_pixels"] > 40 and [min(pixels), max(pixels)] == disabled["ink_x"],
                f"Native disabled text pixel count differs: {name}")
        inset = box["x"] + box["width"] - max(pixels) if rtl else min(pixels) - box["x"]
        require(inset < 12, "Native disabled value no longer aligns at logical start")
    return name, shot["sha256"]


def native_info(proof, binary_manifest, terminal, harness):
    verify_seal(harness, "helpers.sha256", NATIVE_HELPERS)
    require(digest(harness / "replay-filled.py") == NATIVE_REPLAY, "Changed native replay harness")
    entries = verify_seal(proof, "SHA256SUMS", NATIVE_SEAL, strict=True)
    report = read_json(proof / NATIVE_REPORT)
    binaries = manifest(binary_manifest)
    require(set(binaries) == {"surface-material-gallery", "surface-devtools"} and binaries == report["binaries"],
            "The native proof used different Gallery or DevTools executable bytes")
    require(report["source"] == {"manifest_sha256": SOURCE, "files": 670}
            and report["transports"] == ["Page.screenshot", "CLI", "MCP"]
            and len(report["replays"]) == len(report["processes"]) == 2
            and report["replays"][0] == report["replays"][1], "Wrong exact native inputs or replay parity")
    require(set(entries) == {f"replay-{i}/{name}" for i in (1, 2) for name in NATIVE_IMAGES} | {NATIVE_REPORT},
            "Wrong native original inventory")
    processes = []
    for index, (profiles, children) in enumerate(zip(report["replays"], report["processes"], strict=True), 1):
        require(set(profiles) == set(children) == {"wide", "compact"}, "Missing native profile")
        seen = {}
        for mode, profile in profiles.items():
            require(profile["viewport"] == [960 if mode == "wide" else 320, 1080] and len(profile["captures"]) == 2,
                    "Wrong native canvas geometry")
            for shot in profile["captures"]:
                name, checksum = native_pixels(proof / f"replay-{index}", shot, mode)
                require(name not in seen and entries[f"replay-{index}/{name}"] == checksum, "Native image inventory changed")
                seen[name] = checksum
            child = children[mode]
            require(child["pid"] > 0 and child["reaped"] is True and child["visible_windows"] == [0] * 4,
                    "A native process remained or had a visible window")
            processes.append(child["pid"])
        require(set(seen) == set(NATIVE_IMAGES), "Missing native images")
    require(len(set(processes)) == 4 and terminal.read_text().strip().endswith("/exact-1")
            and terminal.read_text().startswith("2 out of 2 headless replays passed at "), "Wrong native terminal proof")
    return {"runs": 2, "binaries": binaries, "binary_manifest_sha256": digest(binary_manifest),
            "transports": report["transports"], "visible_windows": 0, "window_samples": 16,
            "owned_processes_reaped": 4, "captures_each": seen, "seal": NATIVE_SEAL,
            "report_sha256": digest(proof / NATIVE_REPORT)}


def toolchain(workspace, metal, supplemental):
    text, rust = tests(workspace)
    tail = text.rsplit("\ntest result:", 1)[-1]
    named = ("filled_text_field_steps_are_required_by_the_material_case",
             "outlined_text_field_steps_are_required_by_the_material_case",
             "focus_repaints_one_semantic_editor_and_cuts_a_transparent_three_dp_outline",
             "top_inset_keeps_the_whole_editor_target_and_aligns_all_text_geometry",
             "top_inset_preserves_one_measured_line_and_leaves_multiline_unchanged",
             "range_slider_steps_are_required_by_the_material_case")
    require(rust["passed"] >= 1530 and rust["ignored"] == 19
            and tail.count("Finished `dev` profile") == 3 and "Doc-tests surface_material_gallery" in text
            and re.search(r"^provenance: .+ self-test cases passed\)", text, re.M)
            and all(f"{name} ... ok" in text for name in named), "Workspace omitted an actual exact Rust or Wasm gate")
    text, gpu = tests(metal)
    require(gpu["passed"] == 16 and gpu["ignored"] == 0 and 'name: "Apple M1 Max"' in text
            and "backend: Metal" in text, "Expected 16 actual Apple Metal GPU tests")
    text, extra = tests(supplemental)
    measured = ("raster::tests::gpu_offscreen_raster_cache_reuses_shared_pixels_and_evicts_by_count_and_bytes",
                "paired_selection_matches_single_draw_references_at_fractional_clip_edges")
    rows = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; 0 ignored;", text, re.M)
    require(rows == ["1", "1"] and extra["passed"] == 2 and extra["ignored"] == 0
            and len(re.findall(r'^GPU evidence: AdapterInfo \{ name: "Apple M1 Max", .* backend: Metal \}$', text, re.M)) == 2
            and all(f"test {name} ... ok" in text for name in measured),
            "Expected both actual supplemental Apple Metal GPU tests")
    return {"workspace": {**rust, "wasm_checks": 3}, "metal": {**gpu, "backend": "Metal"},
            "supplemental": {**extra, "backend": "Metal", "tests": list(measured)},
            "metal_total_passed": gpu["passed"] + extra["passed"]}


def stock_tools(root):
    verify_seal(root, "harness.sha256", STOCK_HARNESS)
    with helper_paths(root):
        observer = run_path(str(root / "observe-filled-field.py"))
        filled = run_path(str(root / "verify_filled_field.py"))["verify"]
        outlined = run_path(str(root / "verify_outlined_field.py"))["verify"]
    return observer, filled, outlined


def focused_info(root, verifier, live=False):
    invocation = read_json(root / "invocation.json")
    argv = invocation["argv"]
    require(invocation["scope"] == "focused development pack" and argv[-1] == "material"
            and argv[1:7:2] == ["--chrome", "--web-root", "--artifacts"]
            and len(argv) == 8 and Path(argv[4]).parts[-4:] in (
                (STAGE.name, "web-canonical", "web", "release"), (STOCK.name, "bundle", "web", "release"))
            and Path(argv[6]).parts[-3:] == (STAGE.name, FOCUS_DIRECTORY, "artifacts")
            and invocation["runner_sha256"] == FOCUS_RUNNER, "Wrong Filled focused development source or scope")
    if live:
        runner, web = Path(argv[0]), Path(argv[4]) / "material-gallery"
        require(digest(runner) == FOCUS_RUNNER and runner.parent.name == "focused-bin"
                and set(p.name for p in web.iterdir()) == set(invocation["web_files"]), "Development inputs changed")
        verify_entries(web, invocation["web_files"])
    require(len(invocation["web_files"]) == 4 and {"index.html", "surface-web.json"} <= invocation["web_files"].keys(),
            "Incomplete focused development Web inventory")
    window = read_json(root / OBSERVATION)
    guard(window)
    event, = window["guard_events"]
    ledger, = [read_json(path) for path in (root / "guard-ledger").glob("*.json")]
    active = window["launch_observations"]
    logged = event["argv"]
    profile = PurePosixPath(event["profile"])
    require(ledger == event and event["role"] == "chrome-primary" and event["allowed"] is True
            and [v for v in logged if v == "--headless" or v.startswith("--headless=")] == ["--headless=new"]
            and [v for v in logged if v.startswith("--user-data-dir=")] == [f"--user-data-dir={profile}"]
            and profile.is_relative_to(PurePosixPath(argv[6]).parent / "tmp")
            and active["chrome-primary"]["samples"] > 0 and active["chrome-primary"]["pids"] == [event["pid"]]
            and active["native"]["samples"] == active["chrome-bridge"]["samples"] == 0,
            "Invalid development Chrome guard or samples")
    log_path = root / "runner.log"
    log = materialize(log_path if log_path.is_file() else root / "runner.log.gz")
    text = log.decode()
    require(re.findall(r"^case (\S+) \.\.\.$", text, re.M) == ["material"]
            and re.findall(r"^  \[(M\d+)\]", text, re.M) == [f"M{i}" for i in range(329, 333)]
            and re.findall(r"^case material \.\.\. ok \((\d+) steps, (\d+) checks,", text, re.M) == [("4", "235")]
            and re.findall(r"^e2e result: (.*?); artifacts in ", text, re.M) == ["1 passed; 0 failed; 235 checks"],
            "Wrong development run steps or count")
    browser, = re.findall(r"^browser: (Chrome/\S+) ", text, re.M)
    evidence = verifier(root / "artifacts", {"browser": browser})
    return {"scope": "focused exact Gallery only", "checks": 235, "steps": 4, "visible_windows": 0,
            "remaining_owned_processes": 0, "runner_sha256": FOCUS_RUNNER,
            "runner_log_sha256": sha256(log).hexdigest(), "window_receipt_sha256": digest(root / OBSERVATION),
            "web_files": invocation["web_files"], "filled_text_field": evidence}


def match_focused_stock(focused, binary_manifest):
    gallery = PurePosixPath("web/release/material-gallery")
    assets = {PurePosixPath(name).name: checksum for name, checksum in manifest(binary_manifest).items()
              if PurePosixPath(name).parent == gallery}
    require(len(assets) == 4 and focused["web_files"] == assets,
            "Focused Gallery Web assets differ from exact canonical stock")


def stock_info(run, pins, offline, observer, filled, outlined):
    stock, saved = read_json(run / STOCK_REPORT), read_json(offline)
    window, invocation = read_json(run / OBSERVATION), read_json(run / "invocation.json")
    guard(window)
    check_launches(run, window, invocation)
    for key, value in stock.items():
        require(saved[key] == value, f"Archived offline verdict changed at {key}")
    log_path = run / "runner.log"
    log = materialize(log_path if log_path.is_file() else run / "runner.log.gz")
    counts, checks = observer["verify_runner_log"](log.decode(), 332)
    require(digest(run / OBSERVATION) == saved["window_receipt_sha256"]
            and sha256(log).hexdigest() == saved["runner_log_sha256"] and counts == stock["cases"]
            and counts["material"]["steps"] == 331 and checks == stock["total_checks"] and stock["page_errors"] == 0,
            "Missing exact Material steps, original log, or successful window receipt")
    release = {"version": VERSION, "source_sha256": SOURCE, "source_files": 670, "source_tree": TREE,
               "last_material_step": 332, "wide_canvas_height": 800, "rtl_canvas_height": 1020}
    after = read_json(run / "inputs-after.json")
    require(after == invocation["inputs"] == read_json(pins / "inputs-before.json")
            and tuple(invocation["suite_arguments"]) == SUITES and invocation["injected_chrome_flags"] == []
            and after["release"] == saved["release"] == read_json(pins / "release-pin.json") == release
            and after["source_manifest_sha256"] == digest(pins / "browser-source.sha256") == SOURCE
            and after["source_files_checked_per_origin"] == 670
            and after["app_bundle_seal_sha256"] == digest(pins / "sealed-inputs.sha256")
            and after["stock_files"] == manifest(pins / "browser-binary-sha256.txt")
            and after["bundle_files_checked"] == len(after["stock_files"]) == 16
            and after["runner_sha256"] == (pins / "approved-runner.sha256").read_text().strip()
            == after["stock_files"]["debug/surface-e2e"], "Exact stock build or original invocation differs")
    verify_seal(pins, "sealed-inputs.sha256", after["app_bundle_seal_sha256"])
    adapter = stock["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False and stock["guard_and_window_checks"] == "passed",
            "Actual stock WebGPU was not Apple Metal or the guard failed")
    artifacts = run / "artifacts"
    smoke, ime, side, bridge, native = (read_json(artifacts / name) for name in (
        "browser-smoke.json", "browser-ime.json", "browser-material-side-sheet.json",
        "bridge-web-result.json", "devtools-headless-result.json"))
    require(smoke["errors"] == ime["errors"] == side["errors"] == []
            and smoke["browser"] == side["browser"] == stock["browser"]
            and side["adapter"] == {key: smoke["adapter"][key] for key in adapter} == adapter
            and ime["committed"] == "繁體中文、简体中文"
            and bridge["webgpu_ready"] is True and bridge["owned_browser_exited"] is True
            and (native["width"], native["height"], native["scale"]) == (720, 640, 1)
            and native["action_rendered"] is True and native["screenshot_transports_match"] is True,
            "Raw stock Smoke, IME, Bridge or headless Gallery evidence differs")
    select_all = read_json(artifacts / "browser-ime-cancel-redraw.json")
    selected = read_json(artifacts / "browser-ime-cancel-selected-redraw.json")
    keys_before, keys_after = select_all["before"], select_all["after"]
    require(keys_before["browser"] == keys_after["browser"] == [0, 2]
            and keys_before["trustedSelectAll"] is True and keys_after["frames"] > keys_before["frames"]
            and select_all["replacement"] == ["中👨‍👩‍👧‍👦é", "中👨‍👩‍👧‍👦é", "31", "31", "false"]
            and selected["canceled"]["direction"] == "backward" and selected["moved"]["trustedRight"] is True
            and selected["unpainted"]["value"] == selected["repainted"]["value"] == "台文X"
            and selected["unpainted"]["frames"] == selected["canceled"]["frames"] < selected["repainted"]["frames"],
            "Retained I11 copy lost the real selection and redraw evidence")
    metadata = {"browser": stock["browser"], "adapter": adapter}
    newest, predecessor = filled(artifacts, metadata), outlined(artifacts, metadata)
    require(newest == stock["filled_text_field"] and predecessor == stock["outlined_text_field"],
            "Independent WebGPU editor pixel replay differs from stock")
    for name, key in (("material-buttons.json", "common_buttons"), ("material-segmented.json", "segmented")):
        require(digest(artifacts / name) == stock[key]["report_sha256"], f"Retained {name} differs")
    return {**metadata, "total_checks": checks, "cases": counts, "page_errors": 0, "visible_windows": 0,
            "surviving_processes": 0, "runner_sha256": after["runner_sha256"],
            "offline_receipt_sha256": digest(offline), "filled_text_field": newest,
            "outlined_text_field": predecessor, "retained_reports": {name: digest(artifacts / name) for name in RETAINED}}


def copy_sealed(root, target, name):
    for item in (name, *manifest(root / name)):
        copy(root / item, target / item)


def export(out, receipts, stock, offline, native, binary_manifest):
    require(out.parent.resolve().is_relative_to(Path("/tmp").resolve()), "Use a new output under /tmp")
    out.mkdir(mode=0o700)
    copy(STAGE / "source.sha256", out / "source.sha256")
    copy(BASE / "source.sha256", out / "base-source.sha256")
    zipped(STAGE / PATCH_NAME, out / "filled-v0162.patch.gz")
    copy(Path(__file__), out / "pack-filled-field.py")
    for name in RUST_LOGS:
        zipped(STAGE / name, out / "rust" / f"{name}.gz")

    capture = out / "native"
    shutil.copytree(native, capture / "proof")
    copy(binary_manifest, capture / "native-binaries.sha256")
    copy(STAGE / "native-final-runtime.log", capture / "terminal.log")
    copy_sealed(NATIVE, capture / "harness", "helpers.sha256")
    copy(NATIVE / "replay-filled.py", capture / "harness" / "replay-filled.py")

    focused = STAGE / FOCUS_DIRECTORY
    target = out / "focused"
    for name in ("invocation.json", OBSERVATION):
        copy(focused / name, target / name)
    shutil.copytree(focused / "guard-ledger", target / "guard-ledger")
    zipped(focused / "runner.log", target / "runner.log.gz")
    for name in (FILLED_REPORT, *receipts["focused"]["filled_text_field"]["pngs"]):
        copy(focused / "artifacts" / name, target / "artifacts" / name)
    copy_sealed(STAGE, target / "harness", "focused-harness.sha256")
    for name in ("runner.sha256", "scope-selector.rs"):
        copy(STAGE / "focused-bin" / name, target / "harness" / name)

    live_stock, target = stock / "run", out / "stock-macos"
    for name in STOCK_PINS:
        copy(stock / name, target / name)
    copy_sealed(stock, target, "harness.sha256")
    copy(stock / "README.md", target / "reproduction.md")
    for name in ("invocation.json", OBSERVATION, "inputs-after.json", STOCK_REPORT):
        copy(live_stock / name, target / name)
    shutil.copytree(live_stock / "guard-ledger", target / "guard-ledger")
    zipped(live_stock / "runner.log", target / "runner.log.gz")
    copy(offline, target / OFFLINE)
    fields = receipts["stock_macos"]
    for name in (*RETAINED, FILLED_REPORT, OUTLINE_REPORT, *fields["filled_text_field"]["pngs"],
                 *fields["outlined_text_field"]["pngs"]):
        copy(live_stock / "artifacts" / name, target / "artifacts" / name)
    write_json(out / FINAL_REPORT, receipts)
    (out / "README.md").write_text(f'''# Surface v0.1.62 Filled Text Field candidate proof

Exact source manifest `{SOURCE}` pins 670 files. Its Git tree is `{TREE}`.
The 15-path patch applies to the v0.1.61 tree `{BASE_TREE}`. This archive records local evidence.

- All five stock macOS suites passed {fields['total_checks']:,} checks. The
  Material suite ran 331 steps through M332. The IME suite ran all 20 steps.
- Chrome used Apple Metal. The guard observed zero visible owned windows and
  zero owned survivors. Canvas evidence includes the six original Filled PNGs
  and five inherited Outlined PNGs from the same exact stock run.
- The separate focused run passed 235 checks on the four canonical Gallery Web
  files also used by stock. Its six PNGs and guard receipt are separate from stock. The two trusted Enter events, including the first during synthetic
  CDP preedit, produce zero and one submit. The receipt records whether an
  explicit synthetic text commit was needed.
- The native Gallery replay saved four PNGs twice. Page.screenshot, CLI and
  MCP matched. All 16 observations found zero visible windows.
- The workspace passed {receipts['rust']['workspace']['passed']:,} tests and
  three Wasm checks. Apple M1 Max Metal ran 18 offscreen tests, including 16
  primary tests and two supplemental raster-cache and text-selection tests.

## Offline recheck

From the archive, use Python 3.12 or newer with Pillow. Provide the reviewed
SHA-256 of `SHA256SUMS`. This command reads proof only and starts no app.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-filled-field.py --verify-package . --package-sha "$manifest_sha"
```

`source.sha256` and `base-source.sha256` verify the exact source bytes.
Decompress `filled-v0162.patch.gz` for the reviewed patch, and decompress the
Rust and runner logs to obtain their original bytes. The copied stock harness,
focused runner builder and native replay script give the exact test recipes.
The original local stock run retains all raw artifacts. This compact archive
preserves its signed-by-hash receipts, editor PNGs and raw inherited controls.
''')
    seal = manifest_package(out)
    return {"path": str(out), "manifest_sha256": seal, "files": len(manifest(out / "SHA256SUMS"))}


def verify_package(root, expected):
    verify_seal(root, "SHA256SUMS", expected, strict=True)
    result = read_json(root / FINAL_REPORT)
    require(result["status"] == "verified"
            and digest(root / "source.sha256") == SOURCE and len(manifest(root / "source.sha256")) == 670
            and digest(root / "base-source.sha256") == BASE_SOURCE and len(manifest(root / "base-source.sha256")) == 668
            and sha256(materialize(root / "filled-v0162.patch.gz")).hexdigest() == PATCH,
            "Exact candidate or prior source and patch archive changed")
    require(result["source"]["tree"] == TREE and result["source"]["base_tree"] == BASE_TREE
            and result["source"]["source_sha256"] == SOURCE
            and len(result["source"]["changed_paths"]) == 15, "Archived source summary changed")
    stock, focus, native = root / "stock-macos", root / "focused", root / "native"
    observer, filled, outlined = stock_tools(stock)
    verify_seal(focus / "harness", "focused-harness.sha256", FOCUS_HARNESS)
    match_focused_stock(result["focused"], stock / "browser-binary-sha256.txt")
    require((focus / "harness" / "runner.sha256").read_text().strip() == FOCUS_RUNNER
            and focused_info(focus, filled) == result["focused"], "Archived focused evidence differs")
    require(native_info(native / "proof", native / "native-binaries.sha256", native / "terminal.log", native / "harness")
            == result["native"] and digest(stock / "native-binaries.sha256") == digest(native / "native-binaries.sha256"),
            "Archived native evidence or binary parity differs")
    require(toolchain(*(root / "rust" / f"{name}.gz" for name in RUST_LOGS)) == result["rust"],
            "Archived Rust or Metal evidence differs")
    require(stock_info(stock, stock, stock / OFFLINE, observer, filled, outlined) == result["stock_macos"],
            "Archived exact stock evidence differs")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-sha")
    parser.add_argument("--tree")
    parser.add_argument("--package-sha")
    parser.add_argument("--stock-offline-sha")
    parser.add_argument("--stock-runner-sha")
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--preflight", action="store_true")
    action.add_argument("--out", type=Path)
    action.add_argument("--verify-package", type=Path)
    args = parser.parse_args()
    if args.verify_package:
        result = verify_package(args.verify_package.resolve(), args.package_sha)
    else:
        require(args.source_sha == SOURCE and args.tree == TREE, "Supply the exact reviewed .62 source seal and tree")
        source = source_info(STAGE, BASE)
        observer, filled, outlined = stock_tools(STOCK)
        verify_seal(STAGE, "focused-harness.sha256", FOCUS_HARNESS)
        require_paths(STAGE, ("native-v0162-passed", "native-proof-dir", "native-final-runtime.log"))
        native = Path((STAGE / "native-proof-dir").read_text().strip()).resolve(strict=True)
        require(native == (NATIVE / "exact-1").resolve(), "Native sealed path differs")
        binary_manifest = STAGE / "native-v0162-binaries" / "SHA256SUMS"
        verify_entries(binary_manifest.parent, manifest(binary_manifest))
        native_receipt = native_info(native, binary_manifest, STAGE / "native-final-runtime.log", NATIVE)
        require_paths(STAGE, ("workspace-v0162-passed", "gpu-v0162-passed", *RUST_LOGS))
        rust = toolchain(*(STAGE / name for name in RUST_LOGS))
        result = {"status": "preflight only", "source": source, "native": native_receipt, "rust": rust}
        focus = STAGE / FOCUS_DIRECTORY
        focused_ready = all((focus / name).is_file() for name in ("runner.log", OBSERVATION, "invocation.json"))
        if focused_ready:
            result["focused"] = focused_info(focus, filled, live=True)
            if (STOCK / "browser-binary-sha256.txt").is_file():
                match_focused_stock(result["focused"], STOCK / "browser-binary-sha256.txt")
        else:
            require(not args.out, f"Await exact focused evidence: {focus}")
            result["focused"] = "awaiting exact canonical Gallery replay"
        if args.out:
            require_paths(STOCK, ("stock-mac-v0162-passed", OFFLINE, "approved-runner.sha256"))
            offline = STOCK / OFFLINE
            require(args.stock_offline_sha == digest(offline) and HEX.fullmatch(args.stock_offline_sha or "")
                    and args.stock_runner_sha == (STOCK / "approved-runner.sha256").read_text().strip()
                    and HEX.fullmatch(args.stock_runner_sha or "") and digest(STOCK / "native-binaries.sha256") == digest(binary_manifest),
                    "Supply exact stock receipt and runner digests and matching native executable pins")
            baked = subprocess.run([sys.executable, "-E", "-B", str(STOCK / "verify-stock-receipt.py"), str(STOCK / "run")],
                                   capture_output=True, text=True, check=False)
            require(baked.returncode == 0 and json.loads(baked.stdout) == read_json(offline),
                    f"Original strict stock replay failed: {baked.stderr}")
            result["stock_macos"] = stock_info(STOCK / "run", STOCK, offline, observer, filled, outlined)
            require(result["stock_macos"]["runner_sha256"] == args.stock_runner_sha, "The stock runner seal changed")
            result["status"] = "verified"
            result = {**result, "output": export(args.out.resolve(), result, STOCK, offline, native, binary_manifest)}
    print(json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, RuntimeError, FileNotFoundError, subprocess.CalledProcessError, KeyError, TypeError) as error:
        sys.exit(f"Filled Text Field proof stopped. {error}")
