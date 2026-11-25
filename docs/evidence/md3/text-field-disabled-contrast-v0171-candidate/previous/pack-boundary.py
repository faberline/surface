"""Assemble and recheck the exact v0.1.70 headless evidence without starting an app."""

import argparse
from contextlib import contextmanager
from hashlib import sha256
from pathlib import Path, PurePosixPath
import json
import re
import runpy
import sys
from types import ModuleType

from PIL import Image

from evidence_common import (HEX, clean_windows, compressed, copy, copy_members, data, digest, import_from,
                             json_file, normalized, path_entries, raw_log, require, run_json, seal, source_tree,
                             tests, verify_entries, verify_ledgers)

HERE = Path(__file__).resolve().parent
Q = Path("/tmp/surface-md3-field-boundary-v0170.X35mBqoS")
EXACT = Q / "exact-candidate"
STOCK = Path("/tmp/surface-md3-field-boundary-stock-v0170.Mz6M22zL")
FOCUS = Path("/tmp/surface-md3-field-boundary-focused-v0170.rWtO97J5")
NATIVE = Path("/tmp/surface-md3-field-boundary-native.705YkFQO")
PREV = Path("/tmp/surface-md3-placeholder-v0169-package-r2.xzjZ49Cw/portable")
OLD_FOCUS = Path("/tmp/surface-md3-field-placeholder-focused-v0169-r2.8AArzw7l")
PRIOR_SCRIPTS = ("observe-tabs.py", "prepare-focused.py", "pixel-oracle.py", "verify-label-color-original.py",
                 "verify-placeholder.py")
SOURCE = "bc6f3023d68c8bec8c801c7a9a2d7c88207d625697cffb34f1eb4f5fef5a6d85"
TREE = "409a21f6023802f055371cdec4391976c55ffd17"
PATCH = "ca4fe795f6a347be0ddd6296b7bca2537ba70b645b93cfc641228f833f31095f"
PATCH_NAME = "text-field-boundary-v0170.patch"
PRIOR_SOURCE = "1b1379f04bbb534b6a25e857185de34160b367bfa60ef9a36a1fce1d0a8d3d55"
PRIOR_TREE = "b243564b27c4fb8d2856afc5ae19cd4cb2ad905a"
PRIOR_SEAL = "6e6069ce438e25821707ad230972f3a435397cdc98d054b36fddeb8ad774d4e8"
PRIOR_REPORT = "7a4bb0c9eb92353c0212bf3e9450a4125bb85831a7555f029a93f0172b992b25"
PRIOR_OFFLINE = "1fb80a2d85cd6a9642ee60006d06c1694de25662c1f993d57b372377d38d5a8c"
PRIOR_FOCUS_TOOLS = "a55188fe39c863318f9e73fd36a639540d70b11f362d266e5ec1ab8b139e3824"
DEV_SOURCE = "f0b9bf4f43c9ec853fd2ec327028dabfe8215846ef0beec6222f3ac7fecb684c"
FOCUS_SOURCE = "27113e50b159dbefc3901f41a3858b7ae901b3a40b1aa12141e2fda21046797d"
FOCUS_WEB = "cd449385e300bf6a78226070f0a8941534520ab461abf3ddaa764b9e5f47304a"
FOCUS_RUNNER = "0bad066e5bc85537bee80913848dcb87cea5968c6b54fd3cf9ef8a43e46b2833"
FOCUS_LAUNCHER = "dcd18bb0f9a6456fffef989de863ea32a5b00e5c01447bcec8cde908f65015fc"
FOCUS_TOOLS = "b826c77a2cd8ebd0a27a80d12df42e1ba32b6dedb0a02e938ce3adaa12869192"
FOCUS_RECEIPT = "eda2fc14c5b0972ffd5aa396e2778fbaa3b1657d96c38398d1802e89d8e3492c"
SHALLOW = "f413e0269d4dd963572a2965d27bc464a8e2e3f1379b0dab726222d4bef9612d"
NATIVE_TOOLS = "34291d67bd5e47502553bcbf0171b424b4bee2f9be0fa7e5f2873b7c235d7575"
NATIVE_PAIR = "6bd269e0a9fe5b38e4c887aee0342d916b0a42b1157cb9b2378953485fc51a58"
NATIVE_EXACT = "374bc9e179b3501bcc7403c241b8f44eea1a836b59071d1991d0731fdaf89233"
NATIVE_RECEIPT = "cc77028a210bf4319ac7eca376bb3eaa97a76408a9b2aa24e120b034e0594bf1"
NATIVE_R1 = "9bda05e79bc98924cf3a896d7bb2db99aae448236aedf29eddb60f615a28deb3"
NATIVE_R1_RECEIPT = "c4ac73ffcfab1f4f7433df8c20950d7f7336389df46bfb6e9319a9a4c46e8e4d"
STOCK_TOOLS = "68bdc8273786ca5f3eea55637179d6c1d0f1de8a486dbc6209252a1271062f1e"
STOCK_REVIEW = "62374f01b7b2180463fbf2ab70d7528b9ffc754fe53331cedfce98b8dea40979"
STOCK_RUNNER = "cbc7b3664c3dff0bfed3fab4178fd2d0de21dabac403e4a2b1368459fa4b859d"
STOCK_OFFLINE = "stock-mac-exact-offline.json"
STOCK_RECEIPT = "verified-boundary-stock.json"
STOCK_MARKER = "stock-mac-v0170-passed"
REPORT = "verified-boundary-v0170.json"
WINDOW = "headless-window-observation.json"
UPDATER = "--disable-updater-scheduler"
SUITES = ["smoke", "ime", "material", "bridge-web", "devtools-headless"]
STEPS = [f"M{step}" for step in range(325, 357)]
DOCS = ["docs/material-coverage.json"]
REGISTRY = "e2e/src/material.rs"
LOGS = {
    "workspace-v0170-exact.log": "72f380a8cabdfa83613490413b5a924789ec83a6a4b6bc2eb9953e0d6b735061",
    "gpu-v0170-exact.log": "bd003cf774a1a1c5d8c382b869c9dec0243a1f0c33faad481828e0477f8cbb82",
    "gpu-v0170-supplemental.log": "93cd914f2571f1348b4669d93231a30a649d5c11862c74d856122095d06e1ab1",
}
METAL_IMAGES = "c7fb34d394d7702a9e9f6e8c617d552486dfb52bf766fce7bc4c8259440c409f"
MARKERS = {
    "workspace-v0170-passed": Q, "gpu-v0170-passed": Q, "native-final-built": EXACT,
    "stock-canonical-built": EXACT, "native-v0170-passed": Q, STOCK_MARKER: STOCK,
}
RUN_MEMBERS = (WINDOW, "invocation.json", "inputs-after.json", "runner.log")
ARTIFACT_REPORTS = ("material-text-field-boundary-motion.json", "material-text-field-label-motion.json",
                    "material-text-field-placeholder.json")
