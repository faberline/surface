#!/usr/bin/env python3
"""Run and compare two independent native Gallery replays with pinned inputs."""

import argparse
import json
from pathlib import Path
import runpy
import subprocess
import sys

from gallery_transport import require
from native_runtime import digest


def verify_sources(root, manifest):
    return subprocess.check_output(
        ["shasum", "-a", "256", "-c", str(manifest)],
        cwd=root, stderr=subprocess.STDOUT, text=True,
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", required=True, type=Path)
    parser.add_argument("--source-manifest", required=True, type=Path)
    parser.add_argument("--gallery", required=True, type=Path)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()

    repo = args.workspace.resolve(strict=True)
    manifest = args.source_manifest.resolve(strict=True)
    gallery = args.gallery.resolve(strict=True)
    cli = args.cli.resolve(strict=True)
    out = args.out.resolve()
    out.mkdir(parents=True)
    pinned = (manifest, gallery, cli)
    hashes = tuple(digest(path) for path in pinned)
    verifier = Path(__file__).with_name("verify-native.py").resolve(strict=True)
    contract = runpy.run_path(str(verifier))
    expected = contract["IMAGE_NAMES"] | {contract["REPORT_NAME"]}
    (out / "native-source-verified-before.txt").write_text(verify_sources(repo, manifest))
    for name in ("run-1", "run-2"):
        log_path = out / f"{name}.log"
        command = [sys.executable, "-B", verifier, "--gallery", gallery, "--cli", cli,
                   "--source", hashes[0], "--out", out / name,
                   "--contour-data", repo / "crates/surface-material/src/loading_indicator/contour_data.rs"]
        with log_path.open("w") as log:
            result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
        print(log_path.read_text(), end="", flush=True)
        result.check_returncode()
        require(hashes == tuple(digest(path) for path in pinned), "Pinned input changed during replay")
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
        "verified_inputs": {"source_manifest": hashes[0], "gallery": hashes[1], "cli": hashes[2]},
        "independent_replays": 2,
        "byte_identical_outputs": receipts,
    }, indent=2) + "\n")
    print(f"PASS exact replay match for {len(receipts)} files", flush=True)


if __name__ == "__main__":
    main()
