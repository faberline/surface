"""Check the v0.1.72 stock prerequisites and negative controls offline."""

from argparse import Namespace
import json
from pathlib import Path
import re
from runpy import run_path
import subprocess
from tempfile import TemporaryDirectory

from evidence_common import data, digest, path_entries, require, seal, verify_entries
import stock_config as config
from verify_boundary_stock import verify as boundary
from verify_disabled_contrast import regular_gallery_origin, verify as disabled
from verify_label_color import verify as label
from verify_placeholder_stock import verify as placeholder
from verify_system_pair_stock import focused_receipt, tools, verify as pair

ROOT = Path(__file__).resolve().parent
OBS = run_path(str(ROOT / "observe-multiline.py"))
R5_ARCHIVE = Path("/tmp/surface-md3-field-pair-focused-v0172-r6.mIkOozwh/r5-rejected")
R4_ARCHIVE = Path("/tmp/surface-md3-field-pair-focused-v0172-r5.bce6QeJB/r4-rejected")


def rejected(check, phrase):
    try:
        check()
    except (OSError, RuntimeError, ValueError) as error:
        require(phrase in str(error), f"Unexpected rejection: {error}")
        return phrase
    raise ValueError(f"Defective control was accepted: {phrase}")


def predecessor(negative):
    archive = config.Q71
    files = seal(archive, "SHA256SUMS", config.Q71_SEAL, exact=True)
    stock = archive / "stock-macos"
    paths = (archive / "source.sha256", archive / "verified-disabled-contrast-v0171.json",
             stock / "stock-mac-exact-offline.json", stock / "run/runner.log", stock / "run/headless-window-observation.json")
    require(len(files) == 566 and len(path_entries(paths[0])) == 690
            and tuple(map(digest, paths)) == (config.Q71_SOURCE, config.Q71_REPORT, config.Q71_OFFLINE,
                                               config.Q71_LOG, config.Q71_WINDOW), "Wrong completed q71 portable inputs")
    summary, receipt, watch = map(data, (paths[1], paths[2], paths[4]))
    require(summary["source"]["tree"] == receipt["release"]["source_tree"] == config.Q71_TREE
            and summary["stock"]["total_checks"] == receipt["total_checks"] == 29384
            and receipt["cases"]["material"] == {"steps": 358, "checks": 29103}
            and receipt["guard_and_window_checks"] == "passed" and watch["stock_exit"] == 0
            and watch["visible_owned_windows"] == watch["remaining_observed_processes"] == watch["problems"] == [],
            "Wrong actual predecessor stock runtime")
    artifacts = stock / "run/artifacts"
    metadata = {key: receipt[key] for key in ("browser", "adapter")}
    for verifier, name in ((boundary, "text_field_boundary_motion"), (label, "text_field_label_motion"),
                           (placeholder, "text_field_placeholder"), (disabled, "text_field_disabled_contrast")):
        require(verifier(artifacts, metadata) == receipt[name], f"Retained {name} oracle differs")
    old_log = paths[3].read_text()
    require(OBS["verify_runner_log"](old_log, 359) == (receipt["cases"], 29384), "The inherited suite parser changed")
    negative["prior_stock_steps"] = rejected(lambda: OBS["verify_runner_log"](old_log), "Material steps changed")
    negative["prior_has_no_system_pair"] = rejected(lambda: pair(artifacts, metadata), "browser-material-text-field-system-pair.json")
    line = re.search(r"^case material \.\.\. ok \(358 steps, 29103 checks,.*$", old_log, re.M).group()
    new = "\n".join(f"  [M{step}] offline parser sentinel" for step in (360, 361, 362))
    log = old_log.replace(line, new + "\n" + line.replace("358 steps, 29103", "361 steps, 29106")).replace(
        "29384 checks;", "29387 checks;")
    counts, total = OBS["verify_runner_log"](log)
    require(counts["material"]["steps"] == 361 and total == 29387, "The stock step ledger control changed")
    negative["missing_m362"] = rejected(lambda: OBS["verify_runner_log"](log.replace("  [M362] offline parser sentinel\n", "")),
                                        "Material steps changed")
    return receipt, {"archive_sha256": config.Q71_SEAL, "archive_files": len(files), "stock_checks": 29384,
                     "material_steps": 358, "disabled_contrast_pngs": len(receipt["text_field_disabled_contrast"]["png_sha256"])}


