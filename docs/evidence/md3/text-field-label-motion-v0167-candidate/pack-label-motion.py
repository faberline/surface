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
Q = Path("/tmp/surface-md3-text-field-label-motion-v0167.Y7f9HqIi")
EXACT = Q / "exact-candidate"
PREVIOUS = Path("/tmp/surface-md3-textbox-values-v0166-package.H7vrzgeg")
PREVIOUS_SOURCES = Path("/tmp/surface-md3-textbox-values-v0166.l4950l9e/exact-candidate")
PREVIOUS_PUBLIC = Path("/tmp/surface-md3-textbox-values-v0166-publisher-recovery.5V4PA2An/local-proof.json")
STOCK = Path("/tmp/surface-md3-field-label-motion-stock-v0167.ITHgCb8e")
FOCUSED = Path("/tmp/surface-md3-label-motion-e2e.PywRddEC")
NATIVE_HARNESS = Path("/tmp/surface-md3-multiline-native.cZ38fMkb")
SOURCE = "4aa53964b34f033b59b5ab723e58deb612d5db8ce1975bd3cf2115e687479271"
TREE = "c218faaf7527ec2872fb1ec1097c87667f3bf1f2"
BASE_SOURCE = "d4b0c5b7a1ae5daf1f7a9084e765b30b24f8cae6ecbda881efb9468daf730783"
BASE_TREE = "3de0c143c43d35911593cfc95d3ea5931016c308"
PATCH_NAME = "text-field-label-motion-v0167.patch"
PATCH = "d3b38aac44fad4cb071fcf2734e1edc935f741af2e6e5d710da92b2b8b40c6ba"
FEATURE = "material-text-field-label-motion"
FEATURE_PATH = "e2e/src/material_text_field_label_motion.rs"
FEATURE_SOURCE = "76a8a1726a04f0fdee80aeb27b24f784c38222db73b24cc475ff0c6c89950daa"
FOCUSED_SEAL = "a77ef3675e2743cbc7030a918f0b26a7ac61c00539439fa8f6a2fe2c9f0ff8db"
FOCUSED_VERIFIED = "3f693f657ca2d61268b54e3e1fe5a9092261954b365276a9e901cdc41ad1372b"
FOCUSED_RUNNER = "1db87fe350d4051190d1212d75a41eebe98098181def36ab24308502352ced3b"
FOCUSED_CODE = "9a3755f9e41a87318692674089be2d705e6a09a13d305330a2df092026e584da"
FOCUSED_WEB = "5ee0d1175d5c590d39b1c4d4438e241457e391a4abc7711fcf9b73334802ca43"
STOCK_HARNESS = "9e541b1d047e880186941c1e927a10133fa3f2d0fc8e226f2053e3c49abe3ed9"
PREVIOUS_SEAL = "613d62bb6abd3ba245c8a9662ad30a088877e3f3aa046f2210a8df931015764b"
PREVIOUS_RESULT = "9bf426bce9250b238ecebe974e7ba4b24a14c53004cfa05588e84db1c3a60dfa"
PREVIOUS_OFFLINE = "47d397e905a928079411fcf4af821b095a7e76c6e08ae56d4f5cc81483fbeb52"
PREVIOUS_STOCK_NAME = "stock-mac-exact-offline.json"
PREVIOUS_PUBLISHED = "2ce969df988909133af945ae7c3c0534c68351d8"
PREVIOUS_PUBLISHED_TREE = "d37f8b34f7b48d0fa03bec9a5b0bcd3b5bc96025"
PREVIOUS_PUBLIC_URL = f"https://github.com/faberline/surface/tree/{PREVIOUS_PUBLISHED}/docs/evidence/md3/textbox-values-v0166-candidate"
NATIVE_BIN = "03f88fff09a5608f7cd6437194642adcb04ead08d954cf6a113f7b0f505416f6"
NATIVE_REPLAY = "0245dd18654a31723635c4942f1cc4ade1f5d4c85b71d297f46e0b8150e3018e"
NATIVE_VERIFIED = "b9362d6134f56e794a5c2c14ab98be18e4d766e4427dadd3d0d3911ed4bae330"
PREVIOUS_NATIVE = "aa5c88610e5390f677ed653faa5843312fa9f0f11f77edbc659d50fb195f0858"
NATIVE_HARNESS_SHA = "c842ea6ac04a2428ac239bf4ec6357d2912603f9cd5b3decd1029a9e0b1b1f35"
GPU_SHA = "c7fb34d394d7702a9e9f6e8c617d552486dfb52bf766fce7bc4c8259440c409f"
STOCK_RESULT = "verified-label-motion-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
REPORT = "verified-label-motion-v0167.json"
WINDOW = "headless-window-observation.json"
UPDATER = "--disable-updater-scheduler"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
ADAPTER = {"vendor": "apple", "architecture": "metal-3", "isFallbackAdapter": False}
RUST_LOGS = {
    "workspace-v0167-exact.log": "af67958b82666b45d23fc78973c460e4f04a46b31e5bcf4224378b8eca65cea7",
    "gpu-v0167-exact.log": "26be4c03395da3b66aa59f8627fb2a3715326902687010d36b0ea972cf4e99a1",
    "gpu-v0167-supplemental.log": "6ee010ce6785ef7eb65198eeae149ab0d232a2b76db423c4579723a8bdabec6c",
}
READ_MARKERS = {
    "workspace-v0167-passed": Q / "workspace-v0167-passed",
    "gpu-v0167-passed": Q / "gpu-v0167-passed",
    "native-v0167-passed": Q / "native-v0167-passed",
    "native-final-built": EXACT / "native-final-built",
    "stock-canonical-built": EXACT / "stock-canonical-built",
    "stock-mac-v0167-passed": STOCK / "stock-mac-v0167-passed",
}


