#!/usr/bin/env python3
"""Pin, run, and independently read back q83 stock headless Browser evidence."""
import argparse
import json
import math
import os
from pathlib import Path
import re
from runpy import run_path
import shlex
import signal
import subprocess
import sys
from time import monotonic, sleep

from PIL import Image
from evidence_common import data, digest, json_file, path_entries, require, verify_entries

ROOT = Path(__file__).resolve().parent
GUARD = ROOT / "observe-focused-original.py"
GUARD_SHA = "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7"
COMMON_SHA = "a51f3386c63b0cc8408ed1a1d41cae6788886a83b3baec28a2e735092b3716a5"
UPDATER_ARG = "--disable-updater-scheduler"
DISPLAY_VARS = {"DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"}
OVERRIDES = {"SURFACE_CHROME", "SURFACE_CHROME_ARGS", "CHROME_PATH",
             "SURFACE_BRIDGE_SESSION", "SURFACE_DEVTOOLS_SESSION"}
MATERIAL_STEPS = [str(i) for i in range(1, 391) if i != 23]
REPORT = "material-adaptive-navigation-badges.json"
SCREENSHOTS = {
    "light_medium": "material-adaptive-navigation-badges-light-medium-600.png",
    "light_rail": "material-adaptive-navigation-badges-light-rail-840.png",
    "dark_rtl_200": "material-adaptive-navigation-badges-dark-rtl-200-count.png",
    "acknowledged": "material-adaptive-navigation-badges-dark-rtl-200-cleared.png",
}
IDS = ["adaptive-navigation-demo-home", "adaptive-navigation-demo-explore",
       "adaptive-navigation-demo-saved", "adaptive-navigation-demo-updates",
       "adaptive-navigation-demo-settings"]
NAMES = ["首頁", "探索路線", "收藏，有新項目", "最新消息，1,234 則未讀", "設定，有新功能"]
UPDATES = IDS[3]


def source_inventory(root, manifest, expected_sha, expected_count):
    require(digest(manifest) == expected_sha, "source manifest hash differs")
    entries = path_entries(manifest)
    names = list(entries)
    require(names == sorted(names) and len(entries) == expected_count, "source manifest inventory differs")
    verify_entries(root, entries)
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
              if (p.is_file() or p.is_symlink()) and ".git" not in p.relative_to(root).parts}
    require(actual == set(entries), "source checkout has unpinned or missing files")
    return entries


