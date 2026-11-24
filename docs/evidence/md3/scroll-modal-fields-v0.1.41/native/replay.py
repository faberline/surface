#!/usr/bin/env python3
"""Run and compare two independent native Gallery replays with pinned inputs."""

import argparse
import json
from pathlib import Path
import re
import runpy
import subprocess
import sys

from gallery_transport import require
from native_runtime import digest


REQUIRED_SOURCES = {
    "Cargo.lock",
    "Cargo.toml",
    "crates/surface-material/src/lib.rs",
    "crates/surface-material/src/scroll_field.rs",
    "crates/surface-material/src/scroll_field/modal.rs",
    "examples/material-gallery/src/lib.rs",
    "examples/material-gallery/src/scroll_modal_demo.rs",
}


def checksum_entries(manifest):
    for number, line in enumerate(manifest.read_text().splitlines(), 1):
        hash_value, space, name = line.partition(" ")
        require(space and re.fullmatch(r"[0-9a-fA-F]{64}", hash_value) and name.strip(),
                f"Invalid SHA-256 entry at {manifest.name}:{number}")
        yield hash_value.lower(), name.lstrip().removeprefix("*").removeprefix("./")


def verify_binary(binary_manifest, binary, expected_name):
    entries = [checksum for checksum, name in checksum_entries(binary_manifest)
               if Path(name).name == expected_name]
    require(len(entries) == 1, f"Expected one {expected_name} hash in {binary_manifest.name}")
    checksum = digest(binary)
    require(checksum == entries[0], f"{expected_name} does not match the exact binary pin")
    return checksum


def verify_sources(root, manifest):
    entries = [name for _, name in checksum_entries(manifest)]
    missing = REQUIRED_SOURCES.difference(entries)
    require(not missing, f"The source pin is missing {', '.join(sorted(missing))}")
    return subprocess.check_output(
        ["shasum", "-a", "256", "-c", str(manifest)],
        cwd=root, stderr=subprocess.STDOUT, text=True,
    )


def main():
    parser = argparse.ArgumentParser()
    for name in ("workspace", "source-manifest", "binary-manifest", "gallery", "cli"):
        parser.add_argument(f"--{name}", required=True, type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--verify-only", action="store_true", help="Check exact pins without launching Gallery")
    args = parser.parse_args()
    if not args.verify_only and args.out is None:
        parser.error("--out is required for replay")

    repo = args.workspace.resolve(strict=True)
    manifest = args.source_manifest.resolve(strict=True)
    binary_manifest = args.binary_manifest.resolve(strict=True)
    gallery = args.gallery.resolve(strict=True)
    cli = args.cli.resolve(strict=True)
    inputs = {
        "source_manifest": digest(manifest),
        "binary_manifest": digest(binary_manifest),
        "gallery": verify_binary(binary_manifest, gallery, "surface-material-gallery"),
        "cli": verify_binary(binary_manifest, cli, "surface-devtools"),
    }
    verified_source = verify_sources(repo, manifest)
    if args.verify_only:
        print(json.dumps({"verified_inputs": inputs, "source_files": len(verified_source.splitlines())}, indent=2))
        return

    out = args.out.resolve()
    out.mkdir(parents=True)
    verifier = Path(__file__).with_name("verify-native.py").resolve(strict=True)
    scripts = sorted(verifier.parent.glob("*.py"))
    script_hashes = {path: digest(path) for path in scripts}
    pinned = {
        manifest: inputs["source_manifest"],
        binary_manifest: inputs["binary_manifest"],
        gallery: inputs["gallery"],
        cli: inputs["cli"],
        **script_hashes,
    }
    contract = runpy.run_path(str(verifier))
    expected = contract["IMAGE_NAMES"] | {contract["REPORT_NAME"]}
    (out / "native-source-verified-before.txt").write_text(verified_source)
    for name in ("run-1", "run-2"):
        log_path = out / f"{name}.log"
        command = [sys.executable, "-B", verifier, "--gallery", gallery, "--cli", cli,
                   "--source", inputs["source_manifest"], "--out", out / name]
        with log_path.open("w") as log:
            result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
        print(log_path.read_text(), end="", flush=True)
        result.check_returncode()
        require(all(digest(path) == expected_hash for path, expected_hash in pinned.items()),
                "Pinned input changed during replay")
        receipt = out / f"native-source-verified-after-{name}.txt"
        receipt.write_text(verify_sources(repo, manifest))

    first, second = (out / name for name in ("run-1", "run-2"))
    for directory in (first, second):
        require({path.name for path in directory.iterdir()} == expected,
                f"Replay output differs from the expected PNG/report set: {directory.name}")
    receipts = {}
    for name in sorted(expected):
        require((first / name).read_bytes() == (second / name).read_bytes(),
                f"Independent replays produced different bytes for {name}")
        receipts[name] = digest(first / name)
    (out / "native-repeat.json").write_text(json.dumps({
        "verified_inputs": inputs,
        "replay_scripts": {path.name: expected_hash for path, expected_hash in script_hashes.items()},
        "independent_replays": 2,
        "byte_identical_outputs": receipts,
    }, indent=2) + "\n")
    print(f"PASS exact replay match for {len(receipts)} files", flush=True)


if __name__ == "__main__":
    main()
