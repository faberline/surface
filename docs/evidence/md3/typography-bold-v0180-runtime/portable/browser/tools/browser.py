#!/usr/bin/env python3
"""Run and verify the reviewed q80 Material Browser evidence in headless Chrome."""

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

from evidence_common import data, digest, json_file, path_entries, require, verify_entries

ROOT = Path(__file__).resolve().parent
GUARD = ROOT / "observe-focused-original.py"
GUARD_SHA = "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7"
LANE = Path("/tmp/surface-md3-v0180-font-weight.J3lyc9eB").resolve()
UPDATER_ARG = "--disable-updater-scheduler"
DISPLAY_VARS = {"DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"}
OVERRIDES = {
    "SURFACE_CHROME",
    "SURFACE_CHROME_ARGS",
    "CHROME_PATH",
    "SURFACE_BRIDGE_SESSION",
    "SURFACE_DEVTOOLS_SESSION",
}
MATERIAL_STEPS = [str(i) for i in range(1, 385) if i != 23]
TYPOGRAPHY_REPORT = "material-typography-font-weight.json"
TYPOGRAPHY_PNGS = {
    "light": "material-typography-font-weight-wide-light.png",
    "dark": "material-typography-font-weight-compact-dark.png",
}


def source_files(root):
    return {
        path.relative_to(root).as_posix()
        for path in root.rglob("*")
        if (path.is_file() or path.is_symlink())
        and path.relative_to(root).parts[0] != ".git"
    }


def verify_manifest(root, manifest, expected_count, *, prefix=None):
    entries = path_entries(manifest)
    names = list(entries)
    require(names == sorted(names), "manifest paths are not sorted")
    require(len(entries) == expected_count, "manifest file count differs from its reviewed pin")
    if prefix is not None:
        require(all(name.startswith(prefix) for name in names), "manifest contains a path outside its root")
    verify_entries(root, entries)
    return entries


def inputs(args):
    require(digest(GUARD) == GUARD_SHA, "q79 Chrome observer source changed")
    config = data(args.config)
    require(re.fullmatch(r"[0-9a-f]{64}", args.config_sha256)
            and digest(args.config) == args.config_sha256, "wrong reviewed config digest")
    required = {
        "source",
        "source_manifest",
        "source_commit",
        "source_tree",
        "source_sha256",
        "source_files",
        "build_root",
        "build_manifest",
        "build_sha256",
        "runner",
        "runner_sha256",
        "web_root",
    }
    require(set(config) == required, "wrong config fields")
    source_root = (LANE / "surface").resolve(strict=True)
    source_manifest = (LANE / "source-v3.sha256").resolve(strict=True)
    build_root = (LANE / "workspace-v3").resolve(strict=True)
    build_manifest = (build_root / "stock.sha256").resolve(strict=True)
    runner = (build_root / "stock/debug/surface-e2e").resolve(strict=True)
    web_root = (build_root / "stock/web/release").resolve(strict=True)
    require(Path(config["source"]).resolve(strict=True) == source_root
            and Path(config["source_manifest"]).resolve(strict=True) == source_manifest
            and Path(config["build_root"]).resolve(strict=True) == build_root
            and Path(config["build_manifest"]).resolve(strict=True) == build_manifest
            and Path(config["runner"]).resolve(strict=True) == runner
            and Path(config["web_root"]).resolve(strict=True) == web_root,
            "config paths differ from the assigned q80 lane")
    require(re.fullmatch(r"[0-9a-f]{40}", config["source_commit"])
            and re.fullmatch(r"[0-9a-f]{40}", config["source_tree"])
            and re.fullmatch(r"[0-9a-f]{64}", config["source_sha256"])
            and type(config["source_files"]) is int and config["source_files"] == 711,
            "invalid reviewed source pins")
    require(all(re.fullmatch(r"[0-9a-f]{64}", config[key]) for key in ("build_sha256", "runner_sha256")),
            "reviewed build and runner SHA-256 values are required")

    require(digest(source_manifest) == config["source_sha256"], "source manifest SHA differs")
    source_entries = verify_manifest(source_root, source_manifest, config["source_files"])
    require(set(source_entries) == source_files(source_root), "source file inventory differs")
    commit, tree = subprocess.check_output(
        ["git", "-C", str(source_root), "rev-parse", "HEAD", "HEAD^{tree}"], text=True
    ).splitlines()
    require(commit == config["source_commit"] and tree == config["source_tree"],
            "source commit or tree differs")

    require(digest(build_manifest) == config["build_sha256"], "build manifest SHA differs")
    build_entries = verify_manifest(build_root, build_manifest,
                                    sum(1 for path in (build_root / "stock").rglob("*") if path.is_file()),
                                    prefix="stock/")
    stock_files = {
        path.relative_to(build_root).as_posix()
        for path in (build_root / "stock").rglob("*")
        if path.is_file() or path.is_symlink()
    }
    require(set(build_entries) == stock_files, "stock output inventory differs")
    require(build_entries.get(runner.relative_to(build_root).as_posix()) == config["runner_sha256"]
            and digest(runner) == config["runner_sha256"] and os.access(runner, os.X_OK),
            "runner binary differs from the reviewed build pin")
    web_prefix = web_root.relative_to(build_root).as_posix() + "/material-gallery/"
    require(any(name.startswith(web_prefix) for name in build_entries), "built Material Gallery is missing")
    complete = (build_root / "BUILD-COMPLETE").read_text().splitlines()
    require(complete == [config["source_tree"], config["source_sha256"]],
            "build completion marker does not name the reviewed source")

    args.runner = runner
    args.web_root = web_root
    return {
        "source_commit": commit,
        "source_tree": tree,
        "source_sha256": config["source_sha256"],
        "source_files": len(source_entries),
        "build_sha256": config["build_sha256"],
        "build_files": len(build_entries),
        "runner_sha256": config["runner_sha256"],
        "verifier_sha256": digest(Path(__file__).resolve()),
        "shared_helper_sha256": digest(ROOT / "evidence_common.py"),
        "observer_sha256": GUARD_SHA,
        "config_sha256": args.config_sha256,
    }


