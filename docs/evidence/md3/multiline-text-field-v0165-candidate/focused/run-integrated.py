#!/usr/bin/env python3
"""Run the focused Multiline Text Field browser case with an observed headless process."""

import argparse
import os
from pathlib import Path
import re
from runpy import run_path
import shlex
import subprocess
import sys
from time import monotonic, sleep


ROOT = Path(__file__).resolve().parent
OBSERVER = Path("/tmp/surface-md3-tabs-stock-observer.922n2GrO/observe-tabs.py")
OBS = run_path(str(OBSERVER))
require, sha, write_json = OBS["require"], OBS["sha"], OBS["json_to"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--runner-sha256", required=True)
    parser.add_argument("--web-root", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    require(sha(OBSERVER) == "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7",
            "original headless guard implementation differs")
    runner, web, out = args.runner.resolve(strict=True), args.web_root.resolve(strict=True), args.out.resolve()
    require(sha(runner) == args.runner_sha256 and os.access(runner, os.X_OK), "focused runner pin differs")
    require(out.is_relative_to(Path("/tmp").resolve()) and out != Path("/tmp").resolve(),
            "use a fresh private /tmp path")
    root = web / "material-gallery"

    def web_hashes():
        return {str(path.relative_to(root)): sha(path) for path in sorted(root.rglob("*")) if path.is_file()}

    assets = web_hashes()
    require("index.html" in assets and any(name.endswith(".wasm") for name in assets), "missing real Gallery pack")
    for operation in ("guards", "guard-ledger", "tmp", "artifacts"):
        (out / operation).mkdir(parents=True, mode=0o700, exist_ok=False)
    chrome = OBS["CHROME"].resolve(strict=True)
    guard = out / "guards" / "chrome"
    prefix = [sys.executable, "-B", str(OBSERVER), "__guard", "chrome", str(chrome)]
    guard.write_text("#!/bin/sh\nexec " + shlex.join(prefix) + ' "$@"\n')
    guard.chmod(0o700)
    command = [str(runner), "--chrome", str(guard), "--web-root", str(web), "--artifacts", str(out / "artifacts"), "material"]
    environment = os.environ.copy()
    for name in ("SURFACE_CHROME_ARGS", "SURFACE_CHROME", "CHROME_PATH", "SURFACE_BRIDGE_SESSION",
                 "SURFACE_DEVTOOLS_SESSION", "DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET", OBS["JET_MARKER"]):
        environment.pop(name, None)
    environment.update({"TMPDIR": str(out / "tmp"), OBS["PRIVATE_ROOT"]: str(out), "SURFACE_MULTILINE_TEXT_FIELD_FOCUSED": "1"})
    write_json(out / "invocation.json", {"argv": command, "scope": "focused development pack",
                                        "runner_sha256": args.runner_sha256, "web_files": assets})
    windows = OBS["WindowOwners"]()
    with (out / "runner.log").open("xb") as log:
        leader = subprocess.Popen(command, cwd=out, env=environment, start_new_session=True,
                                  stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
        observer = OBS["Observer"](out, windows, "/usr/bin/false", chrome, leader.pid)
        stop = monotonic() + 360
        problems = []
        owned = {}
        try:
            while leader.poll() is None:
                owned = observer.sample()
                if observer.visible or any(not event["allowed"] for event in observer.guards.values()):
                    problems.append("visible owned window or rejected guard")
                    break
                if monotonic() > stop:
                    problems.append("focused runner timed out")
                    break
                sleep(0.05)
            if problems:
                OBS["terminate_owned"](leader, owned)
            owned = observer.sample()
            cleanup = monotonic() + 8
            while owned and monotonic() < cleanup:
                sleep(0.1)
                owned = observer.sample()
            if owned:
                problems.append("owned process survived cleanup grace")
        finally:
            if leader.poll() is None or owned:
                OBS["terminate_owned"](leader, owned)
            code = leader.wait(timeout=8)
            window = observer.report(code, owned, problems)
            write_json(out / "headless-window-observation.json", window)
    require(assets == web_hashes() and sha(runner) == args.runner_sha256, "input bytes changed")
    require(not window["problems"] and not window["visible_owned_windows"]
            and not window["remaining_observed_processes"], "headless window/process guard failed")
    events = window["guard_events"]
    require(len(events) == 1 and events[0]["role"] == "chrome-primary" and events[0]["allowed"]
            and window["launch_observations"]["chrome-primary"]["samples"] > 0, "missing guarded Chrome proof")
    print(f"Headless proof: {out}. Runner status {code}.")
    if code == 0:
        text = (out / "runner.log").read_text()
        require(re.findall(r"^  \[(M\d+)\]", text, re.M) == ["M341", "M342", "M343", "M344", "M345"],
                "runner did not exercise all five focused steps")
    raise SystemExit(code)


if __name__ == "__main__":
    main()