RUN_PATH = runpy.run_path


def checked(path, expected):
    require(path.is_file() and not path.is_symlink() and digest(path) == expected, f"Wrong reviewed file {path.name}")
    return path


def delta(before, after):
    return sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))


def source_info(base, manifest, patch):
    before = path_entries(checked(base, PRIOR_SOURCE))
    after = path_entries(checked(manifest, SOURCE))
    patch_paths = re.findall(r"^diff --git a/(\S+) b/\S+$", patch.decode(), re.M)
    require(sha256(patch).hexdigest() == PATCH and len(before) == 688 and len(after) == 689
            and len(patch_paths) == 12 and sorted(patch_paths) == delta(before, after),
            "Exact source or patch paths differ")
    return {"tree": TREE, "manifest_sha256": SOURCE, "files": len(after), "patch_sha256": PATCH,
            "base_tree": PRIOR_TREE, "base_manifest_sha256": PRIOR_SOURCE, "changed_paths": sorted(patch_paths)}


def live_source():
    source = source_info(PREV / "source.sha256", EXACT / "source.sha256", (Q / PATCH_NAME).read_bytes())
    root, entries = EXACT / "source", path_entries(EXACT / "source.sha256")
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file() and p.relative_to(root).parts[0] != ".git"}
    require(actual == entries.keys() and source_tree(root, entries) == TREE
            and (Q / "exact-apply-checked").read_text() == TREE + "\n", "Wrong current exact source")
    return source


def prior_info(path, seal_name, offline):
    file = path / "verified-placeholder-v0169.json"
    entries = path_entries(checked(path / seal_name, PRIOR_SEAL))
    require(len(entries) == 654 and entries[file.name] == digest(file) == PRIOR_REPORT
            and entries["stock-macos/" + STOCK_OFFLINE] == digest(offline) == PRIOR_OFFLINE,
            "Wrong .69 portable predecessor")
    report, stock = data(file), data(offline)
    require(report["source"]["tree"] == PRIOR_TREE and report["stock_macos"]["checks"] == stock["total_checks"] == 25851
            and stock["cases"]["material"]["steps"] == 352
            and len(stock["text_field_placeholder"]["pngs"]) == 58,
            "Wrong completed .69 stock evidence")
    return {"tree": PRIOR_TREE, "archive_sha256": PRIOR_SEAL, "stock_checks": 25851, "material_steps": 352}