def inputs(args):
    require(digest(GUARD) == GUARD_SHA and digest(ROOT / "evidence_common.py") == COMMON_SHA,
            "copied q82 observer or common helper changed")
    require(re.fullmatch(r"[0-9a-f]{64}", args.config_sha256 or "")
            and digest(args.config) == args.config_sha256, "config hash differs")
    config = data(args.config)
    required = {"source", "source_manifest", "source_commit", "source_tree", "source_sha256",
                "source_files", "build_root", "build_manifest", "build_sha256", "build_files",
                "runner", "runner_sha256", "web_root"}
    require(set(config) == required, "unexpected q83 config fields")
    source = Path(config["source"]).resolve(strict=True)
    manifest = Path(config["source_manifest"]).resolve(strict=True)
    require(source.is_relative_to(Path("/tmp").resolve()) and manifest.is_relative_to(Path("/tmp").resolve()),
            "source and manifest must remain under /tmp")
    require(re.fullmatch(r"[0-9a-f]{40}", config["source_commit"])
            and re.fullmatch(r"[0-9a-f]{40}", config["source_tree"])
            and re.fullmatch(r"[0-9a-f]{64}", config["source_sha256"])
            and type(config["source_files"]) is int and config["source_files"] > 0,
            "invalid q83 source pins")
    entries = source_inventory(source, manifest, config["source_sha256"], config["source_files"])
    commit, tree = subprocess.check_output(
        ["git", "-C", str(source), "rev-parse", "HEAD", "HEAD^{tree}"], text=True
    ).splitlines()
    require((commit, tree) == (config["source_commit"], config["source_tree"]),
            "q83 source commit or tree differs")
    status = subprocess.check_output(
        ["git", "-C", str(source), "--no-optional-locks", "status", "--porcelain"], text=True
    )
    require(not status, "q83 source checkout is not clean")
    journey_name = "e2e/src/material_adaptive_navigation_badges.rs"
    gallery_name = "examples/material-gallery/src/adaptive_navigation_demo.rs"
    require({journey_name, gallery_name, "e2e/src/material.rs", "e2e/src/main.rs"} <= set(entries),
            "q83 E2E or Gallery files are absent")
    journey = (source / journey_name).read_text()
    gallery = (source / gallery_name).read_text()
    module = (source / "e2e/src/material.rs").read_text()
    main = (source / "e2e/src/main.rs").read_text()
    require('["M389", "M390"]' in journey and '"M389", "M390"' in module
            and "material_adaptive_navigation_badges::run" in module
            and "adaptive_navigation_scaffold_with_badges" in gallery,
            "q83 journey or Gallery API is not registered")
    require('"material" => material::STEPS' in main, "stock Material suite does not load its step registry")

    build = Path(config["build_root"]).resolve(strict=True)
    stock = Path(config["build_manifest"]).resolve(strict=True)
    runner = Path(config["runner"]).resolve(strict=True)
    web = Path(config["web_root"]).resolve(strict=True)
    require(build.is_relative_to(Path("/tmp").resolve()), "build must stay under /tmp")
    require(stock == (build / "stock.sha256").resolve(strict=True), "wrong stock build manifest")
    require(runner == (build / "stock/debug/surface-e2e").resolve(strict=True),
            "runner is not the exact stock E2E executable")
    require(web == (build / "stock/web/release").resolve(strict=True),
            "wrong Web root")
    require(re.fullmatch(r"[0-9a-f]{64}", config["build_sha256"])
            and re.fullmatch(r"[0-9a-f]{64}", config["runner_sha256"])
            and type(config["build_files"]) is int and config["build_files"] > 0,
            "invalid stock build pins")
    require(digest(stock) == config["build_sha256"], "stock manifest SHA differs")
    build_entries = path_entries(stock)
    require(list(build_entries) == sorted(build_entries) and len(build_entries) == config["build_files"]
            and all(name.startswith("stock/") for name in build_entries),
            "stock manifest paths or count differ")
    verify_entries(build, build_entries)
    actual_stock = {p.relative_to(build).as_posix() for p in (build / "stock").rglob("*")
                    if p.is_file() or p.is_symlink()}
    require(actual_stock == set(build_entries), "stock build inventory differs")
    runner_key = runner.relative_to(build).as_posix()
    require(build_entries.get(runner_key) == config["runner_sha256"]
            and digest(runner) == config["runner_sha256"] and os.access(runner, os.X_OK),
            "stock E2E runner bytes differ")
    require(any(name.startswith("stock/web/release/material-gallery/") for name in build_entries),
            "stock Material Gallery Web files are missing")
    marker = (build / "BUILD-COMPLETE").read_text().splitlines()
    require(marker == [tree, config["source_sha256"]], "build marker does not match exact q83 source")
    return {
        "source_commit": commit, "source_tree": tree, "source_sha256": config["source_sha256"],
        "source_files": len(entries), "config_sha256": args.config_sha256,
        "build_sha256": config["build_sha256"], "build_files": len(build_entries),
        "runner_sha256": config["runner_sha256"], "guard_sha256": GUARD_SHA,
        "common_helper_sha256": COMMON_SHA, "verifier_sha256": digest(Path(__file__).resolve()),
        "config_path": str(args.config.resolve()), "source_path": str(source),
        "build_path": str(build), "web_root": str(web),
    }


def verify_log(log):
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"],
            "expected one stock Material suite")
    steps = re.findall(r"^  \[M(\d+)\]", log, re.M)
    require(steps == MATERIAL_STEPS, "expected ordered M1-M390, except M23")
    rows = re.findall(r"^case material \.\.\. ok \((\d+) steps, (\d+) checks,", log, re.M)
    require(len(rows) == 1 and int(rows[0][0]) == len(MATERIAL_STEPS)
            and int(rows[0][1]) >= len(MATERIAL_STEPS), "Material summary differs")
    checks = int(rows[0][1])
    require(re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", log, re.M) == [str(checks)],
            "E2E suite result disagrees with Material summary")
    return checks


