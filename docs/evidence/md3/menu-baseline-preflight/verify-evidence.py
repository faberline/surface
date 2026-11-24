#!/usr/bin/env python3
"""Check the Menu receipts and optionally reconstruct the pinned source from Divider."""

import argparse
from hashlib import sha256
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
from tempfile import TemporaryDirectory

from PIL import Image

ROOT = Path(__file__).resolve().parent
E2E = "e2e/src/material_command_menu.rs"
SELECTOR = "e2e/src/material.rs"


def hash_file(path):
    return sha256(path.read_bytes()).hexdigest()


def manifest(path):
    result = {}
    for line in path.read_text().splitlines():
        digest, name = line.split("  ", 1)
        assert re.fullmatch(r"[0-9a-f]{64}", digest), line
        assert not PurePosixPath(name).is_absolute() and ".." not in PurePosixPath(name).parts
        assert name not in result, name
        result[name] = digest
    return result


def check_seal(before, after, inputs):
    text = before.read_bytes()
    assert text == after.read_bytes(), (before, after)
    assert set(text.decode().splitlines()) == {f"{name}: OK" for name in inputs}, before
    assert len(text.splitlines()) == len(inputs), before


def changed(left, right):
    return {name for name in left.keys() | right.keys() if left.get(name) != right.get(name)}


def source_tree(directory, expected):
    actual = {str(path.relative_to(directory)): hash_file(path)
              for path in directory.rglob("*") if path.is_file()}
    assert actual == expected, sorted(changed(actual, expected))


def patched(directory, patch, *, reverse=False):
    command = ["patch", "--batch", "-p1"]
    if reverse:
        command.append("-R")
    with patch.open("rb") as data:
        subprocess.run(command, stdin=data, cwd=directory,
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=True)


def decode(path):
    with Image.open(path) as image:
        image.verify()
    with Image.open(path) as image:
        image.load()
        return image.copy()


def browser_proof(final, native_source, build_root=None):
    web = ROOT / "browser"
    assert manifest(web / "source.sha256") == native_source
    assert manifest(web / "runner-stock.sha256") == final
    focused = manifest(web / "focused-source.sha256")
    assert changed(final, focused) == {SELECTOR}
    check_seal(web / "build-source-before.log", web / "build-source-after.log", native_source)
    check_seal(web / "runner-source-before.log", web / "runner-source-after.log", focused)
    runtime = manifest(web / "runtime.sha256")
    web_bundle = manifest(web / "gallery-web.sha256")
    assert len(web_bundle) == 4 and runtime == {**web_bundle, "pins/surface-e2e": runtime["pins/surface-e2e"]}
    red = web / "negative-inputs"
    assert manifest(red / "runner-stock.sha256") == native_source
    red_focus = manifest(red / "focused-source.sha256")
    assert changed(focused, red_focus) == {E2E}
    assert (web / "focused-selector.patch").read_bytes() == (red / "focused-selector.patch").read_bytes()
    previous = manifest(red / "runtime.sha256")
    assert changed(runtime, previous) == {"pins/surface-e2e"}
    if build_root:
        for name, digest in runtime.items():
            assert hash_file(build_root / name) == digest, name
        assert hash_file(build_root / "attempt-1-inputs/pins/surface-e2e") == previous["pins/surface-e2e"]
    observed = web / "pass"
    check_seal(observed / "source-before.log", observed / "source-after.log", focused)
    check_seal(observed / "runtime-before.log", observed / "runtime-after.log", runtime)
    assert (observed / "exit.txt").read_text() == "runner=0 tee=0\n"
    assert json.loads((observed / "headless-observation.json").read_text()) == {
        "samples": 294, "visible_windows": [], "remaining_chrome": [], "exit": 0}
    log = (observed / "runner.log").read_text()
    assert re.findall(r"\[(M\d+)\]", log) == ["M267", "M268", "M269"]
    assert "3 steps, 117 checks" in log and "1 passed; 0 failed; 117 checks" in log
    raw = json.loads((observed / "browser-material-command-menu.json").read_text())
    assert raw["browser"] == "Chrome/158.0.8127.37" and raw["errors"] == []
    assert raw["keyboard"]["disabled_focus"] is True
    assert raw["dismissal"] == {"tab_forward": "menu-demo-list", "tab_backward": "menu-demo-bottom",
                                "outside_action_suppressed": True}
    compact = raw["compact"]
    assert compact["state"]["rtl"] and compact["state"]["text_scale"] == 2
    assert compact["state"]["open"] is False and compact["wrapped_height"] == 120
    assert compact["panel"] == [52, 8, 240, 392] and compact["last"] == [52, 328, 240, 72]
    wide = decode(observed / "material-command-menu-wide.png")
    narrow = decode(observed / "material-command-menu-compact.png")
    assert wide.size == (1920, 1688) and narrow.size == (640, 1048)
    assert list(wide.getpixel((62, 905))) == raw["keyboard"]["center_rgb"]
    assert list(wide.getpixel((56, 536))) == raw["keyboard"]["corner_rgb"]
    assert raw["keyboard"]["center_rgb"] != raw["keyboard"]["corner_rgb"]
    failed = web / "negative"
    check_seal(failed / "source-before.log", failed / "source-after.log", red_focus)
    check_seal(failed / "runtime-before.log", failed / "runtime-after.log", previous)
    assert (failed / "exit.txt").read_text() == "runner=1 tee=0\n"
    negative = json.loads((failed / "browser-material-command-menu.json").read_text())
    assert not negative["errors"] and negative["failure"].startswith("M268:")
    state = negative["state_at_failure"]
    assert state["menu_demo"]["open"] is False and state["focused"] == "menu-demo-opener"
    assert decode(failed / "material-command-menu-failure.png").size == wide.size
    assert (failed / "material-command-menu-wide.png").read_bytes() == (observed / "material-command-menu-wide.png").read_bytes()
    return {"steps": 3, "checks": 117, "decoded_pngs": 2, "windows": 0,
            "window_process_samples": 294, "remaining_chrome": 0, "browser": raw["browser"],
            "early_projection_failure_preserved": True}


