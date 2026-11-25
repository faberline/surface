"""Assemble and verify exact v0.1.71 evidence from original local artifacts."""

import argparse
from contextlib import contextmanager
from hashlib import sha256
import json
from pathlib import Path, PurePosixPath
import re
import runpy
import sys

from PIL import Image

from evidence_common import (HEX, clean_windows, compressed, copy, copy_members, data, digest, import_from,
                             json_file, normalized, path_entries, raw_log, require, run_json, seal,
                             source_tree, tests, verify_entries, verify_ledgers)

HERE = Path(__file__).resolve().parent
Q = Path("/tmp/surface-md3-field-contrast-v0171.xE0ruhQh")
EXACT = Q / "exact-candidate"
STOCK = Path("/tmp/surface-md3-field-contrast-stock-v0171.PykYn5L5")
FOCUS = Path("/tmp/surface-md3-field-contrast-focused-v0171-r2.ik1t5atk")
NATIVE = Path("/tmp/surface-md3-field-disabled-contrast-native-r3.SXfqVJZd")
NATIVE_HELPERS = Path("/tmp/surface-md3-field-boundary-native.705YkFQO")
PREV = Path("/tmp/surface-md3-boundary-v0170-package.pxSuFZW5/portable-r2")
SOURCE = "65c1bf0fd49f018b575cf069af016aa46229cff9dc23d521942f0ba0b6465ee2"
TREE = "f2b631b0d72830f76b84e71b114018a4410dc813"
PATCH = "9c3e271346b5285bddfd1c10586bed2c5c341b0f434aeb0dbc40c2c8dc92b578"
PATCH_NAME = "text-field-disabled-contrast-v0171.patch"
PREV_SOURCE = "bc6f3023d68c8bec8c801c7a9a2d7c88207d625697cffb34f1eb4f5fef5a6d85"
PREV_TREE = "409a21f6023802f055371cdec4391976c55ffd17"
PREV_SEAL = "5768509d54c91d83054cf12afff6a3eee8070c333dfa29a5324ee67077fc5a48"
PREV_REPORT = "cbf4fb4e20bba3a9ef27d0a921a230b4f432168d5950dfa604096bb86c975843"
PREV_OFFLINE = "78f3585f8292bb91e4983bdbfb05f98ec8c1d29aab375970bfec67803f8da6bd"
PREV_REPORT_NAME = "verified-boundary-v0170.json"
PREV_FOCUS_TOOLS = "b826c77a2cd8ebd0a27a80d12df42e1ba32b6dedb0a02e938ce3adaa12869192"
PREV_FOCUS_LAUNCHER = "dcd18bb0f9a6456fffef989de863ea32a5b00e5c01447bcec8cde908f65015fc"
OLDER_FOCUS_TOOLS = "a55188fe39c863318f9e73fd36a639540d70b11f362d266e5ec1ab8b139e3824"
OLDER_SCRIPTS = ("observe-tabs.py", "prepare-focused.py", "pixel-oracle.py", "verify-label-color-original.py",
                 "verify-placeholder.py")
