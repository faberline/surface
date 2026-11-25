"""Exercise the headless guards and decode the real R2 capture set offline."""

from argparse import Namespace
import json
from pathlib import Path
import re
from runpy import run_path
import subprocess
from tempfile import TemporaryDirectory

from evidence_common import data, digest, path_entries, require, seal, source_tree, verify_entries
import stock_config as config
from verify_boundary_stock import verify as boundary
from verify_disabled_contrast import focused_receipt, regular_gallery_origin, verify as contrast
from verify_label_color import verify as label
from verify_placeholder_stock import verify as placeholder

ROOT = Path(__file__).resolve().parent
OBS = run_path(str(ROOT / "observe-multiline.py"))


def rejected(check, phrase):
    try:
        check()
    except (FileNotFoundError, RuntimeError, ValueError) as error:
        require(phrase in str(error), f"Unexpected rejection: {error}")
        return phrase
    raise ValueError(f"Defective control was accepted: {phrase}")


def stock_predecessor(negative):
    previous = config.Q70
    entries = seal(previous, "SHA256SUMS", config.Q70_SEAL, exact=True)
    receipt_path = previous / "stock-macos/stock-mac-exact-offline.json"
    run = previous / "stock-macos/run"
    log_path = run / "runner.log"
    window_path = run / "headless-window-observation.json"
    report = previous / "verified-boundary-v0170.json"
    require(len(entries) == 613 and digest(previous / "source.sha256") == config.Q70_SOURCE
            and len(path_entries(previous / "source.sha256")) == 689 and digest(report) == config.Q70_REPORT
            and digest(receipt_path) == config.Q70_OFFLINE and digest(log_path) == config.Q70_LOG
            and digest(window_path) == config.Q70_WINDOW, "Wrong .70 portable receipts")
    seal(previous / "stock-macos/offline-repair", "SHA256SUMS", config.Q70_REPAIR)
    receipt, window, proof = map(data, (receipt_path, window_path, report))
    require(receipt["release"]["source_tree"] == proof["source"]["tree"] == config.Q70_TREE
            and receipt["release"]["version"] == "0.1.70" and receipt["release"]["source_sha256"] == config.Q70_SOURCE
            and receipt["total_checks"] == proof["stock"]["total_checks"] == 28706
            and receipt["cases"]["material"] == {"steps": 355, "checks": 28425}
            and window["stock_exit"] == 0 and window["visible_owned_windows"] == []
            and window["remaining_observed_processes"] == [] and window["problems"] == []
            and receipt["guard_and_window_checks"] == "passed", "Wrong actual .70 predecessor stock scope")
    log = log_path.read_text()
    require(OBS["verify_runner_log"](log, 356) == (receipt["cases"], 28706), "The inherited suite parser changed")
    artifacts = run / "artifacts"
    metadata = {key: receipt[key] for key in ("browser", "adapter")}
    require(boundary(artifacts, metadata) == receipt["text_field_boundary_motion"]
            and label(artifacts, metadata) == receipt["text_field_label_motion"]
            and placeholder(artifacts, metadata) == receipt["text_field_placeholder"], "A retained pixel oracle changed")
    negative["old_stock_steps"] = rejected(lambda: OBS["verify_runner_log"](log), "Material steps changed")
    negative["old_without_contrast"] = rejected(lambda: contrast(artifacts, metadata), "material-text-field-disabled-contrast.json")
    line = re.search(r"^case material \.\.\. ok \(355 steps, 28425 checks,.*$", log, re.M).group()
    extra = "\n".join(f"  [M{step}] offline parser sentinel" for step in (357, 358, 359))
    synthetic = log.replace(line, extra + "\n" + line.replace("355 steps, 28425", "358 steps, 28428")).replace(
        "28706 checks;", "28709 checks;")
    counts, total = OBS["verify_runner_log"](synthetic)
    require(counts["material"]["steps"] == 358 and total == 28709, "Wrong positive step-parser control")
    negative["skipped_m358"] = rejected(
        lambda: OBS["verify_runner_log"](synthetic.replace("  [M358] offline parser sentinel\n", "")), "Material steps changed")
    return receipt, {"archive_sha256": config.Q70_SEAL, "files": len(entries), "checks": 28706,
                     "material_steps": 355, "stock_receipt_sha256": config.Q70_OFFLINE,
                     "retained_boundary_pngs": 33, "retained_placeholder_pngs": 58, "retained_label_pngs": 34}