def native_proof(expected, build_root=None):
    native = ROOT / "native"
    for name, digest in manifest(native / "evidence.sha256").items():
        assert hash_file(native / name) == digest, name
    root = native / "native-interim"
    check_seal(root / "source-before.log", root / "source-after.log", expected)
    check_seal(native / "build-source-before.log", native / "build-source-after.log", expected)
    pins = manifest(native / "pins/native-binaries.sha256")
    check_seal(root / "pins-before.log", root / "pins-after.log", pins)
    if build_root:
        for name, digest in pins.items():
            assert hash_file(build_root / "pins" / name) == digest, name
    first, second = root / "replay-1", root / "replay-2"
    one = {p.name: hash_file(p) for p in first.iterdir()}
    two = {p.name: hash_file(p) for p in second.iterdir()}
    assert one == two and len(one) == 6
    raw = json.loads((first / "native-menu-report.json").read_text())
    assert raw["transport"] == ["Page.screenshot", "CLI", "MCP"]
    assert [p["geometry"]["side"] for p in raw["profiles"]] == ["below", "above"]
    assert [v for p in raw["profiles"] for v in p["visible_windows"]] == [0] * 8
    for profile in raw["profiles"]:
        for shot in profile["captures"]:
            name = shot["name"]
            assert one[name] == shot["sha256"]
            for directory in (first, second):
                assert list(decode(directory / name).size) == profile["viewport"]
    receipt = json.loads((root / "verified-native.json").read_text())
    assert receipt["remaining_owned_processes"] == [] and receipt["identical_replay_bytes"]
    return {"replays": 2, "decoded_pngs_per_replay": 5, "windows_samples_per_replay": 8,
            "windows": 0, "transports": raw["transport"], "replay_byte_equality": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--divider-source", type=Path)
    parser.add_argument("--menu-source", type=Path)
    parser.add_argument("--web-pins", type=Path)
    parser.add_argument("--native-pins", type=Path)
    args = parser.parse_args()
    divider = manifest(ROOT / "source/divider.sha256")
    final = manifest(ROOT / "source/menu.sha256")
    native_source = manifest(ROOT / "native/source.sha256")
    assert len(divider) == 583 and len(final) == len(native_source) == 590
    assert changed(final, native_source) == {E2E}
    assert set(divider) <= set(final) and len(changed(divider, final)) == 14
    if args.menu_source:
        source_tree(args.menu_source, final)
    if args.divider_source:
        source_tree(args.divider_source, divider)
        with TemporaryDirectory(prefix="menu-reconstruct-", dir="/tmp") as scratch:
            tree = Path(scratch) / "source"
            shutil.copytree(args.divider_source, tree)
            patched(tree, ROOT / "source/divider-to-menu.patch")
            source_tree(tree, final)
            wait_patch = ROOT / "browser/wait-for-projection.patch"
            patched(tree, wait_patch, reverse=True)
            source_tree(tree, native_source)
            patched(tree, wait_patch)
            patched(tree, ROOT / "browser/focused-selector.patch")
            source_tree(tree, manifest(ROOT / "browser/focused-source.sha256"))
            patched(tree, wait_patch, reverse=True)
            source_tree(tree, manifest(ROOT / "browser/negative-inputs/focused-source.sha256"))
    result = {"scope": "local interim, not a release or full stock run",
              "source_files": len(final), "paths_changed_from_divider": len(changed(divider, final)),
              "only_change_from_native_and_web_gallery_build": E2E,
              "reconstructed_divider_to_menu_and_replay_sources": bool(args.divider_source),
              "browser": browser_proof(final, native_source, args.web_pins),
              "native": native_proof(native_source, args.native_pins),
              "checked_original_browser_and_native_binary_pins": bool(args.web_pins and args.native_pins)}
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