R1_SOURCE = "888bf446dba55df082f0f8c8c9181e5a76fbad76c63e3cc002c9f3d211d07ed8"
R2_SOURCE = "a52220af9e6ee0c7339dfce6641280dc77530f55a0dcdac39de17e37d16fb3ab"
R2_TREE = "a97db3adfe4f3ae0da6646d1ce3d3fe00da0e5f0"
R2_SELECTOR = "51c6a95503ad8d4d22cc465e177539d4644d9fb08e021f6c02c0eb03dc2af162"
R2_WEB = "07e73ce73dc3f5778e3958c21c611d636d5ce7c3b0852c4aacf8e6bc7a18a0ad"
R2_RUNNER = "06bdbfd19fde7d0c20f2c894b94bece803e5082ca416ba702779ae903cc87315"
R2_LAUNCHER = "2f94ab354a6c255590ec84db75f220b4efdc1f2b58d0e23638a9caf907e3e902"
R2_ORACLE = "442ede58283ac8f39cfc3a7ff355e94c53f0639a1130b34bd75a67d789f909eb"
R2_RECEIPT = "c6a3a984fcd408108ae61fcb889e7097f3c70fe370898c8176f6c016361ddada"
R2_REPORT = "b5fce965cf45b245892902ee2b20f77b1873773b47f40274c833915f4debf44e"
FOCUS_REPORT_NAME = "material-text-field-disabled-contrast.json"
NATIVE_TOOLS = "bc24e6dc9f2fed31637a63f31db859ddcce8017673308377b6df50607af93abf"
NATIVE_BASE = "34291d67bd5e47502553bcbf0171b424b4bee2f9be0fa7e5f2873b7c235d7575"
NATIVE_PAIR = "2932f41e240c5d8eb0898b1d40b2c41d6f1f278c301d80186df33c04ffc9be56"
NATIVE_DEV = "42c31232046dd0ed0df6d61caf9890df8d47b47e8c0afc85f8e35eb6676e5330"
NATIVE_DEV_RECEIPT = "867234ef8039b14433f4c4a84dd553f79f24025ddd9ed424c6f2a582ffa83e78"
NATIVE_EXACT = "b359a353265377d8a4603363da07c5c559260eb0e37ca0caba3e203be0677016"
NATIVE_EXACT_RECEIPT = "7325d1eb0e70d3e474aa4296217a7029c98a639890a242cd3d8eccc3bb15f8ee"
NATIVE_DEV_NAME = "development-r1-harness-r3"
NATIVE_EXACT_NAME = "exact-v0171-harness-r3"
NATIVE_REPORT_NAME = "native-text-field-disabled-contrast-report.json"
NATIVE_SCOPE = "Native headless Gallery fixed forced-colors preview, W3C CSS Color Adjustment Level 1 §5.2"
PALETTE = "https://www.w3.org/TR/css-color-adjust-1/#forced-color-palettes"
STOCK_TOOLS = "537239fd209bf5eee1a8d8703752af9e0f32d1cdd2ba088323ffcb71e5069bbb"
STOCK_PREFLIGHT = "998492596864aa1a9bddf8d1737a59ba483b1b93fbf1a7fe8febcfa73c30312d"
STOCK_REVIEW = "bdb1d779ca9f9d4decaebbed00ae7441678ac3fa3fc89d0adbd7f6600d8aab3c"
STOCK_RUNNER = "6828370429ed9bc24a2bcac64e1a77c125e5d053a729ce2625dfd66284e91787"
STOCK_RECEIPT = "verified-disabled-contrast-stock.json"
STOCK_OFFLINE = "stock-mac-exact-offline.json"
STOCK_MARKER = "stock-mac-v0171-passed"
REPORT = "verified-disabled-contrast-v0171.json"
WINDOW = "headless-window-observation.json"
RUN_MEMBERS = (WINDOW, "invocation.json", "inputs-after.json", "runner.log")
SUITES = ["smoke", "ime", "material", "bridge-web", "devtools-headless"]
UPDATER = "--disable-updater-scheduler"
DOCS = ["docs/material-coverage.json"]
REGISTRY = "e2e/src/material.rs"
TEST_DELTA = ["e2e/src/material_multiline_text_field.rs", "e2e/src/material_text_field_disabled_contrast.rs"]
STEPS = [f"M{i}" for i in (*range(241, 248), *range(357, 360))]
WORKSPACE = "workspace-v0171-exact.log"
WORKSPACE_SHA = "db8df375077fc9465dcb7cf965375a524fb70aee83c7340c1c69ba23bf6dcc85"
GPU_LOGS = ("gpu-v0171-exact.log", "gpu-v0171-supplemental.log")
MARKERS = {"workspace-v0171-passed": Q, "gpu-v0171-passed": Q,
           "native-final-built": EXACT, "stock-canonical-built": EXACT,
           STOCK_MARKER: STOCK}
RUN_PATH = runpy.run_path


def checked(path, expected):
    require(HEX.fullmatch(expected or "") and path.is_file() and not path.is_symlink()
            and digest(path) == expected, f"Wrong reviewed file {path.name}")
    return path


def delta(before, after):
    return sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))


def source_info(base, manifest, patch):
    before = path_entries(checked(base, PREV_SOURCE))
    after = path_entries(checked(manifest, SOURCE))
    paths = re.findall(r"^diff --git a/(\S+) b/\S+$", patch.decode(), re.M)
    require(sha256(patch).hexdigest() == PATCH and len(before) == 689 and len(after) == 690
            and len(paths) == 21 and sorted(paths) == delta(before, after), "Wrong exact source or patch paths")
    return {"tree": TREE, "manifest_sha256": SOURCE, "files": len(after), "patch_sha256": PATCH,
            "base_tree": PREV_TREE, "base_manifest_sha256": PREV_SOURCE, "changed_paths": sorted(paths)}


def live_source():
    source = source_info(PREV / "source.sha256", EXACT / "source.sha256", (Q / PATCH_NAME).read_bytes())
    root, entries = EXACT / "source", path_entries(EXACT / "source.sha256")
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file() and p.relative_to(root).parts[0] != ".git"}
    require(actual == entries.keys() and source_tree(root, entries) == TREE
            and (Q / "exact-apply-checked").read_text() == TREE + "\n", "Wrong current exact source")
    return source


def predecessor(root, live=False):
    manifest = root / ("SHA256SUMS" if live else "package-v0170.sha256")
    entries = (seal(root, "SHA256SUMS", PREV_SEAL, exact=True) if live
               else path_entries(checked(manifest, PREV_SEAL)))
    report_path = root / PREV_REPORT_NAME
    receipt_path = root / ("stock-macos/" + STOCK_OFFLINE if live else STOCK_OFFLINE)
    report = data(checked(report_path, PREV_REPORT))
    receipt = data(checked(receipt_path, PREV_OFFLINE))
    require(len(entries) == 613 and entries[PREV_REPORT_NAME] == PREV_REPORT
            and entries["stock-macos/" + STOCK_OFFLINE] == PREV_OFFLINE
            and report["source"]["tree"] == PREV_TREE and report["stock"]["total_checks"] == receipt["total_checks"] == 28706
            and receipt["cases"]["material"]["steps"] == 355 and receipt["guard_and_window_checks"] == "passed"
            and len(receipt["text_field_boundary_motion"]["pngs"]) == 33,
            "Wrong prior completed .70 stock evidence")
    return {"tree": PREV_TREE, "archive_sha256": PREV_SEAL, "stock_checks": 28706, "material_steps": 355}


