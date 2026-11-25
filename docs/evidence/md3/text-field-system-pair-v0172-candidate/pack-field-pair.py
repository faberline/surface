"""Assemble and verify exact v0.1.72 evidence from original local artifacts."""

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
Q = Path("/tmp/surface-md3-field-pair-v0172.SEhFjvMD")
EXACT = Q / "exact-candidate"
STOCK = Path("/tmp/surface-md3-field-pair-stock-v0172-final.qeJADEOL")
FOCUS = Path("/tmp/surface-md3-field-pair-focused-v0172-r8.bAGIL0yE")
NATIVE = Path("/tmp/surface-md3-field-pair-native-v0172.0lhfzlBY")
NATIVE_HELPERS = Path("/tmp/surface-md3-field-boundary-native.705YkFQO")
LEGACY = Path("/tmp/surface-md3-sliders-stock.lK6a9gKl")
PREV = Path("/tmp/surface-md3-disabled-contrast-v0171-package.gBiM0yjr/portable")
SOURCE = "b3137077ef9753a91a4e86b3c7d9dedc14b479f4aa4e74c95b299a6805543cd9"
TREE = "9333c1f8dba9a7d15aba2ca47dc92c997abf98af"
PATCH = "82839f72bf454a8e4bdd22660ecb223684ea7bcd6351158137a9044d1efc3117"
PATCH_NAME = "text-field-system-pair-v0172.patch"
PREV_SOURCE = "65c1bf0fd49f018b575cf069af016aa46229cff9dc23d521942f0ba0b6465ee2"
PREV_TREE = "f2b631b0d72830f76b84e71b114018a4410dc813"
PREV_SEAL = "41d846a3e1f56326e63ddadb607cba6d88a2a3d9255e1698b7fa74a681e92021"
PREV_REPORT = "24ba654f1f8689c5d831864b521c9d77d102de3aa8fabceedcfc21208c14ab0f"
PREV_OFFLINE = "bf18f19d74e7c212747731d29666213feb4b3bf72e8249b3daab1a698da3db1d"
PREV_REPORT_NAME = "verified-disabled-contrast-v0171.json"
PREV_FOCUS_TOOLS = "b826c77a2cd8ebd0a27a80d12df42e1ba32b6dedb0a02e938ce3adaa12869192"
PREV_FOCUS_LAUNCHER = "dcd18bb0f9a6456fffef989de863ea32a5b00e5c01447bcec8cde908f65015fc"
OLDER_FOCUS_TOOLS = "a55188fe39c863318f9e73fd36a639540d70b11f362d266e5ec1ab8b139e3824"
OLDER_SCRIPTS = ("observe-tabs.py", "prepare-focused.py", "pixel-oracle.py", "verify-label-color-original.py",
                 "verify-placeholder.py")
STOCK_TOOLS = "0a2c01309be1d4d5c0cde1153d8fc713f9ed2f0cb26dbd2f657aaf08b144be3c"
STOCK_PREFLIGHT = "feac4620d1e5d51be00085324b62056e56cad5448e22a1bc52b1c46d8c729bd9"
STOCK_REVIEW = "8f9402a5c2ac9f6a55a6e53f29948e3b285aa0d1e946324609345a93f5f32616"
STOCK_RUNNER = "8b0a47d9a826f9fa7ec2f82b1fd3d71a59e1bf2dfb18a04b9fb2db42ecb5fb23"
STOCK_RECEIPT = "verified-system-pair-stock.json"
STOCK_OFFLINE = "stock-mac-exact-offline.json"
STOCK_MARKER = "stock-mac-v0172-passed"
REPORT = "verified-system-pair-v0172.json"
WINDOW = "headless-window-observation.json"
RUN_MEMBERS = (WINDOW, "invocation.json", "inputs-after.json", "runner.log")
SUITES = ["smoke", "ime", "material", "bridge-web", "devtools-headless"]
UPDATER = "--disable-updater-scheduler"
WORKSPACE = "workspace-v0172-exact.log"
WORKSPACE_SHA = "e98275e3e6cf2d2940aa25f326ba0770e08a51f85a4e13a595120d5c4b7dd5e9"
GPU_LOGS = ("gpu-v0172-exact.log", "gpu-v0172-supplemental.log")
BUILD_LOG = "stock-build-v0172-exact.log"
MARKERS = {"workspace-v0172-passed": Q, "gpu-v0172-passed": Q,
           "native-final-built": EXACT, "stock-canonical-built": EXACT, STOCK_MARKER: STOCK}
