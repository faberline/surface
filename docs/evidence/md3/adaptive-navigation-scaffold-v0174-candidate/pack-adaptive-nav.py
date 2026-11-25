#!/usr/bin/env python3
"""Export or recheck the exact local Surface v0.1.74 Adaptive Navigation evidence."""

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
    import_from, normalized, path_entries, raw_log, require, seal, source_tree, tests, verify_entries,
    verify_ledgers,
)


Q74 = Path("/tmp/surface-md3-adaptive-nav-v0174.b8nBXDdY")
EXACT = Q74 / "exact-candidate"
STOCK = Path("/tmp/surface-md3-adaptive-nav-stock-v0174.j8xout6u")
FOCUS = Path("/tmp/surface-md3-adaptive-nav-focused-v0174-r4.L4lJahZo")
NATIVE = Path("/tmp/surface-md3-adaptive-nav-native-v0174-r2.BdPD52tg")
LEGACY = Path("/tmp/surface-md3-sliders-stock.lK6a9gKl")
Q73_FOCUS = Path("/tmp/surface-md3-horizontal-nav-focused-v0173-r3.KYlcfBAK")
REJECTED = Path("/tmp/surface-md3-adaptive-nav-focused-v0174-r3.Ko7vpLbe/development-r3")
SOURCE = "7aedeb9161baadca46aa6fdd83848b7e9c5498bccfac0e9a0e21e2782ec3f0bd"
TREE = "f7a92f77ecc9b37c91d81ace4877cd8038044d5c"
BASE = "72716762c2e2615a92e29dc45c840e0c7278cafbf31d489d165aa92b20556e14"
BASE_TREE = "5d5ab69f57bdb3f31a8db95ae78f9f332789578b"
PATCH = "226a1fb9d70d2358578d662322f89530bab0abef80e86e7832ff467a611810d5"
PATCH_NAME = "adaptive-navigation-v0174.patch"
BINARY = "bf2732532f164a4b8a050d48da85e9f7291a303e30a4eeacf7eed0b94b7efa57"
NATIVE_PROOF = "1662f8a29a28e838a21461b0431084596b7a33afc99c60fd6b42b93e86f77346"
NATIVE_TOOLS = "bd97d1e03f12bf9bc4d2afb9c4c16938c471800dceb05367597aeff08120ac20"
FOCUS_TOOLS = "1248f280ed8cdd7ddc130a753fbb0634ae83deb5cf5d107f746555625f2b5a66"
FOCUS_RESULT = "9db4a851e91efe79d448d4f69737aba9950ec3e759cff3b9af6014f77aabc4e0"
FOCUS_SOURCE = "7a2f1480c1d21a5b5b384bb04f3c52a396ac39c51e5ccf2ffabd97de241d21ab"
FOCUS_SELECTOR = "4e826921a5a7d664d0672b49e5e5df08b8e82e49ddf0c3233d3c6a6f820ad40a"
FOCUS_WEB_SOURCE = "8e3accebefb67a07449011ff637f76e8a717f629bd9e595e526cadac3833e160"
FOCUS_WEB = "b55cb95697b45fe40a5d937eeb5976c288aed4c1d3716a35d7681d3766768897"
STOCK_TOOLS = "7035590a38eab3c58a4f4dc1173e5afc805bd037290678203cb9fdc171fd59eb"
STOCK_SEAL = "50f6c2074aeee433941a49cf2003783702b3503586f9934007d45cbd600863db"
STOCK_ASSETS = "6f2e9d9a94f9312bac70edd5bca437ff82592c7e73710f5714f4999108a037d7"
STOCK_RUNNER = "95586e3e5253eef3433d8709eb201dab59be7bdc626852c5891f69901891e186"
STOCK_REVIEWED = "02e45132c3326256258e83b7f218f3c87179133ed7c650633561a367c0bff7e6"
STOCK_ADAPTIVE = "6b83a0bada18448e618fb38ff085ed52df7bdfc8e60ff8be936a62d3326a2d63"
LEGACY_VERIFIER = "eded5dfdfcf94dc901cb3608b83cc86a557fe4cb02b91122416c6a03e3568558"
LEGACY_MANIFEST = "5a9c99e2bf64a76375d81d5f16ab58228410be56ba32225b01930df949a0ac6f"
WORKSPACE = "workspace-v0174-exact.log"
BUILD = "stock-build-v0174-exact.log"
GPU = ("gpu-v0174-exact.log", "gpu-v0174-supplemental.log")
LOG_HASHES = {
    WORKSPACE: "96cfbe2a0b8189a1ee87ecf8463c9e4aa58d3182e219a4eddbbbd6950749757e",
    BUILD: "48d167a69de563bc37564cf769c9231b1f9d37afddfa908f6dee74344892d3c3",
    GPU[0]: "0b65687198b06e2995f994ced6409a3ce67eb783dae32dafcffa8973dc5ae12c",
    GPU[1]: "49b9eca9d796504b2b49b2b33fed07fd4d7d1f8d5124434c273e04388817e4b9",
}
GPU_IMAGES = "c7fb34d394d7702a9e9f6e8c617d552486dfb52bf766fce7bc4c8259440c409f"
STOCK_MARKER = "stock-mac-v0174-passed"
STOCK_REPORT = "verified-adaptive-navigation-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
WINDOW = "headless-window-observation.json"
REPORT = "verified-adaptive-navigation-v0174.json"
R4_REPORT = "verified-adaptive-navigation-r4.json"
STOCK_TERMINAL = (f"run/{STOCK_REPORT}", OFFLINE, "run/runner.log", f"run/{WINDOW}")
FOCUS_STEPS = [f"M{index}" for index in range(366, 370)]
SUITES = ["smoke", "ime", "material", "bridge-web", "devtools-headless"]
UPDATER = "--disable-updater-scheduler"
PREVIOUS = "https://github.com/faberline/surface/tree/961ab6fe023a4d883e0aa6843854429060a2e402/docs/evidence/md3/horizontal-navigation-bar-v0173-candidate"
ADAPTIVE_CASE = "e2e/src/material_adaptive_navigation_scaffold.rs"
ADAPTIVE_REPORT = "material-adaptive-navigation.json"
BROWSER_REPORTS = ("browser-material-flexible-navigation-bar.json", "material-horizontal-navigation-bar.json")
PROOF_FILES = ("runner.log", WINDOW, "invocation.json", "inputs-after.json")
MARKERS = ("workspace-v0174-exact-passed", "gpu-v0174-passed", "native-final-built", "stock-canonical-built")
COMPARED = ("native-development-comparison", "web-development-comparison")
INHERITED_REPORTS = ("browser-smoke.json", "browser-ime.json", "browser-material-side-sheet.json",
                     "bridge-web-result.json", "devtools-headless-result.json")
