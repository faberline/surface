#!/usr/bin/env python3
"""Seal separately built stock and probe inputs, then replay both twice."""

import argparse
import json
import os
from pathlib import Path
import re
import runpy
import shutil
import subprocess
import sys

from gallery_transport import require
from native_runtime import digest

SCRIPTS = Path(__file__).resolve().parent
OVERLAY = SCRIPTS.parent / "mac-headless-contrast-probe.patch"
OVERLAY_SHA256 = "08517367a5674b7e85825f4f56fa1f80c186f7095b1b220a0ecae6a85260f6a4"
OVERLAY_FILES = {"crates/surface-platform/src/headless.rs", "crates/surface-platform/src/motion.rs"}
GALLERY = "surface-material-gallery"
CLI = "surface-devtools"
STARTUP_MARKER = b"surface.test.macos-display-contrast-startup.v1"


def read_manifest(path):
    entries = {}
    for line in path.read_text().splitlines():
        checksum, marker, name = line.partition("  ")
        require(marker and re.fullmatch(r"[0-9a-f]{64}", checksum), "Invalid source SHA-256 line")
        item = Path(name)
        require(not item.is_absolute() and all(part not in ("..", ".git") for part in item.parts)
                and item.as_posix() not in entries and item.as_posix() != ".",
                f"Unsafe or duplicate input: {name!r}")
        entries[item.as_posix()] = checksum
    require(entries, "The source manifest is empty")
    return entries


def verify_tree(root, manifest):
    entries = read_manifest(manifest)
    for name, expected in entries.items():
        path = root / name
        require(path.resolve(strict=True).is_relative_to(root) and path.is_file(), f"Invalid source {name}")
        require(digest(path) == expected, f"Source hash mismatch: {name}")
    return entries


def copy_tree(root, output, source_manifest):
    for name in source_manifest:
        path = output / name
        path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(root / name, path)


