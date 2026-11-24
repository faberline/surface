#!/usr/bin/env python3
"""Verify both exact native Gallery replays against the captured inputs and prior frames."""

import argparse
from hashlib import sha256
import json
from pathlib import Path
from struct import unpack


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def record(directory):
    receipt = json.loads((directory / "native-full-screen-dialog-report.json").read_text())
    assert receipt["schema"] == "surface.test.full-screen-dialog-fab-native.v1"
    assert receipt["transport"] == ["Page.screenshot", "CLI", "MCP"]
    assert receipt["motion_mode"] == "stock headless Reduce Motion"
    profiles = receipt["profiles"]
    viewport_keys = [(profile["viewport"]["width"], profile["viewport"]["height"]) for profile in profiles]
    short = receipt["short_viewport"]
    viewport_keys.append((short["viewport"]["width"], short["viewport"]["height"]))
    assert viewport_keys == [(320, 400), (480, 640), (960, 800), (320, 240)]
    profiles = [*profiles, short]
    windows = [sample["visible"] for profile in profiles for sample in profile["visible_windows"]]
    assert windows == [0] * 22
    captures = {}

    def visit(value):
        if isinstance(value, list):
            for item in value:
                visit(item)
        elif isinstance(value, dict):
            if value.get("format") == "png" and "name" in value:
                name = value["name"]
                data = (directory / name).read_bytes()
                assert data[:16] == b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR", name
                assert unpack(">II", data[16:24]) == (value["width"], value["height"]), name
                assert sha256(data).hexdigest() == value["sha256"], name
                assert name not in captures, name
                captures[name] = value["sha256"]
            else:
                for item in value.values():
                    visit(item)

    visit(profiles)
    assert len(captures) == 14
    assert set(captures) == {path.name for path in directory.glob("*.png")}
    return receipt, captures, windows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("batch", type=Path)
    parser.add_argument("run", type=Path)
    parser.add_argument("prior_replay", type=Path)
    parser.add_argument("prior_source_manifest", type=Path)
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    batch, run, baseline = args.batch, args.run, args.prior_replay
    source_sum = batch / "source.sha256"
    source_files = len(source_sum.read_text().splitlines())
    assert source_files == 579
    assert 'version = "0.1.44"' in (batch / "source/Cargo.toml").read_text()
    for kind in ("source", "pins"):
        before = (run / f"{kind}-before.log").read_bytes()
        after = (run / f"{kind}-after.log").read_bytes()
        assert before == after and all(line.endswith(b": OK") for line in before.splitlines()), kind
        assert len(before.splitlines()) == (source_files if kind == "source" else 2)
    assert (batch / "build-source-before.log").read_bytes() == (batch / "build-source-after.log").read_bytes()
    assert (run / "repeat.log").read_bytes() == b""
    pin_sum = batch / "pins/native-binaries.sha256"
    hardware_pins = dict((name, checksum) for checksum, name in (line.split() for line in pin_sum.read_text().splitlines()))
    assert set(hardware_pins) == {"surface-material-gallery", "surface-devtools"}
    for name, checksum in hardware_pins.items():
        assert digest(batch / "pins" / name) == checksum
    pin_report = {"gallery": hardware_pins["surface-material-gallery"], "cli": hardware_pins["surface-devtools"]}
    primary, pngs, windows = record(run / "replay-1")
    repeat, repeated_pngs, repeated_windows = record(run / "replay-2")
    assert primary == repeat and primary["binaries"] == pin_report
    assert windows == repeated_windows and pngs == repeated_pngs
    report_name = "native-full-screen-dialog-report.json"
    for name in (*pngs, report_name):
        assert (run / "replay-1" / name).read_bytes() == (run / "replay-2" / name).read_bytes(), name

    prior, previous_pngs, _ = record(baseline)
    assert set(pngs) == set(previous_pngs)
    same = sorted(name for name in pngs if pngs[name] == previous_pngs[name]
                  and (run / "replay-1" / name).read_bytes() == (baseline / name).read_bytes())
    details = {name: {"prior": previous_pngs[name], "exact": pngs[name]}
               for name in pngs if name not in same}
    report_same = ({k: v for k, v in primary.items() if k != "binaries"}
                   == {k: v for k, v in prior.items() if k != "binaries"})
    short = primary["short_viewport"]
    measurements = {"short_preview_scroll_dp": short["before_lower_control"]["y"] - short["after_lower_control"]["y"],
                    "short_fab_clearance_dp": short["fab"]["y"] - short["scroller"]["y"] - short["scroller"]["height"]}
    out = {
        "version": "0.1.44",
        "source_files": source_files, "source_manifest_sha256": digest(source_sum),
        "binary_manifest_sha256": digest(pin_sum), "binaries": pin_report,
        "runs": 2, "pngs_each": len(pngs), "transports": primary["transport"],
        "each_capture_bytes_and_metadata_match": True,
        "windows_samples_each": len(windows), "max_visible_windows": max(windows),
        "replay_pair_all_files_byte_identical": True,
        "prior_source_manifest_sha256": digest(args.prior_source_manifest),
        "prior_identical_pngs": same, "prior_different_pngs": details,
        "prior_behavior_report_equal_excluding_build_hashes": report_same,
        "measurements": measurements,
        "pngs": pngs,
    }
    args.out.write_text(json.dumps(out, ensure_ascii=False, indent=2) + "\n")
    print(f"Verified two 14-PNG replays, each with 22 zero-window samples. {len(same)} of 14 PNGs match the prior capture.")


if __name__ == "__main__":
    main()
