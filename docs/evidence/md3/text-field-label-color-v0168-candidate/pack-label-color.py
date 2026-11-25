#!/usr/bin/env python3
"""Recheck and package Surface v0.1.67 headless proof without running the product."""

import argparse
from hashlib import sha256
import json
import os
from pathlib import Path, PurePosixPath
import re
from runpy import run_path
import subprocess
import sys
import tomllib

from PIL import Image

from evidence_common import (
    HEX, clean_windows, compressed, copy, copy_members, data, digest, import_from,
    json_file, normalized, path_entries, raw_log, require, run_json, seal,
    source_tree, tests, verify_entries, verify_ledgers,
)

HERE = Path(__file__).resolve().parent
Q = Path("/tmp/surface-md3-text-field-label-color-v0168.XNrxeeOp")
EXACT = Q / "exact-candidate"
PREVIOUS = Path("/tmp/surface-md3-field-motion-v0167-package.Z0NY1Ilr")
PREVIOUS_SOURCES = Path("/tmp/surface-md3-text-field-label-motion-v0167.Y7f9HqIi/exact-candidate")
PREVIOUS_PUBLIC = Path("/tmp/surface-md3-field-motion-v0167-package.Z0NY1Ilr/local-proof.json")
STOCK = Path("/tmp/surface-md3-field-label-color-stock-v0168.pxiIhOFe")
FOCUSED = Path("/tmp/surface-md3-label-color-focused-v0168.Ebk8jXAK")
NATIVE_HARNESS = Path("/tmp/surface-md3-multiline-native.cZ38fMkb")
SOURCE = "17aeee497edef781a3da477d8fdb7820c0eaa3494a4b840fcf41133925949df6"
TREE = "0db939d3f663fdb5b9fa3dd334382937693b409e"
BASE_SOURCE = "4aa53964b34f033b59b5ab723e58deb612d5db8ce1975bd3cf2115e687479271"
BASE_TREE = "c218faaf7527ec2872fb1ec1097c87667f3bf1f2"
PATCH_NAME = "text-field-label-color-v0168.patch"
PATCH = "05814fa4065bf7b65ce1b85023ae79623fafc0abb4870555368b71153c3463b9"
FEATURE = "material-text-field-label-motion"
FEATURE_PATH = "e2e/src/material_text_field_label_motion.rs"
FEATURE_SOURCE = "44b422b5f7c1be3fe517153c0867385cf295f9268a0242b765241d4868c72e55"
FOCUSED_TOOLS = "caf6ca8f564a3adec29f1990b055c24287d2a947b88567696b29ca8ece069e4c"
FOCUSED_DEVELOPMENT = "e7c6c189cd48f31257bf8b7de66da02ca9201d58a566544e741651cec4f1b006"
WEB_DEVELOPMENT = "dfc0e717a5a5641c07b63ae3cbf140797fd39cabf3197cd7c93509cd324b0dd0"
NATIVE_DEV_REPLAY = "e278604e4e19090ca8ad4fde4af041b42bda2111410c10fa0676688f45d680ac"
NATIVE_DEV_REPORT = "9bb81ee887cee9ef5d452236c5632d14454eb4e2f499e6ae80ea1b8a16e46fde"
FOCUSED_VERIFIED = "53826be28364c00eb75b08ebefcaebff6c425303270c304aececc5c6efeb4d0a"
FOCUSED_RUNNER = "db598f4bff0236fba17a9aadc0f509f3427877633d86dda49db4b0131c82a28f"
FOCUSED_CODE = "98062ecd67a97a3e7346e97c2321b55ba0ae319c80c8bfcf5a778180d134fecc"
FOCUSED_WEB = "6d3e736df6dd52d1429ccc67700351e1635a8b2ee54a1732e47c0ca01a31bc7f"
STOCK_HARNESS = "66780043b0f9ecbe4c6632487a9b09cb5a49ca53d04e98d68bf890d1341d195d"
PREVIOUS_SEAL = "184b9d871ee954547cbcefef551585c604e0e80b0d4262c305ba3cb46bdfd70b"
PREVIOUS_RESULT = "73c5127194c9594020779bfa59a7c4f534ccd07e6d43f9cc7928749b6684c707"
PREVIOUS_OFFLINE = "e60dd517932766d4e28569649063e89caf7aac6535441e085726f650a9267038"
PREVIOUS_STOCK_NAME = "stock-mac-exact-offline.json"
PREVIOUS_PUBLISHED = "d74ad36e0c588a4a43fb3a43f11d1ce5fe7918f4"
PREVIOUS_PUBLISHED_TREE = "272bbf4c94d80f805934cb9e409f8aa285c843d0"
PREVIOUS_PUBLIC_URL = f"https://github.com/faberline/surface/tree/{PREVIOUS_PUBLISHED}/docs/evidence/md3/text-field-label-motion-v0167-candidate"
NATIVE_BIN = "947a102223c4fb7ea59f7c7b990a8ffc217a6d26a3180509b4cc49d6005e1ad7"
PREVIOUS_NATIVE = "0245dd18654a31723635c4942f1cc4ade1f5d4c85b71d297f46e0b8150e3018e"
NATIVE_HARNESS_SHA = "c842ea6ac04a2428ac239bf4ec6357d2912603f9cd5b3decd1029a9e0b1b1f35"
STOCK_RESULT = "verified-label-color-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
REPORT = "verified-label-color-v0168.json"
WINDOW = "headless-window-observation.json"
UPDATER = "--disable-updater-scheduler"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
ADAPTER = {"vendor": "apple", "architecture": "metal-3", "isFallbackAdapter": False}
RUST_LOGS = ("workspace-v0168-exact.log", "gpu-v0168-exact.log", "gpu-v0168-supplemental.log")
READ_MARKERS = {
    "workspace-v0168-passed": Q / "workspace-v0168-passed",
    "gpu-v0168-passed": Q / "gpu-v0168-passed",
    "native-v0168-passed": Q / "native-v0168-passed",
    "native-final-built": EXACT / "native-final-built",
    "stock-canonical-built": EXACT / "stock-canonical-built",
    "stock-mac-v0168-passed": STOCK / "stock-mac-v0168-passed",
}


