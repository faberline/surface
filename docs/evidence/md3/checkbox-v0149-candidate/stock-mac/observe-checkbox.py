#!/usr/bin/env python3
"""Fail closed and observe the five explicit Surface headless E2E suites."""

import argparse
import ctypes as C
from hashlib import file_digest
import json
import os
from pathlib import Path
import re
import shlex
import signal
import subprocess
import sys
from time import monotonic, sleep

ROOT = Path(__file__).resolve().parent
APP_STAGE = ROOT
E2E = ROOT / "bundle/debug/surface-e2e"
CHROME = Path("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
OBSERVED_ROLES = ("chrome-primary", "chrome-bridge", "native")
GUARD_ROLES = ("chrome", "native", "compact", "jet")
PRIVATE_ROOT = "_SURFACE_DEFAULT_E2E_ROOT"
JET_MARKER = "_SURFACE_DEFAULT_E2E_JET"


def require(condition, description):
    if not condition:
        raise RuntimeError(description)


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def json_to(path, payload):
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n")


def read_json(path):
    return json.loads(path.read_text())


def checksums(path):
    items = {}
    for line in path.read_text().splitlines():
        digest, name = line.split("  ", 1)
        relative = Path(name)
        require(re.fullmatch(r"[0-9a-f]{64}", digest) and not relative.is_absolute()
                and ".." not in relative.parts and name not in items, f"bad manifest entry for {name}")
        items[name] = digest
    return items


def verify_entries(directory, values):
    for name, digest in values.items():
        require(sha(directory / name) == digest, f"input digest changed: {name}")


def verify_inputs(runner, runner_sha, stock):
    require(re.fullmatch(r"[0-9a-f]{64}", runner_sha), "provide the exact candidate runner SHA-256")
    seal = checksums(stock / "sealed-inputs.sha256")
    verify_entries(stock, seal)
    binaries = checksums(stock / "browser-binary-sha256.txt")
    verify_entries(stock / "bundle", binaries)
    verify_entries(stock / "bundle/debug", checksums(stock / "native-binaries.sha256"))
    require(sha(runner) == runner_sha and os.access(runner, os.X_OK), "runner is not the approved binary")
    require(binaries["debug/surface-e2e"] == runner_sha
            and runner == (stock / "bundle/debug/surface-e2e").resolve(), "runner is not the exact stock pin")
    source = stock / "browser-source.sha256"
    expected = "3a9aeda30a8ca3520afd04d9d4db8c26f3a900dc5aae20fad14e4d366d2544fe"
    require(sha(source) == expected, "wrong exact Checkbox source manifest")
    entries = checksums(source)
    require(len(entries) == 602, "wrong source file count")
    origins = read_json(stock / "source-origins.json")
    for kind in ("native", "web"):
        verify_entries(Path(origins[kind]), entries)
    return {"runner_sha256": runner_sha, "app_bundle_seal_sha256": sha(stock / "sealed-inputs.sha256"),
            "bundle_files_checked": len(binaries), "stock_files": binaries,
            "source_manifest_sha256": expected, "source_files_checked_per_origin": len(entries)}


def inside(raw, parent):
    if not raw:
        return False
    path = Path(raw)
    return path.is_absolute() and path.resolve().is_relative_to(parent) and path.resolve() != parent


def guard_verdict(role, args, env, temp):
    """Check the unmodified arguments before the stock process starts."""
    if role == "compact":
        return ("compact-sentinel", False, None)
    if role == "native":
        displays = ("DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET")
        ok = args == ["--headless", "--width", "720", "--height", "640"]
        ok = ok and all(name not in env for name in displays)
        return ("native", ok and inside(env.get("SURFACE_DEVTOOLS_SESSION"), temp), None)
    require(role == "chrome", f"unexpected guard role {role}")
    kind = "chrome-bridge" if env.get(JET_MARKER) == "1" else "chrome-primary"
    flags = [arg for arg in args if arg == "--headless" or arg.startswith("--headless=")]
    profiles = [arg.removeprefix("--user-data-dir=") for arg in args if arg.startswith("--user-data-dir=")]
    ok = flags == ["--headless=new"] and len(profiles) == 1 and inside(profiles[0], temp)
    return (kind, ok, str(Path(profiles[0]).resolve()) if ok else None)


def guarded_launch(role, executable, original):
    root = Path(os.environ[PRIVATE_ROOT]).resolve(strict=True)
    child_env = os.environ.copy()
    if role == "jet":
        child_env[JET_MARKER] = "1"
        os.execve(executable, [executable, *original], child_env)

    kind, allowed, profile = guard_verdict(role, original, child_env, root / "tmp")
    event = {"role": kind, "pid": os.getpid(), "ppid": os.getppid(), "allowed": allowed,
             "argv": original, "profile": profile}
    path = root / "guard-ledger" / f"{kind}-{os.getpid()}.json"
    staging = path.with_suffix(".writing")
    json_to(staging, event)
    staging.rename(path)
    if not allowed:
        print(f"pre-exec guard rejected {kind}. See the run guard ledger.", file=sys.stderr)
        raise SystemExit(81)
    os.execve(executable, [executable, *original], child_env)


def wrappers(out, bundle, chrome):
    executables = {"chrome": str(chrome), "native": str(bundle / "debug/surface-material-gallery"),
                   "jet": str(bundle / "debug/jet-bridge"), "compact": "/usr/bin/false"}
    paths = {}
    for role in GUARD_ROLES:
        path = out / "guards" / role
        command = [sys.executable, "-B", str(Path(__file__).resolve()), "__guard", role, executables[role]]
        path.write_text("#!/bin/sh\nexec " + shlex.join(command) + ' "$@"\n')
        path.chmod(0o700)
        paths[role] = str(path)
    return paths


def processes():
    """Return process identities without keeping metadata about unowned apps."""
    data = subprocess.check_output(["/bin/ps", "-axww", "-o", "pid=,ppid=,pgid=,lstart=,command="], text=True)
    rows = {}
    for line in data.splitlines():
        columns = line.split(None, 8)
        if len(columns) == 9:
            rows[int(columns[0])] = {
                "parent": int(columns[1]), "group": int(columns[2]),
                "start": " ".join(columns[3:8]), "command": columns[8],
            }
    return rows


class WindowOwners:
    def __init__(self):
        graphics = C.CDLL("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics")
        foundation = C.CDLL("/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation")
        pointer = C.c_void_p
        graphics.CGWindowListCopyWindowInfo.argtypes = [C.c_uint32, C.c_uint32]
        graphics.CGWindowListCopyWindowInfo.restype = pointer
        foundation.CFArrayGetCount.argtypes = [pointer]
        foundation.CFArrayGetCount.restype = C.c_long
        foundation.CFArrayGetValueAtIndex.argtypes = [pointer, C.c_long]
        foundation.CFArrayGetValueAtIndex.restype = pointer
        foundation.CFDictionaryGetValue.argtypes = [pointer, pointer]
        foundation.CFDictionaryGetValue.restype = pointer
        foundation.CFNumberGetValue.argtypes = [pointer, C.c_long, pointer]
        foundation.CFNumberGetValue.restype = C.c_bool
        foundation.CFRelease.argtypes = [pointer]
        self.graphics = graphics
        self.foundation = foundation
        self.owner_key = pointer.in_dll(graphics, "kCGWindowOwnerPID")

    def matching(self, owned):
        window_list = self.graphics.CGWindowListCopyWindowInfo(1, 0)
        require(window_list, "cannot read onscreen owner PIDs")
        try:
            counts = {}
            foundation = self.foundation
            for index in range(foundation.CFArrayGetCount(window_list)):
                window = foundation.CFArrayGetValueAtIndex(window_list, index)
                owner = foundation.CFDictionaryGetValue(window, self.owner_key)
                value = C.c_int32()
                if owner and foundation.CFNumberGetValue(owner, 3, C.byref(value)) and value.value in owned:
                    counts[value.value] = counts.get(value.value, 0) + 1
            return counts
        finally:
            self.foundation.CFRelease(window_list)


class Observer:
    def __init__(self, out, windows, gallery, chrome, group):
        self.out = out
        self.windows = windows
        self.gallery = str(gallery)
        self.chrome = str(chrome)
        self.group = group
        self.identities = {}
        self.series = {name: {"samples": 0, "pids": [], "detached_samples": 0}
                       for name in OBSERVED_ROLES}
        self.visible = []
        self.guards = {}

    def sample(self):
        for path in (self.out / "guard-ledger").glob("*.json"):
            if path not in self.guards:
                self.guards[path] = read_json(path)
        roots = {event["pid"]: event for event in self.guards.values() if event["allowed"]}
        profiles = {event["profile"]: event["role"] for event in roots.values() if event["profile"]}
        rows = processes()
        owned = {}
        for pid, row in rows.items():
            command = row["command"]
            identity = (pid, row["start"])
            role = self.identities.get(identity)
            if role is None and row["group"] == self.group:
                role = "runner-family"
            event = roots.get(pid)
            if event and command.startswith(self.gallery + " ") and event["role"] == "native":
                role = "native"
            if event and command.startswith(self.chrome + " ") and event["role"].startswith("chrome-"):
                role = event["role"]
            if role not in OBSERVED_ROLES and profiles and "Google Chrome" in command:
                for profile in profiles:
                    if "--user-data-dir=" + profile + " " in command + " ":
                        role = "owned-chrome-child"
                        break
            if role is not None:
                owned[pid] = role
        # Keep renderer/helper ownership after either browser is detached,
        # even for helpers with no profile in their command line.
        changed = True
        while changed:
            changed = False
            for pid, row in rows.items():
                if pid not in owned and row["parent"] in owned:
                    owned[pid] = "runner-family"
                    changed = True
        for pid, role in owned.items():
            self.identities[(pid, rows[pid]["start"])] = role
            if role in OBSERVED_ROLES:
                series = self.series[role]
                series["samples"] += 1
                if pid not in series["pids"]:
                    series["pids"].append(pid)
                event = roots[pid]
                if role.startswith("chrome-") and rows[pid]["parent"] != event["ppid"]:
                    series["detached_samples"] += 1
        visible = self.windows.matching(owned) if owned else {}
        self.visible.extend({"pid": pid, "role": owned[pid], "count": count}
                            for pid, count in visible.items())
        survivors = {pid: {"role": role, "start": rows[pid]["start"]} for pid, role in owned.items()}
        return survivors

    def report(self, exit_status, survivors, problems):
        chrome = [self.series[role] for role in OBSERVED_ROLES if role.startswith("chrome-")]
        return {"method": "CoreGraphics on-screen owner PIDs filtered to the verified private process family",
                "stock_exit": exit_status, "visible_owned_windows": self.visible,
                "remaining_observed_processes": sorted(survivors),
                "observations": {"chrome": {"samples": sum(group["samples"] for group in chrome),
                                            "pids": sorted(pid for group in chrome for pid in group["pids"])},
                                 "native_gallery": self.series["native"]},
                "launch_observations": self.series, "guard_events": list(self.guards.values()),
                "interval_seconds": 0.05, "problems": problems}


def verify_run(out, observation):
    from PIL import Image

    require(observation["stock_exit"] == 0 and not observation["problems"], "stock run or observer failed")
    require(not observation["visible_owned_windows"] and not observation["remaining_observed_processes"],
            "an owned window or survivor was observed")
    launches = observation["guard_events"]
    require(len(launches) == 3 and all(event["allowed"] for event in launches)
            and {event["role"] for event in launches} == set(OBSERVED_ROLES), "incomplete or rejected guarded launches")
    require(len({event["profile"] for event in launches if event["profile"]}) == 2, "Chrome profiles were not isolated")
    series = observation["launch_observations"]
    require(all(group["samples"] > 0 and len(group["pids"]) == 1 for group in series.values()),
            "each guarded executable needs a process observation")
    require(series["chrome-bridge"]["detached_samples"] > 0, "no Bridge Chrome samples after detach")

    artifacts = out / "artifacts"
    log_path = out / "runner.log"
    log = log_path.read_text()
    begins = re.findall(r"^case (\S+) \.\.\.$", log, re.M)
    rows = re.findall(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks,", log, re.M)
    require(tuple(begins) == tuple(row[0] for row in rows) == SUITES, "runner did not run exactly the ordered five")
    counts = {name: {"steps": int(steps), "checks": int(checks)} for name, steps, checks in rows}
    require(all(row["checks"] > 0 for row in counts.values()), "a suite ran no check")
    checks = sum(case["checks"] for case in counts.values())
    require(f"e2e result: 5 passed; 0 failed; {checks} checks;" in log
            and not re.search(r"^case \S+ \.\.\. (?:FAILED|skipped)", log, re.M), "bad runner summary")
    material = [int(step) for step in re.findall(r"^  \[M(\d+)\]", log, re.M)]
    require(material == [step for step in range(1, 282) if step != 23]
            and counts["material"]["steps"] == 280, "Material steps changed or out of order")
    ime_steps = [int(step) for step in re.findall(r"^  \[I(\d+)\]", log, re.M)]
    require(ime_steps == list(range(1, 21)) and counts["ime"] == {"steps": 20, "checks": 77},
            "IME receipt coverage changed")
    require(counts["bridge-web"]["steps"] == 24 and counts["devtools-headless"]["steps"] == 3,
            "Bridge or headless DevTools coverage changed")

    smoke, ime, side = [read_json(artifacts / name) for name in (
        "browser-smoke.json", "browser-ime.json", "browser-material-side-sheet.json")]
    require(smoke["errors"] == ime["errors"] == side["errors"] == [], "browser reported errors")
    require(smoke["browser"] == side["browser"] and ime["committed"] == "繁體中文、简体中文",
            "browser or IME report disagrees with expected fixture")
    for candidate in artifacts.glob("browser-material*.json"):
        errors = read_json(candidate).get("errors")
        require(errors is None or errors == [], f"material page errors in {candidate.name}")
    metadata = {"browser": smoke["browser"], "adapter": side["adapter"]}
    require({key: smoke["adapter"][key] for key in side["adapter"]} == side["adapter"], "WebGPU adapter disagrees")
    bridge = read_json(artifacts / "bridge-web-result.json")
    require(bridge["webgpu_ready"] is True and bridge["owned_browser_exited"] is True,
            "Bridge did not prove WebGPU or browser cleanup")
    native = read_json(artifacts / "devtools-headless-result.json")
    require(native["width"] == 720 and native["height"] == 640 and native["scale"] == 1
            and native["action_rendered"] is True and native["screenshot_transports_match"] is True,
            "native evidence is missing offscreen actions or transport equality")
    checkbox_path = artifacts / "browser-material-checkbox.json"
    checkbox = read_json(checkbox_path)
    require(checkbox["browser"] == metadata["browser"] and checkbox["adapter"] == metadata["adapter"]
            and checkbox["reduced_motion"] is True and checkbox["errors"] == []
            and "failure" not in checkbox, "Checkbox run or metadata failed")
    compact = checkbox["compact"]
    require(compact["viewport"] == [320, 640] and compact["dark"] is True
            and compact["resolved"]["host"] == "surface", "compact checkbox input receipt disagrees")
    checkbox_images = {
        "material-checkbox-wide-focused.png": (1920, 1688),
        "material-checkbox-wide-held.png": (1920, 1688),
        "material-checkbox-compact-invalid.png": (640, 1368),
        "material-checkbox-compact-resolved.png": (640, 1368),
    }
    require({p.name for p in artifacts.glob("material-checkbox-*.png")} == set(checkbox_images),
            "Checkbox output must contain exactly the four original success images")
    pngs = {}
    for name, dimensions in (("bridge-web.png", None), ("devtools-headless-screenshot.png", (720, 640)),
                             *checkbox_images.items()):
        path = artifacts / name
        with Image.open(path) as image:
            image.verify()
        with Image.open(path) as image:
            image.load()
            require(image.format == "PNG" and (dimensions is None or image.size == dimensions)
                    and image.width > 0 and image.height > 0, f"bad screenshot {name}")
            pngs[name] = {"size": image.size, "sha256": sha(path)}
    return {**metadata, "suite_arguments": SUITES, "cases": counts, "total_checks": checks, "page_errors": 0,
            "headless_transport_pngs": {name: value for name, value in pngs.items()
                                        if name not in checkbox_images},
            "checkbox_pngs": {name: pngs[name] for name in checkbox_images},
            "checkbox_report_sha256": sha(checkbox_path), "guard_and_window_checks": "passed"}


def terminate_owned(leader, owned):
    if leader.poll() is None:
        try:
            os.killpg(leader.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    for pid, record in owned.items():
        current = processes().get(pid)
        if current and current["start"] == record["start"]:
            try:
                os.kill(pid, signal.SIGTERM)
            except ProcessLookupError:
                pass


def observe(out, runner, approved_sha, stock, timeout):
    chrome = CHROME.resolve(strict=True)
    require(out.is_relative_to(Path("/tmp").resolve()) and out != Path("/tmp").resolve(),
            "choose a fresh directory under /tmp")
    require(sys.platform == "darwin" and os.access(chrome, os.X_OK), "requires the standard macOS Chrome executable")
    bundle = stock / "bundle"
    inputs = verify_inputs(runner, approved_sha, stock)
    out.mkdir(mode=0o700)
    for name in ("guards", "guard-ledger", "tmp", "artifacts"):
        (out / name).mkdir(mode=0o700)
    guards = wrappers(out, bundle, chrome)
    command = [str(runner), "--chrome", guards["chrome"],
               "--web-root", str(bundle / "web/release"),
               "--bridge-web", str(bundle / "web/devtools/material-gallery"),
               "--jet-bridge", guards["jet"], "--devtools-bin", str(bundle / "debug/surface-devtools"),
               "--native-bin", guards["native"], "--compact-bin", guards["compact"],
               "--artifacts", str(out / "artifacts"), *SUITES]
    environment = os.environ.copy()
    for name in ("SURFACE_CHROME_ARGS", "SURFACE_CHROME", "CHROME_PATH", JET_MARKER, "SURFACE_BRIDGE_SESSION",
                 "SURFACE_DEVTOOLS_SESSION", "DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"):
        environment.pop(name, None)
    environment.update({"TMPDIR": str(out / "tmp"), PRIVATE_ROOT: str(out)})
    json_to(out / "invocation.json", {"argv": command, "suite_arguments": SUITES,
                                      "injected_chrome_flags": [], "inputs": inputs})
    windows = WindowOwners()
    with (out / "runner.log").open("xb") as log:
        leader = subprocess.Popen(command, cwd=out, env=environment, start_new_session=True,
                                  stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
        observer = Observer(out, windows, (bundle / "debug/surface-material-gallery").resolve(), chrome, leader.pid)
        deadline = monotonic() + timeout
        owned = {}
        problems = []
        try:
            while leader.poll() is None:
                owned = observer.sample()
                if observer.visible or any(not event["allowed"] for event in observer.guards.values()):
                    problems.append("unexpected window or pre-exec guard rejection")
                    break
                if monotonic() >= deadline:
                    problems.append("stock runner timed out")
                    break
                sleep(0.05)
            if problems:
                terminate_owned(leader, owned)
            owned = observer.sample()
            cleanup_deadline = monotonic() + 8
            while owned and monotonic() < cleanup_deadline:
                sleep(0.1)
                owned = observer.sample()
            if owned:
                problems.append("owned processes survived the bounded shutdown grace")
        finally:
            if leader.poll() is None or owned:
                terminate_owned(leader, owned)
            try:
                exit_status = leader.wait(timeout=8)
            except subprocess.TimeoutExpired:
                leader.kill()
                exit_status = leader.wait(timeout=5)
                problems.append("runner required forceful cleanup")
            receipt = observer.report(exit_status, owned, problems)
            json_to(out / "headless-window-observation.json", receipt)
    require(verify_inputs(runner, approved_sha, stock) == inputs, "sealed assets changed during the run")
    verdict = verify_run(out, receipt)
    json_to(out / "verified-checkbox-stock.json", verdict)
    print(json.dumps({"out": str(out), "checks": verdict["total_checks"],
                      "launches": receipt["launch_observations"], "windows": receipt["visible_owned_windows"],
                      "survivors": receipt["remaining_observed_processes"]}, indent=2))


def self_test():
    # Exercise only the argument predicates. Start no executable.
    temp = (Path("/tmp").resolve() / "private-observer-placeholder")
    profile = "--user-data-dir=" + str(temp / "profile")
    native = ["--headless", "--width", "720", "--height", "640"]
    session = {"SURFACE_DEVTOOLS_SESSION": str(temp / "session.json")}
    require(guard_verdict("chrome", [profile, "--headless=new"], {}, temp)[:2] == ("chrome-primary", True),
            "primary Chrome guard failed")
    require(guard_verdict("chrome", [profile, "--headless=new"], {JET_MARKER: "1"}, temp)[:2]
            == ("chrome-bridge", True), "Bridge Chrome guard failed")
    for args in ([profile], [profile, "--headless=old"], [profile, "--headless=new", "--headless=false"],
                 ["--user-data-dir=/tmp/foreign", "--headless=new"], ["--headless=new"]):
        require(not guard_verdict("chrome", args, {}, temp)[1], f"unsafe Chrome args passed: {args}")
    require(guard_verdict("native", native, session, temp)[1], "offscreen native setup rejected")
    require(not guard_verdict("native", native, {**session, "DISPLAY": ":1"}, temp)[1], "native display accepted")
    require(not guard_verdict("native", native[1:], session, temp)[1], "windowed native accepted")
    require(not guard_verdict("compact", [], {}, temp)[1], "compact sentinel allowed")
    print("PASS pure guard predicates; no executable started")


def main():
    if len(sys.argv) >= 4 and sys.argv[1] == "__guard":
        guarded_launch(sys.argv[2], sys.argv[3], sys.argv[4:])
        return
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, help="fresh private /tmp directory")
    parser.add_argument("--runner", type=Path, default=E2E)
    parser.add_argument("--runner-sha256", help="approved runner's exact SHA-256")
    parser.add_argument("--stock", type=Path, default=APP_STAGE)
    parser.add_argument("--timeout", type=int, default=1800)
    parser.add_argument("--self-test", action="store_true", help="test guard logic without starting anything")
    args = parser.parse_args()
    if not __debug__:
        parser.error("run without -O")
    if args.self_test:
        self_test()
        return
    if not args.out or not args.runner_sha256 or args.timeout <= 0:
        parser.error("running requires a new --out directory, approved --runner-sha256, and positive --timeout")
    observe(args.out.resolve(), args.runner.resolve(strict=True), args.runner_sha256,
            args.stock.resolve(strict=True), args.timeout)


if __name__ == "__main__":
    main()