def pending_markers(archived=None, repair=None):
    absent = []
    for name, scope in MARKERS.items():
        base = repair if name == STOCK_MARKER and repair is not None else scope
        candidate = (archived if archived is not None else base) / name
        if not candidate.exists():
            absent.append(name)
        else:
            content = b"" if name.startswith("stock-mac-") else (TREE + "\n").encode()
            require(candidate.is_file() and not candidate.is_symlink() and candidate.read_bytes() == content,
                    f"Wrong exact marker {name}")
    return absent


def repair_info(path, expected, stock):
    entries = seal(path, "SHA256SUMS", expected)
    require({STOCK_MARKER, STOCK_OFFLINE, "repair-provenance.json", "replay-stock.py"} <= entries.keys()
            and (path / STOCK_MARKER).read_bytes() == b"", "Incomplete completed stock repair")
    provenance = data(path / "repair-provenance.json")
    actual = {
        "source_sha256": SOURCE, "tree": TREE, "original_harness_sha256": STOCK_TOOLS,
        "runner_log_sha256": digest(stock / "run/runner.log"),
        "window_receipt_sha256": digest(stock / "run" / WINDOW),
        "observer_receipt_sha256": digest(stock / "run" / STOCK_RECEIPT),
        "offline_receipt_sha256": digest(path / STOCK_OFFLINE),
    }
    require(all(provenance.get(name) == value for name, value in actual.items()),
            "The offline repair is not bound to this immutable stock run")
    return actual


def read_gate_pins(repair=None, repair_sha=None):
    pending = pending_markers(repair=repair)
    require(not pending, "Pending exact gates: " + ", ".join(pending))
    if repair is not None:
        repair_info(repair, repair_sha, STOCK)
    return {"source_sha256": SOURCE, "tree": TREE, "stock_runner_sha256": STOCK_RUNNER,
            "stock_review_sha256": STOCK_REVIEW, "stock_receipt_sha256": digest(STOCK / "run" / STOCK_RECEIPT),
            "stock_window_sha256": digest(STOCK / "run" / WINDOW),
            "stock_offline_sha256": digest((repair or STOCK) / STOCK_OFFLINE), "stock_repair_sha256": repair_sha}


def reviewed_pins(path, expected):
    require(path is not None and HEX.fullmatch(expected or ""), "Supply the reviewed terminal gate pin file and hash")
    pins = data(checked(path, expected))
    require(pins.keys() == {"source_sha256", "tree", "stock_runner_sha256", "stock_review_sha256",
                            "stock_receipt_sha256", "stock_window_sha256", "stock_offline_sha256", "stock_repair_sha256"}
            and pins["tree"] == TREE and pins["source_sha256"] == SOURCE
            and pins["stock_runner_sha256"] == STOCK_RUNNER and pins["stock_review_sha256"] == STOCK_REVIEW
            and all(HEX.fullmatch(value) for key, value in pins.items() if key not in {"tree", "stock_repair_sha256"})
            and (pins["stock_repair_sha256"] is None or HEX.fullmatch(pins["stock_repair_sha256"])),
            "Wrong reviewed terminal gate pins")
    return pins


def rust_metal(rust, metal, archived=False):
    counts = []
    for name, wanted in LOGS.items():
        path = rust / (name + ".gz" if archived else name)
        text, count = tests(path)
        require(count["log_sha256"] == wanted, f"Wrong exact log {name}")
        counts.append((text, count))
    (workspace, full), (base, included), (extra, added) = counts
    require((full["passed"], full["ignored"]) == (1585, 20)
            and workspace.rsplit("\ntest result:", 1)[-1].count("Finished `dev` profile") == 3
            and "provenance:" in workspace, "Wrong exact Rust/provenance/Wasm scope")
    require((included["passed"], included["ignored"], added["passed"], added["ignored"]) == (16, 0, 2, 0)
            and 'name: "Apple M1 Max"' in base and "backend: Metal" in base and extra.count("backend: Metal") == 2,
            "Wrong actual Metal scope")
    manifest = checked(metal / "SHA256SUMS", METAL_IMAGES)
    originals = {name.removeprefix("./"): value for value, name in
                 (line.split("  ", 1) for line in manifest.read_text().splitlines())}
    require(set(originals) == {p.name for p in metal.glob("*.png")} and len(originals) == 21,
            "Wrong actual Metal originals")
    verify_entries(metal, originals)
    for name in originals:
        with Image.open(metal / name) as image:
            require(image.format == "PNG", "Wrong Metal original type")
            image.verify()
    return {"workspace": {**full, "wasm_checks": 3}, "metal": included, "supplemental": added,
            "metal_total": 18, "metal_pngs": originals}


