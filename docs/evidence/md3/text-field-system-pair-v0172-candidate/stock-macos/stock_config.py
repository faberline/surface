"""Reviewed exact and scoped inputs for the v0.1.72 macOS stock gate."""

import json
from pathlib import Path
import re

from evidence_common import digest, require

RELEASE = "0.1.72"
LAST_MATERIAL = 362
FOCUSED_STEPS = 13
PAIR_PNGS = 14
CONTRAST_PNGS = 15
PLACEHOLDER_PNGS = 58
DOC_DELTA = ["docs/material-coverage.json"]
PIN_NAME = "reviewed-pins.json"
PATCH_NAME = "text-field-system-pair-v0172.patch"
STOCK_RECEIPT = "verified-system-pair-stock.json"
DEFAULT_FOCUS = Path("/tmp/surface-md3-field-pair-focused-v0172-r8.bAGIL0yE")
SHA = re.compile(r"[0-9a-f]{64}")
Q72 = Path("/tmp/surface-md3-field-pair-v0172.SEhFjvMD")
Q71 = Path("/tmp/surface-md3-disabled-contrast-v0171-package.gBiM0yjr/portable")
Q71_SEAL = "41d846a3e1f56326e63ddadb607cba6d88a2a3d9255e1698b7fa74a681e92021"
Q71_SOURCE = "65c1bf0fd49f018b575cf069af016aa46229cff9dc23d521942f0ba0b6465ee2"
Q71_TREE = "f2b631b0d72830f76b84e71b114018a4410dc813"
Q71_OFFLINE = "bf18f19d74e7c212747731d29666213feb4b3bf72e8249b3daab1a698da3db1d"
Q71_REPORT = "24ba654f1f8689c5d831864b521c9d77d102de3aa8fabceedcfc21208c14ab0f"
Q71_LOG = "01a16faa23d2b03970189a24034b82851ea9bbf5df8dedc28c176ed77bc5e1e2"
Q71_WINDOW = "51435a2d492c63dbbcedad878bf3c6d3a3fd2199062817fb81cd0b92fcb5bc69"

FOCUSED_LAUNCHER = "5d48d1ee9cbb7bc2ba44a362497a7b38c446b3227b7b6f16d0f3a4c0e51fcd25"
FOCUSED_VERIFIER = "d8e101cdd4fe3909c990674a3153db9553aa75b1303f47e112876646065947b4"
FOCUSED_RECEIPT = "94869c456c7ba738962173021e62d08412c0ca51e5207c69641b080fab3d28b8"
FOCUSED_RUNNER = "adfa819a63cc2e8a5f6d355d52c3c0c10687c5b593df69c0de7c37dd71faccda"
FOCUSED_CHECKS = 1993
FOCUSED_SAMPLES = 233
R4_MANIFEST = "5c342f40941e6f5350411c7787faed8651356f8f739bd64f059bdb65622e8649"
R4_TREE = "166eeafa32115770dcf9097a63de31aa7d3201a3"
R4_WEB_MANIFEST = "84359bf859a0582b2da51875f0e6bd279641a0afe28935b0b38ecb2c0cb1f3b1"
R8_MANIFEST = "4a4ad2522f1c080fd870932077619d3c85e2171598e620e93dfbc2065944ea99"
R8_TREE = "6af2ed5be7232a482d09e5a924dec6ab85536651"
R8_SELECTOR = "9b51973bea8880660c899e9e5e60b9759a5692bd4bb1b97c73b22c9e8783081b"
WEB_TEST_DELTA = [
    "e2e/src/material_text_field_system_pair.rs",
    "e2e/src/material_web_contrast.rs",
    "e2e/src/material_web_contrast_selection.rs",
]
FINAL_SOURCE = "b3137077ef9753a91a4e86b3c7d9dedc14b479f4aa4e74c95b299a6805543cd9"
FINAL_TREE = "9333c1f8dba9a7d15aba2ca47dc92c997abf98af"
FINAL_PATCH = "82839f72bf454a8e4bdd22660ecb223684ea7bcd6351158137a9044d1efc3117"
FINAL_COUNT = 691
HASH_KEYS = {
    "source_sha256", "patch_sha256", "stock_runner_sha256", "native_manifest_sha256",
    "focused_tools_sha256", "focused_verifier_sha256", "focused_receipt_sha256", "focused_runner_sha256",
}


def validate_reviewed(value):
    require(isinstance(value, dict) and set(value) == {
        *HASH_KEYS, "source_tree", "source_files", "exact_vs_scoped_source_paths",
    }, "Provide every reviewed source, patch, binary and focused receipt pin")
    require(all(isinstance(value[key], str) and SHA.fullmatch(value[key]) for key in HASH_KEYS),
            "A reviewed SHA-256 is invalid")
    require(value["source_tree"] == FINAL_TREE and value["source_sha256"] == FINAL_SOURCE
            and value["patch_sha256"] == FINAL_PATCH and value["source_files"] == FINAL_COUNT,
            "Reviewed source scope is not the v0.1.72 candidate")
    require(value["focused_tools_sha256"] == FOCUSED_LAUNCHER
            and value["focused_verifier_sha256"] == FOCUSED_VERIFIER
            and value["focused_receipt_sha256"] == FOCUSED_RECEIPT
            and value["focused_runner_sha256"] == FOCUSED_RUNNER, "Wrong reviewed R8 focused evidence")
    require(value["exact_vs_scoped_source_paths"] == DOC_DELTA,
            "The final/scoped source delta must be exactly the reviewed Material doc")
    return value


def reviewed(path, expected=None):
    require(path.is_file() and not path.is_symlink(), "Missing regular reviewed pin file")
    if expected is not None:
        require(SHA.fullmatch(expected or "") and digest(path) == expected, "Reviewed pin file changed")
    return validate_reviewed(json.loads(path.read_text()))


def release_pin(value):
    return {"version": RELEASE, "last_material_step": LAST_MATERIAL,
            "source_sha256": value["source_sha256"], "source_tree": value["source_tree"],
            "source_files": value["source_files"], "wide_canvas_height": 800, "rtl_canvas_height": 1020}
