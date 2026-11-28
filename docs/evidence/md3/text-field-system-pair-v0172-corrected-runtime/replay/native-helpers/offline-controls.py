"""Calibrate the endpoint probes against sealed v0.1.69 native PNGs."""

import json
from pathlib import Path
from runpy import run_path
import subprocess
import sys

from gallery_transport import read_png, require
from native_runtime import digest

HERE = Path(__file__).resolve().parent
CHECK = run_path(str(HERE / "replay-boundaries.py"))
Q69 = Path("/tmp/surface-md3-text-field-placeholder-v0169.bc9QKVhY")
PRIOR = Q69 / "native-exact-replay"
OLD_SOURCE = "1b1379f04bbb534b6a25e857185de34160b367bfa60ef9a36a1fce1d0a8d3d55"
OLD_BINARY = "85ea052ae4553819e92be2d9313430c0dbe3956c3d98566b48c2ffc931634ec7"
OLD_EVIDENCE = "9b63bc905cf8d3a02a16c97c783c030c505f3c974d1f61e67bcaa0e9ff656f95"
DEVELOPMENT = Path("/tmp/surface-md3-field-boundary-v0170.X35mBqoS/development-snapshot-r1")
DEVELOPMENT_SHA = "f0b9bf4f43c9ec853fd2ec327028dabfe8215846ef0beec6222f3ac7fecb684c"


def denied(action, text):
    try:
        action()
    except (RuntimeError, ValueError) as error:
        require(text in str(error), f"Unexpected rejection {error}")
    else:
        raise AssertionError("A damaged endpoint was accepted")


def put(rows, x, y, color):
    rows[y][x * 4:x * 4 + 4] = bytes(color)


def snapshot(receipt, files, scope, appearance, phase):
    shot = receipt["replays"][0][scope]["appearances"][appearance]["captures"][phase]
    relative = "replay-1/" + shot["name"]
    file = PRIOR / relative
    require(files[relative] == shot["sha256"] == digest(file), "Prior calibration PNG changed")
    _, _, rows = read_png(file.read_bytes())
    return rows, shot["layout"]


def main():
    load = CHECK["PRIOR"]
    development = CHECK["checked_source"](DEVELOPMENT / "source", DEVELOPMENT / "source.sha256", DEVELOPMENT_SHA)
    require(development["files"] == 689, "Unexpected development-only source inventory")
    paths = load["pins"](PRIOR.resolve(strict=True), PRIOR / "SHA256SUMS", OLD_EVIDENCE)
    proof = json.loads((PRIOR / load["REPORT"]).read_text())
    require(proof["source"]["manifest_sha256"] == OLD_SOURCE
            and proof["binary_manifest_sha256"] == OLD_BINARY and proof["replays"][0] == proof["replays"][1],
            "Wrong original calibration source or native repeat")
    verify = CHECK["verify_boundary"]
    actual = {}
    for kind in ("outlined", "filled"):
        for stage in CHECK["STAGES"]:
            rows, measured = snapshot(proof, paths, "wide", kind, stage)
            played = verify(rows, measured, "wide", kind, stage)
            actual[f"wide-{kind}-{stage}"] = [played["color_role"], played["edges"]["bottom"]["stroke_dp"]]
    compact_rows, geometry = snapshot(proof, paths, "compact", "outlined", "above-seeded")
    closed = verify(compact_rows, geometry, "compact", "outlined", "idle")
    committed = CHECK["verify_value"](compact_rows, geometry, "compact")
    require(closed["closed_flat_top"] and closed["edges"]["top"]["probe_count"] > 200
            and committed["above_label_ink_pixels"] >= 100, "Real compact probe did not reach painted RTL glyphs")
    actual["compact-outlined-seeded"] = [closed["color_role"], closed["edges"]["top"]["probe_count"]]

    # Keep the real old geometry and background. Magnify just the tested
    # endpoint stroke in memory to exercise a new compact focused condition.
    box = geometry["face"]
    rgb = CHECK["PALETTES"]["compact"]["primary"]
    background = CHECK["PALETTES"]["compact"]["background"]
    edited = list(map(bytearray, compact_rows))
    points = CHECK["flat_points"](box, len(edited[0]) // 4, len(edited), "compact")
    for anchors, (dx, dy) in points.values():
        for x, y in anchors:
            for offset in range(3):
                put(edited, x + offset * dx, y + offset * dy, rgb)
    verify(edited, geometry, "compact", "outlined", "focused")
    denies = []
    for label, edge, index, depth, color in (
        ("right-notch", "top", -10, 0, background),
        ("thick-bottom", "bottom", 1, 3, rgb),
        ("external-paint", "left", 0, -1, rgb),
        ("wrong-ink", "right", 1, 0, CHECK["PALETTES"]["compact"]["outline"]),
    ):
        anchors, (dx, dy) = points[edge]
        x, y = anchors[index]
        x, y = x + dx * depth, y + dy * depth
        original = load["pixel"](edited, x, y)
        put(edited, x, y, color)
        denied(lambda: verify(edited, geometry, "compact", "outlined", "focused"), f"focused {edge} at")
        put(edited, x, y, original)
        denies.append(label)

    actual_bins = Q69 / "exact-candidate/binaries/SHA256SUMS"
    actual_source = Q69 / "exact-candidate/source.sha256"
    command = [sys.executable, "-E", "-B", str(HERE / "replay-boundaries.py"), "--verify-inputs-only",
               "--source-root", str(actual_source.parent / "source"), "--source-manifest", str(actual_source),
               "--source-manifest-sha", OLD_SOURCE, "--binary-manifest", str(actual_bins),
               "--binary-manifest-sha", OLD_BINARY]
    old = subprocess.run(command, capture_output=True, text=True, check=False)
    require(old.returncode != 0 and "Only the reviewed v0.1.70 candidate is valid" in old.stderr,
            "An older source passed exact input verification")
    require(sum(len(appearances) for _, appearances in CHECK["PROFILES"].values()) * len(CHECK["STAGES"]) * 2 == 18
            and sum(len(CHECK["sample_names"](scope)) for scope in CHECK["PROFILES"]) * 2 == 48,
            "Expected window samples or original counts changed")
    print(json.dumps({"scope": "Offline calibration. No v0.1.70 app or browser launched.",
                      "checked_v0170_development_source": development,
                      "prior_real_evidence_manifest": OLD_EVIDENCE, "calibrated_actual_pngs": actual,
                      "retained_rtl_value": committed, "synthetic_focus_probe_rejections": denies,
                      "previous_source_rejected": True, "expected_v0170_pngs": 18,
                      "expected_v0170_window_samples": 48}, indent=2))


if __name__ == "__main__":
    main()