def native_info(tools, run, typed, previous, previous_typed):
    seal(tools, "SHA256SUMS", NATIVE_TOOLS)
    originals = seal(run, "SHA256SUMS", NATIVE_EXACT, exact=True)
    native = run_json([tools / "replay-boundaries.py", "--verify-stored", run,
                       "--source-manifest-sha", SOURCE, "--binary-manifest-sha", NATIVE_PAIR,
                       "--evidence-manifest-sha", NATIVE_EXACT])
    former = path_entries(checked(previous, NATIVE_R1))
    previous_result = data(checked(previous_typed, NATIVE_R1_RECEIPT))
    require(native == data(checked(typed, NATIVE_RECEIPT))
            and (native["repeats"], native["original_pngs"], native["observed_window_samples"],
                 native["visible_owned_windows"], native["reaped_gallery_processes"]) == (2, 18, 48, 0, 4)
            and native["transports"] == ["Page.screenshot", "CLI", "MCP"]
            and previous_result["source_manifest_sha256"] == DEV_SOURCE
            and {key: value for key, value in originals.items() if key.endswith(".png")}
            == {key: value for key, value in former.items() if key.endswith(".png")},
            "Wrong native result, cancellation or original byte parity")
    return {**native, "matching_development_pngs": 18}


@contextmanager
def relocated_verifiers(focus, previous):
    """Rebind only the audited predecessor path literal of the unchanged verifier files."""
    recorded = 'Path("' + str(OLD_FOCUS) + '")'
    names = {"verify-boundary.py", "verify-inherited-label.py"}
    target = focus.resolve(strict=True)
    previous = previous.resolve(strict=True)

    def load(path, *args, **kwargs):
        file = Path(path).resolve()
        if file.parent != target or file.name not in names:
            return RUN_PATH(str(path), *args, **kwargs)
        source = file.read_text()
        require(source.count(recorded) == 1, "Unexpected inherited verifier relocation seam")
        name = "_boundary_archive_" + file.stem.replace("-", "_")
        module = ModuleType(name)
        module.__file__ = str(file)
        old = sys.modules.get(name)
        sys.modules[name] = module
        try:
            exec(compile(source.replace(recorded, f"Path({str(previous)!r})"), str(file), "exec"), module.__dict__)
        finally:
            if old is None:
                sys.modules.pop(name)
            else:
                sys.modules[name] = old
        return module.__dict__

    runpy.run_path = load
    try:
        yield load
    finally:
        runpy.run_path = RUN_PATH


def focus_info(root, previous, exact_manifest, live=False):
    seal(root, "launcher.sha256", FOCUS_LAUNCHER)
    seal(root, "verifier.sha256", FOCUS_TOOLS)
    old_manifest = path_entries(checked(previous / "SHA256SUMS", PRIOR_FOCUS_TOOLS))
    verify_entries(previous, {name: old_manifest[name] for name in PRIOR_SCRIPTS})
    original = path_entries(checked(root / "original-source.sha256", DEV_SOURCE))
    selected = path_entries(checked(root / "focused-source.sha256", FOCUS_SOURCE))
    exact = path_entries(checked(exact_manifest, SOURCE))
    require(original.keys() == selected.keys() == exact.keys() and len(exact) == 689
            and delta(original, selected) == [REGISTRY] and delta(original, exact) == DOCS,
            "Wrong registry selector or final-to-tested docs scope")
    run = root / "development-r1"
    with relocated_verifiers(root, previous) as load:
        verifier = load(str(root / "verify-boundary.py"))
        captures = {**normalized(verifier["verify"](run / "artifacts")),
                    "inherited_label_motion": normalized(verifier["LABEL"]["verify_artifacts"](run / "artifacts")),
                    "inherited_placeholder": normalized(verifier["PLACEHOLDER"]["verify"](run / "artifacts"))}
        if live:
            require(normalized(verifier["verify_run"](run.resolve(strict=True))) == data(root / "development-r1-verified.json"),
                    "Original scoped readback differs")
    result = data(checked(root / "development-r1-verified.json", FOCUS_RECEIPT))
    window = data(run / WINDOW)
    clean_windows(window)
    verify_ledgers(run, window, {"chrome-primary"})
    invocation = data(run / "invocation.json")
    argv = invocation["argv"]
    recorded = Path(argv[argv.index("--artifacts") + 1]).parent
    focus_root = Path(argv[0]).parents[1]
    log = (run / "runner.log").read_text()
    require(all(result[key] == value for key, value in captures.items()) and captures["decoded_probes"] == 2478
            and len(captures["pngs"]) == 33 and len(captures["inherited_label_motion"]["captures"]) == 34
            and len(captures["inherited_placeholder"]["pngs"]) == 58
            and result["steps"] == re.findall(r"^  \[(M\d+)\]", log, re.M) == STEPS
            and "ok (32 steps, 5228 checks," in log and "1 passed; 0 failed; 5228 checks;" in log
            and window["launch_observations"]["chrome-primary"]["samples"] == result["headless_samples"] == 547
            and not data(run / "owned-shutdown.json")["after_cleanup"]
            and result["window_receipt_sha256"] == digest(run / WINDOW)
            and result["runner_log_sha256"] == digest(run / "runner.log")
            and result["inputs"] == invocation["inputs"] == data(run / "inputs-after.json")
            and result["inputs"]["web_manifest_sha256"] == FOCUS_WEB
            and result["inputs"]["runner_sha256"] == (root / "runner.sha256").read_text().strip() == FOCUS_RUNNER
            and recorded.parent == focus_root and recorded.parts[:3] == ("/", "private", "tmp")
            and argv == [str(focus_root / "pins/surface-e2e"), "--chrome", str(recorded / "guards/chrome"),
                         "--chrome-arg", UPDATER, "--web-root", str(focus_root / "web"), "--artifacts",
                         str(recorded / "artifacts"), "material"], "Wrong actual scoped result or input proof")
    old_edge = data(checked(root / "early-shallow-readback.json", SHALLOW))
    require(old_edge["cut"]["covered_quarters"] == [0, 0, 1]
            and old_edge["uncut"]["covered_quarters"] == [4, 4, 3], "Inherited M347 shallow readback changed")
    return {**result, "final_docs_only_delta": DOCS, "shallow_replay_sha256": SHALLOW}