def pinned_source(base, exact, patch):
    require(digest(base) == BASE_SOURCE and digest(exact) == SOURCE
            and sha256(patch).hexdigest() == PATCH, "Wrong source or original patch digest")
    before, after = path_entries(base), path_entries(exact)
    require((len(before), len(after)) == (684, 685), "Wrong prior or exact path count")
    paths = re.findall(r"^diff --git a/(\S+) b/\S+$", patch.decode(), re.M)
    require(len(paths) == len(set(paths)) == 10
            and set(paths) == {p for p in before.keys() | after.keys() if before.get(p) != after.get(p)}
            and after[FEATURE_PATH] == FEATURE_SOURCE, "Wrong patch path scope or feature test")
    return {"version": "0.1.68", "tree": TREE, "source_sha256": SOURCE, "source_files": len(after),
            "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
            "changed_paths": sorted(paths)}


def source_info():
    patch = Q / PATCH_NAME
    summary = pinned_source(PREVIOUS_SOURCES / "source.sha256", EXACT / "source.sha256", patch.read_bytes())
    for stage, tree, reverse in ((PREVIOUS_SOURCES, BASE_TREE, []), (EXACT, TREE, ["--reverse"])):
        source = stage / "source"
        manifest = path_entries(stage / "source.sha256")
        actual = {file.relative_to(source).as_posix() for file in source.rglob("*")
                  if file.is_file() and file.relative_to(source).parts[0] != ".git"}
        require(actual == manifest.keys(), "Wrong on-disk source inventory")
        verify_entries(source, manifest)
        require(source_tree(source, manifest) == tree, "Wrong exact source tree")
        subprocess.run(["git", "-C", str(source), "apply", "--check", "--whitespace=error-all",
                        *reverse, str(patch)], check=True, env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"})
    version = tomllib.loads((EXACT / "source/Cargo.toml").read_text())["workspace"]["package"]["version"]
    registration = (EXACT / "source/e2e/src/material.rs").read_text().split("pub const STEPS:", 1)[1].split("];", 1)[0]
    require(version == "0.1.68"
            and [int(step) for step in re.findall(r'"M(\d+)"', registration)] == [i for i in range(1, 351) if i != 23]
            and (Q / "exact-apply-checked").read_text().strip() == TREE
            and (Q / "source-tree").read_text().strip() == TREE and (Q / "prior-tree").read_text().strip() == BASE_TREE,
            "Wrong release version, exact replay stamp or registered steps")
    return summary


def prior_info(root):
    receipt = root / "verified-label-motion-v0167.json"
    offline = root / PREVIOUS_STOCK_NAME
    published = data(root / "published.json")
    manifest = root / "package-v0167.sha256"
    require(digest(receipt) == PREVIOUS_RESULT and digest(offline) == PREVIOUS_OFFLINE
            and digest(manifest) == PREVIOUS_SEAL, "Wrong retained v0.1.67 proof files")
    entries = path_entries(manifest)
    require(entries[receipt.name] == PREVIOUS_RESULT
            and entries[f"stock-macos/{PREVIOUS_STOCK_NAME}"] == PREVIOUS_OFFLINE
            and published == {"commit": PREVIOUS_PUBLISHED, "tree": PREVIOUS_PUBLISHED_TREE,
                              "manifest_sha256": PREVIOUS_SEAL, "url": PREVIOUS_PUBLIC_URL},
            "Retained v0.1.67 archive or commit changed")
    report = data(receipt)
    require(report["source"]["tree"] == BASE_TREE and report["source"]["source_sha256"] == BASE_SOURCE
            and report["stock_macos"]["total_checks"] == data(offline)["total_checks"] == 24973
            and report["stock_macos"]["cases"]["material"]["steps"] == 349
            and report["stock_macos"]["visible_windows"] == report["stock_macos"]["surviving_processes"] == 0
            and len(data(offline)["textbox_values"]["inactive_ax"]) == 6
            and len(data(offline)["text_field_label_motion"]["pngs"]) == 15
            and report["focused"]["three_identical_png_runs"] == 3
            and len(report["focused"]["negative_guard_receipts"]) == 2,
            "Retained predecessor AX, suites or counts disagree")
    return {"tree": BASE_TREE, "source_sha256": BASE_SOURCE, "published": published,
            "report_sha256": PREVIOUS_RESULT, "stock_offline_sha256": PREVIOUS_OFFLINE,
            "stock_checks": 24973, "material_steps": 349}


def changed(a, b):
    require(a.keys() == b.keys() and len(a) == 685, "Wrong scoped manifest inventory")
    return sorted(name for name in a if a[name] != b[name])


def focused_info(root, tools, exact):
    original = root / "original-source.sha256"
    selected = root / "focused-source.sha256"
    development_web = root / "preflight-r1/original-source.sha256"
    seal(root, "SHA256SUMS", FOCUSED_TOOLS)
    require(digest(original) == FOCUSED_DEVELOPMENT and digest(selected) == FOCUSED_CODE
            and digest(development_web) == WEB_DEVELOPMENT and digest(exact) == SOURCE
            and digest(root / "web.sha256") == FOCUSED_WEB
            and (root / "runner.sha256").read_text().strip() == FOCUSED_RUNNER
            and (root / "pins/focused-source-manifest.sha256").read_text().strip() == FOCUSED_CODE,
            "Wrong scoped receipt source and asset pins")
    final, dev, selector, web_src = map(path_entries, (exact, original, selected, development_web))
    feature_paths = ["docs/material-coverage.json"]
    require(changed(final, dev) == feature_paths and changed(dev, selector) == ["e2e/src/material.rs"]
            and changed(dev, web_src) == [FEATURE_PATH] and final[FEATURE_PATH] == FEATURE_SOURCE,
            "Wrong exact versus original, scoped selector or Web build delta")
    run = root / "development-r2"
    verifier = run_path(str(tools / "verify_label_color.py"))
    raw = verifier["verify_artifacts"](run / "artifacts")
    expected = data(root / "development-r2-verified.json")
    require(digest(root / "development-r2-verified.json") == FOCUSED_VERIFIED
            and raw == {key: expected[key] for key in raw}
            and len(raw["captures"]) == 34 and raw["adapter"] == ADAPTER, "Wrong real scoped RGBA or original report")
    steps = [f"M{number}" for number in range(325, 351)]
    log = (run / "runner.log").read_text()
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"]
            and re.findall(r"^  \[(M\d+)\]", log, re.M) == steps == expected["steps"]
            and re.findall(r"^case material \.\.\. ok \(26 steps, (\d+) checks,", log, re.M) == ["1798"]
            and re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", log, re.M) == ["1798"]
            and expected["checks"] == 1798 and expected["runner_log_sha256"] == digest(run / "runner.log"),
            "Wrong scoped original Rust test ledger")
    window = data(run / WINDOW)
    clean_windows(window)
    verify_ledgers(run, window, {"chrome-primary"})
    event, = window["guard_events"]
    launch = data(run / "invocation.json")
    settings = data(root / "scope-pin.json")
    gallery = data(root / "gallery-pin.json")
    originals = path_entries(root / "web.sha256")
    require(len(originals) == 4 and gallery["web_files"] == originals
            and gallery["web_manifest_sha256"] == FOCUSED_WEB
            and gallery["web_build_source_manifest_sha256"] == WEB_DEVELOPMENT
            and gallery["web_compatible_test_only_delta"] == [FEATURE_PATH]
            and settings["product_source_manifest_sha256"] == FOCUSED_DEVELOPMENT
            and settings["focused_source_manifest_sha256"] == FOCUSED_CODE
            and settings["source_count"] == 685 and settings["registry_delta"] == ["e2e/src/material.rs"]
            and settings["test_source_sha256"] == FEATURE_SOURCE and settings["steps"] == steps,
            "Raw source and Gallery pin metadata differs")
    inputs = {**settings, **gallery, "runner_sha256": FOCUSED_RUNNER}
    argv = launch["argv"]
    focus_root = Path(argv[0]).parents[1]
    recorded = Path(argv[argv.index("--artifacts") + 1]).parent
    require(focus_root == Path("/private/tmp") / FOCUSED.name and recorded == focus_root / "development-r2"
            and argv == [str(focus_root / "pins/surface-e2e"), "--chrome", str(recorded / "guards/chrome"),
                         "--chrome-arg", UPDATER, "--web-root", str(focus_root / "web"),
                         "--artifacts", str(recorded / "artifacts"), "material"]
            and event["argv"].count(UPDATER) == 1 and Path(event["profile"]).is_relative_to(recorded / "tmp")
            and launch["inputs"] == data(run / "inputs-after.json") == expected["inputs"] == inputs
            and launch["scope"] == expected["scope"] == "q68 focused M325–M350 development"
            and launch["suite_arguments"] == ["material"] and launch["injected_chrome_flags"] == [UPDATER]
            and expected["window_receipt_sha256"] == digest(run / WINDOW)
            and expected["headless_samples"] == window["launch_observations"]["chrome-primary"]["samples"] == 314
            and expected["visible_windows"] == 0 and expected["remaining_owned_processes"] == []
            and data(run / "owned-shutdown.json")["after_grace"]
            == data(run / "owned-shutdown.json")["after_cleanup"] == [],
            "Wrong scoped exact invocation, trusted guard or process shutdown result")
    return {**expected, "steps": 26, "step_ids": steps, "gallery_files": originals,
            "exact_source_sha256": SOURCE, "exact_vs_scoped_source_paths": feature_paths,
            "same_m347_to_m350_source_sha256": FEATURE_SOURCE, "focused_receipt_sha256": FOCUSED_VERIFIED}


