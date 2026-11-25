"""Reviewed exact and scoped inputs for the v0.1.74 macOS stock gate."""

import json
from pathlib import Path
import re

from evidence_common import digest, require

RELEASE = "0.1.74"
LAST_MATERIAL = 369
FOCUSED_STEPS = 13
PAIR_PNGS = 14
CONTRAST_PNGS = 15
PLACEHOLDER_PNGS = 58
PIN_NAME = "reviewed-pins.json"
PATCH_NAME = "adaptive-navigation-v0174.patch"
STOCK_RECEIPT = "verified-adaptive-navigation-stock.json"
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
NAV_STAGE = Path("/tmp/surface-md3-horizontal-nav-v0173.OEjA5TvP")
NAV_FOCUS = Path("/tmp/surface-md3-horizontal-nav-focused-v0173-r3.KYlcfBAK")
NAV_LAUNCHER = "14f92282fe4ef079c5dbf88aef59bba6ad733cb2b12d9d0842d93d328a9dd998"
NAV_VERIFIER = "a81f98dacae2c673587fdb156dd71f60b657c3d9bc6913ea87eaae646951625b"
NAV_VERIFIER_MANIFEST = "740c4e5cb8f2b37c3c7d832f68f13a0b16446a471cd428b5cff10eb0dd1ff71d"
NAV_RECEIPT = "c83e19154407ac7b45e9ee467d3a0f1ef5be36839f3bb8f468a906965e1f5a98"
NAV_RUNNER = "c97429941d2d52ad7a44983b13a758635afbaeeae0c3c2881fad93e173a74152"
NAV_SCOPED = "03b61cdd1e2b7ee5f5a91d9267fe8102883a0a410b226f08cab2363f7277e2e9"
NAV_PRODUCT = "38cdcd510d2808cd3685691ae0656c25ca1017f7c3cbf8d88af42c08370caa27"
NAV_TREE = "1bfec92f8370dee480c497034be62f09310986ba"
NAV_WEB = "d4d194543bb95dda8505681fee1a05bdfb9321e0ad458a96a5c9a9a6f803cc69"
NAV_STEPS = [f"M{i}" for i in (*range(270, 274), *range(363, 366))]
NAV_CHECKS = 1041
NAV_SAMPLES = 58
Q72_FINAL = "b3137077ef9753a91a4e86b3c7d9dedc14b479f4aa4e74c95b299a6805543cd9"
ADAPTIVE_STAGE = Path("/tmp/surface-md3-adaptive-nav-v0174.b8nBXDdY")
ADAPTIVE_FOCUS = Path("/tmp/surface-md3-adaptive-nav-focused-v0174-r4.L4lJahZo")
ADAPTIVE_LAUNCHER = "1248f280ed8cdd7ddc130a753fbb0634ae83deb5cf5d107f746555625f2b5a66"
ADAPTIVE_VERIFIER = "619c318f4c4683a4d41ac6ac9e5db63868ac2608a182db2ead7b8350fa3b8586"
ADAPTIVE_RECEIPT = "9db4a851e91efe79d448d4f69737aba9950ec3e759cff3b9af6014f77aabc4e0"
ADAPTIVE_RUNNER = "9a5c78bc8688ec6303339e10d48e00d9520e329178eaa602f3ed2cdab70ce341"
ADAPTIVE_SCOPED = "4e826921a5a7d664d0672b49e5e5df08b8e82e49ddf0c3233d3c6a6f820ad40a"
ADAPTIVE_PRODUCT = "7a2f1480c1d21a5b5b384bb04f3c52a396ac39c51e5ccf2ffabd97de241d21ab"
ADAPTIVE_TREE = "3eae1560dae129a29b0de18d1826464224fb2e0a"
ADAPTIVE_WEB_SOURCE = "8e3accebefb67a07449011ff637f76e8a717f629bd9e595e526cadac3833e160"
ADAPTIVE_WEB_TREE = "98b75a23d173b72567fb60d925d2c301e1a12ea5"
ADAPTIVE_WEB = "b55cb95697b45fe40a5d937eeb5976c288aed4c1d3716a35d7681d3766768897"
ADAPTIVE_CASE = "e2e/src/material_adaptive_navigation_scaffold.rs"
ADAPTIVE_DELTA = ["docs/material-coverage.json", ADAPTIVE_CASE]
ADAPTIVE_STEPS = [f"M{i}" for i in range(366, 370)]
ADAPTIVE_CHECKS = 200
ADAPTIVE_SAMPLES = 27
Q73_FINAL = "72716762c2e2615a92e29dc45c840e0c7278cafbf31d489d165aa92b20556e14"
FINAL_SOURCE = "7aedeb9161baadca46aa6fdd83848b7e9c5498bccfac0e9a0e21e2782ec3f0bd"
FINAL_TREE = "f7a92f77ecc9b37c91d81ace4877cd8038044d5c"
FINAL_PATCH = "226a1fb9d70d2358578d662322f89530bab0abef80e86e7832ff467a611810d5"
FINAL_COUNT = 696
PREVIOUS_RUNNER = "1959bb6f119f56587cfffc7cce14db460540cd64efc850688770358a91c9413d"
PREVIOUS_ASSETS = "e302e154cc70814d75228841cfb481f20f50b56b110be20104911a0914a0a737"
HASH_KEYS = {
    "source_sha256", "patch_sha256", "stock_runner_sha256", "native_manifest_sha256",
    "stock_runner_manifest_sha256", "stock_assets_sha256",
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
            "Reviewed source scope is not the v0.1.74 candidate")
    require(value["focused_tools_sha256"] == ADAPTIVE_LAUNCHER
            and value["focused_verifier_sha256"] == ADAPTIVE_VERIFIER
            and value["focused_receipt_sha256"] == ADAPTIVE_RECEIPT
            and value["focused_runner_sha256"] == ADAPTIVE_RUNNER,
            "Wrong reviewed R4 adaptive navigation evidence")
    require(value["exact_vs_scoped_source_paths"] == ADAPTIVE_DELTA, "Wrong exact-vs-scoped changed paths")
    require(value["stock_runner_sha256"] != PREVIOUS_RUNNER and value["stock_assets_sha256"] != PREVIOUS_ASSETS
            and value["stock_runner_sha256"] != ADAPTIVE_RUNNER, "Historical or scoped binaries are not exact stock")
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
