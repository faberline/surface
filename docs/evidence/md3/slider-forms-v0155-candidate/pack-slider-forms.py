#!/usr/bin/env python3
"""Package exact Slider Forms evidence without starting a host."""

import argparse
import gzip
from hashlib import file_digest, sha1, sha256
import json
import os
from pathlib import Path, PurePosixPath
import re
from runpy import run_path
import shutil
import subprocess
import sys
import tomllib

from PIL import Image

STAGE = Path("/tmp/surface-md3-slider-forms-v0155.DnpqGklo")
BASE = Path("/tmp/surface-md3-sliders-v0154.FBTvDz1t")
STOCK_ROOT = Path("/tmp/surface-md3-slider-forms-stock.oFuPnVjo")
NATIVE_ROOT = Path("/tmp/surface-md3-slider-forms-native.mVMBzhTT")
VERSION = "0.1.55"
SOURCE = "93fbdb8c80cfaf7698ae702a8bbc155b15411f2fe427048e1040100a672fc2d4"
TREE = "18bf4098eefd03ff562e058ceb5d7f99a34447a2"
BASE_SOURCE = "1e1c1be43dbd652ff26dbaab2f264651c2c811915bc04e41dfbedd425ba830d8"
BASE_TREE = "a92d66c4998656b1749f322723aa4324028afdc4"
PATCH = "943fbb810dc8a252db2281e7a37f004fdf7fb3211b94c5599f9b6606ec52358e"
PATCH_NAME = "slider-forms-v0155.patch"
NATIVE_HARNESS = "515fdf6bfe525b85caa81f1f88da82ba6bb39da34abb287b0b4f735cdfcba6e7"
STOCK_HARNESS = "362d067bea1eb567118cfe2c8bff5d01a6cc227596bd26d0a15d4a841422f520"
PIXELS = "13d2a3b233e36fbbf44410c3b2b4f213a6832e4495670ed54976fce2d6e62b95"
REPORT = "material-slider-forms.json"
NATIVE_REPORT = "native-slider-forms-report.json"
STOCK_REPORT = "verified-slider-forms-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
OBSERVATION = "headless-window-observation.json"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
RUST_LOGS = ("workspace-v0155-exact.log", "gpu-v0155-exact.log")
SUPPLEMENTAL = "gpu-v0155-supplemental.log"
NATIVE_IMAGES = {
    "wide-light-rest.png": (960, 1040),
    "wide-light-centered-focus.png": (960, 1040),
    "wide-light-vertical-focus.png": (960, 1040),
    "compact-dark-rtl-200-centered-focus.png": (320, 880),
    "compact-dark-rtl-200-vertical-focus.png": (320, 880),
}
WEB_IMAGES = (
    "material-slider-forms-centered-light.png", "material-slider-forms-centered-keys.png",
    "material-slider-forms-vertical-light.png", "material-slider-forms-vertical-compact-dark-rtl-200.png",
)
STOCK_PINS = (
    "browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256",
    "release-pin.json", "source-origins.json", "build-markers.json", "sealed-inputs.sha256",
    "inputs-before.json", "approved-runner.sha256",
)
RETAINED = (
    "browser-ime.json", "browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json",
    "browser-smoke.json", "bridge-web-result.json", "devtools-headless-result.json",
    "material-standard-slider.json",
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
    for root, entries, count, tree in ((base / "source", before, 635, BASE_TREE),
                                       (stage / "source", after, 640, TREE)):
        require(len(entries) == count, "Wrong source inventory count")
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
                  if p.is_file() and p.relative_to(root).parts[0] != ".git"}
        require(actual == set(entries), f"Source file inventory differs: {root}")
        verify_entries(root, entries)
        require(source_tree(root, entries) == tree, f"Wrong exact source tree: {root}")
    root = stage / "source"
    patch = stage / PATCH_NAME
    require(digest(patch) == PATCH and (stage / "slider-forms-tree").read_text().strip() == TREE
            and tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == VERSION,
            "Wrong candidate patch, tree marker or workspace version")
    paths = {p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch.read_bytes(), re.M)}
    changed = {name for name in before.keys() | after.keys() if before.get(name) != after.get(name)}
    require(paths == changed == set((stage / "owned-forms.paths").read_text().splitlines()),
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
    require(rust["passed"] == 1439 and rust["ignored"] == 19
            and tail.count("Finished `dev` profile") == 3
            and re.search(r"^provenance: .+ self-test cases passed\)", text, re.M)
            and "Doc-tests surface_material_gallery" in text
            and "compact_dark_rtl_focus_preserves_labels_and_adjusts_only_the_owners ... ok" in text
            and "wide_route_mounts_both_forms_and_restores_the_opener ... ok" in text,
            "The workspace lacks complete Slider Forms, provenance or Wasm checks")
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
        for key, samples in parts.items():
            if key == "engaged":
                require(samples["ink_pixels"] >= 16 and samples["focus_pixels"] >= 40, f"Missing native label: {name}")
                continue
            for sample in samples if key == "ticks" else [samples]:
                require(list(picture.getpixel(tuple(sample["point"]))) == sample["rgba"],
                        f"Native {key} pixel differs in {name}")


def native_info(root, expected, binaries_manifest):
    entries = verify_seal(root, "SHA256SUMS", expected, strict=True)
    report = read_json(root / NATIVE_REPORT)
    binaries = manifest(binaries_manifest)
    require(set(binaries) == {"surface-material-gallery", "surface-devtools"}
            and report["binaries"] == binaries and report["native_binary_manifest_sha256"] == digest(binaries_manifest),
            "Native proof used different Gallery or DevTools executables")
    require(report["schema"] == "surface.test.slider-forms.native.v1"
            and report["source"] == {"manifest_sha256": SOURCE, "files_verified": 640, "version": VERSION}
            and report["rendering"] == "headless" and report["reduce_motion"] is True
            and report["transports"] == ["Page.screenshot", "CLI", "MCP"]
            and len(report["replays"]) == 2, "Native proof scope differs")
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
                    and profile["disabled_skipped_by_tab"] is True and profile["disabled_value"] == 50
                    and profile["closed_focus"] == "slider-forms-demo-show"
                    and profile["disabled_set_value"] == "disabled", "Native profile or guard differs")
            samples += len(profile["visible_windows"])
            prefix = "compact-dark-rtl-200" if rtl else "wide-light"
            centered, vertical = profile["centered"], profile["vertical"]
            require(len(centered["keys"]) == len(vertical["keys"]) == 10
                    and centered["set_value"][-1]["value"] == -37.25
                    and vertical["set_value"]["value"] == 37.25
                    and (not rtl or vertical["scroll_y"] > 0), "Native adjustment or compact scroll differs")
            if not rtl:
                native_shot(directory, profile["rest"]["png"], "wide-light-rest.png", pngs)
            native_shot(directory, centered["png"], f"{prefix}-centered-focus.png", pngs)
            native_shot(directory, vertical["png"], f"{prefix}-vertical-focus.png", pngs)
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
            and steps == [step for step in range(1, 306) if step != 23] and ime == list(range(1, 21))
            and re.findall(r"^e2e result: (.*?); artifacts in ", text, re.M) == [f"5 passed; 0 failed; {total} checks"]
            and cases == stock["cases"] and stock["total_checks"] == total and cases["material"]["steps"] == 304
            and cases["ime"] == {"steps": 20, "checks": 77} and stock["page_errors"] == 0,
            "Stock run lacks all five suites or exact M305 and IME coverage")
    adapter = stock["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False and stock["guard_and_window_checks"] == "passed",
            "Stock browser does not prove actual Apple Metal and guard success")
    after = read_json(root / "inputs-after.json")
    invocation = read_json(root / "invocation.json")
    require(after == invocation["inputs"] == read_json(pins / "inputs-before.json")
            and invocation["suite_arguments"] == list(SUITES) and invocation["injected_chrome_flags"] == []
            and after["source_manifest_sha256"] == SOURCE and after["source_files_checked_per_origin"] == 640
            and after["release"] == offline["release"] == read_json(pins / "release-pin.json")
            and after["release"]["version"] == VERSION and after["release"]["last_material_step"] == 305
            and digest(pins / "browser-source.sha256") == SOURCE
            and after["app_bundle_seal_sha256"] == digest(pins / "sealed-inputs.sha256")
            and after["stock_files"] == manifest(pins / "browser-binary-sha256.txt")
            and after["bundle_files_checked"] == len(after["stock_files"]) == 16
            and after["runner_sha256"] == (pins / "approved-runner.sha256").read_text().strip()
            == after["stock_files"]["debug/surface-e2e"],
            "Stock proof does not match final source, invocation or sealed inputs")
    pixels = verify_pixels(root / "artifacts", {"browser": stock["browser"], "adapter": adapter})
    require(pixels == stock["slider_forms"], "Independent stock pixel replay differs")
    return {"receipt_sha256": offline_sha, "total_checks": total, "cases": cases, "adapter": adapter,
            "browser": stock["browser"], "page_errors": 0, "visible_windows": 0, "captures": pixels["captures"]}