def pinned_source(base, exact, patch):
    require(digest(base) == BASE_SOURCE and digest(exact) == SOURCE
            and sha256(patch).hexdigest() == PATCH, "Wrong source or original patch digest")
    before, after = path_entries(base), path_entries(exact)
    require((len(before), len(after)) == (681, 684), "Wrong prior or exact path count")
    paths = re.findall(r"^diff --git a/(\S+) b/\S+$", patch.decode(), re.M)
    require(len(paths) == len(set(paths)) == 17
            and set(paths) == {p for p in before.keys() | after.keys() if before.get(p) != after.get(p)}
            and after[FEATURE_PATH] == FEATURE_SOURCE, "Wrong patch path scope or feature test")
    return {"version": "0.1.67", "tree": TREE, "source_sha256": SOURCE, "source_files": len(after),
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
    require(version == "0.1.67"
            and [int(step) for step in re.findall(r'"M(\d+)"', registration)] == [i for i in range(1, 351) if i != 23]
            and (Q / "exact-apply-checked").read_text().strip() == TREE
            and (Q / "source-tree").read_text().strip() == TREE and (Q / "prior-tree").read_text().strip() == BASE_TREE,
            "Wrong release version, exact replay stamp or registered steps")
    return summary


def prior_info(root):
    receipt = root / "verified-textbox-values-v0166.json"
    offline = root / PREVIOUS_STOCK_NAME
    published = data(root / "published.json")
    manifest = root / "package-v0166.sha256"
    require(digest(receipt) == PREVIOUS_RESULT and digest(offline) == PREVIOUS_OFFLINE
            and digest(manifest) == PREVIOUS_SEAL, "Wrong retained v0.1.66 proof files")
    entries = path_entries(manifest)
    require(entries[receipt.name] == PREVIOUS_RESULT
            and entries[f"stock-macos/{PREVIOUS_STOCK_NAME}"] == PREVIOUS_OFFLINE
            and published == {"commit": PREVIOUS_PUBLISHED, "tree": PREVIOUS_PUBLISHED_TREE,
                              "manifest_sha256": PREVIOUS_SEAL, "url": PREVIOUS_PUBLIC_URL},
            "Retained v0.1.66 archive or commit changed")
    report = data(receipt)
    require(report["source"]["tree"] == BASE_TREE and report["source"]["source_sha256"] == BASE_SOURCE
            and report["stock_macos"]["total_checks"] == data(offline)["total_checks"] == 24750
            and report["stock_macos"]["cases"]["material"]["steps"] == 345
            and report["stock_macos"]["visible_windows"] == report["stock_macos"]["surviving_processes"] == 0
            and len(report["stock_macos"]["textbox_values"]["inactive_ax"]) == 6,
            "Retained predecessor AX, suites or counts disagree")
    return {"tree": BASE_TREE, "source_sha256": BASE_SOURCE, "published": published,
            "report_sha256": PREVIOUS_RESULT, "stock_offline_sha256": PREVIOUS_OFFLINE,
            "stock_checks": 24750, "material_steps": 345}


def focused_info(root):
    entries = seal(root, "EVIDENCE.sha256", FOCUSED_SEAL)
    require(len(entries) == 81 and digest(root / "development-r3-verified.json") == FOCUSED_VERIFIED
            and digest(root / "source.sha256") == FOCUSED_CODE
            and path_entries(root / "source.sha256")[FEATURE_PATH] == FEATURE_SOURCE
            and digest(root / "web.sha256") == FOCUSED_WEB
            and (root / "runner.sha256").read_text().strip() == FOCUSED_RUNNER, "Wrong original focused proof pins")
    verify = run_path(str(root / "verify-label-motion.py"))["verify_run"]
    outputs = [verify(root / f"development-r{i}", i < 3) for i in (1, 2, 3)]
    for index, result in enumerate(outputs, 1):
        live = root / f"development-r{index}"
        watch = data(live / WINDOW)
        verify_ledgers(live, watch, {"chrome-primary"})
        event, = watch["guard_events"]
        launch = data(live / "invocation.json")
        require((result["checks"], result["steps"], result["visible_windows"], len(result["captures"]))
                == (1495, 26, 0, 15)
                and result["report_sha256"] == outputs[2]["report_sha256"]
                and result["captures"] == outputs[2]["captures"] and result["adapter"] == ADAPTER
                and watch["stock_exit"] == 0 and launch["runner_sha256"] == FOCUSED_RUNNER
                and launch["web_files"] == path_entries(root / "web.sha256")
                and event["argv"].count(UPDATER) == (1 if index == 3 else 0),
                "A focused replay changed scope, inputs, screenshot bytes or launch-only flags")
    winner = outputs[2]
    shutdown = data(root / "development-r2/owned-shutdown.json")
    require(winner == data(root / "development-r3-verified.json") and winner["observed_headless_samples"] == 256
            and winner["guard_passed"] is True and winner["remaining_owned_processes"] == []
            and all(not r["guard_passed"] and len(r["remaining_owned_processes"]) == 2 for r in outputs[:2])
            and sorted(p["pid"] for p in shutdown["after_grace"]) == outputs[1]["remaining_owned_processes"]
            and {p["command"].rsplit(" ", 1)[1] for p in shutdown["after_grace"]} == {"--wake", "--wake-all"}
            and all("GoogleUpdater.app/Contents/MacOS/GoogleUpdater" in p["command"] for p in shutdown["after_grace"])
            and shutdown["after_cleanup"] == []
            and data(root / "development-r3/owned-shutdown.json")["after_grace"] == [],
            "Wrong positive or classified negative Chrome lifecycle")
    return {**winner, "evidence_sha256": FOCUSED_SEAL, "three_identical_png_runs": 3,
            "negative_guard_receipts": [r["window_receipt_sha256"] for r in outputs[:2]]}


def native_info(replay, harness, binaries, receipt, prior_capture):
    require(digest(receipt) == NATIVE_VERIFIED and digest(binaries) == NATIVE_BIN
            and digest(prior_capture) == PREVIOUS_NATIVE, "Wrong exact native seal or predecessor captures")
    entries = seal(replay, "SHA256SUMS", NATIVE_REPLAY, exact=True)
    seal(harness, "harness.sha256", NATIVE_HARNESS_SHA)
    result = run_json([harness / "verify-stored-multiline.py", replay, "--manifest-sha", NATIVE_REPLAY,
                       "--source-manifest-sha", SOURCE, "--binary-manifest-sha", NATIVE_BIN])
    require(result == data(receipt) and
            (result["repeats"], result["original_pngs"], result["visible_owned_windows"],
             result["observed_window_samples"], result["reaped_gallery_processes"]) == (2, 14, 0, 24, 4),
            "Wrong real native headless scope")
    actual = {name: value for name, value in entries.items() if name.endswith(".png")}
    previous = {name: value for name, value in path_entries(prior_capture).items() if name.endswith(".png")}
    require(len(actual) == 14 and actual == previous, "Native reduced-motion endpoints differ from .66")
    return {**result, "matching_v0166": 14, "transports": ["Page.screenshot", "CLI", "MCP"]}


def gpu_info(root):
    manifest = root / "SHA256SUMS"
    require(digest(manifest) == GPU_SHA, "Wrong actual Metal image seal")
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


def rust_info(root, archived):
    records = []
    for name, wanted in RUST_LOGS.items():
        path = root / (name + ".gz" if archived else name)
        require(sha256(raw_log(path)).hexdigest() == wanted, f"Wrong exact log: {name}")
        records.append(tests(path))
    (workspace, record), (primary_log, primary), (extra_log, extra) = records
    after_tests = workspace.rsplit("\ntest result:", 1)[-1]
    require((record["passed"], record["ignored"]) == (1570, 20)
            and after_tests.count("Finished \x60dev\x60 profile") == 3
            and re.search(r"^provenance: .+ self-test cases passed\)", workspace, re.M)
            and "floating_label_reverses_from_the_same_pose_and_reduced_motion_settles_it ... ok" in workspace,
            "Wrong exact Rust, provenance or three Wasm checks")
    require(primary["passed"] == 16 and primary["ignored"] == 0
            and extra["passed"] == 2 and extra["ignored"] == 0
            and "backend: Metal" in primary_log and extra_log.count("backend: Metal") == 2
            and 'name: "Apple M1 Max"' in primary_log, "Wrong actual Apple Metal tests")
    return {"workspace": {**record, "wasm_checks": 3}, "metal": {**primary, "backend": "Metal"},
            "supplemental": {**extra, "backend": "Metal"}, "metal_total_passed": 18}


def native_live():
    return native_info(
        Q / "native-exact-replay", NATIVE_HARNESS, EXACT / "binaries/SHA256SUMS",
        Q / "native-v0167-verified.json", PREVIOUS_SOURCES.parent / "native-exact-replay/SHA256SUMS",
    )


def verified_prior_live():
    original = run_json([PREVIOUS / "pack-textbox-values.py", "--verify-package", PREVIOUS / "portable",
                         "--package-sha", PREVIOUS_SEAL])
    require(original == data(PREVIOUS / "portable/verified-textbox-values-v0166.json"),
            "Prior exact portable archive differs")


def pending_markers():
    pending = []
    for name, path in READ_MARKERS.items():
        if not path.is_file():
            pending.append(name)
        else:
            expected = b"" if name == "stock-mac-v0167-passed" else (TREE + "\n").encode()
            require(path.read_bytes() == expected, f"Wrong exact marker {name}")
    return pending


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
             observer["LAST_MATERIAL"], observer["CHROME_FLAGS"]) == (SOURCE, TREE, 684, 350, (UPDATER,)),
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
            and len(path_entries(pins / "browser-source.sha256")) == after["source_files_checked_per_origin"] == 684
            and after["release"] == dict(version="0.1.67", source_sha256=SOURCE, source_files=684, source_tree=TREE,
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
            and receipt["runner_sha256"] == FOCUSED_RUNNER and receipt["focused_receipt_sha256"] == FOCUSED_VERIFIED
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
            and len(replay["text_field_label_motion"]["pngs"]) == 15
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
    scoped = focused_info(FOCUSED)
    with import_from(STOCK):
        focused_bind = run_path(str(STOCK / "verify_label_motion.py"))["focused_receipt"](EXACT / "source.sha256")
    require(focused_bind["captures"] == scoped["captures"] and focused_bind["checks"] == 1495,
            "The independent focus verifier saw different originals")
    result = {"status": "preflight only", "source": source, "focused": scoped, "pending_markers": missing}
    if "native-v0167-passed" not in missing:
        require(digest(EXACT / "binaries/SHA256SUMS") == NATIVE_BIN, "Wrong actual native executable seal")
        verify_entries(EXACT / "binaries", path_entries(EXACT / "binaries/SHA256SUMS"))
        result["native"] = native_live()
    if not {"workspace-v0167-passed", "gpu-v0167-passed"} & set(missing):
        result["rust"] = rust_info(Q, False)
        result["metal_pngs"] = gpu_info(Q / "gpu-v0167-exact")
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


def export(root, receipt):
    require(root.is_relative_to(Path("/tmp").resolve()) and root != Path("/tmp").resolve(),
            "Use a fresh /tmp private package path")
    root.mkdir(mode=0o700)
    copy(EXACT / "source.sha256", root / "source.sha256")
    copy(PREVIOUS_SOURCES / "source.sha256", root / "base-source.sha256")
    compressed(Q / PATCH_NAME, root / (PATCH_NAME + ".gz"))
    copy_members(HERE, root, ("pack-label-motion.py", "evidence_common.py"))
    for name in RUST_LOGS:
        compressed(Q / name, root / "rust" / (name + ".gz"))
    for name, path in READ_MARKERS.items():
        copy(path, root / "gates" / name)
    copy_members(Q / "gpu-v0167-exact", root / "rust/metal", (*receipt["metal_pngs"], "SHA256SUMS"))
    prev = PREVIOUS / "portable"
    copy(prev / "SHA256SUMS", root / "previous/package-v0166.sha256")
    copy(prev / "stock-macos" / PREVIOUS_STOCK_NAME, root / "previous" / PREVIOUS_STOCK_NAME)
    copy(prev / "verified-textbox-values-v0166.json", root / "previous/verified-textbox-values-v0166.json")
    copy(PREVIOUS_PUBLIC, root / "previous/published.json")
    copy(prev / "native/exact/SHA256SUMS", root / "previous/native-captures.sha256")
    focus = root / "focused"
    names = path_entries(FOCUSED / "EVIDENCE.sha256")
    copy_members(FOCUSED, focus, (*names, "EVIDENCE.sha256", "runner.sha256"))
    native = root / "native"
    replay = Q / "native-exact-replay"
    copy_members(replay, native / "replay", (*path_entries(replay / "SHA256SUMS"), "SHA256SUMS"))
    copy_members(NATIVE_HARNESS, native / "harness", (*path_entries(NATIVE_HARNESS / "harness.sha256"), "harness.sha256"))
    copy(EXACT / "binaries/SHA256SUMS", native / "binaries.sha256")
    copy(Q / "native-v0167-verified.json", native / "verified.json")
    archive_stock(root / "stock-macos")
    json_file(root / REPORT, receipt)
    (root / "README.md").write_text(f"""# Surface v0.1.67 text field label motion candidate proof

Tree {TREE} has 684 source paths and applies the reviewed 17-path patch
on the v0.1.66 tree {BASE_TREE}. The original patch and Rust log bytes
are stored in gzip files.

- The exact workspace passed 1,570 Rust tests and three Wasm checks.
  The actual Metal gate ran 18 tests with 21 original GPU PNGs.
- The exact native replay returned 14 PNGs from two headless repetitions
  with 24 window samples and zero visible windows. All endpoint PNGs
  match the v0.1.66 originals. Page.screenshot, CLI and MCP matched.
- Three scoped Chromium trials ran 1,495 checks each. All 15 original
  label motion PNGs have the same bytes in all three runs. The first
  two remain negative cleanup controls. The third recorded 256
  headless samples with zero visible windows or surviving processes.
- Exact stock ran 349 Material steps and {receipt["stock_macos"]["total_checks"]:,} checks
  in five macOS suites. It reproduced the 15 motion captures, keeps
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
python3 -E -B pack-label-motion.py --verify-package . --package-sha "$reviewed_sha"
~~~

The archive contains original verified screenshots and recorded binary
pins. The independent command starts no app. The stock bundle, raw
scoped and native binaries, and build caches are build inputs rather
than archive contents. The predecessor archive remains at its
[reviewed v0.1.66 commit]({PREVIOUS_PUBLIC_URL}).
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
    for name in READ_MARKERS:
        expected = b"" if name == "stock-mac-v0167-passed" else (TREE + "\n").encode()
        require((root / "gates" / name).read_bytes() == expected, f"Wrong archived marker {name}")
    focused = focused_info(root / "focused")
    previous = prior_info(root / "previous")
    native = root / "native"
    require(proof["focused"] == focused and proof["previous"] == previous
            and proof["native"] == native_info(native / "replay", native / "harness", native / "binaries.sha256",
                                               native / "verified.json", root / "previous/native-captures.sha256")
            and proof["rust"] == rust_info(root / "rust", True)
            and proof["metal_pngs"] == gpu_info(root / "rust/metal"), "Archived exact local proof differs")
    stock = root / "stock-macos"
    result = proof["stock_macos"]
    require(result == stock_info(stock, stock / "run", result["runner_sha256"], result["offline_sha256"], focused),
            "Archived exact stock proof differs")
    return proof


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check-source", action="store_true")
    mode.add_argument("--preflight", action="store_true")
    mode.add_argument("--out", type=Path)
    mode.add_argument("--verify-package", type=Path)
    parser.add_argument("--source-sha")
    parser.add_argument("--tree")
    parser.add_argument("--stock-harness-sha")
    parser.add_argument("--stock-runner-sha")
    parser.add_argument("--stock-offline-sha")
    parser.add_argument("--package-sha")
    args = parser.parse_args()
    if args.verify_package:
        return verify_package(args.verify_package.resolve(strict=True), args.package_sha)
    require(args.source_sha == SOURCE and args.tree == TREE, "Supply the reviewed exact source and tree")
    source = source_info()
    if args.check_source:
        return source
    if args.out:
        pending = pending_markers()
        require(not pending, "Pending exact gates: " + ", ".join(pending))
        require(args.stock_harness_sha == STOCK_HARNESS, "Provide the reviewed exact stock harness seal")
    value = preflight(source)
    if not args.out:
        return value
    require(HEX.fullmatch(args.stock_offline_sha or "") and digest(STOCK / OFFLINE) == args.stock_offline_sha
            and HEX.fullmatch(args.stock_runner_sha or "")
            and (STOCK / "approved-runner.sha256").read_text().strip() == args.stock_runner_sha,
            "Provide the reviewed exact stock runner and offline receipt SHA-256")
    canonical = run_json([STOCK / "verify-stock-receipt.py", STOCK / "run"])
    require(canonical == data(STOCK / OFFLINE), "Actual stock offline verification differs")
    stock = stock_info(STOCK, STOCK / "run", args.stock_runner_sha, args.stock_offline_sha, value["focused"])
    prev = PREVIOUS / "portable"
    previous = {"tree": BASE_TREE, "source_sha256": BASE_SOURCE, "published": data(PREVIOUS_PUBLIC),
                "report_sha256": digest(prev / "verified-textbox-values-v0166.json"),
                "stock_offline_sha256": digest(prev / "stock-macos" / PREVIOUS_STOCK_NAME),
                "stock_checks": 24750, "material_steps": 345}
    value.pop("pending_markers")
    value.update(status="verified", stock_macos=stock, previous=previous)
    root = args.out.resolve()
    marker = root.with_name(root.name + "-verified.json")
    require(not marker.exists(), "Use a fresh external package verification marker")
    checksum = export(root, value)
    require(verify_package(root, checksum) == value, "Fresh archive recheck differs")
    output = {"path": str(root), "manifest_sha256": checksum, "files": len(path_entries(root / "SHA256SUMS")),
              "source_sha256": SOURCE, "tree": TREE, "runner_sha256": args.stock_runner_sha,
              "stock_offline_sha256": args.stock_offline_sha, "report_sha256": digest(root / REPORT)}
    json_file(marker, output)
    return {**value, "output": output, "marker": str(marker)}


if __name__ == "__main__":
    try:
        print(json.dumps(main(), indent=2, sort_keys=True, ensure_ascii=False))
    except (ValueError, KeyError, OSError, RuntimeError, subprocess.CalledProcessError) as error:
        sys.exit(f"Label motion evidence stopped. {error}")