def pending_markers(archived=None):
    absent = []
    for name, directory in MARKERS.items():
        candidate = (archived or directory) / name
        if not candidate.exists():
            absent.append(name)
        else:
            expected = b"" if name == STOCK_MARKER else (TREE + "\n").encode()
            require(candidate.is_file() and not candidate.is_symlink() and candidate.read_bytes() == expected,
                    f"Wrong exact marker {name}")
    return absent


def workspace_info(log):
    text, result = tests(log)
    require(result["log_sha256"] == WORKSPACE_SHA and (result["passed"], result["ignored"]) == (1587, 20)
            and text.rsplit("\ntest result:", 1)[-1].count("Finished `dev` profile") == 3
            and "provenance:" in text, "Wrong exact Rust/provenance/Wasm scope")
    return {**result, "wasm_checks": 3}


def rust_metal(rust, metal, pins, archived=False):
    extension = ".gz" if archived else ""
    workspace = workspace_info(rust / (WORKSPACE + extension))
    (base, included), (extra, added) = [tests(rust / (file + extension)) for file in GPU_LOGS]
    require((included["passed"], included["ignored"], added["passed"], added["ignored"]) == (16, 0, 2, 0)
            and [included["log_sha256"], added["log_sha256"]] == pins["gpu_logs_sha256"]
            and 'name: "Apple M1 Max"' in base and "backend: Metal" in base and extra.count("backend: Metal") == 2,
            "Wrong actual Metal suite, adapter or reviewed log")
    entries = path_entries(checked(metal / "SHA256SUMS", pins["gpu_manifest_sha256"]), allow_dot_prefix=True)
    require(set(entries) == {p.name for p in metal.glob("*.png")} and len(entries) == 21, "Wrong Metal PNG inventory")
    verify_entries(metal, entries)
    for file in entries:
        with Image.open(metal / file) as image:
            require(image.format == "PNG", "Wrong Metal original type")
            image.verify()
    return {"workspace": workspace, "metal": included, "supplemental": added,
            "metal_total": 18, "metal_pngs": entries}


@contextmanager
def native_oracle(tools, base):
    entries = seal(tools, "SHA256SUMS", NATIVE_TOOLS)
    seal(base, "SHA256SUMS", NATIVE_BASE)
    file = checked(tools / "replay-disabled-contrast.py", entries["replay-disabled-contrast.py"])
    source = file.read_text()
    literal = f'Path("{NATIVE_HELPERS}")'
    require(source.count(literal) == 1, "The native helper relocation seam changed")
    namespace = {"__file__": str(file), "__name__": "_portable_native_contrast"}
    original = sys.path[:]
    names = ("native_runtime", "gallery_transport", "render_helpers", "window_check")
    previous = {name: sys.modules.pop(name, None) for name in names}
    try:
        exec(compile(source.replace(literal, f"Path({str(base.resolve())!r})"), str(file), "exec"), namespace)
        yield namespace["verify_stored"]
    finally:
        sys.path[:] = original
        for name, before in previous.items():
            sys.modules.pop(name, None)
            if before is not None:
                sys.modules[name] = before


def native_info(oracle, run, typed, source, run_sha, typed_sha, binaries):
    require(digest(binaries) == NATIVE_PAIR, "Wrong pinned native pair")
    expected = path_entries(binaries)
    entries = seal(run, "SHA256SUMS", run_sha, exact=True)
    actual = normalized(oracle(run.resolve(strict=True), source, NATIVE_PAIR, run_sha))
    raw = data(run / NATIVE_REPORT_NAME)
    require(actual == data(checked(typed, typed_sha)) and raw["binaries"] == expected and raw["source_files"] == 690
            and (actual["repeats"], actual["original_pngs"], actual["observed_window_samples"],
                 actual["visible_owned_windows"], actual["reaped_gallery_processes"]) == (2, 24, 78, 0, 6)
            and actual["scope"] == NATIVE_SCOPE and actual["w3c_palette"] == PALETTE
            and actual["transports"] == ["Page.screenshot", "CLI", "MCP"],
            "Wrong fixed-palette native input, lifetime or original PNG scope")
    return {**actual, "host_setting_observed": False}, {k: v for k, v in entries.items() if k.endswith(".png")}