class ArchivedRun:
    """Read original files locally while checking the recorded private profile roots."""

    def __init__(self, directory, recorded):
        self.directory = directory
        self.recorded = recorded

    def __truediv__(self, part):
        return self.directory / part

    def resolve(self):
        return self.recorded


def stock_info(root, focus_root, previous, focused, pins, receipt_root=None):
    seal(root, "harness.sha256", STOCK_TOOLS)
    run = root / "run"
    checked(root / "reviewed-pins.json", STOCK_REVIEW)
    checked(root / "browser-source.sha256", SOURCE)
    checked(root / "native-binaries.sha256", NATIVE_PAIR)
    typed = data(checked(run / STOCK_RECEIPT, pins["stock_receipt_sha256"]))
    watch = data(checked(run / WINDOW, pins["stock_window_sha256"]))
    offline = data(checked((receipt_root or root) / STOCK_OFFLINE, pins["stock_offline_sha256"]))
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
            and inputs["reviewed_pins_sha256"] == STOCK_REVIEW and inputs["source_files_checked_per_origin"] == 689
            and inputs["stock_files"] == binary and inputs["bundle_files_checked"] == len(binary) == 16
            and origins.keys() == binary.keys() and inputs["runner_sha256"] == binary["debug/surface-e2e"] == STOCK_RUNNER
            and (root / "approved-runner.sha256").read_text().strip() == STOCK_RUNNER
            and inputs["focused_proof_sha256"] == digest(root / "focused-proof.json")
            and offline["focused"] == bound and bound["focused_receipt_sha256"] == FOCUS_RECEIPT
            and bound["canonical_gallery_match"] is True and len(gallery) == 4
            and gallery == focused["inputs"]["web_files"] == bound["canonical_gallery_files"]
            and all(bound[key] == focused[key] for key in ("pngs", "inherited_label_motion", "inherited_placeholder"))
            and invocation["suite_arguments"] == SUITES and invocation["injected_chrome_flags"] == [UPDATER],
            "Wrong stock source, binary, canonical Gallery or launch scope")
    argv = invocation["argv"]
    recorded = Path(argv[argv.index("--artifacts") + 1]).parent
    require(recorded.parts[:3] == ("/", "private", "tmp") and argv[0] == origins["debug/surface-e2e"],
            "Wrong recorded private run or runner origin")
    with import_from(root), relocated_verifiers(focus_root, previous) as load:
        observer = load(str(root / "observe-multiline.py"))
        boundary = sys.modules["verify_boundary_stock"]
        old_focus = boundary.DEFAULT_FOCUS
        boundary.DEFAULT_FOCUS = focus_root
        try:
            require(inputs["release"] == offline["release"] == observer["read_release_pin"](root)
                    and argv == observer["stock_command"](recorded, Path(argv[0]), recorded.parent / "bundle",
                                                           {role: str(recorded / "guards" / role)
                                                            for role in observer["GUARD_ROLES"]}),
                    "Wrong actual exact stock command")
            replayed = normalized(observer["verify_run"](ArchivedRun(run, recorded), watch, inputs["release"]))
        finally:
            boundary.DEFAULT_FOCUS = old_focus
    require(replayed == typed and all(offline[key] == value for key, value in typed.items())
            and offline["window_receipt_sha256"] == pins["stock_window_sha256"]
            and offline["runner_log_sha256"] == digest(run / "runner.log")
            and typed["page_errors"] == 0 and typed["guard_and_window_checks"] == "passed"
            and typed["total_checks"] == 28706 and len(typed["cases"]) == 5
            and typed["cases"]["material"]["steps"] == 355 and typed["cases"]["ime"]["steps"] == 20
            and typed["text_field_boundary_motion"]["decoded_probes"] == 2478
            and len(typed["text_field_boundary_motion"]["pngs"]) == 33
            and len(typed["text_field_label_motion"]["pngs"]) == 34
            and len(typed["text_field_placeholder"]["pngs"]) == 58
            and typed["text_field_label_motion"]["outlined"]["early_shallow"]["coverage_quarters"]
                == {"cut": [0, 0, 1], "uncut": [4, 4, 3]}
            and {"ime_cancellation", "navigation", "menu", "divider", "radio", "checkbox", "switch", "tabs"}
                <= set(offline["retained"]["retained"]), "Wrong completed stock scope or redecoded evidence")
    return {"total_checks": typed["total_checks"], "cases": typed["cases"], "visible_windows": 0,
            "remaining_processes": [], "adapter": typed["adapter"], "browser": typed["browser"],
            "boundary_pngs": typed["text_field_boundary_motion"]["pngs"],
            "label_pngs": typed["text_field_label_motion"]["pngs"],
            "placeholder_pngs": typed["text_field_placeholder"]["pngs"]}


