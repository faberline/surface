#!/usr/bin/env python3
"""Export or recheck the exact local Surface v0.1.73 Navigation evidence."""

import argparse
import json
from hashlib import sha256
from pathlib import Path
import re
from runpy import run_path
import subprocess
import sys

from PIL import Image
from evidence_common import (
    clean_windows, compressed, copy, copy_members, data, digest, json_file,
    normalized, path_entries, raw_log, require, seal, source_tree, tests, verify_entries,
    verify_ledgers,
)

Q73 = Path("/tmp/surface-md3-horizontal-nav-v0173.OEjA5TvP")
EXACT = Q73 / "exact-candidate"
STOCK = Path("/tmp/surface-md3-horizontal-nav-stock-v0173.5buwihck")
FOCUS = Path("/tmp/surface-md3-horizontal-nav-focused-v0173-r3.KYlcfBAK")
NATIVE = Path("/tmp/surface-md3-horizontal-nav-native-v0173-r3.uXROam90")
LEGACY = Path("/tmp/surface-md3-sliders-stock.lK6a9gKl")
SOURCE = "72716762c2e2615a92e29dc45c840e0c7278cafbf31d489d165aa92b20556e14"
TREE = "5d5ab69f57bdb3f31a8db95ae78f9f332789578b"
BASE = "b3137077ef9753a91a4e86b3c7d9dedc14b479f4aa4e74c95b299a6805543cd9"
BASE_TREE = "9333c1f8dba9a7d15aba2ca47dc92c997abf98af"
PATCH = "1f8c51cc1343d543cf37a283eba4b509020ed41c8486d02a85602c082f77a937"
PATCH_NAME = "horizontal-navigation-bar-v0173.patch"
BINARY = "be1868febce8bfcfb611e632122476949b14d8d1470e4a83e2454557ec05cc98"
NATIVE_PROOF = "9a21f7e1571d7c01d07b5d8977d3705ce8db6976284ffab7ea9746da752bcf61"
NATIVE_TOOLS = "4fc8a3572605cb7dbc38e50c7361cefb4ed2e5cc3648d090c482cdd9f0638bea"
FOCUS_TOOLS = "14f92282fe4ef079c5dbf88aef59bba6ad733cb2b12d9d0842d93d328a9dd998"
FOCUS_VERIFIER = "a81f98dacae2c673587fdb156dd71f60b657c3d9bc6913ea87eaae646951625b"
FOCUS_RESULT = "c83e19154407ac7b45e9ee467d3a0f1ef5be36839f3bb8f468a906965e1f5a98"
FOCUS_SOURCE = "38cdcd510d2808cd3685691ae0656c25ca1017f7c3cbf8d88af42c08370caa27"
FOCUS_SELECTOR = "03b61cdd1e2b7ee5f5a91d9267fe8102883a0a410b226f08cab2363f7277e2e9"
FOCUS_WEB_SOURCE = "13716392d80ba252314d2be056d41b7c4812e85347684cca5ee671acd6cf836c"
FOCUS_WEB = "d4d194543bb95dda8505681fee1a05bdfb9321e0ad458a96a5c9a9a6f803cc69"
STOCK_TOOLS = "1bc8c5e7d6d84ec27f8bea662172879502b872f614b507ca73f20f8f22039da7"
STOCK_SEAL = "33b103bc7f5aa0e48fb3def437ab785ea80d63d6018f20e49c78b50abc62ec4d"
STOCK_ASSETS = "e302e154cc70814d75228841cfb481f20f50b56b110be20104911a0914a0a737"
STOCK_RUNNER = "1959bb6f119f56587cfffc7cce14db460540cd64efc850688770358a91c9413d"
LEGACY_VERIFIER = "eded5dfdfcf94dc901cb3608b83cc86a557fe4cb02b91122416c6a03e3568558"
LEGACY_MANIFEST = "5a9c99e2bf64a76375d81d5f16ab58228410be56ba32225b01930df949a0ac6f"
WORKSPACE = "workspace-v0173-exact.log"
BUILD = "stock-build-v0173-exact.log"
GPU = ("gpu-v0173-exact.log", "gpu-v0173-supplemental.log")
LOG_HASHES = {
    WORKSPACE: "e47abf287df042004d3f54f77ca9665f6b9f7cd6292863b1763a1bed3dea73b5",
    BUILD: "c37c0cc5c8a84de69fc40c6dcf9c18135c17388d088aee878112c0e238cce7e5",
    GPU[0]: "17f825e43fd3e45216f378afd34228cf4791f91c0476e530d78bc451b12ef326",
    GPU[1]: "240262dd896e1fa1a797447d7d6ae2b6d3d653b8e4118b6c69d1233e99bebabd",
}
GPU_IMAGES = "c7fb34d394d7702a9e9f6e8c617d552486dfb52bf766fce7bc4c8259440c409f"
STOCK_MARKER = "stock-mac-v0173-passed"
STOCK_REPORT = "verified-horizontal-navigation-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
WINDOW = "headless-window-observation.json"
REPORT = "verified-horizontal-navigation-v0173.json"
R3_REPORT = "verified-navigation-r3.json"
STOCK_TERMINAL = {
    f"run/{STOCK_REPORT}": "dba2a4841a55fc2fb858384b9236debc494367fd056d4abbf3d582a6a106952d",
    OFFLINE: "c224fd0156c540998b85eb01d5cbd540c4ed4da7a3fc2f8e816551f69f2bc250",
    "run/runner.log": "19e96a5c37e6b0e9e2f4e3ab7cc862ca2a0c93c9e9be49283a0ba7b8735c43ea",
    f"run/{WINDOW}": "01f70d75ffa4019ad5f43b4b06ec616467d5bf8b2281c6acbb4b83d798f15cff",
}
FOCUS_STEPS = [f"M{index}" for index in (*range(270, 274), *range(363, 366))]
SUITES = ["smoke", "ime", "material", "bridge-web", "devtools-headless"]
UPDATER = "--disable-updater-scheduler"
PREVIOUS = "https://github.com/faberline/surface/tree/75835231436a1cbc4d0cacf41a1401c5020e1e39/docs/evidence/md3/text-field-system-pair-v0172-candidate"
BROWSER_REPORTS = ("browser-material-flexible-navigation-bar.json", "material-horizontal-navigation-bar.json")
PROOF_FILES = ("runner.log", WINDOW, "invocation.json", "inputs-after.json")
MARKERS = ("workspace-v0173-passed", "gpu-v0173-passed", "native-final-built", "stock-canonical-built")
INHERITED_REPORTS = ("browser-smoke.json", "browser-ime.json", "browser-material-side-sheet.json",
                     "bridge-web-result.json", "devtools-headless-result.json")