def native_info(replay, harness, binaries, receipt, prior_capture, manifest, receipt_sha, source_sha):
    require(digest(receipt) == receipt_sha and digest(binaries) == NATIVE_BIN
            and digest(prior_capture) == PREVIOUS_NATIVE, "Wrong exact native seal or predecessor captures")
    entries = seal(replay, "SHA256SUMS", manifest, exact=True)
    seal(harness, "harness.sha256", NATIVE_HARNESS_SHA)
    result = run_json([harness / "verify-stored-multiline.py", replay, "--manifest-sha", manifest,
                       "--source-manifest-sha", source_sha, "--binary-manifest-sha", NATIVE_BIN])
    require(result == data(receipt) and
            (result["repeats"], result["original_pngs"], result["visible_owned_windows"],
             result["observed_window_samples"], result["reaped_gallery_processes"]) == (2, 14, 0, 24, 4),
            "Wrong real native headless scope")
    actual = {name: value for name, value in entries.items() if name.endswith(".png")}
    previous = native_pngs(prior_capture)
    require(len(actual) == 14 and actual == previous, "Native reduced-motion endpoints differ from .67")
    return {**result, "matching_v0167": 14, "transports": ["Page.screenshot", "CLI", "MCP"]}


def native_pngs(manifest):
    return {name: value for name, value in path_entries(manifest).items() if name.endswith(".png")}


