#!/usr/bin/env python3
"""Recheck sealed native multiline replay files and pixels without launching the Gallery."""

import argparse
import json
from pathlib import Path
from runpy import run_path

from gallery_transport import read_png, require
from native_runtime import digest

CASES = {
    "wide": {
        "wide-outlined-rest.png": (1, (True,), False),
        "wide-outlined-grow.png": (3, (True, True, False), True),
        "wide-filled-fixed-top.png": (3, (True, True, True), False),
        "wide-filled-fixed-bottom.png": (3, (True, True, False), False),
    },
    "compact": {
        "compact-outlined-grow.png": (3, (True, True, False), True),
        "compact-filled-grow.png": (3, (True, True, False), True),
        "compact-filled-fixed-bottom.png": (3, (True, True, False), False),
    },
}
WINDOW_PHASES = {"ready", "opened", "grow_focused", "fixed_revealed", "submitted", "closed"}


def normalized(value):
    return json.loads(json.dumps(value))


def verify(root, manifest_sha, source_sha, binary_sha, repeats):
    helper = run_path(str(Path(__file__).with_name("replay-multiline-text-field.py")))
    manifest = helper["pins"](root, root / "SHA256SUMS", manifest_sha)
    report_file = root / "native-multiline-text-field-report.json"
    report = json.loads(report_file.read_text())
    require(report["schema"] == "surface.test.text-field-multiline.native.v1"
            and report["source"]["manifest_sha256"] == source_sha
            and report["binary_manifest_sha256"] == binary_sha, "Wrong native input pins")
    require(report["transports"] == ["Page.screenshot", "CLI", "MCP"]
            and len(report["replays"]) == len(report["processes"]) == repeats
            and all(result == report["replays"][0] for result in report["replays"]), "Wrong replay scope or parity")
    inventory, pids, window_samples, images = {report_file.name}, set(), 0, 0
    for index, (replay, processes) in enumerate(zip(report["replays"], report["processes"], strict=True), 1):
        require(set(replay) == set(processes) == set(CASES), "Wrong viewport set")
        for scope, expected in CASES.items():
            result, process = replay[scope], processes[scope]
            require(process["pid"] not in pids and process["reaped"] is True
                    and set(process["visible_windows"]) == WINDOW_PHASES
                    and set(process["visible_windows"].values()) == {0}, "Wrong process or window evidence")
            pids.add(process["pid"])
            window_samples += len(process["visible_windows"])
            captures = {asset["name"]: asset for asset in result["captures"]}
            require(set(captures) == set(expected) and len(result["captures"]) == len(expected),
                    "Missing or duplicate capture")
            for name, (row_count, lines, align_control) in expected.items():
                asset = captures[name]
                relative = f"replay-{index}/{name}"
                path = root / relative
                require(asset["sha256"] == manifest[relative] and asset["format"] == "png",
                        "Original capture digest did not match")
                width, height, rows = read_png(path.read_bytes())
                viewport = helper["SCOPES"][scope]
                require((width, height) == viewport == (asset["width"], asset["height"])
                        and result["viewport"] == list(viewport) and asset["scale"] == 1, "Wrong decoded PNG size")
                face, text, measured = asset["face_paint"], asset["text_paint"], asset["geometry"]
                expected_width = 1 if name.endswith("rest.png") else 3
                require(face["stroke_dp"] == expected_width and measured["row_count"] == row_count
                        and measured["line_height"] == (48 if scope == "compact" else 24),
                        "Wrong measured stroke or line-pitch budget")
                bounds = measured["face"]
                middle = bounds["x"] + bounds["width"] / 2
                decoded_edge = [list(helper["pixel"](rows, middle, helper["bottom"](bounds) - step - .5))
                                for step in range(5)]
                require(decoded_edge == face["samples"] and decoded_edge[0] != face["paper"]
                        and decoded_edge[:expected_width] == [decoded_edge[0]] * expected_width
                        and decoded_edge[expected_width:] == [face["paper"]] * (5 - expected_width),
                        "Original PNG stroke does not match the receipt")
                caret = tuple(decoded_edge[0]) if expected_width == 3 else None
                rerun = helper["body_paint"](rows, measured, tuple(text["paper"]), tuple(text["foreground"]), lines,
                                             rtl=scope == "compact", equal_end=align_control, caret_ink=caret)
                require(normalized(rerun) == text, "Stored ink bounds or clipping do not match the original pixels")
                inventory.add(relative)
                images += 1
    require(set(manifest) == inventory, "Unexpected hashed output scope")
    on_disk = {path.relative_to(root).as_posix() for path in root.rglob("*")
               if path.is_file() and path.name != "SHA256SUMS"}
    require(on_disk == inventory, "Unsealed output")
    return {"scope": "native headless rerun of original PNG checks", "original_pngs": images,
            "repeats": repeats, "observed_window_samples": window_samples, "visible_owned_windows": 0,
            "reaped_gallery_processes": len(pids), "source_manifest_sha256": source_sha,
            "binary_manifest_sha256": binary_sha, "replay_manifest_sha256": manifest_sha,
            "report_sha256": digest(report_file)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--manifest-sha", required=True)
    parser.add_argument("--source-manifest-sha", required=True)
    parser.add_argument("--binary-manifest-sha", required=True)
    parser.add_argument("--repeats", type=int, choices=(1, 2), default=2)
    args = parser.parse_args()
    result = verify(args.root.resolve(strict=True), args.manifest_sha, args.source_manifest_sha,
                    args.binary_manifest_sha, args.repeats)
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