def preflight():
    source = live_source()
    seal(STOCK, "harness.sha256", STOCK_TOOLS)
    seal(PREV, "SHA256SUMS", PRIOR_SEAL, exact=True)
    result = {
        "source": source, "predecessor": prior_info(PREV, "SHA256SUMS", PREV / "stock-macos" / STOCK_OFFLINE),
        "focused": focus_info(FOCUS, OLD_FOCUS, EXACT / "source.sha256", live=True),
        "rust_metal": rust_metal(Q, Q / "gpu-v0170-exact"),
        "native": native_info(NATIVE, Q / "native-exact-replay", Q / "native-v0170-verified.json",
                              NATIVE / "development-r1/SHA256SUMS", NATIVE / "development-r1-verified.json"),
    }
    require(not set(pending_markers()) - {STOCK_MARKER}, "A non-stock exact gate is missing")
    return result


def named_pngs(receipt):
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
    ledgers = (p.relative_to(source).as_posix() for p in (source / "guard-ledger").glob("*.json"))
    copy_members(source, target, (*RUN_MEMBERS, *extra, *ledgers))


def archive(target, pin_path, proof, repair=None):
    require(target.is_relative_to(HERE) and target != HERE and not target.exists(), "Use a fresh output under the package lane")
    target.mkdir(mode=0o700)
    copy_members(HERE, target, ("pack-boundary.py", "evidence_common.py", "audit-publication.py"))
    copy(EXACT / "source.sha256", target / "source.sha256")
    copy(PREV / "source.sha256", target / "base-source.sha256")
    copy(pin_path, target / "gate-pins.json")
    compressed(Q / PATCH_NAME, target / (PATCH_NAME + ".gz"))
    for name, base in MARKERS.items():
        if name == STOCK_MARKER and repair is not None:
            base = repair
        copy(base / name, target / "gates" / name)
    for name in LOGS:
        compressed(Q / name, target / "rust" / (name + ".gz"))
    metal = Q / "gpu-v0170-exact"
    copy_members(metal, target / "rust/metal", (*proof["rust_metal"]["metal_pngs"], "SHA256SUMS"))
    prior = target / "previous"
    copy(PREV / "SHA256SUMS", prior / "package-v0169.sha256")
    copy(PREV / "verified-placeholder-v0169.json", prior / "verified-placeholder-v0169.json")
    copy(PREV / "stock-macos" / STOCK_OFFLINE, prior / STOCK_OFFLINE)
    copy_members(OLD_FOCUS, prior / "focused-tools", (*PRIOR_SCRIPTS, "SHA256SUMS"))

    f = target / "focused"
    focus_members = (*path_entries(FOCUS / "launcher.sha256"), *path_entries(FOCUS / "verifier.sha256"),
                     "launcher.sha256", "verifier.sha256", "runner.sha256",
                     "development-r1-verified.json", "early-shallow-readback.json")
    copy_members(FOCUS, f, focus_members)
    copy(checked(Q / "web-development.sha256", FOCUS_WEB), f / "web.sha256")
    photos = proof["focused"]
    raw = f / "development-r1"
    original = FOCUS / "development-r1"
    copy_run(original, raw, ("owned-shutdown.json",))
    copy_members(original / "artifacts", raw / "artifacts",
                 (*ARTIFACT_REPORTS, *photos["pngs"], *photos["inherited_label_motion"]["captures"],
                  *photos["inherited_placeholder"]["pngs"]))

    native = target / "native"
    run = Q / "native-exact-replay"
    copy_members(NATIVE, native / "tools", (*path_entries(NATIVE / "SHA256SUMS"), "SHA256SUMS"))
    copy_members(run, native / "replay", (*path_entries(run / "SHA256SUMS"), "SHA256SUMS"))
    copy(Q / "native-v0170-verified.json", native / "verified.json")
    copy(EXACT / "binaries/SHA256SUMS", native / "binaries.sha256")
    copy(NATIVE / "development-r1/SHA256SUMS", native / "development-pngs.sha256")
    copy(NATIVE / "development-r1-verified.json", native / "development-verified.json")

    stock = target / "stock-macos"
    tools = set(path_entries(STOCK / "harness.sha256")) | set(path_entries(STOCK / "sealed-inputs.sha256"))
    copy_members(STOCK, stock, (*tools, "harness.sha256", "sealed-inputs.sha256",
                                "inputs-before.json", "approved-runner.sha256"))
    copy((repair or STOCK) / STOCK_OFFLINE, stock / STOCK_OFFLINE)
    if repair is not None:
        copy_members(repair, stock / "offline-repair", (*path_entries(repair / "SHA256SUMS"), "SHA256SUMS"))
    stock_run = STOCK / "run"
    copy_run(stock_run, stock / "run", (STOCK_RECEIPT,))
    names, hashes = named_pngs(data(stock / STOCK_OFFLINE))
    for path in (stock_run / "artifacts").rglob("*"):
        if path.suffix not in {".png", ".json"} or not path.is_file():
            continue
        require(not path.is_symlink(), "Unexpected linked stock output")
        if path.suffix == ".json" or path.name in names or digest(path) in hashes:
            copy(path, stock / "run" / path.relative_to(stock_run))

    json_file(target / REPORT, proof)
    (target / "README.md").write_text(f"""# Surface v0.1.70 text-field boundary evidence

The exact 689-file tree `{TREE}` applies the 12-path patch to the
v0.1.69 tree `{PRIOR_TREE}`. The scoped Web and native development
sources differ from the exact source only in the coverage document.

- Exact Rust passed 1,585 tests, provenance, lint and three Wasm checks.
  Actual Apple Metal passed 18 tests and produced 21 original PNGs.
- The exact native run repeated 18 original PNGs byte-for-byte through
  Page.screenshot, CLI and MCP. Its 48 observations showed no windows.
  All four Gallery processes exited.
- Scoped Chrome passed 5,228 checks in 32 steps. It records 33 new
  boundary PNGs plus 92 earlier PNGs on the same product. Its 547
  samples showed zero visible windows or surviving processes.
- Exact stock passed {proof["stock"]["total_checks"]:,} checks in five
  suites. It includes 355 ordered Material steps through M356, 20
  IME steps, 2,478 independent boundary probes and zero windows or survivors.

Use Python 3.12 with Pillow. Keep the original packet seal from the handoff.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-boundary.py --verify-package . --package-sha "$reviewed_sha"
```

The verifier opens the sealed original files without launching a
browser or Gallery. It includes only fixture captures and input hashes.
Build caches and binaries stay outside the packet. Original receipts
include the temporary paths of their guarded processes. The native
evidence covers Reduce Motion endpoints. Browser composition
and platform semantics are separate from live speech or OS input.
""")
    manifest = target / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(p)}  {p.relative_to(target).as_posix()}\n"
                                for p in sorted(target.rglob("*")) if p.is_file()))
    return digest(manifest)


