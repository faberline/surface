#!/usr/bin/env python3
"""Verify the full q81 stock Material Browser run and original badge pixels."""

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
Q81_SOURCE = Path("/tmp/surface-md3-v0181-flexible-nav-badges.lqVxZ01z/surface")
Q81_COMMIT = "b9dc4ad7a5c6cdddd71196a669354f17984af7aa"
Q81_TREE = "ea61008a90b12a3f753398b9e5c1ff58f5813b3d"
Q81_SOURCE_SHA = "8e4f44ab8774aebbdf629bea859101887d42c8904cd6abb32502ff7ffc0b230c"
Q81_SOURCE_FILES = 714
UPDATER_ARG = "--disable-updater-scheduler"
DISPLAY_VARS = {"DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"}
OVERRIDES = {
    "SURFACE_CHROME", "SURFACE_CHROME_ARGS", "CHROME_PATH",
    "SURFACE_BRIDGE_SESSION", "SURFACE_DEVTOOLS_SESSION",
}
MATERIAL_STEPS = [str(i) for i in range(1, 387) if i != 23]
BADGE_REPORT = "browser-material-flexible-navigation-badges.json"
BADGE_PNGS = {
    "light": "material-flexible-navigation-badges-wide-light-600.png",
    "dark": "material-flexible-navigation-badges-compact-dark-rtl-200.png",
    "acknowledged": "material-flexible-navigation-badges-acknowledged.png",
}
IDS = [
    "navigation-badges-demo-home",
    "navigation-badges-demo-inbox",
    "navigation-badges-demo-updates",
    "navigation-badges-demo-explore",
    "navigation-badges-demo-settings",
]
NAMES = ["首頁", "收件匣，1200 則未讀", "最新消息，有新內容", "探索路線，有新內容", "設定"]


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
    require(len(entries) == expected_count, "manifest file count differs from its pin")
    if prefix is not None:
        require(all(name.startswith(prefix) for name in names), "manifest contains a path outside its root")
    verify_entries(root, entries)
    return entries


