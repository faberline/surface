"""Reviewed inputs for the exact v0.1.70 macOS stock gate."""

import json
from pathlib import Path
import re

from evidence_common import digest, require

RELEASE = "0.1.70"
LAST_MATERIAL = 356
STOCK_STEPS = 355
FOCUSED_STEPS = 32
FOCUSED_CHECKS = 5228
FOCUSED_SAMPLES = 547
RETAINED_PNGS = 34
PLACEHOLDER_PNGS = 58
BOUNDARY_PNGS = 33
DOC_DELTA = ["docs/material-coverage.json"]
PIN_NAME = "reviewed-pins.json"
PATCH_NAME = "text-field-boundary-v0170.patch"
STOCK_RECEIPT = "verified-boundary-stock.json"
DEFAULT_FOCUS = Path("/tmp/surface-md3-field-boundary-focused-v0170.rWtO97J5")
SHA = re.compile(r"[0-9a-f]{64}")

Q70 = Path("/tmp/surface-md3-field-boundary-v0170.X35mBqoS")
Q69 = Path("/tmp/surface-md3-placeholder-v0169-package-r2.xzjZ49Cw/portable")
Q69_LOCAL = Path("/tmp/surface-md3-field-placeholder-stock-v0169.RECb56Zt")
Q69_SEAL = "6e6069ce438e25821707ad230972f3a435397cdc98d054b36fddeb8ad774d4e8"
Q69_SOURCE = "1b1379f04bbb534b6a25e857185de34160b367bfa60ef9a36a1fce1d0a8d3d55"
Q69_TREE = "b243564b27c4fb8d2856afc5ae19cd4cb2ad905a"
Q69_OFFLINE = "1fb80a2d85cd6a9642ee60006d06c1694de25662c1f993d57b372377d38d5a8c"
Q69_REPORT = "7a4bb0c9eb92353c0212bf3e9450a4125bb85831a7555f029a93f0172b992b25"
Q69_LOG = "d0b916b926cdda691fa4b7d8300323d0cc9ca34024a18d089e43ad9997e7051b"
Q69_WINDOW = "03ffbbf93b93eab3b44927fff7e709473562be5900bc47e207dd115a3c78b680"
Q69_HARNESS = "0622adc060c174054646cb971089d0133632b4ce6a7dd0e662bb845d79d88e26"

FOCUSED_LAUNCHER = "dcd18bb0f9a6456fffef989de863ea32a5b00e5c01447bcec8cde908f65015fc"
FOCUSED_VERIFIER = "b826c77a2cd8ebd0a27a80d12df42e1ba32b6dedb0a02e938ce3adaa12869192"
FOCUSED_RECEIPT = "eda2fc14c5b0972ffd5aa396e2778fbaa3b1657d96c38398d1802e89d8e3492c"
FOCUSED_RUNNER = "0bad066e5bc85537bee80913848dcb87cea5968c6b54fd3cf9ef8a43e46b2833"
R1_MANIFEST = "f0b9bf4f43c9ec853fd2ec327028dabfe8215846ef0beec6222f3ac7fecb684c"
R1_SELECTOR = "27113e50b159dbefc3901f41a3858b7ae901b3a40b1aa12141e2fda21046797d"
R1_WEB_MANIFEST = "cd449385e300bf6a78226070f0a8941534520ab461abf3ddaa764b9e5f47304a"
FINAL_SOURCE = "bc6f3023d68c8bec8c801c7a9a2d7c88207d625697cffb34f1eb4f5fef5a6d85"
FINAL_TREE = "409a21f6023802f055371cdec4391976c55ffd17"
FINAL_PATCH = "ca4fe795f6a347be0ddd6296b7bca2537ba70b645b93cfc641228f833f31095f"
FINAL_COUNT = 689
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
            "Reviewed source scope is not the v0.1.70 candidate")
    require(value["focused_tools_sha256"] == FOCUSED_LAUNCHER
            and value["focused_verifier_sha256"] == FOCUSED_VERIFIER
            and value["focused_receipt_sha256"] == FOCUSED_RECEIPT
            and value["focused_runner_sha256"] == FOCUSED_RUNNER,
            "Wrong reviewed R1 focused evidence")
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
