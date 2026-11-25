#!/usr/bin/env python3
"""Export and recheck exact Text Field Adornments proof without launching hosts."""

import argparse
import gzip
from hashlib import file_digest, sha1, sha256
import json
from math import ceil, floor
import os
from pathlib import Path, PurePosixPath
import re
from contextlib import contextmanager
from runpy import run_path
import shutil
import subprocess
import sys
import tomllib

from PIL import Image

STAGE = Path("/tmp/surface-md3-text-field-counter-v0164.zL1rFusV")
BASE = Path("/tmp/surface-md3-text-field-slots-v0163.0XGv3LWN")
STOCK = Path("/tmp/surface-md3-counter-stock-v0164.ZBB8Yftn")
NATIVE = Path("/tmp/surface-md3-counter-native-r2.NqEd6dD0")
NATIVE_R1 = Path("/tmp/surface-md3-counter-native.jGyZbVdk")
NATIVE_PREVIEW = Path("/tmp/surface-md3-counter-preview-v0164.cy5DF1If")
VERSION = "0.1.64"
SOURCE = "fe7349b293de3078ca44fd6b1fa1885a5fa156ba559503c795cd9d6daab5e4a3"
TREE = "eeef2dc049409f7df3a67773d8e72e76b298faf6"
BASE_SOURCE = "2d671c1f3c4b450a21ab728a22ba0f8126af4790291a1b718d9005ab4f29eec2"
BASE_TREE = "3e2f280ed159c28f5b939813c4d4fd88d26803ff"
PATCH = "290f982d8fed349dfa10c17a517f8acaa3534ad5db04d2e05618e10caf5b2437"
PATCH_NAME = "counter-v0164.patch"
STOCK_HARNESS = "c95eb4ffc1dbca780d31a2437f9c9784dbf8d6ad34891e0d180fa3be6a42047d"
NATIVE_HELPERS = "1e65797c8f3f03976191d19a02b54de9fe58904ee958b02a1bd13175c5724fbe"
NATIVE_REPLAY = "686c6abd05590584c67dce8f2af2b57bd246109f8c5fe1afa5b46f055daf5d6b"
NATIVE_BINARY_SHA = "1f6bf2107ef940ba23912ac57b349a188cfa034d94dc8ffa574269ba195dc098"
CALIBRATION_SHA = "9171e513fed5dbb7d15a19add70d30c9d787d2714857e6c5cdca4ef74e8e8e4d"
R1_IMAGE = "42bcd73f9e5c1279d0768a2e08d619120838daf919b75db43633728e2d3cf729"
PREVIEW_COMPACT = "8fc0e1b8d92f19275223bade7b138ffedbfd51d63ada704c1c850859738718a1"
NATIVE_SEAL = "657d51095ccd99366c2ff931c5385e4003f17fd430ffeaef9e3108480ed5685d"
FOCUS_HARNESS = "33b95a930b200190420d5a02f09fe2e2a2cab7a8fdf37a307e0272614fc64b72"
FOCUS_DIRECTORY = "development-2"
COUNTER_REPORT = "material-text-field-counter.json"
ADORNED_REPORT = "material-text-field-adornments.json"
FOCUS_RUNNER = "7baaf6e18ccea7ec7e42f106b8bd0c26531b119e40f37ceeb6fa7c4a4b78901a"
STOCK_REPORT = "verified-counter-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
OBSERVATION = "headless-window-observation.json"
NATIVE_REPORT = "native-text-field-counter-report.json"
FILLED_REPORT = "material-filled-text-field.json"
OUTLINE_REPORT = "material-outlined-text-field.json"
FINAL_REPORT = "verified-counter-v0164.json"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
RUST_LOGS = ("workspace-v0164-exact.log", "gpu-v0164-exact.log", "gpu-v0164-supplemental.log")
NATIVE_IMAGES = {
    "wide-outlined-seed.png": (960, 1120), "wide-outlined-overflow.png": (960, 1120),
    "wide-outlined-manual-error.png": (960, 1120), "wide-filled-seed.png": (960, 1120),
    "compact-outlined-rtl-200.png": (320, 1200), "compact-filled-overflow-rtl-200.png": (320, 1200),
    "compact-filled-stacked-112.png": (320, 1200),
}
STOCK_PINS = ("browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256",
              "release-pin.json", "source-origins.json", "build-markers.json", "sealed-inputs.sha256",
              "inputs-before.json", "approved-runner.sha256")
RETAINED = ("browser-ime.json", "browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json",
            "browser-smoke.json", "browser-material-side-sheet.json", "bridge-web-result.json",
            "devtools-headless-result.json", "material-standard-slider.json", "material-slider-forms.json",
            "material-range-slider.json", "material-segmented.json", "material-buttons.json",
            "material-vertical-centered-slider.json", "material-slider-inset.json")
HEX = re.compile(r"[a-f0-9]{64}")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def read_json(path):
    return json.loads(path.read_text())


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True) + "\n")


def manifest(path):
    entries = {}
    for line in path.read_text().splitlines():
        sha, name = line.split("  ", 1)
        relative = PurePosixPath(name)
        require(HEX.fullmatch(sha) and not relative.is_absolute() and ".." not in relative.parts
                and relative.as_posix() == name and name not in entries and name != ".",
                f"Invalid entry in {path.name}: {name}")
        entries[name] = sha
    require(entries, f"Empty manifest: {path}")
    return entries


def verify_entries(root, entries):
    for name, sha in entries.items():
        path = root / name
        require(path.is_file() and not path.is_symlink()
                and path.resolve(strict=True).is_relative_to(root.resolve(strict=True))
                and digest(path) == sha, f"Different or missing manifest member: {path}")


def verify_seal(root, filename, expected, *, strict=False):
    path = root / filename
    require(HEX.fullmatch(expected or "") and digest(path) == expected, f"Wrong {path} seal")
    entries = manifest(path)
    verify_entries(root, entries)
    if strict:
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file() and p != path}
        require(actual == set(entries), f"Unsealed files in {root}")
    return entries


def object_id(kind, data):
    return sha1(f"{kind} {len(data)}\0".encode("ascii") + data).digest()


def source_tree(root, entries):
    """Calculate the exact Git tree without writing repository objects."""
    folders = {}
    for name, checksum in entries.items():
        *parents, filename = PurePosixPath(name).parts
        folder = folders
        for parent in parents:
            folder = folder.setdefault(parent, {})
        path = root / name
        data = path.read_bytes()
        require(sha256(data).hexdigest() == checksum, f"Source changed during tree verification: {name}")
        mode = b"100755" if path.stat().st_mode & 0o100 else b"100644"
        folder[filename] = mode, object_id("blob", data)

    def git_tree(folder):
        def git_name(item):
            name, child = item
            return (name + ("/" if isinstance(child, dict) else "")).encode()

        data = []
        for name, child in sorted(folder.items(), key=git_name):
            mode, oid = (b"40000", git_tree(child)) if isinstance(child, dict) else child
            data.append(mode + b" " + name.encode() + b"\0" + oid)
        return object_id("tree", b"".join(data))

    return git_tree(folders).hex()


