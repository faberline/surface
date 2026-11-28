"""Exact identity and input checks for the q68 R3 test runner and R2 products."""

from difflib import unified_diff
import os
from pathlib import Path

from evidence_common import data, digest, path_entries, require, source_tree, verify_entries

R2 = Path("/tmp/surface-md3-label-color-v0168-corrected-replay.2X3oZ1AB")
PRODUCT = Path("/tmp/surface-md3-label-color-v0168-corrected.6WxZjn4h/surface")
R3 = Path("/tmp/surface-md3-label-color-v0168-browser-clock.ectN4rAV")
PRODUCT_TREE = "72ae6c3eb56e9554532f428b3308aadd8358a242"
RUNNER_TREE = "9d3ce235f8efde1459fd41862cd918367fc85207"
PRODUCT_SHA = "086f06d98a7c2f3728b67c9524b15ed8b380fb078d42c48bb80b1f082c24580c"
RUNNER_SOURCE_SHA = "4bced93c36065b03879ea12060d36d0ce7eb88ebfa43ce4a139fc28200219bd8"
PATCH_SHA = "de0899c86af932cbf6c12ed9c7ee30d7d4454856a502e83f7b3cc345de50a84b"
RUNNER_MANIFEST_SHA = "3b61a424352e6be5f22675093b988720074b1e5a4a2afc0a9916a4b941178a81"
BUILD_MANIFEST_SHA = "bcf203bdc573567a3cfa66919491d5e4ae849b3f5f881f16e48e41a8d1d0df46"
NATIVE_MANIFEST_SHA = "4e68692d2b6de8ed7c854b5a427dcb45b264334909d819b14bdff162b3d5f4f5"
PREVIOUS_BUNDLE_SHA = "83e04f80ed4db67d2c1ed48cd2f34585741fb1cd74cf5c4295795564a9db0a5b"
PATH = "e2e/src/material_text_field_label_motion.rs"
RELEASE = {"version": "0.1.68", "last_material_step": 350, "source_files": 685, "source_tree": RUNNER_TREE,
           "source_sha256": RUNNER_SOURCE_SHA, "wide_canvas_height": 800, "rtl_canvas_height": 1020}


def checked_snapshots(root):
    old_file, new_file, patch = (root / name for name in ("product-source.sha256", "runner-source.sha256",
                                                           "settle-compact-focus.patch"))
    require((digest(old_file), digest(new_file), digest(patch)) == (PRODUCT_SHA, RUNNER_SOURCE_SHA, PATCH_SHA)
            and digest(R3 / "source.sha256") == RUNNER_SOURCE_SHA
            and digest(R3 / patch.name) == PATCH_SHA, "Source or settle patch pins differ")
    old, new = path_entries(old_file), path_entries(new_file)
    require(old.keys() == new.keys() and len(new) == 685 and [path for path in old if old[path] != new[path]] == [PATH],
            "Expected exactly the one test-only change")
    require(source_tree(PRODUCT, old) == PRODUCT_TREE and source_tree(R3 / "surface", new) == RUNNER_TREE,
            "Exact source trees differ")
    a, b = ((source / PATH).read_text().splitlines(keepends=True) for source in (PRODUCT, R3 / "surface"))
    delta = "".join(unified_diff(a, b, fromfile=f"a/{PATH}", tofile=f"b/{PATH}"))
    patch_lines = patch.read_text().splitlines()
    normalized = [line.split(" @@")[0] + " @@" if line.startswith("@@") else line
                  for line in patch_lines if not line.startswith(("diff --git ", "index "))]
    require(normalized == delta.splitlines(), "Settle patch differs from the exact changed file")
    return {"product_source_sha256": PRODUCT_SHA, "product_tree": PRODUCT_TREE,
            "runner_source_sha256": RUNNER_SOURCE_SHA, "runner_tree": RUNNER_TREE,
            "test_only_path": PATH, "test_only_patch_sha256": PATCH_SHA}


def exact_bundle():
    original = R2 / "stock"
    recorded = original / "browser-binary-sha256.txt"
    stock_manifest = R2 / "outputs/stock/SHA256SUMS"
    native_manifest = R2 / "outputs/native/SHA256SUMS"
    runner_manifest = R3 / "runner/SHA256SUMS"
    require((digest(recorded), digest(stock_manifest), digest(native_manifest), digest(runner_manifest))
            == (PREVIOUS_BUNDLE_SHA, BUILD_MANIFEST_SHA, NATIVE_MANIFEST_SHA, RUNNER_MANIFEST_SHA),
            "Original R2 product or new R3 executable manifest differs")
    before = path_entries(recorded)
    old_products = {**path_entries(stock_manifest),
                    **{f"debug/{name}": sha for name, sha in path_entries(native_manifest).items()}}
    require(len(before) == 16 and before == old_products
            and data(original / "run/inputs-after.json")["stock_files"] == before,
            "R2 stock assets do not match the actual prior invocation")
    verify_entries(original / "bundle", before)
    runner = path_entries(runner_manifest)
    require(runner.keys() == {"surface-e2e"}, "Unexpected R3 executable entry")
    verify_entries(runner_manifest.parent, runner)
    return {**before, "debug/surface-e2e": runner["surface-e2e"]}


def verified_inputs(runner, runner_sha, stock):
    identities, assets = checked_snapshots(stock), exact_bundle()
    require(path_entries(stock / "browser-binary-sha256.txt") == assets and len(assets) == 16,
            "Staged R3 runner and exact R2 product inventory differs")
    actual = {p.relative_to(stock / "bundle").as_posix() for p in (stock / "bundle").rglob("*") if p.is_file()}
    require(actual == assets.keys(), "Unpinned files in the runtime bundle")
    verify_entries(stock / "bundle", assets)
    require(runner == (stock / "bundle/debug/surface-e2e").resolve() and os.access(runner, os.X_OK)
            and runner_sha == assets["debug/surface-e2e"] == digest(runner)
            and data(stock / "release-pin.json") == RELEASE, "R3 stock runner pin differs")
    require(data(stock / "harness-pin.json") == {
                name: digest(stock / name) for name in path_entries(stock / "harness.sha256")}, "Headless tools differ")
    return {**identities, "runner_sha256": runner_sha, "runner_manifest_sha256": RUNNER_MANIFEST_SHA,
            "runner_build_log_sha256": digest(R3 / "runner-build.log"), "stock_files": assets,
            "original_r2_stock_manifest_sha256": BUILD_MANIFEST_SHA, "native_binary_manifest_sha256": NATIVE_MANIFEST_SHA,
            "harness_manifest_sha256": digest(stock / "harness.sha256"), "release": RELEASE}