def verify_log(log):
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"],
            "expected exactly one stock Material suite")
    steps = re.findall(r"^  \[M(\d+)\]", log, re.M)
    require(steps == MATERIAL_STEPS, "expected exact ordered M1–M384 steps except M23")
    rows = re.findall(r"^case material \.\.\. ok \((\d+) steps, (\d+) checks,", log, re.M)
    require(len(rows) == 1 and int(rows[0][0]) == len(MATERIAL_STEPS)
            and int(rows[0][1]) >= len(MATERIAL_STEPS), "Material result has invalid step or check totals")
    checks = int(rows[0][1])
    require(re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", log, re.M) == [str(checks)],
            "runner summary does not confirm the passing Material suite")
    return checks


def rect(value, label):
    require(isinstance(value, list) and len(value) == 4
            and all(type(part) in (int, float) and math.isfinite(part) for part in value),
            f"invalid {label} bounds")
    require(value[2] > 0 and value[3] > 0, f"empty {label} bounds")
    return value


def contains(outer, inner, label):
    x, y, width, height = outer
    a, b, w, h = inner
    require(x <= a + 0.75 and y <= b + 0.75
            and a + w <= x + width + 0.75 and b + h <= y + height + 0.75,
            f"{label} lies outside its visible parent")


def actual_ink(path, canvas, dpr, rect_value, rgb):
    with Image.open(path) as raw:
        require(raw.format == "PNG", f"not a PNG: {path.name}")
        image = raw.convert("RGB")
        image.load()
    left, top, width, height = rect_value
    round_pixel = lambda value: math.floor(value + 0.5)
    x1 = round_pixel((canvas[0] + left) * dpr)
    x2 = round_pixel((canvas[0] + left + width) * dpr)
    y1 = round_pixel((canvas[1] + top) * dpr)
    y2 = round_pixel((canvas[1] + top + height) * dpr)
    require(0 <= x1 < x2 <= image.width and 0 <= y1 < y2 <= image.height,
            f"text crop escapes original PNG: {path.name}")
    return sum(
        1
        for red, green, blue in image.crop((x1, y1, x2, y2)).getdata()
        if abs(red - rgb[0]) <= 8 and abs(green - rgb[1]) <= 8 and abs(blue - rgb[2]) <= 8
    ), image.size


