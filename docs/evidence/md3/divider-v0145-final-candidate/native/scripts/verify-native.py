"""Check both sealed native Divider replays and independently decode every PNG."""

import argparse
import hashlib
import importlib.util
import json
import subprocess
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent.parent
VERSION = "0.1.45"
SOURCE = "a2750ba3be68e0b6bc5b6f0183d4d21acf7e90fe9c4a65e623ecacbd192dd0aa"
REPORT = "native-divider-report.json"


def digest(path):
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def audit_module():
    scripts = ROOT / "native-scripts"
    sys.path.insert(0, str(scripts))
    spec = importlib.util.spec_from_file_location("divider_audit", scripts / "replay-divider.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def live_clean(expected_named_hashes):
    for key, name in (("gallery", "surface-material-gallery"), ("cli", "surface-devtools")):
        assert digest(ROOT / "pins" / name) == expected_named_hashes[key], name
    programs = subprocess.check_output(["/bin/ps", "-axww", "-o", "pid=,comm="], text=True)
    owned = [int(pid) for pid, exe in (line.split(maxsplit=1) for line in programs.splitlines())
             if exe.startswith(str(ROOT / "pins") + "/")
             or exe.startswith(str(ROOT / "pins").removeprefix("/private") + "/")]
    assert not owned, owned


def verify(earlier, assert_live_clean):
    before = (ROOT / "pin-before.json").read_bytes()
    assert before == (ROOT / "pin-after.json").read_bytes()
    stage = json.loads(before)
    assert stage["workspace_version"] == VERSION and stage["source_paths"] == 583
    assert stage["source_manifest_sha256"] == SOURCE
    for path, expected in [
        ("source.sha256", SOURCE),
        ("sealed-inputs.sha256", stage["stock_stage_seal_sha256"]),
        ("browser-binary-sha256.txt", stage["sealed_stock_files"]["browser-binary-sha256.txt"]),
        ("harness.sha256", stage["sealed_stock_files"]["harness.sha256"]),
    ]:
        assert digest(ROOT / "inputs" / path) == expected, path
    native = stage["native_hashes"]
    assert (ROOT / "inputs/native-binaries.sha256").read_text() == "".join(
        f"{checksum}  {name}\n" for name, checksum in native.items())
    expected_named_hashes = {"gallery": native["surface-material-gallery"], "cli": native["surface-devtools"]}
    assert not list(ROOT.glob("surface-loading-session-*"))
    assert (ROOT / "repeat.log").read_bytes() == b""

    first, second = [ROOT / f"replay-{number}" for number in (1, 2)]
    assert {file.name: digest(file) for file in first.iterdir()} == {
        file.name: digest(file) for file in second.iterdir()}
    audit = audit_module()
    receipts = []
    for replay in (first, second):
        report = json.loads((replay / REPORT).read_text())
        assert report["schema"] == "surface.test.material-divider-native.v2"
        assert report["transport"] == ["Page.screenshot", "CLI", "MCP"]
        assert report["binaries"] == expected_named_hashes
        assert [profile["viewport"] for profile in report["profiles"]] == [[960, 800], [320, 800]]
        captures = {}
        windows = []
        for profile in report["profiles"]:
            viewport = tuple(profile["viewport"])
            windows.extend(profile["visible_windows"])
            assert profile["tab_path"] == ["divider-demo-" + name for name in
                                           ("back", "theme", "rtl", "large-text", "weight", "back")]
            assert profile["restored_focus"] == "divider-demo-show"
            assert [item["thickness_dp"] for item in profile["weights"]] == [1, 8]
            for mode, item in zip(("regular", "heavy"), profile["weights"]):
                assert item["weight"] == mode and item["selected"] is (mode == "heavy")
                assert item["focused"] == "divider-demo-weight"
                shot = item["capture"]
                name = shot["name"]
                assert name == f"{profile['profile']}-{mode}.png"
                path = replay / name
                assert shot["sha256"] == digest(path)
                assert (shot["width"], shot["height"], shot["scale"]) == (*viewport, 1)
                png = audit.read_png(path.read_bytes())
                assert png[:2] == viewport
                assert len(item["ink"]) == 4
                for suffix, actual in item["ink"].items():
                    checked = audit.stripe(actual["bounds"], item["thickness_dp"], suffix,
                                           bytes(actual["rgba"]), bytes(actual["background"]), png)
                    assert checked == actual, (replay, name, suffix)
                captures[name] = {"sha256": shot["sha256"], "dimensions": viewport,
                                  "thickness_dp": item["thickness_dp"],
                                  "full_depth_solid_pixels": sum(x["solid_pixels"] for x in item["ink"].values()),
                                  "clean_surround_pixels": sum(x["adjacent_pixels"] for x in item["ink"].values())}
        assert len(captures) == 4 and windows == [0] * 8
        assert {file.name for file in replay.iterdir()} == set(captures) | {REPORT}
        receipts.append(captures)
    assert receipts[0] == receipts[1]

    comparison = None
    if earlier:
        prior_hashes = {name: digest(earlier / name) for name in receipts[0]}
        prior_report = json.loads((earlier / REPORT).read_text())
        assert set(prior_hashes) == {path.name for path in earlier.glob("*.png")}
        assert all(prior_hashes[name] == item["sha256"] for name, item in receipts[0].items())
        assert report["profiles"] == prior_report["profiles"]
        comparison = {"matching_png_bytes": len(prior_hashes), "matching_profile_receipts": True,
                      "earlier_binaries": prior_report["binaries"]}

    if assert_live_clean:
        live_clean(expected_named_hashes)
    return {
        "scope": "exact v0.1.45 stock native on the Divider sampler-only candidate",
        **stage,
        "native_manifest_sha256": digest(ROOT / "inputs/native-binaries.sha256"),
        "headless": True,
        "replays": 2,
        "pngs_per_replay": 4,
        "transports": report["transport"],
        "identical_replay_bytes": True,
        "window_samples_per_replay": 8,
        "max_visible_windows": 0,
        "remaining_owned_processes": [] if assert_live_clean else None,
        "pngs": receipts[0],
        "earlier_comparison": comparison,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--earlier", type=Path, help="Earlier Divider native replay directory")
    parser.add_argument("--assert-live-clean", action="store_true", help="Check pinned binary hashes and process table")
    args = parser.parse_args()
    print(json.dumps(verify(args.earlier, args.assert_live_clean), indent=2))
