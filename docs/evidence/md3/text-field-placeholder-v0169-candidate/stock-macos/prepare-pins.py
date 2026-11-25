#!/usr/bin/env python3
"""Pin exact text field placeholder source and build outputs without launching a suite."""

import argparse
from pathlib import Path
import re
from runpy import run_path
from shutil import copy2, copytree

from evidence_common import digest, path_entries, require, source_tree, verify_entries
from stock_config import LAST_MATERIAL, PIN_NAME, Q68, Q68_SOURCE, release_pin, reviewed
from verify_placeholder_stock import delta, focused_receipt


ROOT = Path(__file__).resolve().parent
OBS = run_path(str(ROOT / "observe-multiline.py"))
NATIVE = ("surface-material-gallery", "surface-devtools")
RUNNER = ("surface-e2e", "jet-bridge")
DIST = ("web/release/material-gallery", "web/release/showcase", "web/devtools/material-gallery")


def write_manifest(path, entries):
    path.write_text("".join(f"{checksum}  {name}\n" for name, checksum in sorted(entries.items())))


def source_inputs(args, pins):
    if digest(args.source_manifest) != pins["source_sha256"]:
        raise ValueError("final supplied source manifest SHA-256 did not match")
    files = path_entries(args.source_manifest)
    if len(files) != pins["source_files"]:
        raise ValueError("wrong exact source count")
    if not {"Cargo.toml", "Cargo.lock", "e2e/src/material_chips.rs",
            "e2e/src/material_multiline_text_field.rs", "crates/surface-platform/src/web_accessibility.rs",
            "examples/material-gallery/src/multiline_text_field_demo.rs",
            "crates/surface-app/tests/multiline_touch_scroll.rs",
            "crates/surface-app/src/editing_runtime.rs", "crates/surface-platform/src/macos_accessibility.rs",
            "e2e/src/material_outlined_text_field.rs", "e2e/src/material_filled_text_field.rs",
            "e2e/src/material_text_field_adornments.rs", "e2e/src/material_text_field_counter.rs",
            "crates/surface-material/src/text_field_counter.rs",
            "examples/material-gallery/src/text_field_counter_demo.rs",
            "crates/surface-material/src/text_field_adornments.rs",
            "examples/material-gallery/src/text_field_adornments_demo.rs",
            "crates/surface-material/src/outlined_text_field.rs",
            "crates/surface-app/src/editor_paint.rs",
            "crates/surface-platform/src/web_accessibility/live_status.rs",
            "examples/material-gallery/src/outlined_text_field_demo.rs",
            "examples/material-gallery/src/filled_text_field_demo.rs",
            "crates/surface-material/src/chip.rs",
            "crates/surface-material/src/icon_button.rs", "e2e/src/material_icon_buttons.rs",
            "crates/surface-app/src/slider.rs", "crates/surface-material/src/standard_slider.rs",
            "examples/material-gallery/src/slider_demo.rs",
            "examples/material-gallery/src/slider_forms_demo.rs",
            "examples/material-gallery/src/range_slider_demo.rs",
            "e2e/src/material_range_slider.rs",
            "crates/surface-material/src/standard_slider/range_slider.rs",
            "crates/surface-material/src/standard_slider/vertical.rs",
            "e2e/src/material_slider_forms.rs", "e2e/src/material_standard_slider.rs",
            "e2e/src/material_vertical_centered_slider.rs", "e2e/src/material_slider_inset.rs",
            "crates/surface-material/src/standard_slider/inset_icon.rs",
            "examples/material-gallery/src/vertical_centered_slider_demo.rs",
            "examples/material-gallery/src/slider_inset_demo.rs",
            "crates/surface-material/src/button.rs", "examples/material-gallery/src/button_demo.rs",
            "e2e/src/material_buttons.rs", "e2e/src/material_segmented.rs",
            "crates/surface-material/src/baseline_segmented_button.rs",
            "examples/material-gallery/src/segmented_demo.rs", "e2e/src/material.rs",
            "crates/surface-app/src/touch_scroll.rs", "crates/surface-app/src/scroll_control.rs",
            "crates/surface-app/src/frame_input.rs", "crates/surface-app/tests/touch_scroll.rs",
            "crates/surface-platform/src/web_pointer_capture.rs",
            "e2e/src/material_text_field_label_motion.rs", "crates/surface-material/src/text_field_label_notch.rs",
            "crates/surface-app/src/color_spring.rs", "crates/surface-app/src/placeholder_motion.rs",
            "examples/material-gallery/src/text_field_placeholder_demo.rs",
            "e2e/src/material_text_field_placeholder.rs"} <= files.keys():
        raise ValueError("source seal omits final placeholder or retained inputs")
    base = Q68 / "source.sha256"
    require(digest(base) == Q68_SOURCE and digest(args.patch) == pins["patch_sha256"], "Wrong base or original patch pin")
    paths = re.findall(r"^diff --git a/(\S+) b/\S+$", args.patch.read_text(), re.M)
    require(len(paths) == len(set(paths)) and sorted(paths) == delta(path_entries(base), files),
            "Original patch paths differ from the approved source delta")
    paths = {}
    for key, source in (("native", args.native_source), ("web", args.web_source)):
        origin = source.resolve(strict=True)
        OBS["check_source_version"](origin)
        actual = {item.relative_to(origin).as_posix() for item in origin.rglob("*")
                  if item.is_file() and item.relative_to(origin).parts[0] != ".git"}
        require(actual == set(files), f"{key} source contains unpinned files")
        verify_entries(origin, files)
        require(source_tree(origin, files) == pins["source_tree"], f"{key} source tree differs")
        source = (origin / "e2e/src/material.rs").read_text()
        steps = source.split("pub const STEPS:", 1)[1].split("];", 1)[0]
        require([int(step) for step in re.findall(r'"M(\d+)"', steps)]
                == [i for i in range(1, LAST_MATERIAL + 1) if i != 23],
                f"{key} stock registry lost an ordered Material step")
        paths[key] = str(origin)
    return files, paths