def gpu_info(root, manifest_sha):
    manifest = root / "SHA256SUMS"
    require(digest(manifest) == manifest_sha, "Wrong actual Metal image seal")
    entries = {name.removeprefix("./"): value for value, name in
               (line.split("  ", 1) for line in manifest.read_text().splitlines())}
    require(len(entries) == 21 and set(entries) == {p.name for p in root.glob("*.png")}
            and all(re.fullmatch(r"gpu-[a-z-]+\.png", n) and HEX.fullmatch(value) for n, value in entries.items()),
            "Wrong Metal originals inventory")
    verify_entries(root, entries)
    for name in entries:
        with Image.open(root / name) as image:
            require(image.format == "PNG" and min(image.size) > 0, f"Wrong Metal original {name}")
            image.verify()
    return entries


def rust_info(root, pins, archived):
    records = []
    for name, wanted in zip(RUST_LOGS, (pins["workspace_log"], pins["gpu_log"], pins["gpu_supplemental_log"]), strict=True):
        path = root / (name + ".gz" if archived else name)
        require(sha256(raw_log(path)).hexdigest() == wanted, f"Wrong exact log: {name}")
        records.append(tests(path))
    (workspace, record), (primary_log, primary), (extra_log, extra) = records
    after_tests = workspace.rsplit("\ntest result:", 1)[-1]
    new_tests = (
        "label_color_retargets_on_focus_and_error_without_changing_its_pose",
        "color_uses_the_fast_effects_reference_curve_and_exact_endpoints",
        "reversing_and_retargeting_keep_the_continuous_pose_and_velocity",
        "alpha_has_its_own_track_and_invalid_clocks_or_snap_reset_it",
    )
    require((record["passed"], record["ignored"]) == (pins["workspace_tests"], 20)
            and after_tests.count("Finished \x60dev\x60 profile") == 3
            and re.search(r"^provenance: .+ self-test cases passed\)", workspace, re.M)
            and all(f"{name} ... ok" in workspace for name in new_tests),
            "Wrong exact Rust, provenance or three Wasm checks")
    require(primary["passed"] == 16 and primary["ignored"] == 0
            and extra["passed"] == 2 and extra["ignored"] == 0
            and "backend: Metal" in primary_log and extra_log.count("backend: Metal") == 2
            and 'name: "Apple M1 Max"' in primary_log, "Wrong actual Apple Metal tests")
    return {"workspace": {**record, "wasm_checks": 3}, "metal": {**primary, "backend": "Metal"},
            "supplemental": {**extra, "backend": "Metal"}, "metal_total_passed": 18}


def native_live(pins):
    return native_info(
        Q / "native-exact-replay", NATIVE_HARNESS, EXACT / "binaries/SHA256SUMS",
        Q / "native-v0168-verified.json", PREVIOUS_SOURCES.parent / "native-exact-replay/SHA256SUMS",
        pins["native_manifest"], pins["native_receipt"], SOURCE,
    )


def native_development():
    return native_info(
        Q / "native-development-r2-replay", NATIVE_HARNESS, Q / "native-development-r2/binaries/SHA256SUMS",
        Q / "native-development-r2-verified.json", PREVIOUS_SOURCES.parent / "native-exact-replay/SHA256SUMS",
        NATIVE_DEV_REPLAY, NATIVE_DEV_REPORT, FOCUSED_DEVELOPMENT,
    )


def final_gate_pins(value):
    hashes = ("workspace_log", "gpu_log", "gpu_supplemental_log", "gpu_manifest",
              "native_manifest", "native_receipt", "stock_runner", "stock_offline")
    require(set(value) == {*hashes, "workspace_tests", "source_sha256", "tree"}
            and all(isinstance(value[name], str) and HEX.fullmatch(value[name]) for name in hashes)
            and type(value["workspace_tests"]) is int and value["workspace_tests"] == 1575
            and value["source_sha256"] == SOURCE and value["tree"] == TREE,
            "Reviewed terminal gate pins are incomplete or belong to a different candidate")
    return value


def verified_prior_live():
    original = run_json([PREVIOUS / "pack-label-motion.py", "--verify-package", PREVIOUS / "portable-r2",
                         "--package-sha", PREVIOUS_SEAL])
    require(original == data(PREVIOUS / "portable/verified-label-motion-v0167.json"),
            "Prior exact portable archive differs")


