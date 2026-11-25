#!/usr/bin/env python3
"""Recheck native placeholder proof and original color pixels without launching a Gallery."""

import argparse
import json
from pathlib import Path
from runpy import run_path
import sys

from gallery_transport import require
from native_runtime import digest

SOURCE = run_path(str(Path(__file__).with_name("replay-placeholder.py")))


def verify(root, manifest_sha, source_sha, binary_sha):
    manifest = SOURCE["pins"](root, root / "SHA256SUMS", manifest_sha)
    name = SOURCE["REPORT"]
    report_path = root / name
    report = json.loads(report_path.read_text())
    require(SOURCE["SHA"].fullmatch(source_sha) and SOURCE["SHA"].fullmatch(binary_sha)
            and report["schema"] == SOURCE["SCHEMA"] and report["scope"] == SOURCE["SCOPE"]
            and report["source"]["version"] == "0.1.69"
            and report["source"]["manifest_sha256"] == source_sha
            and report["source"]["files"] >= len(SOURCE["GATES"])
            and report["binary_manifest_sha256"] == binary_sha
            and set(report["binaries"]) == {"surface-material-gallery", "surface-devtools"}
            and all(SOURCE["SHA"].fullmatch(value) for value in report["binaries"].values()),
            "Wrong pinned release input metadata")
    replays, processes = report["replays"], report["processes"]
    require(report["transports"] == SOURCE["TRANSPORTS"] and len(replays) == len(processes) == 2
            and replays[0] == replays[1], "Original screenshot transport proof or replay bytes differ")
    scopes = SOURCE["SCOPES"]
    appearances = SOURCE["APPEARANCES"]
    inventory, pids, windows = {name}, set(), 0
    for index, (replay, samples) in enumerate(zip(replays, processes, strict=True), 1):
        require(set(replay) == set(samples) == set(scopes), "Missing actual native viewport")
        for scope, viewport in scopes.items():
            state = replay[scope]
            process = samples[scope]
            observations = process["visible_windows"]
            require(state["viewport"] == list(viewport) and set(state["appearances"]) == set(appearances)
                    and state["restored_focus"] == SOURCE["SHOW"]
                    and type(process["pid"]) is int and process["pid"] > 0 and process["pid"] not in pids
                    and process["reaped"] is True and set(observations) == SOURCE["phases"](scope)
                    and set(observations.values()) == {0}, "Wrong native reaping or zero-window observations")
            windows += len(observations)
            pids.add(process["pid"])
            directory = root / f"replay-{index}"
            for appearance in appearances:
                scene = state["appearances"][appearance]
                images = scene["captures"]
                actual = SOURCE["verify_one_appearance"](directory, images, scope, appearance)
                require(actual == scene["pixel_checks"], "Original PNGs no longer reproduce endpoint differences")
                for capture in images.values():
                    relative = f"replay-{index}/{capture['name']}"
                    require(relative not in inventory and manifest[relative] == capture["sha256"],
                            "Repeated frame or wrong sealed original")
                    inventory.add(relative)
    require(set(manifest) == inventory and len(inventory) == 41 and windows == 92 and len(pids) == 4,
            "Capture, process or window sample count changed")
    actual_output = {path.relative_to(root).as_posix() for path in root.rglob("*")
                     if path.is_file() and path != root / "SHA256SUMS"}
    require(actual_output == inventory, "Unsealed or unexpected native output")
    return {"source_manifest_sha256": source_sha, "binary_manifest_sha256": binary_sha,
            "replay_manifest_sha256": manifest_sha, "report_sha256": digest(report_path),
            "repeats": 2, "headless_profiles_per_replay": 2, "original_pngs": 40,
            "transports": report["transports"], "visible_owned_windows": 0,
            "reaped_gallery_processes": len(pids), "observed_window_samples": windows,
            "scope": report["scope"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--manifest-sha", required=True)
    parser.add_argument("--source-manifest-sha", required=True)
    parser.add_argument("--binary-manifest-sha", required=True)
    args = parser.parse_args()
    result = verify(args.root.resolve(strict=True), args.manifest_sha, args.source_manifest_sha,
                    args.binary_manifest_sha)
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, ValueError, OSError, RuntimeError) as error:
        sys.exit(f"Stored native placeholder proof stopped. {error}")
