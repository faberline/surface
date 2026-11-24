"""Check manifests and independent native command-menu replay receipts."""

import json
from pathlib import Path
import subprocess
import sys


batch = Path(__file__).resolve().parent
sys.path.insert(0, str(batch / "native-scripts"))
from gallery_transport import read_png
from native_runtime import digest


def checksums(path):
    return {name: sha for sha, name in
            (line.split("  ", 1) for line in path.read_text().splitlines())}


def check_seal(out, name, count):
    before = (out / f"{name}-before.log").read_bytes()
    assert before == (out / f"{name}-after.log").read_bytes()
    lines = before.splitlines()
    assert len(lines) == count and all(line.endswith(b": OK") for line in lines)


def main():
    out = Path(sys.argv[1]).resolve()
    source = batch / "source.sha256"
    pins = batch / "pins" / "native-binaries.sha256"
    check_seal(out, "source", len(checksums(source)))
    check_seal(out, "pins", 2)
    check_seal(batch, "build-source", len(checksums(source)))
    binary_names = {"surface-material-gallery": "gallery", "surface-devtools": "cli"}
    binary_sha = checksums(pins)
    assert set(binary_names) == set(binary_sha)
    for name, sha in binary_sha.items():
        assert digest(batch / "pins" / name) == sha
    assert (out / "repeat.log").read_bytes() == b""
    assert not list(out.glob("surface-loading-session-*"))
    programs = subprocess.check_output(["/bin/ps", "-axww", "-o", "pid=,comm="], text=True)
    owned = [int(pid) for pid, exe in (line.split(maxsplit=1) for line in programs.splitlines())
             if exe.removeprefix("/private") in {str(batch / "pins" / name).removeprefix("/private")
                                                 for name in binary_names}]
    assert not owned, owned

    first, second = out / "replay-1", out / "replay-2"
    report = "native-menu-report.json"
    assert (first / report).read_bytes() == (second / report).read_bytes()
    raw = json.loads((first / report).read_bytes())
    assert raw["schema"] == "surface.test.material-command-menu-native.v2"
    assert raw["transport"] == ["Page.screenshot", "CLI", "MCP"]
    assert raw["binaries"] == {binary_names[name]: sha for name, sha in binary_sha.items()}
    assert [entry["viewport"] for entry in raw["profiles"]] == [[960, 800], [320, 480]]
    assert [entry["geometry"]["side"] for entry in raw["profiles"]] == ["below", "above"]
    assert all(entry["restored_focus"] == "menu-demo-show" for entry in raw["profiles"])
    windows = [count for profile in raw["profiles"] for count in profile["visible_windows"]]
    assert windows == [0] * 10

    screenshots = {}
    for profile in raw["profiles"]:
        for shot in profile["captures"]:
            name = shot["name"]
            png = first / name
            assert png.read_bytes() == (second / name).read_bytes()
            assert shot["sha256"] == digest(png)
            assert list(read_png(png.read_bytes())[:2]) == profile["viewport"]
            assert shot["scale"] == 1
            if "footer" in shot:
                assert shot["footer"]["clear_pixels"] == shot["footer"]["expected_pixels"]
                if name.endswith("status-100.png"):
                    assert shot["footer"]["status_fully_visible"]
                    assert shot["ink"]["menu-demo-last"]["ink_pixels"] > 16
            else:
                assert shot["outline"]["matching_pixels"] >= shot["outline"]["samples"] / 2
                assert shot["palette"]["solid_panel_pixels"] >= shot["palette"]["total_panel_samples"] * 0.95
            screenshots[name] = shot
    assert len(screenshots) == 7
    for replay in [first, second]:
        assert set(screenshots) == {path.name for path in replay.glob("*.png")}
    wide, compact = [profile["checks"] for profile in raw["profiles"]]
    assert wide["disabled"] == {"focus_kept": "menu-demo-archive",
                                "keys_rejected": ["enter", "space"], "direct_error": "disabled"}
    assert wide["choice"] == "已執行「在清單中尋找」"
    assert [wide["tab_focus"], wide["reverse_tab_focus"], wide["escape_focus"]] == [
        "menu-demo-list", "menu-demo-bottom", "menu-demo-opener",
    ]
    assert compact["last_before"]["bounds"]["y"] > compact["last_after"]["y"]
    assert compact["escape_focus"] == "menu-demo-opener"
    assert compact["body_control_focus"] == "menu-demo-list"
    large, regular, restored = [compact[name] for name in ("status_200", "status_100", "restored_200")]
    for state in [large, regular, restored]:
        assert state["trigger"]["y"] - state["body"]["y"] - state["body"]["height"] == 8
        assert state["result"] == "已執行「顯示詳細資料」"
    assert restored["trigger"] == large["trigger"]
    assert regular["trigger"]["width"] < large["trigger"]["width"]
    assert regular["visible_status"] == regular["status"]["bounds"]
    assert 'version = "0.1.46"' in (batch / "source" / "Cargo.toml").read_text()
    result = {"version": "0.1.46", "headless": True, "replays": 2, "identical_replay_bytes": True,
              "window_samples_per_replay": len(windows), "max_onscreen_windows": 0,
              "remaining_owned_processes": owned, "transports": raw["transport"],
              "source_files": len(checksums(source)), "source_manifest_sha256": digest(source),
              "binary_manifest_sha256": digest(pins), "binaries": raw["binaries"],
              "pngs_per_replay": len(screenshots), "screenshots": screenshots,
              "compact_footer": {"gap_dp": 8, "text_percents": [200, 100, 200],
                                 "full_status_at_100_percent": True,
                                 "native_input": "semantic key and activation"}}
    (out / "verified-native.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(f"Verified two headless replays, {len(screenshots)} identical PNGs per replay, and {len(windows)} zero-window samples per replay")


if __name__ == "__main__":
    main()
