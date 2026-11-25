"""Bind the v0.1.69 scoped receipt and recompute original stock placeholder pixels."""

from pathlib import Path
from runpy import run_path

from evidence_common import data, digest, normalized, path_entries, require, seal
from stock_config import DEFAULT_FOCUS, FOCUSED_STEPS, LAST_MATERIAL, PLACEHOLDER_PNGS, RETAINED_PNGS, R2_MANIFEST, R2_WEB_MANIFEST

ROOT = Path(__file__).resolve().parent
CHECK = ROOT / "placeholder-proof/verify-placeholder.py"
FEATURE = "e2e/src/material_text_field_placeholder.rs"
R2_BARRIER = "e2e/src/material_text_field_label_motion.rs"
REGISTRY = "e2e/src/material.rs"
SCOPE = [f"M{number}" for number in range(325, LAST_MATERIAL + 1)]


def verify(artifacts, metadata):
    result = normalized(run_path(str(CHECK))["verify"](artifacts))
    require(result["browser"] == metadata["browser"] and result["adapter"] == metadata["adapter"]
            and len(result["pngs"]) == PLACEHOLDER_PNGS, "Stock placeholder browser, adapter or originals differ")
    return result


def delta(left, right):
    return sorted(path for path in left.keys() | right.keys() if left.get(path) != right.get(path))


def focused_receipt(exact_manifest, pins, canonical=None, focused=DEFAULT_FOCUS):
    focused = focused.resolve(strict=True)
    seal(focused, "SHA256SUMS", pins["focused_tools_sha256"])
    require(digest(focused / "verify-placeholder.py") == digest(CHECK),
            "The archived and real scoped verifier bytes differ")
    require(digest(exact_manifest) == pins["source_sha256"], "Wrong exact source manifest")
    original = focused / "original-source.sha256"
    selected = focused / "focused-source.sha256"
    web_manifest = focused / "web.sha256"
    require(digest(original) == R2_MANIFEST and digest(web_manifest) == R2_WEB_MANIFEST,
            "The reviewed R2 source or Web input changed")
    stock, dev, scoped = map(path_entries, (exact_manifest, original, selected))
    final_delta = delta(stock, dev)
    require(len(stock) == pins["source_files"] and len(dev) == len(scoped) == 688
            and final_delta == pins["exact_vs_scoped_source_paths"]
            and delta(dev, scoped) == [REGISTRY]
            and all(stock[key] == dev[key] == scoped[key] for key in (FEATURE, R2_BARRIER)),
            "An unreviewed source path changed between R2 and exact stock")
    trusted = focused / "development-r2-authorized-verified.json"
    require(digest(trusted) == pins["focused_receipt_sha256"]
            and (focused / "runner.sha256").read_text().strip() == pins["focused_runner_sha256"],
            "The reviewed focused runner or actual receipt changed")
    result = normalized(run_path(str(focused / "verify-placeholder.py"))["verify_run"](focused / "development-r2-authorized"))
    require(result == data(trusted) and result["steps"] == SCOPE and len(SCOPE) == FOCUSED_STEPS
            and result["checks"] == 2373 and result["headless_samples"] == 443 and result["visible_windows"] == 0
            and result["remaining_owned_processes"] == []
            and result["inputs"]["runner_sha256"] == pins["focused_runner_sha256"]
            and result["inputs"]["web_compatible_test_only_delta"] == [R2_BARRIER]
            and len(result["retained_label_motion"]["captures"]) == RETAINED_PNGS
            and len(result["pngs"]) == PLACEHOLDER_PNGS, "The finished actual R2 evidence scope differs")
    gallery = path_entries(web_manifest)
    require(len(gallery) == 4 and result["inputs"]["web_files"] == gallery, "Wrong scoped Gallery inventory")
    canon = None if canonical is None else {path.name: digest(path) for path in canonical.iterdir() if path.is_file()}
    require(canon is None or len(canon) == 4, "Wrong stock Gallery inventory")
    return {**result, "steps": FOCUSED_STEPS, "step_ids": SCOPE,
            "exact_source_sha256": pins["source_sha256"], "exact_vs_scoped_source_paths": final_delta,
            "focused_receipt_sha256": pins["focused_receipt_sha256"], "gallery_files": gallery,
            "canonical_gallery_files": canon, "canonical_gallery_match": None if canon is None else canon == gallery}
