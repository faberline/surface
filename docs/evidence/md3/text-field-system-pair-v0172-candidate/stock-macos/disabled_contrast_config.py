"""Reviewed exact and scoped inputs for the v0.1.71 macOS stock gate."""

import json
from pathlib import Path
import re

from evidence_common import digest, require

RELEASE = "0.1.71"
LAST_MATERIAL = 359
FOCUSED_STEPS = 10
FOCUSED_CHECKS = 1470
FOCUSED_SAMPLES = 162
CONTRAST_PNGS = 15
PLACEHOLDER_PNGS = 58
DOC_DELTA = ["docs/material-coverage.json"]
PIN_NAME = "reviewed-pins.json"
PATCH_NAME = "text-field-disabled-contrast-v0171.patch"
STOCK_RECEIPT = "verified-disabled-contrast-stock.json"
DEFAULT_FOCUS = Path("/tmp/surface-md3-field-contrast-focused-v0171-r2.ik1t5atk")
SHA = re.compile(r"[0-9a-f]{64}")
Q71 = Path("/tmp/surface-md3-field-contrast-v0171.xE0ruhQh")
Q70 = Path("/tmp/surface-md3-boundary-v0170-package.pxSuFZW5/portable-r2")
Q70_SEAL = "5768509d54c91d83054cf12afff6a3eee8070c333dfa29a5324ee67077fc5a48"
Q70_SOURCE = "bc6f3023d68c8bec8c801c7a9a2d7c88207d625697cffb34f1eb4f5fef5a6d85"
Q70_TREE = "409a21f6023802f055371cdec4391976c55ffd17"
Q70_OFFLINE = "78f3585f8292bb91e4983bdbfb05f98ec8c1d29aab375970bfec67803f8da6bd"
Q70_REPORT = "cbf4fb4e20bba3a9ef27d0a921a230b4f432168d5950dfa604096bb86c975843"
Q70_LOG = "7b595c761ca99d5105a2b904caee934a7526304f74bfe6f0326ab209dffa0da7"
Q70_WINDOW = "9a7610e27997ae3ad13870526d79f9fada5bd708d8cdc6c96974dbfcca053a8e"
Q70_REPAIR = "9ec480004710e5ddcb2003d1f949c5f826e0c3e3297c45ffcb85e0eccd7452e6"

FOCUSED_LAUNCHER = "2f94ab354a6c255590ec84db75f220b4efdc1f2b58d0e23638a9caf907e3e902"
FOCUSED_VERIFIER = "442ede58283ac8f39cfc3a7ff355e94c53f0639a1130b34bd75a67d789f909eb"
FOCUSED_RECEIPT = "c6a3a984fcd408108ae61fcb889e7097f3c70fe370898c8176f6c016361ddada"
FOCUSED_RUNNER = "06bdbfd19fde7d0c20f2c894b94bece803e5082ca416ba702779ae903cc87315"
FOCUSED_REPORT = "b5fce965cf45b245892902ee2b20f77b1873773b47f40274c833915f4debf44e"
R1_MANIFEST = "888bf446dba55df082f0f8c8c9181e5a76fbad76c63e3cc002c9f3d211d07ed8"
R2_MANIFEST = "a52220af9e6ee0c7339dfce6641280dc77530f55a0dcdac39de17e37d16fb3ab"
R2_TREE = "a97db3adfe4f3ae0da6646d1ce3d3fe00da0e5f0"
R2_SELECTOR = "51c6a95503ad8d4d22cc465e177539d4644d9fb08e021f6c02c0eb03dc2af162"
R2_WEB_MANIFEST = "07e73ce73dc3f5778e3958c21c611d636d5ce7c3b0852c4aacf8e6bc7a18a0ad"
WEB_TEST_DELTA = ["e2e/src/material_multiline_text_field.rs", "e2e/src/material_text_field_disabled_contrast.rs"]
FINAL_SOURCE = "65c1bf0fd49f018b575cf069af016aa46229cff9dc23d521942f0ba0b6465ee2"
FINAL_TREE = "f2b631b0d72830f76b84e71b114018a4410dc813"
FINAL_PATCH = "9c3e271346b5285bddfd1c10586bed2c5c341b0f434aeb0dbc40c2c8dc92b578"
FINAL_COUNT = 690
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
            "Reviewed source scope is not the v0.1.71 candidate")
    require(value["focused_tools_sha256"] == FOCUSED_LAUNCHER
            and value["focused_verifier_sha256"] == FOCUSED_VERIFIER
            and value["focused_receipt_sha256"] == FOCUSED_RECEIPT
            and value["focused_runner_sha256"] == FOCUSED_RUNNER, "Wrong reviewed R2 focused evidence")
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
