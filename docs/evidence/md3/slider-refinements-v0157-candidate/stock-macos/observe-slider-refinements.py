#!/usr/bin/env python3
"""Fail closed and observe the five explicit Surface headless E2E suites."""

import argparse
import ctypes as C
from hashlib import file_digest
import json
from math import isfinite
import os
from pathlib import Path
import re
import shlex
import signal
import subprocess
import sys
import tomllib
from time import monotonic, sleep

from verify_forms_pixels import verify as verify_forms
from verify_range_pixels import verify as verify_range
from verify_refinement_pixels import verify as verify_refinements

ROOT = Path(__file__).resolve().parent
APP_STAGE = ROOT
E2E = ROOT / "bundle/debug/surface-e2e"
CHROME = Path("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
RELEASE = "0.1.57"
LAST_MATERIAL = 315
SOURCE_SHA = "5936ee7ecfdf584a2ca3acf7fa3297db09e86132b8a5dd60bd169391a90d44c1"
SOURCE_TREE = "72cf21a76d284e3a299aedff91b953eae232db07"
SOURCE_FILES = 652
CHIPS_REPORT = "material-chips.json"
CHIP_MODES = ("wide-light", "wide-focus", "wide-pressed", "compact-dark-rtl-200")
ICON_REPORT = "material-icon-buttons.json"
ICON_IMAGES = {
    "material-icon-buttons-wide-light.png": (1920, 1688),
    "material-icon-buttons-selected-focus.png": (1920, 1688),
    "material-icon-buttons-pressed.png": (1920, 1688),
    "material-icon-buttons-compact-dark-rtl-200.png": (640, 1608),
}
SLIDER_REPORT = "material-standard-slider.json"
SLIDER_IMAGES = {
    "material-standard-slider-wide-light.png": (1920, 2168),
    "material-standard-slider-keyboard.png": (1920, 2168),
    "material-standard-slider-live-drag.png": (1920, 2168),
    "material-standard-slider-compact-dark-rtl-200.png": (640, 1848),
}
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


def read_release_pin(stock):
    pin = read_json(stock / "release-pin.json")
    require(pin["version"] == RELEASE and pin["last_material_step"] == LAST_MATERIAL,
            "incorrect stock release or Material level")
    require(pin["source_sha256"] == SOURCE_SHA and pin["source_files"] == SOURCE_FILES
            and pin["source_tree"] == SOURCE_TREE, "wrong exact Slider Refinements source pin")
    require(all(type(pin[key]) is int and pin[key] > 0 for key in ("wide_canvas_height", "rtl_canvas_height")),
            "invalid CSS viewport heights")
    return pin


def check_source_version(source):
    version = tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == RELEASE, f"expected exact v{RELEASE} input, got v{version}")


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
    pin = read_release_pin(stock)
    expected = pin["source_sha256"]
    require(sha(source) == expected, "wrong final Slider Refinements source manifest")
    entries = checksums(source)
    require(len(entries) == pin["source_files"], "wrong source file count")
    origins = read_json(stock / "source-origins.json")
    markers = read_json(stock / "build-markers.json")
    for kind in ("native", "web"):
        origin = Path(origins[kind]).resolve(strict=True)
        check_source_version(origin)
        verify_entries(origin, entries)
        marker = markers[kind]
        require(sha(Path(marker["path"])) == marker["sha256"], f"{kind} build marker changed")
    return {"runner_sha256": runner_sha, "app_bundle_seal_sha256": sha(stock / "sealed-inputs.sha256"),
            "bundle_files_checked": len(binaries), "stock_files": binaries,
            "source_manifest_sha256": expected, "source_files_checked_per_origin": len(entries),
            "release": pin}


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


