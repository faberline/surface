"""Verify both exact release replays without starting any process."""

import argparse
from hashlib import sha256
import json
from pathlib import Path

from gallery_transport import read_png

ROOT = Path(__file__).resolve().parent.parent
REPORT = "native-gallery-report.json"
PNG_VIEWPORTS = {"wide-light-focused.png": [960, 800], "compact-dark-rtl-200.png": [320, 400]}


def hash_file(path):
    return sha256(path.read_bytes()).hexdigest()


def read_json(path):
    return json.loads(path.read_text())


def bounds(box):
    return [box[name] for name in ("x", "y", "width", "height")]


def verify(preflight=None):
    before, after = (ROOT / "pin-before.json").read_bytes(), (ROOT / "pin-after.json").read_bytes()
    assert before == after and (ROOT / "repeat.log").read_bytes() == b""
    stage = json.loads(before)
    assert stage["workspace_version"] == "0.1.47" and stage["source_paths"] == 594
    for path, checksum in [("source.sha256", stage["source_manifest_sha256"]),
                           ("sealed-inputs.sha256", stage["stock_stage_seal_sha256"]),
                           ("browser-binary-sha256.txt", stage["sealed_stock_files"]["browser-binary-sha256.txt"]),
                           ("harness.sha256", stage["sealed_stock_files"]["harness.sha256"])]:
        assert hash_file(ROOT / "inputs" / path) == checksum, path
    native = stage["native_hashes"]
    assert (ROOT / "inputs/native-binaries.sha256").read_text() == "".join(
        f"{checksum}  {name}\n" for name, checksum in native.items())
    expected_pins = {"gallery": native["surface-material-gallery"], "cli": native["surface-devtools"]}
    runs = [ROOT / f"replay-{run}" for run in (1, 2)]
    first, second = ({path.name: hash_file(path) for path in run.iterdir()} for run in runs)
    assert first == second and set(first) == PNG_VIEWPORTS.keys() | {REPORT}
    decoded = {}
    for run in runs:
        report = read_json(run / REPORT)
        assert report["schema"] == "surface.test.flexible-nav-gallery-native.v1"
        assert report["headless"] and report["binaries"] == expected_pins
        assert report["transport"] == ["Page.screenshot", "CLI", "MCP"]
        assert [count for profile in report["profiles"] for count in profile["windows"]] == [0] * 6
        for profile in report["profiles"]:
            shot = profile["shot"]
            png = run / shot["name"]
            assert first[shot["name"]] == shot["sha256"]
            width, height, _ = read_png(png.read_bytes())
            assert [width, height] == [shot["width"], shot["height"]] == PNG_VIEWPORTS[shot["name"]]
            decoded[shot["name"]] = {"size": [width, height], "sha256": shot["sha256"]}
    wide, compact = report["profiles"]
    assert set(wide["counts"]) == {"3", "4", "5"}
    wide_bar = bounds(wide["counts"]["5"]["bar"])
    compact_bar = bounds(compact["geometry"]["bar"])
    assert wide_bar == [240, 736, 480, 64] and compact_bar == [0, 288, 320, 112]
    assert compact["rtl"] and compact["text_scale"] == 2
    title = compact["shot"]["ink"]["navigation-bar-demo-page-title"]
    wrap = compact["shot"]["ink"]["navigation-bar-demo-updates-label"]["bounds"]["height"]
    assert bounds(title["bounds"]) == [148, 16, 64, 48] and title["ink_pixels"] > 100 and wrap == 64
    assert not list(ROOT.glob("surface-loading-session-*"))
    comparison = None
    if preflight:
        prior = read_json(preflight / REPORT)
        comparison = {"identical_pixels": [name for name in sorted(PNG_VIEWPORTS)
                                            if (runs[0] / name).read_bytes() == (preflight / name).read_bytes()],
                      "same_behavior_except_binary_hashes": report["profiles"] == prior["profiles"]}
    return {"scope": "exact v0.1.47 stock native run", **stage,
            "native_manifest_sha256": hash_file(ROOT / "inputs/native-binaries.sha256"),
            "replays": len(runs), "decoded_pngs_per_replay": len(decoded), "pngs": decoded,
            "window_samples_per_replay": 6, "max_visible_windows": 0, "transports": report["transport"],
            "replay_directories_byte_identical": True, "wide_bar": wide_bar, "compact_bar": compact_bar,
            "pinned_title": bounds(title["bounds"]), "pinned_title_ink_pixels": title["ink_pixels"],
            "wrapped_label_height_dp": wrap, "preflight_comparison": comparison}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preflight", type=Path, help="Optional prior v0.1.46-preflight native replay")
    args = parser.parse_args()
    print(json.dumps(verify(args.preflight), indent=2))


if __name__ == "__main__":
    main()
