"""Recheck final source, original .68 stock, and actual .69 R2 files offline."""

from argparse import Namespace
import json
from pathlib import Path
import re
from runpy import run_path

from evidence_common import data, digest, path_entries, require, seal
from stock_config import (DEFAULT_FOCUS, DEFAULT_FOCUS_TOOLS, FINAL_PATCH, FINAL_SOURCE, FINAL_TREE,
                          FOCUSED_RECEIPT, FOCUSED_RUNNER, Q68, Q68_OFFLINE, Q68_REPORT, Q68_SEAL,
                          Q68_SOURCE, Q68_TREE, validate_reviewed)
from verify_placeholder_stock import CHECK, delta, focused_receipt, verify

ROOT = Path(__file__).resolve().parent
Q69 = Path("/tmp/surface-md3-text-field-placeholder-v0169.bc9QKVhY")
CANDIDATE = Q69 / "exact-candidate"
OBS = run_path(str(ROOT / "observe-multiline.py"))


def rejected(check, phrase):
    try:
        check()
    except (ValueError, RuntimeError) as error:
        require(phrase in str(error), f"Unexpected rejection: {error}")
        return str(error)
    raise ValueError(f"Defective control was accepted: {phrase}")


def prior_stock(negatives):
    paths = seal(Q68, "SHA256SUMS", Q68_SEAL, exact=True)
    require(len(paths) == 400 and digest(Q68 / "source.sha256") == Q68_SOURCE
            and len(path_entries(Q68 / "source.sha256")) == 685
            and digest(Q68 / "verified-label-color-v0168.json") == Q68_REPORT,
            "Wrong prior release archive")
    stock = Q68 / "stock-macos"
    old_receipt = stock / "stock-mac-exact-offline.json"
    require(digest(old_receipt) == Q68_OFFLINE, "Prior exact stock receipt changed")
    receipt = data(old_receipt)
    watch = data(stock / "run/headless-window-observation.json")
    require(receipt["release"]["source_tree"] == Q68_TREE and receipt["release"]["version"] == "0.1.68"
            and receipt["total_checks"] == 25276 and len(receipt["text_field_label_motion"]["pngs"]) == 34
            and receipt["guard_and_window_checks"] == "passed"
            and watch["visible_owned_windows"] == watch["remaining_observed_processes"] == watch["problems"] == [],
            "Prior exact stock result, PNG or window scope changed")
    log = (stock / "run/runner.log").read_text()
    require(OBS["verify_runner_log"](log, 350) == (receipt["cases"], 25276),
            "The inherited five stock suite parser changed")
    negatives["prior_stock_as_q69"] = rejected(lambda: OBS["verify_runner_log"](log), "Material steps changed")
    retained = run_path(str(ROOT / "verify_label_color.py"))["verify"](
        stock / "run/artifacts", {key: receipt[key] for key in ("browser", "adapter")})
    require(retained["pngs"] == receipt["text_field_label_motion"]["pngs"], "Retained label originals changed")
    # A synthetic ledger proves only the new step sequence and aggregate parser.
    line = re.search(r"^case material \.\.\. ok \(349 steps, 24995 checks,.*$", log, re.M).group()
    extra = "\n".join(f"  [M{step}] offline parser sentinel" for step in (351, 352, 353))
    control = log.replace(line, extra + "\n" + line.replace("349 steps, 24995", "352 steps, 24998")).replace(
        "25276 checks;", "25279 checks;")
    counts, total = OBS["verify_runner_log"](control)
    require(counts["material"]["steps"] == 352 and total == 25279, "Final ledger parser rejected its control")
    negatives["skipped_m352"] = rejected(
        lambda: OBS["verify_runner_log"](control.replace("  [M352] offline parser sentinel\n", "")),
        "Material steps changed")
    return {"archive_sha256": Q68_SEAL, "source_sha256": Q68_SOURCE, "stock_receipt_sha256": Q68_OFFLINE,
            "checks": 25276, "material_steps": 349, "label_pngs": 34, "guard_and_window_checks": "passed"}