def rect(value, label):
    require(isinstance(value, list) and len(value) == 4
            and all(type(n) in (int, float) and math.isfinite(n) for n in value),
            f"invalid {label} bounds")
    require(value[2] > 0 and value[3] > 0, f"empty {label} bounds")
    return value


def inside(parent, child, tolerance=0.75):
    return (child[0] >= parent[0] - tolerance and child[1] >= parent[1] - tolerance
            and child[0] + child[2] <= parent[0] + parent[2] + tolerance
            and child[1] + child[3] <= parent[1] + parent[3] + tolerance)


def image_crop(path, canvas, bounds, viewport_width):
    with Image.open(path) as source:
        require(source.format == "PNG", f"not a PNG: {path.name}")
        im = source.convert("RGB")
        im.load()
    scale = im.width / viewport_width
    require(scale > 0, f"invalid screenshot scale: {path.name}")
    x, y, w, h = bounds
    rnd = lambda n: math.floor(n + 0.5)
    box = (rnd((canvas[0] + x) * scale), rnd((canvas[1] + y) * scale),
           rnd((canvas[0] + x + w) * scale), rnd((canvas[1] + y + h) * scale))
    require(0 <= box[0] < box[2] <= im.width and 0 <= box[1] < box[3] <= im.height,
            f"crop escapes screenshot: {path.name}")
    return im.crop(box), im.size, scale


def near_count(counter, rgb, tolerance=8):
    return sum(count for color, count in counter.items()
               if all(abs(color[i] - rgb[i]) <= tolerance for i in range(3)))


def readback_error_pixels(crops, reported, label, expected_rgb):
    values = [near_count(counter, expected_rgb) for counter, _ in crops]
    require(values == reported, f"original PNG does not reproduce Error pixels for {label}")
    return {"error_rgb": expected_rgb, "error_pixels_by_owner": values}


def rgba(value, name):
    require(isinstance(value, list) and len(value) == 4
            and all(type(n) is int and 0 <= n <= 255 for n in value), f"invalid {name} token")
    require(value[3] == 255, f"{name} token is not opaque")
    return value[:3]


def readback_on_error_pixels(crop, expected, label, expected_rgb):
    from collections import Counter
    counter = Counter(crop.getdata())
    observed = near_count(counter, expected_rgb)
    require(observed == expected, f"On Error pixels differ in original screenshot: {label}")
    return {"on_error_rgb": expected_rgb, "on_error_pixels": observed}


def access_rows(value, acknowledged=False):
    require(value["landmark"] == "主要導覽", "wrong navigation landmark")
    items = value["items"]
    require(len(items) == len(IDS), "wrong number of accessible destinations")
    names = list(NAMES)
    if acknowledged:
        names[3] = "最新消息"
    current = IDS[3] if acknowledged else IDS[2]
    observed = {}
    for index, item in enumerate(items):
        require(item["id"] == IDS[index] and item["name"] == names[index]
                and item["disabled"] is (index == 1), f"wrong AX name/ID/disabled state at {IDS[index]}")
        require(item["current"] in (None, "page"), f"invalid aria-current value: {item['current']}")
        require((item["current"] == "page") is (item["id"] == current),
                f"wrong current page for {item['id']}")
        observed[item["id"]] = item["current"]
    return observed


