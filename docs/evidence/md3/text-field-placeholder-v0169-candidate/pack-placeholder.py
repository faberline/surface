"""Assemble and verify the exact local v0.1.69 evidence. Never start an app."""

import argparse
from hashlib import sha256
import json
from pathlib import Path, PurePosixPath
import re
from runpy import run_path
import sys
import tomllib

from PIL import Image

from evidence_common import (HEX, clean_windows, compressed, copy, copy_members, data, digest, import_from,
                             json_file, normalized, path_entries, raw_log, require, run_json, seal,
                             source_tree, tests, verify_entries, verify_ledgers)

HERE = Path(__file__).resolve().parent
Q = Path("/tmp/surface-md3-text-field-placeholder-v0169.bc9QKVhY")
EXACT = Q / "exact-candidate"
PREV = Path("/tmp/surface-md3-field-color-v0168-package.hdUSH5cz/portable")
PREV_SOURCE = "/tmp/surface-md3-text-field-label-color-v0168.XNrxeeOp/exact-candidate/source"
R1 = Path("/tmp/surface-md3-field-placeholder-focused-v0169.612HMDhp")
R2 = Path("/tmp/surface-md3-field-placeholder-focused-v0169-r2.8AArzw7l")
NATIVE = Path("/tmp/surface-md3-placeholder-native.oHJCRmeE")
STOCK = Path("/tmp/surface-md3-field-placeholder-stock-v0169.RECb56Zt")
TREE = "b243564b27c4fb8d2856afc5ae19cd4cb2ad905a"
SOURCE = "1b1379f04bbb534b6a25e857185de34160b367bfa60ef9a36a1fce1d0a8d3d55"
PATCH = "fb2ad6586be0d5d1c7957ce53d91b37119a1c53bbe3e550488e9b24325c295ba"
BASE_TREE = "0db939d3f663fdb5b9fa3dd334382937693b409e"
BASE_SOURCE = "17aeee497edef781a3da477d8fdb7820c0eaa3494a4b840fcf41133925949df6"
BASE_ARCHIVE = "5e19c8384a44e5ed31e5d61d1a7f03e14234476960b2f78ba6b78ba23fc37811"
BASE_PROOF = "b86bbf7eacbe3399170feb7ea314d99edad8692e7e7acf3197c7e8f0c6c5d9f1"
BASE_STOCK = "010a9848cecc3f0f1b0dddbab6d2e498b2d93b62865f3d6221a6191117cdafaa"
BASE_PUBLISHED = "14b110f1c269c1127389be68b3e9c7cb40e69b5f"
R1_SOURCE = "15e771de1fc3e14fddc8810e3d3c6388b823b477b124eb3f6d43171320f224e7"
R1_FAILURE = "67e9d45be83a8e627fe8dda4c3fcf634cbebdd8dc8f9609de5ab9f4cd7bd1c32"
R1_TOOLS = "e11d704203e7b8365c3ef771aeae526849ab33dc46fbf8bb93b2cfdde14bea1e"
R2_SOURCE = "47642376bb5f97f07a542ba6c1fb159e1bdeda1d904cfb4c3dc39096d2c75354"
R2_SELECTED = "783c7463a93b34a33db63e709c52dec7817d44af6f17579cb4e40fcc7a249c30"
R2_TOOLS = "a55188fe39c863318f9e73fd36a639540d70b11f362d266e5ec1ab8b139e3824"
R2_RUNNER = "bd2f0a2bba9b2c0e969eff3352bbf04980bc38c3ac19c865f4ecf9dc5a391e55"
R2_VERIFIED = "dcf665eb0c3b390322ac8c7e5bef9590a4759ca871417ce13f9afa6e3b66fd8f"
R2_WEB = "700ba34b6e25aaf7406f0ecbdbae056274a6fba9725e7a3e120a6e099e81cd33"
NATIVE_TOOLS = "0c7827f26ce712114923b47d8228a913c045622fe3d9583d8ff7748e70b12d00"
NATIVE_BINARY = "85ea052ae4553819e92be2d9313430c0dbe3956c3d98566b48c2ffc931634ec7"
NATIVE_R1 = "00ea3c9da4f3eac2e83dbd3077ee290db7e2db8789beadd8441f03afeee879f8"
NATIVE_R1_PROOF = "ce1880e01dbb3f50d63955412e0bca3aad3e24a7ece7616b319f1b5a3afd1cc5"
STOCK_TOOLS = "0622adc060c174054646cb971089d0133632b4ce6a7dd0e662bb845d79d88e26"
PATCH_NAME = "text-field-placeholder-v0169.patch"
REPORT = "verified-placeholder-v0169.json"
STOCK_REPORT = "verified-placeholder-stock.json"
STOCK_OFFLINE = "stock-mac-exact-offline.json"
R2_REPORT = "development-r2-authorized-verified.json"
R2_RUN = "development-r2-authorized"
WINDOW = "headless-window-observation.json"
UPDATER = "--disable-updater-scheduler"
MOTION = "material-text-field-label-motion"
PLACEHOLDER = "material-text-field-placeholder"
BARRIER = "e2e/src/material_text_field_label_motion.rs"
REGISTRY = "e2e/src/material.rs"
DOCS = ["docs/material-coverage.json"]
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
STEPS = [f"M{step}" for step in range(325, 354)]
MARKERS = ("workspace-v0169-passed", "gpu-v0169-passed", "native-final-built", "stock-canonical-built",
           "native-v0169-passed", "stock-mac-v0169-passed")