def checked(path, sha):
    require(path.is_file() and not path.is_symlink() and digest(path) == sha, f"Wrong exact input {path}")
    return path


def delta(base, current):
    return sorted(name for name in base.keys() | current.keys() if base.get(name) != current.get(name))


def source_proof(base, source, patch):
    before, after = path_entries(checked(base, BASE)), path_entries(checked(source, SOURCE))
    paths = re.findall(r"^diff --git a/(\S+) b/\S+$", checked(patch, PATCH).read_text(), re.M)
    require(len(before) == 691 and len(after) == 692 and len(paths) == len(set(paths)) == 11
            and sorted(paths) == delta(before, after), "Wrong exact source delta or scope")
    return {"tree": TREE, "manifest_sha256": SOURCE, "files": 692, "patch_sha256": PATCH,
            "base_tree": BASE_TREE, "base_manifest_sha256": BASE, "changed_paths": sorted(paths)}


def rust_proof(logs, metal, archived):
    ext = ".gz" if archived else ""
    workspace, receipt = tests(logs / (WORKSPACE + ext))
    require(receipt == {"passed": 1598, "ignored": 20, "log_sha256": LOG_HASHES[WORKSPACE]}
            and "provenance:" in workspace
            and workspace.rsplit("\ntest result:", 1)[-1].count(f"Finished {chr(96)}dev{chr(96)} profile") == 3,
            "Wrong workspace, provenance or Wasm gates")
    gpu = {}
    for expected, name in zip((16, 2), GPU, strict=True):
        text, result = tests(logs / (name + ext))
        require(result == {"passed": expected, "ignored": 0, "log_sha256": LOG_HASHES[name]}
                and text.count("backend: Metal") >= 2 and 'name: "Apple M1 Max"' in text,
                "Wrong actual Apple Metal gate")
        gpu[name] = result
    require(sha256(raw_log(logs / (BUILD + ext))).hexdigest() == LOG_HASHES[BUILD], "Wrong original exact build log")
    entries = path_entries(checked(metal / "SHA256SUMS", GPU_IMAGES), allow_dot_prefix=True)
    require(len(entries) == 21 and entries.keys() == {p.name for p in metal.glob("*.png")},
            "Wrong Metal originals")
    verify_entries(metal, entries)
    for file in entries:
        with Image.open(metal / file) as image:
            require(image.format == "PNG", "Wrong Metal screenshot format")
            image.load()
    return {"workspace": {**receipt, "wasm_checks": 3}, "metal": {"passed": 18, "backend": "Metal", "logs": gpu},
            "metal_pngs": entries, "canonical_build_log_sha256": LOG_HASHES[BUILD]}


