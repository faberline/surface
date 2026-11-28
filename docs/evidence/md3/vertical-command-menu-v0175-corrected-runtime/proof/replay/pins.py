#!/usr/bin/env python3
"""Independently bind a corrected v0.1.75 build to the exact recovered source and bytes."""

import argparse
import os
from pathlib import Path
import tomllib

from evidence_common import data, digest, json_file, path_entries, require, source_tree, verify_entries

SOURCE_SHA = "8a2037d8b5fb11afcb226f5b27b41656d7c3dd428b09db9c4aaf2d711cacf2d0"
SOURCE_TREE = "8046e25a630284411e1ebf9f9d16d1004b02651b"
PATCH_SHA = "4f1fb9d57083d7f36b7eec8900e657c1db42f182c80bd7351b74444c63a69de4"
VERSION = "0.1.75"
CANONICAL_SOURCE = Path("/tmp/surface-md3-disabled-menu-click.qfrcXZ1r/source")
NATIVE = {"surface-material-gallery", "surface-devtools"}
RUNNERS = {"surface-e2e"}
BUILD_LOGS = ("native-build.log", "runner-build.log", "web-build.log", "toolchain.txt")


def inventory(root):
    items = set()
    for path in root.rglob("*"):
        require(not path.is_symlink(), f"Unexpected link: {path}")
        if path.is_file():
            items.add(path.relative_to(root).as_posix())
    return items


def check_source(candidate):
    manifest = candidate / "source.sha256"
    source = CANONICAL_SOURCE
    require(digest(manifest) == SOURCE_SHA, "Source manifest SHA-256 differs")
    require((candidate / "source.tree").read_text().strip() == SOURCE_TREE,
            "Candidate Git tree pin differs")
    require(digest(candidate / "disabled-menu.patch") == PATCH_SHA,
            "Recovered source patch SHA-256 differs")
    entries = path_entries(manifest)
    require(len(entries) == 698 and inventory(source) == entries.keys(),
            "Exact recovered source inventory must contain 698 files")
    verify_entries(source, entries)
    require(source_tree(source, entries) == SOURCE_TREE, "Recomputed Git tree differs")
    package = tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]
    require(package["version"] == VERSION, "Source version differs")
    return {"version": VERSION, "tree": SOURCE_TREE, "source_sha256": SOURCE_SHA,
            "patch_sha256": PATCH_SHA, "source_file_count": len(entries)}


def web_members(gallery):
    web = data(gallery / "surface-web.json")
    require({key: web[key] for key in ("app", "package", "profile", "cargo_profile", "rustc_target", "features")}
            == {"app": "material-gallery", "package": "surface-material-gallery", "profile": "release",
                "cargo_profile": "wasm-release", "rustc_target": "wasm32-unknown-unknown", "features": []},
            "Built web profile or features differ")
    names = {item["path"] for item in web["files"]}
    require(names == {web["entry"], web["js"], web["wasm"]} and web["entry"] == "index.html"
            and web["js"].endswith(".js") and web["wasm"].endswith(".wasm"), "Unexpected web asset names")
    members = names | {"surface-web.json"}
    require(inventory(gallery) == members, "Built gallery file inventory differs")
    for item in web["files"]:
        path = gallery / item["path"]
        require(path.stat().st_size == item["bytes"] and digest(path) == item["sha256"],
                f"Built asset differs from surface-web.json: {path}")
    with (gallery / web["wasm"]).open("rb") as payload:
        require(payload.read(8) == b"\x00asm\x01\x00\x00\x00", "Invalid Wasm module header")
    return members


def write_manifest(root, manifest, names):
    with manifest.open("x") as destination:
        for name in sorted(names):
            destination.write(f"{digest(root / name)}  {name}\n")


def build_record(out, source_record, *, seal=False):
    native, runners, gallery = out / "native", out / "runners", out / "web/material-gallery"
    scopes = [
        (native, native / "SHA256SUMS", NATIVE),
        (runners, runners / "SHA256SUMS", RUNNERS),
        (gallery, out / "web/SHA256SUMS", web_members(gallery)),
    ]
    if seal:
        for root, manifest, names in scopes:
            write_manifest(root, manifest, names)

    pins = {}
    for root, manifest, names in scopes:
        entries = path_entries(manifest)
        require(entries.keys() == names, f"Different manifest inventory: {manifest}")
        verify_entries(root, entries)
        pins[manifest.relative_to(out).as_posix()] = digest(manifest)
    for root, names in ((native, NATIVE), (runners, RUNNERS)):
        require(inventory(root) == names | {"SHA256SUMS"}, f"Unexpected binary output: {root}")
        require(all(os.access(root / name, os.X_OK) for name in names), f"Missing executable mode in {root}")
    logs = {name: digest(out / name) for name in BUILD_LOGS}
    return {"source": source_record, "artifact_manifests": pins, "logs": logs}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("source", "reserve", "seal", "verify"))
    parser.add_argument("candidate", type=Path)
    parser.add_argument("out", type=Path, nargs="?")
    args = parser.parse_args()
    if args.mode == "source":
        print(check_source(args.candidate.resolve(strict=True)))
        return

    require(args.out is not None and not args.out.is_symlink(), "Expected a private build output")
    out = args.out.resolve()
    temp = Path("/tmp").resolve()
    require(out.is_relative_to(temp) and out != temp and not out.is_relative_to(CANONICAL_SOURCE.parent.resolve()),
            "Output must be under /tmp and outside the source recovery tree")
    if args.mode == "reserve":
        out.mkdir(mode=0o700)
        print(out)
        return

    source_record = check_source(args.candidate.resolve(strict=True))
    report = out / "verified-build-inputs.json"
    if args.mode == "seal":
        require(not report.exists(), "The build already has a receipt")
        result = build_record(out, source_record, seal=True)
        json_file(report, result)
    else:
        expected = data(report)
        require(all((out / name).is_file() for name in (
            "native/SHA256SUMS", "runners/SHA256SUMS", "web/SHA256SUMS")),
            "Built artifact manifests are missing")
        require(build_record(out, source_record) == expected, "A verified build input changed")
    print(f"Corrected v{VERSION} source and build pins verified: {report}")


if __name__ == "__main__":
    main()
