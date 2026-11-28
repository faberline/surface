#!/usr/bin/env python3
"""Verify the full q82 stock Material Browser run and original wide-rail pixels."""

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
UPDATER_ARG = "--disable-updater-scheduler"
DISPLAY_VARS = {"DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"}
OVERRIDES = {
    "SURFACE_CHROME", "SURFACE_CHROME_ARGS", "CHROME_PATH",
    "SURFACE_BRIDGE_SESSION", "SURFACE_DEVTOOLS_SESSION",
}
MATERIAL_STEPS = [str(i) for i in range(1, 389) if i != 23]
BADGE_REPORT = "browser-material-wide-navigation-badges.json"
BADGE_PNGS = {
    "collapsed_light": "material-wide-navigation-badges-collapsed-light.png",
    "expanded_light": "material-wide-navigation-badges-expanded-light.png",
    "dark": "material-wide-navigation-badges-expanded-dark-rtl-200.png",
    "acknowledged": "material-wide-navigation-badges-acknowledged.png",
}
IDS = [
    "wide-navigation-badges-demo-home",
    "wide-navigation-badges-demo-inbox",
    "wide-navigation-badges-demo-updates",
    "wide-navigation-badges-demo-explore",
    "wide-navigation-badges-demo-settings",
]
NAMES = ["首頁", "收件匣，1,234 則未讀", "最新消息，有新內容", "探索路線，有新內容", "設定，無法使用"]


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
    require(source_root.is_relative_to(Path("/tmp").resolve()), "source checkout must stay under /tmp")
    require(source_manifest.is_relative_to(Path("/tmp").resolve()), "source manifest must stay under /tmp")
    require(re.fullmatch(r"[0-9a-f]{40}", config["source_commit"])
            and re.fullmatch(r"[0-9a-f]{40}", config["source_tree"])
            and re.fullmatch(r"[0-9a-f]{64}", config["source_sha256"])
            and type(config["source_files"]) is int and config["source_files"] > 0,
            "source manifest, commit, tree, or count pin is invalid")
    require(digest(source_manifest) == config["source_sha256"], "current q82 source manifest SHA differs")
    source_entries = verify_manifest(source_root, source_manifest, config["source_files"])
    require(set(source_entries) == source_files(source_root), "q82 source inventory differs from manifest")
    commit, tree = subprocess.check_output(
        ["git", "-C", str(source_root), "rev-parse", "HEAD", "HEAD^{tree}"], text=True
    ).splitlines()
    require(commit == config["source_commit"] and tree == config["source_tree"], "q82 commit or tree differs")
    required_source = {
        "e2e/src/material_wide_navigation_badges.rs",
        "examples/material-gallery/src/wide_navigation_badge_demo.rs",
    }
    require(required_source <= set(source_entries), "q82 wide rail source files are absent from the manifest")
    journey = (source_root / "e2e/src/material_wide_navigation_badges.rs").read_text()
    gallery = (source_root / "examples/material-gallery/src/wide_navigation_badge_demo.rs").read_text()
    entry = (source_root / "e2e/src/material.rs").read_text()
    require('"M387", "M388"' in journey and "wide_navigation_rail_item_with_badge" in gallery
            and "material_wide_navigation_badges::run" in entry,
            "q82 route or the registered browser journey is absent")
    status = subprocess.check_output(
        ["git", "-C", str(source_root), "--no-optional-locks", "status", "--porcelain"], text=True
    )
    require(status == "", "q82 source checkout is not clean")

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
    require(marker == [config["source_tree"], config["source_sha256"]],
            "build marker does not pin the reviewed q82 source")

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
        "config_sha256": args.config_sha256,
        "config_path": str(args.config.resolve()),
        "observer_sha256": GUARD_SHA,
        "verifier_sha256": digest(Path(__file__).resolve()),
    }


def verify_log(log):
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"],
            "expected exactly one stock Material suite")
    steps = re.findall(r"^  \[M(\d+)\]", log, re.M)
    require(steps == MATERIAL_STEPS, "expected exact ordered M1–M388 steps except M23")
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


def near_color(actual, expected, tolerance=8):
    return all(abs(a - b) <= tolerance for a, b in zip(actual, expected))


def pixel_count(path, canvas, dpr, rectangle, rgb, tolerance=8):
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
                if near_color((r, g, b), rgb, tolerance))
    return count, image.size


def verify_accessibility(value, acknowledged=False):
    require(value["landmark"] == "工作區導覽", "wrong wide rail landmark name")
    items = value["items"]
    require(len(items) == len(IDS), "wrong wide rail destination count")
    for index, item in enumerate(items):
        expected_name = "收件匣" if acknowledged and index == 1 else NAMES[index]
        require(item == {"id": IDS[index], "name": expected_name, "disabled": index == 4},
                f"wrong accessible name, ID, or disabled state for {IDS[index]}")