def native_proof(path, helpers, binary_manifest):
    seal(helpers, "SHA256SUMS", NATIVE_TOOLS)
    result = subprocess.run(
        [sys.executable, "-E", "-B", str(helpers / "replay-horizontal-nav.py"), "--readback", str(path),
         "--source-sha", SOURCE, "--binary-sha", BINARY, "--evidence-sha", NATIVE_PROOF],
        check=True, capture_output=True, text=True,
    )
    result = json.loads(result.stdout)
    original = data(path / "native-horizontal-nav-report.json")
    require(result["original_pngs"] == 4 and result["window_samples"] == 20
            and result["reaped"] == 4 and result["visible_windows"] == 0
            and original["source"]["file_count"] == 692
            and original["binaries"]["files"] == path_entries(checked(binary_manifest, BINARY)),
            "Wrong native source or two-run proof")
    return result


def toolkit(folder):
    return run_path(str(checked(folder / "verify-navigation-pair.py", FOCUS_VERIFIER)))


def navigation(artifacts, tools):
    old = tools["OLD"]
    compact_path = artifacts / BROWSER_REPORTS[0]
    report = data(compact_path)
    browser, adapter = old["metadata"](report)
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False and report["errors"] == []
            and report["reduced_motion"] is True and not {"failure", "state_at_failure"} & report.keys(),
            "Wrong Mac WebGPU adapter, reduced motion or page errors")
    return normalized({"browser": browser, "adapter": adapter, "page_errors": 0,
                       "compact": {"report_sha256": digest(compact_path), **old["validate_navigation"](report, artifacts)},
                       "horizontal": tools["medium"](artifacts, report)})


def windows(run, roles):
    watch = data(run / WINDOW)
    clean_windows(watch)
    verify_ledgers(run, watch, roles)
    require("CoreGraphics" in watch["method"], "No owned-window observer")
    for event in watch["guard_events"]:
        if event["role"].startswith("chrome-"):
            require(event["argv"].count(UPDATER) == 1 and "--" not in event["argv"], "Wrong Chrome flag evidence")
    if "chrome-bridge" in roles:
        require(watch["launch_observations"]["chrome-bridge"]["detached_samples"] > 0,
                "No owned detached Bridge evidence")
    return {"sha256": digest(run / WINDOW), "launches": watch["launch_observations"],
            "visible_windows": 0, "remaining_owned_processes": 0}


