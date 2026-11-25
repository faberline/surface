#!/usr/bin/env python3
"""Verify exact Standard Slider evidence and export compact and full offline archives."""

from argparse import ArgumentParser
import gzip
from hashlib import file_digest, sha1, sha256
import json
from pathlib import Path
import re
from shutil import copy2, copyfileobj, copytree
import subprocess
import sys
import tomllib

from PIL import Image

STAGE = Path("/tmp/surface-md3-sliders-v0154.FBTvDz1t")
OBSERVER = Path("/tmp/surface-md3-sliders-stock.lK6a9gKl")
BASE = Path("/tmp/surface-md3-icon-buttons-v0153.csjCVwaB")
NATIVE = Path("/tmp/surface-md3-slider-native-proof.RtLxWU1f/exact-metal")
SOURCE_SHA = "1e1c1be43dbd652ff26dbaab2f264651c2c811915bc04e41dfbedd425ba830d8"
TREE = "a92d66c4998656b1749f322723aa4324028afdc4"
PATCH_SHA = "f8990bc3b6a1aa81ee1a409d25c38f2b68d94839cff5437f08f94b1904671319"
BASE_SOURCE = "b957fec84eb0c5f3574df79c9d7756d1263f7667d0ac0d3b0c3ec8915d49775f"
BASE_TREE = "c694ddfc095f5cc18e3c27b99f7d02985b49abbc"
NATIVE_SEAL = "92f0a0d87713f5764be964e769303279f1d95b04bfe99223cd2492db2d77fe80"
NATIVE_HARNESS_SEAL = "d7b1beca4199e90b48e3d44324f6371dde3bec21ffffd7c6e8c1bea1b333f5f6"
TAG = "0.1.54"
LAST_MATERIAL = 301
SUITES = {"smoke", "ime", "material", "bridge-web", "devtools-headless"}
TRANSPORTS = ["Page.screenshot", "CLI", "MCP"]
NATIVE_IMAGES = {
    "wide-light-sliders.png": [960, 1040],
    "wide-light-focused-slider.png": [960, 1040],
    "compact-dark-rtl-200-sliders.png": [320, 880],
    "compact-dark-rtl-200-xl.png": [320, 880],
}
WEB_IMAGES = {
    "material-standard-slider-wide-light.png": [1920, 2168],
    "material-standard-slider-keyboard.png": [1920, 2168],
    "material-standard-slider-live-drag.png": [1920, 2168],
    "material-standard-slider-compact-dark-rtl-200.png": [640, 1848],
}
MARKERS = ("workspace-v0154-passed", "gpu-v0154-exact-passed", "native-v0154-passed")
LOGS = ("workspace-v0154-exact.log", "gpu-v0154-exact.log")
SUPPLEMENTAL = "gpu-v0154-supplemental.log"
SOURCES = ("source.sha256", "icon-buttons-base-tree", "sliders-tree", "sliders-v0154.patch",
           "actual-sliders.paths", "integrated-slider-production.sha256")
REPORT = "verified-sliders-v0154.json"
STOCK_REPORT = "verified-sliders-stock.json"
OFFLINE_REPORT = "verified-sliders-offline.json"
WEB_REPORT = "material-standard-slider.json"
BROWSER_RECEIPTS = ("headless-window-observation.json", "invocation.json", "inputs-after.json", STOCK_REPORT)
BROWSER_ARTIFACTS = {
    WEB_REPORT, "browser-smoke.json", "browser-ime.json", "browser-ime-cancel-redraw.json",
    "browser-ime-cancel-selected-redraw.json", "browser-material-side-sheet.json",
    "bridge-web-result.json", "devtools-headless-result.json",
    "material-tabs.json", "material-chips.json", "material-icon-buttons.json",
}

def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def read_json(path):
    return json.loads(path.read_text())


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n")


def manifest_entries(path):
    entries = {}
    for line in path.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        relative = Path(name)
        require(re.fullmatch(r"[0-9a-f]{64}", checksum) and name not in entries
                and not relative.is_absolute() and ".." not in relative.parts
                and str(relative) == name, f"Malformed manifest entry in {path}: {name}")
        entries[name] = checksum
    require(entries, f"Empty manifest: {path}")
    return entries


def verify_entries(path, entries):
    for name, checksum in entries.items():
        item = path / name
        require(item.is_file() and not item.is_symlink()
                and item.resolve(strict=True).is_relative_to(path.resolve(strict=True))
                and digest(item) == checksum, f"Different manifest file: {item}")