R3_LOG = "694383657fda4840b2c0aa41826bb64d929d2a5026dc568424b16913ae8cb1f6"
R3_WINDOW = "9dd981adc035de834e0f2e11cc9779baf1dbdee705f16c0caf02d36e94bc17cd"
Q73_VERIFIER = "a81f98dacae2c673587fdb156dd71f60b657c3d9bc6913ea87eaae646951625b"


def checked(path, sha):
    require(path.is_file() and not path.is_symlink() and digest(path) == sha, f"Wrong exact input {path}")
    return path


def delta(base, current):
    return sorted(name for name in base.keys() | current.keys() if base.get(name) != current.get(name))


def source_proof(base, source, patch):
    before, after = path_entries(checked(base, BASE)), path_entries(checked(source, SOURCE))
    paths = re.findall(r"^diff --git a/(\S+) b/\S+$", checked(patch, PATCH).read_text(), re.M)
    require(len(before) == 692 and len(after) == 696 and len(paths) == len(set(paths)) == 13
            and sorted(paths) == delta(before, after), "Wrong exact source delta or scope")
    return {"tree": TREE, "manifest_sha256": SOURCE, "files": 696, "patch_sha256": PATCH,
            "base_tree": BASE_TREE, "base_manifest_sha256": BASE, "changed_paths": sorted(paths)}