LOGS = ("workspace-v0169-exact.log", "gpu-v0169-exact.log", "gpu-v0169-supplemental.log")
RUN_MEMBERS = (WINDOW, "invocation.json", "inputs-after.json", "runner.log")
OBSERVED_RUST = {
    "workspace_log": "01a09046b3849c9406922f0581f44cc53d22fe903a091c60efff019950ad6547",
    "gpu_log": "21bae779f7b9ebda397f35aa72b7248715c4c7ed3c76f8539ab87fe10ea74d8b",
    "gpu_supplemental_log": "5c1b2eaa98edf7633ffd06bfdd62248f4e2df6bd6bcd0a7afb11a3d80d937db8",
    "gpu_manifest": "c7fb34d394d7702a9e9f6e8c617d552486dfb52bf766fce7bc4c8259440c409f",
}


def changed(before, after):
    return sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))


def manifest(path, wanted):
    require(digest(path) == wanted, f"Wrong pinned manifest {path.name}")
    return path_entries(path)


def source_info(base, exact, patch):
    before, after = manifest(base, BASE_SOURCE), manifest(exact, SOURCE)
    require(sha256(patch).hexdigest() == PATCH and (len(before), len(after)) == (685, 688),
            "Wrong exact v0.1.69 patch or source inventory")
    paths = re.findall(r"^diff --git a/(\S+) b/\S+$", patch.decode(), re.M)
    require(len(paths) == len(set(paths)) == 16 and sorted(paths) == changed(before, after)
            and "crates/surface-app/src/placeholder_motion.rs" in paths and BARRIER in paths,
            "Wrong exact 16-path patch scope")
    return {"version": "0.1.69", "tree": TREE, "source_sha256": SOURCE, "source_files": len(after),
            "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
            "changed_paths": sorted(paths)}


def check_live_sources():
    source = source_info(PREV / "source.sha256", EXACT / "source.sha256", (Q / PATCH_NAME).read_bytes())
    for path, seal_path, tree in ((Path(PREV_SOURCE), PREV / "source.sha256", BASE_TREE),
                                  (EXACT / "source", EXACT / "source.sha256", TREE)):
        entries = path_entries(seal_path)
        require(set(entries) == {item.relative_to(path).as_posix() for item in path.rglob("*")
                                 if item.is_file() and item.relative_to(path).parts[0] != ".git"},
                "Wrong on-disk source inventory")
        verify_entries(path, entries)
        require(source_tree(path, entries) == tree, "Wrong exact source tree")
    version = tomllib.loads((EXACT / "source/Cargo.toml").read_text())["workspace"]["package"]["version"]
    registry = (EXACT / "source" / REGISTRY).read_text().split("pub const STEPS:", 1)[1].split("];", 1)[0]
    require(version == "0.1.69"
            and [int(step) for step in re.findall(r'"M(\d+)"', registry)] == [i for i in range(1, 354) if i != 23]
            and all((Q / name).read_text() == TREE + "\n" for name in ("exact-apply-checked", "source-tree")),
            "Wrong release version, exact replay stamp or stock registry")
    return source


def predecessor(root):
    entries = manifest(root / "package-v0168.sha256", BASE_ARCHIVE)
    proof_name, offline_name = "verified-label-color-v0168.json", STOCK_OFFLINE
    require(len(entries) == 400 and digest(root / proof_name) == entries[proof_name] == BASE_PROOF
            and digest(root / offline_name) == entries[f"stock-macos/{offline_name}"] == BASE_STOCK,
            "Wrong retained .68 archive")
    proof, stock = data(root / proof_name), data(root / offline_name)
    require(proof["source"]["tree"] == BASE_TREE and proof["source"]["source_sha256"] == BASE_SOURCE
            and proof["stock_macos"]["total_checks"] == stock["total_checks"] == 25276
            and proof["stock_macos"]["visible_windows"] == proof["stock_macos"]["surviving_processes"] == 0
            and stock["cases"]["material"]["steps"] == 349 and len(stock["text_field_label_motion"]["pngs"]) == 34,
            "Wrong retained predecessor coverage")
    return {"commit": BASE_PUBLISHED, "archive_sha256": BASE_ARCHIVE, "source_tree": BASE_TREE,
            "stock_checks": 25276, "material_steps": 349, "label_pngs": 34}