def focus_info(focus, exact_manifest, web_manifest, producer_manifest):
    seal(focus, "launcher.sha256", R2_LAUNCHER)
    original = path_entries(checked(focus / "original-source.sha256", R2_SOURCE))
    selected = path_entries(checked(focus / "focused-source.sha256", R2_SELECTOR))
    exact = path_entries(checked(exact_manifest, SOURCE))
    producer = path_entries(checked(producer_manifest, R1_SOURCE))
    require(len(original) == len(selected) == len(exact) == len(producer) == 690
            and delta(original, selected) == [REGISTRY] and delta(original, exact) == DOCS
            and delta(producer, original) == TEST_DELTA, "Wrong exact R1 Web build, R2 selector or final docs scope")
    run = focus / "development-r2"
    trusted = data(checked(focus / "development-r2-verified.json", R2_RECEIPT))
    oracle = RUN_PATH(str(checked(focus / "verify-field-contrast.py", R2_ORACLE)))
    checked(run / "artifacts" / FOCUS_REPORT_NAME, R2_REPORT)
    result = normalized(oracle["verify"](run))
    watch = data(run / WINDOW)
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary"})
    invocation = data(run / "invocation.json")
    observed = invocation["inputs"]
    argv = invocation["argv"]
    recorded = Path(argv[argv.index("--artifacts") + 1]).parent
    root = recorded.parent
    web = path_entries(checked(web_manifest, R2_WEB), allow_dot_prefix=True)
    report = data(run / "artifacts" / FOCUS_REPORT_NAME)
    require(result == trusted and result["checks"] == 1470 and result["steps"] == STEPS and result["chrome_samples"] == 162
            and result["visible_owned_windows"] == 0 and len(result["png_sha256"]) == 15
            and report["preference_source"] == "CDP browser media emulation"
            and result["adapter"]["vendor"] == "apple" and result["adapter"]["architecture"].startswith("metal")
            and result["adapter"]["isFallbackAdapter"] is False
            and invocation["suite_arguments"] == ["material"] and invocation["injected_chrome_flags"] == [UPDATER]
            and observed["runner_sha256"] == (focus / "runner.sha256").read_text().strip() == R2_RUNNER
            and observed["product_tree"] == R2_TREE and observed["source_count"] == 690
            and observed["product_source_manifest_sha256"] == R2_SOURCE and observed["focused_source_manifest_sha256"] == R2_SELECTOR
            and observed["registry_delta"] == [REGISTRY] and observed["steps"] == STEPS
            and observed["stock_registry_sha256"] == exact[REGISTRY] and observed["focused_registry_sha256"] == selected[REGISTRY]
            and observed["web_manifest_sha256"] == R2_WEB and observed["web_producer_manifest_sha256"] == R1_SOURCE
            and observed["web_compatible_test_only_delta"] == TEST_DELTA
            and observed["web_files"] == web and len(web) == 4 and recorded.parts[:3] == ("/", "private", "tmp")
            and argv == [str(root / "pins/surface-e2e"), "--chrome", str(recorded / "guards/chrome"),
                         "--chrome-arg", UPDATER, "--web-root", str(root / "web"), "--artifacts",
                         str(recorded / "artifacts"), "material"], "Wrong actual scoped .71 source, browser scope or receipts")
    return {**result, "checked_intake": observed, "exact_source_delta": DOCS,
            "web_source_delta": TEST_DELTA, "browser_preference": report["preference_source"],
            "window_receipt_sha256": digest(run / WINDOW)}


class ArchivedRun:
    """Use archived files with the original private headless-profile root."""

    def __init__(self, directory, recorded):
        self.directory, self.recorded = directory, recorded

    def __truediv__(self, part):
        return self.directory / part

    def resolve(self):
        return self.recorded


@contextmanager
def stock_observer(root, prior_tools, current_boundary, old_placeholder):
    old = RUN_PATH(str(checked(prior_tools / "pack-boundary.py",
                              "4f82c9615db2693b80a17b713cdad9a96bd9e96c15a2761038ba465d416b48ad")))
    seal(current_boundary, "verifier.sha256", PREV_FOCUS_TOOLS)
    seal(current_boundary, "launcher.sha256", PREV_FOCUS_LAUNCHER)
    old_entries = path_entries(checked(old_placeholder / "SHA256SUMS", OLDER_FOCUS_TOOLS))
    verify_entries(old_placeholder, {name: old_entries[name] for name in OLDER_SCRIPTS})
    with import_from(root), old["relocated_verifiers"](current_boundary, old_placeholder) as load:
        observer = load(str(root / "observe-multiline.py"))
        boundary = sys.modules["verify_boundary_stock"]
        saved = boundary.DEFAULT_FOCUS
        boundary.DEFAULT_FOCUS = current_boundary
        try:
            yield observer
        finally:
            boundary.DEFAULT_FOCUS = saved