def rust_proof(logs, metal, archived):
    ext = ".gz" if archived else ""
    workspace, receipt = tests(logs / (WORKSPACE + ext))
    require(receipt == {"passed": 1603, "ignored": 20, "log_sha256": LOG_HASHES[WORKSPACE]}
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
        [sys.executable, "-E", "-B", str(helpers / "replay-adaptive-nav.py"), "--readback", str(path),
         "--source-sha", SOURCE, "--binary-sha", BINARY, "--evidence-sha", NATIVE_PROOF],
        check=True, capture_output=True, text=True,
    )
    result = json.loads(result.stdout)
    original = data(path / "native-adaptive-nav-report.json")
    require(result["original_pngs"] == 6 and result["window_samples"] == 30
            and result["reaped"] == 6 and result["visible_windows"] == 0
            and original["source"]["file_count"] == 696
            and original["binaries"]["files"] == path_entries(checked(binary_manifest, BINARY)),
            "Wrong native source or two-run proof")
    return result


def toolkit(folder):
    return run_path(str(checked(folder / "verify-navigation-pair.py", Q73_VERIFIER)))


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


def adaptive_module(tools_root):
    module = checked(tools_root / "verify_adaptive_navigation_stock.py", STOCK_ADAPTIVE)
    with import_from(tools_root):
        return run_path(str(module))


def focus_proof(folder, q73_tools, adaptive, exact):
    manifest = ("original-source.sha256", "focused-source.sha256", "web-producer-source.sha256")
    original, selected, producer = [path_entries(checked(folder / name, sha)) for name, sha in
                                    zip(manifest, (FOCUS_SOURCE, FOCUS_SELECTOR, FOCUS_WEB_SOURCE), strict=True)]
    require(original.keys() == selected.keys() == producer.keys()
            and len(original) == 696 and delta(original, selected) == ["e2e/src/material.rs"]
            and delta(producer, original) == [ADAPTIVE_CASE]
            and delta(path_entries(exact), original) == ["docs/material-coverage.json", ADAPTIVE_CASE],
            "Wrong approved R4, Web-producer or stock source delta")
    actual = data(checked(folder / R4_REPORT, FOCUS_RESULT))
    run = folder / "run"
    invocation = data(run / "invocation.json")
    inputs = invocation["inputs"]
    require(inputs == data(run / "inputs-after.json") and inputs["steps"] == FOCUS_STEPS
            and inputs["product_source_manifest_sha256"] == FOCUS_SOURCE
            and inputs["focused_source_manifest_sha256"] == FOCUS_SELECTOR
            and inputs["web_producer_manifest_sha256"] == FOCUS_WEB_SOURCE
            and inputs["web_test_only_delta"] == [ADAPTIVE_CASE]
            and inputs["web_manifest_sha256"] == FOCUS_WEB
            and inputs["web_files"] == path_entries(checked(folder / "web.sha256", FOCUS_WEB), allow_dot_prefix=True)
            and invocation["suite_arguments"] == ["material"] and invocation["injected_chrome_flags"] == [UPDATER],
            "Wrong focused launch or actual Gallery asset pin")
    starts, cases, steps, summary, browsers = q73_tools["OLD"]["runner_result"]((run / "runner.log").read_text())
    browser, = browsers
    photos = adaptive["verify"](run / "artifacts", {"browser": browser})
    watch = windows(run, {"chrome-primary"})
    require(starts == ["material"] and cases == [("material", "4", "200")]
            and steps == FOCUS_STEPS and actual["checks"] == 200
            and summary == (1, 0, 200) and len(photos["pngs"]) == 4
            and photos["pngs"] == actual["pictures"] and photos["draft"] == actual["draft"]
            and photos["rail_scroll"] == actual["rail_scroll"] and photos["page_scroll"] == actual["page_scroll"]
            and photos["report_sha256"] == actual["report_sha256"]
            and normalized(adaptive["tools"]()["verify"](run)) == actual
            and watch["launches"]["chrome-primary"]["samples"] == actual["window_samples"] == 27
            and data(run / "owned-shutdown.json")["after_cleanup"] == []
            and digest(run / "runner.log") == actual["runner_log_sha256"],
            "R4 actual log, pixels or headless receipt differ")
    return {"checks": 200, "steps": steps, "pngs": 4, "headless": watch,
            "receipt_sha256": FOCUS_RESULT, "gallery_files": inputs["web_files"], "navigation": photos}