def inputs(args):
    require(digest(GUARD) == GUARD_SHA, "q79 headless observer source changed")
    require(re.fullmatch(r"[0-9a-f]{64}", args.config_sha256 or "")
            and digest(args.config) == args.config_sha256, "wrong reviewed config digest")
    config = data(args.config)
    required = {
        "source", "source_manifest", "source_commit", "source_tree", "source_sha256", "source_files",
        "build_root", "build_manifest", "build_sha256", "build_files", "runner", "runner_sha256", "web_root",
    }
    require(set(config) == required, "wrong config fields")

    source_root = Path(config["source"]).resolve(strict=True)
    source_manifest = Path(config["source_manifest"]).resolve(strict=True)
    require(source_root == Q81_SOURCE.resolve(strict=True), "source path differs from q81 checkout")
    require(source_manifest.is_relative_to(Path("/tmp").resolve()), "source manifest must stay under /tmp")
    require(config["source_commit"] == Q81_COMMIT and config["source_tree"] == Q81_TREE
            and config["source_sha256"] == Q81_SOURCE_SHA and config["source_files"] == Q81_SOURCE_FILES,
            "source pin differs from reviewed q81 candidate")
    require(digest(source_manifest) == Q81_SOURCE_SHA, "current q81 source manifest SHA differs")
    source_entries = verify_manifest(source_root, source_manifest, Q81_SOURCE_FILES)
    require(set(source_entries) == source_files(source_root), "q81 source inventory differs from manifest")
    commit, tree = subprocess.check_output(
        ["git", "-C", str(source_root), "rev-parse", "HEAD", "HEAD^{tree}"], text=True
    ).splitlines()
    require(commit == Q81_COMMIT and tree == Q81_TREE, "q81 commit or tree differs")
    status = subprocess.check_output(
        ["git", "-C", str(source_root), "--no-optional-locks", "status", "--porcelain"], text=True
    )
    require(status == "", "q81 source checkout is not clean")

    for key in ("build_root", "build_manifest", "build_sha256", "build_files", "runner", "runner_sha256", "web_root"):
        require(config[key] not in (None, ""), f"reviewed build pin is not set: {key}")
    build_root = Path(config["build_root"]).resolve(strict=True)
    build_manifest = Path(config["build_manifest"]).resolve(strict=True)
    runner = Path(config["runner"]).resolve(strict=True)
    web_root = Path(config["web_root"]).resolve(strict=True)
    require(build_root.is_relative_to(Path("/tmp").resolve()), "stock build must stay under /tmp")
    require(build_manifest == (build_root / "stock.sha256").resolve(strict=True),
            "build manifest is not the workspace stock manifest")
    require(runner == (build_root / "stock/debug/surface-e2e").resolve(strict=True),
            "runner is not the stock E2E binary")
    require(web_root == (build_root / "stock/web/release").resolve(strict=True),
            "web root is not the stock release package")
    require(re.fullmatch(r"[0-9a-f]{64}", config["build_sha256"])
            and re.fullmatch(r"[0-9a-f]{64}", config["runner_sha256"])
            and type(config["build_files"]) is int and config["build_files"] > 0,
            "build manifest, file count, or runner pin is invalid")
    require(digest(build_manifest) == config["build_sha256"], "build manifest SHA differs")
    build_entries = verify_manifest(build_root, build_manifest, config["build_files"], prefix="stock/")
    stock_files = {
        path.relative_to(build_root).as_posix()
        for path in (build_root / "stock").rglob("*")
        if path.is_file() or path.is_symlink()
    }
    require(set(build_entries) == stock_files, "stock build inventory differs from manifest")
    runner_key = runner.relative_to(build_root).as_posix()
    require(build_entries.get(runner_key) == config["runner_sha256"]
            and digest(runner) == config["runner_sha256"] and os.access(runner, os.X_OK),
            "stock runner hash differs")
    web_prefix = web_root.relative_to(build_root).as_posix() + "/material-gallery/"
    require(any(name.startswith(web_prefix) for name in build_entries), "Material Gallery web build is missing")
    marker = (build_root / "BUILD-COMPLETE").read_text().splitlines()
    require(marker == [Q81_TREE, Q81_SOURCE_SHA], "build marker does not pin current q81 source")

    args.runner = runner
    args.web_root = web_root
    return {
        "source_commit": commit,
        "source_tree": tree,
        "source_sha256": Q81_SOURCE_SHA,
        "source_files": len(source_entries),
        "build_sha256": config["build_sha256"],
        "build_files": len(build_entries),
        "runner_sha256": config["runner_sha256"],
        "config_sha256": args.config_sha256,
        "config_path": str(args.config.resolve()),
        "observer_sha256": GUARD_SHA,
        "verifier_sha256": digest(Path(__file__).resolve()),
    }


def verify_log(log):
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"],
            "expected exactly one stock Material suite")
    steps = re.findall(r"^  \[M(\d+)\]", log, re.M)
    require(steps == MATERIAL_STEPS, "expected exact ordered M1–M386 steps except M23")
    rows = re.findall(r"^case material \.\.\. ok \((\d+) steps, (\d+) checks,", log, re.M)
    require(len(rows) == 1 and int(rows[0][0]) == len(MATERIAL_STEPS)
            and int(rows[0][1]) >= len(MATERIAL_STEPS), "Material summary has the wrong steps or check count")
    checks = int(rows[0][1])
    require(re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", log, re.M) == [str(checks)],
            "E2E result does not confirm one passing Material suite")
    return checks


def rect(value, label):
    require(isinstance(value, list) and len(value) == 4
            and all(type(part) in (int, float) and math.isfinite(part) for part in value),
            f"invalid {label} rectangle")
    require(value[2] > 0 and value[3] > 0, f"empty {label} rectangle")
    return value


def near_color(actual, expected):
    return all(abs(a - b) <= 8 for a, b in zip(actual, expected))


