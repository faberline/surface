#!/usr/bin/env python3
"""Verify the Material Gallery boundary cases in headless Chrome on exact v0.1.74 inputs."""

import argparse
import json
import math
import os
from pathlib import Path
import re
from runpy import run_path
import shlex
import subprocess
import sys
from time import monotonic, sleep

from PIL import Image

from evidence_common import data, digest, json_file, require

ROOT = Path(__file__).resolve().parent
GUARD = ROOT / "observe-focused-original.py"
GUARD_SHA = "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7"
STEPS = [str(i) for i in range(1, 370) if i != 23]
UPDATER_ARG = "--disable-updater-scheduler"
DISPLAY_VARS = {"DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"}
OVERRIDES = {"SURFACE_CHROME", "SURFACE_CHROME_ARGS", "CHROME_PATH",
             "SURFACE_BRIDGE_SESSION", "SURFACE_DEVTOOLS_SESSION"}


def inputs(args):
    require(digest(GUARD) == GUARD_SHA, "Different macOS observer source")
    subprocess.run([sys.executable, "-E", "-B", str(ROOT / "source-check.py")], check=True)
    build = Path("/tmp/surface-md3-v0174-replay.jLEUA2mj/outputs").resolve(strict=True)
    require(args.runner.resolve() == build / "stock/debug/surface-e2e", "Wrong runner")
    require(args.web_root.resolve() == build / "stock/web/release", "Wrong Web root")
    require((build / "BUILD-COMPLETE").is_file(), "Build incomplete")
    subprocess.run(["shasum", "-a", "256", "-c", "SHA256SUMS"], cwd=build, stdout=subprocess.DEVNULL, check=True)
    require(digest(args.runner) == args.runner_sha256, "Runner SHA mismatch")
    require(digest(build / "SHA256SUMS") == args.web_sha256, "Build manifest SHA mismatch")
    require(args.source.resolve() == Path("/tmp/surface-md3-v0174-corrected.gP1uy0FW/source").resolve(), "Wrong source")
    require(digest(args.source_manifest) == "53fb1fd46ab0b360186238bc3a946862c16047646e4578fc25dff9d308811f8f", "Wrong source manifest")
    require(args.web_manifest.resolve() == build / "SHA256SUMS", "Wrong build manifest")
    return {"source_sha256": digest(args.source_manifest), "runner_sha256": digest(args.runner), "build_manifest_sha256": digest(build / "SHA256SUMS"), "observer_sha256": GUARD_SHA}


def verify_log(log):
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"],
            "Expected exactly one stock Material suite")
    require(re.findall(r"^  \[M(\d+)\]", log, re.M) == STEPS,
            "Expected all ordered M1–M369 steps except M23")
    rows = re.findall(r"^case material \.\.\. ok \((\d+) steps, (\d+) checks,", log, re.M)
    require(len(rows) == 1 and int(rows[0][0]) == len(STEPS) and int(rows[0][1]) >= len(STEPS),
            "Material result has invalid step or assertion totals")
    checks = rows[0][1]
    require(re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", log, re.M) == [checks],
            "Material result lacks the successful runner summary")
    return int(checks)


def verify_run(out, window):
    require(window["stock_exit"] == 0 and not window["problems"]
            and not window["visible_owned_windows"] and not window["remaining_observed_processes"],
            "The suite, window observation, or private process cleanup failed")
    launches = window["guard_events"]
    chrome = window["launch_observations"]["chrome-primary"]
    require(len(launches) == 1 and launches[0]["role"] == "chrome-primary"
            and launches[0]["allowed"] and chrome["samples"] > 0,
            "A single guarded Chrome launch must be observed")
    flags = launches[0]["argv"]
    require(flags.count("--headless=new") == flags.count(UPDATER_ARG) == 1 and "--" not in flags,
            "Guarded Chrome arguments differ")

    checks = verify_log((out / "runner.log").read_text())
    artifacts = out / "artifacts"
    compact = data(artifacts / "browser-material-flexible-navigation-bar.json")
    require(compact.get("errors") == [] and "failure" not in compact, "Compact navigation report failed")
    metadata = run_path(str(ROOT / "verify-compact-predecessor.py"))["metadata"](compact)
    browser, adapter = metadata
    require(adapter.get("vendor") == "apple" and adapter.get("architecture", "").startswith("metal-") and adapter.get("isFallbackAdapter") is False, "Expected real Apple Metal WebGPU")
    adaptive = run_path(str(ROOT / "verify_adaptive_navigation_stock.py"))["verify"](artifacts, {"browser": browser, "adapter": adapter})
    return {"steps": len(STEPS), "checks": checks, "chromium_samples": chrome["samples"], "adaptive_navigation": adaptive}