def verify_typography(artifacts):
    path = artifacts / TYPOGRAPHY_REPORT
    report = data(path)
    require(set(report) == {"light", "dark"}, "typography report has unexpected profile keys")
    profiles = {}
    expected_profiles = {
        "light": {"viewport": [960, 800], "dark": False, "image": TYPOGRAPHY_PNGS["light"]},
        "dark": {"viewport": [320, 720], "dark": True, "image": TYPOGRAPHY_PNGS["dark"]},
    }
    for name, expected in expected_profiles.items():
        item = report[name]
        require(set(item) == {"canvas", "device_pixel_ratio", "card", "regular", "bold", "screenshot"},
                f"unexpected {name} report fields")
        canvas = item["canvas"]
        require(isinstance(canvas, list) and len(canvas) == 4
                and all(type(value) in (int, float) and math.isfinite(value) for value in canvas)
                and canvas[2:] == expected["viewport"], f"wrong {name} canvas profile")
        dpr = item["device_pixel_ratio"]
        require(type(dpr) in (int, float) and dpr == 2, f"wrong {name} device scale")
        card = rect(item["card"], f"{name} card")
        viewport = [0., 0., *expected["viewport"]]
        contains(viewport, card, f"{name} card")
        require(item["screenshot"] == expected["image"], f"wrong {name} screenshot name")

        regular = item["regular"]
        bold = item["bold"]
        keys = {"bounds", "weight", "font_size", "line_height", "foreground", "ink_pixels"}
        require(set(regular) == keys and set(bold) == keys, f"wrong {name} specimen fields")
        regular_rect = rect(regular["bounds"], f"{name} Regular text")
        bold_rect = rect(bold["bounds"], f"{name} Bold text")
        contains(card, regular_rect, f"{name} Regular text")
        contains(card, bold_rect, f"{name} Bold text")
        require(regular["weight"] == 400 and bold["weight"] == 700,
                f"wrong {name} Regular/Bold weights")
        require(regular["font_size"] == bold["font_size"] == 16
                and regular["line_height"] == bold["line_height"] == 24,
                f"wrong {name} font size or line height")
        regular_color, bold_color = regular["foreground"], bold["foreground"]
        require(regular_color == bold_color and isinstance(regular_color, list)
                and len(regular_color) == 3
                and all(type(channel) is int and 0 <= channel <= 255 for channel in regular_color),
                f"wrong {name} foreground")

        image_path = artifacts / expected["image"]
        regular_ink, size = actual_ink(image_path, canvas, dpr, regular_rect, regular_color)
        bold_ink, bold_size = actual_ink(image_path, canvas, dpr, bold_rect, regular_color)
        require(size == bold_size, f"{name} PNG changed while reading")
        require(regular_ink == regular["ink_pixels"] and bold_ink == bold["ink_pixels"],
                f"recorded {name} pixel counts differ from the original PNG")
        require(regular_ink >= 12 and bold_ink > regular_ink,
                f"{name} image does not show visibly heavier Bold text")
        with Image.open(image_path) as png:
            dimensions = png.size
        expected_dimensions = (
            math.floor((canvas[0] + canvas[2]) * dpr + 0.5),
            math.floor((canvas[1] + canvas[3]) * dpr + 0.5),
        )
        require(dimensions == expected_dimensions, f"wrong {name} original screenshot dimensions")
        profiles[name] = {
            "screenshot_sha256": digest(image_path),
            "screenshot_size": list(dimensions),
            "canvas": canvas,
            "device_pixel_ratio": dpr,
            "regular_ink_pixels": regular_ink,
            "bold_ink_pixels": bold_ink,
        }
    require({path.name for path in artifacts.glob("material-typography-font-weight-*.png")}
            == set(TYPOGRAPHY_PNGS.values()), "unexpected typography screenshots")
    return {"report_sha256": digest(path), "profiles": profiles}


def verify_run(out, window):
    require(window["stock_exit"] == 0 and window["problems"] == []
            and window["visible_owned_windows"] == []
            and window["remaining_observed_processes"] == [],
            "Material run, window check, or process cleanup failed")
    launches = window["guard_events"]
    require(len(launches) == 1 and launches[0]["role"] == "chrome-primary"
            and launches[0]["allowed"] is True, "expected one allowed guarded Chrome process")
    chrome = window["launch_observations"]["chrome-primary"]
    require(chrome["samples"] > 0 and len(chrome["pids"]) == 1,
            "guarded Chrome process was not observed")
    flags = launches[0]["argv"]
    require(flags.count("--headless=new") == 1 and flags.count(UPDATER_ARG) == 1
            and "--" not in flags and launches[0]["profile"] is not None
            and str(launches[0]["profile"]).startswith(str((out / "tmp").resolve()) + os.sep),
            "Chrome was not launched headlessly with a private profile")

    checks = verify_log((out / "runner.log").read_text())
    artifacts = out / "artifacts"
    browser_report_path = artifacts / "browser-material.json"
    browser_report = data(browser_report_path)
    require(browser_report.get("errors") == [] and "failure" not in browser_report
            and re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", browser_report.get("browser", "")),
            "stock Material report has browser errors or wrong Chrome identity")
    for candidate in artifacts.glob("browser-material*.json"):
        errors = data(candidate).get("errors")
        require(errors is None or errors == [], f"browser errors in {candidate.name}")
    typography = verify_typography(artifacts)
    return {
        "material_steps": len(MATERIAL_STEPS),
        "checks": checks,
        "browser": browser_report["browser"],
        "browser_material_sha256": digest(browser_report_path),
        "typography": typography,
        "chrome_samples": chrome["samples"],
        "guard_and_window_checks": "passed",
    }