def source_and_focused(negative):
    focus = config.DEFAULT_FOCUS
    seal(focus, "launcher.sha256", config.FOCUSED_LAUNCHER)
    with_inputs = run_path(str(focus / "prepare-focused.py"))
    scope, gallery = with_inputs["check_scope"](), with_inputs["check_gallery"]()
    require(scope["product_tree"] == config.R8_TREE and scope["steps"] == tools()["STEPS"]
            and scope["product_source_manifest_sha256"] == config.R8_MANIFEST
            and gallery["web_manifest_sha256"] == config.R4_WEB_MANIFEST
            and gallery["web_producer_manifest_sha256"] == config.R4_MANIFEST
            and gallery["web_compatible_test_only_delta"] == config.WEB_TEST_DELTA,
            "Wrong original q72 scoped inputs")
    r4 = run_path(str(focus / "preserve-r4-rejection.py"))["review"](R4_ARCHIVE)
    r5 = run_path(str(focus / "preserve-r5-rejection.py"))["review"](R5_ARCHIVE)
    require((r4["checks"], r5["checks"]) == (1669, 1675) and r5["composited_rgb"] == [8, 9, 68],
            "Wrong original rejected provenance")
    for archived in (R4_ARCHIVE / "development-r4", R5_ARCHIVE / "development-r5"):
        negative[archived.name + "_cannot_pass"] = rejected(
            lambda: tools()["verify_captures"](archived / "artifacts"), "did not finish in its approved scope")
    # These two sentinel hashes are input-schema controls. Prepare checks the
    # actual build bytes and will reject both.
    pins = {**data(ROOT / "reviewed-pins.example.json"),
            "stock_runner_sha256": "1" * 64, "native_manifest_sha256": "2" * 64}
    config.validate_reviewed(pins)
    exact = config.Q72 / "exact-candidate"
    manifest = exact / "source.sha256"
    args = Namespace(source_manifest=manifest, native_source=exact / "source", web_source=exact / "source",
                     patch=config.Q72 / config.PATCH_NAME)
    run_path(str(ROOT / "prepare-pins.py"))["source_inputs"](args, pins)
    completed = focused_receipt(manifest, pins, Path(gallery["original_gallery"]))
    negative["prior_manifest"] = rejected(lambda: config.validate_reviewed({**pins, "source_sha256": config.Q71_SOURCE}),
                                         "not the v0.1.72 candidate")
    negative["wrong_scoped_digest"] = rejected(lambda: config.validate_reviewed({**pins, "focused_runner_sha256": "0" * 64}),
                                              "Wrong reviewed R8 focused evidence")
    negative["wrong_final_delta"] = rejected(lambda: config.validate_reviewed({**pins, "exact_vs_scoped_source_paths": []}),
                                            "must be exactly the reviewed Material doc")
    return pins, completed, {"type": "offline scoped intake", "product_sha256": config.R8_MANIFEST,
                             "exact_source_sha256": pins["source_sha256"], "exact_source_tree": pins["source_tree"],
                             "exact_source_files": pins["source_files"], "exact_patch_sha256": pins["patch_sha256"],
                             "gallery_sha256": config.R4_WEB_MANIFEST, "scoped_steps": completed["steps"],
                             "scoped_checks": completed["checks"], "scoped_headless_samples": completed["headless_samples"],
                             "scoped_originals": len(completed["png_sha256"]), "actual_retained_disabled_contrast_originals":
                             len(completed["retained_disabled_contrast"]["png_sha256"]),
                             "rejected_r4_checks": r4["checks"], "rejected_r5_checks": r5["checks"],
                             "verified_actual_scope": True}


def canonical_links(negative):
    lane = Path("/tmp/surface-md3-field-contrast-stock-v0171.PykYn5L5")
    origins = data(lane / "bundle-origins.json")
    hashes = OBS["checksums"](lane / "browser-binary-sha256.txt")
    actual = regular_gallery_origin(lane, origins, hashes)
    names = sorted(name for name in hashes if Path(name).parent.as_posix() == "web/release/material-gallery")
    require(actual == Path(origins[names[0]]).parent, "Wrong original canonical link endpoint")
    negative["reject_bundle_as_original"] = rejected(
        lambda: verify_entries(lane / "bundle/web/release/material-gallery", {Path(name).name: hashes[name] for name in names}),
        "Different or missing pinned file")
    negative["wrong_canonical_digest"] = rejected(
        lambda: regular_gallery_origin(lane, origins, {**hashes, names[0]: "0" * 64}), "Different or missing pinned file")
    return len(names)


def shell_control(negative, previous, pins, focus):
    # A q71 stock receipt plus q72 scoped originals exercises the final jq
    # predicate. The synthetic combination cannot qualify a q72 stock run.
    pair_data = pair(config.DEFAULT_FOCUS / "development-r8/artifacts", focus)
    proof = {**previous, "release": config.release_pin(pins), "focused": focus, "text_field_system_pair": pair_data,
             "cases": {**previous["cases"], "material": {"steps": 361, "checks": 29106}}}
    with TemporaryDirectory(dir=ROOT, prefix="offline-control-") as directory:
        root = Path(directory)
        p = root / "pins.json"
        r = root / "receipt.json"
        p.write_text(json.dumps(pins))

        def check(value):
            r.write_text(json.dumps(value))
            return subprocess.run(["sh", ROOT / "check-receipt.sh", p, r], check=False, capture_output=True).returncode

        require(check(proof) == 0, "The synthetic shell predicate control was rejected")
        require(check({**proof, "text_field_system_pair": {**pair_data, "png_sha256": {}}}) != 0,
                "The shell accepted an incomplete capture inventory")
        require(check({**proof, "focused": {**focus, "visible_windows": 1}}) != 0,
                "The shell accepted a visible window")
        require(check({**proof, "focused": {**focus, "headless_samples": 1}}) != 0,
                "The shell accepted a different scoped observation count")
        negative.update({"shell_missing_pngs": True, "shell_visible_window": True, "shell_wrong_scope_samples": True})


def main():
    OBS["self_test"]()
    negative = {}
    previous, receipt = predecessor(negative)
    pins, focused, scope = source_and_focused(negative)
    links = canonical_links(negative)
    shell_control(negative, previous, pins, focused)
    print(json.dumps({"scope": "offline only; no v0.1.72 stock launch", "verified_predecessor": receipt,
                      "original_canonical_links_checked": links, "scoped_intake": scope, "negatives": negative}, indent=2))


if __name__ == "__main__":
    main()