def materialize(log):
    return gzip.decompress(log.read_bytes()) if log.suffix == ".gz" else log.read_bytes()


def tests(log):
    raw = materialize(log)
    text = raw.decode()
    rows = re.findall(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", text, re.M)
    require(rows and all(result == "ok" and failed == "0" for result, _, failed, _ in rows),
            f"Missing or failed tests in {log.name}")
    return text, {"passed": sum(int(passed) for _, passed, _, _ in rows),
                  "ignored": sum(int(ignored) for _, _, _, ignored in rows),
                  "log_sha256": sha256(raw).hexdigest()}


def open_png(path, expected):
    with Image.open(path) as picture:
        require(picture.format == "PNG" and picture.size == expected, f"Wrong PNG dimensions: {path}")
        picture.load()
        return picture.convert("RGBA")


def scan_box(picture, rect):
    left = max(0, ceil(rect["x"]))
    top = max(0, ceil(rect["y"]))
    right = min(picture.width, floor(rect["x"] + rect["width"]))
    bottom = min(picture.height, floor(rect["y"] + rect["height"]))
    require(left < right and top < bottom, "Empty native paint sample")
    return ((x, y) for y in range(top, bottom) for x in range(left, right))


def guard(window):
    require(window["stock_exit"] == 0 and window["visible_owned_windows"] == []
            and window["remaining_observed_processes"] == [] and window["problems"] == [],
            "Chrome guard saw a visible window, surviving process or failure")


def check_launches(root, window, invocation):
    launch = window["guard_events"]
    ledger = [read_json(file) for file in (root / "guard-ledger").glob("*.json")]
    roles = {"chrome-primary", "chrome-bridge", "native"}
    require(len(launch) == 3 and {event["role"] for event in launch} == roles
            and len({event["pid"] for event in launch}) == 3
            and sorted(launch, key=lambda event: event["pid"]) == sorted(ledger, key=lambda event: event["pid"])
            and all(event["allowed"] is True for event in launch)
            and len({event["profile"] for event in launch if event["profile"]}) == 2,
            "Exact stock process ledger differs")
    observations = window["launch_observations"]
    require(set(observations) == roles and observations["chrome-bridge"]["detached_samples"] > 0
            and all(observations[event["role"]]["samples"] > 0
                    and observations[event["role"]]["pids"] == [event["pid"]] for event in launch),
            "A guarded child was not observed live")
    argv = invocation["argv"]
    require(argv.count("--artifacts") == 1 and argv[-len(SUITES):] == list(SUITES),
            "Raw stock command differs")
    profile_parent = PurePosixPath(argv[argv.index("--artifacts") + 1]).parent / "tmp"
    for event in launch:
        if event["role"] == "native":
            require(event["profile"] is None and event["argv"] == ["--headless", "--width", "720", "--height", "640"],
                    "Raw native headless flags differ")
            continue
        flags = [arg for arg in event["argv"] if arg == "--headless" or arg.startswith("--headless=")]
        profiles = [arg.removeprefix("--user-data-dir=") for arg in event["argv"] if arg.startswith("--user-data-dir=")]
        profile = PurePosixPath(event["profile"])
        require(flags == ["--headless=new"] and profiles == [event["profile"]]
                and profile.is_absolute() and ".." not in profile.parts
                and profile.is_relative_to(profile_parent) and profile != profile_parent,
                "Raw Chrome headless flags or private profile differ")


def copy(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)


def zipped(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(gzip.compress(source.read_bytes(), mtime=0))


def manifest_package(path):
    files = sorted(p for p in path.rglob("*") if p.is_file())
    (path / "SHA256SUMS").write_text("".join(f"{digest(p)}  {p.relative_to(path)}\n" for p in files))
    return digest(path / "SHA256SUMS")


def source_info(stage, base):
    prior, candidate = base / "source.sha256", stage / "source.sha256"
    require(digest(prior) == BASE_SOURCE and digest(candidate) == SOURCE, "Wrong base or candidate source seal")
    before, after = manifest(prior), manifest(candidate)
    for root, entries, count, tree in ((base / "source", before, 675, BASE_TREE),
                                       (stage / "source", after, 678, TREE)):
        require(len(entries) == count, "Wrong source inventory count")
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
                  if p.is_file() and p.relative_to(root).parts[0] != ".git"}
        require(actual == set(entries), f"Source file inventory differs: {root}")
        verify_entries(root, entries)
        require(source_tree(root, entries) == tree, f"Wrong exact source tree: {root}")
    root = stage / "source"
    patch = stage / PATCH_NAME
    require(digest(patch) == PATCH and (stage / "source-tree").read_text().strip() == TREE
            and tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == VERSION,
            "Wrong candidate patch, tree marker or workspace version")
    material = (root / "e2e/src/material.rs").read_text()
    registry = re.search(r"pub const STEPS: &\[&str\] = &\[(.*?)\];", material, re.S)
    required_steps = [str(step) for step in range(1, 341) if step != 23]
    require(registry and re.findall(r'"M(\d+)"', registry.group(1)) == required_steps,
            "The stock runner must require every Material step through M340 except failure-only M23")
    paths = {p.decode() for p in re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch.read_bytes(), re.M)}
    changed = {name for name in before.keys() | after.keys() if before.get(name) != after.get(name)}
    require(paths == changed and len(paths) == 13,
            "Patch, source inventory and the 13 reviewed paths differ")
    for directory, options in ((base / "source", []), (root, ["--reverse"])):
        subprocess.run(["git", "-C", str(directory), "apply", "--check", "--whitespace=error-all",
                        *options, str(patch)], check=True, env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"})
    return {"version": VERSION, "source_sha256": SOURCE, "source_files": len(after), "tree": TREE,
            "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
            "changed_paths": sorted(paths)}


def require_paths(root, names):
    missing = [str(root / name) for name in names if not (root / name).is_file()]
    require(not missing, "Await exact evidence:\n" + "\n".join(missing))


@contextmanager
def helper_paths(root):
    sys.path.insert(0, str(root))
    try:
        yield
    finally:
        sys.path.pop(0)


def geometry_rect(rect, viewport, *, empty=False):
    require(set(rect) == {"x", "y", "width", "height"}, "wrong native rectangle keys")
    x, y, width, height = (rect[key] for key in ("x", "y", "width", "height"))
    require(all(isinstance(value, (float, int)) for value in (x, y, width, height))
            and 0 <= x <= x + width <= viewport[0] and 0 <= y <= y + height <= viewport[1]
            and (empty or min(width, height) > 0), "native rectangle outside its PNG")
    return x, y, width, height


INITIAL = "中a\u0301👍🏽"
AT_LIMIT = INITIAL + "🇹🇼👩‍💻"
OVER = AT_LIMIT + "Z"
HELP = "請輸入 5 個字以內的標籤名稱。英文字母、中文或表情符號都可以使用。"
OVERFLOW = "請輸入最多 5 個字。"
MANUAL = "這個標籤已存在。"
EDITABLE = "text-field-counter-demo-editable"
LOCKED = {"text-field-counter-demo-read-only": ("🇹🇼a\u0301", 2, "已儲存的標籤。"),
          "text-field-counter-demo-disabled": ("停用標籤內容", 6, None)}
NATIVE_CASES = {
    "wide-outlined-seed.png": ("outlined", INITIAL, 3, HELP, False, 1),
    "wide-outlined-overflow.png": ("outlined", OVER, 6, OVERFLOW, False, 3),
    "wide-outlined-manual-error.png": ("outlined", INITIAL, 3, MANUAL, False, 3),
    "wide-filled-seed.png": ("filled", INITIAL, 3, HELP, False, 3),
    "compact-outlined-rtl-200.png": ("outlined", INITIAL, 3, HELP, False, 3),
    "compact-filled-overflow-rtl-200.png": ("filled", OVER, 6, OVERFLOW, False, 3),
    "compact-filled-stacked-112.png": ("filled", INITIAL, 3, HELP, True, 3),
}
PAPER = {False: (253, 247, 255, 255), True: (20, 18, 24, 255)}
NEUTRAL = {False: (73, 69, 78, 255), True: (202, 196, 207, 255)}
ERROR = {False: (186, 26, 26, 255), True: (255, 180, 171, 255)}
PRIMARY = {False: (101, 85, 143, 255), True: (207, 189, 254, 255)}
FILLED_FACE = {False: (230, 224, 233, 255), True: (54, 52, 58, 255)}
DISABLED_INK = {False: (29, 27, 32, 97), True: (230, 225, 233, 97)}


def native_state(state, field, text, count, helper, invalid=False):
    desc = ([field + "-prefix-description"] if field == EDITABLE else []) + [field + "-counter-description"]
    action = {"name": "清除標籤", "disabled": not text} if field == EDITABLE else None
    require(state == {"id": field, "value": text, "counter": f"{count}/5", "invalid": invalid,
                      "description": helper, "description_ids": desc,
                      "spoken_count": f"字數，已輸入 {count} 個字，上限 5 個字。",
                      "assertive_status": helper if invalid else "", "clear": action}
            and state["invalid"] is invalid, "native counter state, limit or passive description differs")


def rows_of(picture):
    width = 4 * picture.width
    pixels = picture.tobytes()
    return [pixels[offset:offset + width] for offset in range(0, len(pixels), width)]


def native_geometry(geometry, viewport, *, rtl, stacked, appearance, edited):
    bounds = {key: geometry_rect(geometry[key], viewport) for key in ("face", "editor", "count", "spoken")}
    face, editor, count, spoken = (bounds[key] for key in ("face", "editor", "count", "spoken"))
    x, y, width, height = face
    cx, cy, cw, ch = count
    require(count == spoken and ch == (32 if rtl else 16) and cx == (x + 16 if rtl else x + width - cw - 16)
            and height == ((96 if appearance == "filled" else 80) if rtl else 56),
            "native face or logical counter inset differs")
    helper = geometry.get("helper")
    if helper:
        hx, hy, hw, hh = geometry_rect(helper, viewport)
        require(hy == y + height + 4 and (hx + hw == x + width - 16 if rtl else hx == x + 16),
                "native support 4/16 dp spacing differs")
        if stacked:
            require(width == 112 and (hx, hw, cy) == (x + 16, 80, hy + hh + 4),
                    "native 112 dp help and count stack differs")
        else:
            gap = hx - cx - cw if rtl else cx - hx - hw
            require(cy == hy and gap == 16 and hw >= 48, "native 16 dp help-to-count gap differs")
    else:
        require(cy == y + height + 4, "counter without helper no longer starts below its face")
    if edited:
        ax, ay, aw, ah = geometry_rect(geometry["action"], viewport)
        require((aw, ah) == (48, 48), "native Clear target is undersized")
        if stacked:
            require(editor == face and ax == x and ay == cy + ch + 4, "relocated native action crowds count")
        else:
            expected_editor_x = x + 48 if rtl else x
            require(editor == (expected_editor_x, y, width - 48, height)
                    and ax == (x if rtl else x + width - 48) and ay + 24 == y + height / 2,
                    "native action and editor must have disjoint hit bounds")
        tx, ty, tw, th = geometry_rect(geometry["trailing"], viewport)
        require((tx, ty, tw, th) == (ax + 12, ay + 12, 24, 24), "native trailing glyph must fit its action")
        geometry_rect(geometry["value"], viewport)
        geometry_rect(geometry["prefix"], viewport)
    else:
        require(editor == face and "action" not in geometry, "locked native editor gained an action")
    return face


def native_ink(measure, rows, rect, authored, backdrop, reported):
    require(reported["rgba"] == list(authored), "native authored count or helper color differs")
    calculated = measure(rows, rect, authored, backdrop)
    require(json.loads(json.dumps(calculated)) == reported, "native painted glyph fit differs")
    return calculated


def native_pixels(directory, shot, mode, measure):
    name = shot["name"]
    appearance, value, count, helper, stacked, edge = NATIVE_CASES[name]
    invalid = helper != HELP
    rtl = mode == "compact"
    viewport = NATIVE_IMAGES[name]
    require(mode == name.split("-")[0] and shot["appearance"] == appearance and shot["rtl"] is rtl
            and shot["stacked"] is stacked and (shot["width"], shot["height"], shot["scale"]) == (*viewport, 1)
            and shot["format"] == "png", "native screenshot scope or geometry differs")
    path = directory / name
    require(digest(path) == shot["sha256"], "native PNG digest differs")
    picture = open_png(path, viewport)
    rows = rows_of(picture)
    native_state(shot["state"], EDITABLE, value, count, helper, invalid)
    x, y, width, height = native_geometry(shot["geometry"], viewport,
                                          rtl=rtl, stacked=stacked, appearance=appearance, edited=True)
    palette, geometry = shot["paint"], shot["geometry"]
    paper, neutral, error = PAPER[rtl], NEUTRAL[rtl], ERROR[rtl]
    fill = FILLED_FACE[rtl] if appearance == "filled" else paper
    stroke = (122, 117, 127, 255) if edge == 1 else error if invalid else PRIMARY[rtl]
    actual_edge = [list(picture.getpixel((int(x + width / 2), floor(y + height - index - 0.5))))
                   for index in range(5)]
    require(actual_edge == shot["face_paint"]["samples"] == [list(stroke)] * edge + [list(fill)] * (5 - edge)
            and shot["face_paint"]["bottom_edge_dp"] == edge and shot["face_paint"]["fill"] == list(fill)
            and palette["backdrop"] == list(paper), "native actual boundary or canvas color differs")
    if appearance == "filled":
        top = floor(y + .5)
        require(picture.getpixel((floor(x + .5), top)) == paper
                and picture.getpixel((floor(x + 6), top)) == fill
                and picture.getpixel((floor(x + .5), floor(y + height - .5))) == stroke,
                "native Filled corners lost")
    native_ink(measure, rows, geometry["count"],
               error if count > 5 else neutral, paper, palette["counter"])
    native_ink(measure, rows, geometry["helper"], error if invalid else neutral, paper, palette["helper"])
    native_ink(measure, rows, geometry["prefix"], neutral, paper if stacked else fill, palette["affix"])
    locked = shot["locked"]
    wanted_locked = set(LOCKED) if name in {"wide-outlined-seed.png", "wide-filled-seed.png"} else set()
    require(set(locked) == wanted_locked, "native locked specimen evidence differs")
    for field, data in locked.items():
        text, count, helper = LOCKED[field]
        native_state(data["state"], field, text, count, helper)
        native_geometry(data["geometry"], viewport, rtl=rtl, stacked=False, appearance=appearance, edited=False)
        inks = data["paint"]
        native_ink(measure, rows, data["geometry"]["count"], neutral if helper else DISABLED_INK[rtl],
                   paper, inks["counter"])
        if helper:
            native_ink(measure, rows, data["geometry"]["helper"], neutral, paper, inks["helper"])
    return name, shot["sha256"]


def native_profile_behavior(behavior, mode):
    require(behavior["clear_feedback"] == "已清除標籤。", "native Clear feedback differs")
    if mode == "compact":
        native_state(behavior["relocated_clear"], EDITABLE, "", 0, HELP)
        return
    native_state(behavior["soft_overflow_submit"], EDITABLE, OVER, 6, OVERFLOW, True)
    native_state(behavior["manual_priority_during_overflow"], EDITABLE, OVER, 6, MANUAL, True)
    native_state(behavior["exact_limit"], EDITABLE, AT_LIMIT, 5, HELP)
    native_state(behavior["clear"], EDITABLE, "", 0, HELP)
    require(behavior["submit_feedback"] == "已送出標籤。" and set(behavior["locked"]) == set(LOCKED),
            "native soft limit blocks independent submission or lacks locked fields")
    for key, (text, count, helper) in LOCKED.items():
        native_state(behavior["locked"][key], key, text, count, helper)


def native_tools(harness):
    verify_seal(harness, "helpers.sha256", NATIVE_HELPERS)
    replay = harness / "replay-text-field-counter.py"
    require(digest(replay) == NATIVE_REPLAY, "Changed native Counter replay harness")
    with helper_paths(harness):
        return run_path(str(replay))["painted_ink"]


def native_info(proof, binary_manifest, terminal, harness):
    measure = native_tools(harness)
    entries = verify_seal(proof, "SHA256SUMS", NATIVE_SEAL, strict=True)
    report = read_json(proof / NATIVE_REPORT)
    binaries = manifest(binary_manifest)
    require(digest(binary_manifest) == NATIVE_BINARY_SHA and set(binaries) == {"surface-material-gallery", "surface-devtools"}
            and binaries == report["binaries"] and report["binary_manifest_sha256"] == NATIVE_BINARY_SHA, "The native proof used different exact executable bytes")
    require(report["source"] == {"manifest_sha256": SOURCE, "files": 678}
            and report["transports"] == ["Page.screenshot", "CLI", "MCP"]
            and report["schema"] == "surface.test.text-field-counter.native.v1"
            and report["semantics"] == "Styled Runtime DOM through native headless DevTools"
            and len(report["replays"]) == len(report["processes"]) == 2
            and report["replays"][0] == report["replays"][1], "Wrong exact native inputs or replay parity")
    require(set(entries) == {f"replay-{i}/{name}" for i in (1, 2) for name in NATIVE_IMAGES} | {NATIVE_REPORT},
            "Wrong native original inventory")
    processes = []
    common = {"ready", "opened", "captures_saved", "closed"}
    signals = {"wide": common | {"submitted_soft_overflow", "clear_and_locked_fields"},
               "compact": common | {"compact_overflow", "compact_relocated_clear"}}
    for index, (profiles, children) in enumerate(zip(report["replays"], report["processes"], strict=True), 1):
        require(set(profiles) == set(children) == {"wide", "compact"}, "Missing native profile")
        seen = {}
        for mode, profile in profiles.items():
            require(profile["viewport"] == ([960, 1120] if mode == "wide" else [320, 1200])
                    and len(profile["captures"]) == (4 if mode == "wide" else 3), "Wrong native canvas geometry")
            native_profile_behavior(profile["behavior"], mode)
            for shot in profile["captures"]:
                name, checksum = native_pixels(proof / f"replay-{index}", shot, mode, measure)
                require(name not in seen and entries[f"replay-{index}/{name}"] == checksum, "Native image inventory changed")
                seen[name] = checksum
            child = children[mode]
            require(child["pid"] > 0 and child["reaped"] is True
                    and child["visible_windows"] == dict.fromkeys(signals[mode], 0),
                    "A native process remained or had a visible window")
            processes.append(child["pid"])
        require(set(seen) == set(NATIVE_IMAGES), "Missing native images")
    require(len(set(processes)) == 4 and terminal.read_text().strip().endswith("/exact-1")
            and terminal.read_text().startswith("2 out of 2 headless replays passed at "), "Wrong native terminal proof")
    return {"runs": 2, "binaries": binaries, "binary_manifest_sha256": digest(binary_manifest),
            "transports": report["transports"], "visible_windows": 0, "window_samples": 24,
            "owned_processes_reaped": 4, "captures_each": seen, "seal": NATIVE_SEAL,
            "report_sha256": digest(proof / NATIVE_REPORT)}


def calibration_info(details, original, preview_wide, preview_compact, failed_log, harness):
    require(digest(details) == CALIBRATION_SHA and digest(original) == digest(preview_wide) == R1_IMAGE
            and digest(preview_compact) == PREVIEW_COMPACT, "native calibration receipts or originals changed")
    data = read_json(details)
    require(data["scope"] == "previously captured original PNG calibration"
            and data["preview_wide_byte_identical"] is True and data["failed_run_png_sha256"] == R1_IMAGE
            and data["original_r1_rejection"] in materialize(failed_log).decode(),
            "native R1 predicate failure does not bind its original capture")
    measure = native_tools(harness)
    for mode, image, rtl in (("wide", original, False), ("compact", preview_compact, True)):
        shot = data[mode]
        viewport = (320, 1200) if rtl else (960, 1120)
        picture = open_png(image, viewport)
        geometry_rect(shot["bounds"], viewport)
        expected = NEUTRAL[rtl]
        hits = sum(max(abs(a - b) for a, b in zip(picture.getpixel(point)[:3], expected[:3], strict=True)) <= 3
                   for point in scan_box(picture, shot["bounds"]))
        require(shot["sha256"] == digest(image) and shot["background"] == list(PAPER[rtl])
                and hits == shot["old_endpoint_hits"], "native calibration endpoint count differs")
        measured = native_ink(measure, rows_of(picture), shot["bounds"], expected, PAPER[rtl],
                              shot["r2_original_pixel_check"])
        require(shot["erroneous_palette_rejected"].startswith("Text pixels do not match the authored hue and opacity")
                and shot["erroneous_38_percent_alpha_rejected"].startswith("Text pixels do not match the authored hue and opacity")
                and (rtl or (hits == 0 and measured["ink_pixels"] == 51 and measured["peak_coverage"] > 0.95)),
                "native calibration no longer distinguishes antialiasing from absent ink")
    return {"scope": "archived native calibration only", "source_sha256": CALIBRATION_SHA,
            "wide_original_sha256": R1_IMAGE, "wide_r1_endpoint_hits": data["wide"]["old_endpoint_hits"],
            "wide_r2_ink_pixels": data["wide"]["r2_original_pixel_check"]["ink_pixels"],
            "compact_preview_sha256": PREVIEW_COMPACT}


def toolchain(workspace, metal, supplemental):
    text, rust = tests(workspace)
    tail = text.rsplit("\ntest result:", 1)[-1]
    named = ("text_field_counter_steps_are_required_by_the_material_case",
             "counter_counts_committed_graphemes_and_exposes_one_soft_limit_error",
             "counter_and_support_share_wide_rows_and_stack_in_compact_rtl",
             "text_field_adornment_steps_are_required_by_the_material_case",
             "adornments_keep_one_editor_and_a_separate_action_across_both_variants",
             "native_ime_session_commits_before_a_separate_enter_and_cancels_without_editing",
             "ordered_descriptions_keep_clipped_validation_in_ax_help_without_an_id",
             "supplementary_descriptions_require_passive_visible_unique_targets_in_scope",
             "filled_text_field_steps_are_required_by_the_material_case",
             "outlined_text_field_steps_are_required_by_the_material_case",
             "focus_repaints_one_semantic_editor_and_cuts_a_transparent_three_dp_outline",
             "top_inset_keeps_the_whole_editor_target_and_aligns_all_text_geometry",
             "top_inset_preserves_one_measured_line_and_leaves_multiline_unchanged",
             "range_slider_steps_are_required_by_the_material_case")
    require(rust["passed"] == 1543 and rust["ignored"] == 19
            and tail.count("Finished `dev` profile") == 3 and "Doc-tests surface_material_gallery" in text
            and re.search(r"^provenance: .+ self-test cases passed\)", text, re.M)
            and all(f"{name} ... ok" in text for name in named), "Workspace omitted an actual exact Rust or Wasm gate")
    text, gpu = tests(metal)
    require(gpu["passed"] == 16 and gpu["ignored"] == 0 and 'name: "Apple M1 Max"' in text
            and "backend: Metal" in text, "Expected 16 actual Apple Metal GPU tests")
    text, extra = tests(supplemental)
    measured = ("raster::tests::gpu_offscreen_raster_cache_reuses_shared_pixels_and_evicts_by_count_and_bytes",
                "paired_selection_matches_single_draw_references_at_fractional_clip_edges")
    rows = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; 0 ignored;", text, re.M)
    require(rows == ["1", "1"] and extra["passed"] == 2 and extra["ignored"] == 0
            and len(re.findall(r'^GPU evidence: AdapterInfo \{ name: "Apple M1 Max", .* backend: Metal \}$', text, re.M)) == 2
            and all(f"test {name} ... ok" in text for name in measured),
            "Expected both actual supplemental Apple Metal GPU tests")
    return {"workspace": {**rust, "wasm_checks": 3}, "metal": {**gpu, "backend": "Metal"},
            "supplemental": {**extra, "backend": "Metal", "tests": list(measured)},
            "metal_total_passed": gpu["passed"] + extra["passed"]}


def stock_tools(root):
    verify_seal(root, "harness.sha256", STOCK_HARNESS)
    with helper_paths(root):
        observer = run_path(str(root / "observe-counter.py"))
        filled = run_path(str(root / "verify_filled_field.py"))["verify"]
        outlined = run_path(str(root / "verify_outlined_field.py"))["verify"]
        adorned = run_path(str(root / "verify_adorned_field.py"))["verify"]
        counter = run_path(str(root / "verify_counter.py"))["verify"]
    return observer, counter, adorned, filled, outlined


def focused_info(root, verifier, live=False):
    invocation = read_json(root / "invocation.json")
    argv = invocation["argv"]
    require(invocation["scope"] == "focused development pack" and argv[-1] == "material"
            and argv[1:7:2] == ["--chrome", "--web-root", "--artifacts"]
            and len(argv) == 8 and Path(argv[4]).parts[-3:] == (STAGE.name, "web-development", "release")
            and Path(argv[6]).parts[-3:] == (STAGE.name, FOCUS_DIRECTORY, "artifacts")
            and invocation["runner_sha256"] == FOCUS_RUNNER, "Wrong Counter focused source or scope")
    if live:
        runner, web = Path(argv[0]), Path(argv[4]) / "material-gallery"
        require(digest(runner) == FOCUS_RUNNER and runner.parent.name == "focused-bin-r2"
                and set(p.name for p in web.iterdir()) == set(invocation["web_files"]), "Development inputs changed")
        verify_entries(web, invocation["web_files"])
    require(len(invocation["web_files"]) == 4 and {"index.html", "surface-web.json"} <= invocation["web_files"].keys(),
            "Incomplete focused development Web inventory")
    window = read_json(root / OBSERVATION)
    guard(window)
    event, = window["guard_events"]
    ledger, = [read_json(path) for path in (root / "guard-ledger").glob("*.json")]
    active = window["launch_observations"]
    logged = event["argv"]
    profile = PurePosixPath(event["profile"])
    require(ledger == event and event["role"] == "chrome-primary" and event["allowed"] is True
            and [v for v in logged if v == "--headless" or v.startswith("--headless=")] == ["--headless=new"]
            and [v for v in logged if v.startswith("--user-data-dir=")] == [f"--user-data-dir={profile}"]
            and profile.is_relative_to(PurePosixPath(argv[6]).parent / "tmp")
            and active["chrome-primary"]["samples"] > 0 and active["chrome-primary"]["pids"] == [event["pid"]]
            and active["native"]["samples"] == active["chrome-bridge"]["samples"] == 0,
            "Invalid development Chrome guard or samples")
    log_path = root / "runner.log"
    log = materialize(log_path if log_path.is_file() else root / "runner.log.gz")
    text = log.decode()
    require(re.findall(r"^case (\S+) \.\.\.$", text, re.M) == ["material"]
            and re.findall(r"^  \[(M\d+)\]", text, re.M) == [f"M{i}" for i in range(337, 341)]
            and re.findall(r"^case material \.\.\. ok \((\d+) steps, (\d+) checks,", text, re.M) == [("4", "308")]
            and re.findall(r"^e2e result: (.*?); artifacts in ", text, re.M) == ["1 passed; 0 failed; 308 checks"],
            "Wrong development run steps or count")
    browser, = re.findall(r"^browser: (Chrome/\S+) ", text, re.M)
    raw_dir = root / "artifacts"
    adapter = read_json(raw_dir / COUNTER_REPORT)["adapter"]
    require(adapter == {"vendor": "apple", "architecture": "metal-3", "isFallbackAdapter": False},
            "focused Gallery verification requires the observed Apple Metal adapter")
    evidence = verifier(raw_dir, {"browser": browser, "adapter": adapter})
    return {"scope": "focused exact Gallery only", "checks": 308, "steps": 4, "visible_windows": 0,
            "remaining_owned_processes": 0, "runner_sha256": FOCUS_RUNNER,
            "runner_log_sha256": sha256(log).hexdigest(), "window_receipt_sha256": digest(root / OBSERVATION),
            "web_files": invocation["web_files"], "text_field_counter": evidence}


def match_focused_stock(focused, binary_manifest):
    gallery = PurePosixPath("web/release/material-gallery")
    assets = {PurePosixPath(name).name: checksum for name, checksum in manifest(binary_manifest).items()
              if PurePosixPath(name).parent == gallery}
    require(len(assets) == 4 and focused["web_files"] == assets,
            "Focused Gallery Web assets differ from exact canonical stock")


def stock_info(run, pins, offline, observer, counter, adorned, filled, outlined):
    stock, saved = read_json(run / STOCK_REPORT), read_json(offline)
    window, invocation = read_json(run / OBSERVATION), read_json(run / "invocation.json")
    guard(window)
    check_launches(run, window, invocation)
    for key, value in stock.items():
        require(saved[key] == value, f"Archived offline verdict changed at {key}")
    log_path = run / "runner.log"
    log = materialize(log_path if log_path.is_file() else run / "runner.log.gz")
    counts, checks = observer["verify_runner_log"](log.decode(), 340)
    require(digest(run / OBSERVATION) == saved["window_receipt_sha256"]
            and sha256(log).hexdigest() == saved["runner_log_sha256"] and counts == stock["cases"]
            and counts["material"]["steps"] == 339 and checks == stock["total_checks"] and stock["page_errors"] == 0,
            "Missing exact Material steps, original log, or successful window receipt")
    release = {"version": VERSION, "source_sha256": SOURCE, "source_files": 678, "source_tree": TREE,
               "last_material_step": 340, "wide_canvas_height": 800, "rtl_canvas_height": 1020}
    after = read_json(run / "inputs-after.json")
    require(after == invocation["inputs"] == read_json(pins / "inputs-before.json")
            and tuple(invocation["suite_arguments"]) == SUITES and invocation["injected_chrome_flags"] == []
            and after["release"] == saved["release"] == read_json(pins / "release-pin.json") == release
            and after["source_manifest_sha256"] == digest(pins / "browser-source.sha256") == SOURCE
            and after["source_files_checked_per_origin"] == 678
            and after["app_bundle_seal_sha256"] == digest(pins / "sealed-inputs.sha256")
            and after["stock_files"] == manifest(pins / "browser-binary-sha256.txt")
            and after["bundle_files_checked"] == len(after["stock_files"]) == 16
            and after["runner_sha256"] == (pins / "approved-runner.sha256").read_text().strip()
            == after["stock_files"]["debug/surface-e2e"], "Exact stock build or original invocation differs")
    verify_seal(pins, "sealed-inputs.sha256", after["app_bundle_seal_sha256"])
    adapter = stock["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False and stock["guard_and_window_checks"] == "passed",
            "Actual stock WebGPU was not Apple Metal or the guard failed")
    artifacts = run / "artifacts"
    smoke, ime, side, bridge, native = (read_json(artifacts / name) for name in (
        "browser-smoke.json", "browser-ime.json", "browser-material-side-sheet.json",
        "bridge-web-result.json", "devtools-headless-result.json"))
    require(smoke["errors"] == ime["errors"] == side["errors"] == []
            and smoke["browser"] == side["browser"] == stock["browser"]
            and side["adapter"] == {key: smoke["adapter"][key] for key in adapter} == adapter
            and ime["committed"] == "繁體中文、简体中文"
            and bridge["webgpu_ready"] is True and bridge["owned_browser_exited"] is True
            and (native["width"], native["height"], native["scale"]) == (720, 640, 1)
            and native["action_rendered"] is True and native["screenshot_transports_match"] is True,
            "Raw stock Smoke, IME, Bridge or headless Gallery evidence differs")
    select_all = read_json(artifacts / "browser-ime-cancel-redraw.json")
    selected = read_json(artifacts / "browser-ime-cancel-selected-redraw.json")
    keys_before, keys_after = select_all["before"], select_all["after"]
    require(keys_before["browser"] == keys_after["browser"] == [0, 2]
            and keys_before["trustedSelectAll"] is True and keys_after["frames"] > keys_before["frames"]
            and select_all["replacement"] == ["中👨‍👩‍👧‍👦é", "中👨‍👩‍👧‍👦é", "31", "31", "false"]
            and selected["canceled"]["direction"] == "backward" and selected["moved"]["trustedRight"] is True
            and selected["unpainted"]["value"] == selected["repainted"]["value"] == "台文X"
            and selected["unpainted"]["frames"] == selected["canceled"]["frames"] < selected["repainted"]["frames"],
            "Retained I11 copy lost the real selection and redraw evidence")
    metadata = {"browser": stock["browser"], "adapter": adapter}
    newest = counter(artifacts, metadata)
    adornments_receipt = adorned(artifacts, metadata)
    predecessor = filled(artifacts, metadata)
    outlined_receipt = outlined(artifacts, metadata)
    require(newest == stock["text_field_counter"] and adornments_receipt == stock["text_field_adornments"]
            and predecessor == stock["filled_text_field"]
            and outlined_receipt == stock["outlined_text_field"],
            "Independent WebGPU editor pixel replay differs from stock")
    for name, key in (("material-buttons.json", "common_buttons"), ("material-segmented.json", "segmented")):
        require(digest(artifacts / name) == stock[key]["report_sha256"], f"Retained {name} differs")
    return {**metadata, "total_checks": checks, "cases": counts, "page_errors": 0, "visible_windows": 0,
            "surviving_processes": 0, "runner_sha256": after["runner_sha256"],
            "offline_receipt_sha256": digest(offline), "text_field_counter": newest,
            "text_field_adornments": adornments_receipt, "filled_text_field": predecessor,
            "outlined_text_field": outlined_receipt, "retained_reports": {name: digest(artifacts / name) for name in RETAINED}}


def copy_sealed(root, target, name):
    for item in (name, *manifest(root / name)):
        copy(root / item, target / item)


def export(out, receipts, stock, offline, native, binary_manifest):
    require(out.parent.resolve().is_relative_to(Path("/tmp").resolve()), "Use a new output under /tmp")
    out.mkdir(mode=0o700)
    copy(STAGE / "source.sha256", out / "source.sha256")
    copy(BASE / "source.sha256", out / "base-source.sha256")
    zipped(STAGE / PATCH_NAME, out / "counter-v0164.patch.gz")
    copy(Path(__file__), out / "pack-counter.py")
    for name in RUST_LOGS:
        zipped(STAGE / name, out / "rust" / f"{name}.gz")

    capture = out / "native"
    shutil.copytree(native, capture / "proof")
    copy(binary_manifest, capture / "native-binaries.sha256")
    copy(STAGE / "native-final-runtime.log", capture / "terminal.log")
    copy_sealed(NATIVE, capture / "harness", "helpers.sha256")
    copy(NATIVE / "replay-text-field-counter.py", capture / "harness" / "replay-text-field-counter.py")
    proof = out / "calibration"
    copy(NATIVE / "original-pixel-calibration.json", proof / "original-pixel-calibration.json")
    copy(NATIVE_R1 / "development-1" / "replay-1" / "wide-outlined-seed.png", proof / "r1-wide-outlined-seed.png")
    copy(NATIVE_PREVIEW / "wide.png", proof / "preview-wide.png")
    copy(NATIVE_PREVIEW / "compact.png", proof / "preview-compact.png")
    copy(NATIVE_PREVIEW / "preview.json", proof / "preview.json")
    copy(NATIVE / "inspect-stored-counter.py", proof / "inspect-stored-counter.py")
    zipped(STAGE / "native-development-runtime.log", proof / "r1-runner.log.gz")
    copy(NATIVE_R1 / "replay-text-field-counter.py", proof / "r1-replay-text-field-counter.py")
    copy(NATIVE / "README.md", capture / "harness" / "README.md")

    focused = STAGE / FOCUS_DIRECTORY
    target = out / "focused"
    for name in ("invocation.json", OBSERVATION):
        copy(focused / name, target / name)
    shutil.copytree(focused / "guard-ledger", target / "guard-ledger")
    zipped(focused / "runner.log", target / "runner.log.gz")
    for name in (COUNTER_REPORT, *receipts["focused"]["text_field_counter"]["pngs"]):
        copy(focused / "artifacts" / name, target / "artifacts" / name)
    copy_sealed(STAGE, target / "harness", "focused-harness.sha256")
    for name in ("runner.sha256", "scope-selector.rs"):
        copy(STAGE / "focused-bin-r2" / name, target / "harness" / name)

    live_stock, target = stock / "run", out / "stock-macos"
    for name in STOCK_PINS:
        copy(stock / name, target / name)
    copy_sealed(stock, target, "harness.sha256")
    copy(stock / "README.md", target / "reproduction.md")
    for name in ("invocation.json", OBSERVATION, "inputs-after.json", STOCK_REPORT):
        copy(live_stock / name, target / name)
    shutil.copytree(live_stock / "guard-ledger", target / "guard-ledger")
    zipped(live_stock / "runner.log", target / "runner.log.gz")
    copy(offline, target / OFFLINE)
    fields = receipts["stock_macos"]
    for name in (*RETAINED, COUNTER_REPORT, ADORNED_REPORT, FILLED_REPORT, OUTLINE_REPORT,
                 *fields["text_field_counter"]["pngs"], *fields["text_field_adornments"]["pngs"],
                 *fields["filled_text_field"]["pngs"],
                 *fields["outlined_text_field"]["pngs"]):
        copy(live_stock / "artifacts" / name, target / "artifacts" / name)
    write_json(out / FINAL_REPORT, receipts)
    (out / "README.md").write_text(f'''# Surface v0.1.64 Character Counter candidate proof

Exact source manifest `{SOURCE}` pins 678 files. Its Git tree is `{TREE}`.
The 13-path patch applies to the v0.1.63 tree `{BASE_TREE}`. This archive records local evidence.

- All five stock macOS suites passed {fields['total_checks']:,} checks. The
  Material suite ran 339 steps through M340. The IME suite ran all 20 steps.
- Chrome used Apple Metal. The guard observed zero visible owned windows and
  zero owned survivors. The exact run supplied seven Counter PNGs and the
  inherited Adornments, Filled and Outlined originals.
- The separate focused run passed 308 checks on the four canonical Gallery Web
  files also used by stock. Its seven PNGs and observer record have their own
  scope. The focused receipt keeps the passive count, live error deliveries,
  trusted keyboard events and the synthetic CDP cancel and commit steps.
- The native Gallery saved seven PNGs in each of two replays. Page.screenshot,
  CLI and MCP matched. All 24 samples found zero visible windows, and all four
  Gallery processes exited. The replay uses Runtime DOM and DevTools.
- The separate R1 calibration found zero pixels near the unblended foreground
  in the wide ratio. R2 finds 51 strong glyph pixels in the same original PNG.
  Wrong hue and wrong opacity remain rejection controls.
- The workspace passed {receipts['rust']['workspace']['passed']:,} tests and
  three Wasm checks. Apple M1 Max Metal ran 18 offscreen tests, including 16
  primary tests and two supplemental raster-cache and text-selection tests.

System IME and screen-reader speech require separate evidence.

## Offline recheck

From the archive, use Python 3.12 or newer with Pillow. Provide the reviewed
SHA-256 of `SHA256SUMS`. This command reads proof only and starts no app.

```sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-counter.py --verify-package . --package-sha "$manifest_sha"
```

`source.sha256` and `base-source.sha256` verify the exact source bytes.
Decompress `counter-v0164.patch.gz` for the reviewed patch, and decompress the
Rust and runner logs to obtain their original bytes. The copied stock harness,
focused runner builder and native replay script give the exact test recipes.
The original stock run retains all raw artifacts. This archive preserves
its guarded inputs, original observer and runner bytes, editor PNGs and
inherited IME controls.
''')
    seal = manifest_package(out)
    return {"path": str(out), "manifest_sha256": seal, "files": len(manifest(out / "SHA256SUMS"))}


def verify_package(root, expected):
    verify_seal(root, "SHA256SUMS", expected, strict=True)
    result = read_json(root / FINAL_REPORT)
    require(result["status"] == "verified"
            and digest(root / "source.sha256") == SOURCE and len(manifest(root / "source.sha256")) == 678
            and digest(root / "base-source.sha256") == BASE_SOURCE and len(manifest(root / "base-source.sha256")) == 675
            and sha256(materialize(root / "counter-v0164.patch.gz")).hexdigest() == PATCH,
            "Exact candidate or prior source and patch archive changed")
    require(result["source"]["tree"] == TREE and result["source"]["base_tree"] == BASE_TREE
            and result["source"]["source_sha256"] == SOURCE
            and len(result["source"]["changed_paths"]) == 13, "Archived source summary changed")
    stock, focus, native = root / "stock-macos", root / "focused", root / "native"
    observer, counter, adorned, filled, outlined = stock_tools(stock)
    verify_seal(focus / "harness", "focused-harness.sha256", FOCUS_HARNESS)
    match_focused_stock(result["focused"], stock / "browser-binary-sha256.txt")
    require((focus / "harness" / "runner.sha256").read_text().strip() == FOCUS_RUNNER
            and focused_info(focus, counter) == result["focused"], "Archived focused evidence differs")
    require(native_info(native / "proof", native / "native-binaries.sha256", native / "terminal.log", native / "harness")
            == result["native"] and digest(stock / "native-binaries.sha256") == digest(native / "native-binaries.sha256"),
            "Archived native evidence or binary parity differs")
    calibration = root / "calibration"
    require(calibration_info(calibration / "original-pixel-calibration.json", calibration / "r1-wide-outlined-seed.png",
                             calibration / "preview-wide.png", calibration / "preview-compact.png",
                             calibration / "r1-runner.log.gz", native / "harness") == result["calibration"],
            "Archived native calibration differs")
    require(toolchain(*(root / "rust" / f"{name}.gz" for name in RUST_LOGS)) == result["rust"],
            "Archived Rust or Metal evidence differs")
    require(stock_info(stock, stock, stock / OFFLINE, observer, counter, adorned, filled, outlined) == result["stock_macos"],
            "Archived exact stock evidence differs")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-sha")
    parser.add_argument("--tree")
    parser.add_argument("--package-sha")
    parser.add_argument("--stock-offline-sha")
    parser.add_argument("--stock-runner-sha")
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--preflight", action="store_true")
    action.add_argument("--out", type=Path)
    action.add_argument("--verify-package", type=Path)
    args = parser.parse_args()
    if args.verify_package:
        result = verify_package(args.verify_package.resolve(), args.package_sha)
    else:
        require(args.source_sha == SOURCE and args.tree == TREE, "Supply the exact reviewed .64 source seal and tree")
        source = source_info(STAGE, BASE)
        observer, counter, adorned, filled, outlined = stock_tools(STOCK)
        verify_seal(STAGE, "focused-harness.sha256", FOCUS_HARNESS)
        require_paths(STAGE, ("native-v0164-passed", "native-proof-dir", "native-final-runtime.log"))
        native = Path((STAGE / "native-proof-dir").read_text().strip()).resolve(strict=True)
        require(native == (NATIVE / "exact-1").resolve(), "Native sealed path differs")
        binary_manifest = STAGE / "native-v0164-binaries" / "SHA256SUMS"
        verify_entries(binary_manifest.parent, manifest(binary_manifest))
        native_receipt = native_info(native, binary_manifest, STAGE / "native-final-runtime.log", NATIVE)
        calibration = calibration_info(NATIVE / "original-pixel-calibration.json",
            NATIVE_R1 / "development-1" / "replay-1" / "wide-outlined-seed.png", NATIVE_PREVIEW / "wide.png",
            NATIVE_PREVIEW / "compact.png", STAGE / "native-development-runtime.log", NATIVE)
        result = {"status": "preflight only", "source": source, "native": native_receipt, "calibration": calibration}
        required_rust = ("workspace-v0164-passed", "gpu-v0164-passed", *RUST_LOGS)
        if args.out or all((STAGE / name).is_file() for name in required_rust):
            require_paths(STAGE, required_rust)
            result["rust"] = toolchain(*(STAGE / name for name in RUST_LOGS))
        else:
            result["rust"] = "awaiting exact workspace and Metal pass markers"
        focus = STAGE / FOCUS_DIRECTORY
        focused_ready = all((focus / name).is_file() for name in ("runner.log", OBSERVATION, "invocation.json"))
        if focused_ready:
            result["focused"] = focused_info(focus, counter, live=True)
            canonical = STAGE / "web-canonical" / "web" / "release" / "material-gallery"
            require(set(result["focused"]["web_files"]) == {p.name for p in canonical.iterdir()},
                    "canonical Gallery Web inventory differs")
            verify_entries(canonical, result["focused"]["web_files"])
            if (STOCK / "browser-binary-sha256.txt").is_file():
                match_focused_stock(result["focused"], STOCK / "browser-binary-sha256.txt")
        else:
            require(not args.out, f"Await exact focused evidence: {focus}")
            result["focused"] = "awaiting exact canonical Gallery replay"
        if args.out:
            require_paths(STOCK, ("stock-mac-v0164-passed", OFFLINE, "approved-runner.sha256"))
            offline = STOCK / OFFLINE
            require(args.stock_offline_sha == digest(offline) and HEX.fullmatch(args.stock_offline_sha or "")
                    and args.stock_runner_sha == (STOCK / "approved-runner.sha256").read_text().strip()
                    and HEX.fullmatch(args.stock_runner_sha or "") and digest(STOCK / "native-binaries.sha256") == digest(binary_manifest),
                    "Supply exact stock receipt and runner digests and matching native executable pins")
            baked = subprocess.run([sys.executable, "-E", "-B", str(STOCK / "verify-stock-receipt.py"), str(STOCK / "run")],
                                   capture_output=True, text=True, check=False)
            require(baked.returncode == 0 and json.loads(baked.stdout) == read_json(offline),
                    f"Original strict stock replay failed: {baked.stderr}")
            result["stock_macos"] = stock_info(STOCK / "run", STOCK, offline, observer, counter, adorned, filled, outlined)
            require(result["stock_macos"]["runner_sha256"] == args.stock_runner_sha, "The stock runner seal changed")
            result["status"] = "verified"
            result = {**result, "output": export(args.out.resolve(), result, STOCK, offline, native, binary_manifest)}
    print(json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, RuntimeError, FileNotFoundError, subprocess.CalledProcessError, KeyError, TypeError) as error:
        sys.exit(f"Character Counter proof stopped. {error}")
