"""Read exact v0.1.70 source, prior portable stock and real R1 captures offline."""

from argparse import Namespace
import json
from pathlib import Path
import re
from runpy import run_path

from evidence_common import data, digest, import_from, normalized, path_entries, require, seal
from stock_config import (
    DEFAULT_FOCUS, DOC_DELTA, FINAL_COUNT, FINAL_PATCH, FINAL_SOURCE, FINAL_TREE,
    FOCUSED_LAUNCHER, FOCUSED_RECEIPT, FOCUSED_RUNNER, FOCUSED_VERIFIER, PATCH_NAME,
    Q69, Q69_LOCAL, Q69_LOG, Q69_OFFLINE, Q69_REPORT, Q69_SEAL, Q69_SOURCE, Q69_TREE,
    Q69_WINDOW, Q70, validate_reviewed,
)
from verify_boundary_stock import focused_receipt, tools, verify as boundary
from verify_label_color import verify as label
from verify_placeholder_stock import verify as placeholder

ROOT = Path(__file__).resolve().parent
CANDIDATE = Q70 / "exact-candidate"
OBS = run_path(str(ROOT / "observe-multiline.py"))


def rejected(check, phrase):
    try:
        check()
    except (ValueError, RuntimeError) as error:
        require(phrase in str(error), f"Unexpected rejection: {error}")
        return str(error)
    raise ValueError(f"Defective control was accepted: {phrase}")


def prior_stock(negative):
    paths = seal(Q69, "SHA256SUMS", Q69_SEAL, exact=True)
    require(len(paths) == 654 and digest(Q69 / "source.sha256") == Q69_SOURCE
            and len(path_entries(Q69 / "source.sha256")) == 688
            and digest(Q69 / "verified-placeholder-v0169.json") == Q69_REPORT,
            "Wrong prior portable package")
    stock = Q69 / "stock-macos"
    receipt_path = stock / "stock-mac-exact-offline.json"
    log_path = stock / "run/runner.log"
    window_path = stock / "run/headless-window-observation.json"
    require(digest(receipt_path) == Q69_OFFLINE and digest(log_path) == Q69_LOG
            and digest(window_path) == Q69_WINDOW, "Prior portable raw receipts changed")
    receipt, window = data(receipt_path), data(window_path)
    proof = data(Q69 / "verified-placeholder-v0169.json")
    require(receipt["release"]["source_tree"] == Q69_TREE and receipt["release"]["version"] == "0.1.69"
            and receipt["release"]["source_sha256"] == Q69_SOURCE and receipt["total_checks"] == 25851
            and receipt["cases"]["material"] == {"steps": 352, "checks": 25570}
            and len(receipt["text_field_label_motion"]["pngs"]) == 34
            and len(receipt["text_field_placeholder"]["pngs"]) == 58 and receipt["guard_and_window_checks"] == "passed"
            and window["stock_exit"] == 0 and window["visible_owned_windows"] == window["remaining_observed_processes"] == window["problems"] == []
            and proof["stock_macos"]["offline_sha256"] == Q69_OFFLINE and proof["stock_macos"]["checks"] == 25851,
            "Wrong prior actual stock outcome, PNG or process coverage")
    log = log_path.read_text()
    require(OBS["verify_runner_log"](log, 353) == (receipt["cases"], 25851),
            "The inherited five stock suite parser changed")
    artifacts = stock / "run/artifacts"
    metadata = {key: receipt[key] for key in ("browser", "adapter")}
    with import_from(stock):
        older_label = run_path(str(stock / "verify_label_color.py"))["verify"](artifacts, metadata)
    require(normalized(older_label) == receipt["text_field_label_motion"],
            "Previously verified original label bytes differ")
    require(placeholder(artifacts, metadata) == receipt["text_field_placeholder"],
            "Unchanged previous placeholder proof differs")
    negative["prior_stock_as_q70"] = rejected(lambda: OBS["verify_runner_log"](log), "Material steps changed")
    negative["prior_pngs_as_new_motion"] = rejected(lambda: boundary(artifacts, metadata), "missing the actual .70 typed boundary report")
    negative["prior_label_as_migrated"] = rejected(lambda: label(artifacts, metadata), "timed outline ink")
    line = re.search(r"^case material \.\.\. ok \(352 steps, 25570 checks,.*$", log, re.M).group()
    extra = "\n".join(f"  [M{s}] offline parser sentinel" for s in (354, 355, 356))
    synthetic = log.replace(line, extra + "\n" + line.replace("352 steps, 25570", "355 steps, 25573")).replace(
        "25851 checks;", "25854 checks;")
    counts, total = OBS["verify_runner_log"](synthetic)
    require(counts["material"]["steps"] == 355 and total == 25854, "the offline ledger control was not recognized")
    negative["skipped_m355"] = rejected(
        lambda: OBS["verify_runner_log"](synthetic.replace("  [M355] offline parser sentinel\n", "")), "Material steps changed")
    # The portable archive is the source of authority above. An active
    # original observer folder is only compared by byte identity when present.
    identical_to_local = None
    if (Q69_LOCAL / "stock-mac-v0169-passed").is_file():
        identical_to_local = all(digest(Q69_LOCAL / rel) == checksum for rel, checksum in (
            ("stock-mac-exact-offline.json", Q69_OFFLINE),
            ("run/runner.log", Q69_LOG),
            ("run/headless-window-observation.json", Q69_WINDOW),
        ))
        require(identical_to_local, "local predecessor receipts disagree with the portable originals")
    return {"portable_sha256": Q69_SEAL, "portable_entries": len(paths), "source_sha256": Q69_SOURCE,
            "stock_receipt_sha256": Q69_OFFLINE, "checks": 25851, "material_steps": 352,
            "label_pngs": 34, "placeholder_pngs": 58, "local_result_matches_portable": identical_to_local}


