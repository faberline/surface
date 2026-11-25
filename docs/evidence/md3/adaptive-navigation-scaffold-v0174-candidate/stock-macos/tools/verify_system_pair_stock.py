"""Bind the finished scoped q72 run and decode the actual stock Field-pair captures."""

from pathlib import Path
from runpy import run_path

from evidence_common import data, digest, normalized, path_entries, require, seal, verify_entries
from stock_config import (
    CONTRAST_PNGS, DEFAULT_FOCUS, FOCUSED_CHECKS, FOCUSED_LAUNCHER, FOCUSED_RECEIPT,
    FOCUSED_RUNNER, FOCUSED_SAMPLES, FOCUSED_STEPS, FOCUSED_VERIFIER, PAIR_PNGS,
    R4_MANIFEST, R4_TREE, R4_WEB_MANIFEST, R8_MANIFEST, R8_SELECTOR, R8_TREE, WEB_TEST_DELTA,
)
from verify_boundary_stock import delta

ROOT = Path(__file__).resolve().parent
REGISTRY = "e2e/src/material.rs"
SCOPED = "development-r8"
SCOPE = [f"M{step}" for step in (*range(241, 248), *range(357, 363))]


def tools():
    path = ROOT / "verify-system-pair.py"
    require(digest(path) == FOCUSED_VERIFIER, "The exact system-pair pixel verifier changed")
    return run_path(str(path))


def verify(artifacts, metadata):
    actual = normalized(tools()["verify_captures"](artifacts))
    require(actual["browser"] == metadata["browser"] and actual["adapter"] == metadata["adapter"]
            and len(actual["png_sha256"]) == PAIR_PNGS,
            "Stock system-pair browser, adapter or original PNG inventory differs")
    return actual


def focused_receipt(exact_manifest, pins, canonical):
    focus = DEFAULT_FOCUS
    seal(focus, "launcher.sha256", pins["focused_tools_sha256"])
    require(pins["focused_tools_sha256"] == FOCUSED_LAUNCHER
            and pins["focused_verifier_sha256"] == FOCUSED_VERIFIER
            and digest(focus / "verify-system-pair.py") == FOCUSED_VERIFIER, "Wrong exact q72 focused tools")
    require(digest(exact_manifest) == pins["source_sha256"], "Wrong exact source manifest")
    original = focus / "original-source.sha256"
    selected = focus / "focused-source.sha256"
    require(digest(original) == R8_MANIFEST and digest(selected) == R8_SELECTOR, "Wrong expected R8 product or selector")
    exact, dev, scoped = map(path_entries, (exact_manifest, original, selected))
    final_delta = delta(exact, dev)
    require(len(exact) == pins["source_files"] == len(dev) == len(scoped)
            and exact.keys() == dev.keys() == scoped.keys()
            and final_delta == pins["exact_vs_scoped_source_paths"] and delta(dev, scoped) == [REGISTRY],
            "An unreviewed source path changed between R8 and the exact stock build")
    trusted = focus / (SCOPED + "-verified.json")
    require(digest(trusted) == pins["focused_receipt_sha256"] == FOCUSED_RECEIPT
            and (focus / "runner.sha256").read_text().strip() == pins["focused_runner_sha256"] == FOCUSED_RUNNER,
            "Wrong reviewed R8 runner or completed receipt")
    run = (focus / SCOPED).resolve(strict=True)
    result = normalized(tools()["verify_run"](run))
    invocation = data(run / "invocation.json")
    observed = invocation["inputs"]
    require(result == data(trusted) and result["steps"] == SCOPE and len(SCOPE) == FOCUSED_STEPS
            and result["checks"] == FOCUSED_CHECKS and result["chrome_samples"] == FOCUSED_SAMPLES
            and result["visible_owned_windows"] == 0 and len(result["png_sha256"]) == PAIR_PNGS
            and len(result["retained_disabled_contrast"]["png_sha256"]) == CONTRAST_PNGS
            and observed["runner_sha256"] == FOCUSED_RUNNER and observed["product_tree"] == R8_TREE
            and observed["source_count"] == len(exact) and observed["product_source_manifest_sha256"] == R8_MANIFEST
            and observed["focused_source_manifest_sha256"] == R8_SELECTOR
            and observed["registry_delta"] == [REGISTRY] and observed["steps"] == SCOPE
            and observed["stock_registry_sha256"] == exact[REGISTRY]
            and observed["focused_registry_sha256"] == scoped[REGISTRY]
            and observed["web_manifest_sha256"] == R4_WEB_MANIFEST and observed["web_producer_tree"] == R4_TREE
            and observed["web_compatible_test_only_delta"] == WEB_TEST_DELTA, "The finished actual R8 proof scope changed")
    producer = Path(observed["web_producer_manifest"])
    require(digest(producer) == observed["web_producer_manifest_sha256"] == R4_MANIFEST
            and delta(path_entries(producer), dev) == WEB_TEST_DELTA, "Wrong tested R4 Web producer delta")
    web = Path(observed["original_web_manifest"])
    gallery = observed["web_files"]
    require(digest(web) == R4_WEB_MANIFEST and len(gallery) == 4 and path_entries(web, allow_dot_prefix=True) == gallery,
            "Wrong tested R4 Gallery assets")
    verify_entries(canonical, gallery)
    require({path.name for path in canonical.iterdir()} == gallery.keys(), "Wrong canonical Gallery inventory")
    return {**result, "steps": FOCUSED_STEPS, "step_ids": SCOPE, "headless_samples": result["chrome_samples"],
            "visible_windows": 0, "remaining_owned_processes": [], "exact_source_sha256": pins["source_sha256"],
            "exact_vs_scoped_source_paths": final_delta, "focused_receipt_sha256": FOCUSED_RECEIPT,
            "gallery_files": gallery, "canonical_gallery_files": gallery, "canonical_gallery_match": True,
            "inputs": observed}