def focus_proof(folder, tools, exact):
    manifest = ("original-source.sha256", "focused-source.sha256", "web-producer-source.sha256")
    original, selected, producer = [path_entries(checked(folder / name, sha)) for name, sha in
                                    zip(manifest, (FOCUS_SOURCE, FOCUS_SELECTOR, FOCUS_WEB_SOURCE), strict=True)]
    require(original.keys() == selected.keys() == producer.keys()
            and len(original) == 692 and delta(original, selected) == ["e2e/src/material.rs"]
            and delta(producer, original) == ["e2e/src/material_horizontal_navigation_bar.rs"]
            and delta(path_entries(exact), original) == ["docs/material-coverage.json", "e2e/src/material_text_field_system_pair.rs"],
            "Wrong approved focused, Web-producer or stock source delta")
    actual = data(checked(folder / R3_REPORT, FOCUS_RESULT))
    run = folder / "run"
    invocation = data(run / "invocation.json")
    inputs = invocation["inputs"]
    require(actual["inputs"] == inputs == data(run / "inputs-after.json")
            and inputs["product_source_manifest_sha256"] == FOCUS_SOURCE
            and inputs["focused_source_manifest_sha256"] == FOCUS_SELECTOR
            and inputs["web_producer_manifest_sha256"] == FOCUS_WEB_SOURCE
            and inputs["web_manifest_sha256"] == FOCUS_WEB
            and inputs["web_files"] == path_entries(checked(folder / "web.sha256", FOCUS_WEB), allow_dot_prefix=True)
            and invocation["suite_arguments"] == ["material"] and invocation["injected_chrome_flags"] == [UPDATER],
            "Wrong focused launch or actual Gallery asset pin")
    photos = navigation(run / "artifacts", tools)
    result = tools["OLD"]["runner_result"]((run / "runner.log").read_text())
    starts, cases, steps, summary, browsers = result
    watch = windows(run, {"chrome-primary"})
    require(starts == ["material"] and cases == [("material", "7", "1041")]
            and steps == actual["steps"] == FOCUS_STEPS and actual["checks"] == 1041
            and summary == (1, 0, 1041) and browsers == {actual["browser"]}
            and photos == {**{key: actual[key] for key in ("compact", "horizontal", "browser", "adapter")}, "page_errors": 0}
            and watch["launches"]["chrome-primary"]["samples"] == 58
            and watch["sha256"] == actual["headless"]["sha256"]
            and data(run / "owned-shutdown.json")["after_cleanup"] == []
            and digest(run / "runner.log") == actual["runner_log_sha256"],
            "R3 actual log, pixels or headless receipt differ")
    return {"checks": 1041, "steps": steps, "pngs": 7, "headless": watch,
            "receipt_sha256": FOCUS_RESULT, "gallery_files": inputs["web_files"], "navigation": photos}


def stock_terminal(root):
    require((root / STOCK_MARKER).is_file() and (root / STOCK_MARKER).read_bytes() == b"",
            "Exact terminal stock marker missing")
    verify_entries(root, STOCK_TERMINAL)