def red_info(root, r2):
    entries = manifest(r2 / "r1-failure.sha256", R1_FAILURE)
    verify_entries(root, entries)
    seal(root, "SHA256SUMS", R1_TOOLS)
    manifest(root / "original-source.sha256", R1_SOURCE)
    run = root / "development-r1-authorized"
    log, watch = (run / "runner.log").read_text(), data(run / WINDOW)
    report = data(run / "artifacts" / f"{MOTION}.json")
    require(len(entries) == 83 and "last step: M350" in log and "0 passed; 1 failed; 1786 checks" in log
            and report["failure"] == "the wrapped fallback changes only ink at the same glyph positions"
            and watch["stock_exit"] == 1 and watch["problems"] == []
            and watch["visible_owned_windows"] == watch["remaining_observed_processes"] == [],
            "Original observed R1 failure differs")
    verify_ledgers(run, watch, {"chrome-primary"})
    return {"last_step": "M350", "checks": 1786, "failure": report["failure"],
            "raw_entries": len(entries), "raw_manifest_sha256": R1_FAILURE,
            "failure_png_sha256": digest(run / "artifacts" / f"{MOTION}-failure.png"),
            "visible_windows": 0, "surviving_processes": 0}


def focus_info(root, first, exact_manifest):
    """Replay the real scoped pixels from original bytes, without archived build binaries."""
    seal(root, "SHA256SUMS", R2_TOOLS)
    require(digest(root / R2_REPORT) == R2_VERIFIED and (root / "runner.sha256").read_text() == R2_RUNNER + "\n",
            "Wrong exact R2 result or runner")
    r1 = manifest(first / "original-source.sha256", R1_SOURCE)
    r2 = manifest(root / "original-source.sha256", R2_SOURCE)
    selected = manifest(root / "focused-source.sha256", R2_SELECTED)
    exact = manifest(exact_manifest, SOURCE)
    require(r1.keys() == r2.keys() == selected.keys() == exact.keys()
            and changed(r1, r2) == [BARRIER] and changed(r2, selected) == [REGISTRY] and changed(r2, exact) == DOCS,
            "Focused selector, R1 paint barrier or final docs delta differs")
    run = root / R2_RUN
    observation = data(run / WINDOW)
    clean_windows(observation)
    verify_ledgers(run, observation, {"chrome-primary"})
    verifier = run_path(str(root / "verify-placeholder.py"))
    artifacts = run / "artifacts"
    physical = normalized(verifier["verify"](artifacts))
    retained = normalized(verifier["PAST"]["verify_artifacts"](artifacts))
    receipt = data(root / R2_REPORT)
    invocation = data(run / "invocation.json")
    profile = observation["guard_events"][0]
    argv = invocation["argv"]
    recorded_run = Path(argv[argv.index("--artifacts") + 1]).parent
    recorded_root = Path(argv[0]).parents[1]
    require(recorded_run.parent == recorded_root and recorded_run.parts[:3] == ("/", "private", "tmp")
            and argv == [str(recorded_root / "pins/surface-e2e"), "--chrome", str(recorded_run / "guards/chrome"),
                         "--chrome-arg", UPDATER, "--web-root", str(recorded_root / "web"), "--artifacts",
                         str(recorded_run / "artifacts"), "material"]
            and verifier["OBS"]["guard_verdict"]("chrome", profile["argv"], {}, recorded_run / "tmp")
            == ("chrome-primary", True, profile["profile"]),
            "R2 process or original command scope changed")
    web = manifest(root / "web.sha256", R2_WEB)
    inputs = {**data(root / "scope-pin.json"), **data(root / "gallery-pin.json"), "runner_sha256": R2_RUNNER}
    log = (run / "runner.log").read_text()
    require(all(receipt[key] == value for key, value in physical.items()) and retained == receipt["retained_label_motion"]
            and len(physical["pngs"]) == 58 and len(retained["captures"]) == 34 and len(web) == 4
            and receipt["inputs"] == invocation["inputs"] == data(run / "inputs-after.json") == inputs
            and inputs["web_files"] == web and inputs["web_compatible_test_only_delta"] == [BARRIER]
            and receipt["steps"] == re.findall(r"^  \[(M\d+)\]", log, re.M) == STEPS
            and invocation["suite_arguments"] == ["material"] and invocation["injected_chrome_flags"] == [UPDATER]
            and receipt["checks"] == 2373 and "ok (29 steps, 2373 checks," in log
            and "1 passed; 0 failed; 2373 checks;" in log
            and receipt["headless_samples"] == observation["launch_observations"]["chrome-primary"]["samples"] == 443
            and receipt["visible_windows"] == 0 and receipt["remaining_owned_processes"] == []
            and receipt["window_receipt_sha256"] == digest(run / WINDOW)
            and receipt["runner_log_sha256"] == digest(run / "runner.log")
            and data(run / "owned-shutdown.json")["after_cleanup"] == [],
            "Wrong original R2 check count, launch, files or rendered proof")
    return {"checks": 2373, "steps": STEPS, "headless_samples": 443, "visible_windows": 0,
            "surviving_processes": 0, "receipt_sha256": R2_VERIFIED, "runner_sha256": R2_RUNNER,
            "web_files": web, "browser": physical["browser"], "adapter": physical["adapter"],
            "placeholder_pngs": physical["pngs"], "retained_label_pngs": retained["captures"],
            "r1_to_r2_test_only_delta": [BARRIER], "final_to_scoped_docs_only_delta": DOCS}