def observe(args, out, pins):
    observer_code = run_path(str(GUARD))
    chrome = (args.chrome or observer_code["CHROME"]).resolve(strict=True)
    require(sys.platform == "darwin" and os.access(chrome, os.X_OK),
            "requires the reviewed macOS headless Chrome executable")
    out.mkdir(mode=0o700)
    for name in ("guards", "guard-ledger", "tmp", "artifacts"):
        (out / name).mkdir(mode=0o700)
    guard = out / "guards" / "chrome"
    prefix = [sys.executable, "-B", str(GUARD), "__guard", "chrome", str(chrome)]
    guard.write_text("#!/bin/sh\nexec " + shlex.join(prefix) + ' "$@"\n')
    guard.chmod(0o700)
    command = [
        str(args.runner.resolve()),
        "--chrome", str(guard),
        "--chrome-arg", UPDATER_ARG,
        "--web-root", str(args.web_root.resolve()),
        "--artifacts", str(out / "artifacts"),
        "material",
    ]
    excluded = DISPLAY_VARS | OVERRIDES | {observer_code["JET_MARKER"]}
    environment = {
        name: value
        for name, value in os.environ.items()
        if name not in excluded
        and not (name.startswith("SURFACE_") and name.endswith("_FOCUSED"))
    }
    environment.update({"TMPDIR": str(out / "tmp"), observer_code["PRIVATE_ROOT"]: str(out)})
    json_file(out / "invocation.json", {"argv": command, "inputs": pins})
    windows = observer_code["WindowOwners"]()
    with (out / "runner.log").open("xb") as log:
        leader = subprocess.Popen(
            command,
            cwd=out,
            env=environment,
            start_new_session=True,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=subprocess.STDOUT,
        )
        observer = observer_code["Observer"](out, windows, "/usr/bin/false", chrome, leader.pid)
        deadline = monotonic() + args.timeout
        problems = []
        owned = {}
        try:
            while leader.poll() is None:
                owned = observer.sample()
                if observer.visible or any(not event["allowed"] for event in observer.guards.values()):
                    problems.append("visible owned window or rejected pre-exec guard")
                    break
                if monotonic() > deadline:
                    problems.append("stock Material suite timed out")
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
                problems.append("private process survived cleanup grace")
        finally:
            if leader.poll() is None or owned:
                observer_code["terminate_owned"](leader, owned)
            exit_status = leader.wait(timeout=8)
            window = observer.report(exit_status, observer.sample(), problems)
            json_file(out / "headless-window-observation.json", window)
    return window


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--config-sha256", required=True)
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--timeout", type=int, default=1800)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check-inputs", action="store_true")
    mode.add_argument("--run", action="store_true")
    mode.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    pins = inputs(args)
    if args.check_inputs:
        print("PASS exact q80 source, build, runner, and Chrome observer pins; no app started")
        return
    require(args.out is not None, "provide a new output directory under /tmp")
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()) and out != Path("/tmp").resolve(),
            "output must be under /tmp")
    if args.verify:
        require(data(out / "invocation.json")["inputs"] == pins
                and data(out / "inputs-after.json") == pins, "run used different inputs")
        result = verify_run(out, data(out / "headless-window-observation.json"))
        receipt = out / "verified-stock-material.json"
        require(receipt.is_file() and data(receipt) == json.loads(json.dumps(result)),
                "stored receipt differs from independent verification")
    else:
        window = observe(args, out, pins)
        require(inputs(args) == pins, "source or build inputs changed during the run")
        json_file(out / "inputs-after.json", pins)
        result = verify_run(out, window)
        json_file(out / "verified-stock-material.json", result)
    print(f"PASS {result['material_steps']} Material steps and {result['checks']} checks; receipt: {out}")


if __name__ == "__main__":
    main()
