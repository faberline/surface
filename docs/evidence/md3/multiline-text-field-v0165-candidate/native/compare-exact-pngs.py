#!/usr/bin/env python3
"""Decode and compare exact and development native renders without starting the host."""

import argparse
from hashlib import sha256
from io import BytesIO
import json
from pathlib import Path

from PIL import Image

REPORT = "native-multiline-text-field-report.json"
DIMENSIONS = {"wide": (960, 900), "compact": (320, 900)}


def load(root):
    path = root / REPORT
    report = json.loads(path.read_text())
    if len(report["replays"]) != 2:
        raise ValueError("Missing one of the two native repetitions.")
    images = {}
    for repeat, scenarios in enumerate(report["replays"], 1):
        for scenario, result in scenarios.items():
            for asset in result["captures"]:
                name = f"replay-{repeat}/{asset['name']}"
                payload = (root / name).read_bytes()
                checksum = sha256(payload).hexdigest()
                with Image.open(BytesIO(payload)) as image:
                    image.load()
                    if (image.format != "PNG" or image.mode != "RGBA"
                            or image.size != DIMENSIONS[scenario] or checksum != asset["sha256"]):
                        raise ValueError(f"Wrong original image or image dimensions: {name}")
                    images[name] = {"encoded_sha256": checksum, "rgba_sha256": sha256(image.tobytes()).hexdigest(),
                                    "size": list(image.size)}
    actual = {str(path.relative_to(root)) for path in root.glob("replay-*/*.png")}
    if len(images) != 14 or set(images) != actual:
        raise ValueError("Unexpected original PNG scope.")
    return report, images


def read_manifest(path):
    return {name: checksum for checksum, name in
            (line.split("  ", 1) for line in path.read_text().splitlines())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("development", type=Path)
    parser.add_argument("exact", type=Path)
    parser.add_argument("--development-source-manifest", type=Path, required=True)
    parser.add_argument("--exact-source-manifest", type=Path, required=True)
    args = parser.parse_args()
    prior, prior_images = load(args.development)
    exact, images = load(args.exact)
    if (prior["replays"] != exact["replays"] or prior["binaries"] != exact["binaries"]
            or prior["binary_manifest_sha256"] != exact["binary_manifest_sha256"] or prior_images != images):
        raise ValueError("Exact replay differs from the pinned development behavior, binaries or pixels.")
    before = read_manifest(args.development_source_manifest)
    after = read_manifest(args.exact_source_manifest)
    for receipt, manifest, entries in ((prior, args.development_source_manifest, before),
                                       (exact, args.exact_source_manifest, after)):
        if receipt["source"] != {"manifest_sha256": sha256(manifest.read_bytes()).hexdigest(), "files": len(entries)}:
            raise ValueError("Replay source receipt does not match the supplied manifest.")
    changed = {path: ("added" if path not in before else "removed" if path not in after else "changed")
               for path in sorted(before.keys() | after.keys()) if before.get(path) != after.get(path)}
    print(json.dumps({"comparison": "original PNG bytes and all decoded RGBA pixels",
                      "images_equal": len(images), "behavior_equal": True, "native_binaries_equal": True,
                      "source_changes": changed, "development_source": prior["source"],
                      "exact_source": exact["source"], "images": images}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
