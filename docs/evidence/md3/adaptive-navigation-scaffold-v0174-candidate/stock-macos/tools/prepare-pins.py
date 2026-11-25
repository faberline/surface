#!/usr/bin/env python3
"""Pin exact adaptive Navigation source and build outputs without launching a suite."""

import argparse
import json
import os
from hashlib import sha256
from pathlib import Path
import re
from runpy import run_path
from shutil import copy2

from evidence_common import data, digest, path_entries, require, source_tree, verify_entries
from stock_config import (
    ADAPTIVE_CASE, ADAPTIVE_DELTA, ADAPTIVE_LAUNCHER, ADAPTIVE_RECEIPT, ADAPTIVE_RUNNER,
    ADAPTIVE_STAGE, ADAPTIVE_VERIFIER, FINAL_COUNT, FINAL_PATCH, FINAL_SOURCE, FINAL_TREE,
    LAST_MATERIAL, PATCH_NAME, PIN_NAME, Q73_FINAL, release_pin, reviewed, validate_reviewed,
)
from verify_boundary_stock import delta
from verify_adaptive_navigation_stock import focused_receipt


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
            "e2e/src/material_text_field_placeholder.rs", "e2e/src/material_text_field_boundary_motion.rs",
            "e2e/src/material_text_field_disabled_contrast.rs",
            "e2e/src/material_text_field_system_pair.rs",
            "e2e/src/material_flexible_navigation_bar.rs", "e2e/src/material_horizontal_navigation_bar.rs",
            "crates/surface-material/src/flexible_navigation_bar.rs",
            "examples/material-gallery/src/flexible_navigation_bar_demo.rs", ADAPTIVE_CASE,
            "crates/surface-material/src/adaptive_navigation_scaffold.rs",
            "examples/material-gallery/src/adaptive_navigation_demo.rs"} <= files.keys():
        raise ValueError("source seal omits the Navigation case or retained inputs")
    base = ADAPTIVE_STAGE / "base.sha256"
    require(digest(base) == Q73_FINAL and digest(args.patch) == pins["patch_sha256"], "Wrong base or original patch pin")
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
    markers = {key: path for key, path in (
        ("native", args.native_ready), ("web", args.web_ready),
    )}
    if (len({path.resolve() for path in markers.values()}) != 2
            or any(not path.is_file() or path.is_symlink() or path.read_text() != pins["source_tree"] + "\n"
                   for path in markers.values())):
        raise ValueError("provide distinct markers from the exact final native and Web builds")
    return {key: {"path": str(path.resolve()), "sha256": OBS["sha"](path)} for key, path in markers.items()}



def canonical_dist(path, app, features):
    files = {entry.name for entry in path.iterdir()}
    build = data(path / "surface-web.json")
    named = [build[key] for key in ("entry", "js", "wasm")]
    require(len(files) == 4 and len(set(named)) == 3 and files == set(named) | {"surface-web.json"}
            and build["entry"] == "index.html" and build["js"].endswith(".js") and build["wasm"].endswith(".wasm")
            and build["app"] == app and build["profile"] == "release"
            and build["cargo_profile"] == "wasm-release" and build["rustc_target"] == "wasm32-unknown-unknown"
            and build["features"] == features, f"wrong canonical Web inventory or profile in {path}")
    descriptor = build["files"]
    hashes = {entry["path"]: entry["sha256"] for entry in descriptor}
    require(len(descriptor) == 3 and set(hashes) == set(named), f"wrong embedded manifest in {path}")
    verify_entries(path, hashes)
    require(all((path / entry["path"]).stat().st_size == entry["bytes"] for entry in descriptor),
            f"wrong embedded byte count in {path}")
    return sorted(path / name for name in files)


def canonical_build(args):
    native = args.native_binaries.resolve(strict=True)
    web = args.web_target.resolve(strict=True)
    native_hashes = path_entries(native / "SHA256SUMS")
    if set(native_hashes) != set(NATIVE):
        raise ValueError("exact Gallery and DevTools are required")
    verify_entries(native, native_hashes)
    origins_by_file = {f"debug/{name}": (native / name).resolve(strict=True) for name in NATIVE}
    origins_by_file.update({f"debug/{name}": web / "debug" / name for name in RUNNER})
    runners = web / "test-binaries.sha256"
    runner_hashes = path_entries(runners)
    require(set(runner_hashes) == {f"debug/{name}" for name in RUNNER}, "Wrong runner manifest inventory")
    verify_entries(web, runner_hashes)
    for component in DIST:
        dist = web / component
        features = ["devtools"] if "/devtools/" in component else []
        originals = canonical_dist(dist, dist.name, features)
        origins_by_file.update({f"{component}/{path.name}": path for path in originals})
    require(len(origins_by_file) == 16 and all(path.is_file() and not path.is_symlink()
            and path.resolve(strict=True) == path for path in origins_by_file.values()),
            "expected four originals executables and 12 regular stock Web files")
    require(all(os.access(path, os.X_OK) for name, path in origins_by_file.items() if name.startswith("debug/")),
            "All four exact stock inputs must be executable")
    binaries = {name: digest(path) for name, path in origins_by_file.items()}
    manifest_text = "".join(f"{checksum}  {name}\n" for name, checksum in sorted(binaries.items()))
    return native, web, runners, origins_by_file, binaries, manifest_text