def rejected_proof(root):
    log = checked(root / "runner.log", R3_LOG).read_text()
    watch = data(checked(root / WINDOW, R3_WINDOW))
    require("last step: M368" in log and "JSON.stringify" in log
            and "0 passed; 1 failed; 115 checks;" in log
            and watch["stock_exit"] == 1 and not watch["visible_owned_windows"]
            and not watch["remaining_observed_processes"] and not watch["problems"],
            "Wrong original R3 test-oracle failure")
    verify_ledgers(root, watch, {"chrome-primary"})
    photos = {}
    sizes = {"compact": (1198, 1328), "medium": (1200, 1328), "rail": (1680, 1328)}
    for tag, size in sizes.items():
        name = f"material-adaptive-navigation-{tag}.png"
        path = root / "artifacts" / name
        with Image.open(path) as image:
            require(image.format == "PNG" and image.size == size, "Wrong R3 original image")
            image.load()
        photos[name] = digest(path)
    return {"checks": 115, "step": "M368", "reason": "JSON object property order in the old assertion",
            "runner_log_sha256": R3_LOG, "window_receipt_sha256": R3_WINDOW, "pngs": photos}


def stock_terminal(root):
    require((root / STOCK_MARKER).is_file() and (root / STOCK_MARKER).read_bytes() == b"",
            "Exact terminal stock marker missing")
    return {name: digest(root / name) for name in STOCK_TERMINAL}


def stock_proof(root, tools, adaptive, focused, terminal):
    require(stock_terminal(root) == terminal, "The terminal stock receipts differ from the original seals")
    run = root / "run"
    raw, independent = data(run / STOCK_REPORT), data(root / OFFLINE)
    guard = windows(run, {"chrome-primary", "chrome-bridge", "native"})
    log = (run / "runner.log").read_text()
    starts, cases, steps, summary, browsers = tools["OLD"]["runner_result"](log)
    counts = {case: {"steps": int(count), "checks": int(checks)} for case, count, checks in cases}
    checks = sum(entry["checks"] for entry in counts.values())
    require(starts == [row[0] for row in cases] == SUITES
            and steps == [f"M{index}" for index in range(1, 370) if index != 23]
            and counts["material"]["steps"] == 368 and counts["ime"] == {"steps": 20, "checks": 77}
            and re.findall(r"^  \[I(\d+)\]", log, re.M) == [str(index) for index in range(1, 21)]
            and counts["bridge-web"]["steps"] == 24 and counts["devtools-headless"]["steps"] == 3
            and summary == (5, 0, checks) and checks > 30490
            and browsers == {raw["browser"]} and counts == raw["cases"] and raw["total_checks"] == checks,
            "Wrong full stock/IME log scope")
    nav = navigation(run / "artifacts", tools)
    scaffold = adaptive["verify"](run / "artifacts", {"browser": nav["browser"], "adapter": nav["adapter"]})
    require(scaffold == raw["adaptive_navigation"] and nav == raw["navigation_bars"] and all(independent[key] == value for key, value in raw.items())
            and raw["guard_and_window_checks"] == "passed" and raw["page_errors"] == 0
            and independent["window_receipt_sha256"] == guard["sha256"]
            and independent["runner_log_sha256"] == terminal["run/runner.log"],
            "Independent stock Navigation replay differs")
    binary = path_entries(checked(root / "browser-binary-sha256.txt", STOCK_ASSETS))
    invocation = data(run / "invocation.json")
    inputs = invocation["inputs"]
    require(inputs == data(run / "inputs-after.json") == data(root / "inputs-before.json") == data(root / "inputs-rechecked.json")
            and inputs["stock_files"] == binary and len(binary) == 16
            and inputs["runner_sha256"] == binary["debug/surface-e2e"] == STOCK_RUNNER
            and inputs["source_manifest_sha256"] == SOURCE and inputs["source_files_checked_per_origin"] == 696
            and inputs["release"] == independent["release"] and inputs["release"]["source_tree"] == TREE
            and inputs["app_bundle_seal_sha256"] == STOCK_SEAL and invocation["suite_arguments"] == SUITES
            and invocation["injected_chrome_flags"] == [UPDATER], "Wrong stock source, run or executable receipt")
    gallery = {Path(name).name: sha for name, sha in binary.items()
               if Path(name).parent.as_posix() == "web/release/material-gallery"}
    require(gallery == focused["gallery_files"] == independent["focused"]["canonical_gallery_files"]
            and len(gallery) == 4 and independent["focused"]["canonical_gallery_match"] is True,
            "Stock Gallery differs from focused R4")
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
    return {"checks": checks, "cases": counts, "material_steps": 368, "browser": nav["browser"],
            "adapter": nav["adapter"], "navigation": nav, "headless": guard,
            "adaptive_navigation": scaffold, "raw_pins": terminal, "original_artifacts_indexed": len(index),
            "retained_validated": inherited, "previous_published_evidence": PREVIOUS}


