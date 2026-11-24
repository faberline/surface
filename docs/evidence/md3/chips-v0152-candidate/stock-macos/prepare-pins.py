#!/usr/bin/env python3
"""Pin approved Chips source and build outputs without launching the suite."""

import argparse
from pathlib import Path
import re
from runpy import run_path
from shutil import copy2, copytree


ROOT = Path(__file__).resolve().parent
OBS = run_path(str(ROOT / "observe-chips.py"))
NATIVE = ("surface-material-gallery", "surface-devtools")
RUNNER = ("surface-e2e", "jet-bridge")
DIST = ("web/release/material-gallery", "web/release/showcase", "web/devtools/material-gallery")


def write_manifest(path, entries):
    path.write_text("".join(f"{checksum}  {name}\n" for name, checksum in sorted(entries.items())))


def source_inputs(args):
    digest = OBS["sha"]
    if not re.fullmatch(r"[a-f0-9]{64}", args.source_sha256) or digest(args.source_manifest) != args.source_sha256:
        raise ValueError("final supplied source manifest SHA-256 did not match")
    files = OBS["checksums"](args.source_manifest)
    if not {"Cargo.toml", "Cargo.lock", "e2e/src/material_chips.rs",
            "crates/surface-material/src/chip.rs"} <= files.keys():
        raise ValueError("source seal omits final Chips inputs")
    paths = {}
    for key, source in (("native", args.native_source), ("web", args.web_source)):
        origin = source.resolve(strict=True)
        OBS["check_source_version"](origin)
        OBS["verify_entries"](origin, files)
        paths[key] = str(origin)
    return files, paths


def marker_pins(args):
    markers = {key: path.resolve(strict=True) for key, path in (
        ("native", args.native_ready), ("web", args.web_ready),
    )}
    if len(set(markers.values())) != 2 or any(not path.is_file() for path in markers.values()):
        raise ValueError("provide distinct markers from the exact final native and Web builds")
    return {key: {"path": str(path), "sha256": OBS["sha"](path)} for key, path in markers.items()}


def prepare(args):
    markers = marker_pins(args)
    files, origins = source_inputs(args)
    native = args.native_binaries.resolve(strict=True)
    web = args.web_target.resolve(strict=True)
    native_hashes = OBS["checksums"](native / "SHA256SUMS")
    if set(native_hashes) != set(NATIVE):
        raise ValueError("exact Gallery and DevTools are required")
    OBS["verify_entries"](native, native_hashes)
    web_inputs = [web / "debug" / name for name in RUNNER]
    for component in DIST:
        dist = web / component
        web_inputs.extend(sorted(dist.iterdir()))
    if len(web_inputs) != 14 or any(not path.is_file() for path in web_inputs):
        raise ValueError("expected the two runner executables and 12 stock Web files")
    if args.wide_canvas_height < 1 or args.rtl_canvas_height < 1:
        raise ValueError("provide positive final CSS Canvas heights")
    bundle = ROOT / "bundle"
    if bundle.exists() or (ROOT / "run").exists() or (ROOT / "release-pin.json").exists():
        raise ValueError("pin in a fresh private scratch directory")
    bundle.mkdir()
    (bundle / "debug").mkdir()
    for name in NATIVE:
        copy2(native / name, bundle / "debug" / name)
    for name in RUNNER:
        copy2(web / "debug" / name, bundle / "debug" / name)
    for component in DIST:
        copytree(web / component, bundle / component)
    copy2(args.source_manifest, ROOT / "browser-source.sha256")
    copy2(native / "SHA256SUMS", ROOT / "native-binaries.sha256")
    pin = {"version": OBS["RELEASE"], "last_material_step": OBS["LAST_MATERIAL"],
           "source_sha256": args.source_sha256, "source_files": len(files),
           "wide_canvas_height": args.wide_canvas_height, "rtl_canvas_height": args.rtl_canvas_height}
    OBS["json_to"](ROOT / "release-pin.json", pin)
    OBS["json_to"](ROOT / "source-origins.json", origins)
    OBS["json_to"](ROOT / "build-markers.json", markers)
    binaries = {str(path.relative_to(bundle)): OBS["sha"](path) for path in bundle.rglob("*") if path.is_file()}
    if len(binaries) != 16:
        raise ValueError("unexpected executable bundle inventory")
    write_manifest(ROOT / "browser-binary-sha256.txt", binaries)
    write_manifest(ROOT / "sealed-inputs.sha256", {
        name: OBS["sha"](ROOT / name) for name in (
            "browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256",
            "source-origins.json", "release-pin.json", "build-markers.json", "observe-chips.py",
        )
    })
    runner = bundle / "debug/surface-e2e"
    runner_sha = binaries["debug/surface-e2e"]
    preflight = OBS["verify_inputs"](runner.resolve(), runner_sha, ROOT)
    OBS["json_to"](ROOT / "inputs-before.json", preflight)
    (ROOT / "approved-runner.sha256").write_text(runner_sha + "\n")
    print(f"Pinned v{pin['version']} with {len(files)} source files and 16 bundle files.")
    print(f"Exact runner SHA-256 {runner_sha}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("source-manifest", "native-source", "web-source", "native-binaries",
                 "native-ready", "web-ready", "web-target"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--source-sha256", required=True)
    parser.add_argument("--wide-canvas-height", type=int, required=True)
    parser.add_argument("--rtl-canvas-height", type=int, required=True)
    prepare(parser.parse_args())


if __name__ == "__main__":
    main()