def pending_markers():
    pending = []
    for name, path in READ_MARKERS.items():
        if not path.is_file():
            pending.append(name)
        else:
            expected = b"" if name == "stock-mac-v0168-passed" else (TREE + "\n").encode()
            require(path.read_bytes() == expected, f"Wrong exact marker {name}")
    return pending


def observed_rust_pins():
    _, workspace = tests(Q / RUST_LOGS[0])
    return {
        "workspace_tests": workspace["passed"],
        "workspace_log": digest(Q / RUST_LOGS[0]),
        "gpu_log": digest(Q / RUST_LOGS[1]),
        "gpu_supplemental_log": digest(Q / RUST_LOGS[2]),
        "gpu_manifest": digest(Q / "gpu-v0168-exact/SHA256SUMS"),
    }


def observed_native_pins():
    return {
        "native_manifest": digest(Q / "native-exact-replay/SHA256SUMS"),
        "native_receipt": digest(Q / "native-v0168-verified.json"),
    }


def observed_final_pins():
    pending = pending_markers()
    require(not pending, "Pending exact gates: " + ", ".join(pending))
    return final_gate_pins({
        "tree": TREE,
        "source_sha256": SOURCE,
        **observed_rust_pins(),
        **observed_native_pins(),
        "stock_runner": (STOCK / "approved-runner.sha256").read_text().strip(),
        "stock_offline": digest(STOCK / OFFLINE),
    })


class ArchivedRun:
    """Read package files locally, and check profile containment with the recorded run root."""

    def __init__(self, directory, recorded_root):
        self.directory = directory
        self.recorded_root = recorded_root

    def __truediv__(self, entry):
        return self.directory / entry

    def resolve(self):
        return self.recorded_root


def stock_info(pins, run, runner_sha, offline_sha, focused):
    seal(pins, "harness.sha256", STOCK_HARNESS)
    with import_from(pins):
        observer = run_path(str(pins / "observe-multiline.py"))
    require((observer["SOURCE_SHA"], observer["SOURCE_TREE"], observer["SOURCE_FILES"],
             observer["LAST_MATERIAL"], observer["CHROME_FLAGS"]) == (SOURCE, TREE, 685, 350, (UPDATER,)),
            "Wrong retained stock verifier")
    verdict = data(run / STOCK_RESULT)
    require(HEX.fullmatch(offline_sha or "") and digest(pins / OFFLINE) == offline_sha,
            "Wrong reviewed stock offline receipt")
    raw = data(pins / OFFLINE)
    before, after = data(pins / "inputs-before.json"), data(run / "inputs-after.json")
    invocation = data(run / "invocation.json")
    watch = data(run / WINDOW)
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary", "chrome-bridge", "native"})
    require(watch["launch_observations"]["chrome-bridge"]["detached_samples"] > 0
            and all(event["argv"].count(UPDATER) == 1 for event in watch["guard_events"] if event["profile"]),
            "Detached Chrome or launch-only flag was lost")
    require(before == after == invocation["inputs"] and tuple(invocation["suite_arguments"]) == SUITES
            and invocation["injected_chrome_flags"] == [UPDATER]
            and after["release"] == raw["release"] == data(pins / "release-pin.json")
            and after["source_manifest_sha256"] == digest(pins / "browser-source.sha256") == SOURCE
            and len(path_entries(pins / "browser-source.sha256")) == after["source_files_checked_per_origin"] == 685
            and after["release"] == dict(version="0.1.68", source_sha256=SOURCE, source_files=685, source_tree=TREE,
                                         last_material_step=350, wide_canvas_height=800, rtl_canvas_height=1020),
            "Stock source, build or launch pins disagree")
    expected_marker_sha = sha256((TREE + "\n").encode()).hexdigest()
    markers = data(pins / "build-markers.json")
    require(set(markers) == {"native", "web"} and all(v["sha256"] == expected_marker_sha for v in markers.values())
            and len({v["path"] for v in markers.values()}) == 2, "Wrong build marker identity")
    seal(pins, "sealed-inputs.sha256", after["app_bundle_seal_sha256"])
    binary = path_entries(pins / "browser-binary-sha256.txt")
    require(HEX.fullmatch(runner_sha or "") and runner_sha == after["runner_sha256"] == binary["debug/surface-e2e"]
            == (pins / "approved-runner.sha256").read_text().strip()
            and after["stock_files"] == binary and len(binary) == after["bundle_files_checked"] == 16
            and digest(pins / "native-binaries.sha256") == NATIVE_BIN,
            "Stock executable or binary manifest differed")
    gallery = {PurePosixPath(name).name: sha for name, sha in binary.items()
               if PurePosixPath(name).parent == PurePosixPath("web/release/material-gallery")}
    receipt = data(pins / "focused-proof.json")
    require(after["focused_proof_sha256"] == digest(pins / "focused-proof.json") and raw["focused"] == receipt
            and receipt["inputs"]["runner_sha256"] == FOCUSED_RUNNER and receipt["focused_receipt_sha256"] == FOCUSED_VERIFIED
            and all(receipt[key] == focused[key] for key in ("checks", "steps", "adapter", "captures", "report_sha256"))
            and len(gallery) == 4 and receipt["canonical_gallery_files"] == gallery
            and receipt["canonical_gallery_match"] == (gallery == receipt["gallery_files"]),
            "Scoped and canonical Gallery comparison lost its actual values")
    argv = invocation["argv"]
    artifact_origin = Path(argv[argv.index("--artifacts") + 1])
    recorded = artifact_origin.parent
    flags_path = Path(argv[0])
    stock_origin = flags_path.parents[2]
    require(recorded.is_absolute() and recorded.parts[:3] == ("/", "private", "tmp")
            and artifact_origin.name == "artifacts" and recorded.parent == stock_origin
            and argv == observer["stock_command"](recorded, flags_path, stock_origin / "bundle",
                                                  {r: str(recorded / "guards" / r) for r in observer["GUARD_ROLES"]}),
            "Original exact stock command or captured private root differs")

    replay = normalized(observer["verify_run"](ArchivedRun(run, recorded), watch, after["release"]))
    require(replay == verdict and all(raw[key] == value for key, value in verdict.items())
            and raw["window_receipt_sha256"] == digest(run / WINDOW)
            and raw["runner_log_sha256"] == digest(run / "runner.log")
            and replay["page_errors"] == 0 and replay["adapter"] == ADAPTER
            and replay["cases"]["material"]["steps"] == 349
            and len(replay["text_field_label_motion"]["pngs"]) == 34
            and replay["text_field_label_motion"]["label_color"]["space"] == "Oklab"
            and len(replay["textbox_values"]["inactive_ax"]) == 6
            and len(replay["multiline_text_field"]["pngs"]) == 12
            and set(raw["retained"]["retained"]) >= {"ime_cancellation", "navigation", "menu", "divider", "radio", "checkbox", "switch", "tabs"},
            "Wrong full stock execution scope or independent original pixels")
    return {
        "runner_sha256": runner_sha, "offline_sha256": offline_sha, "visible_windows": 0,
        "surviving_processes": 0, "stock_gallery_matches_scoped": receipt["canonical_gallery_match"],
        "label_pngs": replay["text_field_label_motion"]["pngs"], "cases": replay["cases"],
        "total_checks": replay["total_checks"], "browser": replay["browser"], "adapter": ADAPTER,
    }


