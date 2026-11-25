"""Replay both q74 Navigation scroll proofs without a running browser."""

from math import isfinite
from pathlib import Path
import re
from runpy import run_path

from evidence_common import clean_windows, data, digest, normalized, path_entries, require, seal, verify_entries, verify_ledgers
from stock_config import (
    ADAPTIVE_CASE, ADAPTIVE_CHECKS, ADAPTIVE_DELTA, ADAPTIVE_FOCUS, ADAPTIVE_LAUNCHER, ADAPTIVE_PRODUCT,
    ADAPTIVE_RECEIPT, ADAPTIVE_RUNNER, ADAPTIVE_SAMPLES, ADAPTIVE_SCOPED, ADAPTIVE_STAGE,
    ADAPTIVE_STEPS, ADAPTIVE_TREE, ADAPTIVE_VERIFIER, ADAPTIVE_WEB, ADAPTIVE_WEB_SOURCE,
)
from verify_boundary_stock import delta

ROOT = Path(__file__).resolve().parent
REGISTRY = "e2e/src/material.rs"
STEM = "material-adaptive-navigation"
PLACEMENT = {
    "compact": ((599, 620), [0, 556, 599, 64], [242, 236, 244], [232, 222, 248]),
    "medium": ((600, 620), [0, 556, 600, 64], [242, 236, 244], [232, 222, 248]),
    "rail": ((840, 620), [0, 0, 96, 620], [253, 247, 255], [232, 222, 248]),
    "dark-rtl-short-rail": ((840, 300), [744, 0, 96, 300], [20, 18, 24], [74, 68, 88]),
}
DRAFT = {"value": "draft", "runtime": [5, 3], "browser": ["draft", 3, 5, "backward"]}
SAVED, UPDATES, SETTINGS = ["adaptive-navigation-demo-" + suffix for suffix in ("saved", "updates", "settings")]


def tools():
    module = ROOT / "verify-adaptive-focused-original.py"
    require(digest(module) == ADAPTIVE_VERIFIER, "Wrong known adaptive Navigation decoder")
    return run_path(str(module))


def rect(r):
    require(isinstance(r, list) and len(r) == 4 and all(type(x) in (float, int) and isfinite(x) for x in r)
            and min(r[2:]) > 0, "Invalid measured rectangle")
    return r


def inside(r, viewport):
    x, y, w, h = rect(r)
    left, top, span, height = rect(viewport)
    return left <= x and top <= y and x + w <= left + span and y + h <= top + height


def verify(artifacts, metadata):
    report = artifacts / f"{STEM}.json"
    proof = data(report)
    bars, rail, dark = proof["bars"], proof["rail"], proof["dark"]
    require(proof["draft"] == DRAFT and bars["current"] == bars["focus"] == SAVED
            and rail["current"] == UPDATES and rail["focus"] == dark["focus"] == SETTINGS,
            "Current page, retained focus or backward selection differs")
    require(rail["page"] == [96, 0, 744, 620] and dark["page"] == [0, 0, 744, 300]
            and inside(dark["editor"], dark["page"]), "The separate content viewport or editor differs")
    before, revealed = rect(dark["before"]), rect(dark["revealed"])
    offset = dark["rail_scroll"]
    require(dark["clip"] == dark["paint"]["bounds"] and offset == [0,100] and dark["page_scroll"] == 0
            and not inside(before, dark["clip"]) and inside(revealed, dark["clip"])
            and before[:1] + before[2:] == revealed[:1] + revealed[2:]
            and before[1] - revealed[1] == offset[1] - offset[0],
            "The focused row did not move into the rail clip by its own real scroll offset")
    records = {"compact": bars["compact"], "medium": bars["medium"], "rail": rail["paint"],
               "dark-rtl-short-rail": dark["paint"]}
    filenames = {f"{STEM}-{name}.png" for name in records}
    require({path.name for path in artifacts.glob(f"{STEM}-*.png")} == filenames, "Wrong adaptive Navigation image inventory")
    screenshots = {}
    checker = tools()["verify_frame"]
    for tag, record in records.items():
        path = artifacts / f"{STEM}-{tag}.png"
        viewport, nav, surface, selected = PLACEMENT[tag]
        cell = rect(record["current_bounds"])
        require(record["bounds"] == nav and record["surface"] == surface and record["selected"] == selected
                and inside(cell, nav) and min(cell[2:]) >= 48, "Wrong adaptive Navigation token or selected-cell geometry")
        if tag == "medium":
            require(cell[2:] == [112,64] and cell[0] + cell[2] / 2 == 300, "Wrong centred Medium cell")
        elif "rail" in tag:
            require(cell[0] == nav[0] and cell[2] == 96, "Wrong logical-start rail cell")
        screenshots[path.name] = checker(path, record, viewport)
    return normalized({"report_sha256": digest(report), "pngs": screenshots,
                       "draft": proof["draft"], "selected": [bars["current"], rail["current"]],
                       "rail_scroll": offset, "page_scroll": dark["page_scroll"], "scroll_witness": dark,
                       **metadata})


