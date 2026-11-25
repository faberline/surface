"""Recheck the original stock receipts using the pinned underlying Gallery files."""

import argparse
from hashlib import file_digest
import json
from pathlib import Path
from runpy import run_path
import sys

HARNESS = "68bdc8273786ca5f3eea55637179d6c1d0f1de8a486dbc6209252a1271062f1e"
ORIGINS = "f77fbdecf352bcc23f95cfd53d1d3196cc425b146bdeb8fad98b46a0abe27c28"
VERIFIER = "bd4f3b7444c60c0e3fea3acbae29cc4b9f288cc6db46e12bb434eb8eeea4ea66"
RELEASE_DIR = Path("web/release/material-gallery")


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def replay(original, canonical):
    manifest = original / "harness.sha256"
    require(digest(manifest) == HARNESS, "original harness manifest changed")
    for line in manifest.read_text().splitlines():
        expected, name = line.split("  ", 1)
        path = original / name
        require(path.resolve().is_relative_to(original) and not path.is_symlink()
                and digest(path) == expected, f"original reviewed harness member changed: {name}")
    verifier_path = original / "verify-stock-receipt.py"
    origins_path = original / "bundle-origins.json"
    require(digest(verifier_path) == VERIFIER and digest(origins_path) == ORIGINS,
            "original verifier or bundle origin metadata changed")
    origins = json.loads(origins_path.read_text())
    bundle = original / "bundle" / RELEASE_DIR
    members = {Path(name).name: Path(target) for name, target in origins.items()
               if Path(name).parent == RELEASE_DIR}
    require(len(members) == 4 and members.keys() == {path.name for path in bundle.iterdir()}
            and members.keys() == {path.name for path in canonical.iterdir()},
            "release Gallery file inventory differs")
    for name, origin in members.items():
        link = bundle / name
        target = canonical / name
        require(link.is_symlink() and origin == target and link.resolve(strict=True) == target
                and target.is_file() and not target.is_symlink(), f"pinned Gallery origin differs: {name}")

    sys.path.insert(0, str(original))
    try:
        main = run_path(str(verifier_path))["main"]
        original_focused = main.__globals__["focused_receipt"]

        def use_regular_files(source_manifest, pins, supplied):
            require(supplied == bundle, "unrecognized path for the focused receipt's Gallery comparison")
            return original_focused(source_manifest, pins, canonical)

        # The main verifier still checks all 16 bundle links and original run receipts.
        # The focused helper then checks the same Gallery bytes at their real origin.
        main.__globals__["focused_receipt"] = use_regular_files
        main(original / "run")
    finally:
        sys.path.pop(0)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("original", type=Path)
    parser.add_argument("canonical", type=Path)
    args = parser.parse_args()
    replay(args.original.resolve(strict=True), args.canonical.resolve(strict=True))
