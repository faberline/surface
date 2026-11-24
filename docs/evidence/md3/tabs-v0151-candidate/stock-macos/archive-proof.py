#!/usr/bin/env python3
"""Seal the raw Tabs run or recheck a sealed copy without UI or network."""

import argparse
import json
from pathlib import Path
from runpy import run_path
from shutil import copy2, copytree


ROOT = Path(__file__).resolve().parent
CHECK = run_path(str(ROOT / "verify-existing.py"))
OBS = CHECK["OBS"]
verify_existing = CHECK["existing"]
digest = CHECK["digest"]
require = CHECK["require"]
canonical = lambda payload: json.loads(json.dumps(payload))
INPUTS = ("README.md", "browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256",
          "source-origins.json", "release-pin.json", "build-markers.json", "sealed-inputs.sha256",
          "inputs-before.json", "approved-runner.sha256", "observe-tabs.py", "prepare-pins.py",
          "verify-existing.py", "archive-proof.py")


def contents(directory):
    return sorted(path for path in directory.rglob("*") if path.is_file() and path != directory / "SHA256SUMS")


def verify(archive):
    seal = archive / "SHA256SUMS"
    entries = OBS["checksums"](seal)
    require(set(entries) == {str(path.relative_to(archive)) for path in contents(archive)},
            "archive file inventory differs")
    OBS["verify_entries"](archive, entries)
    verdict = verify_existing(archive / "run", archive)
    require(canonical(verdict) == OBS["read_json"](archive / "verified-tabs-offline.json"),
            "archived verification receipt differs")
    return {"files": len(entries), "manifest_sha256": digest(seal), "checks": verdict["total_checks"],
            "source_sha256": verdict["release"]["source_sha256"],
            "window_receipt_sha256": verdict["window_receipt_sha256"]}


def seal(run, archive):
    verdict = verify_existing(run, ROOT)
    require(archive.parent.resolve(strict=True).is_relative_to(Path("/tmp").resolve()),
            "archive path must be inside /tmp")
    archive.mkdir()
    for name in INPUTS:
        copy2(ROOT / name, archive / name)
    copytree(ROOT / "retained", archive / "retained")
    target = archive / "run"
    target.mkdir()
    for name in ("headless-window-observation.json", "invocation.json", "runner.log",
                 "inputs-after.json", "verified-tabs-stock.json"):
        copy2(run / name, target / name)
    copytree(run / "artifacts", target / "artifacts")
    copytree(run / "guard-ledger", target / "guard-ledger")
    OBS["json_to"](archive / "verified-tabs-offline.json", verdict)
    (archive / "SHA256SUMS").write_text("".join(
        f"{digest(path)}  {path.relative_to(archive)}\n" for path in contents(archive)
    ))
    return verify(archive)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seal", type=Path, help="write a new evidence directory")
    parser.add_argument("--verify", type=Path, help="recheck a sealed evidence directory")
    parser.add_argument("--run", type=Path, default=ROOT / "run")
    args = parser.parse_args()
    if (args.seal is None) == (args.verify is None):
        parser.error("Specify exactly one of --seal and --verify.")
    result = seal(args.run, args.seal) if args.seal else verify(args.verify)
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