def verify_wide_rail_image(artifacts, profile, key, width, expanded, rtl, acknowledged=False):
    require(set(profile) == {"canvas", "rail", "expanded", "rtl", "owners", "inbox_label", "count_ink",
                             "error", "screenshot", "accessibility"},
            f"unexpected report fields for {key}")
    canvas = profile["canvas"]
    require(isinstance(canvas, list) and len(canvas) == 4
            and all(type(part) in (int, float) and math.isfinite(part) for part in canvas)
            and canvas[2:] == [width, 720.], f"wrong recorded canvas for {key}")
    dpr = 2.
    require(profile["expanded"] is expanded and profile["rtl"] is rtl,
            f"wrong wide rail mode or direction for {key}")
    rail = rect(profile["rail"], f"{key} rail")
    width_ok = 219.25 <= rail[2] <= 360.75 if expanded else abs(rail[2] - 96.) <= 0.75
    require(width_ok, f"wrong wide rail width for {key}: {rail[2]}")
    expected_x = 8. if not rtl else canvas[2] - 8. - rail[2]
    require(abs(rail[0] - expected_x) <= 0.75,
            f"rail is not at logical start in {key}")
    require(rail[0] >= canvas[0] and rail[1] >= canvas[1]
            and rail[0] + rail[2] <= canvas[0] + canvas[2]
            and rail[1] + rail[3] <= canvas[1] + canvas[3], f"rail escapes canvas for {key}")
    error = profile["error"]
    require(isinstance(error, list) and len(error) == 3
            and all(type(channel) is int and 0 <= channel <= 255 for channel in error),
            f"invalid Error color for {key}")
    require(len(profile["owners"]) == len(IDS), f"missing owner samples for {key}")
    reports = {row["id"]: row for row in profile["owners"]}
    require(set(reports) == set(IDS), f"owner IDs differ for {key}")
    image_path = artifacts / profile["screenshot"]
    require(image_path.name == BADGE_PNGS[key] and image_path.is_file(),
            f"missing original PNG for {key}")
    observed = {}
    for index, item_id in enumerate(IDS):
        row = reports[item_id]
        require(set(row) == {"id", "bounds", "error_pixels"}, f"unexpected owner row for {item_id}")
        owner = rect(row["bounds"], f"{item_id} owner in {key}")
        require(owner[0] >= rail[0] and owner[1] >= rail[1]
                and owner[0] + owner[2] <= rail[0] + rail[2]
                and owner[1] + owner[3] <= rail[1] + rail[3],
                f"{item_id} owner escapes rail in {key}")
        fill, device_size = pixel_count(image_path, canvas, dpr, owner, error)
        require(fill == row["error_pixels"], f"reported Error pixel count differs in {key}: {item_id}")
        hidden = index in (0, 4) or (index == 1 and acknowledged)
        if hidden:
            require(fill == 0, f"hidden {item_id} still paints an Error badge in {key}")
        else:
            floor = 8 if index == 2 else 30
            require(fill >= floor, f"visible badge pixels are missing in {key}: {item_id}")
        observed[item_id] = {"bounds": owner, "error_pixels": fill}

    count_ink = profile["count_ink"]
    inbox_owner = reports[IDS[1]]["bounds"]
    if acknowledged:
        require(count_ink is None, f"acknowledged capture still reports count text in {key}")
    else:
        require(set(count_ink) == {"bounds", "ink", "pixels"},
                f"count text evidence is missing in {key}")
        count_bounds = rect(count_ink["bounds"], f"Inbox count text in {key}")
        require(count_bounds[0] >= inbox_owner[0] and count_bounds[1] >= inbox_owner[1]
                and count_bounds[0] + count_bounds[2] <= inbox_owner[0] + inbox_owner[2]
                and count_bounds[1] + count_bounds[3] <= inbox_owner[1] + inbox_owner[3],
                f"count text escapes Inbox owner in {key}")
        count_color = count_ink["ink"]
        require(isinstance(count_color, list) and len(count_color) == 3
                and all(type(channel) is int and 0 <= channel <= 255 for channel in count_color),
                f"invalid count text ink in {key}")
        count_pixels, count_size = pixel_count(image_path, canvas, dpr, count_bounds, count_color)
        require(count_pixels == count_ink["pixels"] and count_pixels >= 4,
                f"999+ count text is not visible in original PNG: {key}")
        require(count_size == device_size, f"PNG dimensions changed during count readback for {key}")

    label = profile["inbox_label"]
    if expanded:
        require(set(label) == {"bounds", "ink", "pixels"}, f"expanded label evidence missing in {key}")
        label_bounds = rect(label["bounds"], f"Inbox label in {key}")
        if not acknowledged:
            overlaps = all(
                min(count_bounds[axis] + count_bounds[axis + 2],
                    label_bounds[axis] + label_bounds[axis + 2])
                > max(count_bounds[axis], label_bounds[axis])
                for axis in (0, 1)
            )
            require(not overlaps, f"count and expanded destination text overlap in {key}")
        ink = label["ink"]
        require(isinstance(ink, list) and len(ink) == 3
                and all(type(channel) is int and 0 <= channel <= 255 for channel in ink),
                f"invalid Inbox label ink in {key}")
        label_pixels, label_size = pixel_count(image_path, canvas, dpr, label_bounds, ink, 12)
        require(label_pixels == label["pixels"] and label_pixels >= 10,
                f"expanded Inbox label is clipped or unreadable in {key}")
        require(label_size == device_size, f"PNG dimensions changed during readback for {key}")
    else:
        require(label is None, f"collapsed capture unexpectedly reports expanded label: {key}")

    verify_accessibility(profile["accessibility"], acknowledged)
    width_px, height_px = device_size
    require(width_px >= math.floor((canvas[0] + canvas[2]) * dpr + 0.5)
            and height_px >= math.floor((canvas[1] + canvas[3]) * dpr + 0.5),
            f"original PNG is smaller than the reported canvas: {key}")
    return {"screenshot": image_path.name, "sha256": digest(image_path),
            "device_size": [width_px, height_px], "canvas": canvas,
            "device_pixel_ratio": dpr, "owners": observed,
            "expanded_label_pixels": label["pixels"] if expanded else None}


