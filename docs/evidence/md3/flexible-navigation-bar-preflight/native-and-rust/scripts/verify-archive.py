"""Verify the archived native runs and reconstruct their sources from Menu v0.1.46."""

import argparse
from hashlib import sha256
import json
from pathlib import Path
import re
import shutil
import subprocess
from tempfile import TemporaryDirectory

from gallery_transport import read_png

ROOT = Path(__file__).resolve().parent.parent
REPORT = "native-gallery-report.json"
CAPTURES = {"wide-light-focused.png", "compact-dark-rtl-200.png"}
TITLE = "navigation-bar-demo-page-title"
LIBRARY_TEST = "crates/surface-material/tests/flexible_navigation_bar.rs"
GALLERY_DEMO = "examples/material-gallery/src/flexible_navigation_bar_demo.rs"


def hash_file(path):
    return sha256(path.read_bytes()).hexdigest()


def manifest(path):
    entries = {}
    for line in path.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        assert re.fullmatch(r"[a-f0-9]{64}", checksum)
        assert name not in entries and not Path(name).is_absolute() and ".." not in Path(name).parts
        entries[name] = checksum
    return entries


def compare_tree(tree, expected):
    actual = {str(path.relative_to(tree)): hash_file(path)
              for path in tree.rglob("*") if path.is_file()}
    assert actual == expected, "source file set or checksum differs"


def changed(left, right):
    return {name for name in left.keys() | right.keys() if left.get(name) != right.get(name)}


def apply(tree, patch, *, reverse=False):
    command = ["patch", "--batch", "-p1"]
    if reverse:
        command.append("-R")
    with patch.open("rb") as content:
        subprocess.run(command, cwd=tree, stdin=content, stdout=subprocess.PIPE, check=True)


def verify_sources(menu_source):
    inputs = ROOT / "inputs"
    baseline = manifest(inputs / "menu-v0.1.46.sha256")
    library = manifest(inputs / "library-source.sha256")
    final = manifest(inputs / "combined-source.sha256")
    before = manifest(inputs / "pre-header-source.sha256")
    assert [len(a) for a in (baseline, library, final, before)] == [590, 592, 593, 593]
    assert changed(final, before) == {LIBRARY_TEST, GALLERY_DEMO}
    assert changed(final, library) == {"examples/material-gallery/src/lib.rs", GALLERY_DEMO}
    logs = ROOT / "logs/gallery"
    sealed = (logs / "final-build-source-before.log").read_bytes()
    assert sealed == (logs / "final-build-source-after.log").read_bytes()
    assert set(sealed.decode().splitlines()) == {f"{name}: OK" for name in final}
    if menu_source is not None:
        compare_tree(menu_source, baseline)
        with TemporaryDirectory(dir="/tmp", prefix="surface-nav-reconstruct-") as temp:
            tree = Path(temp) / "source"
            shutil.copytree(menu_source, tree)
            apply(tree, ROOT / "patches/flexible-navigation-bar.patch")
            compare_tree(tree, library)
            apply(tree, ROOT / "patches/flexible-nav-gallery.patch")
            compare_tree(tree, final)
            apply(tree, ROOT / "patches/pin-gallery-header.patch", reverse=True)
            apply(tree, ROOT / "patches/restore-preflight-test-import.patch")
            compare_tree(tree, before)
    return {"base_files": len(baseline), "component_files": len(library),
            "combined_files": len(final), "source_reconstruction_performed": menu_source is not None}


def binaries(path, existing_bin_dir=None):
    pin = manifest(path)
    assert set(pin) == {"surface-material-gallery", "surface-devtools"}
    if existing_bin_dir is not None:
        for name, checksum in pin.items():
            assert hash_file(existing_bin_dir / name) == checksum
    return {"gallery": pin["surface-material-gallery"], "cli": pin["surface-devtools"]}


def verify_rasters(folder, profile, shot):
    png = folder / shot["name"]
    assert hash_file(png) == shot["sha256"]
    width, height, _ = read_png(png.read_bytes())
    assert [width, height] == profile["viewport"] == [shot["width"], shot["height"]]
    return [width, height]