def write_json(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")


def build(root, target, label, packages, binaries):
    env = os.environ.copy()
    env.update({
        "PATH": f"{Path.home()}/.cargo/bin:{Path.home()}/.local/bin:{env['PATH']}",
        "CARGO_HOME": env.get("CARGO_HOME", "/tmp/surface-md3-cargo"),
        "CARGO_BUILD_JOBS": "2", "CARGO_NET_OFFLINE": "true",
        "CARGO_TARGET_DIR": str(target),
        "DEVELOPER_DIR": "/Library/Developer/CommandLineTools",
        "SDKROOT": "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk",
    })
    command = ["cap", "cargo", "build", "--locked", *packages,
               "--features", "surface-material-gallery/devtools"]
    with (root.parent / f"{label}-build.log").open("w") as log:
        completed = subprocess.run(command, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
    completed.check_returncode()
    destination = root.parent / f"{label}-bin"
    destination.mkdir()
    for name in binaries:
        shutil.copy2(target / "debug" / name, destination / name)


def prepare(options):
    source = options.source.resolve(strict=True)
    manifest = options.source_manifest.resolve(strict=True)
    require(digest(manifest) == options.source_sha256, "Candidate source manifest hash mismatch")
    require(digest(OVERLAY) == OVERLAY_SHA256, "Unexpected probe overlay")
    entries = verify_tree(source, manifest)
    require(OVERLAY_FILES.union({"Cargo.toml", "Cargo.lock", "examples/material-gallery/src/lib.rs"})
            <= entries.keys(), "Candidate source pin omits required build inputs")
    require(re.search(r'^version = "0\.1\.42"$', (source / "Cargo.toml").read_text(), re.MULTILINE),
            "Expected the v0.1.42 release candidate")
    out = options.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Seal output must be under /tmp")
    out.mkdir(parents=True)
    stock, probe = (out / f"{name}-source" for name in ("stock", "probe"))
    copy_tree(source, stock, entries)
    copy_tree(stock, probe, entries)
    stock_manifest = out / "stock-source.sha256"
    shutil.copy2(manifest, stock_manifest)
    verify_tree(stock, stock_manifest)
    subprocess.run(["patch", "--batch", "--forward", "-p1", "-i", str(OVERLAY)], cwd=probe, check=True)
    probe_entries = {name: digest(probe / name) for name in entries}
    require({name for name in entries if entries[name] != probe_entries[name]} == OVERLAY_FILES,
            "The overlay did not change exactly its two authored files")
    probe_manifest = out / "probe-source.sha256"
    probe_manifest.write_text("".join(f"{checksum}  {name}\n" for name, checksum in probe_entries.items()))
    targets = Path("/tmp/cargo-target")
    probe_target = targets / f"surface-{out.parent.name}-{out.name}-{probe.name}"
    require(not probe_target.exists(), "Use a fresh target for the native probe")
    stock_stage = options.stock_stage.resolve(strict=True)
    incoming = stock_stage / "incoming-source.sha256"
    rehashed = stock_stage / "built-input-sha256.txt"
    binary_receipt = stock_stage / "browser-binary-sha256.txt"
    require(incoming.read_bytes() == manifest.read_bytes()
            and read_manifest(rehashed) == entries, "The stock source input receipt differs")
    original_receipt = digest(binary_receipt)
    fixed = read_manifest(binary_receipt)
    stock_bin = out / "stock-bin"
    stock_bin.mkdir()
    for name in (GALLERY, CLI):
        path = f"debug/{name}"
        executable = stock_stage / "bundle" / path
        require(digest(executable) == fixed[path], f"Stock binary hash mismatch: {name}")
        shutil.copy2(executable, stock_bin / name)
        require(digest(stock_bin / name) == fixed[path], f"Copied stock pin hash mismatch: {name}")
    print(f"Reusing verified stock copies, building exact diagnostic in {probe_target}", flush=True)
    build(probe, probe_target, "probe", ["-p", GALLERY], (GALLERY,))
    verify_tree(stock, stock_manifest)
    verify_tree(probe, probe_manifest)
    verify_tree(source, manifest)
    paths = ("stock-source.sha256", "probe-source.sha256", f"stock-bin/{GALLERY}",
             f"probe-bin/{GALLERY}", f"stock-bin/{CLI}")
    hashes = {path: digest(out / path) for path in paths}
    stock_exe = f"stock-bin/{GALLERY}"
    probe_exe = f"probe-bin/{GALLERY}"
    require(hashes[stock_exe] != hashes[probe_exe], "Stock and probe executable hashes match")
    require(STARTUP_MARKER not in (out / stock_exe).read_bytes()
            and STARTUP_MARKER in (out / probe_exe).read_bytes(), "Incorrect stock/probe build identity")
    require(digest(binary_receipt) == original_receipt and all(
        digest(stock_stage / "bundle" / f"debug/{name}") == fixed[f"debug/{name}"]
        for name in (GALLERY, CLI)), "Stock bundle changed")
    write_json(out / "native-inputs.json", {
        "release": "0.1.42", "candidate_head": options.head, "candidate_tree": options.tree,
        "original_stock_binary_manifest_sha256": original_receipt,
        "original_stock_postbuild_source_manifest_sha256": digest(rehashed),
        "overlay_sha256": digest(OVERLAY), "pinned_files": hashes,
    })
    print("PASS both builds and all source and executable pins", flush=True)


def replay(options):
    seal = options.sealed.resolve(strict=True)
    out = options.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Replay output must be under /tmp")
    out.mkdir(parents=True)
    input_receipt = seal / "native-inputs.json"
    identity = json.loads(input_receipt.read_text())
    require(identity["release"] == "0.1.42" and identity["overlay_sha256"] == digest(OVERLAY),
            "Wrong sealed release or overlay")
    pinned = {seal / name: expected for name, expected in identity["pinned_files"].items()}
    pinned.update({path: digest(path) for path in (*sorted(SCRIPTS.glob("*.py")), OVERLAY, input_receipt)})

    def verify():
        require(all(digest(path) == expected for path, expected in pinned.items()), "Pinned replay input changed")
        for name in ("stock", "probe"):
            verify_tree(seal / f"{name}-source", seal / f"{name}-source.sha256")

    verify()
    verifier = SCRIPTS / "verify-contrast.py"
    contract = runpy.run_path(str(verifier))
    expected = contract["IMAGE_NAMES"] | {contract["REPORT_NAME"]}
    for run in ("run-1", "run-2"):
        command = [sys.executable, "-B", verifier, "--stock-gallery", seal / "stock-bin" / GALLERY,
                   "--probe-gallery", seal / "probe-bin" / GALLERY, "--cli", seal / "stock-bin" / CLI,
                   "--out", out / run]
        with (out / f"{run}.log").open("w") as log:
            completed = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
        print((out / f"{run}.log").read_text(), end="", flush=True)
        completed.check_returncode()
        verify()

    first, second = (out / name for name in ("run-1", "run-2"))
    for directory in (first, second):
        require({path.name for path in directory.iterdir()} == expected, "Unexpected native replay files")
    receipts = {}
    for name in sorted(expected):
        require((first / name).read_bytes() == (second / name).read_bytes(),
                f"Separate native replays produced different bytes for {name}")
        receipts[name] = digest(first / name)
    write_json(out / "native-repeat.json", {
        "inputs": identity, "input_receipt_sha256": pinned[input_receipt], "independent_replays": 2,
        "scripts": {path.name: pinned[path] for path in sorted(SCRIPTS.glob("*.py"))},
        "byte_identical_outputs": receipts,
    })
    print(f"PASS exact repeat for {len(receipts)} files", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for kind, paths in (("prepare", ("source", "source-manifest", "stock-stage", "out")),
                        ("replay", ("sealed", "out"))):
        cmd = commands.add_parser(kind)
        for name in paths:
            cmd.add_argument(f"--{name}", required=True, type=Path)
        if kind == "prepare":
            cmd.add_argument("--source-sha256", required=True)
            cmd.add_argument("--head", required=True)
            cmd.add_argument("--tree", required=True)
    options = parser.parse_args()
    if options.command == "prepare":
        prepare(options)
    else:
        replay(options)


if __name__ == "__main__":
    main()