def pixel_count(path, canvas, dpr, rectangle, rgb):
    with Image.open(path) as raw:
        require(raw.format == "PNG", f"not an original PNG: {path.name}")
        image = raw.convert("RGB")
        image.load()
    x, y, width, height = rectangle
    round_pixel = lambda value: math.floor(value + 0.5)
    x1 = round_pixel((canvas[0] + x) * dpr)
    x2 = round_pixel((canvas[0] + x + width) * dpr)
    y1 = round_pixel((canvas[1] + y) * dpr)
    y2 = round_pixel((canvas[1] + y + height) * dpr)
    require(0 <= x1 < x2 <= image.width and 0 <= y1 < y2 <= image.height,
            f"owner crop escapes original PNG: {path.name}")
    count = sum(1 for r, g, b in image.crop((x1, y1, x2, y2)).getdata()
                if near_color((r, g, b), rgb))
    return count, image.size


def verify_accessibility(value, acknowledged=False):
    landmark = value["landmark"]
    require(landmark["value"] == "導覽列徽章範例", "wrong navigation landmark name")
    items = value["items"]
    require(len(items) == 5, "wrong navigation item count")
    for index, item in enumerate(items):
        expected_name = "收件匣" if acknowledged and index == 1 else NAMES[index]
        require(item["id"] == IDS[index] and item["name"] == expected_name,
                f"wrong accessible name or stable ID for {IDS[index]}")
        require(item["disabled"] is (index == 4), f"wrong disabled state for {IDS[index]}")


def verify_badge_image(artifacts, profile, expected_name, viewport, horizontal, rtl):
    require(set(profile) == {"canvas", "device_pixel_ratio", "viewport", "layout", "colors",
                             "paint", "accessibility", "screenshot"}, "unexpected badge image report fields")
    canvas = profile["canvas"]
    require(isinstance(canvas, list) and len(canvas) == 4
            and all(type(item) in (int, float) and math.isfinite(item) for item in canvas),
            "invalid badge canvas")
    require(profile["viewport"] == viewport and canvas[2:] == viewport,
            f"wrong viewport for {expected_name}")
    dpr = profile["device_pixel_ratio"]
    require(type(dpr) in (int, float) and dpr == 2, "badge screenshot must use the recorded 2x device scale")
    layout = profile["layout"]
    require(layout["horizontal"] is horizontal and layout["rtl"] is rtl,
            f"wrong layout flags for {expected_name}")
    bar = rect(layout["bar"], f"{expected_name} bar")
    require(abs(bar[2] - viewport[0]) <= 0.75, "navigation bar does not fill its measured width")
    cells = {cell["id"]: rect(cell["bounds"], f"{cell['id']} cell") for cell in layout["cells"]}
    require(set(cells) == set(IDS), "layout cell IDs differ")
    colors = profile["colors"]
    error = colors["error"]
    on_error = colors["on_error"]
    require(all(type(channel) is int and 0 <= channel <= 255 for channel in error + on_error),
            "invalid Error palette colors")
    require(len(profile["paint"]) == len(IDS), "missing recorded owner paint rows")

    image_path = artifacts / expected_name
    require(image_path.is_file(), f"missing original screenshot {expected_name}")
    reports = {item["id"]: item for item in profile["paint"]}
    require(set(reports) == set(IDS), "paint rows differ from stable destination IDs")
    observed = {}
    for index, item_id in enumerate(IDS):
        row = reports[item_id]
        owner = rect(row["owner"], f"{item_id} owner")
        require(owner == cells[item_id], f"recorded owner bounds differ from layout for {item_id}")
        fill_pixels, size = pixel_count(image_path, canvas, dpr, owner, error)
        ink_pixels, ink_size = pixel_count(image_path, canvas, dpr, owner, on_error)
        require(size == ink_size, "screenshot dimensions changed between palette scans")
        require(fill_pixels == row["error_pixels"] and ink_pixels == row["on_error_pixels"],
                f"recorded color counts differ from original PNG for {item_id}")
        if index in (0, 4):
            require(fill_pixels == 0, f"hidden {item_id} paints Error color")
        elif index in (1, 3):
            require(fill_pixels >= 30 and ink_pixels >= 4, f"content badge pixels are missing for {item_id}")
        else:
            require(fill_pixels >= 8 and ink_pixels == 0, "Dot badge fill or text pixels differ")
        observed[item_id] = {"owner": owner, "error_pixels": fill_pixels, "on_error_pixels": ink_pixels}
    verify_accessibility(profile["accessibility"])
    width, height = size
    require(width >= math.floor((canvas[0] + canvas[2]) * dpr + 0.5)
            and height >= math.floor((canvas[1] + canvas[3]) * dpr + 0.5),
            f"{expected_name} PNG is smaller than its reported canvas")
    return {"screenshot": expected_name, "sha256": digest(image_path), "device_size": [width, height],
            "canvas": canvas, "device_pixel_ratio": dpr, "owner_pixels": observed}