def observe(args, out, pins):
    observer_code = run_path(str(GUARD))
    chrome = (args.chrome or observer_code["CHROME"]).resolve(strict=True)
    out.mkdir(mode=0o700)
    for name in ("guards", "guard-ledger", "tmp", "artifacts"):
        (out / name).mkdir(mode=0o700)
    guard = out / "guards" / "chrome"
    prefix = [sys.executable, "-B", str(GUARD), "__guard", "chrome", str(chrome)]
    guard.write_text("#!/bin/sh\nexec " + shlex.join(prefix) + ' "$@"\n')
    guard.chmod(0o700)
    command = [str(args.runner.resolve()), "--chrome", str(guard), "--chrome-arg", UPDATER_ARG,
               "--web-root", str(args.web_root.resolve()), "--artifacts", str(out / "artifacts"), "material"]
    excluded = DISPLAY_VARS | OVERRIDES | {observer_code["JET_MARKER"]}
    environment = {name: value for name, value in os.environ.items()
                   if name not in excluded and not (name.startswith("SURFACE_") and name.endswith("_FOCUSED"))}
    environment.update({"TMPDIR": str(out / "tmp"), observer_code["PRIVATE_ROOT"]: str(out)})
    json_file(out / "invocation.json", {"argv": command, "inputs": pins})
    windows = observer_code["WindowOwners"]()
    with (out / "runner.log").open("xb") as log:
        leader = subprocess.Popen(command, cwd=out, env=environment, start_new_session=True,
                                  stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
        observer = observer_code["Observer"](out, windows, "/usr/bin/false", chrome, leader.pid)
        deadline = monotonic() + args.timeout
        problems = []
        owned = {}
        try:
            while leader.poll() is None:
                owned = observer.sample()
                if observer.visible or any(not event["allowed"] for event in observer.guards.values()):
                    problems.append("Visible owned window or rejected pre-exec guard")
                    break
                if monotonic() > deadline:
                    problems.append("Stock Material suite timed out")
                    break
                sleep(0.05)
            if problems:
                observer_code["terminate_owned"](leader, owned)
            owned = observer.sample()
            cleanup = monotonic() + 8
            while owned and monotonic() < cleanup:
                sleep(0.1)
                owned = observer.sample()
            if owned:
                problems.append("Private process survived cleanup grace")
        finally:
            if leader.poll() is None or owned:
                observer_code["terminate_owned"](leader, owned)
            exit_status = leader.wait(timeout=8)
            window = observer.report(exit_status, observer.sample(), problems)
            json_file(out / "headless-window-observation.json", window)
    return window


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("source", "source-manifest", "runner", "web-root", "web-manifest"):
        parser.add_argument("--" + name, required=True, type=Path)
    for name in ("runner-sha256", "web-sha256"):
        parser.add_argument("--" + name, required=True)
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--timeout", type=int, default=1800)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check-inputs", action="store_true")
    mode.add_argument("--verify", action="store_true")
    mode.add_argument("--recover-receipt", action="store_true")
    mode.add_argument("--run", action="store_true")
    args = parser.parse_args()
    pins = inputs(args)
    if args.check_inputs:
        print("PASS corrected q74 runner and Web inputs. No executable started.")
        return
    require(args.out is not None, "Provide a private output under /tmp")
    out = args.out.resolve()
    temp = Path("/tmp").resolve()
    require(out.is_relative_to(temp) and out != temp, "Output must be private and under /tmp")
    if args.verify or args.recover_receipt:
        require(data(out / "invocation.json")["inputs"] == pins
                and data(out / "inputs-after.json") == pins, "Run used different inputs")
        result = verify_run(out, data(out / "headless-window-observation.json"))
        receipt = out / "verified-stock-material.json"
        if args.recover_receipt:
            require(not receipt.exists(), "Recovery cannot replace an existing receipt")
            json_file(receipt, result)
        elif receipt.exists():
            require(data(receipt) == json.loads(json.dumps(result)), "Stored Browser receipt differs")
    else:
        window = observe(args, out, pins)
        require(inputs(args) == pins, "Build inputs changed while the suite ran")
        json_file(out / "inputs-after.json", pins)
        result = verify_run(out, window)
        json_file(out / "verified-stock-material.json", result)
    print(f"PASS {result['steps']} Material steps and {result['checks']} checks. Receipt: {out}")


if __name__ == "__main__":
    main()