def verify_runner_log(log, last_material=LAST_MATERIAL):
    begins = re.findall(r"^case (\S+) \.\.\.$", log, re.M)
    rows = re.findall(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks,", log, re.M)
    require(tuple(begins) == tuple(row[0] for row in rows) == SUITES, "runner did not run exactly the ordered five")
    counts = {name: {"steps": int(steps), "checks": int(checks)} for name, steps, checks in rows}
    require(all(row["checks"] >= row["steps"] > 0 for row in counts.values()), "invalid suite check or step count")
    checks = sum(case["checks"] for case in counts.values())
    summaries = re.findall(r"^e2e result: (\d+) passed; (\d+) failed; (\d+) checks;", log, re.M)
    require(summaries == [("5", "0", str(checks))]
            and not re.search(r"^case \S+ \.\.\. (?:FAILED|skipped)", log, re.M), "bad runner summary")
    material = [int(step) for step in re.findall(r"^  \[M(\d+)\]", log, re.M)]
    expected = [step for step in range(1, last_material + 1) if step != 23]
    require(material == expected and counts["material"]["steps"] == len(expected),
            "Material steps changed or out of order")
    ime_steps = [int(step) for step in re.findall(r"^  \[I(\d+)\]", log, re.M)]
    require(ime_steps == list(range(1, 21)) and counts["ime"] == {"steps": 20, "checks": 77},
            "IME receipt coverage changed")
    require(counts["bridge-web"]["steps"] == 24 and counts["devtools-headless"]["steps"] == 3,
            "Bridge or headless DevTools coverage changed")
    return counts, checks


def verify_run(out, observation, pin):
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
    counts, checks = verify_runner_log(log)

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
    adapter = metadata["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False, "macOS stock proof needs actual Apple Metal")
    bridge = read_json(artifacts / "bridge-web-result.json")
    require(bridge["webgpu_ready"] is True and bridge["owned_browser_exited"] is True,
            "Bridge did not prove WebGPU or browser cleanup")
    native = read_json(artifacts / "devtools-headless-result.json")
    require(native["width"] == 720 and native["height"] == 640 and native["scale"] == 1
            and native["action_rendered"] is True and native["screenshot_transports_match"] is True,
            "native evidence is missing offscreen actions or transport equality")
    chips_path = artifacts / CHIPS_REPORT
    chips = read_json(chips_path)
    verify_chip_report(chips, metadata)
    chips_images = {
        f"material-chips-{mode}.png":
            (640, 2 * (pin["rtl_canvas_height"] + 44)) if mode == "compact-dark-rtl-200"
            else (1920, 2 * (pin["wide_canvas_height"] + 44))
        for mode in CHIP_MODES
    }
    require({p.name for p in artifacts.glob("material-chips-*.png")} == set(chips_images),
            "Chips output must contain exactly the four original success images")
    icons_path = artifacts / ICON_REPORT
    verify_icon_report(read_json(icons_path), metadata)
    require({p.name for p in artifacts.glob("material-icon-buttons-*.png")} == set(ICON_IMAGES),
            "Icon Button output must contain exactly the four original success images")
    sliders_path = artifacts / SLIDER_REPORT
    verify_slider_report(read_json(sliders_path), metadata)
    require({p.name for p in artifacts.glob("material-standard-slider-*.png")} == set(SLIDER_IMAGES),
            "Standard Slider output must contain exactly the four original success images")
    forms = verify_forms(artifacts, metadata)
    range_slider = verify_range(artifacts, metadata)
    slider_refinements = verify_refinements(artifacts, metadata)
    pngs = {}
    for name, dimensions in (("bridge-web.png", None), ("devtools-headless-screenshot.png", (720, 640)),
                             *chips_images.items(), *ICON_IMAGES.items(), *SLIDER_IMAGES.items()):
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
                                        if name not in chips_images and name not in ICON_IMAGES
                                        and name not in SLIDER_IMAGES},
            "chips_pngs": {name: pngs[name] for name in chips_images},
            "chips_report_sha256": sha(chips_path),
            "icons_pngs": {name: pngs[name] for name in ICON_IMAGES},
            "icons_report_sha256": sha(icons_path),
            "sliders_pngs": {name: pngs[name] for name in SLIDER_IMAGES},
            "sliders_report_sha256": sha(sliders_path), "slider_forms": forms, "range_slider": range_slider,
            "slider_refinements": slider_refinements, "guard_and_window_checks": "passed"}