def pixels_program(path):
    require(digest(path) == PIXELS, "The stock pixel verifier changed")
    return run_path(str(path))["verify"]


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
    return web_info(stock / "run", stock, offline, offline_sha, pixels_program(stock / "verify_forms_pixels.py"))


def focused_info(stage, verify_pixels):
    path = Path((stage / "focused-dir").read_text().strip()) / "run-2"
    log = (path / "runner.log").read_text()
    require(re.findall(r"^  \[M(\d+)\]", log, re.M) == list(map(str, range(298, 306)))
            and re.findall(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks,", log, re.M)
            == [("material", "8", "504")]
            and re.search(r"^e2e result: 1 passed; 0 failed; 504 checks;", log, re.M),
            "Focused receipt is missing its actual original eight steps")
    guard(read_json(path / OBSERVATION))
    invocation = read_json(path / "invocation.json")
    require(invocation["scope"] == "focused development pack" and invocation["argv"][-1] == "material",
            "Focused receipt has the wrong scope")
    pixels = verify_pixels(path / "artifacts")
    require(pixels == read_json(stage / "focused-mac-r2-pixels.json"), "Focused original pixel report disagrees")
    return path, {"scope": invocation["scope"], "checks": 504, "steps": 8, "visible_windows": 0,
                  "window_receipt_sha256": digest(path / OBSERVATION), "runner_log_sha256": digest(path / "runner.log"),
                  "pixels_sha256": digest(stage / "focused-mac-r2-pixels.json")}


def required_files(stage, stock, native):
    paths = [stage / name for name in ("workspace-v0155-passed", "gpu-v0155-exact-passed",
             "native-v0155-passed", "stock-mac-exact-passed", OFFLINE, *RUST_LOGS)]
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
    prose = f"""# Surface v0.1.55 Slider Forms evidence

The 640-file candidate matches tree `{TREE}`. The patch applies to v0.1.54 tree `{BASE_TREE}`.
The source manifest SHA-256 is `{SOURCE}`.

- The workspace gate passed {rust['workspace']['passed']:,} Rust tests and three Wasm checks. The GPU gate ran {rust['metal']['passed']} offscreen tests on Metal.
- Two native headless replays each saved five PNGs. The captures match byte for byte across Page.screenshot, CLI and MCP and across both replays. {native['window_samples']} samples found zero visible windows.
- The five stock macOS suites passed {stock['total_checks']:,} checks through M305. Chrome used Apple Metal. The observer found zero owned windows and zero surviving processes. Four original Slider Forms PNGs are included.
- The separate focused run passed 504 checks through M298–M305. Its development receipts support diagnosis. The stock run supplies release evidence.

## Recheck in place

Use Python 3.12 or newer with Pillow 12.2 or newer. Supply the reviewed package manifest SHA-256.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-slider-forms.py --verify-package . --package-sha "$reviewed_seal"
```

The offline command checks the sealed contents, compares the native sampled pixels and replays the stock Slider Forms pixel checks. It opens no app. The compact package keeps the stock verdict and selected raw reports. The original run keeps the full browser artifact inventory. The source patch and test logs decompress to their original bytes.

## Reproduce from source

Apply the decompressed patch to the exact base tree. Use the committed dependency lockfile and the standard Surface build environment.

```sh
gzip -dc surface-v0.1.55.patch.gz > /tmp/surface-v0.1.55.patch
git apply --check --whitespace=error-all /tmp/surface-v0.1.55.patch
git apply --whitespace=error-all /tmp/surface-v0.1.55.patch
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
    copy(Path(__file__), out / "pack-slider-forms.py")
    copy(stage / "source.sha256", out / "source.sha256")
    copy(args.base / "source.sha256", out / "standard-slider-base.sha256")
    copy(stage / "slider-forms-tree", out / "slider-forms-tree")
    copy(stage / "owned-forms.paths", out / "owned-forms.paths")
    zipped(stage / PATCH_NAME, out / "surface-v0.1.55.patch.gz")
    for name in RUST_LOGS:
        zipped(stage / name, out / "rust" / f"{name}.gz")
    if "supplemental" in proof["rust"]:
        zipped(stage / SUPPLEMENTAL, out / "rust" / f"{SUPPLEMENTAL}.gz")
    shutil.copytree(native, out / "native" / "proof")
    shutil.copytree(args.native_harness, out / "native" / "harness", ignore=shutil.ignore_patterns("__pycache__"))
    copy(binary_manifest, out / "native" / "native-binaries.sha256")
    target = out / "stock-macos"
    for name in (*STOCK_PINS, "harness.sha256", "verify_forms_pixels.py", "bundle-cli.txt",
                 "observe-slider-forms.py", "prepare-pins.py", "verify-stock-receipt.py", "headless-guard-self-test.txt"):
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
    write_json(out / "verified-slider-forms-v0155.json", proof)
    readme(out, proof)
    seal = manifest_package(out)
    verify_package(out, seal)
    return {"path": str(out), "manifest_sha256": seal, "files": len(manifest(out / "SHA256SUMS"))}


def verify_package(root, expected):
    verify_seal(root, "SHA256SUMS", expected, strict=True)
    report = read_json(root / "verified-slider-forms-v0155.json")
    source = report["source"]
    require(source == {"version": VERSION, "source_sha256": SOURCE, "source_files": 640, "tree": TREE,
                       "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
                       "changed_paths": len((root / "owned-forms.paths").read_text().splitlines())}
            and digest(root / "source.sha256") == SOURCE and digest(root / "standard-slider-base.sha256") == BASE_SOURCE
            and (root / "slider-forms-tree").read_text().strip() == TREE
            and sha256(materialize(root / "surface-v0.1.55.patch.gz")).hexdigest() == PATCH,
            "Packaged source pins differ")
    extra = root / "rust" / f"{SUPPLEMENTAL}.gz"
    rust = rust_info(*(root / "rust" / f"{name}.gz" for name in RUST_LOGS),
                     supplemental=extra if extra.is_file() else None)
    require(rust == report["rust"], "Packaged Rust or Metal receipt differs")
    native = root / "native"
    verify_seal(native / "harness", "harness.sha256", NATIVE_HARNESS)
    require(native_info(native / "proof", report["native"]["seal"], native / "native-binaries.sha256")
            == report["native"], "Packaged native receipt differs")
    stock = root / "stock-macos"
    offline = read_json(stock / OFFLINE)
    verify_seal(stock, "harness.sha256", STOCK_HARNESS)
    verify_seal(stock, "sealed-inputs.sha256", read_json(stock / "inputs-before.json")["app_bundle_seal_sha256"])
    require(web_info(stock, stock, stock / OFFLINE, report["stock_macos"]["receipt_sha256"],
                     pixels_program(stock / "verify_forms_pixels.py")) == report["stock_macos"],
            "Packaged stock receipt differs")
    ime = offline["retained"]["retained"]["ime_cancellation"]
    for name, key in (("browser-ime-cancel-redraw.json", "select_all_after_cancel_redraw"),
                      ("browser-ime-cancel-selected-redraw.json", "backward_cancel_before_redraw")):
        require(read_json(stock / "artifacts" / name) == ime[key], f"Packaged I11 {name} differs")
    require(digest(stock / "artifacts/material-standard-slider.json") == offline["sliders_report_sha256"]
            and digest(stock / "native-binaries.sha256") == digest(native / "native-binaries.sha256"),
            "Packaged Standard Slider or native executable pins differ")
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
    parser.add_argument("--stock", type=Path, default=STOCK_ROOT)
    parser.add_argument("--native", type=Path, default=NATIVE_ROOT / "exact-v0155-r2")
    parser.add_argument("--native-harness", type=Path, default=NATIVE_ROOT / "harness-r2")
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
        checked_source = source_info(args.stage, args.base)
        required_files(args.stage, args.stock, args.native)
        require(HEX.fullmatch(args.native_seal or "") and HEX.fullmatch(args.stock_offline_sha or ""),
                "Supply the reviewed native manifest and offline stock receipt digests")
        require(Path((args.stage / "native-proof-dir").read_text().strip()).resolve() == args.native.resolve()
                and Path((args.stage / "stock-observer-dir").read_text().strip()).resolve() == args.stock.resolve(),
                "Stage native and stock pointers differ")
        verify_seal(args.native_harness, "harness.sha256", NATIVE_HARNESS)
        binary_manifest = args.stage / "native-v0155-binaries" / "SHA256SUMS"
        verify_entries(binary_manifest.parent, manifest(binary_manifest))
        native = native_info(args.native, args.native_seal, binary_manifest)
        extra = (args.stage / SUPPLEMENTAL if (args.stage / "gpu-v0155-supplemental-passed").is_file() else None)
        rust = rust_info(*(args.stage / name for name in RUST_LOGS), supplemental=extra)
        stock = verify_stock_input(args.stock, args.stage / OFFLINE, args.stock_offline_sha, binary_manifest)
        focused, supplemental = focused_info(args.stage, pixels_program(args.stock / "verify_forms_pixels.py"))
        result = {"status": "verified", "source": checked_source, "rust": rust, "native": native,
                  "stock_macos": stock, "focused": supplemental}
        if args.out:
            result = {**result, "output": export(args, result, binary_manifest, focused)}
    print(json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, FileNotFoundError, subprocess.CalledProcessError, KeyError, TypeError) as error:
        print(f"Slider Forms proof stopped. {error}", file=sys.stderr)
        sys.exit(1)
