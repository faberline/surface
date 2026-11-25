"""Decode actual q73 stock Navigation originals and bind the separate R3 proof."""

from pathlib import Path
from runpy import run_path

from evidence_common import data, digest, normalized, path_entries, require, seal, verify_entries
from stock_config import (
    NAV_FOCUS, NAV_LAUNCHER, NAV_VERIFIER, NAV_VERIFIER_MANIFEST, NAV_RECEIPT, NAV_RUNNER,
    NAV_SCOPED, NAV_PRODUCT, NAV_TREE, NAV_WEB, NAV_STEPS, NAV_CHECKS, NAV_SAMPLES,
)
from verify_boundary_stock import delta

REGISTRY = "e2e/src/material.rs"


def tools():
    seal(NAV_FOCUS, "launcher.sha256", NAV_LAUNCHER)
    seal(NAV_FOCUS, "verifier.sha256", NAV_VERIFIER_MANIFEST)
    path = NAV_FOCUS / "verify-navigation-pair.py"
    require(digest(path) == NAV_VERIFIER, "Wrong R3 Navigation pixel verifier")
    return run_path(str(path))


def verify(artifacts, metadata):
    toolkit = tools()
    old = toolkit["OLD"]
    compact_path = artifacts / toolkit["COMPACT"]
    horizontal_path = artifacts / f'{toolkit["STEM"]}.json'
    require(compact_path.is_file() and horizontal_path.is_file(),
            "Both original stock Navigation reports are required")
    compact = data(compact_path)
    browser, adapter = old["metadata"](compact)
    require(browser == metadata["browser"] and adapter == metadata["adapter"]
            and compact["errors"] == [] and not {"failure", "state_at_failure"} & compact.keys()
            and compact["reduced_motion"] is True, "Stock Navigation browser, adapter or page errors differ")
    previous = old["validate_navigation"](compact, artifacts)
    horizontal = toolkit["medium"](artifacts, compact)
    require(len(previous["pngs"]) == 4 and len(horizontal["pngs"]) == 3,
            "Wrong original stock Navigation PNG inventory")
    return normalized({"browser": browser, "adapter": adapter, "page_errors": 0,
                       "compact": {"report_sha256": digest(compact_path), **previous},
                       "horizontal": horizontal})


def focused_receipt(exact_manifest, pins, canonical):
    toolkit = tools()
    original = NAV_FOCUS / "original-source.sha256"
    selector = NAV_FOCUS / "focused-source.sha256"
    require(digest(exact_manifest) == pins["source_sha256"]
            and digest(original) == NAV_PRODUCT and digest(selector) == NAV_SCOPED,
            "Wrong exact source or R3 focused manifests")
    exact, source, selected = map(path_entries, (exact_manifest, original, selector))
    final_delta = delta(exact, source)
    require(exact.keys() == source.keys() == selected.keys()
            and len(exact) == pins["source_files"]
            and final_delta == pins["exact_vs_scoped_source_paths"] and delta(source, selected) == [REGISTRY],
            "An unreviewed source path differs between R3 and the stock build")
    receipt = NAV_FOCUS / "verified-navigation-r3.json"
    require(digest(receipt) == pins["focused_receipt_sha256"] == NAV_RECEIPT,
            "Wrong R3 actual scoped receipt")
    result = normalized(toolkit["verify"](NAV_FOCUS / "development-r3"))
    inputs = result["inputs"]
    headless = result["headless"]
    require(result == data(receipt) and result["steps"] == NAV_STEPS and result["checks"] == NAV_CHECKS
            and headless["chrome_samples"] == NAV_SAMPLES and headless["visible_windows"] == 0
            and headless["remaining_owned_processes"] == 0
            and inputs["runner_sha256"] == pins["focused_runner_sha256"] == NAV_RUNNER
            and inputs["product_tree"] == NAV_TREE and inputs["product_source_manifest_sha256"] == NAV_PRODUCT
            and inputs["stock_registry_sha256"] == exact[REGISTRY]
            and inputs["focused_registry_sha256"] == selected[REGISTRY]
            and inputs["web_manifest_sha256"] == NAV_WEB
            and len(result["compact"]["pngs"]) == 4 and len(result["horizontal"]["pngs"]) == 3,
            "Wrong checked R3 scope or Navigation artifacts")
    gallery = inputs["web_files"]
    verify_entries(canonical, gallery)
    require({path.name for path in canonical.iterdir()} == gallery.keys() and len(gallery) == 4,
            "Wrong exact canonical Gallery originals")
    return {**result, "headless_samples": headless["chrome_samples"], "visible_windows": 0,
            "remaining_owned_processes": [], "exact_source_sha256": pins["source_sha256"],
            "exact_vs_scoped_source_paths": final_delta, "focused_receipt_sha256": NAV_RECEIPT,
            "gallery_files": gallery, "canonical_gallery_files": gallery, "canonical_gallery_match": True}