def source_and_focused(negative):
    q71 = config.Q71
    scoped = q71 / "development-snapshot-r2"
    paths = path_entries(scoped / "source.sha256")
    source = scoped / "source"
    verify_entries(source, paths)
    require(source_tree(source, paths) == config.R2_TREE and OBS["check_source_version"](source) is None,
            "R2 source tree and version are not exact")
    pins = {
        "source_tree": config.FINAL_TREE, "source_sha256": config.FINAL_SOURCE,
        "source_files": config.FINAL_COUNT, "patch_sha256": config.FINAL_PATCH,
        "stock_runner_sha256": "1" * 64, "native_manifest_sha256": "2" * 64,
        "focused_tools_sha256": config.FOCUSED_LAUNCHER, "focused_verifier_sha256": config.FOCUSED_VERIFIER,
        "focused_runner_sha256": config.FOCUSED_RUNNER, "focused_receipt_sha256": config.FOCUSED_RECEIPT,
        "exact_vs_scoped_source_paths": config.DOC_DELTA,
    }
    config.validate_reviewed(pins)
    candidate = q71 / "exact-candidate"
    manifest = candidate / "source.sha256"
    inputs = Namespace(source_manifest=manifest, native_source=candidate / "source", web_source=candidate / "source",
                       patch=q71 / config.PATCH_NAME)
    run_path(str(ROOT / "prepare-pins.py"))["source_inputs"](inputs, pins)
    negative["old_source"] = rejected(lambda: config.validate_reviewed({**pins, "source_sha256": config.Q70_SOURCE}),
                                      "not the v0.1.71 candidate")
    negative["product_delta_instead_of_docs"] = rejected(
        lambda: config.validate_reviewed({**pins, "exact_vs_scoped_source_paths": ["e2e/src/material.rs"]}), "Material doc")
    negative["missing_exact_runner_pin"] = rejected(
        lambda: config.validate_reviewed({key: value for key, value in pins.items() if key != "stock_runner_sha256"}),
        "Provide every reviewed")
    run = config.DEFAULT_FOCUS / "development-r2"
    negative["scoped_as_stock"] = rejected(lambda: OBS["verify_runner_log"]((run / "runner.log").read_text()),
                                         "exactly the ordered five")
    focused = focused_receipt(manifest, pins, q71 / "web-development")
    metadata = {key: focused[key] for key in ("browser", "adapter")}
    raw = run / "artifacts"
    actual = contrast(raw, metadata)
    require(actual["png_sha256"] == focused["png_sha256"], "The stock contrast decoder differs on the actual R2 captures")
    negative["wrong_actual_adapter"] = rejected(lambda: contrast(raw, {**metadata, "adapter": {}}), "original PNG inventory")
    negative["unreviewed_source_path"] = rejected(
        lambda: focused_receipt(manifest, {**pins, "exact_vs_scoped_source_paths": ["e2e/src/material.rs"]}, q71 / "web-development"),
        "An unreviewed source path changed")
    return pins, focused, actual


def canonical_links(negative):
    old = Path("/tmp/surface-md3-field-boundary-stock-v0170.Mz6M22zL")
    origins = data(old / "bundle-origins.json")
    hashes = OBS["checksums"](old / "browser-binary-sha256.txt")
    canonical = regular_gallery_origin(old, origins, hashes)
    names = sorted(name for name in hashes if Path(name).parent.as_posix() == "web/release/material-gallery")
    names_to_hash = {Path(name).name: hashes[name] for name in names}
    negative["checking_the_link_as_original"] = rejected(
        lambda: verify_entries(old / "bundle/web/release/material-gallery", names_to_hash), "Different or missing pinned file")
    negative["swapped_original_name"] = rejected(
        lambda: regular_gallery_origin(old, {**origins, names[0]: origins[names[1]]}, hashes), "exact canonical target")
    negative["changed_original_digest"] = rejected(
        lambda: regular_gallery_origin(old, origins, {**hashes, names[0]: "0" * 64}), "Different or missing pinned file")
    require(canonical == Path(origins[names[0]]).parent, "Validated canonical directory differs from declared original")
    return len(names)


def receipt_controls(negative, prior, pins, focused, captures):
    with TemporaryDirectory(dir=ROOT, prefix="offline-control-") as directory:
        temp = Path(directory)
        reference = {**prior, "release": config.release_pin(pins), "focused": focused,
                     "text_field_disabled_contrast": captures, "cases": {**prior["cases"],
                     "material": {"steps": 358, "checks": 28428}}}
        # The synthetic receipt only exercises the shell predicate. All image
        # and process evidence was read independently from actual runs above.
        config_path = temp / "pins.json"
        config_path.write_text(json.dumps(pins))
        receipt_path = temp / "receipt.json"

        def check(payload):
            receipt_path.write_text(json.dumps(payload))
            return subprocess.run(["sh", ROOT / "check-receipt.sh", config_path, receipt_path], check=False,
                                  capture_output=True).returncode

        require(check(reference) == 0, "Shell receipt predicate rejected the positive synthetic control")
        incomplete = dict(captures["png_sha256"])
        incomplete.pop(next(iter(incomplete)))
        require(check({**reference, "text_field_disabled_contrast": {**captures, "png_sha256": incomplete}}) != 0,
                "Shell predicate accepted a missing image")
        negative["shell_missing_png"] = True
        require(check({**reference, "focused": {**focused, "visible_windows": 1}}) != 0,
                "Shell predicate accepted a visible window")
        negative["shell_visible_window"] = True
        require(check({**reference, "text_field_boundary_motion": {**prior["text_field_boundary_motion"], "decoded_probes": 1}}) != 0,
                "Shell predicate weakened inherited boundary acceptance")
        negative["shell_inherited_oracle_weakened"] = True


def main():
    OBS["self_test"]()
    negative = {}
    prior, previous = stock_predecessor(negative)
    pins, focused, captures = source_and_focused(negative)
    links = canonical_links(negative)
    receipt_controls(negative, prior, pins, focused, captures)
    print(json.dumps({"scope": "offline only; no .71 stock run or process launch",
                      "verified_previous_portable_stock": previous,
                      "original_canonical_links_checked": links,
                      "focused": {"actual_receipt_sha256": config.FOCUSED_RECEIPT, "checks": focused["checks"],
                                  "steps": focused["step_ids"], "headless_samples": focused["headless_samples"],
                                  "original_pngs": len(captures["png_sha256"]), "ink_pixels": captures["ink_pixels"],
                                  "exact_source_bound": True},
                      "negative_controls": negative}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
