"""Bind the finished scoped run and decode the live stock contrast screenshots."""

from pathlib import Path
from runpy import run_path

from evidence_common import data, digest, normalized, path_entries, require, seal, verify_entries
from stock_config import (
    CONTRAST_PNGS, DEFAULT_FOCUS, FOCUSED_CHECKS, FOCUSED_LAUNCHER, FOCUSED_RECEIPT,
    FOCUSED_REPORT, FOCUSED_RUNNER, FOCUSED_SAMPLES, FOCUSED_STEPS, FOCUSED_VERIFIER,
    R1_MANIFEST, R2_MANIFEST, R2_SELECTOR, R2_TREE, R2_WEB_MANIFEST, WEB_TEST_DELTA,
)
from verify_boundary_stock import delta

ROOT = Path(__file__).resolve().parent
REGISTRY = "e2e/src/material.rs"
GALLERY_DIR = "web/release/material-gallery"
SCOPE = [f"M{step}" for step in (*range(241, 248), *range(357, 360))]


def tools():
    path = ROOT / "verify-field-contrast.py"
    require(digest(path) == FOCUSED_VERIFIER, "The exact contrast pixel verifier changed")
    return run_path(str(path))


def verify(artifacts, metadata):
    oracle = tools()
    actual = normalized(oracle["verify_captures"](artifacts))
    report = data(artifacts / (oracle["PREFIX"] + ".json"))
    require(actual["browser"] == metadata["browser"] and actual["adapter"] == metadata["adapter"]
            and len(actual["png_sha256"]) == CONTRAST_PNGS
            and report["preference_source"] == "CDP browser media emulation",
            "Stock contrast source, adapter, browser or original PNG inventory differs")
    checksums = actual["png_sha256"]
    for scene in oracle["SCENES"]:
        hashes = [checksums[f"{oracle['PREFIX']}-{scene}-{phase}.png"] for phase in oracle["PHASES"]]
        require(hashes[1] not in (hashes[0], hashes[2]), f"Forced contrast did not change the {scene} PNG")
    return actual


def regular_gallery_origin(stock, origins, hashes):
    """Validate the bundle links and return their original regular-file directory."""
    names = {name for name in hashes if Path(name).parent == Path(GALLERY_DIR)}
    require(len(names) == 4 and names <= origins.keys(), "Wrong release Gallery origin inventory")
    originals = [Path(origins[name]) for name in sorted(names)]
    directories = {path.parent for path in originals}
    require(len(directories) == 1, "Release Gallery bundle mixes different origin directories")
    canonical = directories.pop()
    require(canonical.is_absolute() and canonical.resolve(strict=True) == canonical,
            "Release Gallery origin is outside the canonical regular path")
    gallery = {}
    for name, original in zip(sorted(names), originals):
        link = stock / "bundle" / name
        leaf = Path(name).name
        require(link.is_symlink() and link.resolve(strict=True) == original == canonical / leaf,
                "Release Gallery link does not name its exact canonical target")
        gallery[leaf] = hashes[name]
    verify_entries(canonical, gallery)
    require({path.name for path in canonical.iterdir()} == gallery.keys(), "Wrong canonical Gallery inventory")
    return canonical


def focused_receipt(exact_manifest, pins, canonical):
    focused = DEFAULT_FOCUS
    seal(focused, "launcher.sha256", pins["focused_tools_sha256"])
    require(pins["focused_tools_sha256"] == FOCUSED_LAUNCHER
            and pins["focused_verifier_sha256"] == FOCUSED_VERIFIER, "The approved R2 tools changed")
    require(digest(exact_manifest) == pins["source_sha256"], "Wrong exact source manifest")
    original = focused / "original-source.sha256"
    selector = focused / "focused-source.sha256"
    require(digest(original) == R2_MANIFEST and digest(selector) == R2_SELECTOR,
            "The reviewed R2 source or selector manifest differs")
    exact, dev, scoped = map(path_entries, (exact_manifest, original, selector))
    final_delta = delta(exact, dev)
    require(len(exact) == pins["source_files"] == len(dev) == len(scoped)
            and final_delta == pins["exact_vs_scoped_source_paths"] and delta(dev, scoped) == [REGISTRY],
            "An unreviewed source path changed between R2 and exact stock")
    trusted = focused / "development-r2-verified.json"
    require(digest(trusted) == pins["focused_receipt_sha256"] == FOCUSED_RECEIPT
            and (focused / "runner.sha256").read_text().strip() == pins["focused_runner_sha256"] == FOCUSED_RUNNER,
            "The reviewed R2 runner or actual receipt changed")
    run = (focused / "development-r2").resolve(strict=True)
    result = normalized(tools()["verify"](run))
    invocation = data(run / "invocation.json")
    observed = invocation["inputs"]
    watch = data(run / "headless-window-observation.json")
    ledger = [data(path) for path in (run / "guard-ledger").glob("*.json")]
    require(result == data(trusted) and result["steps"] == SCOPE and len(SCOPE) == FOCUSED_STEPS
            and result["checks"] == FOCUSED_CHECKS and result["chrome_samples"] == FOCUSED_SAMPLES
            and result["visible_owned_windows"] == 0 and watch["remaining_observed_processes"] == []
            and ledger == watch["guard_events"] and invocation["suite_arguments"] == ["material"]
            and invocation["injected_chrome_flags"] == ["--disable-updater-scheduler"]
            and result["report_sha256"] == FOCUSED_REPORT and len(result["png_sha256"]) == CONTRAST_PNGS
            and observed["runner_sha256"] == FOCUSED_RUNNER and observed["product_tree"] == R2_TREE
            and observed["product_source_manifest_sha256"] == R2_MANIFEST
            and observed["focused_source_manifest_sha256"] == R2_SELECTOR
            and observed["registry_delta"] == [REGISTRY] and observed["steps"] == SCOPE
            and observed["source_count"] == len(exact)
            and exact[REGISTRY] == observed["stock_registry_sha256"]
            and scoped[REGISTRY] == observed["focused_registry_sha256"]
            and observed["web_manifest_sha256"] == R2_WEB_MANIFEST
            and observed["web_compatible_test_only_delta"] == WEB_TEST_DELTA,
            "The finished actual R2 evidence scope differs")
    producer = Path(observed["web_producer_manifest"])
    require(digest(producer) == observed["web_producer_manifest_sha256"] == R1_MANIFEST
            and delta(path_entries(producer), dev) == WEB_TEST_DELTA, "Wrong R1 Web producer source delta")
    web_manifest = Path(observed["original_web_manifest"])
    gallery = observed["web_files"]
    require(digest(web_manifest) == R2_WEB_MANIFEST
            and path_entries(web_manifest, allow_dot_prefix=True) == gallery and len(gallery) == 4,
            "Wrong scoped Gallery asset manifest")
    verify_entries(canonical, gallery)
    require({path.name for path in canonical.iterdir()} == gallery.keys(), "Wrong canonical Gallery inventory")
    return {**result, "steps": FOCUSED_STEPS, "step_ids": SCOPE, "headless_samples": FOCUSED_SAMPLES,
            "visible_windows": 0, "remaining_owned_processes": [], "exact_source_sha256": pins["source_sha256"],
            "exact_vs_scoped_source_paths": final_delta, "focused_receipt_sha256": FOCUSED_RECEIPT,
            "gallery_files": gallery, "canonical_gallery_files": gallery, "canonical_gallery_match": True,
            "inputs": observed}