def marker_paths(stock):
    return {name: (stock if name.startswith("stock-mac-") else
                   EXACT if name in ("native-final-built", "stock-canonical-built") else Q) / name for name in MARKERS}


def pending_markers(paths):
    pending = []
    for name in MARKERS:
        path = paths[name]
        if not path.exists():
            pending.append(name)
        else:
            expected = b"" if name == "stock-mac-v0169-passed" else (TREE + "\n").encode()
            require(path.is_file() and not path.is_symlink() and path.read_bytes() == expected,
                    f"Wrong exact marker {name}")
    return pending


def reviewed_gate(value):
    hashes = {"native_manifest", "native_receipt", "stock_runner", "stock_offline", "stock_reviewed_config",
              "stock_observer_receipt", "stock_window_receipt"}
    require(set(value) == {*OBSERVED_RUST, *hashes, "tree", "source_sha256"}
            and value["tree"] == TREE and value["source_sha256"] == SOURCE
            and all(value[name] == wanted for name, wanted in OBSERVED_RUST.items())
            and all(isinstance(value[name], str) and HEX.fullmatch(value[name]) for name in hashes),
            "Reviewed terminal gates are incomplete or from another candidate")
    return value


def verified_file(path, wanted):
    require(path.is_file() and not path.is_symlink() and digest(path) == wanted,
            f"Wrong reviewed evidence {path.name}")


def read_gate_pins(stock):
    pending = pending_markers(marker_paths(stock))
    require(not pending, "Pending exact gates: " + ", ".join(pending))
    return reviewed_gate({"tree": TREE, "source_sha256": SOURCE, **OBSERVED_RUST,
                          "native_manifest": digest(Q / "native-exact-replay/SHA256SUMS"),
                          "native_receipt": digest(Q / "native-v0169-verified.json"),
                          "stock_runner": (stock / "approved-runner.sha256").read_text().strip(),
                          "stock_offline": digest(stock / STOCK_OFFLINE),
                          "stock_reviewed_config": digest(stock / "reviewed-pins.json"),
                          "stock_observer_receipt": digest(stock / "run" / STOCK_REPORT),
                          "stock_window_receipt": digest(stock / "run" / WINDOW)})


def rust_metal(logs, image_root, pins):
    full, primary, extra = [tests(path) for path in logs]
    for (log, record), name in zip((full, primary, extra),
                                   ("workspace_log", "gpu_log", "gpu_supplemental_log"), strict=True):
        require(record["log_sha256"] == pins[name], f"Wrong exact {name}")
    require((full[1]["passed"], full[1]["ignored"]) == (1583, 20)
            and full[0].rsplit("\ntest result:", 1)[-1].count("Finished `dev` profile") == 3
            and re.search(r"^provenance: .+ self-test cases passed\)", full[0], re.M)
            and full[0].count("test placeholder_motion::tests::") == 6
            and "placeholder_only_paints_and_yields_to_both_native_composition_routes ... ok" in full[0],
            "Wrong Rust, provenance, composition or Wasm scope")
    require((primary[1]["passed"], primary[1]["ignored"], extra[1]["passed"], extra[1]["ignored"]) == (16, 0, 2, 0)
            and "backend: Metal" in primary[0] and extra[0].count("backend: Metal") == 2
            and 'name: "Apple M1 Max"' in primary[0], "Wrong actual Metal proof")
    seal_path = image_root / "SHA256SUMS"
    verified_file(seal_path, pins["gpu_manifest"])
    entries = {name.removeprefix("./"): value for value, name in
               (line.split("  ", 1) for line in seal_path.read_text().splitlines())}
    require(len(entries) == 21 and all(re.fullmatch(r"gpu-[a-z-]+\.png", n) and HEX.fullmatch(v) for n, v in entries.items())
            and set(entries) == {path.name for path in image_root.glob("*.png")}, "Wrong Metal original inventory")
    verify_entries(image_root, entries)
    for name in entries:
        with Image.open(image_root / name) as image:
            require(image.format == "PNG" and min(image.size) > 0, f"Wrong Metal PNG {name}")
            image.verify()
    return {"workspace": {**full[1], "wasm_checks": 3}, "standard_metal": primary[1],
            "supplemental_metal": extra[1], "metal_total": 18, "metal_pngs": entries}