def object_id(kind, payload):
    header = f"{kind} {len(payload)}\0".encode("ascii")
    return sha1(header + payload).digest()


def source_tree(root, entries):
    """Reconstruct the Git SHA-1 tree without creating any Git objects."""
    top = {}
    for name in entries:
        *parents, basename = Path(name).parts
        directory = top
        for part in parents:
            directory = directory.setdefault(part, {})
            require(isinstance(directory, dict), f"Source file and directory collide: {name}")
        path = root / name
        data = path.read_bytes()
        require(sha256(data).hexdigest() == entries[name], f"Source changed during tree check: {name}")
        mode = b"100755" if path.stat().st_mode & 0o100 else b"100644"
        directory[basename] = (mode, object_id("blob", data))

    def tree_id(directory):
        def sort_key(item):
            name, value = item
            return (name + ("/" if isinstance(value, dict) else "")).encode("utf-8")

        payload = []
        for name, value in sorted(directory.items(), key=sort_key):
            mode, child = (b"40000", tree_id(value)) if isinstance(value, dict) else value
            payload.append(mode + b" " + name.encode("utf-8") + b"\0" + child)
        return object_id("tree", b"".join(payload))

    return tree_id(top).hex()


def copy(source, dest):
    dest.parent.mkdir(exist_ok=True, parents=True)
    copy2(source, dest)
    require(digest(dest) == digest(source), f"File changed while copying: {source}")


def compress(source, dest):
    dest.parent.mkdir(exist_ok=True, parents=True)
    with source.open("rb") as contents, dest.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
            copyfileobj(contents, compressed)
    with gzip.open(dest, "rb") as restored:
        require(file_digest(restored, "sha256").hexdigest() == digest(source),
                f"Compressed bytes differ: {source}")


def seal(directory):
    files = sorted(path for path in directory.rglob("*") if path.is_file())
    manifest = directory / "SHA256SUMS"
    require(not manifest.exists(), "A proof manifest already exists")
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(directory)}\n" for path in files))
    entries = manifest_entries(manifest)
    verify_entries(directory, entries)
    return {"manifest_sha256": digest(manifest), "files": len(entries)}


def verify_seal(directory, expected):
    manifest = directory / "SHA256SUMS"
    require(digest(manifest) == expected, f"Reviewed manifest differs: {directory}")
    entries = manifest_entries(manifest)
    require(set(entries) == {str(path.relative_to(directory)) for path in directory.rglob("*")
                             if path.is_file() and path != manifest}, f"Archive inventory differs: {directory}")
    verify_entries(directory, entries)
    return entries


def verify_source(stage):
    root = stage / "source"
    base = BASE / "source"
    manifest = stage / "source.sha256"
    base_manifest = BASE / "source.sha256"
    require(digest(manifest) == SOURCE_SHA and digest(base_manifest) == BASE_SOURCE,
            "Wrong source or base manifest")
    sources, prior = manifest_entries(manifest), manifest_entries(base_manifest)
    require(len(sources) == 635 and len(prior) == 628, "Wrong source count")
    for directory, rows, tree in ((root, sources, TREE), (base, prior, BASE_TREE)):
        paths = (path.relative_to(directory) for path in directory.rglob("*") if path.is_file())
        require({str(path) for path in paths if path.parts[0] != ".git"} == set(rows),
                f"Source inventory differs: {directory}")
        verify_entries(directory, rows)
        require(source_tree(directory, rows) == tree, f"Wrong source tree: {directory}")
    require((stage / "sliders-tree").read_text().strip() == TREE
            and (stage / "icon-buttons-base-tree").read_text().strip() == BASE_TREE,
            "Candidate or base tree marker differs")
    require(tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == TAG,
            "The workspace is not the exact v0.1.54 candidate")
    patch = stage / "sliders-v0154.patch"
    require(digest(patch) == PATCH_SHA, "The patch digest differs")
    patched = {name.decode() for name in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch.read_bytes(), re.M)}
    changed = {name for name in sources.keys() | prior.keys() if sources.get(name) != prior.get(name)}
    require(patched == changed == set((stage / "actual-sliders.paths").read_text().splitlines()),
            "The patch and recorded changed source paths differ")
    for directory, args in ((base, []), (root, ["--reverse"])):
        subprocess.run(["git", "-C", str(directory), "apply", "--check", "--whitespace=error-all",
                        *args, str(patch)], check=True)
    production = manifest_entries(stage / "integrated-slider-production.sha256")
    require(all(sources.get(name) == value for name, value in production.items()),
            "Final source differs from integrated native production")
    return {"version": TAG, "source_sha256": SOURCE_SHA, "source_files": len(sources), "tree": TREE,
            "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH_SHA,
            "changed_paths": len(changed)}