def verify_chip_report(report, metadata):
    require(report["browser"] == metadata["browser"] and report["adapter"] == metadata["adapter"],
            "Chips Chrome or WebGPU adapter disagrees")
    require(report["page_errors"] == [] and not {"error", "failure", "debug", "state_at_failure"} & report.keys(),
            "Chips browser report contains a failure")
    require(all(isinstance(report[key], dict) and report[key] for key in ("wide", "keyboard", "pointer", "compact")),
            "Chips browser report lacked a complete journey")

    wide, keyboard, pointer, compact = (
        report[key] for key in ("wide", "keyboard", "pointer", "compact")
    )
    require(wide["canvas"] == [960, 800] and compact["canvas"] == [320, 1020],
            "Chips browser viewport differs")
    require(compact["rtl"] and compact["text_scale"] == 2 and
            compact["back_focus"] == "chip-demo-show", "compact Chips interaction is missing")
    require(len(wide["variants"]) == 7 and len({entry["id"] for entry in wide["variants"]}) == 7,
            "Chips style coverage differs")
    require(all(abs(variant["face"][3] - 32) <= 0.75 and min(variant["hit"][2:]) >= 48
                for variant in wide["variants"]), "Chips face or target size differs")
    require(keyboard["focus_pixels"] > 0 and
            pointer["before"] != pointer["held"], "Chips focus or press paint is absent")
    require(compact["following_row"][1] > compact["first_row"][1]
            and len(compact["label_line_ink"]) >= 2
            and all(value > 0 for value in compact["label_line_ink"]), "Chips wrapping proof is missing")


def verify_icon_report(report, meta):
    require(report["browser"] == meta["browser"] and report["adapter"] == meta["adapter"],
            "Icon Button Chrome or WebGPU adapter disagrees")
    require(report["page_errors"] == [] and not {"failure", "state_at_failure"} & report.keys(),
            "Icon Button browser report contains a failure")
    wide, keyboard, pointer, compact = (report[key] for key in ("wide", "keyboard", "pointer", "compact"))
    require([part["capture"] for part in (wide, keyboard, pointer, compact)] == list(ICON_IMAGES),
            "wrong Icon Button screenshot paths")
    require(wide["viewport"] == [960, 800] and compact["viewport"] == [320, 760]
            and compact["rtl"] is True and compact["text_scale"] == 2, "Icon Button profiles differ")
    footprints = [(28, 32, 20), (52, 40, 24), (48, 56, 24), (56, 56, 24), (64, 96, 32), (184, 136, 40)]
    suffixes = ("command", "toggle", "disabled", "medium", "large", "extra-large")
    ids = [f"icon-button-demo-{suffix}" for suffix in suffixes]
    for profile in (wide, compact):
        require([entry["id"] for entry in profile["geometry"]] == ids, "missing Icon Button specimens")
        for entry, (width, height, icon) in zip(profile["geometry"], footprints, strict=True):
            expected = {"hit": [max(width, 48), max(height, 48)], "face": [width, height],
                        "icon": [icon, icon], "focus": [width + 10, height + 10]}
            for kind, target in expected.items():
                box = entry[kind]
                require(len(box) == 4 and all(abs(a - b) <= 0.75 for a, b in zip(box[2:], target)),
                        f"wrong Icon Button {kind} dimensions")
    require([entry["id"] for entry in wide["ax"]] == ids
            and [entry["disabled"] for entry in wide["ax"]] == [False, False, True, False, False, False]
            and wide["ax"][1]["pressed"] == "false", "actual Chrome role evidence differs")
    require(keyboard["checked_ax"]["pressed"] == "true"
            and keyboard["unchecked_ax"]["pressed"] == "false"
            and keyboard["checked_ax"]["name"] == keyboard["unchecked_ax"]["name"]
            and keyboard["requests"] == [1, 2, 3] and keyboard["repeated_spaces"] > 0
            and keyboard["focus_pixels"] > 0 and len(keyboard["on_ink"]) > 0
            and len(wide["off_ink"]) > 0 and keyboard["on_ink"] != wide["off_ink"],
            "Icon Button toggle, independent focus, or changed artwork is missing")
    rest, early, held, released = pointer["corner_pixels"]
    require(rest < early < held and released == rest
            and pointer["reduced_corner_pixels"] > early
            and pointer["requests_after_cancel"] == 0 and pointer["requests_after_hit"] == 1,
            "Icon Button press, cancellation, or Reduce Motion measure is missing")
    tip_x, tip_y, tip_width, tip_height = compact["tooltip"]
    command = compact["geometry"][0]
    require(wide["command_fill"] == wide["palette"]["primary"][:3]
            and compact["command_icon_pixels"] > 0 and compact["description"]
            and tip_width > 0 and tip_height > 0 and tip_x >= -0.5 and tip_y >= -0.5
            and tip_x + tip_width <= 320.5
            and tip_y + tip_height < min(command["hit"][1], command["focus"][1]),
            "Icon Button palette or compact accessibility evidence is missing")