def native_result(harness, replay, binaries, report, manifest_sha, report_sha, source_sha):
    seal(harness, "SHA256SUMS", NATIVE_TOOLS)
    verified_file(report, report_sha)
    verified_file(binaries, NATIVE_BINARY)
    entries = seal(replay, "SHA256SUMS", manifest_sha, exact=True)
    fresh = run_json([harness / "verify-stored-placeholder.py", replay, "--manifest-sha", manifest_sha,
                      "--source-manifest-sha", source_sha, "--binary-manifest-sha", NATIVE_BINARY])
    require(fresh == data(report) and
            (fresh["repeats"], fresh["original_pngs"], fresh["visible_owned_windows"],
             fresh["observed_window_samples"], fresh["reaped_gallery_processes"]) == (2, 40, 0, 92, 4)
            and fresh["transports"] == ["Page.screenshot", "CLI", "MCP"], "Wrong native endpoint or transport scope")
    return fresh, {name: value for name, value in entries.items() if name.endswith(".png")}


def native_exact(harness, replay, binaries, report, pins, development_seal, development_result):
    exact, pngs = native_result(harness, replay, binaries, report, pins["native_manifest"], pins["native_receipt"], SOURCE)
    developed = manifest(development_seal, NATIVE_R1)
    verified_file(development_result, NATIVE_R1_PROOF)
    require(data(development_result)["source_manifest_sha256"] == R1_SOURCE
            and pngs == {name: value for name, value in developed.items() if name.endswith(".png")},
            "Exact and R1 native endpoints differ")
    return {**exact, "matching_r1_pngs": len(pngs)}


class ArchivedRun:
    """Open originals locally and validate profiles against their recorded private root."""

    def __init__(self, directory, recorded):
        self.directory = directory
        self.recorded = recorded

    def __truediv__(self, member):
        return self.directory / member

    def resolve(self):
        return self.recorded


def stock_info(root, pins, focus):
    seal(root, "harness.sha256", STOCK_TOOLS)
    run = root / "run"
    for path, name in ((root / STOCK_OFFLINE, "stock_offline"), (root / "reviewed-pins.json", "stock_reviewed_config"),
                       (run / STOCK_REPORT, "stock_observer_receipt"), (run / WINDOW, "stock_window_receipt")):
        verified_file(path, pins[name])
    with import_from(root):
        observer = run_path(str(root / "observe-multiline.py"))
        config = run_path(str(root / "stock_config.py"))
    reviewed = config["reviewed"](root / "reviewed-pins.json")
    require(reviewed["stock_runner_sha256"] == pins["stock_runner"]
            and reviewed["native_manifest_sha256"] == NATIVE_BINARY and reviewed["exact_vs_scoped_source_paths"] == DOCS,
            "Stock reviewed runtime input differs")
    offline, verdict = data(root / STOCK_OFFLINE), data(run / STOCK_REPORT)
    inputs = data(root / "inputs-before.json")
    invocation = data(run / "invocation.json")
    watch = data(run / WINDOW)
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary", "chrome-bridge", "native"})
    seal(root, "sealed-inputs.sha256", inputs["app_bundle_seal_sha256"])
    require(len(manifest(root / "browser-source.sha256", SOURCE)) == 688, "Wrong stock source pin")
    require(digest(root / "native-binaries.sha256") == NATIVE_BINARY, "Wrong stock native manifest")
    binaries = path_entries(root / "browser-binary-sha256.txt")
    markers = data(root / "build-markers.json")
    marker_sha = sha256((TREE + "\n").encode()).hexdigest()
    require(set(markers) == {"native", "web"} and {v["sha256"] for v in markers.values()} == {marker_sha}
            and len({v["path"] for v in markers.values()}) == 2, "Wrong stock build markers")
    require(inputs == data(run / "inputs-after.json") == invocation["inputs"]
            and inputs["stock_files"] == binaries and len(binaries) == inputs["bundle_files_checked"] == 16
            and inputs["runner_sha256"] == binaries["debug/surface-e2e"] == pins["stock_runner"]
            == (root / "approved-runner.sha256").read_text().strip()
            and inputs["reviewed_pins_sha256"] == pins["stock_reviewed_config"]
            and inputs["source_manifest_sha256"] == SOURCE and inputs["source_files_checked_per_origin"] == 688
            and tuple(invocation["suite_arguments"]) == SUITES and invocation["injected_chrome_flags"] == [UPDATER]
            and inputs["release"] == offline["release"] == observer["read_release_pin"](root),
            "Stock source, runner, launch or bundle differs")
    focus_bound = data(root / "focused-proof.json")
    canonical_gallery = {PurePosixPath(name).name: digest for name, digest in binaries.items()
                         if PurePosixPath(name).parent == PurePosixPath("web/release/material-gallery")}
    require(inputs["focused_proof_sha256"] == digest(root / "focused-proof.json")
            and offline["focused"] == focus_bound and focus_bound["focused_receipt_sha256"] == R2_VERIFIED
            and focus_bound["canonical_gallery_match"] is True
            and canonical_gallery == focus_bound["canonical_gallery_files"] == focus_bound["gallery_files"] == focus["web_files"]
            and focus_bound["pngs"] == focus["placeholder_pngs"]
            and focus_bound["retained_label_motion"]["captures"] == focus["retained_label_pngs"],
            "Original focused probe and exact Gallery do not match")
    argv = invocation["argv"]
    recorded = Path(argv[argv.index("--artifacts") + 1]).parent
    recorded_bundle = Path(argv[0]).parents[1]
    require(recorded.parts[:3] == ("/", "private", "tmp") and recorded.parent == recorded_bundle.parent
            and argv == observer["stock_command"](recorded, Path(argv[0]), recorded_bundle,
                                                   {name: str(recorded / "guards" / name) for name in observer["GUARD_ROLES"]}),
            "Wrong original exact stock command")
    original = normalized(observer["verify_run"](ArchivedRun(run, recorded), watch, inputs["release"]))
    require(original == verdict and all(offline[key] == value for key, value in verdict.items())
            and offline["window_receipt_sha256"] == digest(run / WINDOW)
            and offline["runner_log_sha256"] == digest(run / "runner.log")
            and original["guard_and_window_checks"] == "passed" and original["cases"]["material"]["steps"] == 352
            and original["cases"]["ime"]["steps"] == 20 and len(original["multiline_text_field"]["pngs"]) == 12
            and len(original["textbox_values"]["inactive_ax"]) == 6
            and len(original["text_field_label_motion"]["pngs"]) == 34 and len(original["text_field_placeholder"]["pngs"]) == 58
            and set(offline["retained"]["retained"]) >= {"ime_cancellation", "navigation", "menu", "divider", "radio",
                                                        "checkbox", "switch", "tabs"},
            "Wrong completed stock coverage or original pixel evidence")
    return {"runner_sha256": pins["stock_runner"], "offline_sha256": pins["stock_offline"],
            "visible_windows": 0, "surviving_processes": 0, "cases": original["cases"],
            "checks": original["total_checks"], "browser": original["browser"], "adapter": original["adapter"],
            "label_pngs": original["text_field_label_motion"]["pngs"],
            "placeholder_pngs": original["text_field_placeholder"]["pngs"], "canonical_gallery_match": True}