def test_totals(log, text):
    rows = re.findall(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", text, re.M)
    require(rows and all(kind == "ok" and failed == "0" for kind, _, failed, _ in rows),
            f"Missing or failing tests: {log.name}")
    return {"passed": sum(int(passed) for _, passed, _, _ in rows),
            "ignored": sum(int(ignored) for _, _, _, ignored in rows), "log_sha256": digest(log)}


def verify_rust(stage):
    workspace, metal = (stage / name for name in LOGS)
    text = workspace.read_text()
    ws = test_totals(workspace, text)
    wasm = text.rsplit("\ntest result:", 1)[-1].count("Finished `dev` profile")
    require(ws["passed"] > 0 and wasm == 3 and re.search(r"^provenance: .+ self-test cases passed\)", text, re.M)
            and "Doc-tests surface_material_gallery" in text
            and "closely_spaced_discrete_values_keep_their_exact_logical_placement ... ok" in text,
            "The workspace log lacks the exact Slider, provenance or Wasm checks")
    gpu_text = metal.read_text()
    gpu = test_totals(metal, gpu_text)
    require(gpu["passed"] == 16 and gpu["ignored"] == 0 and "backend: Metal" in gpu_text,
            "The sixteen-case offscreen Metal run is missing")
    result = {"workspace": {**ws, "wasm_checks": wasm}, "metal": {**gpu, "backend": "Metal"}}
    optional = stage / SUPPLEMENTAL
    if optional.is_file():
        extra = optional.read_text()
        successes = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; 0 ignored;", extra, re.M)
        if sum(map(int, successes)) == 2 and "backend: Metal" in extra:
            checks = test_totals(optional, extra)
            require(checks["passed"] == 2 and checks["ignored"] == 0, "Wrong supplemental test count")
            result["supplemental"] = checks
    return result


def native_png(path, checksum, size):
    require(digest(path) == checksum, f"Native PNG bytes changed: {path.name}")
    with Image.open(path) as opened:
        rgb = opened.convert("RGB")
        require(opened.format == "PNG" and list(rgb.size) == size
                and any(lo < hi for lo, hi in rgb.getextrema()), f"Native PNG did not decode: {path.name}")
        return {"size": size, "file_sha256": checksum, "decoded_rgb_sha256": sha256(rgb.tobytes()).hexdigest()}


def verify_native(stage, replay=NATIVE):
    entries = verify_seal(replay, NATIVE_SEAL)
    harness = replay.parent
    require(digest(harness / "SHA256SUMS") == NATIVE_HARNESS_SEAL, "Wrong native replay harness seal")
    verify_entries(harness, manifest_entries(harness / "SHA256SUMS"))
    pins_path = stage / "native-v0154-binaries/SHA256SUMS"
    bins = manifest_entries(pins_path)
    require(set(bins) == {"surface-material-gallery", "surface-devtools"}, "Wrong native executable list")
    verify_entries(pins_path.parent, bins)
    report = read_json(replay / "native-slider-report.json")
    require(report["schema"] == "surface.test.standard-sliders.native.v1"
            and report["rendering"] == "headless" and report["reduce_motion"] is True
            and report["transports"] == TRANSPORTS and report["binaries"] == bins
            and report["native_binary_manifest_sha256"] == digest(pins_path), "Native report provenance differs")
    expected = report["byte_identical_pngs"]
    require(set(expected) == set(NATIVE_IMAGES) and len(report["replays"]) == 2, "Wrong native capture count")
    runs = []
    for run, profiles in enumerate(report["replays"], start=1):
        require(set(profiles) == {"wide", "compact"}, "Native case inventory differs")
        rows = {}
        for profile, dimensions, rtl in (("wide", [960, 1040], False), ("compact", [320, 880], True)):
            row = profiles[profile]
            xs = row["continuous"]["png"]["paint"]["slider-demo-xs"]
            require(row["viewport"] == dimensions and row["rtl"] is rtl
                    and row["text_scale"] == (2 if rtl else 1) and row["visible_windows"] == [0] * 5
                    and row["gallery_reaped"] is True and row["disabled_value"] == 50
                    and row["closed_focus"] == "slider-demo-show"
                    and row["continuous"]["absolute_value"] == 37.25 and row["discrete"]["tie_value"] == 50
                    and xs["focus_pixels"] > 0 and xs["label"]["ink_pixels"] > 0,
                    f"Native Slider profile differs: {profile}")
            require(all([key["value"] for key in row[mode]["keys"]][-4:] == [0, 0, 100, 100]
                        for mode in ("continuous", "discrete")), f"Native endpoints differ: {profile}")
        require(profiles["compact"]["compact_xl"]["scroll_y"] > 0, "Missing compact XL native scroll")
        for filename, size in sorted(NATIVE_IMAGES.items()):
            path = f"replay-{run}/{filename}"
            require(entries.get(path) == expected[filename], f"Native capture derives from a different run: {path}")
            rows[filename] = native_png(replay / path, expected[filename], size)
        runs.append(rows)
    require(runs[0] == runs[1], "The native repeat did not match")
    return {"seal": NATIVE_SEAL, "harness_seal": NATIVE_HARNESS_SEAL, "runs": len(runs),
            "transports": TRANSPORTS, "window_samples": 20, "visible_windows": 0, "binaries": bins,
            "report_sha256": entries["native-slider-report.json"], "captures": runs[0]}


def verify_stock(archive, observer, reviewed_seal, native):
    entries = verify_seal(archive, reviewed_seal)
    for name in ("observe-sliders.py", "verify-existing.py", "archive-proof.py", "prepare-pins.py"):
        require(digest(observer / name) == entries.get(name), f"Wrong stock observer helper: {name}")
    checked = subprocess.run([sys.executable, "-E", "-B", str(observer / "archive-proof.py"),
                              "--verify", str(archive)], check=True, capture_output=True, text=True)
    replay = json.loads(checked.stdout)
    receipt = read_json(archive / OFFLINE_REPORT)
    pin = receipt["release"]
    run = archive / "run"
    raw = read_json(run / STOCK_REPORT)
    windows = read_json(run / BROWSER_RECEIPTS[0])
    require(replay["manifest_sha256"] == reviewed_seal
            and replay["source_sha256"] == pin["source_sha256"] == SOURCE_SHA
            and digest(archive / "browser-source.sha256") == SOURCE_SHA
            and pin["version"] == TAG and pin["source_files"] == 635 and pin["last_material_step"] == LAST_MATERIAL
            and manifest_entries(archive / "native-binaries.sha256") == native["binaries"],
            "Actual stock input does not match the sealed source and native pins")
    require(set(receipt["cases"]) == SUITES and receipt["cases"]["ime"] == {"steps": 20, "checks": 77}
            and receipt["cases"]["material"]["steps"] == 300 and raw["page_errors"] == 0
            and raw["total_checks"] == replay["checks"] == receipt["total_checks"],
            "The result lacks complete five-suite stock behavior")
    adapter = receipt["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False, "The recorded stock adapter is not Apple Metal")
    require(windows["stock_exit"] == 0 and
            windows["visible_owned_windows"] == windows["remaining_observed_processes"] == windows["problems"] == [],
            "The stock observer recorded a failure, window or surviving process")
    require(replay["window_receipt_sha256"] == receipt["window_receipt_sha256"]
            == digest(run / BROWSER_RECEIPTS[0]), "Stock window receipt differs")
    shots = receipt["sliders_pngs"]
    require(set(shots) == set(WEB_IMAGES)
            and all(shots[name]["size"] == WEB_IMAGES[name]
                    and shots[name]["sha256"] == entries[f"run/artifacts/{name}"] for name in shots),
            "The four original Slider Chrome PNGs differ")
    require(len({shot["decoded_rgb_sha256"] for shot in shots.values()}) == 4,
            "The Chrome Slider screenshots share decoded pixels")
    pixels = receipt["sliders_pixels"]
    require(all(type(pixels[key]) is int and pixels[key] > 0 for key in (
                "tick_pixels", "focus_pixels", "key_label_ink", "key_label_fill",
                "compact_label_ink", "compact_label_fill")), "The decoded Slider paint evidence is incomplete")
    for filename in BROWSER_ARTIFACTS:
        require(f"run/artifacts/{filename}" in entries, f"Missing retained stock report: {filename}")
    require(entries[f"run/artifacts/{WEB_REPORT}"] == receipt["sliders_report_sha256"],
            "Stock Slider state report differs")
    return {"seal": reviewed_seal, "files": replay["files"], "checks": receipt["total_checks"],
            "adapter": adapter, "cases": receipt["cases"], "screenshots": shots,
            "report_sha256": receipt["sliders_report_sha256"], "sliders_pixels": pixels}


def copy_native(stage, replay, directory):
    copytree(replay, directory)
    verify_seal(directory, NATIVE_SEAL)
    copy(directory / "SHA256SUMS", directory / "original-replay.sha256")
    # The copied proof remains covered by this archive's outer manifest.
    for name in ("replay-standard-sliders.py", "gallery_transport.py", "native_runtime.py",
                 "render_helpers.py", "window_check.py", "helpers.sha256", "pins.json",
                 "exact-metal.log", "README.md", "SHA256SUMS"):
        copy(replay.parent / name, directory / "harness" / name)
    copy(stage / "native-v0154-binaries/SHA256SUMS", directory / "native-binaries.sha256")
    copy(stage / "native-v0154-build.log", directory / "build.log")


def readme(directory, verified, full_seal):
    rust, stock = verified["rust"], verified["stock_macos"]
    lines = [
        "# Surface Material 3 Standard Slider v0.1.54 candidate",
        "",
        f"The 635 checked source paths form tree `{TREE}`.",
        f"The source patch applies to the exact Icon Button tree `{BASE_TREE}`.",
        "",
        f"The workspace passed {rust['workspace']['passed']:,} Rust tests, provenance, fmt, Clippy and three Wasm checks.",
        f"All {rust['metal']['passed']} official offscreen Metal cases passed.",
    ]
    if "supplemental" in rust:
        lines.append(f"The separately recorded supplemental check passed {rust['supplemental']['passed']} tests.")
    lines += [
        "",
        "Two native headless recurrences each captured four original PNGs.",
        "The API, CLI and MCP returned the same bytes, and each image matched across both recurrences.",
        "The native observer recorded 20 samples with zero visible windows.",
        "",
        f"Five stock macOS headless cases passed {stock['checks']:,} checks using Apple Metal.",
        "They completed 300 Material steps through M301 and 77 IME checks.",
        "The observer recorded no page errors, visible windows or surviving owned processes.",
        "",
        "## Original screenshots",
        "",
    ]
    for name in NATIVE_IMAGES:
        lines.append(f"- [Native {name}](native/replay-1/{name})")
    for name in WEB_IMAGES:
        lines.append(f"- [Chrome {name}](stock-macos/artifacts/{name})")
    lines += [
        "",
        "## Receipts and replay",
        "",
        f"- [Exact candidate receipt]({REPORT})",
        "- [Native action and pixel report](native/native-slider-report.json)",
        f"- [Stock offline receipt](stock-macos/{OFFLINE_REPORT})",
        f"- [Chrome behavior report](stock-macos/artifacts/{WEB_REPORT})",
        "- [Stock process observation](stock-macos/headless-window-observation.json)",
        "",
        "Verify this copy with `shasum -a 256 -c SHA256SUMS`.",
        "Decompress the source patch and apply it to the recorded Icon Button tree.",
        "Verify the reconstructed files against `source.sha256`.",
        "Use the scripts in the full archive to replay the retained stock regression checks.",
        "",
        f"The full local archive manifest digest is `{full_seal}`.",
        f"The sealed stock input manifest digest is `{stock['seal']}`.",
        "",
        "Coverage is partial. Hosted Linux, additional Slider forms and live screen-reader speech have separate gates.",
        "",
    ]
    (directory / "README.md").write_text("\n".join(lines))


def export(stage, replay, archive, out, verified):
    require(not out.exists() and out.parent.resolve(strict=True).is_relative_to(Path("/tmp").resolve()),
            "Choose a fresh output directory below /tmp")
    out.mkdir(mode=0o700)
    full, compact = (out / name for name in ("full", "candidate"))
    full.mkdir()
    for file in (*SOURCES, *MARKERS):
        copy(stage / file, full / file)
    copy(BASE / "source.sha256", full / "icon-buttons-base.sha256")
    require(digest(full / "source.sha256") == SOURCE_SHA and digest(full / "sliders-v0154.patch") == PATCH_SHA,
            "Candidate source or patch changed while copying")
    logs = [(LOGS[0], "workspace"), (LOGS[1], "metal")]
    if "supplemental" in verified["rust"]:
        logs.append((SUPPLEMENTAL, "supplemental"))
    for name, family in logs:
        target = full / "rust" / name
        copy(stage / name, target)
        require(digest(target) == verified["rust"][family]["log_sha256"], "A copied test log changed")
    for filename in ("workspace-source-before.log", "workspace-source-after.log",
                     "gpu-source-before.log", "gpu-source-after.log"):
        if (stage / filename).is_file():
            copy(stage / filename, full / "rust" / filename)
    for image in sorted((stage / "gpu-v0154-exact").glob("*.png")):
        copy(image, full / "gpu" / image.name)
    copy_native(stage, replay, full / "native")
    browser = full / "stock-macos"
    copytree(archive, browser)
    verify_seal(browser, verified["stock_macos"]["seal"])
    copy(Path(__file__), full / "pack-sliders.py")
    write_json(full / REPORT, verified)
    full_seal = seal(full)

    compact.mkdir()
    for name in ("source.sha256", "icon-buttons-base.sha256", "icon-buttons-base-tree", "sliders-tree",
                 "actual-sliders.paths", "integrated-slider-production.sha256", REPORT, "pack-sliders.py"):
        copy(full / name, compact / name)
    copy(full / "SHA256SUMS", compact / "full-local-archive.sha256")
    compress(full / "sliders-v0154.patch", compact / "surface-v0.1.54.patch.gz")
    copytree(full / "native", compact / "native")
    for name in MARKERS:
        copy(full / name, compact / "rust" / name)
    for log in sorted((full / "rust").glob("*.log")):
        compress(log, compact / "rust" / f"{log.name}.gz")
    for example in ("gpu-shapes.png", "gpu-offscreen.png", "gpu-motion-midpoint.png"):
        if (full / "gpu" / example).is_file():
            copy(full / "gpu" / example, compact / "gpu" / example)

    stock = compact / "stock-macos"
    copy(browser / "SHA256SUMS", stock / "full-stock-archive.sha256")
    for name in (OFFLINE_REPORT, "browser-source.sha256", "release-pin.json", "build-markers.json",
                 "native-binaries.sha256", "browser-binary-sha256.txt", "inputs-before.json",
                 "approved-runner.sha256", "source-origins.json", "sealed-inputs.sha256"):
        copy(browser / name, stock / name)
    for name in BROWSER_RECEIPTS:
        copy(browser / "run" / name, stock / name)
    copytree(browser / "run/guard-ledger", stock / "guard-ledger")
    compress(browser / "run/runner.log", stock / "runner.log.gz")
    for name in sorted(set(WEB_IMAGES) | BROWSER_ARTIFACTS):
        copy(browser / "run/artifacts" / name, stock / "artifacts" / name)
    readme(compact, verified, full_seal["manifest_sha256"])
    return {"full": {"path": str(full), **full_seal}, "compact": {"path": str(compact), **seal(compact)}}


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--stage", type=Path, default=STAGE)
    parser.add_argument("--native", type=Path, default=NATIVE)
    parser.add_argument("--observer", type=Path, default=OBSERVER)
    parser.add_argument("--stock-archive", type=Path, default=OBSERVER / "portable-proof")
    parser.add_argument("--stock-seal", help="SHA-256 printed when the real stock observer archive passes")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true", help="verify inputs without writing")
    mode.add_argument("--out", type=Path, help="write both sealed archives to a new /tmp path")
    args = parser.parse_args()
    try:
        stage = args.stage.resolve(strict=True)
        source = verify_source(stage)
        native = verify_native(stage, args.native)
        missing = [name for name in (*MARKERS, *LOGS) if not (stage / name).is_file()]
        if not (args.stock_archive / "SHA256SUMS").is_file():
            missing.append(str(args.stock_archive / "SHA256SUMS"))
        if not args.stock_seal:
            missing.append("--stock-seal")
        else:
            require(re.fullmatch(r"[0-9a-f]{64}", args.stock_seal), "The approved stock seal is malformed")
        if missing:
            print(json.dumps({"status": "pending", "source": source, "native": native,
                              "missing": missing}, indent=2))
            return 2
        rust = verify_rust(stage)
        stock = verify_stock(args.stock_archive, args.observer, args.stock_seal, native)
        verified = {"status": "verified", "source": source, "native": native, "rust": rust, "stock_macos": stock}
        if args.out:
            verified["outputs"] = export(stage, args.native, args.stock_archive, args.out, verified)
        print(json.dumps(verified, indent=2, ensure_ascii=False))
        return 0
    except subprocess.CalledProcessError as error:
        print(f"Rejected offline check ({error.returncode}). {(error.stderr or '').strip()}", file=sys.stderr)
        return 1
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"Rejected evidence. {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