def verify_profile(artifacts, profile, name, width, dark, rtl, scale, acknowledged=False):
    required = {"screenshot", "canvas", "navigation", "rtl", "text_scale", "dark",
                "colors", "owners", "count", "accessibility"}
    require(set(profile) == required, f"unexpected q83 report fields: {name}")
    require(profile["screenshot"] == SCREENSHOTS[name], f"wrong screenshot name: {name}")
    canvas = profile["canvas"]
    require(isinstance(canvas, list) and len(canvas) == 4
            and all(type(n) in (int, float) and math.isfinite(n) for n in canvas)
            and canvas[2:] == [float(width), 720.0], f"wrong viewport in {name}")
    require(profile["rtl"] is rtl and profile["dark"] is dark
            and profile["text_scale"] == scale, f"wrong theme/direction/scale: {name}")
    colors = profile["colors"]
    require(set(colors) == {"error", "on_error"}, f"wrong token palette keys in {name}")
    for token, rgb in colors.items():
        require(isinstance(rgb, list) and len(rgb) == 3
                and all(type(n) is int and 0 <= n <= 255 for n in rgb),
                f"invalid {token} RGB color in {name}")
    nav = rect(profile["navigation"], f"{name} navigation")
    require(inside([0.0, 0.0, canvas[2], canvas[3]], nav), f"navigation escapes viewport: {name}")
    if width == 600:
        require(nav[2] > nav[3], "600 dp layout did not use a horizontal navigation bar")
    else:
        require(nav[3] > nav[2], "840 dp layout did not use a vertical navigation rail")
    owners = profile["owners"]
    require(len(owners) == len(IDS), f"missing owner bounds in {name}")
    values = []
    mapped = {}
    for index, row in enumerate(owners):
        require(set(row) == {"id", "bounds", "error_pixels"}, "unexpected badge owner fields")
        require(row["id"] == IDS[index] and type(row["error_pixels"]) is int
                and row["error_pixels"] >= 0, f"wrong owner report for {IDS[index]}")
        bounds = rect(row["bounds"], f"{IDS[index]} in {name}")
        require(inside(nav, bounds), f"{IDS[index]} escapes navigation bounds")
        expected_active = index in (2, 4) or (index == 3 and not acknowledged)
        if expected_active:
            require(row["error_pixels"] >= 8, f"visible badge missing for {IDS[index]} in {name}")
        else:
            require(row["error_pixels"] == 0, f"hidden badge still paints for {IDS[index]} in {name}")
        values.append(row["error_pixels"])
        mapped[row["id"]] = bounds
    image_path = artifacts / profile["screenshot"]
    crop_list = [image_crop(image_path, canvas, mapped[item_id], width) for item_id in IDS]
    counters = []
    device_size = None
    scale = None
    for cropped, size, actual_scale in crop_list:
        from collections import Counter
        counters.append((Counter(cropped.getdata()), size))
        if device_size is None:
            device_size = size
        require(size == device_size, f"capture dimensions changed within {name}")
        if scale is None:
            scale = actual_scale
        require(abs(actual_scale - scale) < 1e-9, f"owner image scale differs within {name}")
    pixel_result = readback_error_pixels(counters, values, name, colors["error"])
    require(device_size[0] >= math.floor((canvas[0] + canvas[2]) * scale + .5)
            and device_size[1] >= math.floor((canvas[1] + canvas[3]) * scale + .5),
            f"PNG is smaller than its reported viewport: {name}")
    count = profile["count"]
    if acknowledged:
        require(count is None, "cleared count still appears in report")
    else:
        require(set(count) == {"bounds", "on_error_pixels"} and type(count["on_error_pixels"]) is int
                and count["on_error_pixels"] >= 4, f"count text evidence missing in {name}")
        count_bounds = rect(count["bounds"], f"999+ count in {name}")
        require(inside(mapped[UPDATES], count_bounds), f"999+ count escaped Updates owner in {name}")
        count_crop, _, _ = image_crop(image_path, canvas, count_bounds, width)
        require(count_crop.width > 0 and count_crop.height > 0, f"empty count crop in {name}")
        on_error_result = readback_on_error_pixels(count_crop, count["on_error_pixels"], name,
                                                   colors["on_error"])
    if acknowledged:
        on_error_result = None
    ax = access_rows(profile["accessibility"], acknowledged)
    return {"screenshot": image_path.name, "sha256": digest(image_path),
            "device_size": list(device_size), "canvas": canvas, "navigation": nav,
            "badge_pixels": pixel_result, "count_pixels": on_error_result,
            "accessibility_current": ax}