def preflight():
    source = source_proof(Q74 / "base.sha256", EXACT / "source.sha256", EXACT / PATCH_NAME)
    files = path_entries(EXACT / "source.sha256")
    require(source_tree(EXACT / "source", files) == TREE, "Exact source bytes or file modes differ")
    require(all(((EXACT if name.endswith("built") else Q74) / name).read_text() == TREE + "\n"
                for name in MARKERS), "Exact build or tested source marker missing")
    require(all((EXACT / name).read_text() == "identical\n" for name in COMPARED),
            "Exact native and regular Gallery must match the R2 builds")
    require((EXACT / "binaries/SHA256SUMS").read_bytes() == (Q74 / "native-development/SHA256SUMS").read_bytes(),
            "The exact native pair differs from the development manifest")
    seal(FOCUS, "launcher.sha256", FOCUS_TOOLS)
    seal(STOCK, "harness.sha256", STOCK_TOOLS)
    seal(STOCK, "sealed-inputs.sha256", STOCK_SEAL)
    checked(LEGACY / "verify-existing.py", LEGACY_VERIFIER)
    rust = rust_proof(Q74, Q74 / "gpu-v0174-exact", archived=False)
    native = native_proof(NATIVE / "exact-v0174", NATIVE, EXACT / "binaries/SHA256SUMS")
    before = subprocess.run(
        [sys.executable, "-E", "-B", str(STOCK / "observe-multiline.py"), "--verify-inputs-only",
         "--runner", str(STOCK / "bundle/debug/surface-e2e"), "--runner-sha256", STOCK_RUNNER],
        check=True, capture_output=True, text=True,
    )
    require(json.loads(before.stdout) == data(STOCK / "inputs-before.json"), "Exact stock preflight differs")
    archive = adaptive_module(STOCK)
    bound = archive["focused_receipt"](EXACT / "source.sha256", data(checked(STOCK / "authority-pins.json", STOCK_REVIEWED)),
                                       EXACT / "web-canonical/web/release/material-gallery")
    require(bound == data(STOCK / "focused-proof.json") and bound["checks"] == 200,
            "Actual R4 and canonical Gallery binding differs")
    return {"status": "offline preflight", "source": source, "rust": rust, "native": native,
            "focused_checks": bound["checks"], "rejected_r3": rejected_proof(REJECTED),
            "stock_terminal": (STOCK / STOCK_MARKER).exists()}


def copy_run(run, out, report_names, patterns, extra=()):
    copy_members(run, out, (*PROOF_FILES, *extra))
    copy_members(run, out, [file.relative_to(run).as_posix() for file in (run / "guard-ledger").glob("*.json")])
    artifacts = run / "artifacts"
    images = [file.name for pattern in patterns for file in artifacts.glob(pattern)]
    copy_members(artifacts, out / "artifacts", (*report_names, *images))


