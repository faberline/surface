#!/usr/bin/env python3
"""Run two separate native Material list replays against pinned inputs."""

import argparse
import json
from pathlib import Path
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
    inputs = [manifest, gallery, cli]
    hashes = [digest(path) for path in inputs]
    (out / "native-source-verified-before.txt").write_text(verify_sources(repo, manifest))
    replay = Path(__file__).with_name("verify-native.py").resolve(strict=True)
    for run in ("run-1", "run-2"):
        with (out / f"{run}.log").open("w") as log:
            result = subprocess.run(
                [sys.executable, replay, "--gallery", gallery, "--cli", cli,
                 "--source", hashes[0], "--out", out / run],
                stdout=log, stderr=subprocess.STDOUT,
            )
        print((out / f"{run}.log").read_text(), end="", flush=True)
        result.check_returncode()
        require(hashes == [digest(path) for path in inputs], "Pinned input changed during replay")
        (out / f"native-source-verified-after-{run}.txt").write_text(verify_sources(repo, manifest))

    names = sorted(path.name for path in (out / "run-1").iterdir())
    require(names == sorted(path.name for path in (out / "run-2").iterdir()), "Replay outputs differ")
    require(set(names) == {
        "native-list-report.json", "segmented-light.png", "segmented-focused.png",
        "segmented-wrap-selected.png", "standard-dark-action.png", "baseline-light-passive.png",
        "segmented-compact-dark-rtl.png", "segmented-short-dark-rtl.png",
    }, "Missing report or screenshot")
    receipts = {}
    for name in names:
        first = out / "run-1" / name
        second = out / "run-2" / name
        require(first.read_bytes() == second.read_bytes(), f"Replay output differs for {name}")
        receipts[name] = digest(first)
    (out / "native-repeat.json").write_text(json.dumps({
        "verified_inputs": {"source_manifest": hashes[0], "gallery": hashes[1], "cli": hashes[2]},
        "independent_replays": 2,
        "byte_identical_outputs": receipts,
    }, indent=2) + "\n")
    print(f"PASS exact replay match for {len(receipts)} files", flush=True)


if __name__ == "__main__":
    main()