def stock_proof(root, tools, focused):
    stock_terminal(root)
    run = root / "run"
    raw, independent = data(run / STOCK_REPORT), data(root / OFFLINE)
    guard = windows(run, {"chrome-primary", "chrome-bridge", "native"})
    log = (run / "runner.log").read_text()
    starts, cases, steps, summary, browsers = tools["OLD"]["runner_result"](log)
    counts = {case: {"steps": int(count), "checks": int(checks)} for case, count, checks in cases}
    require(starts == [row[0] for row in cases] == SUITES
            and steps == [f"M{index}" for index in range(1, 366) if index != 23]
            and counts["material"]["steps"] == 364 and counts["ime"] == {"steps": 20, "checks": 77}
            and re.findall(r"^  \[I(\d+)\]", log, re.M) == [str(index) for index in range(1, 21)]
            and counts["bridge-web"]["steps"] == 24 and counts["devtools-headless"]["steps"] == 3
            and summary == (5, 0, 30490) and sum(entry["checks"] for entry in counts.values()) == 30490
            and browsers == {raw["browser"]} and counts == raw["cases"] and raw["total_checks"] == 30490,
            "Wrong full stock/IME log scope")
    nav = navigation(run / "artifacts", tools)
    require(nav == raw["navigation_bars"] and all(independent[key] == value for key, value in raw.items())
            and raw["guard_and_window_checks"] == "passed" and raw["page_errors"] == 0
            and independent["window_receipt_sha256"] == guard["sha256"]
            and independent["runner_log_sha256"] == STOCK_TERMINAL["run/runner.log"],
            "Independent stock Navigation replay differs")
    binary = path_entries(checked(root / "browser-binary-sha256.txt", STOCK_ASSETS))
    invocation = data(run / "invocation.json")
    inputs = invocation["inputs"]
    require(inputs == data(run / "inputs-after.json") == data(root / "inputs-before.json") == data(root / "inputs-rechecked.json")
            and inputs["stock_files"] == binary and len(binary) == 16
            and inputs["runner_sha256"] == binary["debug/surface-e2e"] == STOCK_RUNNER
            and inputs["source_manifest_sha256"] == SOURCE and inputs["source_files_checked_per_origin"] == 692
            and inputs["release"] == independent["release"] and inputs["release"]["source_tree"] == TREE
            and inputs["app_bundle_seal_sha256"] == STOCK_SEAL and invocation["suite_arguments"] == SUITES
            and invocation["injected_chrome_flags"] == [UPDATER], "Wrong stock source, run or executable receipt")
    gallery = {Path(name).name: sha for name, sha in binary.items()
               if Path(name).parent.as_posix() == "web/release/material-gallery"}
    require(gallery == focused["gallery_files"] == independent["focused"]["canonical_gallery_files"]
            and len(gallery) == 4 and independent["focused"]["canonical_gallery_match"] is True,
            "Stock Gallery differs from focused R3")
    inherited = sorted(independent["retained"]["retained"])
    require({"divider", "ime_cancellation", "navigation", "menu", "switch", "checkbox", "radio", "tabs"}
            <= set(inherited)
            and len(raw["text_field_system_pair"]["png_sha256"]) == 14
            and len(raw["text_field_disabled_contrast"]["png_sha256"]) == 15
            and len(raw["text_field_boundary_motion"]["pngs"]) == 33
            and len(raw["text_field_label_motion"]["pngs"]) == 34
            and len(raw["text_field_placeholder"]["pngs"]) == 58, "An inherited stock receipt is missing")
    index = data(root / "original-artifact-digests.json")
    present = {file.name: digest(file) for file in (run / "artifacts").iterdir() if file.is_file()}
    require(all(index[name] == sha for name, sha in present.items()), "Portable original stock bytes differ from the index")
    return {"checks": 30490, "cases": counts, "material_steps": 364, "browser": nav["browser"],
            "adapter": nav["adapter"], "navigation": nav, "headless": guard,
            "raw_pins": STOCK_TERMINAL, "original_artifacts_indexed": len(index),
            "retained_validated": inherited, "previous_published_evidence": PREVIOUS}


def preflight():
    source = source_proof(Q73 / "base-exact.sha256", EXACT / "source.sha256", Q73 / PATCH_NAME)
    files = path_entries(EXACT / "source.sha256")
    require(source_tree(EXACT / "source", files) == TREE, "Exact source bytes or file modes differ")
    require(all(((EXACT if name.endswith("built") else Q73) / name).read_text() == TREE + "\n"
                for name in MARKERS), "Exact build or tested source marker missing")
    seal(FOCUS, "launcher.sha256", FOCUS_TOOLS)
    seal(STOCK, "harness.sha256", STOCK_TOOLS)
    seal(STOCK, "sealed-inputs.sha256", STOCK_SEAL)
    checked(LEGACY / "verify-existing.py", LEGACY_VERIFIER)
    rust = rust_proof(Q73, Q73 / "gpu-v0173-exact", archived=False)
    native = native_proof(NATIVE / "exact-v0173", NATIVE, EXACT / "binaries/SHA256SUMS")
    before = subprocess.run(
        [sys.executable, "-E", "-B", str(STOCK / "observe-multiline.py"), "--verify-inputs-only",
         "--runner", str(STOCK / "bundle/debug/surface-e2e"), "--runner-sha256", STOCK_RUNNER],
        check=True, capture_output=True, text=True,
    )
    require(json.loads(before.stdout) == data(STOCK / "inputs-before.json"), "Exact stock preflight differs")
    scoped = data(checked(FOCUS / R3_REPORT, FOCUS_RESULT))
    require(navigation(FOCUS / "development-r3/artifacts", toolkit(FOCUS))["horizontal"] == scoped["horizontal"],
            "Actual R3 paint replay differs")
    return {"status": "offline preflight", "source": source, "rust": rust, "native": native,
            "focused_checks": scoped["checks"], "stock_terminal": (STOCK / STOCK_MARKER).exists()}


