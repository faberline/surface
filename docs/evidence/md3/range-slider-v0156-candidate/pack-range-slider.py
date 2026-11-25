#!/usr/bin/env python3
"""Package exact Range Slider evidence without starting a host."""

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

STAGE = Path("/tmp/surface-md3-range-slider-v0156.DvRoswnb")
BASE = Path("/tmp/surface-md3-slider-forms-v0155.DnpqGklo")
NATIVE_ROOT = Path("/tmp/surface-md3-range-slider-native.w3YSl0Ba")
VERSION = "0.1.56"
SOURCE = "de92395ee2ab7b8d7d49760adb2722d5b44ca270c2002deb6fefbecd809e8647"
TREE = "6e7c09a6eda249eb046babf3f2a4b392b0aaa99c"
BASE_SOURCE = "93fbdb8c80cfaf7698ae702a8bbc155b15411f2fe427048e1040100a672fc2d4"
BASE_TREE = "18bf4098eefd03ff562e058ceb5d7f99a34447a2"
PATCH = "4c529b2797dd18eea855bcfe354dc1d8a85f8c1cc06aba47cbc076a1dc35e28c"
PATCH_NAME = "range-slider-v0156.patch"
NATIVE_HARNESS = "f0a3d5795564c9986efd14a990fd077eabbf09542be46b3013080161e7a2abdf"
STOCK_HARNESS = "5ab80d4e6b807d1ee8c5b2c49456fb00b7567727617cb011216b0d7d0e01f690"
PIXELS = "eb0b46d89b9c73dbe933b9d592077ccde9cbc75bdb379fef97f2aedb1c2cd40d"
FORMS_PIXELS = "13d2a3b233e36fbbf44410c3b2b4f213a6832e4495670ed54976fce2d6e62b95"
REPORT = "material-range-slider.json"
NATIVE_REPORT = "native-range-slider-report.json"
STOCK_REPORT = "verified-range-slider-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
OBSERVATION = "headless-window-observation.json"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
RUST_LOGS = ("workspace-v0156-exact.log", "gpu-v0156-exact.log")
SUPPLEMENTAL = "gpu-v0156-supplemental.log"
NATIVE_IMAGES = {
    "wide-light-rest.png": (960, 1040),
    "wide-light-discrete-upper-focus.png": (960, 1040),
    "wide-light-equal-lower-focus.png": (960, 1040),
    "compact-dark-rtl-200-continuous-lower-focus.png": (320, 880),
    "compact-dark-rtl-200-discrete-upper-focus.png": (320, 880),
}
WEB_IMAGES = (
    "material-range-slider-wide-light.png", "material-range-slider-overlap-focus.png",
    "material-range-slider-held-loss.png", "material-range-slider-compact-dark-rtl-200.png",
)
STOCK_PINS = (
    "browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256",
    "release-pin.json", "source-origins.json", "build-markers.json", "sealed-inputs.sha256",
    "inputs-before.json", "approved-runner.sha256",
)
RETAINED = (
    "browser-ime.json", "browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json",
    "browser-smoke.json", "bridge-web-result.json", "devtools-headless-result.json",
    "material-standard-slider.json", "material-slider-forms.json",
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
    for root, entries, count, tree in ((base / "source", before, 640, BASE_TREE),
                                       (stage / "source", after, 646, TREE)):
        require(len(entries) == count, "Wrong source inventory count")
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
                  if p.is_file() and p.relative_to(root).parts[0] != ".git"}
        require(actual == set(entries), f"Source file inventory differs: {root}")
        verify_entries(root, entries)
        require(source_tree(root, entries) == tree, f"Wrong exact source tree: {root}")
    root = stage / "source"
    patch = stage / PATCH_NAME
    require(digest(patch) == PATCH and (stage / "range-slider-tree").read_text().strip() == TREE
            and tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == VERSION,
            "Wrong candidate patch, tree marker or workspace version")
    material = (root / "e2e/src/material.rs").read_text()
    registry = re.search(r"pub const STEPS: &\[&str\] = &\[(.*?)\];", material, re.S)
    required_steps = [str(step) for step in range(1, 310) if step != 23]
    require(registry and re.findall(r'"M(\d+)"', registry.group(1)) == required_steps,
            "The stock runner must require every Material step through M309 except failure-only M23")
    paths = {p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch.read_bytes(), re.M)}
    changed = {name for name in before.keys() | after.keys() if before.get(name) != after.get(name)}
    require(paths == changed == set((stage / "owned-range.paths").read_text().splitlines()),
            "Patch, source inventory and owned paths differ")
    for directory, options in ((base / "source", []), (root, ["--reverse"])):
        subprocess.run(["git", "-C", str(directory), "apply", "--check", "--whitespace=error-all",
                        *options, str(patch)], check=True, env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"})
    return {"version": VERSION, "source_sha256": SOURCE, "source_files": len(after), "tree": TREE,
            "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
            "changed_paths": len(paths)}


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


def rust_info(workspace, gpu, supplemental=None):
    text, rust = tests(workspace)
    tail = text.rsplit("\ntest result:", 1)[-1]
    require(rust["passed"] == 1464 and rust["ignored"] == 19
            and tail.count("Finished `dev` profile") == 3
            and re.search(r"^provenance: .+ self-test cases passed\)", text, re.M)
            and "Doc-tests surface_material_gallery" in text
            and "range_slider_steps_are_required_by_the_material_case ... ok" in text
            and "compact_dark_rtl_focus_and_scroll_keep_the_rail_label_and_disabled_targets_inside ... ok" in text
            and "old_slider_routes_keep_their_ids_and_range_has_its_own_named_focus_cycle ... ok" in text,
            "The workspace lacks complete Range Slider, provenance or Wasm checks")
    metal_text, metal = tests(gpu)
    require(metal["passed"] == 16 and metal["ignored"] == 0 and "backend: Metal" in metal_text,
            "Missing the 16 actual offscreen Metal tests")
    result = {"workspace": {**rust, "wasm_checks": 3}, "metal": {**metal, "backend": "Metal"}}
    if supplemental is not None:
        extra_text, extra = tests(supplemental)
        require(extra["passed"] == 2 and extra["ignored"] == 0 and "backend: Metal" in extra_text,
                "The marked supplemental Metal run is incomplete")
        result["supplemental"] = {**extra, "backend": "Metal"}
    return result


def open_png(path, expected):
    with Image.open(path) as picture:
        require(picture.format == "PNG" and picture.size == expected, f"Wrong PNG dimensions: {path}")
        picture.load()
        return picture.convert("RGBA")


def native_shot(directory, shot, name, hashes):
    require(shot["name"] == name and shot["sha256"] == hashes[name], f"Wrong native capture receipt: {name}")
    picture = open_png(directory / name, NATIVE_IMAGES[name])
    require(list(picture.getpixel((4, 4))) == shot["background"], f"Native background mismatch: {name}")
    for parts in shot["paint"].values():
        require(parts["caption_ink"]["ink_pixels"] >= 16, f"Missing native caption: {name}")
        samples = list(parts["pixels"].values())
        if "gap_pixel" in parts:
            samples.append(parts["gap_pixel"])
        for sample in samples:
            require(list(picture.getpixel(tuple(sample["at"]))) == sample["rgba"],
                    f"Native sampled pixel differs in {name}")
        label = parts.get("value_label")
        if label is not None:
            b, r = label["label"], label["ring"]
            midpoint = (floor(b["x"] + b["width"] / 2), floor(b["y"] + 8))
            require(list(picture.getpixel(midpoint)) == label["label_rgba"]
                    and label["ink"]["ink_pixels"] >= 16, f"Missing native label ink in {name}")
            ring = picture.crop((ceil(r["x"]), ceil(r["y"]), floor(r["x"] + r["width"]), floor(r["y"] + r["height"])))
            total = sum(color == tuple(label["ring_rgba"]) for color in ring.get_flattened_data())
            require(total == label["ring_pixels"] and total >= 40, f"Native focus paint differs in {name}")


def native_info(root, expected, binaries_manifest):
    entries = verify_seal(root, "SHA256SUMS", expected, strict=True)
    report = read_json(root / NATIVE_REPORT)
    binaries = manifest(binaries_manifest)
    require(set(binaries) == {"surface-material-gallery", "surface-devtools"}
            and report["binaries"] == binaries and report["native_binary_manifest_sha256"] == digest(binaries_manifest),
            "Native proof used different Gallery or DevTools executables")
    require(report["schema"] == "surface.test.range-slider.native.v1"
            and report["source"] == {"manifest_sha256": SOURCE, "files_verified": 646, "version": VERSION}
            and report["rendering"] == "headless" and report["reduce_motion"] is True
            and report["transports"] == ["Page.screenshot", "CLI", "MCP"]
            and len(report["replays"]) == 2 and report["replays"][0] == report["replays"][1],
            "Native proof scope or repeated typed receipts differ")
    pngs = report["byte_identical_pngs"]
    require(set(pngs) == set(NATIVE_IMAGES) and len(set(pngs.values())) == len(NATIVE_IMAGES),
            "Native PNG inventory differs or reused a capture across scenes")
    expected_files = {f"replay-{i}/{name}" for i in (1, 2) for name in NATIVE_IMAGES}
    require(set(entries) == expected_files | {NATIVE_REPORT}, "Native sealed recurrence files differ")
    samples = 0
    for index, profiles in enumerate(report["replays"], 1):
        directory = root / f"replay-{index}"
        require(set(profiles) == {"wide", "compact"}, "Missing native replay profile")
        for name, (width, height, rtl, scale) in {"wide": (960, 1040, False, 1),
                                                   "compact": (320, 880, True, 2)}.items():
            profile = profiles[name]
            require(profile["viewport"] == [width, height] and profile["rtl"] is rtl
                    and profile["text_scale"] == scale and profile["gallery_reaped"] is True
                    and profile["visible_windows"] and all(n == 0 for n in profile["visible_windows"])
                    and profile["skipped_disabled_tab"] is True
                    and profile["closed_focus"] == "range-slider-demo-show"
                    and profile["disabled"]["set_value"] == "disabled", "Native profile or guard differs")
            samples += len(profile["visible_windows"])
            continuous, discrete, equal = profile["continuous"], profile["discrete"], profile["equal"]
            require(continuous["set_value"][-1]["values"] == [37.25, 85.]
                    and discrete["set_value"][1]["values"] == [27.5, 52.5]
                    and discrete["upper_end"]["values"] == [52.5, 90.]
                    and len(equal["keys"]) == 6, "Native constrained range adjustment differs")
            if rtl:
                native_shot(directory, continuous["png"], "compact-dark-rtl-200-continuous-lower-focus.png", pngs)
                native_shot(directory, discrete["png"], "compact-dark-rtl-200-discrete-upper-focus.png", pngs)
            else:
                native_shot(directory, profile["rest"]["png"], "wide-light-rest.png", pngs)
                native_shot(directory, discrete["png"], "wide-light-discrete-upper-focus.png", pngs)
                native_shot(directory, equal["png"], "wide-light-equal-lower-focus.png", pngs)
        require(all(entries[f"replay-{index}/{name}"] == sha for name, sha in pngs.items()),
                "Two native captures do not have identical original bytes")
    require(samples == report["window_samples"] and samples >= 20, "Native observation sampling differs")
    return {"seal": expected, "report_sha256": digest(root / NATIVE_REPORT), "runs": 2,
            "visible_windows": 0, "window_samples": samples, "transports": report["transports"],
            "binaries": binaries, "captures": pngs}


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
            and all(event["allowed"] is True for event in launch), "Exact stock process ledger differs")
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
            and steps == [step for step in range(1, 310) if step != 23] and ime == list(range(1, 21))
            and re.findall(r"^e2e result: (.*?); artifacts in ", text, re.M) == [f"5 passed; 0 failed; {total} checks"]
            and cases == stock["cases"] and stock["total_checks"] == total and cases["material"]["steps"] == 308
            and cases["ime"] == {"steps": 20, "checks": 77} and stock["page_errors"] == 0,
            "Stock run lacks all five suites or exact M309 and IME coverage")
    adapter = stock["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False and stock["guard_and_window_checks"] == "passed",
            "Stock browser does not prove actual Apple Metal and guard success")
    after = read_json(root / "inputs-after.json")
    invocation = read_json(root / "invocation.json")
    require(after == invocation["inputs"] == read_json(pins / "inputs-before.json")
            and invocation["suite_arguments"] == list(SUITES) and invocation["injected_chrome_flags"] == []
            and after["source_manifest_sha256"] == SOURCE and after["source_files_checked_per_origin"] == 646
            and after["release"] == offline["release"] == read_json(pins / "release-pin.json")
            and after["release"]["version"] == VERSION and after["release"]["last_material_step"] == 309
            and digest(pins / "browser-source.sha256") == SOURCE
            and after["app_bundle_seal_sha256"] == digest(pins / "sealed-inputs.sha256")
            and after["stock_files"] == manifest(pins / "browser-binary-sha256.txt")
            and after["bundle_files_checked"] == len(after["stock_files"]) == 16
            and after["runner_sha256"] == (pins / "approved-runner.sha256").read_text().strip()
            == after["stock_files"]["debug/surface-e2e"],
            "Stock proof does not match final source, invocation or sealed inputs")
    pixels = verify_pixels(root / "artifacts", {"browser": stock["browser"], "adapter": adapter})
    require(pixels == stock["range_slider"], "Independent stock pixel replay differs")
    return {"receipt_sha256": offline_sha, "total_checks": total, "cases": cases, "adapter": adapter,
            "browser": stock["browser"], "page_errors": 0, "visible_windows": 0, "captures": pixels["captures"]}


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
    return web_info(stock / "run", stock, offline, offline_sha, pixels_program(stock / "verify_range_pixels.py"))


