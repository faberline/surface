#!/usr/bin/env python3
"""Independently bind a v0.1.76 build to the exact recovered source and bytes."""

import argparse
import os
from pathlib import Path
import re
import tomllib

from evidence_common import data, digest, json_file, path_entries, require, source_tree, verify_entries

SOURCE_SHA = "a9aa38ee7ab64746471491ccfd6cd7df0132e8f30254d536a6fdd3fc95e10eb8"
SOURCE_TREE = "37e4454bc7d3f63f7111c8264d4e59e451c8ef9e"
BASE_SHA = "b1b3dea6427bb8bdd5debdd2715d3956e8ac4549a03a70880aa7fe12aabc700f"
BASE_TREE = "b3e62f07a03b08f1741de8695fe845d3b771f11d"
PATCH_SET_SHA = "54696fc9387a2248948771c2682a98cc7d40a742de9c4098d0846a6dee2a0c18"
CHANGES_SHA = "25db5df536e81b926bbba66180effd37ac0b2c90b770ac6b8f3af4b4af01492d"
CANONICAL_SOURCE = Path("/tmp/surface-md3-choice-menu-v0176-corrected.2R544r7F/source")
BASE_SOURCE = Path("/tmp/surface-md3-choice-menu-v0176-recovered.IhgvpwHQ/exact-candidate/source")
VERSION = "0.1.76"
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
    require(digest(manifest) == SOURCE_SHA, "Corrected source manifest SHA-256 differs")
    require((candidate / "source.tree").read_text().strip() == SOURCE_TREE, "Corrected tree pin differs")
    require(digest(candidate / "base.sha256") == BASE_SHA, "Original R3 manifest differs")
    patch_manifest = candidate / "patches.sha256"
    changed_file = candidate / "changed-paths.txt"
    require(digest(patch_manifest) == PATCH_SET_SHA and digest(changed_file) == CHANGES_SHA,
            "Reviewed patch and changed-paths manifests differ")
    patches, touched = {}, set()
    for line in patch_manifest.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        patch = candidate / "patches" / Path(name).name
        require(digest(patch) == checksum, f"Reviewed patch bytes differ: {patch.name}")
        patches[patch.name] = checksum
        touched.update(re.findall(r"^\+\+\+ b/(\S+)$", patch.read_text(), re.M))
    before, entries = path_entries(candidate / "base.sha256"), path_entries(manifest)
    changed = sorted(name for name in before.keys() | entries.keys() if before.get(name) != entries.get(name))
    require(len(before) == 702 and len(entries) == 703 and inventory(source) == entries.keys()
            and changed == changed_file.read_text().splitlines() and set(changed) == touched,
            "Corrected inventory or seven-path patch delta differs")
    verify_entries(source, entries)
    require(source_tree(source, entries) == SOURCE_TREE, "Recomputed corrected Git tree differs")
    require(source_tree(source, before, overrides={
        name: BASE_SOURCE / name for name in changed if name in before
    }) == BASE_TREE, "Recomputed original R3 tree differs")
    package = tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]
    require(package["version"] == VERSION, "Source version differs")
    return {"version": VERSION, "tree": SOURCE_TREE, "source_sha256": SOURCE_SHA, "source_file_count": len(entries),
            "base_tree": BASE_TREE, "base_source_sha256": BASE_SHA, "patch_set_sha256": PATCH_SET_SHA,
            "patches": patches, "changed_paths": changed}


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