def copy_run(run, out, extra=()):
    copy_members(run, out, (*PROOF_FILES, *extra))
    copy_members(run, out, [file.relative_to(run).as_posix() for file in (run / "guard-ledger").glob("*.json")])
    artifacts = run / "artifacts"
    images = [file.name for pattern in ("material-flexible-navigation-bar-*.png",
                                      "material-horizontal-navigation-bar-*.png") for file in artifacts.glob(pattern)]
    copy_members(artifacts, out / "artifacts", (*BROWSER_REPORTS, *images))


def archived_proof(root):
    require(all((root / "gates" / name).read_text() == TREE + "\n" for name in MARKERS),
            "Wrong portable build markers")
    native = native_proof(root / "native/exact", root / "native/tools", root / "native/binaries.sha256")
    tools = toolkit(root / "browser-tools")
    focused = focus_proof(root / "focused", tools, root / "source.sha256")
    stock = stock_proof(root / "stock-macos", tools, focused)
    inventory = path_entries(checked(root / "stock-macos/harness.sha256", STOCK_TOOLS))
    verify_entries(root / "stock-macos/tools", inventory)
    checked(root / "stock-macos/sealed-inputs.sha256", STOCK_SEAL)
    prior = checked(root / "stock-macos/legacy/verify-existing.py", LEGACY_VERIFIER)
    inherited = path_entries(checked(prior.parent / "retained/SHA256SUMS", LEGACY_MANIFEST))
    verify_entries(prior.parent / "retained", inherited)
    return {"status": "verified exact local candidate",
            "source": source_proof(root / "base-source.sha256", root / "source.sha256", root / PATCH_NAME),
            "rust": rust_proof(root / "rust", root / "rust/metal", archived=True),
            "native": native, "focused": focused, "stock_macos": stock,
            "inherited_decoders": {"stock_harness_sha256": STOCK_TOOLS,
                                   "legacy_verifier_sha256": LEGACY_VERIFIER,
                                   "legacy_manifest_sha256": LEGACY_MANIFEST}}