def preflight(source):
    missing = pending_markers()
    verified_prior_live()
    seal(STOCK, "harness.sha256", STOCK_HARNESS)
    scoped = focused_info(FOCUSED, STOCK, EXACT / "source.sha256")
    with import_from(STOCK):
        focused_bind = run_path(str(STOCK / "verify_label_color.py"))["focused_receipt"](EXACT / "source.sha256")
    require(focused_bind["captures"] == scoped["captures"] and focused_bind["checks"] == 1798,
            "The independent focus verifier saw different originals")
    result = {"status": "preflight only", "source": source, "focused": scoped,
              "native_development_r2": native_development(), "pending_markers": missing}
    if "native-v0168-passed" not in missing:
        require(digest(EXACT / "binaries/SHA256SUMS") == NATIVE_BIN, "Wrong exact native pair pin")
        verify_entries(EXACT / "binaries", path_entries(EXACT / "binaries/SHA256SUMS"))
        result["native"] = native_live(observed_native_pins())
    if not {"workspace-v0168-passed", "gpu-v0168-passed"} & set(missing):
        pins = observed_rust_pins()
        result["rust"] = rust_info(Q, pins, False)
        result["metal_pngs"] = gpu_info(Q / "gpu-v0168-exact", pins["gpu_manifest"])
    return result


def archive_stock(directory):
    run = STOCK / "run"
    copy_members(STOCK, directory, (*path_entries(STOCK / "harness.sha256"),
                                  *path_entries(STOCK / "sealed-inputs.sha256"), "harness.sha256",
                                  "sealed-inputs.sha256", "inputs-before.json", "approved-runner.sha256", OFFLINE))
    target = directory / "run"
    copy_members(run, target, (WINDOW, STOCK_RESULT, "invocation.json", "inputs-after.json", "runner.log",
                              *(str(path.relative_to(run)) for path in (run / "guard-ledger").glob("*.json"))))
    for file in (run / "artifacts").iterdir():
        if file.suffix == ".json":
            copy(file, target / "artifacts" / file.name)
    _archive_stock_pngs(run / "artifacts", target / "artifacts", data(STOCK / OFFLINE))


def _archive_stock_pngs(source, target, receipt):
    png_names = set()
    raw_hashes = set()
    queue = [receipt]
    while queue:
        element = queue.pop()
        if isinstance(element, dict):
            queue.extend((*element.keys(), *element.values()))
        elif isinstance(element, list):
            queue.extend(element)
        elif isinstance(element, str):
            if re.fullmatch(r"[a-zA-Z0-9_+.-]+\.png", element):
                png_names.add(element)
            elif HEX.fullmatch(element):
                raw_hashes.add(element)
    for path in source.glob("*.png"):
        if path.name in png_names or digest(path) in raw_hashes or path.name.startswith(FEATURE):
            copy(path, target / path.name)