GPU_PINS = {
    "gpu_logs_sha256": ["3f657a01be301cfedf695ba269efa1e5d4b355b54d5204bdf5c5f2236f4166ca",
                        "2ef1a1c9867437ff28fc5c16a8c54678f600077fa5a1a0a0e87d33d2192880c5"],
    "gpu_manifest_sha256": "c7fb34d394d7702a9e9f6e8c617d552486dfb52bf766fce7bc4c8259440c409f",
}
BUILD_SHA = "e20ed9cf79b5b3c3c46541cb4fb94110c2e7fee197e845a7953e16fb1697b700"
NATIVE_TOOLS = "b21a5c6f7836330facd279593cf155fc383fd529ee1ca5d16d040211268855e8"
NATIVE_BASE = "34291d67bd5e47502553bcbf0171b424b4bee2f9be0fa7e5f2873b7c235d7575"
NATIVE_PAIR = "68d60a3f8fa383646c4889f683b1e6fe1567e7c063dfd2e51edc9c4357a3c4f8"
NATIVE_RUNS = {
    "development-r4": ("5c342f40941e6f5350411c7787faed8651356f8f739bd64f059bdb65622e8649",
                       "579f13921a054236bdae90694b2ba1664e6a544a6cced85b4399811fc1abe838",
                       "abaf339024f01ae0fb98f28ea8794eeada76dd30724cfeea7ed56d143f73eca8"),
    "exact-v0172": (SOURCE, "f0e354d178164ead2a63569b89bea8a849fb3b65d707701a2d195ae4324e7fcb",
                    "dac4a9485d7826ca6f10d4afad9d13045e11c6fa155ea34d8b29933b8832a942"),
}
LEGACY_HASHES = {
    "verify-existing.py": "eded5dfdfcf94dc901cb3608b83cc86a557fe4cb02b91122416c6a03e3568558",
    "observe-sliders.py": "1c39e718513bd2224f3c37c58515e80f9fb0b4dd14a3b00ce3256078ca5639ce",
    "retained/SHA256SUMS": "5a9c99e2bf64a76375d81d5f16ab58228410be56ba32225b01930df949a0ac6f",
}
FOCUS_TOOLS = "5d48d1ee9cbb7bc2ba44a362497a7b38c446b3227b7b6f16d0f3a4c0e51fcd25"
FOCUS_ORACLE = "d8e101cdd4fe3909c990674a3153db9553aa75b1303f47e112876646065947b4"
FOCUS_RECEIPT = "94869c456c7ba738962173021e62d08412c0ca51e5207c69641b080fab3d28b8"
FOCUS_RUNNER = "adfa819a63cc2e8a5f6d355d52c3c0c10687c5b593df69c0de7c37dd71faccda"
FOCUS_SOURCE = "4a4ad2522f1c080fd870932077619d3c85e2171598e620e93dfbc2065944ea99"
FOCUS_SELECTOR = "9b51973bea8880660c899e9e5e60b9759a5692bd4bb1b97c73b22c9e8783081b"
FOCUS_WEB = "84359bf859a0582b2da51875f0e6bd279641a0afe28935b0b38ecb2c0cb1f3b1"
WEB_SOURCE = NATIVE_RUNS["development-r4"][0]
WEB_TESTS = ["e2e/src/material_text_field_system_pair.rs", "e2e/src/material_web_contrast.rs",
             "e2e/src/material_web_contrast_selection.rs"]
STEPS = [f"M{i}" for i in (*range(241, 248), *range(357, 363))]
REGISTRY = "e2e/src/material.rs"
DOCS = ["docs/material-coverage.json"]
PREFIX = "browser-material-text-field-system-pair"
R4 = Path("/tmp/surface-md3-field-pair-focused-v0172-r5.bce6QeJB/r4-rejected")
R5 = Path("/tmp/surface-md3-field-pair-focused-v0172-r6.mIkOozwh/r5-rejected")
R7 = Path("/tmp/surface-md3-field-pair-focused-v0172-r7.B20dFVGG")
R4_SEAL = "88a59373cf2389a587d5c828196fef2d3856c56b7535749b732d09cb5bf6927e"
R5_SEAL = "d027e29322d0c60f852a6c7706e256794cd4bc27e88367804d889cd45df500e0"
R6_COMPILER = "890ae524717f6593ba9ab442b28d97f9f8c7a21b76465e3f409725020185d07d"
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
    require(sha256(patch).hexdigest() == PATCH and len(before) == 690 and len(after) == 691
            and paths and len(paths) == len(set(paths)) and sorted(paths) == delta(before, after), "Wrong exact source or patch paths")
    return {"tree": TREE, "manifest_sha256": SOURCE, "files": len(after), "patch_sha256": PATCH,
            "base_tree": PREV_TREE, "base_manifest_sha256": PREV_SOURCE, "changed_paths": sorted(paths)}