def exact_source_and_focus(negatives):
    manifest = CANDIDATE / "source.sha256"
    delta_paths = delta(path_entries(manifest), path_entries(DEFAULT_FOCUS / "original-source.sha256"))
    # Unbuilt binaries are never used in this offline control.
    pins = validate_reviewed({
        "source_tree": FINAL_TREE, "source_sha256": FINAL_SOURCE, "source_files": 688,
        "patch_sha256": FINAL_PATCH, "stock_runner_sha256": "1" * 64, "native_manifest_sha256": "2" * 64,
        "focused_tools_sha256": DEFAULT_FOCUS_TOOLS, "focused_runner_sha256": FOCUSED_RUNNER,
        "focused_receipt_sha256": FOCUSED_RECEIPT, "exact_vs_scoped_source_paths": delta_paths,
    })
    require(delta_paths == ["docs/material-coverage.json"], "The real R2 delta exceeds the known doc path")
    inputs = Namespace(source_manifest=manifest, native_source=CANDIDATE / "source",
                       web_source=CANDIDATE / "source", patch=Q69 / "text-field-placeholder-v0169.patch")
    files, _ = run_path(str(ROOT / "prepare-pins.py"))["source_inputs"](inputs, pins)
    negatives["prior_source"] = rejected(
        lambda: validate_reviewed({**pins, "source_sha256": Q68_SOURCE}), "not the v0.1.69 candidate")
    negatives["product_delta_allowed"] = rejected(
        lambda: validate_reviewed({**pins, "exact_vs_scoped_source_paths": ["e2e/src/material.rs"]}), "Material docs")
    negatives["missing_stock_pin"] = rejected(
        lambda: validate_reviewed({key: value for key, value in pins.items() if key != "stock_runner_sha256"}),
        "Provide every reviewed")
    negatives["reviewed_doc_delta_missing"] = rejected(
        lambda: focused_receipt(manifest, {**pins, "exact_vs_scoped_source_paths": []}), "unreviewed source path")
    raw = DEFAULT_FOCUS / "development-r2-authorized"
    focused_log = (raw / "runner.log").read_text()
    negatives["scoped_as_stock"] = rejected(
        lambda: OBS["verify_runner_log"](focused_log), "exactly the ordered five")
    focused = focused_receipt(manifest, pins)
    metadata = {key: focused[key] for key in ("browser", "adapter")}
    physical = verify(raw / "artifacts", metadata)
    require(physical["pngs"] == focused["pngs"], "Stock pixel oracle differs on actual focused originals")
    negatives["wrong_actual_metadata"] = rejected(
        lambda: verify(raw / "artifacts", {**metadata, "browser": "Chrome/0.0.0.0"}), "or originals differ")
    return {"source_sha256": FINAL_SOURCE, "source_tree": FINAL_TREE, "source_files": len(files),
            "patch_sha256": FINAL_PATCH, "receipt_sha256": FOCUSED_RECEIPT, "scoped_runner_sha256": FOCUSED_RUNNER,
            "docs_only_delta": delta_paths, "scoped_steps": focused["steps"], "scoped_checks": focused["checks"],
            "scoped_samples": focused["headless_samples"], "scoped_windows": focused["visible_windows"],
            "scoped_survivors": focused["remaining_owned_processes"], "new_pngs": len(focused["pngs"]),
            "retained_pngs": len(focused["retained_label_motion"]["captures"])}


def main():
    OBS["self_test"]()
    negatives = {}
    prior = prior_stock(negatives)
    current = exact_source_and_focus(negatives)
    oracle = run_path(str(CHECK))["self_test"]()
    print(json.dumps({"scope": "Offline preflight. No q69 stock runner launched.",
                      "prior_actual_stock": prior, "exact_source_and_actual_scoped": current,
                      "pixel_controls": oracle, "negative_controls": negatives},
                     ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