def verify_package(root, checksum):
    seal(root, "SHA256SUMS", checksum, exact=True)
    saved = data(root / REPORT)
    pins = reviewed_pins(root / "gate-pins.json", saved["gate_pins_sha256"])
    require(not pending_markers(root / "gates"), "Missing archived exact gate")
    source = source_info(root / "base-source.sha256", root / "source.sha256", raw_log(root / (PATCH_NAME + ".gz")))
    previous = root / "previous"
    focused = focus_info(root / "focused", previous / "focused-tools", root / "source.sha256")
    gallery = path_entries(checked(root / "focused/web.sha256", FOCUS_WEB), allow_dot_prefix=True)
    require(gallery == focused["inputs"]["web_files"] and len(gallery) == 4, "Wrong focused Gallery manifest")
    native = root / "native"
    checked(native / "binaries.sha256", NATIVE_PAIR)
    stock = root / "stock-macos"
    repair_sha = pins["stock_repair_sha256"]
    result = {
        "status": "verified exact local candidate", "gate_pins_sha256": saved["gate_pins_sha256"], "source": source,
        "predecessor": prior_info(previous, "package-v0169.sha256", previous / STOCK_OFFLINE),
        "focused": focused, "rust_metal": rust_metal(root / "rust", root / "rust/metal", archived=True),
        "native": native_info(native / "tools", native / "replay", native / "verified.json",
                              native / "development-pngs.sha256", native / "development-verified.json"),
        "stock": stock_info(stock, root / "focused", previous / "focused-tools", focused, pins),
        "stock_offline_repair": None if repair_sha is None else repair_info(stock / "offline-repair", repair_sha, stock),
    }
    require(saved == result, "Archive readback differs from actual verified proof")
    return result


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
    parser.add_argument("--repair", type=Path)
    parser.add_argument("--repair-sha")
    args = parser.parse_args()
    require((args.repair is None and args.repair_sha is None)
            or (args.repair is not None and HEX.fullmatch(args.repair_sha or "")), "Supply both repair path and approved seal")
    if args.preflight:
        recovered = repair_info(args.repair, args.repair_sha, STOCK) if args.repair is not None else None
        result = {"status": "offline preflight", **preflight(), "pending_exact_markers": pending_markers(repair=args.repair)}
        if recovered is not None:
            result["stock_offline_repair"] = recovered
    elif args.read_gate_pins:
        result = read_gate_pins(args.repair, args.repair_sha)
    elif args.verify_package:
        result = verify_package(args.verify_package.resolve(strict=True), args.package_sha)
    else:
        pin = reviewed_pins(args.gate_pins, args.gate_pins_sha)
        require(pin == read_gate_pins(args.repair, args.repair_sha), "The current terminal gates differ from the reviewed digests")
        claims = preflight()
        if args.repair is None:
            independent = run_json([STOCK / "verify-stock-receipt.py", STOCK / "run"])
            require(independent == data(STOCK / STOCK_OFFLINE), "Existing stock readback changed")
            recovered = None
        else:
            recovered = repair_info(args.repair, args.repair_sha, STOCK)
            retained_tool = checked(Path("/tmp/surface-md3-sliders-stock.lK6a9gKl/verify-existing.py"),
                                    "eded5dfdfcf94dc901cb3608b83cc86a557fe4cb02b91122416c6a03e3568558")
            retained = RUN_PATH(str(retained_tool))
            prior = data(args.repair / STOCK_OFFLINE)
            require(normalized(retained["retained"](STOCK / "run", STOCK / "browser-source.sha256", SOURCE, 356))
                    == prior["retained"]
                    and normalized(retained["slider_pixels"](STOCK / "run/artifacts")) == prior["standard_pixels"],
                    "Preserved predecessor checks differ after the offline repair")
        proof = {"status": "verified exact local candidate", "gate_pins_sha256": args.gate_pins_sha, **claims,
                 "stock": stock_info(STOCK, FOCUS, OLD_FOCUS, claims["focused"], pin, args.repair),
                 "stock_offline_repair": recovered}
        target = args.out.resolve()
        seal_sha = archive(target, args.gate_pins, proof, args.repair)
        require(run_json([target / "pack-boundary.py", "--verify-package", target, "--package-sha", seal_sha])
                == proof, "Independent portable readback differs")
        safety = run_json([target / "audit-publication.py", target, "--seal", seal_sha])
        blocked = {"credential_token", "private_key", "bearer_value", "jwt", "credential_like_json_field",
                   "url_auth", "private_network_urls", "runtime_loopback_urls", "mail_address",
                   "os_session_directory", "other_url_origin", "unparsed_url", "binary_signature",
                   "unrecognized_nontext", "png_text_exif_or_icc_metadata", "large_file"}
        require(not blocked & safety["observations"].keys(),
                "Publication scan needs review. Run the path-only audit against the packet")
        result = {"status": proof["status"], "archive": str(target), "archive_sha256": seal_sha,
                  "report_sha256": digest(target / REPORT), "stock_checks": proof["stock"]["total_checks"],
                  "publication_audit": safety}
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit(f"Exact boundary proof stopped. {error}")