def live_predecessor():
    seal(PREV, "SHA256SUMS", BASE_ARCHIVE, exact=True)
    require((PREV.parent / "published-evidence-commit").read_text() == BASE_PUBLISHED + "\n",
            "Prior publication receipt changed")
    return {"commit": BASE_PUBLISHED, "archive_sha256": BASE_ARCHIVE, "source_tree": BASE_TREE,
            "stock_checks": 25276, "material_steps": 349, "label_pngs": 34}


def live_native(pins):
    bins = EXACT / "binaries"
    verified_file(bins / "SHA256SUMS", NATIVE_BINARY)
    verify_entries(bins, path_entries(bins / "SHA256SUMS"))
    return native_exact(NATIVE, Q / "native-exact-replay", bins / "SHA256SUMS", Q / "native-v0169-verified.json", pins,
                        NATIVE / "development-r1/SHA256SUMS", NATIVE / "development-r1-verified.json")


def preflight(stock):
    pending = pending_markers(marker_paths(stock))
    seal(stock, "harness.sha256", STOCK_TOOLS)
    with import_from(R2):
        scoped_live = run_path(str(R2 / "verify-placeholder.py"))["verify_run"]((R2 / R2_RUN).resolve(strict=True))
    require(normalized(scoped_live) == data(R2 / R2_REPORT), "Original R2 production and scoped inputs changed")
    exact = check_live_sources()
    focused = focus_info(R2, R1, EXACT / "source.sha256")
    development, _ = native_result(
        NATIVE, NATIVE / "development-r1", Q / "native-development-r1/binaries/SHA256SUMS",
        NATIVE / "development-r1-verified.json", NATIVE_R1, NATIVE_R1_PROOF, R1_SOURCE,
    )
    result = {"status": "offline preflight", "source": exact, "predecessor": live_predecessor(),
              "original_r1_failure": red_info(R1, R2), "focused_r2": focused,
              "native_r1_development": development, "pending_exact_markers": pending}
    if not {"workspace-v0169-passed", "gpu-v0169-passed"} & set(pending):
        result["exact_rust_metal"] = rust_metal([Q / name for name in LOGS], Q / "gpu-v0169-exact", OBSERVED_RUST)
    if "native-v0169-passed" not in pending:
        result["exact_native"] = live_native({
            "native_manifest": digest(Q / "native-exact-replay/SHA256SUMS"),
            "native_receipt": digest(Q / "native-v0169-verified.json"),
        })
    return result


def approved_pins(path, expected):
    require(path is not None and HEX.fullmatch(expected or ""), "Supply the reviewed gate file and its SHA-256")
    verified_file(path, expected)
    return reviewed_gate(data(path))