def marker_pins(args, pins):
    markers = {key: path.resolve(strict=True) for key, path in (
        ("native", args.native_ready), ("web", args.web_ready),
    )}
    if len(set(markers.values())) != 2 or any(not path.is_file() or path.read_text() != pins["source_tree"] + "\n"
                                              for path in markers.values()):
        raise ValueError("provide distinct markers from the exact final native and Web builds")
    return {key: {"path": str(path), "sha256": OBS["sha"](path)} for key, path in markers.items()}


def prepare(args):
    pins = reviewed(args.pins, args.pins_sha256)
    markers = marker_pins(args, pins)
    files, origins = source_inputs(args, pins)
    native = args.native_binaries.resolve(strict=True)
    web = args.web_target.resolve(strict=True)
    require(digest(native / "SHA256SUMS") == pins["native_manifest_sha256"], "Wrong reviewed exact native pair")
    native_hashes = path_entries(native / "SHA256SUMS")
    if set(native_hashes) != set(NATIVE):
        raise ValueError("exact Gallery and DevTools are required")
    verify_entries(native, native_hashes)
    web_inputs = [web / "debug" / name for name in RUNNER]
    for component in DIST:
        dist = web / component
        web_inputs.extend(sorted(dist.iterdir()))
    if len(web_inputs) != 14 or any(not path.is_file() for path in web_inputs):
        raise ValueError("expected the two runner executables and 12 stock Web files")
    require(digest(web / "debug/surface-e2e") == pins["stock_runner_sha256"], "Wrong reviewed stock runner")
    focused = focused_receipt(args.source_manifest, pins, web / "web/release/material-gallery")
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
    copy2(args.pins, ROOT / PIN_NAME)
    copy2(args.patch, ROOT / "text-field-placeholder-v0169.patch")
    pin = release_pin(pins)
    OBS["json_to"](ROOT / "release-pin.json", pin)
    OBS["json_to"](ROOT / "source-origins.json", origins)
    OBS["json_to"](ROOT / "build-markers.json", markers)
    OBS["json_to"](ROOT / "focused-proof.json", focused)
    binaries = {str(path.relative_to(bundle)): OBS["sha"](path) for path in bundle.rglob("*") if path.is_file()}
    if len(binaries) != 16:
        raise ValueError("unexpected executable bundle inventory")
    write_manifest(ROOT / "browser-binary-sha256.txt", binaries)
    write_manifest(ROOT / "sealed-inputs.sha256", {
        name: OBS["sha"](ROOT / name) for name in (
            "browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256",
            "source-origins.json", "release-pin.json", "build-markers.json", "observe-multiline.py",
            "verify_forms_pixels.py", "verify_range_pixels.py", "verify_refinement_pixels.py",
            "verify_button_pixels.py", "verify_segmented_pixels.py", "verify_outlined_field.py", "verify_filled_field.py",
            "verify-stock-receipt.py", "verify_adorned_field.py", "verify_counter.py",
            "verify_multiline.py", "verify_textbox_values.py", "verify-focused-original.py",
            "verify_label_color.py", "label-color-oracle.py", "observe-focused-original.py", "focused-proof.json",
            PIN_NAME, "text-field-placeholder-v0169.patch", "evidence_common.py", "stock_config.py",
            "verify_placeholder_stock.py", "placeholder-proof/verify-placeholder.py",
            "placeholder-proof/prepare-focused.py", "placeholder-proof/observe-tabs.py",
            "placeholder-proof/verify-label-color-original.py", "placeholder-proof/pixel-oracle.py",
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
                 "native-ready", "web-ready", "web-target", "pins", "patch"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--pins-sha256", required=True)
    prepare(parser.parse_args())


if __name__ == "__main__":
    main()
