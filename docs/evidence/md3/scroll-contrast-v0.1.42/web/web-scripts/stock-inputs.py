"""Verify the incoming source seal and pin every stock build input."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib

VERSION = "0.1.42"
SCOPES = (
    "Cargo.toml", "Cargo.lock", "rust-toolchain", "rust-toolchain.toml", "AGENTS.md",
    ".cargo", "crates", "e2e", "examples", "third-party", "xtask", "scripts", "docs",
)
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
}
LOCAL_PACKAGES = {
    "surface", "surface-app", "surface-devtools", "surface-devtools-protocol", "surface-e2e",
    "surface-material", "surface-material-gallery", "surface-platform", "surface-showcase",
    "surface-wgpu", "ui-runtime", "xtask",
}


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def git(source, *args):
    return subprocess.check_output(
        ["git", *args], cwd=source, env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"}
    )


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
    candidates = set()
    for args in (("--cached",), ("--others", "--exclude-standard")):
        raw = git(source, "ls-files", "-z", *args, "--", *SCOPES)
        candidates.update(path.decode() for path in raw.split(b"\0") if path)
    return sorted(path for path in candidates if (source / path).is_file())


def main():
    stage = Path(sys.argv[2]).resolve()
    phase = sys.argv[1]
    receipt = stage / "stock-stage.json"
    if phase == "prepare":
        source = Path(sys.argv[3]).resolve()
        head = sys.argv[4]
        upstream_seal = Path(sys.argv[5]).resolve()
        assert re.fullmatch(r"[0-9a-f]{40}", head), head
        assert git(source, "rev-parse", "HEAD").decode().strip() == head
        incoming = verify_seal(source, upstream_seal)
        seal = stage / "incoming-source.sha256"
        seal.write_bytes(upstream_seal.read_bytes())
        target = Path("/tmp/cargo-target") / f"surface-{stage.name}"
        receipt.write_text(json.dumps({
            "expected_workspace_version": VERSION,
            "source": str(source),
            "source_git_head": head,
            "incoming_source_manifest": str(seal),
            "incoming_source_manifest_sha256": sha(seal),
            "incoming_paths": len(incoming),
            "cargo_target": str(target),
            "release_target": "stock surface-e2e and stock Gallery",
        }, indent=2) + "\n")
        (stage / "source-git-status.txt").write_bytes(git(source, "status", "--porcelain=v1"))
    state = json.loads(receipt.read_text())
    source = Path(state["source"])
    assert git(source, "rev-parse", "HEAD").decode().strip() == state["source_git_head"]
    seal = Path(state["incoming_source_manifest"])
    assert sha(seal) == state["incoming_source_manifest_sha256"]
    verify_seal(source, seal)

    paths = source_files(source)
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