def slider_rect(box, name):
    require(isinstance(box, list) and len(box) == 4
            and all(type(n) in (int, float) and isfinite(n) for n in box)
            and box[2] > 0 and box[3] > 0, f"bad Slider bounds for {name}")
    return box


def slider_near(actual, expected, tolerance=0.75):
    return type(actual) in (int, float) and isfinite(actual) and abs(actual - expected) <= tolerance


def slider_contains(parent, child):
    px, py, pw, ph = parent
    x, y, w, h = child
    return px - 0.75 <= x and py - 0.75 <= y and x + w <= px + pw + 0.75 and y + h <= py + ph + 0.75


def slider_geometry(entry, suffix, viewport, track, handle, fraction, engaged, rtl=False):
    require(entry["id"] == f"slider-demo-{suffix}", "wrong Slider specimen")
    hit, rail, active, inactive, grip = [slider_rect(entry[kind], kind)
                                        for kind in ("hit", "rail", "active", "inactive", "handle")]
    require(slider_contains([0, 0, *viewport], hit) and all(
        slider_contains(hit, part) for part in (rail, active, inactive, grip)
    ), "Slider target or artwork goes outside its viewport")
    require(hit[2] >= 48 and slider_near(hit[3], max(48, handle))
            and slider_near(rail[0] - hit[0], 12) and slider_near(hit[2] - rail[2], 24)
            and slider_near(grip[2], 2 if engaged else 4) and slider_near(grip[3], handle)
            and slider_near(active[3], track) and slider_near(inactive[3], track),
            "Slider track, handle or target dimensions differ")
    left, right = (inactive, active) if rtl else (active, inactive)
    center = grip[0] + grip[2] / 2
    ratio = (center - rail[0]) / rail[2]
    require(slider_near(ratio, 1 - fraction if rtl else fraction, 0.005)
            and slider_near(grip[0] - (left[0] + left[2]), 6)
            and slider_near(right[0] - (grip[0] + grip[2]), 6),
            "Slider value position or logical direction differs")
    return entry


def slider_color(color, channels):
    return isinstance(color, list) and len(color) == channels and all(type(c) is int and 0 <= c <= 255 for c in color)


