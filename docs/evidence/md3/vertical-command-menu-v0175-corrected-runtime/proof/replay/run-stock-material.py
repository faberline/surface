#!/usr/bin/env python3
"""Observe the corrected stock Surface v0.1.75 Material suite on headless macOS Chrome."""

import argparse
import os
from pathlib import Path
import re
from runpy import run_path
import shlex
import struct
import subprocess
import sys
from time import monotonic, sleep

from evidence_common import data, digest, json_file, path_entries, require, source_tree, verify_entries

ROOT = Path(__file__).resolve().parent
GUARD = ROOT / "observe-focused-original.py"
GUARD_SHA = "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7"
SOURCE_SHA = "8a2037d8b5fb11afcb226f5b27b41656d7c3dd428b09db9c4aaf2d711cacf2d0"
SOURCE_TREE = "8046e25a630284411e1ebf9f9d16d1004b02651b"
ORIGINAL_WEB_SHA = "0922d45798b7ddb779fb9710fe5936a35228ff1b203fa8a057108efba4e4ff44"
STEPS = [str(i) for i in range(1, 373) if i != 23]
UPDATER_ARG = "--disable-updater-scheduler"
DISPLAY_VARS = {"DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"}
OVERRIDES = {"SURFACE_CHROME", "SURFACE_CHROME_ARGS", "CHROME_PATH",
             "SURFACE_BRIDGE_SESSION", "SURFACE_DEVTOOLS_SESSION"}
MENU_REPORTS = ["browser-material-command-menu.json", "browser-material-vertical-menu.json"]
MENU_IMAGES = [
    "material-command-menu-wide.png",
    "material-command-menu-compact.png",
    "material-command-menu-compact-footer.png",
    "material-vertical-menu-standard.png",
    "material-vertical-menu-vibrant.png",
    "material-vertical-menu-dark-rtl.png",
]


def inputs(args):
    require(digest(GUARD) == GUARD_SHA, "Different macOS observer source")
    require(digest(args.source_manifest) == SOURCE_SHA, "Different corrected v0.1.75 source manifest")
    entries = path_entries(args.source_manifest)
    require(source_tree(args.source.resolve(strict=True), entries) == SOURCE_TREE,
            "Different corrected v0.1.75 source tree")
    require(args.runner.is_file() and os.access(args.runner, os.X_OK)
            and digest(args.runner) == args.runner_sha256, "Different or unavailable runner")
    require(digest(args.web_manifest) == args.web_sha256, "Different Gallery manifest")
    require(args.web_sha256 != ORIGINAL_WEB_SHA, "The original v0.1.75 Web build is not the corrected build")
    gallery = args.web_root.resolve(strict=True) / "material-gallery"
    web_entries = path_entries(args.web_manifest, allow_dot_prefix=True)
    require("index.html" in web_entries and any(name.endswith(".wasm") for name in web_entries),
            "Gallery manifest needs HTML and Wasm")
    verify_entries(gallery, web_entries)
    actual = {path.relative_to(gallery).as_posix() for path in gallery.rglob("*") if path.is_file()}
    require(actual == web_entries.keys(), "Gallery contains unpinned files")
    return {"source_tree": SOURCE_TREE, "source_sha256": SOURCE_SHA,
            "runner_sha256": args.runner_sha256, "web_manifest_sha256": args.web_sha256,
            "observer_sha256": GUARD_SHA}


def verify_log(log):
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"],
            "Expected exactly one stock Material suite")
    require(re.findall(r"^  \[M(\d+)\]", log, re.M) == STEPS,
            "Expected all ordered M1–M372 steps except M23")
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
    files = {}
    for name in MENU_REPORTS:
        path = artifacts / name
        report = data(path)
        require(report["errors"] == [] and "failure" not in report, f"Menu report has browser errors: {name}")
        fallback = report.get("adapter", {}).get("isFallbackAdapter")
        require(isinstance(fallback, bool), f"Missing WebGPU adapter result: {name}")
        files[name] = {"sha256": digest(path), "webgpu_fallback": fallback}
    for name in MENU_IMAGES:
        path = artifacts / name
        with path.open("rb") as stream:
            header = stream.read(24)
        require(len(header) == 24 and header[:8] == b"\x89PNG\r\n\x1a\n" and header[12:16] == b"IHDR",
                f"Invalid screenshot: {name}")
        width, height = struct.unpack(">II", header[16:24])
        require(width > 0 and height > 0, f"Invalid dimensions: {name}")
        files[name] = {"sha256": digest(path), "size": [width, height]}
    return {"steps": len(STEPS), "checks": checks, "chromium_samples": chrome["samples"], "menus": files}


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
    mode.add_argument("--run", action="store_true")
    args = parser.parse_args()
    pins = inputs(args)
    if args.check_inputs:
        print("PASS exact corrected v0.1.75 inputs. No executable started.")
        return
    require(args.out is not None, "Provide a private output under /tmp")
    out = args.out.resolve()
    temp = Path("/tmp").resolve()
    require(out.is_relative_to(temp) and out != temp, "Output must be private and under /tmp")
    if args.verify:
        require(data(out / "invocation.json")["inputs"] == pins
                and data(out / "inputs-after.json") == pins, "Run used different inputs")
        result = verify_run(out, data(out / "headless-window-observation.json"))
    else:
        window = observe(args, out, pins)
        require(inputs(args) == pins, "Build inputs changed while the suite ran")
        json_file(out / "inputs-after.json", pins)
        result = verify_run(out, window)
        json_file(out / "verified-stock-material.json", result)
    print(f"PASS {result['steps']} Material steps and {result['checks']} checks. Receipt: {out}")


if __name__ == "__main__":
    main()