def stock_info(root, focused, prior_tools, boundary, placeholder, pins):
    seal(root, "harness.sha256", STOCK_TOOLS)
    checked(root / "offline-preflight.log", STOCK_PREFLIGHT)
    checked(root / "browser-source.sha256", SOURCE)
    checked(root / "native-binaries.sha256", NATIVE_PAIR)
    reviewed = data(checked(root / "reviewed-pins.json", pins["stock_review_sha256"]))
    offline = data(checked(root / STOCK_OFFLINE, pins["stock_offline_sha256"]))
    run = root / "run"
    typed = data(checked(run / STOCK_RECEIPT, pins["stock_receipt_sha256"]))
    watch = data(checked(run / WINDOW, pins["stock_window_sha256"]))
    checked(run / "runner.log", pins["stock_log_sha256"])
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary", "chrome-bridge", "native"})
    inputs = data(root / "inputs-before.json")
    require(inputs == data(run / "inputs-after.json"), "The guarded stock input changed")
    seal(root, "sealed-inputs.sha256", inputs["app_bundle_seal_sha256"])
    invocation = data(run / "invocation.json")
    binary = path_entries(root / "browser-binary-sha256.txt")
    origins = data(root / "bundle-origins.json")
    bound = data(root / "focused-proof.json")
    gallery = {PurePosixPath(name).name: checksum for name, checksum in binary.items()
               if PurePosixPath(name).parent == PurePosixPath("web/release/material-gallery")}
    require(inputs == invocation["inputs"] and inputs["source_manifest_sha256"] == SOURCE
            and inputs["reviewed_pins_sha256"] == pins["stock_review_sha256"] and inputs["source_files_checked_per_origin"] == 690
            and inputs["stock_files"] == binary and inputs["bundle_files_checked"] == len(binary) == 16
            and origins.keys() == binary.keys() and inputs["runner_sha256"] == binary["debug/surface-e2e"] == pins["stock_runner_sha256"]
            and (root / "approved-runner.sha256").read_text().strip() == pins["stock_runner_sha256"]
            and reviewed["source_sha256"] == SOURCE and reviewed["source_tree"] == TREE
            and reviewed["patch_sha256"] == PATCH and reviewed["native_manifest_sha256"] == NATIVE_PAIR
            and inputs["focused_proof_sha256"] == digest(root / "focused-proof.json")
            and offline["focused"] == bound and bound["focused_receipt_sha256"] == R2_RECEIPT
            and bound["canonical_gallery_match"] is True and len(gallery) == 4
            and gallery == focused["checked_intake"]["web_files"] == bound["canonical_gallery_files"]
            and bound["inputs"] == focused["checked_intake"] and bound["step_ids"] == STEPS
            and bound["checks"] == 1470 and bound["headless_samples"] == 162
            and all(bound[key] == focused[key] for key in ("ink_pixels", "png_sha256", "input_receipt_sha256", "report_sha256"))
            and bound["exact_vs_scoped_source_paths"] == DOCS
            and invocation["suite_arguments"] == SUITES and invocation["injected_chrome_flags"] == [UPDATER],
            "Wrong stock binary, source, canonical Gallery or invocation scope")
    argv = invocation["argv"]
    recorded = Path(argv[argv.index("--artifacts") + 1]).parent
    require(recorded.parts[:3] == ("/", "private", "tmp") and argv[0] == origins["debug/surface-e2e"],
            "Wrong actual private run or runner origin")
    with stock_observer(root, prior_tools, boundary, placeholder) as observer:
        require(inputs["release"] == offline["release"] == observer["read_release_pin"](root)
                and argv == observer["stock_command"](recorded, Path(argv[0]), recorded.parent / "bundle",
                                                       {role: str(recorded / "guards" / role) for role in observer["GUARD_ROLES"]}),
                "Wrong exact stock execution")
        replay = normalized(observer["verify_run"](ArchivedRun(run, recorded), watch, inputs["release"]))
    criteria = typed["text_field_disabled_contrast"]
    require(replay == typed and all(offline[key] == value for key, value in typed.items())
            and offline["window_receipt_sha256"] == pins["stock_window_sha256"]
            and offline["runner_log_sha256"] == pins["stock_log_sha256"]
            and set(typed["cases"]) == set(SUITES) and typed["cases"]["material"]["steps"] == 358
            and typed["cases"]["ime"]["steps"] == 20 and typed["page_errors"] == 0
            and typed["guard_and_window_checks"] == "passed" and typed["total_checks"] > 28706
            and len(criteria["png_sha256"]) == 15 and criteria["adapter"] == typed["adapter"]
            and len(typed["text_field_boundary_motion"]["pngs"]) == 33
            and typed["text_field_boundary_motion"]["decoded_probes"] == 2478
            and len(typed["text_field_label_motion"]["pngs"]) == 34 and len(typed["text_field_placeholder"]["pngs"]) == 58
            and {"ime_cancellation", "navigation", "menu", "divider", "radio", "checkbox", "switch", "tabs"}
            <= set(offline["retained"]["retained"]), "Wrong completed stock scope or redecoded evidence")
    return {"total_checks": typed["total_checks"], "cases": typed["cases"], "visible_windows": 0,
            "remaining_processes": [], "adapter": typed["adapter"], "browser": typed["browser"],
            "dark_and_light_contrast_pngs": criteria["png_sha256"], "ink_pixels": criteria["ink_pixels"],
            "retained_boundary_pngs": 33, "retained_label_pngs": 34, "retained_placeholder_pngs": 58}


