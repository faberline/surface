"""Bind the actual .70 R1 result and replay the 33 new stock PNG originals."""

from pathlib import Path
from runpy import run_path

from evidence_common import data, digest, normalized, path_entries, require, seal, verify_entries
from stock_config import (
    BOUNDARY_PNGS, DEFAULT_FOCUS, FOCUSED_CHECKS, FOCUSED_LAUNCHER, FOCUSED_SAMPLES,
    FOCUSED_STEPS, FOCUSED_VERIFIER, LAST_MATERIAL, PLACEHOLDER_PNGS, R1_MANIFEST,
    R1_SELECTOR, R1_WEB_MANIFEST, RETAINED_PNGS,
)

ROOT = Path(__file__).resolve().parent
REGISTRY = "e2e/src/material.rs"
SCOPE = [f"M{step}" for step in range(325, LAST_MATERIAL + 1)]


def tools():
    seal(DEFAULT_FOCUS, "verifier.sha256", FOCUSED_VERIFIER)
    return run_path(str(DEFAULT_FOCUS / "verify-boundary.py"))


def verify(artifacts, metadata):
    actual = normalized(tools()["verify"](artifacts))
    require(actual["browser"] == metadata["browser"] and actual["adapter"] == metadata["adapter"]
            and len(actual["pngs"]) == BOUNDARY_PNGS and actual["decoded_probes"] == 2478,
            "Stock boundary browser, adapter, original PNG or probe count differs")
    return actual


def delta(left, right):
    return sorted(path for path in left.keys() | right.keys() if left.get(path) != right.get(path))


def focused_receipt(exact_manifest, pins, canonical=None):
    focused = DEFAULT_FOCUS
    seal(focused, "launcher.sha256", pins["focused_tools_sha256"])
    require(pins["focused_tools_sha256"] == FOCUSED_LAUNCHER
            and pins["focused_verifier_sha256"] == FOCUSED_VERIFIER,
            "The approved R1 tools changed")
    verify_focus = tools()["verify_run"]
    require(digest(exact_manifest) == pins["source_sha256"], "Wrong exact source manifest")
    original = focused / "original-source.sha256"
    selected = focused / "focused-source.sha256"
    require(digest(original) == R1_MANIFEST and digest(selected) == R1_SELECTOR,
            "The reviewed R1 source or selector manifest differs")
    exact, dev, scoped = map(path_entries, (exact_manifest, original, selected))
    final_delta = delta(exact, dev)
    require(len(exact) == pins["source_files"] == len(dev) == len(scoped)
            and final_delta == pins["exact_vs_scoped_source_paths"]
            and delta(dev, scoped) == [REGISTRY],
            "An unreviewed source path changed between R1 and exact stock")
    trusted = focused / "development-r1-verified.json"
    require(digest(trusted) == pins["focused_receipt_sha256"]
            and (focused / "runner.sha256").read_text().strip() == pins["focused_runner_sha256"],
            "The reviewed R1 runner or actual receipt changed")
    result = normalized(verify_focus((focused / "development-r1").resolve(strict=True)))
    observed = result["inputs"]
    require(result == data(trusted) and result["steps"] == SCOPE and len(SCOPE) == FOCUSED_STEPS
            and result["checks"] == FOCUSED_CHECKS and result["headless_samples"] == FOCUSED_SAMPLES
            and result["visible_windows"] == 0 and result["remaining_owned_processes"] == []
            and observed["runner_sha256"] == pins["focused_runner_sha256"]
            and observed["web_manifest_sha256"] == R1_WEB_MANIFEST
            and observed["web_compatible_test_only_delta"] == []
            and len(result["inherited_label_motion"]["captures"]) == RETAINED_PNGS
            and len(result["inherited_placeholder"]["pngs"]) == PLACEHOLDER_PNGS
            and len(result["pngs"]) == BOUNDARY_PNGS, "The finished actual R1 evidence scope differs")
    gallery = observed["web_files"]
    require(len(gallery) == 4, "Wrong scoped Gallery inventory")
    if canonical is not None:
        verify_entries(canonical, gallery)
        require({path.name for path in canonical.iterdir()} == gallery.keys(),
                "Wrong canonical Gallery inventory")
    return {**result, "steps": FOCUSED_STEPS, "step_ids": SCOPE,
            "exact_source_sha256": pins["source_sha256"], "exact_vs_scoped_source_paths": final_delta,
            "focused_receipt_sha256": pins["focused_receipt_sha256"], "gallery_files": gallery,
            "canonical_gallery_files": None if canonical is None else gallery,
            "canonical_gallery_match": None if canonical is None else True}