def verify_badges(artifacts):
    path = artifacts / BADGE_REPORT
    report = data(path)
    require(set(report) == {"browser", "steps", "light", "dark", "acknowledged", "errors"},
            "q81 badge report fields differ")
    require(re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", report["browser"]), "bad Chrome version")
    require(report["steps"] == ["M385", "M386"], "q81 badge steps differ")
    require(report["errors"] == [], "q81 badge report includes browser errors")
    light_report = report["light"]
    require(set(light_report) == {"layouts", "focus_order", "current_after_resize", "activations",
                                  "disabled_pointer", "accessibility", "image"},
            "q81 light interaction report fields differ")
    require(light_report["focus_order"] == [
        "navigation-badges-demo-theme", "navigation-badges-demo-rtl",
        "navigation-badges-demo-large-text", IDS[0], IDS[1], IDS[2], IDS[3],
    ], "q81 tab order differs")
    require(light_report["current_after_resize"] == IDS[0] and light_report["activations"] == 1
            and light_report["disabled_pointer"] == IDS[4], "q81 keyboard or disabled-pointer report differs")
    layout_widths = []
    for layout in light_report["layouts"]:
        bar = rect(layout["bar"], "light navigation bar")
        layout_widths.append((bar[2], layout["horizontal"], layout["rtl"]))
    require(layout_widths == [
        (599., False, False), (600., True, False), (599., False, False),
        (600., True, False), (599., False, False),
    ], "q81 light layout transitions differ")
    verify_accessibility(light_report["accessibility"])
    light = verify_badge_image(artifacts, report["light"]["image"], BADGE_PNGS["light"],
                               [600., 720.], True, False)
    dark = verify_badge_image(artifacts, report["dark"]["image"], BADGE_PNGS["dark"],
                              [320., 720.], False, True)
    require(set(report["dark"]) == {"geometry", "dark", "rtl", "text_scale", "current",
                                     "accessibility", "image"}, "q81 dark report fields differ")
    require(report["dark"]["dark"] is True and report["dark"]["rtl"] is True
            and report["dark"]["text_scale"] == 2.0, "dark compact profile is not RTL at 200 percent")
    require(report["dark"]["current"] == IDS[0]
            and report["dark"]["geometry"] == report["dark"]["image"]["layout"],
            "dark selection or captured geometry differs")
    verify_accessibility(report["dark"]["accessibility"])

    acknowledged = report["acknowledged"]
    require(set(acknowledged) == {"id", "accessible_name", "activations", "screenshot"}
            and acknowledged["id"] == IDS[1] and acknowledged["accessible_name"] == "收件匣"
            and acknowledged["activations"] == 2
            and acknowledged["screenshot"] == BADGE_PNGS["acknowledged"],
            "acknowledged Inbox result is incomplete")
    dark_png = artifacts / BADGE_PNGS["dark"]
    ack_png = artifacts / BADGE_PNGS["acknowledged"]
    with Image.open(dark_png) as dark_image, Image.open(ack_png) as ack_image:
        require(dark_image.format == ack_image.format == "PNG" and dark_image.size == ack_image.size,
                "acknowledged capture changed the dark profile geometry")
    inbox_owner = dark["owner_pixels"][IDS[1]]["owner"]
    error_color = report["dark"]["image"]["colors"]["error"]
    dark_canvas = report["dark"]["image"]["canvas"]
    dark_dpr = report["dark"]["image"]["device_pixel_ratio"]
    cleared, ack_size = pixel_count(ack_png, dark_canvas, dark_dpr, inbox_owner, error_color)
    require(cleared == 0, "acknowledged Inbox retains Error-colored pixels in its stable dark owner bounds")
    verify_accessibility(report["dark"]["image"]["accessibility"], acknowledged=False)
    require({path.name for path in artifacts.glob("material-flexible-navigation-badges-*.png")}
            == set(BADGE_PNGS.values()), "unexpected or missing q81 original PNGs")
    return {"report_sha256": digest(path), "light": light, "dark": dark,
            "acknowledged": {"screenshot": BADGE_PNGS["acknowledged"], "sha256": digest(ack_png),
                             "stable_inbox_owner": inbox_owner, "stable_owner_error_pixels": cleared,
                             "device_size": list(ack_size), "accessible_name": acknowledged["accessible_name"]}}


def verify_run(out, window):
    require(window["stock_exit"] == 0 and window["problems"] == []
            and window["visible_owned_windows"] == [] and window["remaining_observed_processes"] == [],
            "Material run, visible-window check, or process cleanup failed")
    launches = window["guard_events"]
    require(len(launches) == 1 and launches[0]["role"] == "chrome-primary"
            and launches[0]["allowed"] is True, "expected one allowed guarded Chrome process")
    chrome = window["launch_observations"]["chrome-primary"]
    require(chrome["samples"] > 0 and len(chrome["pids"]) == 1, "guarded Chrome process was not observed")
    flags = launches[0]["argv"]
    require(flags.count("--headless=new") == 1 and flags.count(UPDATER_ARG) == 1
            and launches[0]["profile"] is not None
            and str(launches[0]["profile"]).startswith(str((out / "tmp").resolve()) + os.sep),
            "Chrome was not launched headlessly with a private profile")
    checks = verify_log((out / "runner.log").read_text())
    artifacts = out / "artifacts"
    base = data(artifacts / "browser-material.json")
    require(base.get("errors") == [] and re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", base.get("browser", "")),
            "base Material report has page errors or wrong Chrome identity")
    for candidate in artifacts.glob("browser-material*.json"):
        errors = data(candidate).get("errors")
        require(errors is None or errors == [], f"page errors in {candidate.name}")
    badge = verify_badges(artifacts)
    return {"material_steps": len(MATERIAL_STEPS), "checks": checks, "browser": base["browser"],
            "browser_material_sha256": digest(artifacts / "browser-material.json"),
            "badge_evidence": badge, "chrome_samples": chrome["samples"],
            "guard_and_window_checks": "passed"}


def observe(args, out, pins):
    observer_code = run_path(str(GUARD))
    chrome = (args.chrome or observer_code["CHROME"]).resolve(strict=True)
    require(sys.platform == "darwin" and os.access(chrome, os.X_OK), "requires the reviewed macOS Chrome executable")
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
        problems, owned = [], {}
        try:
            while leader.poll() is None:
                owned = observer.sample()
                if observer.visible or any(not event["allowed"] for event in observer.guards.values()):
                    problems.append("visible owned window or rejected pre-exec guard")
                    break
                if monotonic() > deadline:
                    problems.append("stock Material run timed out")
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
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--check-inputs", action="store_true")
    modes.add_argument("--run", action="store_true")
    modes.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    pins = inputs(args)
    if args.check_inputs:
        print("PASS exact q81 source, current stock build, runner, and observer pins; no app started")
        return
    require(args.out is not None, "provide a fresh /tmp output directory")
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()) and out != Path("/tmp").resolve(),
            "output must be under /tmp")
    if args.verify:
        require(data(out / "invocation.json")["inputs"] == pins
                and data(out / "inputs-after.json") == pins, "run inputs differ from current pins")
        result = verify_run(out, data(out / "headless-window-observation.json"))
        require(data(out / "verified-stock-material.json") == json.loads(json.dumps(result)),
                "saved receipt differs from independent verification")
    else:
        window = observe(args, out, pins)
        require(inputs(args) == pins, "source or build changed during the run")
        json_file(out / "inputs-after.json", pins)
        result = verify_run(out, window)
        json_file(out / "verified-stock-material.json", result)
    print(f"PASS {result['material_steps']} Material steps and {result['checks']} checks; receipt: {out}")


if __name__ == "__main__":
    main()