def focused_info(stage, verify_pixels):
    path = Path((stage / "focused-dir").read_text().strip()) / "run-2"
    log = (path / "runner.log").read_text()
    require(re.findall(r"^  \[M(\d+)\]", log, re.M) == list(map(str, range(298, 310)))
            and re.findall(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks,", log, re.M)
            == [("material", "12", "875")]
            and re.search(r"^e2e result: 1 passed; 0 failed; 875 checks;", log, re.M),
            "Focused receipt is missing its actual original twelve steps")
    guard(read_json(path / OBSERVATION))
    invocation = read_json(path / "invocation.json")
    require(invocation["scope"] == "focused development pack" and invocation["argv"][-1] == "material",
            "Focused receipt has the wrong scope")
    pixels = verify_pixels(path / "artifacts")
    require(pixels == read_json(stage / "focused-mac-r2-pixels.json"), "Focused original pixel report disagrees")
    return path, {"scope": invocation["scope"], "checks": 875, "steps": 12, "visible_windows": 0,
                  "window_receipt_sha256": digest(path / OBSERVATION), "runner_log_sha256": digest(path / "runner.log"),
                  "pixels_sha256": digest(stage / "focused-mac-r2-pixels.json")}


def required_files(stage, stock, native):
    paths = [stage / name for name in ("workspace-v0156-passed", "gpu-v0156-exact-passed",
             "native-v0156-passed", "stock-mac-exact-passed", OFFLINE, *RUST_LOGS)]
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
    prose = f"""# Surface v0.1.56 Range Slider evidence

The 646-file candidate matches tree `{TREE}`. The patch applies to v0.1.55 tree `{BASE_TREE}`.
The source manifest SHA-256 is `{SOURCE}`.

- The workspace gate passed {rust['workspace']['passed']:,} Rust tests and three Wasm checks. The GPU gate ran {rust['metal']['passed']} offscreen tests on Metal.
- Two native headless replays each saved five PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. {native['window_samples']} samples found zero visible windows.
- The five stock macOS suites passed {stock['total_checks']:,} checks through M309. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. Four original Range Slider PNGs are included.
- The separate focused run passed 875 checks through M298–M309. Its development receipts support diagnosis. The stock run supplies release evidence.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-range-slider.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Range Slider pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.56.patch.gz > /tmp/surface-v0.1.56.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.56.patch
git apply --whitespace=error-all /tmp/surface-v0.1.56.patch
shasum -a 256 -c /path/to/evidence/source.sha256
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/cargo-target/surface-slider-forms-repro
export SURFACE_GPU_EVIDENCE=/tmp/surface-slider-forms-repro/gpu-offscreen.png
sh scripts/check.sh
sh scripts/check-gpu.sh
```

The native harness README records the exact command and independent repeats. The stock invocation and inputs record the five case arguments and preflight and postflight pins. The local evidence does not cover Linux hosted CI, actual screen-reader speech or system accessibility settings.
"""
    (out / "README.md").write_text(prose)


def export(args, proof, binary_manifest, focused):
    out = args.out
    require(not out.exists() and not out.is_symlink()
            and out.parent.resolve(strict=True).is_relative_to(Path("/tmp").resolve()),
            "Use a fresh output directory under /tmp")
    out.mkdir(mode=0o700)
    stage, stock, native = args.stage, args.stock, args.native
    copy(Path(__file__), out / "pack-range-slider.py")
    copy(stage / "source.sha256", out / "source.sha256")
    copy(args.base / "source.sha256", out / "slider-forms-base.sha256")
    copy(stage / "range-slider-tree", out / "range-slider-tree")
    copy(stage / "owned-range.paths", out / "owned-range.paths")
    zipped(stage / PATCH_NAME, out / "surface-v0.1.56.patch.gz")
    for name in RUST_LOGS:
        zipped(stage / name, out / "rust" / f"{name}.gz")
    if "supplemental" in proof["rust"]:
        zipped(stage / SUPPLEMENTAL, out / "rust" / f"{SUPPLEMENTAL}.gz")
    shutil.copytree(native, out / "native" / "proof")
    harness = out / "native" / "harness"
    for name in ("harness.sha256", *manifest(args.native_harness / "harness.sha256")):
        copy(args.native_harness / name, harness / name)
    copy(binary_manifest, out / "native" / "native-binaries.sha256")
    copy(native.parent / f"{native.name}-pins.json", out / "native" / "input-pins.json")
    copy(native.parent / f"{native.name}.log", out / "native" / "terminal.log")
    target = out / "stock-macos"
    for name in (*STOCK_PINS, "harness.sha256", "verify_forms_pixels.py", "verify_range_pixels.py",
                 "observe-range-slider.py", "prepare-pins.py", "verify-stock-receipt.py"):
        copy(stock / name, target / name)
    for name in (STOCK_REPORT, OBSERVATION, "invocation.json", "inputs-after.json"):
        copy(stock / "run" / name, target / name)
    for path in (stock / "run" / "guard-ledger").glob("*.json"):
        copy(path, target / "guard-ledger" / path.name)
    zipped(stock / "run" / "runner.log", target / "runner.log.gz")
    copy(stage / OFFLINE, target / OFFLINE)
    for name in (REPORT, *WEB_IMAGES, *RETAINED):
        copy(stock / "run" / "artifacts" / name, target / "artifacts" / name)
    for name in (OBSERVATION, "invocation.json"):
        copy(focused / name, out / "focused" / name)
    zipped(focused / "runner.log", out / "focused" / "runner.log.gz")
    copy(focused / "artifacts" / REPORT, out / "focused" / REPORT)
    copy(stage / "focused-mac-r2-pixels.json", out / "focused" / "pixels.json")
    write_json(out / "verified-range-slider-v0156.json", proof)
    readme(out, proof)
    seal = manifest_package(out)
    verify_package(out, seal)
    return {"path": str(out), "manifest_sha256": seal, "files": len(manifest(out / "SHA256SUMS"))}


def verify_package(root, expected):
    verify_seal(root, "SHA256SUMS", expected, strict=True)
    report = read_json(root / "verified-range-slider-v0156.json")
    source = report["source"]
    require(source == {"version": VERSION, "source_sha256": SOURCE, "source_files": 646, "tree": TREE,
                       "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
                       "changed_paths": len((root / "owned-range.paths").read_text().splitlines())}
            and digest(root / "source.sha256") == SOURCE and digest(root / "slider-forms-base.sha256") == BASE_SOURCE
            and (root / "range-slider-tree").read_text().strip() == TREE
            and sha256(materialize(root / "surface-v0.1.56.patch.gz")).hexdigest() == PATCH,
            "Packaged source pins differ")
    extra = root / "rust" / f"{SUPPLEMENTAL}.gz"
    rust = rust_info(*(root / "rust" / f"{name}.gz" for name in RUST_LOGS),
                     supplemental=extra if extra.is_file() else None)
    require(rust == report["rust"], "Packaged Rust or Metal receipt differs")
    native = root / "native"
    verify_seal(native / "harness", "harness.sha256", NATIVE_HARNESS)
    require(native_info(native / "proof", report["native"]["seal"], native / "native-binaries.sha256")
            == report["native"], "Packaged native receipt differs")
    pins = read_json(native / "input-pins.json")
    terminal = read_json(native / "terminal.log")
    require(pins["status"] == "pins-verified" and pins["binaries"] == report["native"]["binaries"]
            and pins["source"] == {"manifest_sha256": SOURCE, "files_verified": 646, "version": VERSION}
            and terminal["manifest_sha256"] == report["native"]["seal"]
            and terminal["visible_windows"] == 0 and terminal["pngs"] == report["native"]["captures"],
            "Original native preflight or terminal receipt differs")
    stock = root / "stock-macos"
    offline = read_json(stock / OFFLINE)
    verify_seal(stock, "harness.sha256", STOCK_HARNESS)
    verify_seal(stock, "sealed-inputs.sha256", read_json(stock / "inputs-before.json")["app_bundle_seal_sha256"])
    require(web_info(stock, stock, stock / OFFLINE, report["stock_macos"]["receipt_sha256"],
                     pixels_program(stock / "verify_range_pixels.py")) == report["stock_macos"],
            "Packaged stock receipt differs")
    ime = offline["retained"]["retained"]["ime_cancellation"]
    for name, key in (("browser-ime-cancel-redraw.json", "select_all_after_cancel_redraw"),
                      ("browser-ime-cancel-selected-redraw.json", "backward_cancel_before_redraw")):
        require(read_json(stock / "artifacts" / name) == ime[key], f"Packaged I11 {name} differs")
    require(digest(stock / "artifacts/material-standard-slider.json") == offline["sliders_report_sha256"]
            and digest(stock / "artifacts/material-slider-forms.json") == offline["slider_forms"]["report_sha256"]
            and digest(stock / "native-binaries.sha256") == digest(native / "native-binaries.sha256"),
            "Packaged predecessor report or native executable pins differ")
    focus_dir = root / "focused"
    extra = report["focused"]
    guard(read_json(focus_dir / OBSERVATION))
    require(sha256(materialize(focus_dir / "runner.log.gz")).hexdigest() == extra["runner_log_sha256"]
            and digest(focus_dir / OBSERVATION) == extra["window_receipt_sha256"]
            and digest(focus_dir / "pixels.json") == extra["pixels_sha256"]
            and digest(focus_dir / REPORT) == read_json(focus_dir / "pixels.json")["report_sha256"],
            "Packaged supplemental focused receipt differs")
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
        binary_manifest = args.stage / "native-v0156-r3-binaries" / "SHA256SUMS"
        verify_entries(binary_manifest.parent, manifest(binary_manifest))
        native = native_info(args.native, args.native_seal, binary_manifest)
        extra = (args.stage / SUPPLEMENTAL if (args.stage / "gpu-v0156-supplemental-passed").is_file() else None)
        rust = rust_info(*(args.stage / name for name in RUST_LOGS), supplemental=extra)
        stock = verify_stock_input(args.stock, args.stage / OFFLINE, args.stock_offline_sha, binary_manifest)
        focused, supplemental = focused_info(args.stage, pixels_program(args.stock / "verify_range_pixels.py"))
        result = {"status": "verified", "source": checked_source, "rust": rust, "native": native,
                  "stock_macos": stock, "focused": supplemental}
        if args.out:
            result = {**result, "output": export(args, result, binary_manifest, focused)}
    print(json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, FileNotFoundError, subprocess.CalledProcessError, KeyError, TypeError) as error:
        print(f"Range Slider proof stopped. {error}", file=sys.stderr)
        sys.exit(1)