def verify_badges(artifacts):
    path = artifacts / BADGE_REPORT
    report = data(path)
    require(set(report) == {"browser", "steps", "collapsed_light", "expanded_light", "collapsed_scroll", "dark",
                             "acknowledged", "errors"}, "q82 wide rail report fields differ")
    require(re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", report["browser"]), "bad Chrome version")
    require(report["steps"] == ["M387", "M388"], "q82 wide rail steps differ")
    require(report["errors"] == [], "q82 report includes browser errors")
    short = report["collapsed_scroll"]
    require(set(short) == {"clip", "before", "revealed", "page", "focus", "scroll_offset"},
            "short collapsed scroll fields differ")
    view = rect(short["clip"], "short collapsed rail")
    before = rect(short["before"], "target before reveal")
    revealed = rect(short["revealed"], "target after reveal")
    require(short["focus"] == IDS[3] and type(short["scroll_offset"]) in (int, float)
            and short["scroll_offset"] > 0 and revealed[1] < before[1]
            and before[1] + before[3] > view[1] + view[3]
            and revealed[1] >= view[1] - .75
            and revealed[1] + revealed[3] <= view[1] + view[3] + .75,
            "arrow focus did not reveal the destination in the short rail")
    rect(short["page"], "short collapsed rail page")
    collapsed = verify_wide_rail_image(artifacts, report["collapsed_light"], "collapsed_light",
                                       840., False, False)
    expanded = verify_wide_rail_image(artifacts, report["expanded_light"], "expanded_light",
                                      840., True, False)
    dark = verify_wide_rail_image(artifacts, report["dark"], "dark", 420., True, True)
    acknowledged = verify_wide_rail_image(artifacts, report["acknowledged"], "acknowledged",
                                          420., True, True, acknowledged=True)
    require(report["dark"]["accessibility"] == report["acknowledged"]["accessibility"]
            or report["acknowledged"]["accessibility"]["items"][1]["name"] == "收件匣",
            "dark RTL or acknowledged accessibility state is wrong")
    require(dark["device_size"] == acknowledged["device_size"],
            "acknowledgement capture changed original PNG dimensions")
    require(report["collapsed_light"]["error"] == report["expanded_light"]["error"],
            "light rail captures use different Error colors")
    require(report["dark"]["error"] != report["expanded_light"]["error"],
            "dark capture does not show a distinct dark palette")
    inbox_owner = dark["owners"][IDS[1]]["bounds"]
    ack_owner = acknowledged["owners"][IDS[1]]["bounds"]
    require(inbox_owner == ack_owner, "acknowledgement changed stable Inbox bounds")
    require({image.name for image in artifacts.glob("material-wide-navigation-badges-*.png")}
            == set(BADGE_PNGS.values()), "unexpected or missing q82 original PNGs")
    return {"report_sha256": digest(path), "collapsed_light": collapsed, "collapsed_scroll": short,
            "expanded_light": expanded, "dark": dark,
            "acknowledged": acknowledged}


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
        print("PASS exact q82 source, current stock build, runner, and observer pins; no app started")
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