def export(root):
    stock_terminal(STOCK)
    preflight()
    execution = subprocess.run(
        [sys.executable, "-E", "-B", str(STOCK / "verify-stock-receipt.py"), str(STOCK / "run")],
        check=True, capture_output=True,
    ).stdout
    require(execution == (STOCK / OFFLINE).read_bytes(), "Exact complete stock readback differs")
    stock_terminal(STOCK)
    root.mkdir()
    copy(Q73 / "base-exact.sha256", root / "base-source.sha256")
    copy_members(EXACT, root, ("source.sha256", "source.tree"))
    copy(Q73 / PATCH_NAME, root / PATCH_NAME)
    copy(Path(__file__), root / Path(__file__).name)
    copy(Path(__file__).with_name("evidence_common.py"), root / "evidence_common.py")
    for name in MARKERS:
        copy((EXACT if name.endswith("built") else Q73) / name, root / "gates" / name)
    for name in LOG_HASHES:
        compressed(Q73 / name, root / "rust" / (name + ".gz"))
    metal = Q73 / "gpu-v0173-exact"
    copy_members(metal, root / "rust/metal", ("SHA256SUMS", *path_entries(metal / "SHA256SUMS", allow_dot_prefix=True)))
    copy(EXACT / "binaries/SHA256SUMS", root / "native/binaries.sha256")
    copy_members(NATIVE, root / "native/tools", ("SHA256SUMS", *path_entries(NATIVE / "SHA256SUMS")))
    native = NATIVE / "exact-v0173"
    copy_members(native, root / "native/exact", ("SHA256SUMS", *path_entries(native / "SHA256SUMS")))
    copy_members(FOCUS, root / "browser-tools", ("verify-navigation-pair.py", "verify-compact-predecessor.py", "prepare-focused.py"))
    focus = root / "focused"
    copy_members(FOCUS, focus, ("original-source.sha256", "focused-source.sha256", R3_REPORT))
    copy(Q73 / "development-r2/source.sha256", focus / "web-producer-source.sha256")
    copy(Q73 / "web-development.sha256", focus / "web.sha256")
    copy_run(FOCUS / "development-r3", focus / "run", ("owned-shutdown.json",))
    stock = root / "stock-macos"
    copy_members(STOCK, stock, ("harness.sha256", "sealed-inputs.sha256", "browser-binary-sha256.txt",
                               "inputs-before.json", "inputs-rechecked.json", OFFLINE, STOCK_MARKER))
    copy_members(STOCK, stock / "tools", path_entries(STOCK / "harness.sha256"))
    copy_members(LEGACY, stock / "legacy", ("verify-existing.py", "retained/SHA256SUMS"))
    copy_members(LEGACY / "retained", stock / "legacy/retained", path_entries(LEGACY / "retained/SHA256SUMS"))
    copy_run(STOCK / "run", stock / "run", (STOCK_REPORT,))
    original = STOCK / "run/artifacts"
    copy_members(original, stock / "run/artifacts", INHERITED_REPORTS)
    json_file(stock / "original-artifact-digests.json", {
        file.relative_to(original).as_posix(): digest(file)
        for file in sorted(original.rglob("*"))
        if file.is_file() and not file.is_symlink() and file.suffix in {".json", ".png"}
    })
    proof = archived_proof(root)
    json_file(root / REPORT, proof)
    (root / "README.md").write_text(
        f"# Surface v0.1.73 Horizontal Navigation evidence\n\n"
        f"Exact candidate tree {TREE}. Five macOS headless stock suites passed "
        f'{proof["stock_macos"]["checks"]:,} checks, including 364 Material steps through M365. '
        "The packet retains the full runner log and original window and process receipts. "
        "It independently decodes seven Navigation PNGs from stock and seven from scoped R3. "
        "It also decodes four exact native originals and verifies 18 Apple Metal tests and 21 GPU PNGs.\n\n"
        "The inherited decoders ran on the original stock outputs before export. "
        "Their scripts, raw report hashes and the published q72 evidence identify the inherited coverage. "
        "This packet retains all Navigation PNGs and selected inherited reports. "
        "The complete original log and artifact index preserve the rest of the stock scope.\n\n"
        f"[Published q72 Field-pair evidence]({PREVIOUS})\n\n"
        "Run the following with Python 3.12 and Pillow. Use the reviewed digest of SHA256SUMS.\n\n"
        "    python3 -E -B pack-horizontal-nav.py --verify-package . --package-sha REVIEWED_SHA\n\n"
        "Offline verification needs no source or build directories. "
        "The packet contains no app binaries, browser profiles or development session credentials.\n",
    )
    manifest = root / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(file)}  {file.relative_to(root).as_posix()}\n"
                                for file in sorted(root.rglob("*")) if file.is_file()))
    return digest(manifest)


def verify(root, expected):
    seal(root, "SHA256SUMS", expected, exact=True)
    result = archived_proof(root)
    require(result == data(root / REPORT), "Portable recheck differs from the archived verdict")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--preflight", action="store_true")
    mode.add_argument("--export", type=Path)
    mode.add_argument("--verify-package", type=Path)
    parser.add_argument("--package-sha")
    args = parser.parse_args()
    if args.preflight:
        result = preflight()
    elif args.export:
        output = args.export.resolve()
        require(output.is_relative_to(Path("/tmp").resolve()) and output.parent.is_dir() and not output.exists(),
                "Use a fresh output under /tmp")
        expected = export(output)
        verify(output, expected)
        result = {"portable_path": str(output), "manifest_sha256": expected,
                  "files": len(path_entries(output / "SHA256SUMS"))}
    else:
        result = verify(args.verify_package.resolve(strict=True), args.package_sha)
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
