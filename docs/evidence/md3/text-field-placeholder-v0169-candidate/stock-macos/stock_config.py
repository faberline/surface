"""Explicit inputs for one reviewed v0.1.69 stock run."""

import json
from pathlib import Path
import re

from evidence_common import digest, require

RELEASE = "0.1.69"
LAST_MATERIAL = 353
STOCK_STEPS = 352
FOCUSED_STEPS = 29
RETAINED_PNGS = 34
PLACEHOLDER_PNGS = 58
ALLOWED_DOCS = {"docs/material-coverage.json", "docs/material.md"}
PIN_NAME = "reviewed-pins.json"
DEFAULT_FOCUS = Path("/tmp/surface-md3-field-placeholder-focused-v0169-r2.8AArzw7l")
SHA = re.compile(r"[0-9a-f]{64}")
Q68 = Path("/tmp/surface-md3-field-color-v0168-package.hdUSH5cz/portable")
Q68_SEAL = "5e19c8384a44e5ed31e5d61d1a7f03e14234476960b2f78ba6b78ba23fc37811"
Q68_SOURCE = "17aeee497edef781a3da477d8fdb7820c0eaa3494a4b840fcf41133925949df6"
Q68_TREE = "0db939d3f663fdb5b9fa3dd334382937693b409e"
Q68_OFFLINE = "010a9848cecc3f0f1b0dddbab6d2e498b2d93b62865f3d6221a6191117cdafaa"
Q68_REPORT = "b86bbf7eacbe3399170feb7ea314d99edad8692e7e7acf3197c7e8f0c6c5d9f1"
DEFAULT_FOCUS_TOOLS = "a55188fe39c863318f9e73fd36a639540d70b11f362d266e5ec1ab8b139e3824"
R2_MANIFEST = "47642376bb5f97f07a542ba6c1fb159e1bdeda1d904cfb4c3dc39096d2c75354"
R2_WEB_MANIFEST = "700ba34b6e25aaf7406f0ecbdbae056274a6fba9725e7a3e120a6e099e81cd33"
FINAL_SOURCE = "1b1379f04bbb534b6a25e857185de34160b367bfa60ef9a36a1fce1d0a8d3d55"
FINAL_TREE = "b243564b27c4fb8d2856afc5ae19cd4cb2ad905a"
FINAL_PATCH = "fb2ad6586be0d5d1c7957ce53d91b37119a1c53bbe3e550488e9b24325c295ba"
FOCUSED_RECEIPT = "dcf665eb0c3b390322ac8c7e5bef9590a4759ca871417ce13f9afa6e3b66fd8f"
FOCUSED_RUNNER = "bd2f0a2bba9b2c0e969eff3352bbf04980bc38c3ac19c865f4ecf9dc5a391e55"
HASH_KEYS = {
    "source_sha256", "patch_sha256", "stock_runner_sha256", "native_manifest_sha256",
    "focused_tools_sha256", "focused_receipt_sha256", "focused_runner_sha256",
}


def validate_reviewed(value):
    require(isinstance(value, dict) and set(value) == {
        *HASH_KEYS, "source_tree", "source_files", "exact_vs_scoped_source_paths",
    }, "Provide every reviewed source, patch, binary and focused receipt pin")
    require(all(isinstance(value[key], str) and SHA.fullmatch(value[key]) for key in HASH_KEYS),
            "A reviewed SHA-256 is invalid")
    require(value["source_tree"] == FINAL_TREE and value["source_sha256"] == FINAL_SOURCE
            and value["patch_sha256"] == FINAL_PATCH and value["source_files"] == 688,
            "Reviewed source scope is not the v0.1.69 candidate")
    require(value["focused_tools_sha256"] == DEFAULT_FOCUS_TOOLS
            and value["focused_receipt_sha256"] == FOCUSED_RECEIPT
            and value["focused_runner_sha256"] == FOCUSED_RUNNER,
            "Wrong reviewed R2 scoped evidence")
    delta = value["exact_vs_scoped_source_paths"]
    require(isinstance(delta, list) and all(isinstance(name, str) for name in delta)
            and delta == sorted(set(delta)) and set(delta) <= ALLOWED_DOCS,
            "The permitted final/scoped delta must name only reviewed Material docs")
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
