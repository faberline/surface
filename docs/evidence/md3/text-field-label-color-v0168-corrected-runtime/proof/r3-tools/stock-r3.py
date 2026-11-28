#!/usr/bin/env python3
"""Prepare and verify the original five-suite q68 browser observer with pinned R3/R2 inputs."""

import argparse
import json
import math
from pathlib import Path
from runpy import run_path
import subprocess
import sys

from PIL import Image

from evidence_common import copy, copy_members, data, digest, import_from, json_file, path_entries, require, seal, verify_ledgers
from r3_identity import R2, R3, RELEASE, checked_snapshots, exact_bundle, verified_inputs

ROOT = Path(__file__).resolve().parent
TOOLS = ROOT / "tools"
HARNESS_SHA = "5e883b18779244e515fa1e07d619676982e5585471a7ace2cd669602389610ba"
ARCHIVE = Path("/tmp/surface-md3-label-color-v0168-fresh-replay.AwmrhZ8K/historical")
ARCHIVE_SHA = "5e19c8384a44e5ed31e5d61d1a7f03e14234476960b2f78ba6b78ba23fc37811"
INPUTS = ("product-source.sha256", "runner-source.sha256", "settle-compact-focus.patch")
RUNNER = "bundle/debug/surface-e2e"
CAPTURE = "material-text-field-label-motion-rtl-above-label-middle.png"


def read_observer(stage):
    manifest = seal(stage, "harness.sha256", HARNESS_SHA)
    require(data(stage / "harness-pin.json") == manifest, "Exact original process and typed tools differ")
    with import_from(stage):
        return run_path(str(stage / "observe-multiline.py"))


def prepare(stage):
    checked_snapshots(ROOT)
    assets = exact_bundle()
    tools = seal(TOOLS, "harness.sha256", HARNESS_SHA)
    base = Path("/tmp").resolve()
    require(stage != base and stage.is_relative_to(base) and all(not stage.is_relative_to(source.resolve())
                                                                 for source in (R2, R3)), "Use a fresh /tmp stage")
    stage.mkdir(mode=0o700)
    copy_members(TOOLS, stage, ("harness.sha256", *tools))
    copy_members(ROOT, stage, INPUTS)
    for name in assets:
        source = R3 / "runner/surface-e2e" if name == "debug/surface-e2e" else R2 / "stock/bundle" / name
        copy(source, stage / "bundle" / name)
    (stage / "browser-binary-sha256.txt").write_text("".join(f"{sha}  {name}\n" for name, sha in sorted(assets.items())))
    json_file(stage / "release-pin.json", RELEASE)
    json_file(stage / "harness-pin.json", tools)
    observed = read_observer(stage)["verify_inputs"]((stage / RUNNER).resolve(), assets["debug/surface-e2e"], stage)
    json_file(stage / "inputs-before.json", observed)
    return observed


def check(stage):
    observer = read_observer(stage)
    runner = (stage / RUNNER).resolve(strict=True)
    inputs = verified_inputs(runner, digest(runner), stage)
    require(data(stage / "inputs-before.json") == inputs, "Staged input seal changed")
    return observer, inputs


def check_m350(artifacts, label):
    fallback = label["compact_dark_rtl_200"]["fallback"]
    frames = fallback["color_frames"]
    middle = frames[1]
    expected = [206, 191, 243]
    require(middle["ms"] == 44 and middle["png"] == CAPTURE and middle["label_rgb"] == expected
            and middle["solid_probe_rgb"] == [expected] * 4 and middle["label_pixels"] > 40,
            "M350 did not record the exact 44 ms transition and all four same-position probes")
    source = artifacts / CAPTURE
    with Image.open(source) as png:
        require(png.format == "PNG" and png.size == (640, 2008), "Wrong actual compact PNG dimensions")
        image = png.convert("RGB")
        probes = [list(image.getpixel((math.floor(x * 2), math.floor((y + 44) * 2))))
                  for x, y in fallback["probe_points"]]
    require(probes == [expected] * 4, "Actual compact PNG middle ink differs")
    return {"ms": 44, "expected_rgb": expected, "original_png_sha256": digest(source),
            "probe_positions": fallback["probe_points"], "actual_probe_rgb": probes,
            "whole_original_label_pixels": middle["label_pixels"]}


def historical_comparison(actual):
    manifest = ARCHIVE / "SHA256SUMS"
    path = "stock-macos/focused-proof.json"
    require(digest(manifest) == ARCHIVE_SHA and digest(ARCHIVE / path) == path_entries(manifest)[path],
            "Historical label PNG comparison pins differ")
    before = data(ARCHIVE / path)["captures"]
    now = {name: receipt["sha256"] for name, receipt in actual["text_field_label_motion"]["pngs"].items()}
    require(before.keys() == now.keys() and len(now) == 34, "Wrong actual label PNG inventory")
    different = {name: {"historical": before[name], "actual": sha}
                 for name, sha in sorted(now.items()) if before[name] != sha}
    return {"compared": len(now), "byte_identical": len(now) - len(different), "differences": different}


def verify(stage):
    observer, inputs = check(stage)
    run = stage / "run"
    launch, watch = (data(run / name) for name in ("invocation.json", "headless-window-observation.json"))
    roles = {"chrome-primary", "chrome-bridge", "native"}
    verify_ledgers(run, watch, roles)
    guards = {role: str(run / "guards" / role) for role in observer["GUARD_ROLES"]}
    require(launch["argv"] == observer["stock_command"](run, stage / RUNNER, stage / "bundle", guards)
            and launch["suite_arguments"] == list(observer["SUITES"])
            and launch["injected_chrome_flags"] == list(observer["CHROME_FLAGS"])
            and launch["inputs"] == data(run / "inputs-after.json") == inputs,
            "Original stock scopes, exact launch or before/after pins differ")
    actual = json.loads(json.dumps(observer["verify_run"](run, watch, RELEASE)))
    require(actual == data(run / "verified-label-color-stock.json")
            and actual["cases"]["material"]["steps"] == 349 and actual["page_errors"] == 0,
            "Original five-suite receipt differs")
    return {**actual, "inputs": inputs, "m350": check_m350(run / "artifacts", actual["text_field_label_motion"]),
            "historical_byte_comparison": historical_comparison(actual),
            "window_receipt_sha256": digest(run / "headless-window-observation.json"),
            "runner_log_sha256": digest(run / "runner.log")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    for action in ("prepare", "check-inputs", "run", "verify"):
        mode.add_argument(f"--{action}", type=Path)
    args = parser.parse_args()
    if args.prepare:
        result = prepare(args.prepare.resolve())
    elif args.check_inputs:
        result = check(args.check_inputs.resolve(strict=True))[1]
    else:
        stage = (args.run or args.verify).resolve(strict=True)
        if args.run:
            _, inputs = check(stage)
            subprocess.run([sys.executable, "-E", "-B", str(stage / "observe-multiline.py"),
                            "--runner", str(stage / RUNNER), "--runner-sha256", inputs["runner_sha256"],
                            "--out", str(stage / "run"), "--timeout", "1800"], check=True)
        result = verify(stage)
        if args.run:
            json_file(stage / "verified-r3-stock.json", result)
        else:
            require(data(stage / "verified-r3-stock.json") == result, "Portable R3 receipt changed")
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