def read_gate_pins():
    absent = pending_markers()
    require(not absent, "Pending exact gates: " + ", ".join(absent))
    reviewed = data(checked(STOCK / "reviewed-pins.json", STOCK_REVIEW))
    runner = (STOCK / "approved-runner.sha256").read_text().strip()
    require(reviewed["stock_runner_sha256"] == runner == STOCK_RUNNER
            and reviewed["source_sha256"] == SOURCE and reviewed["source_tree"] == TREE
            and reviewed["patch_sha256"] == PATCH and reviewed["native_manifest_sha256"] == NATIVE_PAIR,
            "Wrong exact stock review")
    return {"source_sha256": SOURCE, "tree": TREE, "stock_runner_sha256": runner,
            "stock_review_sha256": digest(STOCK / "reviewed-pins.json"),
            "stock_receipt_sha256": digest(STOCK / "run" / STOCK_RECEIPT),
            "stock_window_sha256": digest(STOCK / "run" / WINDOW),
            "stock_log_sha256": digest(STOCK / "run/runner.log"),
            "stock_offline_sha256": digest(STOCK / STOCK_OFFLINE),
            "gpu_logs_sha256": [digest(Q / name) for name in GPU_LOGS],
            "gpu_manifest_sha256": digest(Q / "gpu-v0171-exact/SHA256SUMS"),
            "native_replay_sha256": digest(checked(NATIVE / NATIVE_EXACT_NAME / "SHA256SUMS", NATIVE_EXACT)),
            "native_receipt_sha256": digest(checked(NATIVE / (NATIVE_EXACT_NAME + "-verified.json"), NATIVE_EXACT_RECEIPT)),
            "canonical_build_log_sha256": digest(Q / "stock-build-v0171-exact.log")}


def reviewed_pins(path, expected):
    require(path is not None and HEX.fullmatch(expected or ""), "Supply the reviewed terminal gate pin file and hash")
    pins = data(checked(path, expected))
    require(isinstance(pins, dict) and pins.keys() == {
        "source_sha256", "tree", "stock_runner_sha256", "stock_review_sha256", "stock_receipt_sha256",
        "stock_window_sha256", "stock_log_sha256", "stock_offline_sha256", "gpu_logs_sha256", "gpu_manifest_sha256",
        "native_replay_sha256", "native_receipt_sha256", "canonical_build_log_sha256",
    } and pins["source_sha256"] == SOURCE and pins["tree"] == TREE
        and pins["stock_review_sha256"] == STOCK_REVIEW and pins["stock_runner_sha256"] == STOCK_RUNNER
        and pins["native_replay_sha256"] == NATIVE_EXACT and pins["native_receipt_sha256"] == NATIVE_EXACT_RECEIPT
        and all(isinstance(value, str) and HEX.fullmatch(value) for key, value in pins.items() if key not in {"tree", "gpu_logs_sha256"})
        and isinstance(pins["gpu_logs_sha256"], list) and len(pins["gpu_logs_sha256"]) == 2
        and all(isinstance(value, str) and HEX.fullmatch(value) for value in pins["gpu_logs_sha256"]),
            "Wrong reviewed terminal gate pins")
    return pins


def png_inventory(receipt):
    names, hashes = set(), set()
    queue = [receipt]
    while queue:
        value = queue.pop()
        if isinstance(value, dict):
            queue.extend((*value.keys(), *value.values()))
        elif isinstance(value, list):
            queue.extend(value)
        elif isinstance(value, str):
            if re.fullmatch(r"[A-Za-z0-9_.+-]+\.png", value):
                names.add(value)
            elif HEX.fullmatch(value):
                hashes.add(value)
    return names, hashes


def copy_run(source, target, extra=()):
    ledgers = (path.relative_to(source).as_posix() for path in (source / "guard-ledger").glob("*.json"))
    copy_members(source, target, (*RUN_MEMBERS, *extra, *ledgers))


def native_complete(tools, base, development, dev_receipt, exact, receipt, binaries, pins):
    with native_oracle(tools, base) as oracle:
        former, former_png = native_info(oracle, development, dev_receipt, R1_SOURCE, NATIVE_DEV, NATIVE_DEV_RECEIPT, binaries)
        current, current_png = native_info(oracle, exact, receipt, SOURCE, pins["native_replay_sha256"],
                                           pins["native_receipt_sha256"], binaries)
    require(former_png == current_png, "Exact native original PNGs differ from development")
    return {"exact": current, "development_r3": former, "matching_development_pngs": len(current_png)}


def baseline():
    source = live_source()
    prior = predecessor(PREV, live=True)
    focused = focus_info(FOCUS, EXACT / "source.sha256", Q / "web-development.sha256",
                         Q / "development-snapshot-r1/source.sha256")
    with native_oracle(NATIVE, NATIVE_HELPERS) as oracle:
        native, _ = native_info(oracle, NATIVE / NATIVE_DEV_NAME, NATIVE / (NATIVE_DEV_NAME + "-verified.json"),
                                R1_SOURCE, NATIVE_DEV, NATIVE_DEV_RECEIPT, Q / "native-development/SHA256SUMS")
    return {"source": source, "predecessor": prior, "focused": focused, "native_development_r3": native,
            "workspace": workspace_info(Q / WORKSPACE)}


def stock_previous_tools():
    return PREV, PREV / "focused", PREV / "previous/focused-tools"