def live_source():
    source = source_info(PREV / "source.sha256", EXACT / "source.sha256", (Q / PATCH_NAME).read_bytes())
    root, entries = EXACT / "source", path_entries(EXACT / "source.sha256")
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file() and p.relative_to(root).parts[0] != ".git"}
    require(actual == entries.keys() and source_tree(root, entries) == TREE
            and (Q / "exact-apply-checked").read_text() == TREE + "\n", "Wrong current exact source")
    return source


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
    require(result["log_sha256"] == WORKSPACE_SHA and (result["passed"], result["ignored"]) == (1593, 20)
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
    seal(current_boundary, "verifier.sha256", PREV_FOCUS_TOOLS)
    entries = seal(current_boundary, "launcher.sha256", PREV_FOCUS_LAUNCHER)
    old_entries = path_entries(checked(old_placeholder / "SHA256SUMS", OLDER_FOCUS_TOOLS))
    verify_entries(old_placeholder, {name: old_entries[name] for name in OLDER_SCRIPTS})
    parser = checked(current_boundary / "prepare-focused.py", entries["prepare-focused.py"]).resolve()
    parser_source = parser.read_text()
    literal = 'Path("/tmp/surface-md3-field-placeholder-focused-v0169-r2.8AArzw7l")'
    require(parser_source.count(literal) == 1, "The sealed retained parser's relocation seam changed")

    def relocate_parser(path, *args, **kwargs):
        if Path(path).resolve() != parser:
            return RUN_PATH(str(path), *args, **kwargs)
        namespace = {"__file__": str(parser), "__name__": "_portable_boundary_parser"}
        relocated = parser_source.replace(literal, f"Path({str(old_placeholder.resolve())!r})")
        exec(compile(relocated, str(parser), "exec"), namespace)
        return namespace

    # The inherited relocator captures run_path when loaded. Give it this
    # third sealed parser relocation while leaving all original files intact.
    runpy.run_path = relocate_parser
    try:
        old = RUN_PATH(str(checked(prior_tools / "pack-boundary.py",
                                  "4f82c9615db2693b80a17b713cdad9a96bd9e96c15a2761038ba465d416b48ad")))
        with import_from(root), old["relocated_verifiers"](current_boundary, old_placeholder) as load:
            observer = load(str(root / "observe-multiline.py"))
            boundary = sys.modules["verify_boundary_stock"]
            saved = boundary.DEFAULT_FOCUS
            boundary.DEFAULT_FOCUS = current_boundary
            try:
                yield observer
            finally:
                boundary.DEFAULT_FOCUS = saved
    finally:
        runpy.run_path = RUN_PATH


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


def copy_sealed(root, destination, manifest):
    copy_members(root, destination, (*path_entries(root / manifest), manifest))


def predecessor(root, live=False):
    if live:
        inventory = seal(root, "SHA256SUMS", PREV_SEAL, exact=True)
        result = run_json([root / "pack-disabled-contrast.py", "--verify-package", root,
                           "--package-sha", PREV_SEAL])
    else:
        inventory = path_entries(checked(root / "package-v0171.sha256", PREV_SEAL))
        result = data(checked(root / PREV_REPORT_NAME, PREV_REPORT))
    offline = root / ("stock-macos/" + STOCK_OFFLINE if live else STOCK_OFFLINE)
    receipt = data(checked(offline, PREV_OFFLINE))
    require(len(inventory) == 566 and inventory[PREV_REPORT_NAME] == PREV_REPORT
            and inventory["stock-macos/" + STOCK_OFFLINE] == PREV_OFFLINE
            and result["source"]["tree"] == PREV_TREE
            and result["stock"]["total_checks"] == receipt["total_checks"] == 29384
            and receipt["cases"]["material"]["steps"] == 358
            and receipt["guard_and_window_checks"] == "passed"
            and len(receipt["text_field_disabled_contrast"]["png_sha256"]) == 15,
            "Wrong sealed predecessor stock evidence")
    if live:
        stock = root / "stock-macos"
        original_path = sys.path[:]
        try:
            prior = normalized(legacy_verifier(LEGACY)["retained"](stock / "run",
                               stock / "browser-source.sha256", PREV_SOURCE, 359))
        finally:
            sys.path[:] = original_path
        require(prior == receipt["retained"], "Raw predecessor decoders differ from the published .71 receipt")
    return {"tree": PREV_TREE, "archive_sha256": PREV_SEAL, "stock_checks": 29384, "material_steps": 358}


@contextmanager
def native_oracle(tool, helpers):
    seal(tool, "launcher.sha256", NATIVE_TOOLS)
    seal(helpers, "SHA256SUMS", NATIVE_BASE)
    file = tool / "replay-field-pair.py"
    original = file.read_text()
    literal = f'Path("{NATIVE_HELPERS}")'
    require(original.count(literal) == 1, "Wrong native helper relocation seam")
    namespace = {"__file__": str(file), "__name__": "_portable_native_field_pair"}
    path = sys.path[:]
    names = ("native_runtime", "gallery_transport", "render_helpers", "window_check")
    saved = {name: sys.modules.pop(name, None) for name in names}
    try:
        exec(compile(original.replace(literal, f"Path({str(helpers.resolve())!r})"), str(file), "exec"), namespace)
        yield namespace["readback"]
    finally:
        sys.path[:] = path
        for name, module in saved.items():
            sys.modules.pop(name, None)
            if module is not None:
                sys.modules[name] = module


def native_info(root, tools, helpers, binaries):
    hashes = path_entries(checked(binaries, NATIVE_PAIR))
    require(hashes.keys() == {"surface-material-gallery", "surface-devtools"}, "Wrong native binary inventory")
    captures = {}
    reports = {}
    with native_oracle(tools, helpers) as read:
        for name, (source, manifest, receipt) in NATIVE_RUNS.items():
            run = root / name
            entries = seal(run, "SHA256SUMS", manifest, exact=True)
            observed = read(run.resolve(strict=True), manifest)
            require(observed == data(checked(root / (name + "-verified.json"), receipt))
                    and observed["source"] == source and observed["binaries"] == NATIVE_PAIR
                    and observed["scope"] == "Native synthetic fixed-palette preview"
                    and (observed["original_pngs"], observed["window_samples"], observed["visible_windows"],
                         observed["reaped"]) == (16, 44, 0, 4)
                    and observed["transports"] == ["Page.screenshot", "CLI", "MCP"],
                    "Wrong exact native replay, host-window or transport scope")
            captures[name] = {file: checksum for file, checksum in entries.items() if file.endswith(".png")}
            reports[name] = observed
    require(captures["exact-v0172"] == captures["development-r4"], "Native exact PNGs differ from development R4")
    return {"runs": reports, "matching_development_pngs": len(captures["exact-v0172"]),
            "host_setting_observed": False}


def focus_info(root, source, web_manifest, producer):
    seal(root, "launcher.sha256", FOCUS_TOOLS)
    original = path_entries(checked(root / "original-source.sha256", FOCUS_SOURCE))
    selected = path_entries(checked(root / "focused-source.sha256", FOCUS_SELECTOR))
    exact = path_entries(checked(source, SOURCE))
    published = path_entries(checked(producer, WEB_SOURCE))
    require(len(exact) == len(original) == len(selected) == len(published) == 691
            and delta(original, selected) == [REGISTRY] and delta(original, exact) == DOCS
            and delta(published, original) == WEB_TESTS, "Wrong R4 assets, R8 selector or final doc delta")
    run = root / "development-r8"
    oracle = RUN_PATH(str(checked(root / "verify-system-pair.py", FOCUS_ORACLE)))
    result = normalized(oracle["verify_run"](run))
    expected = data(checked(root / "development-r8-verified.json", FOCUS_RECEIPT))
    inputs = data(run / "invocation.json")["inputs"]
    web = path_entries(checked(web_manifest, FOCUS_WEB), allow_dot_prefix=True)
    require(result == expected and result["checks"] == 1993 and result["steps"] == STEPS
            and result["chrome_samples"] == 233 and result["visible_owned_windows"] == 0
            and len(result["png_sha256"]) == 14 and len(result["retained_disabled_contrast"]["png_sha256"]) == 15
            and inputs["runner_sha256"] == (root / "runner.sha256").read_text().strip() == FOCUS_RUNNER
            and inputs["source_count"] == len(exact) and inputs["product_source_manifest_sha256"] == FOCUS_SOURCE
            and inputs["focused_source_manifest_sha256"] == FOCUS_SELECTOR and inputs["steps"] == STEPS
            and inputs["registry_delta"] == [REGISTRY] and inputs["stock_registry_sha256"] == exact[REGISTRY]
            and inputs["focused_registry_sha256"] == selected[REGISTRY] and inputs["web_manifest_sha256"] == FOCUS_WEB
            and inputs["web_producer_manifest_sha256"] == WEB_SOURCE and inputs["web_compatible_test_only_delta"] == WEB_TESTS
            and inputs["web_files"] == web and len(web) == 4, "Wrong actual R8 execution or browser assets")
    return {**result, "inputs": inputs, "exact_source_delta": DOCS, "gallery_files": web}


R7_SEALS = {
    "original-source.sha256": "6d8d03a89b2e2a95b8c725efe04774badcc5e922b471d51bb5ed8ebdbca40135",
    "focused-source.sha256": "30acef153e870d44361d317768af89b3d63418f095bc84ed96806725962d6d4f",
    "launcher.sha256": "23131ff7df7ff7effb318ead7bce12bcbf32b30f47eb98dee6309441c4cffe1c",
    "development-r7/runner.log": "9076b3eb80ac1ec01374bcdd9094715689e12c43536ad512f0062ad5522d8e11",
    "development-r7/headless-window-observation.json": "83eb7f7c48ac4347d93bdeeb61a419351086b0346975a7d1b5d77d686af61f42",
    f"development-r7/artifacts/{PREFIX}.json": "b5eadb44e8850437f698eb2c72c159c4c079cac45081bcce0e114101f88603ce",
    f"development-r7/artifacts/{PREFIX}-failure.png": "74c196e830721e5829d9384e3ceae737a327e1bbc00c76fd5c9a54c482817a32",
}


def rejected_info(focus, r4, r5, compiler, r7):
    output = {}
    for version, prior, checksum in (("r4", r4, R4_SEAL), ("r5", r5, R5_SEAL)):
        seal(prior, "SHA256SUMS", checksum, exact=True)
        observed = RUN_PATH(str(focus / f"preserve-{version}-rejection.py"))["review"](prior)
        require(observed == data(prior / "verified-rejection.json"), f"Wrong rejected {version} readback")
        output[version] = {**observed, "manifest_sha256": checksum}
    text = raw_log(compiler).decode()
    require(sha256(text.encode()).hexdigest() == R6_COMPILER
            and "error[E0432]: unresolved import" in text and 'could not compile `surface-e2e`' in text,
            "Wrong R6 compiler-only rejection")
    output["r6"] = {"scope": "compiler-only rejection before browser", "log_sha256": R6_COMPILER}
    for name, checksum in R7_SEALS.items():
        checked(r7 / name, checksum)
    run = r7 / "development-r7"
    artifacts = run / "artifacts"
    raw = data(artifacts / (PREFIX + ".json"))
    watch = data(run / WINDOW)
    invocation = data(run / "invocation.json")
    inputs = invocation["inputs"]
    verify_ledgers(run, watch, {"chrome-primary"})
    require(watch["stock_exit"] == 1 and watch["visible_owned_windows"] == watch["remaining_observed_processes"]
            == watch["problems"] == data(run / "owned-shutdown.json")["after_cleanup"] == []
            and invocation["suite_arguments"] == ["material"] and inputs == data(run / "inputs-after.json")
            and inputs["runner_sha256"] == (r7 / "runner.sha256").read_text().strip()
            == "e5ad49db929f696de764d135187c76b9dd504a2879db6e4813e7d6192b7a08aa"
            and inputs["product_source_manifest_sha256"] == R7_SEALS["original-source.sha256"]
            and inputs["focused_source_manifest_sha256"] == R7_SEALS["focused-source.sha256"]
            and inputs["web_manifest_sha256"] == FOCUS_WEB
            and "0 passed; 1 failed; 1726 checks" in (run / "runner.log").read_text()
            and "filled-text-field-demo-show" in raw["failure"] and raw["page_errors"] == [],
            "Wrong R7 rejected scope or guard")
    dom = raw["failure_dom"]
    opener, = [index for index, node in enumerate(dom["dom"]["nodes"])
               if node["stable_id"] == "filled-text-field-demo-show"]
    node, = [item for item in dom["layout"]["nodes"] if item["id"] == opener]
    require((node["bounds"]["y"], node["bounds"]["height"], dom["layout"]["viewport"]["height"]) == (2060, 48, 2000),
            "Wrong observed launch failure geometry")
    pair = RUN_PATH(str(focus / "verify-system-pair.py"))
    selected = raw["outlined"]["selected"]
    pair["verify_highlight"](selected["selection"], raw["outlined"]["accepted"]["highlight"]["background"], [19, 45, 47])
    with Image.open(artifacts / pair["name"]("outlined-selected")) as png:
        require(pair["witnesses"](png.convert("RGB"), selected) > 10, "Wrong R7 selection witnesses")
    images = {}
    for png in artifacts.glob(PREFIX + "-*.png"):
        with Image.open(png) as image:
            require(image.format == "PNG" and image.size == ((1920, 4088) if png.name.endswith("-failure.png") else (1920, 1800)),
                    "Wrong R7 original image type or size")
            image.verify()
        images[png.name] = digest(png)
    require(set(images) == {pair["name"](name) for name in
            ("real-light", "real-dark", "outlined-normal", "outlined-selected", "outlined-caret", "outlined-restored", "failure")},
            "Missing original R7 partial or failure image")
    contrast = RUN_PATH(str(focus / "verify-field-contrast.py"))["verify_captures"](artifacts)
    require(len(contrast["png_sha256"]) == 15 and watch["launch_observations"]["chrome-primary"]["samples"] == 463,
            "Wrong R7 inherited screenshot or process count")
    output["r7"] = {"scope": "rejected clipped setup launcher", "checks": 1726, "visible_windows": 0,
                    "chrome_samples": 463, "launcher_y": 2060, "viewport_height": 2000, "png_sha256": images,
                    "inherited_gray_text_png_sha256": contrast["png_sha256"], "report_sha256": digest(artifacts / (PREFIX + ".json"))}
    return output


def stock_pins():
    absent = pending_markers()
    require(not absent, "Pending exact gates: " + ", ".join(absent))
    return {key: digest(artifact) for key, artifact in (
        ("stock_receipt_sha256", STOCK / "run" / STOCK_RECEIPT),
        ("stock_window_sha256", STOCK / "run" / WINDOW),
        ("stock_log_sha256", STOCK / "run/runner.log"),
        ("stock_offline_sha256", STOCK / STOCK_OFFLINE),
    )}


def reviewed_pins(path, expected):
    require(path is not None and HEX.fullmatch(expected or ""), "Provide the reviewed terminal stock digest file and hash")
    pins = data(checked(path, expected))
    require(set(pins) == {"stock_receipt_sha256", "stock_window_sha256", "stock_log_sha256", "stock_offline_sha256"}
            and all(isinstance(value, str) and HEX.fullmatch(value) for value in pins.values()), "Wrong reviewed stock pins")
    return pins


def legacy_verifier(root):
    verify_entries(root, LEGACY_HASHES)
    seal(root / "retained", "SHA256SUMS", LEGACY_HASHES["retained/SHA256SUMS"])
    return RUN_PATH(str(root / "verify-existing.py"))


def stock_info(root, focused, prior, legacy, pins):
    seal(root, "harness.sha256", STOCK_TOOLS)
    checked(root / "offline-preflight.log", STOCK_PREFLIGHT)
    checked(root / "browser-source.sha256", SOURCE)
    checked(root / "native-binaries.sha256", NATIVE_PAIR)
    reviewed = data(checked(root / "reviewed-pins.json", STOCK_REVIEW))
    offline = data(checked(root / STOCK_OFFLINE, pins["stock_offline_sha256"]))
    run = root / "run"
    typed = data(checked(run / STOCK_RECEIPT, pins["stock_receipt_sha256"]))
    watch = data(checked(run / WINDOW, pins["stock_window_sha256"]))
    checked(run / "runner.log", pins["stock_log_sha256"])
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary", "chrome-bridge", "native"})
    inputs = data(root / "inputs-before.json")
    require(inputs == data(root / "inputs-rechecked.json") == data(run / "inputs-after.json"),
            "The guarded stock input changed")
    seal(root, "sealed-inputs.sha256", inputs["app_bundle_seal_sha256"])
    invocation = data(run / "invocation.json")
    binary = path_entries(root / "browser-binary-sha256.txt")
    origins = data(root / "bundle-origins.json")
    bound = data(root / "focused-proof.json")
    gallery = {PurePosixPath(name).name: checksum for name, checksum in binary.items()
               if PurePosixPath(name).parent == PurePosixPath("web/release/material-gallery")}
    require(inputs == invocation["inputs"] and inputs["source_manifest_sha256"] == SOURCE
            and inputs["reviewed_pins_sha256"] == STOCK_REVIEW and inputs["source_files_checked_per_origin"] == 691
            and inputs["stock_files"] == binary and inputs["bundle_files_checked"] == len(binary) == 16
            and origins.keys() == binary.keys() and inputs["runner_sha256"] == binary["debug/surface-e2e"] == STOCK_RUNNER
            and (root / "approved-runner.sha256").read_text().strip() == STOCK_RUNNER
            and reviewed["source_sha256"] == SOURCE and reviewed["source_tree"] == TREE
            and reviewed["patch_sha256"] == PATCH and reviewed["native_manifest_sha256"] == NATIVE_PAIR
            and inputs["focused_proof_sha256"] == digest(root / "focused-proof.json")
            and offline["focused"] == bound and bound["focused_receipt_sha256"] == FOCUS_RECEIPT
            and bound["canonical_gallery_match"] is True and len(gallery) == 4
            and gallery == focused["gallery_files"] == bound["canonical_gallery_files"]
            and bound["inputs"] == focused["inputs"] and bound["step_ids"] == STEPS
            and bound["checks"] == 1993 and bound["headless_samples"] == 233
            and all(bound[key] == focused[key] for key in ("png_sha256", "retained_disabled_contrast", "report_sha256"))
            and bound["exact_vs_scoped_source_paths"] == DOCS
            and invocation["suite_arguments"] == SUITES and invocation["injected_chrome_flags"] == [UPDATER],
            "Wrong stock source, binaries, invocation or scoped Gallery match")
    argv = invocation["argv"]
    recorded = Path(argv[argv.index("--artifacts") + 1]).parent
    require(recorded.parts[:3] == ("/", "private", "tmp") and argv[0] == origins["debug/surface-e2e"],
            "Wrong original private headless run")
    with stock_observer(root, prior, prior / "boundary", prior / "placeholder") as observer:
        require(inputs["release"] == offline["release"] == observer["read_release_pin"](root)
                and argv == observer["stock_command"](recorded, Path(argv[0]), recorded.parent / "bundle",
                                                      {role: str(recorded / "guards" / role) for role in observer["GUARD_ROLES"]}),
                "Wrong exact stock command")
        replay = normalized(observer["verify_run"](ArchivedRun(run, recorded), watch, inputs["release"]))
    legacy_tool = legacy_verifier(legacy)
    old_path = sys.path[:]
    try:
        earlier = normalized(legacy_tool["retained"](run, root / "browser-source.sha256", SOURCE, 362))
        slider = normalized(legacy_tool["slider_pixels"](run / "artifacts"))
    finally:
        sys.path[:] = old_path
    pair = typed["text_field_system_pair"]
    require(replay == typed and all(offline[key] == value for key, value in typed.items())
            and earlier == offline["retained"] and slider == offline["standard_pixels"]
            and offline["window_receipt_sha256"] == pins["stock_window_sha256"]
            and offline["runner_log_sha256"] == pins["stock_log_sha256"]
            and set(typed["cases"]) == set(SUITES) and typed["cases"]["material"]["steps"] == 361
            and typed["cases"]["ime"]["steps"] == 20 and typed["page_errors"] == 0
            and typed["guard_and_window_checks"] == "passed" and typed["total_checks"] > 29384
            and len(pair["png_sha256"]) == 14 and pair["adapter"] == typed["adapter"]
            and len(typed["text_field_disabled_contrast"]["png_sha256"]) == 15
            and len(typed["text_field_boundary_motion"]["pngs"]) == 33
            and len(typed["text_field_label_motion"]["pngs"]) == 34 and len(typed["text_field_placeholder"]["pngs"]) == 58
            and {"ime_cancellation", "navigation", "menu", "divider", "radio", "checkbox", "switch", "tabs"}
            <= set(earlier["retained"]), "Wrong stock scope, raw field pixels or predecessor checks")
    return {"total_checks": typed["total_checks"], "cases": typed["cases"], "visible_windows": 0,
            "remaining_processes": [], "adapter": typed["adapter"], "browser": typed["browser"],
            "pair_pngs": pair["png_sha256"], "retained_gray_text": 15, "retained_boundary": 33,
            "retained_label_motion": 34, "retained_placeholder": 58,
            "retained_decoders": sorted(earlier["retained"]), "decoded_pair_probes": pair["decoded_probes"]}


def baseline():
    native_bin = EXACT / "binaries"
    verify_entries(native_bin, path_entries(checked(native_bin / "SHA256SUMS", NATIVE_PAIR)))
    focused = focus_info(FOCUS, EXACT / "source.sha256", Q / "web-development.sha256",
                         Q / "development-r4/source.sha256")
    result = {"source": live_source(), "predecessor": predecessor(PREV, live=True), "focused": focused,
              "native": native_info(NATIVE, NATIVE, NATIVE_HELPERS, native_bin / "SHA256SUMS"),
              "rejected_attempts": rejected_info(FOCUS, R4, R5, FOCUS / "r6-compiler.log", R7),
              "rust_metal": rust_metal(Q, Q / "gpu-v0172-exact", GPU_PINS),
              "canonical_build_log_sha256": digest(checked(Q / BUILD_LOG, BUILD_SHA))}
    seal(STOCK, "harness.sha256", STOCK_TOOLS)
    checked(STOCK / "offline-preflight.log", STOCK_PREFLIGHT)
    checked(STOCK / "reviewed-pins.json", STOCK_REVIEW)
    return result


def live_proof(pins, pin_sha):
    stage = baseline()
    original = run_json([STOCK / "verify-stock-receipt.py", STOCK / "run"])
    require(original == data(STOCK / STOCK_OFFLINE), "Original independent stock readback differs")
    return {"status": "verified exact local candidate", "gate_pins_sha256": pin_sha, **stage,
            "stock": stock_info(STOCK, stage["focused"], PREV / "previous", LEGACY, pins)}


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
    for name in (WORKSPACE, *GPU_LOGS, BUILD_LOG):
        compressed(Q / name, destination / "rust" / (name + ".gz"))
    copy_members(Q / "gpu-v0172-exact", destination / "rust/metal", (*proof["rust_metal"]["metal_pngs"], "SHA256SUMS"))

    prior = destination / "previous"
    copy(PREV / "SHA256SUMS", prior / "package-v0171.sha256")
    copy(PREV / PREV_REPORT_NAME, prior / PREV_REPORT_NAME)
    copy(PREV / "stock-macos" / STOCK_OFFLINE, prior / STOCK_OFFLINE)
    copy_members(PREV / "previous", prior, ("pack-boundary.py",))
    copy_sealed(PREV / "previous/boundary", prior / "boundary", "verifier.sha256")
    copy_sealed(PREV / "previous/boundary", prior / "boundary", "launcher.sha256")
    copy_members(PREV / "previous/placeholder", prior / "placeholder", (*OLDER_SCRIPTS, "SHA256SUMS"))
    copy_members(LEGACY, prior / "legacy-slider", LEGACY_HASHES)
    copy_sealed(LEGACY / "retained", prior / "legacy-slider/retained", "SHA256SUMS")

    focused = destination / "focused"
    copy_sealed(FOCUS, focused, "launcher.sha256")
    copy_members(FOCUS, focused, ("runner.sha256", "development-r8-verified.json", "verify-system-pair.py",
                                 "preserve-r4-rejection.py", "preserve-r5-rejection.py", "verifier.sha256",
                                 "gallery-pin.json", "prepared.json"))
    copy(Q / "web-development.sha256", focused / "web.sha256")
    copy(Q / "development-r4/source.sha256", focused / "web-producer-source.sha256")
    run = focused / "development-r8"
    original = FOCUS / "development-r8"
    copy_run(original, run, ("owned-shutdown.json",))
    contrast = proof["focused"]["retained_disabled_contrast"]
    copy_members(original / "artifacts", run / "artifacts", (PREFIX + ".json", *proof["focused"]["png_sha256"],
                 "material-text-field-disabled-contrast.json", *contrast["png_sha256"]))

    rejected = destination / "rejected"
    copy_sealed(R4, rejected / "r4", "SHA256SUMS")
    copy_sealed(R5, rejected / "r5", "SHA256SUMS")
    compressed(FOCUS / "r6-compiler.log", rejected / "r6-compiler.log.gz")
    copy_members(R7, rejected / "r7", ("original-source.sha256", "focused-source.sha256", "runner.sha256",
                                       "launcher.sha256", "scope-pin.json", "gallery-pin.json"))
    run7 = "development-r7"
    copy_run(R7 / run7, rejected / "r7" / run7, ("owned-shutdown.json",))
    reject = proof["rejected_attempts"]["r7"]
    copy_members(R7 / run7 / "artifacts", rejected / "r7" / run7 / "artifacts",
                 (PREFIX + ".json", *reject["png_sha256"], "material-text-field-disabled-contrast.json",
                  *reject["inherited_gray_text_png_sha256"]))

    native = destination / "native"
    copy_sealed(NATIVE, native / "tools", "launcher.sha256")
    copy_sealed(NATIVE_HELPERS, native / "helpers", "SHA256SUMS")
    copy(EXACT / "binaries/SHA256SUMS", native / "binaries.sha256")
    for name in NATIVE_RUNS:
        copy_sealed(NATIVE / name, native / name, "SHA256SUMS")
        copy_members(NATIVE, native, (name + "-verified.json", name + ".log"))

    stock = destination / "stock-macos"
    tools = set(path_entries(STOCK / "harness.sha256")) | set(path_entries(STOCK / "sealed-inputs.sha256"))
    copy_members(STOCK, stock, (*tools, "harness.sha256", "sealed-inputs.sha256", "offline-preflight.log",
                                "inputs-before.json", "inputs-rechecked.json", "approved-runner.sha256", STOCK_OFFLINE))
    stock_run = STOCK / "run"
    copy_run(stock_run, stock / "run", (STOCK_RECEIPT,))
    names, hashes = png_inventory(data(stock / STOCK_OFFLINE))
    for path in (stock_run / "artifacts").rglob("*"):
        if path.suffix in {".png", ".json"} and path.is_file():
            require(not path.is_symlink(), "Unexpected linked stock output")
            if path.suffix == ".json" or path.name in names or digest(path) in hashes:
                copy(path, stock / "run" / path.relative_to(stock_run))

    json_file(destination / REPORT, proof)
    (destination / "README.md").write_text(f"""# Surface v0.1.72 system Field pair evidence

The 691-file candidate tree `{TREE}` applies the
{len(proof["source"]["changed_paths"])}-path patch to v0.1.71 tree `{PREV_TREE}`.
Only the Material coverage document differs from scoped R8.
The exact stock Gallery uses the same four Web assets as scoped R8.

- Exact workspace passed 1,593 Rust tests and three Wasm checks.
  Apple Metal passed 18 tests and produced 21 original PNGs.
- Exact native produced 16 original PNGs across two replays.
  Page.screenshot, CLI and MCP bytes match. All 44 observations showed
  zero visible windows. Four Gallery processes exited. The separate
  R4 development replay has 16 identical PNGs.
- Scoped R8 browser completed 1,993 checks across 13 Material steps.
  Its 233 headless observations found zero visible windows or survivors.
  It includes 14 Field pair and 15 retained disabled-contrast PNGs.
- Full stock completed {proof["stock"]["total_checks"]:,} checks in five suites.
  It includes 361 Material steps through M362, 20 IME steps, 14 Field pair
  PNGs, prior feature decoders and zero visible windows or survivors.

The real equal contrast pairs come from system CSS in headless Chrome
media emulation. The browser also tests a distinct Field and FieldText
pair through a fixture-only probe. Native uses the fixed synthetic-palette
preview, with no change to system preferences. These runs observe
rendering and input. Live assistive speech and hosted CI are outside this
local packet. Rejected R4, R5 and R7 image evidence and the R6
compiler-only failure are in `rejected/`.

Use Python 3.12 with Pillow. Supply the reviewed SHA-256 of `SHA256SUMS`.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-field-pair.py --verify-package . --package-sha "$reviewed_sha"
```

The offline verifier reads the original images and source/launch receipts.
The packet does not include app binaries or browser profiles.
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
    prior, focus, native, rejected = (root / part for part in ("previous", "focused", "native", "rejected"))
    focused = focus_info(focus, root / "source.sha256", focus / "web.sha256", focus / "web-producer-source.sha256")
    build = sha256(raw_log(root / "rust" / (BUILD_LOG + ".gz"))).hexdigest()
    require(build == BUILD_SHA, "Wrong original canonical build log")
    proof = {"status": "verified exact local candidate", "gate_pins_sha256": saved["gate_pins_sha256"],
             "source": source_info(root / "base-source.sha256", root / "source.sha256", raw_log(root / (PATCH_NAME + ".gz"))),
             "predecessor": predecessor(prior), "focused": focused,
             "native": native_info(native, native / "tools", native / "helpers", native / "binaries.sha256"),
             "rejected_attempts": rejected_info(focus, rejected / "r4", rejected / "r5", rejected / "r6-compiler.log.gz", rejected / "r7"),
             "rust_metal": rust_metal(root / "rust", root / "rust/metal", GPU_PINS, archived=True),
             "canonical_build_log_sha256": build,
             "stock": stock_info(root / "stock-macos", focused, prior, prior / "legacy-slider", pins)}
    require(proof == saved, "Portable readback differs from the original verified proof")
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
        result = {"status": "offline preflight", **baseline(), "pending_exact_markers": pending_markers()}
    elif args.read_gate_pins:
        result = stock_pins()
    elif args.verify_package:
        result = verify_package(args.verify_package.resolve(strict=True), args.package_sha)
    else:
        pins = reviewed_pins(args.gate_pins, args.gate_pins_sha)
        require(pins == stock_pins(), "Original stock receipts differ from the reviewed pins")
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
        require(not blocked & audit["observations"].keys(), "Review the path-only publication audit")
        result = {"status": proof["status"], "archive": str(target), "archive_sha256": checksum,
                  "report_sha256": digest(target / REPORT), "stock_checks": proof["stock"]["total_checks"],
                  "publication_audit": audit}
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit(f"Exact Field pair evidence stopped. {error}")