def source_and_focused(negative):
    # Valid-format inert pins exercise source scope only. No binary is opened.
    pins = validate_reviewed({
        "source_tree": FINAL_TREE, "source_sha256": FINAL_SOURCE, "source_files": FINAL_COUNT,
        "patch_sha256": FINAL_PATCH, "stock_runner_sha256": "1" * 64, "native_manifest_sha256": "2" * 64,
        "focused_tools_sha256": FOCUSED_LAUNCHER, "focused_verifier_sha256": FOCUSED_VERIFIER,
        "focused_runner_sha256": FOCUSED_RUNNER, "focused_receipt_sha256": FOCUSED_RECEIPT,
        "exact_vs_scoped_source_paths": DOC_DELTA,
    })
    manifest = CANDIDATE / "source.sha256"
    inputs = Namespace(source_manifest=manifest, native_source=CANDIDATE / "source",
                       web_source=CANDIDATE / "source", patch=Q70 / PATCH_NAME)
    files, _ = run_path(str(ROOT / "prepare-pins.py"))["source_inputs"](inputs, pins)
    negative["prior_source"] = rejected(
        lambda: validate_reviewed({**pins, "source_sha256": Q69_SOURCE}), "not the v0.1.70 candidate")
    negative["product_delta_instead_of_docs"] = rejected(
        lambda: validate_reviewed({**pins, "exact_vs_scoped_source_paths": ["e2e/src/material.rs"]}), "Material doc")
    negative["missing_exact_runner_pin"] = rejected(
        lambda: validate_reviewed({key: val for key, val in pins.items() if key != "stock_runner_sha256"}), "Provide every reviewed")
    negative["missing_exact_focused_verifier_pin"] = rejected(
        lambda: validate_reviewed({key: val for key, val in pins.items() if key != "focused_verifier_sha256"}), "Provide every reviewed")
    raw = DEFAULT_FOCUS / "development-r1"
    log = (raw / "runner.log").read_text()
    negative["scoped_as_stock"] = rejected(lambda: OBS["verify_runner_log"](log), "exactly the ordered five")
    canonical_reference = Q70 / "web-development"
    focused = focused_receipt(manifest, pins, canonical_reference)
    meta = {key: focused[key] for key in ("browser", "adapter")}
    actual = boundary(raw / "artifacts", meta)
    label_motion = label(raw / "artifacts", meta)
    hints = placeholder(raw / "artifacts", meta)
    require(actual["pngs"] == focused["pngs"] and actual["decoded_probes"] == 2478
            and len(label_motion["pngs"]) == 34 and hint_hashes(hints) == hint_hashes(focused["inherited_placeholder"]),
            "the eventual full-stock decoder differs on actual R1 captures")
    negative["wrong_chrome"] = rejected(
        lambda: boundary(raw / "artifacts", {**meta, "browser": "Chrome/0.0.0.0"}), "probe count differs")
    return {"source_sha256": FINAL_SOURCE, "tree": FINAL_TREE, "files": len(files), "patch_sha256": FINAL_PATCH,
            "doc_delta": focused["exact_vs_scoped_source_paths"], "focused_receipt_sha256": FOCUSED_RECEIPT,
            "focused_checks": focused["checks"], "focused_steps": focused["steps"], "focused_window_samples": focused["headless_samples"],
            "focused_new_pngs": len(focused["pngs"]), "previous_pngs": 92,
            "stock_ready_claim": False, "actual_new_coverage_quarters": actual["coverage_quarters_at_14_and_44_ms"],
            "actual_early_shallow_quarters": label_motion["outlined"]["early_shallow"]["coverage_quarters"]}


def hint_hashes(receipt):
    return receipt["pngs"]


def main():
    OBS["self_test"]()
    negative = {}
    previous = prior_stock(negative)
    active = source_and_focused(negative)
    print(json.dumps({"scope": "Offline preflight. No v0.1.70 stock process started.",
                      "verified_previous_portable_stock": previous, "independent_current_source_and_focused": active,
                      "negative_controls": negative, "focused_pixel_negative_controls": tools()["selftest"]()},
                     ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