def scoped(exact_manifest, canonical=None):
    seal(ADAPTIVE_FOCUS, "launcher.sha256", ADAPTIVE_LAUNCHER)
    runner = ADAPTIVE_FOCUS / "pins/surface-e2e"
    require(digest(runner) == ADAPTIVE_RUNNER and (ADAPTIVE_FOCUS / "runner.sha256").read_text().strip() == ADAPTIVE_RUNNER,
            "Wrong actual R4 focused runner")
    source = ADAPTIVE_STAGE / "development-r4/source.sha256"
    focused = ADAPTIVE_FOCUS / "focused-source.sha256"
    producer = ADAPTIVE_STAGE / "development-r2/source.sha256"
    require([digest(path) for path in (source, focused, producer)]
            == [ADAPTIVE_PRODUCT, ADAPTIVE_SCOPED, ADAPTIVE_WEB_SOURCE], "Wrong R4 producer, selector or tested source")
    exact, dev, selected, web = [path_entries(path) for path in (exact_manifest, source, focused, producer)]
    require(len(exact) == 696 and exact.keys() == dev.keys() == selected.keys() == web.keys()
            and delta(exact, dev) == ADAPTIVE_DELTA and delta(dev, selected) == [REGISTRY]
            and delta(web, dev) == [ADAPTIVE_CASE], "Wrong exact build scope versus actual R4")
    run = ADAPTIVE_FOCUS / "development-r4"
    typed = ADAPTIVE_FOCUS / "verified-adaptive-navigation-r4.json"
    require(digest(typed) == ADAPTIVE_RECEIPT, "Wrong original R4 focused receipt")
    results = normalized(tools()["verify"](run))
    invocation = data(run / "invocation.json")
    pins = invocation["inputs"]
    prep = run_path(str(ADAPTIVE_FOCUS / "prepare-focused.py"))
    bound = {**prep["check_scope"](), **prep["gallery"](ADAPTIVE_WEB), "runner_sha256": ADAPTIVE_RUNNER}
    require(pins == bound, "The original R4 source files, modes or Gallery asset seal changed")
    require(results == data(typed) and results["checks"] == ADAPTIVE_CHECKS and results["window_samples"] == ADAPTIVE_SAMPLES
            and pins == data(run / "inputs-after.json")
            and invocation["suite_arguments"] == ["material"] and invocation["injected_chrome_flags"] == ["--disable-updater-scheduler"]
            and pins["steps"] == ADAPTIVE_STEPS and pins["product_tree"] == ADAPTIVE_TREE
            and pins["product_source_manifest_sha256"] == ADAPTIVE_PRODUCT
            and pins["focused_source_manifest_sha256"] == ADAPTIVE_SCOPED and pins["registry_delta"] == [REGISTRY]
            and pins["web_producer_manifest_sha256"] == ADAPTIVE_WEB_SOURCE and pins["web_test_only_delta"] == [ADAPTIVE_CASE]
            and pins["web_manifest_sha256"] == ADAPTIVE_WEB and pins["runner_sha256"] == ADAPTIVE_RUNNER,
            "Wrong trusted R4 run or pinned inputs")
    watch = data(run / "headless-window-observation.json")
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary"})
    require(watch["guard_events"][0]["argv"].count("--disable-updater-scheduler") == 1
            and data(run / "owned-shutdown.json")["after_cleanup"] == [], "Incomplete R4 cleanup evidence")
    browser, = re.findall(r"^browser: (\S+)", (run / "runner.log").read_text(), re.M)
    captures = verify(run / "artifacts", {"browser": browser})
    require(results["pictures"] == captures["pngs"], "Independent focused pixel replay differs")
    gallery = pins["web_files"]
    manifest = ADAPTIVE_STAGE / "web-development.sha256"
    require(digest(manifest) == ADAPTIVE_WEB and path_entries(manifest, allow_dot_prefix=True) == gallery,
            "Wrong R4 Gallery file manifest")
    verify_entries(ADAPTIVE_STAGE / "web-development/material-gallery", gallery)
    canonical_match = None
    if canonical is not None:
        verify_entries(canonical, gallery)
        require({path.name for path in canonical.iterdir()} == gallery.keys(), "Wrong canonical Gallery inventory")
        canonical_match = True
    return {**results, "steps": ADAPTIVE_STEPS, "headless_samples": results["window_samples"],
            "remaining_owned_processes": [], "inputs": pins,
            "exact_source_sha256": digest(exact_manifest), "exact_vs_scoped_source_paths": ADAPTIVE_DELTA,
            "focused_receipt_sha256": ADAPTIVE_RECEIPT, "gallery_files": gallery,
            "canonical_gallery_files": gallery if canonical_match else None, "canonical_gallery_match": canonical_match}


def focused_receipt(exact_manifest, pins, canonical):
    require(digest(exact_manifest) == pins["source_sha256"], "Wrong exact source manifest")
    return scoped(exact_manifest, canonical)
