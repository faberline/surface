"""Read-only identity checks for the exact R5 runner and R4 Web producer."""

from difflib import unified_diff
import os
from pathlib import Path

from evidence_common import data, digest, path_entries, require, source_tree, verify_entries

ROOT = Path(__file__).resolve().parent
R4 = Path("/tmp/surface-md3-choice-menu-v0176-corrected.2R544r7F/source")
R5 = Path("/tmp/surface-md3-choice-menu-v0176-browser-corner.3ixT0mHe")
PRODUCER = Path("/tmp/surface-md3-choice-menu-v0176-corrected-replay.Q1k95t8V/outputs")
R4_TREE = "37e4454bc7d3f63f7111c8264d4e59e451c8ef9e"
R5_TREE = "b6dc18c4baf8e6b65bddfbc6f59b7b07b4a0b5b9"
R4_SHA = "a9aa38ee7ab64746471491ccfd6cd7df0132e8f30254d536a6fdd3fc95e10eb8"
R5_SHA = "28c5bf44e9618f47ab4b541851d50c0d1d3c7df4748aac5518103d9f24e92893"
PATCH_SHA = "4239e9da0541b1794b4fc15f26ddef035d97d0a6f0f35572de5cccb0039c7b39"
WEB_SHA = "3c471440987193713a55394e334f0b8a5078da7bad8dc98016c9dca1985d9ec4"
CHANGED_PATH = "e2e/src/material_choice_menu.rs"


def check_source_and_web():
    parent, candidate, patch = (ROOT / name for name in
                                ("product-source.sha256", "runner-source.sha256", "rtl-corner-probe.patch"))
    require((digest(parent), digest(candidate), digest(patch)) == (R4_SHA, R5_SHA, PATCH_SHA)
            and digest(R5 / "source.sha256") == R5_SHA
            and (R5 / "source.tree").read_text().strip() == R5_TREE
            and digest(R5 / patch.name) == PATCH_SHA, "Exact R4, R5 or probe patch pin differs")
    old, new = path_entries(parent), path_entries(candidate)
    changes = sorted(path for path in old.keys() | new.keys() if old.get(path) != new.get(path))
    require(old.keys() == new.keys() and len(new) == 703 and changes == [CHANGED_PATH],
            "R5 must overlay the single browser test file")
    for root, entries, tree in ((R4, old, R4_TREE), (R5 / "source", new, R5_TREE)):
        actual = {file.relative_to(root).as_posix() for file in root.rglob("*") if file.is_file()}
        require(actual == entries.keys() and source_tree(root, entries) == tree, f"Wrong source snapshot: {root}")
    a, b = ((root / CHANGED_PATH).read_text().splitlines(keepends=True) for root in (R4, R5 / "source"))
    diff = "".join(unified_diff(a, b, fromfile=f"a/{CHANGED_PATH}", tofile=f"b/{CHANGED_PATH}"))
    require(patch.read_text() in (diff, f"diff --git a/{CHANGED_PATH} b/{CHANGED_PATH}\n{diff}"),
            "Probe patch bytes differ from the only source change")
    manifest = PRODUCER / "web/SHA256SUMS"
    require(digest(manifest) == WEB_SHA, "Use the verified R4 Web producer")
    proof = data(PRODUCER / "verified-build-inputs.json")
    require(proof["source"]["tree"] == R4_TREE and proof["source"]["source_sha256"] == R4_SHA
            and proof["artifact_manifests"]["web/SHA256SUMS"] == WEB_SHA,
            "Web assets do not bind to the exact R4 producer")
    gallery = PRODUCER / "web/material-gallery"
    assets = path_entries(manifest, allow_dot_prefix=True)
    require(len(assets) == 4 and "index.html" in assets
            and {p.relative_to(gallery).as_posix() for p in gallery.rglob("*") if p.is_file()} == assets.keys(),
            "Wrong exact Gallery asset inventory")
    verify_entries(gallery, assets)
    return {"source_tree": R5_TREE, "source_sha256": R5_SHA,
            "web_producer_tree": R4_TREE, "web_producer_source_sha256": R4_SHA,
            "web_manifest_sha256": WEB_SHA, "test_only_path": CHANGED_PATH, "test_only_patch_sha256": PATCH_SHA}


def check_runner(runner):
    expected = (R5 / "runner/surface-e2e").resolve(strict=True)
    require(runner.resolve(strict=True) == expected, "Expected the separately built R5 stock runner")
    manifest = expected.with_name("SHA256SUMS")
    entries = path_entries(manifest, allow_dot_prefix=True)
    require(entries.keys() == {"surface-e2e"} and expected.is_file()
            and not expected.is_symlink() and os.access(expected, os.X_OK), "Wrong R5 executable manifest")
    verify_entries(expected.parent, entries)
    return {"runner_sha256": entries["surface-e2e"], "runner_manifest_sha256": digest(manifest),
            "runner_build_log_sha256": digest(R5 / "runner-build.log")}


if __name__ == "__main__":
    print(check_source_and_web())