def bounds(box):
    return [box[k] for k in ("x", "y", "width", "height")]


def verify_native(existing_bin_dir, earlier_bin_dir):
    pins = binaries(ROOT / "inputs/native-binaries.sha256", existing_bin_dir)
    runs = [ROOT / "native" / f"replay-{n}" for n in (1, 2)]
    first, second = ({path.name: hash_file(path) for path in run.iterdir()} for run in runs)
    assert first == second and set(first) == CAPTURES | {REPORT}
    decoded = []
    for run in runs:
        report = json.loads((run / REPORT).read_text())
        assert report["schema"] == "surface.test.flexible-nav-gallery-native.v1"
        assert report["headless"] and report["binaries"] == pins
        assert report["transport"] == ["Page.screenshot", "CLI", "MCP"]
        assert [v for profile in report["profiles"] for v in profile["windows"]] == [0] * 6
        decoded.extend(verify_rasters(run, profile, profile["shot"]) for profile in report["profiles"])
    assert len(decoded) == 4

    wide, compact = report["profiles"]
    assert set(wide["counts"]) == {"3", "4", "5"}
    assert bounds(wide["counts"]["5"]["bar"]) == [240, 736, 480, 64]
    assert compact["rtl"] and compact["text_scale"] == 2
    assert bounds(compact["geometry"]["bar"]) == [0, 288, 320, 112]
    title = compact["shot"]["ink"][TITLE]
    assert bounds(title["bounds"]) == [148, 16, 64, 48] and title["ink_pixels"] == 614
    assert compact["shot"]["ink"]["navigation-bar-demo-updates-label"]["bounds"]["height"] == 64

    original = ROOT / "native/pre-header"
    old = json.loads((original / REPORT).read_text())
    assert old["binaries"] == binaries(ROOT / "inputs/pre-header-binaries.sha256", earlier_bin_dir)
    prior_profile = old["profiles"][1]
    prior_title = prior_profile["page_position"]
    assert prior_title["bounds"]["y"] == prior_title["clip"]["height"] == 288
    assert prior_title["clip"]["y"] == 0
    verify_rasters(original, prior_profile, prior_profile["shot"])
    return {"runs": len(runs), "decoded_final_pngs": len(decoded), "decoded_pre_header_pngs": 1,
            "identical_replay_bytes": True, "max_visible_windows": 0, "window_samples_per_run": 6,
            "transport": report["transport"], "clipped_title_old_y": 288, "pinned_title_new_y": 16,
            "pinned_title_ink_pixels": 614,
            "original_binary_hashes_verified": existing_bin_dir is not None and earlier_bin_dir is not None}


def verify_checks():
    logs = ROOT / "logs"
    test_files = (logs / "library/targeted-final.log", logs / "library/consumer-contract-final.log",
                  logs / "gallery/final-targeted-gallery.log")
    counts = [int(n) for path in test_files for n in
              re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored;", path.read_text())]
    assert counts == [7, 6, 8, 1, 2]
    clippy = (logs / "library/clippy-final.log", logs / "gallery/final-clippy.log")
    assert all("Finished `dev` profile" in path.read_text() and "error:" not in path.read_text() for path in clippy)
    return {"rust_tests_passed": sum(counts), "flexible_bar": 7, "badges": 6, "legacy_bar": 8,
            "consumer_contract": 1, "gallery": 2, "scoped_strict_clippy": 2}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--menu-source", type=Path, help="Exact sealed .46 source to confirm patch reconstruction")
    parser.add_argument("--native-pins", type=Path, help="Optional folder with original captured binaries")
    parser.add_argument("--pre-header-pins", type=Path, help="Optional folder with earlier captured binaries")
    args = parser.parse_args()
    print(json.dumps({"scope": "v0.1.46-based product preflight",
                      "source": verify_sources(args.menu_source),
                      "native": verify_native(args.native_pins, args.pre_header_pins),
                      "checks": verify_checks()}, indent=2))


if __name__ == "__main__":
    main()
