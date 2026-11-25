#!/usr/bin/env python3
"""Pin exact text field label motion source and build outputs without launching a suite."""

import argparse
from pathlib import Path
from runpy import run_path
from shutil import copy2, copytree

from verify_label_motion import focused_receipt


ROOT = Path(__file__).resolve().parent
OBS = run_path(str(ROOT / "observe-multiline.py"))
NATIVE = ("surface-material-gallery", "surface-devtools")
RUNNER = ("surface-e2e", "jet-bridge")
DIST = ("web/release/material-gallery", "web/release/showcase", "web/devtools/material-gallery")


def write_manifest(path, entries):
    path.write_text("".join(f"{checksum}  {name}\n" for name, checksum in sorted(entries.items())))


def source_inputs(args):
    digest = OBS["sha"]
    if args.source_sha256 != OBS["SOURCE_SHA"] or digest(args.source_manifest) != args.source_sha256:
        raise ValueError("final supplied source manifest SHA-256 did not match")
    files = OBS["checksums"](args.source_manifest)
    if len(files) != OBS["SOURCE_FILES"]:
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
            "e2e/src/material_text_field_label_motion.rs", "crates/surface-material/src/text_field_label_notch.rs"} <= files.keys():
        raise ValueError("source seal omits final label motion or retained inputs")
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
    focused = focused_receipt(args.source_manifest, web / "web/release/material-gallery")
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
           "source_sha256": args.source_sha256, "source_files": len(files), "source_tree": OBS["SOURCE_TREE"],
           "wide_canvas_height": args.wide_canvas_height, "rtl_canvas_height": args.rtl_canvas_height}
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
            "verify_label_motion.py", "verify-label-motion-focused-original.py", "focused-proof.json",
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
