#!/usr/bin/env python3
"""Run the focused text field label motion case with an observed headless process."""

import argparse
import hashlib
import os
from pathlib import Path
import re
from runpy import run_path
import shlex
import subprocess
import sys
from time import monotonic, sleep


ROOT = Path(__file__).resolve().parent
OBSERVER = ROOT / "observe-tabs.py"
OBSERVER_SHA = "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7"
STOCK_PASSED = Path("/tmp/surface-md3-textbox-values-stock-v0166.z81UZBVd/stock-mac-v0166-passed")
WEB_SHA = "5ee0d1175d5c590d39b1c4d4438e241457e391a4abc7711fcf9b73334802ca43"
if hashlib.sha256(OBSERVER.read_bytes()).hexdigest() != OBSERVER_SHA:
    raise SystemExit("the pinned headless guard implementation differs")
OBS = run_path(str(OBSERVER))
require, sha, write_json = OBS["require"], OBS["sha"], OBS["json_to"]


def owned_details(owned):
    if not owned:
        return []
    result = subprocess.run(
        ["/bin/ps", "-ww", "-p", ",".join(map(str, sorted(owned))),
         "-o", "pid=,ppid=,pgid=,state=,lstart=,command="],
        capture_output=True, text=True, check=False,
    )
    rows = []
    for line in result.stdout.splitlines():
        pid, parent, group, state, *rest = line.split(None, 9)
        start = " ".join(rest[:5])
        command = rest[5]
        record = owned.get(int(pid))
        if record and start == record["start"]:
            rows.append({"pid": int(pid), "ppid": int(parent), "pgid": int(group),
                         "role": record["role"], "state": state, "command": command})
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--runner-sha256", required=True)
    parser.add_argument("--web-root", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    require(STOCK_PASSED.is_file(), "wait for the controller's exact v0.1.66 stock completion marker")
    runner, web, out = args.runner.resolve(strict=True), args.web_root.resolve(strict=True), args.out.resolve()
    require(sha(runner) == args.runner_sha256 and os.access(runner, os.X_OK), "focused runner pin differs")
    require(out.is_relative_to(ROOT) and out != ROOT,
            "use a fresh private /tmp path")
    root = web / "material-gallery"

    def web_hashes():
        return {str(path.relative_to(root)): sha(path) for path in sorted(root.rglob("*")) if path.is_file()}

    assets = web_hashes()
    manifest = ROOT / "web.sha256"
    require(sha(manifest) == WEB_SHA and OBS["checksums"](manifest) == assets,
            "the exact q67 development Web pack differs")
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
    environment.update({"TMPDIR": str(out / "tmp"), OBS["PRIVATE_ROOT"]: str(out), "SURFACE_TEXT_FIELD_LABEL_MOTION_FOCUSED": "1"})
    write_json(out / "invocation.json", {"argv": command, "scope": "q67 focused M325–M350 development",
                                        "web_manifest_sha256": WEB_SHA,
                                        "runner_sha256": args.runner_sha256, "web_files": assets})
    windows = OBS["WindowOwners"]()
    with (out / "runner.log").open("xb") as log:
        leader = subprocess.Popen(command, cwd=out, env=environment, start_new_session=True,
                                  stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
        observer = OBS["Observer"](out, windows, "/usr/bin/false", chrome, leader.pid)
        stop = monotonic() + 360
        problems = []
        owned = {}
        shutdown = {}
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
            grace_start = monotonic()
            cleanup = grace_start + 8
            shutdown["initial"] = owned_details(owned)
            while owned and monotonic() < cleanup:
                sleep(0.1)
                owned = observer.sample()
            shutdown["after_grace"] = owned_details(owned)
            shutdown["elapsed_seconds"] = round(monotonic() - grace_start, 3)
            if owned:
                problems.append("owned process survived cleanup grace")
        finally:
            if leader.poll() is None or owned:
                OBS["terminate_owned"](leader, owned)
            code = leader.wait(timeout=8)
            shutdown["after_cleanup"] = owned_details(observer.sample())
            write_json(out / "owned-shutdown.json", shutdown)
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
        require(re.findall(r"^  \[(M\d+)\]", text, re.M) == [f"M{step}" for step in range(325, 351)],
                "runner did not exercise all 26 focused steps")
    raise SystemExit(code)


if __name__ == "__main__":
    main()
