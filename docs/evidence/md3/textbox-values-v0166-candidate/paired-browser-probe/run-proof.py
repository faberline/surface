"""Run the independent Gallery textbox proof with a pinned headless browser guard."""

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
GUARD_SHA = "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7"
NODE = Path("/opt/homebrew/bin/node")
OBS = run_path(str(OBSERVER))
require, sha, write_json, read = (
    OBS["require"], OBS["sha"], OBS["json_to"], OBS["read_json"],
)


def check_assets(directory, pinned_manifest, digest):
    require(re.fullmatch(r"[a-f0-9]{64}", digest) and sha(pinned_manifest) == digest,
            "the independent Web asset manifest hash differs")
    pins = {}
    for line in pinned_manifest.read_text().splitlines():
        expected, raw = line.split("  ", 1)
        name = Path(raw).name
        require(re.fullmatch(r"[a-f0-9]{64}", expected) and name not in pins,
                "invalid asset pin entry")
        pins[name] = expected
    package = read(directory / "surface-web.json")
    require(package["app"] == "material-gallery" and package["profile"] == "release"
            and package["rustc_target"] == "wasm32-unknown-unknown"
            and package["entry"] == "index.html", "wrong Gallery pack metadata")
    assets = {item["path"]: item for item in package["files"]}
    require(set(assets) == {"index.html", package["js"], package["wasm"]}
            and all(name == Path(name).name and assets[name]["sha256"] == pins[name]
                    and assets[name]["bytes"] == (directory / name).stat().st_size
                    for name in assets)
            and set(pins) == {path.name for path in directory.iterdir()}
            == {*assets, "surface-web.json"}, "Gallery pack has a wrong asset inventory")
    require(all((directory / name).is_file() and not (directory / name).is_symlink()
                and sha(directory / name) == expected for name, expected in pins.items()),
            "Gallery pack bytes differ")
    return pins


def proof(args):
    require(sys.platform == "darwin" and sha(OBSERVER) == GUARD_SHA,
            "the proven macOS guard differs or is unavailable")
    root, manifest, out = args.web.resolve(strict=True), args.manifest.resolve(strict=True), args.out.resolve()
    require(out.is_relative_to(ROOT) and out != ROOT, "use a fresh private proof directory")
    assets = check_assets(root, manifest, args.manifest_sha256)
    chrome = OBS["CHROME"].resolve(strict=True)
    require(os.access(chrome, os.X_OK) and os.access(NODE, os.X_OK), "missing Chrome or Node")
    out.mkdir(mode=0o700)
    for part in ("guards", "guard-ledger", "tmp"):
        (out / part).mkdir(mode=0o700)
    guard = out / "guards/chrome"
    prefix = [sys.executable, "-B", str(OBSERVER), "__guard", "chrome", str(chrome)]
    guard.write_text("#!/bin/sh\nexec " + shlex.join(prefix) + ' "$@"\n')
    guard.chmod(0o700)
    probe = ROOT / "probe.mjs"
    command = [str(NODE), str(probe), str(root), str(out), str(guard), args.expect]
    environment = os.environ.copy()
    for name in ("SURFACE_CHROME_ARGS", "SURFACE_CHROME", "CHROME_PATH", "SURFACE_BRIDGE_SESSION",
                 "SURFACE_DEVTOOLS_SESSION", "DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET",
                 OBS["JET_MARKER"], "NODE_OPTIONS"):
        environment.pop(name, None)
    environment.update({"TMPDIR": str(out / "tmp"), OBS["PRIVATE_ROOT"]: str(out)})
    write_json(out / "asset-pins.json", assets)
    probe_sha = sha(probe)
    chrome_sha = sha(chrome)
    write_json(out / "invocation.json", {"scope": "independent focused Gallery development pack",
        "expect": args.expect, "argv": command, "manifest_sha256": args.manifest_sha256,
        "probe_sha256": probe_sha, "chrome_launcher_sha256": chrome_sha,
        "headless_guard_sha256": GUARD_SHA, "assets": assets})
    windows = OBS["WindowOwners"]()
    with (out / "runner.log").open("xb") as log:
        leader = subprocess.Popen(command, cwd=out, env=environment, start_new_session=True,
                                  stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
        observer = OBS["Observer"](out, windows, "/usr/bin/false", chrome, leader.pid)
        deadline = monotonic() + 120
        problems = []
        owned = {}
        try:
            while leader.poll() is None:
                owned = observer.sample()
                if observer.visible or any(not event["allowed"] for event in observer.guards.values()):
                    problems.append("visible owned window or rejected guard")
                    break
                if monotonic() >= deadline:
                    problems.append("scoped proof timed out")
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
    require(check_assets(root, manifest, args.manifest_sha256) == assets
            and sha(probe) == probe_sha and sha(chrome) == chrome_sha, "input bytes changed")
    require(not window["problems"] and not window["visible_owned_windows"]
            and not window["remaining_observed_processes"], "headless window/process guard failed")
    events = window["guard_events"]
    observed = window["launch_observations"]["chrome-primary"]
    require(len(events) == 1 and events[0]["role"] == "chrome-primary" and events[0]["allowed"] is True
            and observed["pids"] == [events[0]["pid"]] and observed["samples"] > 0,
            "missing guarded Chrome observations")
    require(code == 0, f"the scoped probe failed. See {out / 'runner.log'}")
    report = read(out / "textbox-values.json")
    expected = {name: digest for name, digest in assets.items() if name != "surface-web.json"}
    require(report["actual_served_assets"] == expected, "served assets differ from the pin")
    write_json(out / "verified.json", {"scope": report["scope"], "expect": args.expect,
        "browser": report["browser"], "adapter": report["adapter"], "asset_manifest_sha256": args.manifest_sha256,
        "value_samples": len(report["inactive"]), "window_samples": observed["samples"],
        "visible_windows": 0, "survivors": 0,
        "report_sha256": sha(out / "textbox-values.json"),
        "window_receipt_sha256": sha(out / "headless-window-observation.json")})
    print(f"Verified {args.expect}: {out / 'verified.json'}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--web", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--manifest-sha256", required=True)
    parser.add_argument("--expect", choices=("present", "missing"), required=True)
    parser.add_argument("--out", type=Path, required=True)
    proof(parser.parse_args())