def source_pins():
    return {"source_sha256": FINAL_SOURCE, "source_tree": FINAL_TREE, "source_files": FINAL_COUNT,
            "patch_sha256": FINAL_PATCH, "exact_vs_scoped_source_paths": ADAPTIVE_DELTA,
            "focused_tools_sha256": ADAPTIVE_LAUNCHER, "focused_verifier_sha256": ADAPTIVE_VERIFIER,
            "focused_receipt_sha256": ADAPTIVE_RECEIPT, "focused_runner_sha256": ADAPTIVE_RUNNER}


def build_pins(native, runners, manifest_text, binaries):
    return {"native_manifest_sha256": digest(native / "SHA256SUMS"),
            "stock_runner_manifest_sha256": digest(runners),
            "stock_assets_sha256": sha256(manifest_text.encode()).hexdigest(),
            "stock_runner_sha256": binaries["debug/surface-e2e"]}


def describe(args):
    pins = source_pins()
    marker_pins(args, pins)
    source_inputs(args, pins)
    native, web, runners, _, binaries, manifest_text = canonical_build(args)
    pins.update(build_pins(native, runners, manifest_text, binaries))
    validate_reviewed(pins)
    focused_receipt(args.source_manifest, pins, web / "web/release/material-gallery")
    return pins


def prepare(args):
    pins = reviewed(args.pins, args.pins_sha256)
    markers = marker_pins(args, pins)
    files, origins = source_inputs(args, pins)
    native, web, runners, origins_by_file, binaries, manifest_text = canonical_build(args)
    require(all(pins[key] == value for key, value in build_pins(native, runners, manifest_text, binaries).items()),
            "Actual canonical native or stock Web bundle differs from the reviewed pins")
    focused = focused_receipt(args.source_manifest, pins, web / "web/release/material-gallery")
    bundle = ROOT / "bundle"
    if bundle.exists() or (ROOT / "run").exists() or (ROOT / "release-pin.json").exists():
        raise ValueError("pin in a fresh private scratch directory")
    bundle.mkdir()
    for name, origin in origins_by_file.items():
        linked = bundle / name
        linked.parent.mkdir(parents=True, exist_ok=True)
        linked.symlink_to(origin)
    OBS["json_to"](ROOT / "bundle-origins.json", {name: str(path) for name, path in origins_by_file.items()})
    copy2(args.source_manifest, ROOT / "browser-source.sha256")
    copy2(native / "SHA256SUMS", ROOT / "native-binaries.sha256")
    copy2(runners, ROOT / "test-binaries.sha256")
    copy2(args.pins, ROOT / PIN_NAME)
    copy2(args.patch, ROOT / PATCH_NAME)
    pin = release_pin(pins)
    OBS["json_to"](ROOT / "release-pin.json", pin)
    OBS["json_to"](ROOT / "source-origins.json", origins)
    OBS["json_to"](ROOT / "build-markers.json", markers)
    OBS["json_to"](ROOT / "focused-proof.json", focused)
    (ROOT / "browser-binary-sha256.txt").write_text(manifest_text)
    write_manifest(ROOT / "sealed-inputs.sha256", {
        name: OBS["sha"](ROOT / name) for name in (
            "browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256",
            "test-binaries.sha256", "verify_navigation_stock.py", "verify_adaptive_navigation_stock.py",
            "verify-adaptive-focused-original.py",
            "source-origins.json", "bundle-origins.json", "release-pin.json", "build-markers.json", "observe-multiline.py",
            "verify_forms_pixels.py", "verify_range_pixels.py", "verify_refinement_pixels.py",
            "verify_button_pixels.py", "verify_segmented_pixels.py", "verify_outlined_field.py", "verify_filled_field.py",
            "verify-stock-receipt.py", "verify_adorned_field.py", "verify_counter.py",
            "verify_multiline.py", "verify_textbox_values.py", "verify-focused-original.py",
            "verify_label_color.py", "label-color-oracle.py", "observe-focused-original.py", "focused-proof.json",
            PIN_NAME, PATCH_NAME, "evidence_common.py", "stock_config.py",
            "verify_placeholder_stock.py", "verify_boundary_stock.py", "boundary_config.py",
            "verify_disabled_contrast.py", "verify-field-contrast.py", "disabled_contrast_config.py",
            "verify_system_pair_stock.py", "verify-system-pair.py", "placeholder-proof/verify-placeholder.py",
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
                 "native-ready", "web-ready", "web-target", "patch"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--describe-built", action="store_true")
    parser.add_argument("--pins", type=Path)
    parser.add_argument("--pins-sha256")
    args = parser.parse_args()
    if args.describe_built:
        if args.pins or args.pins_sha256:
            parser.error("--describe-built only reads the verified build outputs")
        print(json.dumps(describe(args), indent=2, sort_keys=True))
    else:
        if not args.pins or not args.pins_sha256:
            parser.error("--pins and --pins-sha256 are required for --prepare")
        prepare(args)


if __name__ == "__main__":
    main()