def archived_proof(root):
    require(all((root / "gates" / name).read_text() == TREE + "\n" for name in MARKERS)
            and all((root / "gates" / name).read_text() == "identical\n" for name in COMPARED),
            "Wrong portable build markers or R2 asset comparison")
    native = native_proof(root / "native/exact", root / "native/tools", root / "native/binaries.sha256")
    tools = toolkit(root / "browser-tools")
    stock_tools = root / "stock-macos/tools"
    inventory = path_entries(checked(root / "stock-macos/harness.sha256", STOCK_TOOLS))
    verify_entries(stock_tools, inventory)
    adaptive = adaptive_module(stock_tools)
    focused = focus_proof(root / "focused", tools, adaptive, root / "source.sha256")
    stock = stock_proof(root / "stock-macos", tools, adaptive, focused, data(root / "terminal-pins.json"))
    inputs = path_entries(checked(root / "stock-macos/sealed-inputs.sha256", STOCK_SEAL))
    common = {name: sha for name, sha in inputs.items() if name in inventory}
    require(all(inventory[name] == sha for name, sha in common.items()),
            "The stock tool and build seals disagree")
    verify_entries(root / "stock-macos", {name: sha for name, sha in inputs.items() if name not in inventory})
    cfg = data(checked(root / "stock-macos/reviewed-pins.json", STOCK_REVIEWED))
    require(cfg["source_sha256"] == SOURCE and cfg["source_tree"] == TREE and cfg["stock_runner_sha256"] == STOCK_RUNNER
            and cfg["native_manifest_sha256"] == BINARY and cfg["stock_assets_sha256"] == STOCK_ASSETS,
            "Wrong portable reviewed stock inputs")
    require((root / "native/development-binaries.sha256").read_bytes() == (root / "native/binaries.sha256").read_bytes(),
            "Exact native comparison manifest differs")
    prior = checked(root / "stock-macos/legacy/verify-existing.py", LEGACY_VERIFIER)
    inherited = path_entries(checked(prior.parent / "retained/SHA256SUMS", LEGACY_MANIFEST))
    verify_entries(prior.parent / "retained", inherited)
    return {"status": "verified exact local candidate",
            "source": source_proof(root / "base-source.sha256", root / "source.sha256", root / PATCH_NAME),
            "rust": rust_proof(root / "rust", root / "rust/metal", archived=True),
            "native": native, "focused": focused, "rejected_r3": rejected_proof(root / "rejected/browser-r3"),
            "stock_macos": stock, "r2_gallery_and_native_assets_match": True,
            "inherited_decoders": {"stock_harness_sha256": STOCK_TOOLS,
                                   "legacy_verifier_sha256": LEGACY_VERIFIER,
                                   "legacy_manifest_sha256": LEGACY_MANIFEST}}