def exact_live(stock, gate_file, gate_sha):
    pins = approved_pins(gate_file, gate_sha)
    require(read_gate_pins(stock) == pins, "Current completed gates differ from reviewed input digests")
    prepared = preflight(stock)
    require(prepared["pending_exact_markers"] == [], "Exact gates disappeared")
    independent = run_json([stock / "verify-stock-receipt.py", stock / "run"])
    require(independent == data(stock / STOCK_OFFLINE), "Independent exact stock readback changed")
    return {"status": "verified local exact candidate", "gate_pins_sha256": gate_sha,
            **{key: val for key, val in prepared.items() if key not in {"status", "pending_exact_markers"}},
            "stock_macos": stock_info(stock, pins, prepared["focused_r2"])}


def recorded_run(source, target, receipts=()):
    ledger = tuple(path.relative_to(source).as_posix() for path in (source / "guard-ledger").glob("*.json"))
    copy_members(source, target, (*RUN_MEMBERS, *ledger, *receipts))


def referenced_pngs(receipt):
    names, hashes = set(), set()
    pending = [receipt]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            pending.extend((*value.keys(), *value.values()))
        elif isinstance(value, list):
            pending.extend(value)
        elif isinstance(value, str):
            if re.fullmatch(r"[A-Za-z0-9_.+-]+\.png", value):
                names.add(value)
            elif HEX.fullmatch(value):
                hashes.add(value)
    return names, hashes


def archive_stock(source, target):
    tools = path_entries(source / "harness.sha256")
    inputs = path_entries(source / "sealed-inputs.sha256")
    copy_members(source, target, (*sorted(tools.keys() | inputs.keys()), "harness.sha256", "sealed-inputs.sha256",
                                  "inputs-before.json", "approved-runner.sha256", STOCK_OFFLINE))
    run, archived = source / "run", target / "run"
    recorded_run(run, archived, (STOCK_REPORT,))
    names, hashes = referenced_pngs(data(source / STOCK_OFFLINE))
    for item in (run / "artifacts").rglob("*"):
        if item.suffix not in {".json", ".png"} or not item.is_file():
            continue
        require(not item.is_symlink(), "Unexpected linked stock artifact")
        if item.suffix == ".json" or item.name in names or digest(item) in hashes:
            copy(item, archived / "artifacts" / item.relative_to(run / "artifacts"))


