#!/usr/bin/env python3
"""Run the exact scoped v0.1.72 field-pair case under the sealed headless guard."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
from runpy import run_path
import shlex
import subprocess
import sys
from time import monotonic, sleep

from evidence_common import HEX, seal


ROOT = Path(__file__).resolve().parent
OBSERVER = ROOT / "observe-tabs.py"
OBSERVER_SHA = "839fb09917811135fa0699c8d7b9a599014633b4661b1fde59003ea185ef3365"
UPDATER_ARG = "--disable-updater-scheduler"
if hashlib.sha256(OBSERVER.read_bytes()).hexdigest() != OBSERVER_SHA:
    raise SystemExit("the pinned headless guard implementation differs")
OBS = run_path(str(OBSERVER))
require, sha, write_json = OBS["require"], OBS["sha"], OBS["json_to"]
PREP = run_path(str(ROOT / "prepare-focused.py"))


def inputs(launcher_sha, web_sha, approved_runner):
    seal(ROOT, "launcher.sha256", launcher_sha)
    scope = PREP["check_scope"]()
    gallery = PREP["check_gallery"]()
    require(gallery["web_manifest_sha256"] == web_sha and gallery["web_compatible_test_only_delta"] == PREP["WEB_DELTA_R7"],
            "Gallery must match the R4 build with only the approved R8 test changes")
    runner = ROOT / "pins/surface-e2e"
    runner_sha = (ROOT / "runner.sha256").read_text().strip()
    require(sha(runner) == runner_sha == approved_runner and HEX.fullmatch(approved_runner or "")
            and runner.is_file() and os.access(runner, os.X_OK), "Focused runner pin differs")
    require(
        (ROOT / "pins/focused-source-manifest.sha256").read_text().strip() == scope["focused_source_manifest_sha256"],
        "runner is not pinned to the same focused source",
    )
    return {**scope, **gallery, "runner_sha256": runner_sha}


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
    parser.add_argument("--out", type=Path, help="new directory inside this private lane")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--run", action="store_true")
    mode.add_argument("--verify-inputs-only", action="store_true")
    parser.add_argument("--launcher-sha256", required=True)
    parser.add_argument("--web-sha256", required=True)
    parser.add_argument("--runner-sha256", required=True)
    args = parser.parse_args()
    if not __debug__:
        parser.error("run without -O")
    pins = (args.launcher_sha256, args.web_sha256, args.runner_sha256)
    sealed = inputs(*pins)
    if args.verify_inputs_only:
        print(json.dumps(sealed, ensure_ascii=False, indent=2))
        return
    require(args.out is not None, "provide a fresh output for the authorized scoped run")
    runner, web, out = ROOT / "pins/surface-e2e", ROOT / "web", args.out.resolve()
    require(out.is_relative_to(ROOT) and out != ROOT and not out.exists(),
            "use a fresh private output inside this lane")
    for operation in ("guards", "guard-ledger", "tmp", "artifacts"):
        (out / operation).mkdir(parents=True, mode=0o700, exist_ok=False)
    chrome = OBS["CHROME"].resolve(strict=True)
    guard = out / "guards" / "chrome"
    prefix = [sys.executable, "-B", str(OBSERVER), "__guard", "chrome", str(chrome)]
    guard.write_text("#!/bin/sh\nexec " + shlex.join(prefix) + ' "$@"\n')
    guard.chmod(0o700)
    command = [str(runner), "--chrome", str(guard), "--chrome-arg", UPDATER_ARG,
               "--web-root", str(web), "--artifacts", str(out / "artifacts"), "material"]
    environment = os.environ.copy()
    for name in ("SURFACE_CHROME_ARGS", "SURFACE_CHROME", "CHROME_PATH", "SURFACE_BRIDGE_SESSION",
                 "SURFACE_DEVTOOLS_SESSION", "DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET",
                 "SURFACE_TEXT_FIELD_LABEL_MOTION_FOCUSED", "SURFACE_TEXT_FIELD_LABEL_COLOR_FOCUSED",
                 "SURFACE_TEXT_FIELD_PLACEHOLDER_FOCUSED", "SURFACE_TEXT_FIELD_BOUNDARY_FOCUSED",
                 "SURFACE_TEXT_FIELD_CONTRAST_FOCUSED", "SURFACE_TEXT_FIELD_DISABLED_CONTRAST_FOCUSED",
                 PREP["ENV_FLAG"], OBS["JET_MARKER"]):
        environment.pop(name, None)
    environment.update({"TMPDIR": str(out / "tmp"), OBS["PRIVATE_ROOT"]: str(out), PREP["ENV_FLAG"]: "1"})
    write_json(out / "invocation.json", {
        "argv": command, "scope": "q72 R8 focused M241–M247 and M357–M362", "suite_arguments": ["material"],
        "injected_chrome_flags": [UPDATER_ARG], "inputs": sealed,
    })
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
    after = inputs(*pins)
    require(after == sealed, "input bytes changed")
    write_json(out / "inputs-after.json", after)
    require(not window["problems"] and not window["visible_owned_windows"]
            and not window["remaining_observed_processes"], "headless window/process guard failed")
    events = window["guard_events"]
    require(len(events) == 1 and events[0]["role"] == "chrome-primary" and events[0]["allowed"]
            and events[0]["argv"].count(UPDATER_ARG) == events[0]["argv"].count("--headless=new") == 1
            and "--" not in events[0]["argv"]
            and window["launch_observations"]["chrome-primary"]["samples"] > 0, "missing guarded Chrome proof")
    print(f"Headless proof: {out}. Runner status {code}.")
    if code == 0:
        text = (out / "runner.log").read_text()
        checks = re.findall(r"^case material \.\.\. ok \(13 steps, (\d+) checks,", text, re.M)
        require(re.findall(r"^case (\S+) \.\.\.$", text, re.M) == ["material"]
                and re.findall(r"^  \[(M\d+)\]", text, re.M) == PREP["STEPS"]
                and len(checks) == 1 and int(checks[0]) >= 13
                and re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", text, re.M) == checks,
                "Runner did not complete the exact 13 focused Material steps")
    raise SystemExit(code)


if __name__ == "__main__":
    main()