def verify_adaptive_badges(artifacts, base_report=None):
    path = artifacts / REPORT
    report = data(path)
    required = {"browser", "steps", "breakpoints", "launcher_accessibility",
                "light_medium", "light_rail", "dark_rtl_200", "acknowledged",
                "initial_accessibility", "acknowledged_accessibility",
                "selected", "activations", "errors"}
    require(set(report) == required, "q83 report schema differs")
    require(report["steps"] == ["M389", "M390"] and report["breakpoints"] == [599, 600, 839, 840],
            "q83 test IDs or responsive breakpoints differ")
    require(report["errors"] == [] and report["selected"] == UPDATES and report["activations"] == 2,
            "q83 runtime errors, selected page, or activation count differ")
    require(report["launcher_accessibility"] == "自適應導覽徽章", "wrong Gallery launcher accessible name")
    require(re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", report["browser"]), "q83 browser identity differs")
    dark_palette = report["dark_rtl_200"]["colors"]
    require(report["light_medium"]["colors"] == report["light_rail"]["colors"],
            "changing the light layout changed the badge palette")
    require(report["acknowledged"]["colors"] == dark_palette,
            "clearing the count changed the dark badge palette")
    require(report["light_medium"]["colors"] != dark_palette,
            "light and dark badge palettes did not change")
    if base_report is not None:
        require(report["browser"] == base_report["browser"], "q83 report browser differs from base")
        state = base_report["state"]
        require(state["dark"] is True, "base Material capture did not retain the M6 dark theme")
        adaptive = state["adaptive_navigation"]
        palette = adaptive["badge_palette"]
        require(set(palette) == {"error", "on_error"}, "base report badge palette fields differ")
        require({token: rgba(value, token) for token, value in palette.items()} == dark_palette,
                "dark palette differs from the base Material capture")
    initial = access_rows(report["initial_accessibility"])
    final = access_rows(report["acknowledged_accessibility"], acknowledged=True)
    require(report["acknowledged_accessibility"] == report["acknowledged"]["accessibility"],
            "final AX state differs from cleared capture")
    require(report["initial_accessibility"] == report["light_medium"]["accessibility"]
            and report["light_rail"]["accessibility"] == report["initial_accessibility"]
            and report["dark_rtl_200"]["accessibility"] == report["initial_accessibility"],
            "breakpoint/theme change lost stable AX names or selection")
    captures = {
        "light_medium": verify_profile(artifacts, report["light_medium"], "light_medium", 600,
                                        False, False, 1.0),
        "light_rail": verify_profile(artifacts, report["light_rail"], "light_rail", 840,
                                      False, False, 1.0),
        "dark_rtl_200": verify_profile(artifacts, report["dark_rtl_200"], "dark_rtl_200", 840,
                                        True, True, 2.0),
        "acknowledged": verify_profile(artifacts, report["acknowledged"], "acknowledged", 840,
                                       True, True, 2.0, acknowledged=True),
    }
    require(report["dark_rtl_200"]["navigation"] == report["acknowledged"]["navigation"],
            "clearing the count changed rail geometry")
    names = {p.name for p in artifacts.glob("material-adaptive-navigation-badges-*.png")}
    require(names == set(SCREENSHOTS.values()), "missing or unexpected q83 original PNGs")
    return {"report_sha256": digest(path), "capture_readbacks": captures,
            "initial_current": initial, "acknowledged_current": final,
            "dark_palette": dark_palette}


def verify_run(out, window, pins, original_out=None):
    require(window["stock_exit"] == 0 and window["problems"] == []
            and window["visible_owned_windows"] == []
            and window["remaining_observed_processes"] == [],
            "visible-window, exit or process cleanup evidence failed")
    launches = window["guard_events"]
    require(len(launches) == 1 and launches[0]["role"] == "chrome-primary"
            and launches[0]["allowed"] is True, "expected one guarded Chrome launch")
    event = launches[0]
    launched_from = original_out if original_out is not None else out
    require(event["argv"].count("--headless=new") == 1
            and event["argv"].count(UPDATER_ARG) == 1
            and event["profile"] and Path(event["profile"]).resolve().is_relative_to(
                (launched_from / "tmp").resolve()),
            "Chrome was not headless with a private profile")
    chrome = window["launch_observations"]["chrome-primary"]
    require(chrome["samples"] > 0 and len(chrome["pids"]) == 1, "Chrome process was not observed")
    log = (out / "runner.log").read_text()
    checks = verify_log(log)
    artifacts = out / "artifacts"
    base = data(artifacts / "browser-material.json")
    require(base.get("errors") == [] and re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", base.get("browser", "")),
            "base Material report has browser errors")
    for candidate in artifacts.glob("browser-material*.json"):
        payload = data(candidate)
        require(payload.get("errors") in (None, []), f"browser errors in {candidate.name}")
    adaptive = verify_adaptive_badges(artifacts, base)
    return {"material_steps": len(MATERIAL_STEPS), "checks": checks, "browser": base["browser"],
            "adaptive_badges": adaptive, "chrome_samples": chrome["samples"],
            "headless_and_window_checks": "passed", "pins": pins}


def observe(args, out, pins):
    guard_code = run_path(str(GUARD))
    chrome = (args.chrome or guard_code["CHROME"]).resolve(strict=True)
    require(sys.platform == "darwin" and os.access(chrome, os.X_OK), "requires the reviewed macOS Chrome executable")
    out.mkdir(mode=0o700)
    for name in ("guards", "guard-ledger", "tmp", "artifacts"):
        (out / name).mkdir(mode=0o700)
    guard = out / "guards/chrome"
    prefix = [sys.executable, "-B", str(GUARD), "__guard", "chrome", str(chrome)]
    guard.write_text("#!/bin/sh\nexec " + shlex.join(prefix) + ' "$@"\n')
    guard.chmod(0o700)
    runner = Path(pins["build_path"]) / "stock/debug/surface-e2e"
    command = [str(runner), "--chrome", str(guard), "--chrome-arg", UPDATER_ARG,
               "--web-root", pins["web_root"], "--artifacts", str(out / "artifacts"), "material"]
    environment = {name: value for name, value in os.environ.items()
                   if name not in DISPLAY_VARS | OVERRIDES | {guard_code["JET_MARKER"]}
                   and not (name.startswith("SURFACE_") and name.endswith("_FOCUSED"))}
    environment.update({"TMPDIR": str(out / "tmp"), guard_code["PRIVATE_ROOT"]: str(out)})
    json_file(out / "invocation.json", {"argv": command, "inputs": pins})
    windows = guard_code["WindowOwners"]()
    with (out / "runner.log").open("xb") as log:
        leader = subprocess.Popen(command, cwd=out, env=environment, start_new_session=True,
                                  stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
        observer = guard_code["Observer"](out, windows, Path("/usr/bin/false"), chrome, leader.pid)
        deadline = monotonic() + args.timeout
        problems, owned = [], {}
        try:
            while leader.poll() is None:
                owned = observer.sample()
                if observer.visible or any(not event["allowed"] for event in observer.guards.values()):
                    problems.append("visible window or rejected Chrome guard")
                    break
                if monotonic() >= deadline:
                    problems.append("stock Browser timed out")
                    break
                sleep(0.05)
            if problems:
                guard_code["terminate_owned"](leader, owned)
            owned = observer.sample()
            cleanup = monotonic() + 8
            while owned and monotonic() < cleanup:
                sleep(0.1)
                owned = observer.sample()
            if owned:
                problems.append("owned process survived cleanup window")
        finally:
            if leader.poll() is None or owned:
                guard_code["terminate_owned"](leader, owned)
            try:
                exit_status = leader.wait(timeout=8)
            except subprocess.TimeoutExpired:
                leader.kill()
                exit_status = leader.wait(timeout=5)
                problems.append("runner required forceful cleanup")
            receipt = observer.report(exit_status, observer.sample(), problems)
            json_file(out / "headless-window-observation.json", receipt)
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--config-sha256", required=True)
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--timeout", type=int, default=1800)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--check-inputs", action="store_true")
    modes.add_argument("--run", action="store_true")
    modes.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    pins = inputs(args)
    if args.check_inputs:
        print("PASS q83 source, stock build and guard pins; no application started")
        return
    require(args.out is not None, "provide a fresh output directory")
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()) and out != Path("/tmp").resolve(),
            "run output must stay under /tmp")
    if args.verify:
        require(data(out / "invocation.json")["inputs"] == pins
                and data(out / "inputs-after.json") == pins, "run inputs differ from config")
        result = verify_run(out, data(out / "headless-window-observation.json"), pins)
        require(data(out / "verified-q83-stock.json") == json.loads(json.dumps(result)),
                "saved q83 receipt differs from independent readback")
    else:
        window = observe(args, out, pins)
        require(inputs(args) == pins, "source or stock build changed during Browser run")
        json_file(out / "inputs-after.json", pins)
        result = verify_run(out, window, pins)
        json_file(out / "verified-q83-stock.json", result)
    print(json.dumps({"status": "PASS", "steps": result["material_steps"], "checks": result["checks"],
                      "run": str(out)}, indent=2))


if __name__ == "__main__":
    main()