def verify_slider_report(report, metadata, require_palette=True):
    require(report["browser"] == metadata["browser"] and report["adapter"] == metadata["adapter"],
            "Slider Chrome or WebGPU adapter disagrees")
    require(report["page_errors"] == [] and not {"error", "failure", "debug", "state_at_failure"} & report.keys(),
            "Slider browser report contains a failure")
    wide, keyboard, pointer, compact = (report[k] for k in ("wide", "keyboard", "pointer", "compact"))
    require([part["capture"] for part in (wide, keyboard, pointer, compact)] == list(SLIDER_IMAGES),
            "wrong Slider screenshot paths")
    require(wide["viewport"] == [960, 1040] and compact["viewport"] == [320, 880]
            and compact["rtl"] is True and compact["text_scale"] == 2,
            "Slider CSS viewports, RTL or text scaling differ")

    require(len(wide["geometry"]) == 5, "missing wide Slider size tokens")
    for entry, suffix, track, handle, fraction in zip(
        wide["geometry"], ("xs", "s", "m", "l", "xl"),
        (16, 24, 40, 56, 96), (44, 44, 52, 68, 108), (0.25, 0.5, 0.5, 0.5, 0.5), strict=True,
    ):
        slider_geometry(entry, suffix, wide["viewport"], track, handle, fraction, False)
    require(wide["ax"] == ["25", "25％"] and wide["disabled_ax"] == ["50", "50％"]
            and keyboard["ax"] == ["10", "10％"] and compact["ax"] == ["30", "30％"],
            "Slider numeric or localized accessibility value differs")
    colors = wide["colors"]
    require(len(colors) == 3 and all(slider_color(color, 3) for color in colors)
            and colors[0] == colors[2] != colors[1], "Slider track/handle colors differ")
    for kind in ("stop", "tick"):
        dot = slider_rect(wide[kind], kind)
        require(all(slider_near(side, 4) for side in dot[2:]), f"wrong Slider {kind} dimensions")
    require(slider_contains(wide["geometry"][0]["inactive"], wide["stop"])
            and slider_contains(wide["geometry"][1]["active"], wide["tick"]),
            "Slider stop or tick falls outside its track")

    g = slider_geometry(keyboard["geometry"], "xs", wide["viewport"], 16, 44, 0.1, True)
    first = wide["geometry"][0]
    label = slider_rect(keyboard["value_label"], "keyboard indicator")
    ring = slider_rect(keyboard["ring"], "keyboard focus")
    require(g["hit"] == first["hit"] and g["rail"] == first["rail"]
            and slider_near(ring[0], g["handle"][0] - 5) and slider_near(ring[1], g["handle"][1] - 5)
            and slider_near(ring[2], 12) and slider_near(ring[3], 54)
            and label[3] >= 44 and slider_near(label[1] + label[3] + 4, ring[1])
            and type(keyboard["focus_ink"]) is int and keyboard["focus_ink"] > 15
            and type(keyboard["label_ink"]) is int and keyboard["label_ink"] > 15
            and type(keyboard["continuous_requests"]) is int and keyboard["continuous_requests"] == 4
            and keyboard["discrete"] == 62.5, "Slider keys, focus ring or indicator differ")

    drag = slider_rect(pointer["handle"], "held mouse handle")
    require(slider_near(pointer["tap"], 33.3, 0.4)
            and slider_near(pointer["held"], 73.1, 0.4)
            and pointer["canceled"] == pointer["held"]
            and slider_near(drag[2], 2) and slider_near(drag[3], 44)
            and slider_near(drag[0] + drag[2] / 2,
                            first["rail"][0] + first["rail"][2] * pointer["held"] / 100)
            and pointer["touch"] == 75 and pointer["requests"] == [2, 2, 0, 0, 0, 0]
            and all(type(n) is int for n in pointer["requests"]),
            "Slider continuous drag, cancellation, discrete touch or disabled counts differ")

    g = slider_geometry(compact["geometry"], "xs", compact["viewport"], 16, 44, 0.3, True, True)
    label = slider_rect(compact["label"], "compact indicator")
    require(slider_contains([0, 0, *compact["viewport"]], label)
            and label[3] >= 64 and slider_near(label[1] + label[3] + 9, g["handle"][1])
            and type(compact["label_ink"]) is int and compact["label_ink"] > 40
            and compact["back_focus"] == "slider-demo-show", "compact Slider paint or return focus differs")

    keys = {"background", "primary", "on_primary", "secondary", "secondary_container",
            "on_secondary_container", "inverse_surface", "inverse_on_surface"}
    if require_palette or any("palette" in part for part in (wide, keyboard, compact)):
        for part in (wide, keyboard, compact):
            palette = part.get("palette")
            require(isinstance(palette, dict) and keys <= palette.keys()
                    and all(slider_color(palette[key], 4) and palette[key][3] == 255 for key in keys),
                    "missing or invalid Slider palette")
        require(wide["palette"] == keyboard["palette"] != compact["palette"]
                and colors == [wide["palette"][key][:3] for key in ("primary", "secondary_container", "primary")],
                "Slider light and dark palettes disagree")


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
    after = verify_inputs(runner, approved_sha, stock)
    require(after == inputs, "sealed assets changed during the run")
    json_to(out / "inputs-after.json", after)
    verdict = verify_run(out, receipt, inputs["release"])
    json_to(out / "verified-slider-refinements-stock.json", verdict)
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
    parser.add_argument("--verify-inputs-only", action="store_true", help="check the sealed executable bundle without starting it")
    args = parser.parse_args()
    if not __debug__:
        parser.error("run without -O")
    if args.self_test:
        self_test()
        return
    if args.verify_inputs_only:
        if not args.runner_sha256:
            parser.error("--verify-inputs-only requires --runner-sha256")
        print(json.dumps(verify_inputs(args.runner.resolve(strict=True), args.runner_sha256,
                                       args.stock.resolve(strict=True)), indent=2))
        return
    if not args.out or not args.runner_sha256 or args.timeout <= 0:
        parser.error("running requires a new --out directory, approved --runner-sha256, and positive --timeout")
    observe(args.out.resolve(), args.runner.resolve(strict=True), args.runner_sha256,
            args.stock.resolve(strict=True), args.timeout)


if __name__ == "__main__":
    main()