def live_proof(pins, pin_sha):
    stage = baseline()
    checked(Q / "stock-build-v0171-exact.log", pins["canonical_build_log_sha256"])
    # The original independent receipt rechecks the 16 live build inputs before they are reduced to hashes in the archive.
    original = run_json([STOCK / "verify-stock-receipt.py", STOCK / "run"])
    require(original == data(STOCK / STOCK_OFFLINE), "The live exact stock readback differs from its original receipt")
    native = native_complete(NATIVE, NATIVE_HELPERS, NATIVE / NATIVE_DEV_NAME, NATIVE / (NATIVE_DEV_NAME + "-verified.json"),
                             NATIVE / NATIVE_EXACT_NAME, NATIVE / (NATIVE_EXACT_NAME + "-verified.json"),
                             EXACT / "binaries/SHA256SUMS", pins)
    require(native["development_r3"] == stage["native_development_r3"], "Native R3 inputs changed")
    return {"status": "verified exact local candidate", "gate_pins_sha256": pin_sha,
            "source": stage["source"], "predecessor": stage["predecessor"], "focused": stage["focused"], "native": native,
            "stock": stock_info(STOCK, stage["focused"], *stock_previous_tools(), pins),
            "rust_metal": rust_metal(Q, Q / "gpu-v0171-exact", pins)}


def copy_sealed(root, destination, manifest):
    copy_members(root, destination, (*path_entries(root / manifest), manifest))


def archive(destination, pin_file, proof):
    require(destination.is_relative_to(HERE) and destination != HERE and not destination.exists(),
            "Use a fresh output under the package lane")
    destination.mkdir(mode=0o700)
    copy_members(HERE, destination, (Path(__file__).name, "evidence_common.py", "audit-publication.py"))
    copy(EXACT / "source.sha256", destination / "source.sha256")
    copy(PREV / "source.sha256", destination / "base-source.sha256")
    copy(pin_file, destination / "gate-pins.json")
    compressed(Q / PATCH_NAME, destination / (PATCH_NAME + ".gz"))
    for name, stage in MARKERS.items():
        copy(stage / name, destination / "gates" / name)
    for name in (WORKSPACE, *GPU_LOGS, "stock-build-v0171-exact.log"):
        compressed(Q / name, destination / "rust" / (name + ".gz"))
    metal = Q / "gpu-v0171-exact"
    copy_members(metal, destination / "rust/metal", (*proof["rust_metal"]["metal_pngs"], "SHA256SUMS"))

    prior = destination / "previous"
    copy(PREV / "SHA256SUMS", prior / "package-v0170.sha256")
    copy(PREV / PREV_REPORT_NAME, prior / PREV_REPORT_NAME)
    copy(PREV / "stock-macos" / STOCK_OFFLINE, prior / STOCK_OFFLINE)
    copy(PREV / "pack-boundary.py", prior / "pack-boundary.py")
    copy_sealed(PREV / "focused", prior / "boundary", "verifier.sha256")
    copy_sealed(PREV / "focused", prior / "boundary", "launcher.sha256")
    copy_members(PREV / "previous/focused-tools", prior / "placeholder", (*OLDER_SCRIPTS, "SHA256SUMS"))

    focused = destination / "focused"
    copy_sealed(FOCUS, focused, "launcher.sha256")
    copy_members(FOCUS, focused, ("runner.sha256", "development-r2-verified.json"))
    copy(Q / "web-development.sha256", focused / "web.sha256")
    copy(Q / "development-snapshot-r1/source.sha256", focused / "web-producer-source.sha256")
    run = focused / "development-r2"
    original = FOCUS / "development-r2"
    copy_run(original, run, ("owned-shutdown.json",))
    copy_members(original / "artifacts", run / "artifacts", (FOCUS_REPORT_NAME, *proof["focused"]["png_sha256"]))

    native = destination / "native"
    copy_sealed(NATIVE, native / "tools", "SHA256SUMS")
    copy_sealed(NATIVE_HELPERS, native / "helpers", "SHA256SUMS")
    copy(EXACT / "binaries/SHA256SUMS", native / "binaries.sha256")
    for name in (NATIVE_EXACT_NAME, NATIVE_DEV_NAME):
        copy_sealed(NATIVE / name, native / name, "SHA256SUMS")
        copy_members(NATIVE, native, (name + "-verified.json", name + ".log"))

    stock = destination / "stock-macos"
    tools = set(path_entries(STOCK / "harness.sha256")) | set(path_entries(STOCK / "sealed-inputs.sha256"))
    copy_members(STOCK, stock, (*tools, "harness.sha256", "sealed-inputs.sha256", "offline-preflight.log",
                                "inputs-before.json", "approved-runner.sha256", STOCK_OFFLINE))
    stock_run = STOCK / "run"
    copy_run(stock_run, stock / "run", (STOCK_RECEIPT,))
    names, hashes = png_inventory(data(stock / STOCK_OFFLINE))
    for path in (stock_run / "artifacts").rglob("*"):
        if path.suffix in {".png", ".json"} and path.is_file():
            require(not path.is_symlink(), "Unexpected linked stock output")
            if path.suffix == ".json" or path.name in names or digest(path) in hashes:
                copy(path, stock / "run" / path.relative_to(stock_run))

    json_file(destination / REPORT, proof)
    (destination / "README.md").write_text(f"""# Surface v0.1.71 disabled text-field contrast evidence

The exact 690-file tree `{TREE}` applies the 21-path patch to the
v0.1.70 tree `{PREV_TREE}`. The scoped browser runner selects ten Material
steps. Its product snapshot and the exact candidate differ only in the
coverage document. Its four Web assets match the exact stock Gallery.

- Exact Rust passed 1,587 tests, provenance, lint and three Wasm checks.
  Actual Apple Metal passed 18 tests and produced 21 original PNGs.
- The exact native replay produced 24 original PNGs across two repeats.
  Direct Page.screenshot, CLI and MCP returned identical bytes.
  The 78 observations showed no visible windows. All six Gallery
  processes exited. The development R3 replay is stored separately.
  Its 24 images match the exact images byte-for-byte.
- Scoped Chrome passed 1,470 checks in ten steps. It records 15 contrast
  images and 162 headless observations with zero windows or survivors.
- Full stock passed {proof["stock"]["total_checks"]:,} checks in five
  suites. It includes 358 ordered Material steps through M359, 20 IME
  steps, 15 contrast PNGs, retained decoders and zero windows or survivors.

The native Gallery used the fixed light and dark palettes in
[W3C CSS Color Adjustment Level 1 §5.2]({PALETTE}). It verifies native
paint and the Runtime state in that preview. The browser verifies CSS
GrayText via headless Chrome media emulation and actual Canvas pixels.
Neither result measures macOS setting changes or live assistive speech.

Use Python 3.12 with Pillow and the reviewed archive manifest digest.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-disabled-contrast.py --verify-package . --package-sha "$reviewed_sha"
```

This reads the sealed files locally. App and browser binaries stay outside
this packet. Raw receipts keep temporary process paths and original pixels.
""")
    manifest = destination / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(destination).as_posix()}\n"
                                for path in sorted(destination.rglob("*")) if path.is_file()))
    return digest(manifest)