def export(root, receipt, gate_pins):
    require(root.is_relative_to(Path("/tmp").resolve()) and root != Path("/tmp").resolve(),
            "Use a fresh /tmp private package path")
    root.mkdir(mode=0o700)
    copy(EXACT / "source.sha256", root / "source.sha256")
    copy(PREVIOUS_SOURCES / "source.sha256", root / "base-source.sha256")
    copy(gate_pins, root / "gate-pins.json")
    compressed(Q / PATCH_NAME, root / (PATCH_NAME + ".gz"))
    copy_members(HERE, root, ("pack-label-color.py", "evidence_common.py"))
    for name in RUST_LOGS:
        compressed(Q / name, root / "rust" / (name + ".gz"))
    for name, path in READ_MARKERS.items():
        copy(path, root / "gates" / name)
    copy_members(Q / "gpu-v0168-exact", root / "rust/metal", (*receipt["metal_pngs"], "SHA256SUMS"))
    prev = PREVIOUS / "portable-r2"
    copy(prev / "SHA256SUMS", root / "previous/package-v0167.sha256")
    copy(prev / "stock-macos" / PREVIOUS_STOCK_NAME, root / "previous" / PREVIOUS_STOCK_NAME)
    copy(prev / "verified-label-motion-v0167.json", root / "previous/verified-label-motion-v0167.json")
    copy(PREVIOUS_PUBLIC, root / "previous/published.json")
    copy(prev / "native/replay/SHA256SUMS", root / "previous/native-captures.sha256")
    focus = root / "focused"
    names = path_entries(FOCUSED / "SHA256SUMS")
    copy_members(FOCUSED, focus, (*names, "SHA256SUMS", "runner.sha256",
                                  "development-r2-verified.json", "pins/focused-source-manifest.sha256"))
    focus_run = FOCUSED / "development-r2"
    copy_members(focus_run, focus / "development-r2", (
        WINDOW, "runner.log", "invocation.json", "inputs-after.json", "owned-shutdown.json",
        f"artifacts/{FEATURE}.json", *(f"artifacts/{name}" for name in receipt["focused"]["captures"]),
        *(str(file.relative_to(focus_run)) for file in (focus_run / "guard-ledger").glob("*.json")),
    ))
    native = root / "native"
    replay = Q / "native-exact-replay"
    copy_members(replay, native / "replay", (*path_entries(replay / "SHA256SUMS"), "SHA256SUMS"))
    copy_members(NATIVE_HARNESS, native / "harness", (*path_entries(NATIVE_HARNESS / "harness.sha256"), "harness.sha256"))
    copy(EXACT / "binaries/SHA256SUMS", native / "binaries.sha256")
    copy(Q / "native-v0168-verified.json", native / "verified.json")
    copy(Q / "native-development-r2-verified.json", native / "development-r2-verified.json")
    copy(Q / "native-development-r2-replay/SHA256SUMS", native / "development-r2-captures.sha256")
    archive_stock(root / "stock-macos")
    json_file(root / REPORT, receipt)
    (root / "README.md").write_text(f"""# Surface v0.1.68 text field label color candidate proof

Tree {TREE} has 685 source paths and applies the reviewed ten-path patch
on the v0.1.67 tree {BASE_TREE}. The original patch and Rust log bytes
are stored in gzip files.

- The exact workspace passed {receipt["rust"]["workspace"]["passed"]:,} Rust tests and three Wasm checks.
  The actual Metal gate ran 18 tests with 21 original GPU PNGs.
- The exact native replay returned 14 PNGs from two headless repetitions
  with 24 window samples and zero visible windows. All endpoint PNGs
  match the v0.1.67 and scoped R2 originals. Page.screenshot, CLI and MCP matched.
- Scoped R2 ran 1,798 checks from M325 through M350. It keeps 34
  original PNGs, its exact typed receipt, and 314 headless samples
  with zero visible windows or surviving processes.
- Exact stock ran 349 Material steps and {receipt["stock_macos"]["total_checks"]:,} checks
  in five macOS suites. It reproduced the 34 motion and color captures, keeps
  the six inactive textbox value cases and all 20 IME steps, and shows
  zero windows and survivors.

The headless traces cover real Chromium accessibility structure and
synthetic composition, alongside real WebGPU rendered pixels. Native
captures here verify reduced-motion endpoints. Live speech and OS
input composition require separate proof.

## Offline replay

Use Python 3.12 with Pillow and the reviewed manifest digest.

~~~sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-label-color.py --verify-package . --package-sha "$reviewed_sha"
~~~

The archive contains original verified screenshots and recorded binary
pins. The independent command starts no app. The stock bundle, raw
scoped and native binaries, and build caches are build inputs rather
than archive contents. The predecessor archive remains at its
[reviewed v0.1.67 commit]({PREVIOUS_PUBLIC_URL}).
""")
    names = sorted((p for p in root.rglob("*") if p.is_file()), key=lambda p: p.relative_to(root).as_posix())
    manifest = root / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(p)}  {p.relative_to(root).as_posix()}\n" for p in names))
    return digest(manifest)


