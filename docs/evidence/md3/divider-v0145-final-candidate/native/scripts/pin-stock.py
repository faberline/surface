"""Verify the sealed v0.1.45 stock bundle and copy only its native binaries."""

import argparse
from hashlib import sha256
import json
import os
from pathlib import Path
import re
from shutil import copy2, copyfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
VERSION = "0.1.45"
NATIVE = ("surface-material-gallery", "surface-devtools")


def digest(path):
    with path.open("rb") as data:
        return sha256_file(data)


def sha256_file(data):
    result = sha256()
    for block in iter(lambda: data.read(1024 * 1024), b""):
        result.update(block)
    return result.hexdigest()


def manifest(path):
    items = {}
    for line in path.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        assert re.fullmatch(r"[0-9a-f]{64}", checksum), line
        assert not Path(name).is_absolute() and ".." not in Path(name).parts and name not in items
        items[name] = checksum
    return items


def validate(directory, items):
    for name, checksum in items.items():
        path = directory / name
        assert path.is_file() and digest(path) == checksum, name


def verify(release, stage, expected_sha):
    release = release.resolve(strict=True)
    stage = stage.resolve(strict=True)
    source = release / "source"
    expected = release / "source.sha256"
    assert digest(expected) == expected_sha
    seal = manifest(stage / "sealed-inputs.sha256")
    assert set(seal) == {"stock-stage.json", "incoming-source.sha256", "browser-source.sha256",
                         "browser-binary-sha256.txt", "harness.sha256"}
    validate(stage, seal)
    build = json.loads((stage / "stock-stage.json").read_text())
    assert build["expected_workspace_version"] == VERSION and build["incoming_paths"] == 583
    assert build["incoming_source_manifest_sha256"] == expected_sha
    assert Path(build["source"]).resolve() == source and build["source_git_head"] is None
    assert Path(build["incoming_source_manifest"]).resolve() == stage / "incoming-source.sha256"
    for path in ("incoming-source.sha256", "browser-source.sha256", "built-input-sha256.txt"):
        assert (stage / path).read_bytes() == expected.read_bytes(), path
    source_items = manifest(expected)
    actual_paths = {str(path.relative_to(source)) for path in source.rglob("*") if path.is_file()}
    assert len(source_items) == 583 and set(source_items) == actual_paths
    validate(source, source_items)
    crate_manifest = tomllib.loads((source / "Cargo.toml").read_text())
    assert crate_manifest["workspace"]["package"]["version"] == VERSION
    harness = manifest(stage / "harness.sha256")
    assert {"stock-build.sh", "stock-run.sh", "stock-inputs.py"} <= harness.keys()
    validate(stage.parent, harness)
    bundle = stage / "bundle"
    binaries = manifest(stage / "browser-binary-sha256.txt")
    assert set(binaries) == {str(path.relative_to(bundle)) for path in bundle.rglob("*") if path.is_file()}
    validate(bundle, binaries)
    native = {name: binaries[f"debug/{name}"] for name in NATIVE}
    result = {"workspace_version": VERSION, "source_paths": len(source_items), "source_manifest_sha256": expected_sha,
              "stock_stage_seal_sha256": digest(stage / "sealed-inputs.sha256"),
              "checked_bundle_files": len(binaries), "checked_harness_files": len(harness),
              "sealed_stock_files": seal, "native_hashes": native}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("release", type=Path, help="Root with source and source.sha256")
    parser.add_argument("stage", type=Path, help="Sealed stock build stage")
    parser.add_argument("source_sha256")
    parser.add_argument("--pin", action="store_true", help="Copy native binaries to this scratch folder")
    args = parser.parse_args()
    result = verify(args.release, args.stage, args.source_sha256)
    native = result["native_hashes"]
    pins = ROOT / "pins"
    if args.pin:
        assert ROOT.is_relative_to(Path("/tmp").resolve()), "Use a private /tmp stage"
        pins.mkdir()
        receipts = ROOT / "inputs"
        receipts.mkdir()
        for name in native:
            copy2(args.stage / "bundle/debug" / name, pins / name)
            assert os.access(pins / name, os.X_OK)
        copyfile(args.release / "source.sha256", receipts / "source.sha256")
        for name in ("sealed-inputs.sha256", "harness.sha256", "browser-binary-sha256.txt"):
            copyfile(args.stage / name, receipts / name)
        (receipts / "native-binaries.sha256").write_text(
            "".join(f"{checksum}  {name}\n" for name, checksum in native.items())
        )
    validate(pins, native)
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