def export(root):
    terminal = stock_terminal(STOCK)
    preflight()
    execution = subprocess.run(
        [sys.executable, "-E", "-B", str(STOCK / "verify-stock-receipt.py"), str(STOCK / "run")],
        check=True, capture_output=True,
    ).stdout
    require(execution == (STOCK / OFFLINE).read_bytes() and stock_terminal(STOCK) == terminal,
            "Exact complete stock readback differs")
    root.mkdir()
    copy(Q74 / "base.sha256", root / "base-source.sha256")
    copy_members(EXACT, root, ("source.sha256", "source.tree", PATCH_NAME))
    copy(Path(__file__), root / Path(__file__).name)
    copy(Path(__file__).with_name("evidence_common.py"), root / "evidence_common.py")
    for name in (*MARKERS, *COMPARED):
        copy((EXACT if name.endswith("built") or name in COMPARED else Q74) / name, root / "gates" / name)
    for name in LOG_HASHES:
        compressed(Q74 / name, root / "rust" / (name + ".gz"))
    metal = Q74 / "gpu-v0174-exact"
    copy_members(metal, root / "rust/metal", ("SHA256SUMS", *path_entries(metal / "SHA256SUMS", allow_dot_prefix=True)))
    copy(EXACT / "binaries/SHA256SUMS", root / "native/binaries.sha256")
    copy(Q74 / "native-development/SHA256SUMS", root / "native/development-binaries.sha256")
    copy_members(NATIVE, root / "native/tools", ("SHA256SUMS", *path_entries(NATIVE / "SHA256SUMS")))
    native = NATIVE / "exact-v0174"
    copy_members(native, root / "native/exact", ("SHA256SUMS", *path_entries(native / "SHA256SUMS")))
    copy_members(Q73_FOCUS, root / "browser-tools", ("verify-navigation-pair.py", "verify-compact-predecessor.py", "prepare-focused.py"))
    focus = root / "focused"
    copy_members(FOCUS, focus, ("focused-source.sha256", R4_REPORT))
    copy(Q74 / "development-r4/source.sha256", focus / "original-source.sha256")
    copy(Q74 / "development-r2/source.sha256", focus / "web-producer-source.sha256")
    copy(Q74 / "web-development.sha256", focus / "web.sha256")
    adaptive_patterns = ("material-adaptive-navigation-*.png",)
    copy_run(FOCUS / "development-r4", focus / "run", (ADAPTIVE_REPORT,), adaptive_patterns, ("owned-shutdown.json",))
    copy_run(REJECTED, root / "rejected/browser-r3", (), adaptive_patterns, ("owned-shutdown.json",))
    stock = root / "stock-macos"
    copy_members(STOCK, stock, ("harness.sha256", "sealed-inputs.sha256", "browser-binary-sha256.txt",
                               "inputs-before.json", "inputs-rechecked.json", OFFLINE, STOCK_MARKER))
    inventory = path_entries(STOCK / "harness.sha256")
    copy_members(STOCK, stock / "tools", inventory)
    copy_members(STOCK, stock, path_entries(STOCK / "sealed-inputs.sha256").keys() - inventory.keys())
    copy_members(LEGACY, stock / "legacy", ("verify-existing.py", "retained/SHA256SUMS"))
    copy_members(LEGACY / "retained", stock / "legacy/retained", path_entries(LEGACY / "retained/SHA256SUMS"))
    old_patterns = ("material-flexible-navigation-bar-*.png", "material-horizontal-navigation-bar-*.png")
    copy_run(STOCK / "run", stock / "run", (*BROWSER_REPORTS, ADAPTIVE_REPORT), (*old_patterns, *adaptive_patterns), (STOCK_REPORT,))
    original = STOCK / "run/artifacts"
    copy_members(original, stock / "run/artifacts", INHERITED_REPORTS)
    json_file(stock / "original-artifact-digests.json", {
        file.relative_to(original).as_posix(): digest(file)
        for file in sorted(original.rglob("*"))
        if file.is_file() and not file.is_symlink() and file.suffix in {".json", ".png"}
    })
    json_file(root / "terminal-pins.json", terminal)
    proof = archived_proof(root)
    json_file(root / REPORT, proof)
    (root / "README.md").write_text(
        f"# Surface v0.1.74 Adaptive Navigation evidence\n\n"
        f"Exact candidate tree {TREE}. Five macOS headless stock suites passed "
        f'{proof["stock_macos"]["checks"]:,} checks, including 368 Material steps through M369. '
        "The packet retains the full runner log and original window and process receipts. "
        "It independently decodes four Adaptive and seven inherited Navigation PNGs from stock, "
        "plus four from scoped R4. It verifies six exact native originals, 1,603 Rust tests, three Wasm checks "
        "and 18 Apple Metal tests with 21 GPU PNGs.\n\n"
        "Native uses three fixed viewport profiles. The browser exercises real viewport changes, "
        "stable backward draft selection and independent rail scroll. "
        "The preserved R3 browser failure belongs to its old JSON key-order assertion.\n\n"
        "The inherited decoders ran on the original stock outputs before export. "
        "The packet includes their scripts, selected reports and the complete original stock PNG/JSON hash index.\n\n"
        f"[Published q73 Horizontal Navigation evidence]({PREVIOUS})\n\n"
        "Run the following with Python 3.12 and Pillow. Use the reviewed digest of SHA256SUMS.\n\n"
        "    python3 -E -B pack-adaptive-nav.py --verify-package . --package-sha REVIEWED_SHA\n\n"
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