def verify_package(root, checksum):
    seal(root, "SHA256SUMS", checksum, exact=True)
    proof = data(root / REPORT)
    source = pinned_source(root / "base-source.sha256", root / "source.sha256", raw_log(root / (PATCH_NAME + ".gz")))
    require(proof["status"] == "verified" and proof["source"] == source, "Wrong archived source scope")
    gates = root / "gate-pins.json"
    require(digest(gates) == proof["gate_pins_sha256"], "Changed reviewed terminal gate pins")
    pins = final_gate_pins(data(gates))
    for name in READ_MARKERS:
        expected = b"" if name == "stock-mac-v0168-passed" else (TREE + "\n").encode()
        require((root / "gates" / name).read_bytes() == expected, f"Wrong archived marker {name}")
    stock = root / "stock-macos"
    focused = focused_info(root / "focused", stock, root / "source.sha256")
    previous = prior_info(root / "previous")
    native = root / "native"
    require(proof["focused"] == focused and proof["previous"] == previous
            and proof["native"] == native_info(native / "replay", native / "harness", native / "binaries.sha256",
                                               native / "verified.json", root / "previous/native-captures.sha256",
                                               pins["native_manifest"], pins["native_receipt"], SOURCE)
            and proof["rust"] == rust_info(root / "rust", pins, True)
            and proof["metal_pngs"] == gpu_info(root / "rust/metal", pins["gpu_manifest"]),
            "Archived exact local proof differs")
    dev_report = native / "development-r2-verified.json"
    dev_manifest = native / "development-r2-captures.sha256"
    require(digest(dev_report) == NATIVE_DEV_REPORT and digest(dev_manifest) == NATIVE_DEV_REPLAY
            and proof["native_development_r2"] == {
                **data(dev_report), "matching_v0167": 14, "transports": ["Page.screenshot", "CLI", "MCP"],
            }
            and native_pngs(dev_manifest) == native_pngs(native / "replay/SHA256SUMS"),
            "Scoped and exact native raster endpoints differ")
    result = proof["stock_macos"]
    require(result == stock_info(stock, stock / "run", pins["stock_runner"], pins["stock_offline"], focused),
            "Archived exact stock proof differs")
    return proof


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check-source", action="store_true")
    mode.add_argument("--read-gate-pins", action="store_true")
    mode.add_argument("--preflight", action="store_true")
    mode.add_argument("--out", type=Path)
    mode.add_argument("--verify-package", type=Path)
    parser.add_argument("--source-sha")
    parser.add_argument("--tree")
    parser.add_argument("--stock-harness-sha")
    parser.add_argument("--gate-pins", type=Path)
    parser.add_argument("--gate-pins-sha")
    parser.add_argument("--package-sha")
    args = parser.parse_args()
    if args.verify_package:
        return verify_package(args.verify_package.resolve(strict=True), args.package_sha)
    require(args.source_sha == SOURCE and args.tree == TREE, "Supply the reviewed exact source and tree")
    source = source_info()
    if args.check_source:
        return source
    if args.read_gate_pins:
        return observed_final_pins()
    if args.out:
        pending = pending_markers()
        require(not pending, "Pending exact gates: " + ", ".join(pending))
        require(args.stock_harness_sha == STOCK_HARNESS, "Provide the reviewed exact stock harness seal")
        require(args.gate_pins is not None and HEX.fullmatch(args.gate_pins_sha or "")
                and digest(args.gate_pins) == args.gate_pins_sha,
                "Provide the reviewed terminal gate pin file and its SHA-256")
        pins = final_gate_pins(data(args.gate_pins))
    value = preflight(source)
    if not args.out:
        return value
    require(digest(STOCK / OFFLINE) == pins["stock_offline"]
            and (STOCK / "approved-runner.sha256").read_text().strip() == pins["stock_runner"],
            "Provide the reviewed exact stock runner and offline receipt SHA-256")
    canonical = run_json([STOCK / "verify-stock-receipt.py", STOCK / "run"])
    require(canonical == data(STOCK / OFFLINE), "Actual stock offline verification differs")
    stock = stock_info(STOCK, STOCK / "run", pins["stock_runner"], pins["stock_offline"], value["focused"])
    require(digest(EXACT / "binaries/SHA256SUMS") == NATIVE_BIN, "Wrong actual native executable seal")
    verify_entries(EXACT / "binaries", path_entries(EXACT / "binaries/SHA256SUMS"))
    prev = PREVIOUS / "portable-r2"
    previous = {"tree": BASE_TREE, "source_sha256": BASE_SOURCE, "published": data(PREVIOUS_PUBLIC),
                "report_sha256": digest(prev / "verified-label-motion-v0167.json"),
                "stock_offline_sha256": digest(prev / "stock-macos" / PREVIOUS_STOCK_NAME),
                "stock_checks": 24973, "material_steps": 349}
    value.pop("pending_markers")
    value.update(status="verified", stock_macos=stock, previous=previous,
                 native=native_live(pins), rust=rust_info(Q, pins, False),
                 metal_pngs=gpu_info(Q / "gpu-v0168-exact", pins["gpu_manifest"]),
                 gate_pins_sha256=args.gate_pins_sha)
    root = args.out.resolve()
    marker = root.with_name(root.name + "-verified.json")
    require(not marker.exists(), "Use a fresh external package verification marker")
    checksum = export(root, value, args.gate_pins)
    require(verify_package(root, checksum) == value, "Fresh archive recheck differs")
    output = {"path": str(root), "manifest_sha256": checksum, "files": len(path_entries(root / "SHA256SUMS")),
              "source_sha256": SOURCE, "tree": TREE, "runner_sha256": pins["stock_runner"],
              "stock_offline_sha256": pins["stock_offline"], "report_sha256": digest(root / REPORT)}
    json_file(marker, output)
    return {**value, "output": output, "marker": str(marker)}


if __name__ == "__main__":
    try:
        print(json.dumps(main(), indent=2, sort_keys=True, ensure_ascii=False))
    except (ValueError, KeyError, OSError, RuntimeError, subprocess.CalledProcessError) as error:
        sys.exit(f"Label color evidence stopped. {error}")