def verify_package(root, expected):
    seal(root, "SHA256SUMS", expected, exact=True)
    saved = data(root / REPORT)
    pins = reviewed_pins(root / "gate-pins.json", saved["gate_pins_sha256"])
    require(not pending_markers(root / "gates"), "Missing archived exact gate")
    source = source_info(root / "base-source.sha256", root / "source.sha256", raw_log(root / (PATCH_NAME + ".gz")))
    prior = root / "previous"
    focused = focus_info(root / "focused", root / "source.sha256", root / "focused/web.sha256",
                         root / "focused/web-producer-source.sha256")
    native = root / "native"
    native_report = native_complete(native / "tools", native / "helpers", native / NATIVE_DEV_NAME,
                                    native / (NATIVE_DEV_NAME + "-verified.json"), native / NATIVE_EXACT_NAME,
                                    native / (NATIVE_EXACT_NAME + "-verified.json"), native / "binaries.sha256", pins)
    build = sha256(raw_log(root / "rust/stock-build-v0171-exact.log.gz")).hexdigest()
    require(build == pins["canonical_build_log_sha256"], "Wrong original canonical build log")
    proof = {"status": "verified exact local candidate", "gate_pins_sha256": saved["gate_pins_sha256"],
             "source": source, "predecessor": predecessor(prior), "focused": focused, "native": native_report,
             "stock": stock_info(root / "stock-macos", focused, prior, prior / "boundary", prior / "placeholder", pins),
             "rust_metal": rust_metal(root / "rust", root / "rust/metal", pins, archived=True)}
    require(saved == proof, "Archive readback differs from the actual verified evidence")
    return proof


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--preflight", action="store_true")
    mode.add_argument("--read-gate-pins", action="store_true")
    mode.add_argument("--out", type=Path)
    mode.add_argument("--verify-package", type=Path)
    parser.add_argument("--gate-pins", type=Path)
    parser.add_argument("--gate-pins-sha")
    parser.add_argument("--package-sha")
    args = parser.parse_args()
    if args.preflight:
        result = {"status": "offline preflight only", **baseline(), "pending_exact_markers": pending_markers()}
    elif args.read_gate_pins:
        result = read_gate_pins()
    elif args.verify_package:
        result = verify_package(args.verify_package.resolve(strict=True), args.package_sha)
    else:
        pins = reviewed_pins(args.gate_pins, args.gate_pins_sha)
        require(pins == read_gate_pins(), "The actual terminal evidence changed from the reviewed digests")
        proof = live_proof(pins, args.gate_pins_sha)
        target = args.out.resolve()
        checksum = archive(target, args.gate_pins, proof)
        require(run_json([target / Path(__file__).name, "--verify-package", target, "--package-sha", checksum]) == proof,
                "Independent portable readback differs")
        audit = run_json([target / "audit-publication.py", target, "--seal", checksum])
        blocked = {"credential_token", "private_key", "bearer_value", "jwt", "credential_like_json_field",
                   "url_auth", "private_network_urls", "runtime_loopback_urls", "mail_address", "os_session_directory",
                   "other_url_origin", "unparsed_url", "binary_signature", "unrecognized_nontext",
                   "png_text_exif_or_icc_metadata", "large_file"}
        require(not blocked & audit["observations"].keys(), "Run the path-only publication audit before posting this packet")
        result = {"status": proof["status"], "archive": str(target), "archive_sha256": checksum,
                  "report_sha256": digest(target / REPORT), "stock_checks": proof["stock"]["total_checks"],
                  "publication_audit": audit}
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit(f"Exact disabled contrast evidence stopped. {error}")