def archive(target, stock, gates, verified):
    require(target.parent.is_dir() and target.is_relative_to(HERE)
            and target != HERE and not target.exists(), "Use a fresh package directory inside this /tmp lane")
    target.mkdir(mode=0o700)
    copy_members(HERE, target, ("pack-placeholder.py", "evidence_common.py"))
    copy(PREV / "source.sha256", target / "base-source.sha256")
    copy(EXACT / "source.sha256", target / "source.sha256")
    copy(gates, target / "gate-pins.json")
    compressed(Q / PATCH_NAME, target / (PATCH_NAME + ".gz"))
    for name, path in marker_paths(stock).items():
        copy(path, target / "gates" / name)
    for name in LOGS:
        compressed(Q / name, target / "rust" / (name + ".gz"))
    copy_members(Q / "gpu-v0169-exact", target / "rust/metal",
                 (*verified["exact_rust_metal"]["metal_pngs"], "SHA256SUMS"))
    prior = target / "previous"
    copy(PREV / "SHA256SUMS", prior / "package-v0168.sha256")
    copy(PREV / "verified-label-color-v0168.json", prior / "verified-label-color-v0168.json")
    copy(PREV / "stock-macos" / STOCK_OFFLINE, prior / STOCK_OFFLINE)
    dev = target / "focused"
    copy_members(R1, dev / "r1", (*path_entries(R1 / "SHA256SUMS"), "SHA256SUMS", "runner.sha256", "web.sha256",
                                 *path_entries(R2 / "r1-failure.sha256")))
    copy_members(R2, dev / "r2", (*path_entries(R2 / "SHA256SUMS"), "SHA256SUMS", "runner.sha256", R2_REPORT))
    raw = dev / "r2" / R2_RUN
    recorded_run(R2 / R2_RUN, raw, ("owned-shutdown.json",))
    frames = verified["focused_r2"]
    files = [f"{PLACEHOLDER}.json", f"{MOTION}.json", *frames["placeholder_pngs"], *frames["retained_label_pngs"]]
    copy_members(R2 / R2_RUN / "artifacts", raw / "artifacts", files)
    native = target / "native"
    replay = Q / "native-exact-replay"
    copy_members(NATIVE, native / "harness", (*path_entries(NATIVE / "SHA256SUMS"), "SHA256SUMS"))
    copy_members(replay, native / "replay", (*path_entries(replay / "SHA256SUMS"), "SHA256SUMS"))
    copy(EXACT / "binaries/SHA256SUMS", native / "binaries.sha256")
    copy(Q / "native-v0169-verified.json", native / "verified.json")
    copy(NATIVE / "development-r1/SHA256SUMS", native / "r1-native-captures.sha256")
    copy(NATIVE / "development-r1-verified.json", native / "r1-native-verified.json")
    archive_stock(stock, target / "stock-macos")
    json_file(target / REPORT, verified)
    exact_native, exact_stock = verified["exact_native"], verified["stock_macos"]
    prior_url = f"https://github.com/faberline/surface/tree/{BASE_PUBLISHED}/docs/evidence/md3/text-field-label-color-v0168-candidate"
    (target / "README.md").write_text(f"""# Surface v0.1.69 placeholder candidate proof

This packet binds the local exact source tree `{TREE}` and all 688 source
paths to the 16-path patch. The [v0.1.68 evidence package]({prior_url}) is the
recorded predecessor. This packet covers local acceptance.

- The exact workspace passed 1,583 Rust tests and three Wasm checks. Real
  Apple Metal ran 18 tests and produced 21 retained PNGs.
- Native exact headless testing ran two repeats and made 40 original
  endpoint PNGs. All match development R1. Page.screenshot, CLI and MCP
  returned equal bytes. Four Gallery processes were reaped and
  {exact_native["observed_window_samples"]} samples showed zero visible windows.
- The original R1 run stopped at M350. Its PNG, typed output and full
  raw receipt remain in `focused/r1`. R2 adds a paint barrier in that
  test. It keeps the original checks and Gallery bytes.
- Scoped R2 passed 2,373 checks in 29 steps. It retains 34 label motion and
  color PNGs and adds 58 paired placeholder PNGs. Its 443 process samples
  show zero visible windows or surviving processes.
- Exact stock passed {exact_stock["checks"]:,} checks across five suites. It
  ran 352 Material steps through M353, 20 IME steps and the prior feature
  journey. It produced both sets of rendered PNGs, with zero visible
  windows or surviving processes.

Browser frames cover real WebGPU pixels and Chrome accessibility output.
The IME evidence records browser composition and controlled input events.
Native evidence covers reduced-motion endpoints and Runtime semantics.
Live speech and OS input composition need separate proof.

## Offline replay

Use Python 3.12 and Pillow with the reviewed package seal.

~~~sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-placeholder.py --verify-package . --package-sha "$reviewed_sha"
~~~

The readback opens the archived original images and process receipts. It
needs no browser, native executable or source checkout. Private temporary
paths in raw logs identify the original owned runs. The archive uses
manifests to pin the original executable bytes.
""")
    files = sorted(p for p in target.rglob("*") if p.is_file())
    proof = target / "SHA256SUMS"
    proof.write_text("".join(f"{digest(path)}  {path.relative_to(target).as_posix()}\n" for path in files))
    return digest(proof)


def verify_package(root, package_sha):
    seal(root, "SHA256SUMS", package_sha, exact=True)
    report = data(root / REPORT)
    pins_sha = report["gate_pins_sha256"]
    pins = approved_pins(root / "gate-pins.json", pins_sha)
    missing = pending_markers({name: root / "gates" / name for name in MARKERS})
    require(not missing, "Archived exact gates are missing")
    source = source_info(root / "base-source.sha256", root / "source.sha256",
                         raw_log(root / (PATCH_NAME + ".gz")))
    current = root / "focused"
    focused = focus_info(current / "r2", current / "r1", root / "source.sha256")
    native = root / "native"
    require(report == {
        "status": "verified local exact candidate", "gate_pins_sha256": pins_sha, "source": source,
        "predecessor": predecessor(root / "previous"), "original_r1_failure": red_info(current / "r1", current / "r2"),
        "focused_r2": focused, "native_r1_development": data(native / "r1-native-verified.json"),
        "exact_rust_metal": rust_metal([root / "rust" / (name + ".gz") for name in LOGS], root / "rust/metal", pins),
        "exact_native": native_exact(native / "harness", native / "replay", native / "binaries.sha256",
                                     native / "verified.json", pins, native / "r1-native-captures.sha256",
                                     native / "r1-native-verified.json"),
        "stock_macos": stock_info(root / "stock-macos", pins, focused),
    }, "Archived exact local evidence differs")
    return report


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
    parser.add_argument("--stock", type=Path, default=STOCK)
    args = parser.parse_args()
    if args.preflight:
        result = preflight(args.stock)
    elif args.read_gate_pins:
        result = read_gate_pins(args.stock)
    elif args.out:
        result = exact_live(args.stock, args.gate_pins, args.gate_pins_sha)
        target = args.out.resolve()
        checksum = archive(target, args.stock, args.gate_pins, result)
        require(verify_package(target, checksum) == result, "Independent archive readback differs")
        result = {"archive": str(target), "archive_sha256": checksum, "report_sha256": digest(target / REPORT),
                  "status": "verified local exact candidate"}
    else:
        result = verify_package(args.verify_package.resolve(strict=True), args.package_sha)
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit(f"Exact placeholder proof stopped. {error}")
