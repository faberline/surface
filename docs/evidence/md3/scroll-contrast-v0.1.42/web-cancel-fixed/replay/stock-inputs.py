"""Verify the incoming source seal and pin every stock build input."""

from hashlib import file_digest
import json
from pathlib import Path
import re
import sys
import tomllib

VERSION = "0.1.42"
SOURCE_DIGEST = "c99ed8ff89b0cee66a84732598ac2ba652802c2e52c9122c3c5bb27f250ccd4b"
SOURCE_COUNT = 549
REQUIRED = {
    "Cargo.toml", "Cargo.lock", "docs/provenance.json", "docs/material-coverage.json",
    "crates/surface/src/paint.rs", "crates/surface/src/paint/text_selection.rs",
    "crates/surface/tests/text_selection_paint.rs", "crates/surface-wgpu/src/lib.rs",
    "crates/surface-wgpu/tests/text_selection_gpu.rs", "crates/surface-app/src/display_contrast.rs",
    "crates/surface-app/src/frame_paint.rs", "crates/surface-app/tests/display_contrast.rs",
    "crates/surface-platform/src/web_contrast.rs", "crates/surface-platform/src/lib.rs",
    "crates/surface-material/src/tokens.rs", "crates/surface-material/tests/theme.rs",
    "examples/material-gallery/src/lib.rs", "examples/material-gallery/src/scroll_popup_demo.rs",
    "e2e/src/main.rs", "e2e/src/ime.rs", "e2e/src/material.rs", "e2e/src/material_web_contrast.rs",
    "e2e/src/material_web_contrast_png.rs", "e2e/src/material_web_contrast_selection.rs",
    "e2e/src/material_web_contrast_preedit_underline.rs",
    "crates/surface-platform/src/web_input.rs",
}
LOCAL_PACKAGES = {
    "surface", "surface-app", "surface-devtools", "surface-devtools-protocol", "surface-e2e",
    "surface-material", "surface-material-gallery", "surface-platform", "surface-showcase",
    "surface-wgpu", "ui-runtime", "xtask",
}


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def verify_seal(source, seal):
    paths = {}
    for line in seal.read_text().splitlines():
        parts = re.fullmatch(r"([0-9a-f]{64}) [ *](.+)", line)
        if not parts:
            raise ValueError(f"Malformed incoming SHA-256 line: {line!r}")
        digest, original = parts.groups()
        path = (source / original).resolve()
        if Path(original).is_absolute() or not path.is_relative_to(source):
            raise ValueError(f"Incoming seal contains a path outside the source: {original}")
        if not path.is_file() or sha(path) != digest:
            raise ValueError(f"Invalid incoming source digest for {original}")
        relative = path.relative_to(source).as_posix()
        if relative in paths:
            raise ValueError(f"Duplicate incoming source digest for {relative}")
        paths[relative] = digest
    if not paths:
        raise ValueError("The incoming source seal is empty")
    return paths


def source_files(source):
    return sorted(path.relative_to(source).as_posix()
                  for path in source.rglob("*") if path.is_file())


def main():
    stage = Path(sys.argv[2]).resolve()
    phase = sys.argv[1]
    receipt = stage / "stock-stage.json"
    if phase == "prepare":
        source = Path(sys.argv[3]).resolve()
        upstream_seal = Path(sys.argv[4]).resolve()
        assert sha(upstream_seal) == SOURCE_DIGEST
        incoming = verify_seal(source, upstream_seal)
        assert len(incoming) == SOURCE_COUNT
        seal = stage / "incoming-source.sha256"
        seal.write_bytes(upstream_seal.read_bytes())
        target = Path("/tmp/cargo-target") / f"surface-{stage.name}"
        receipt.write_text(json.dumps({
            "expected_workspace_version": VERSION,
            "source": str(source),
            "source_git_head": None,
            "source_identity_kind": "sealed source export",
            "incoming_source_manifest": str(seal),
            "incoming_source_manifest_sha256": sha(seal),
            "incoming_paths": len(incoming),
            "cargo_target": str(target),
            "release_target": "stock surface-e2e and stock Gallery",
        }, indent=2) + "\n")
    state = json.loads(receipt.read_text())
    source = Path(state["source"])
    seal = Path(state["incoming_source_manifest"])
    assert sha(seal) == state["incoming_source_manifest_sha256"]
    incoming = verify_seal(source, seal)

    paths = source_files(source)
    assert set(paths) == set(incoming), {
        "unexpected": sorted(set(paths) - set(incoming)), "missing": sorted(set(incoming) - set(paths))
    }
    missing = REQUIRED - set(paths)
    assert not missing, f"Missing release inputs: {sorted(missing)}"
    cargo = tomllib.loads((source / "Cargo.toml").read_text())
    lock = tomllib.loads((source / "Cargo.lock").read_text())
    assert cargo["workspace"]["package"]["version"] == VERSION
    local_versions = {
        item["name"]: item["version"]
        for item in lock["package"]
        if "source" not in item and item["name"] in LOCAL_PACKAGES
    }
    assert local_versions == {name: VERSION for name in LOCAL_PACKAGES}, local_versions

    steps_source = (source / "e2e/src/material.rs").read_text()
    match = re.search(r"pub const STEPS: &\[&str\] = &\[(.*?)\];", steps_source, re.S)
    steps = re.findall(r'"(M\d+)"', match.group(1)) if match else []
    assert steps == [f"M{n}" for n in range(1, 248) if n != 23], steps
    ime_source = (source / "e2e/src/ime.rs").read_text()
    ime_match = re.search(r"pub const STEPS: &\[&str\] = &\[(.*?)\];", ime_source, re.S)
    ime_steps = re.findall(r'"(I\d+)"', ime_match.group(1)) if ime_match else []
    assert ime_steps == [f"I{n}" for n in range(1, 21)], ime_steps
    manifest = "".join(f"{sha(source / name)}  {name}\n" for name in paths)
    if phase == "prepare":
        (stage / "browser-source.sha256").write_text(manifest)
        (stage / "source-paths.txt").write_text("".join(f"{name}\n" for name in paths))
    else:
        output = Path(sys.argv[3]).resolve()
        output.write_text(manifest)
        assert (stage / "browser-source.sha256").read_text() == manifest, "Stock source changed"


if __name__ == "__main__":
    main()
