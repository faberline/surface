#!/usr/bin/env python3
"""Collect or recheck the corrected Surface v0.1.76 evidence without running the app."""

import argparse
import r5
from hashlib import sha256
import json
from pathlib import Path
import re
from runpy import run_path
import sys
from tempfile import TemporaryDirectory

from PIL import Image

from evidence_common import (
    compressed, copy_members, data, digest, import_from, json_file,
    path_entries, raw_log, require, run_json, seal, tests, verify_entries,
)

HERE = Path(__file__).resolve().parent
SOURCE = Path("/tmp/surface-md3-choice-menu-v0176-corrected.2R544r7F")
TOOLS = Path("/tmp/surface-md3-choice-menu-v0176-corrected-replay.Q1k95t8V")
TREE = "37e4454bc7d3f63f7111c8264d4e59e451c8ef9e"
SOURCE_SHA = "a9aa38ee7ab64746471491ccfd6cd7df0132e8f30254d536a6fdd3fc95e10eb8"
BASE = "b3e62f07a03b08f1741de8695fe845d3b771f11d"
BASE_SHA = "b1b3dea6427bb8bdd5debdd2715d3956e8ac4549a03a70880aa7fe12aabc700f"
TOOLS_SHA = "53b47b5d334e051e94b2f43220262a5fe261548edd7902c9c17ad31c67d44a3b"
NATIVE_TOOLS_SHA = "e00371100ceb78317eff49949a5bb0038aa35d59f565de6329ae214cb79651dd"
PATCH_SET_SHA = "54696fc9387a2248948771c2682a98cc7d40a742de9c4098d0846a6dee2a0c18"
CHANGED_SHA = "25db5df536e81b926bbba66180effd37ac0b2c90b770ac6b8f3af4b4af01492d"
R3_NATIVE = "bc9b9cf64478abed3d5d0b282fe87cd0930e5ceba8b9c2de12d4aa7fd7d9a1d4"
R3_WEB = "9c9dd0be666f8d6b2bd5c363167bc5be11380b45fd2e8c221e6f4d2f2e9e23c0"
WORKSPACE = "workspace-fixed.log"
WORKSPACE_SHA = "e36cd7c245ec666d581c56d9794c32c8b5428d54adb11f29f38989c54d52fc8f"
GPU_SHA = "c7fb34d394d7702a9e9f6e8c617d552486dfb52bf766fce7bc4c8259440c409f"
GPU_LOGS = {
    "gpu-fixed-primary.log": (16, "c887f287809070513fe0e47cff3b59ab88377b2bf9fd105492603b104902e3ea"),
    "gpu-fixed-supplemental.log": (2, "9084b55fa73f68bd19237a2ee4b36bbc38252890cce86db2c29f36d6465bc5f7"),
}
BUILD_MANIFESTS = ("native/SHA256SUMS", "runners/SHA256SUMS", "web/SHA256SUMS")
BUILD_LOGS = ("native-build.log", "runner-build.log", "web-build.log")
BUILD_RECEIPT = "verified-build-inputs.json"
NATIVE_RECEIPT = "native-vertical-choice-menu-report.json"
R4_NATIVE_RESULT = "fee03d7712c9d80298959980c38c1b4b63b6f1ed6420cba557ce7e18d27fd10e"
NATIVE_TRANSPORTS = ["Page.screenshot", "surface-devtools page screenshot", "MCP page_screenshot"]
ORIGINAL = "https://github.com/faberline/surface/tree/6ad148ea4b6419d5d011873bcb45a720f2f88c34/docs/evidence/md3/vertical-choice-menu-v0176-recovered-source"


def source_info(tools):
    seal(tools, "SHA256SUMS", TOOLS_SHA)
    seal(tools / "native-tools", "SHA256SUMS", NATIVE_TOOLS_SHA)
    require((tools / "source.tree").read_text() == TREE + "\n"
            and digest(tools / "source.sha256") == SOURCE_SHA
            and digest(tools / "base.sha256") == BASE_SHA
            and digest(tools / "patches.sha256") == PATCH_SET_SHA
            and digest(tools / "changed-paths.txt") == CHANGED_SHA, "Wrong corrected source recipe")
    before, after = (path_entries(tools / name) for name in ("base.sha256", "source.sha256"))
    changed = sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))
    patches = {}
    touched = set()
    for row in (tools / "patches.sha256").read_text().splitlines():
        checksum, original = row.split("  ", 1)
        patch = tools / "patches" / Path(original).name
        require(digest(patch) == checksum and patch.name not in patches, f"Wrong patch {patch.name}")
        patches[patch.name] = checksum
        touched.update(re.findall(r"^\+\+\+ b/(\S+)$", patch.read_text(), re.M))
    require(len(before) == 702 and len(after) == 703 and len(patches) == 3 and len(changed) == 7
            and changed == (tools / "changed-paths.txt").read_text().splitlines()
            and set(changed) == touched, "Wrong corrected source inventory or patch set")
    return {"version": "0.1.76", "tree": TREE, "source_sha256": SOURCE_SHA, "source_file_count": 703,
            "base_tree": BASE, "base_source_sha256": BASE_SHA, "patch_set_sha256": PATCH_SET_SHA,
            "patches": patches, "changed_paths": changed}


def saved_log(root, name):
    path = root / name
    return path if path.is_file() else root / (name + ".gz")


def decode_png(path):
    with Image.open(path) as image:
        require(image.format == "PNG" and min(image.size) > 0, f"Invalid PNG: {path.name}")
        image.load()
        return list(image.size)


def rust_info(root):
    require((root / "gpu-fixed-passed").read_text() == TREE + "\n"
            and (root / "full-fixed-workspace-passed").read_text().split()
                == [SOURCE_SHA, "/tmp/surface-md3-choice-menu-v0176-corrected.2R544r7F/source.sha256"],
            "Expected original corrected Rust run markers")
    log, workspace = tests(saved_log(root, WORKSPACE))
    wasm_completions = log.rsplit("\ntest result:", 1)[-1].count("Finished `dev` profile")
    require(workspace["passed"] == 1615 and workspace["log_sha256"] == WORKSPACE_SHA
            and "provenance:" in log and wasm_completions == 3
            and "test disabled_click_stays_in_a_separated_menu_and_outside_click_dismisses ... ok" in log,
            "Expected exact corrected workspace, Wasm and menu regression proof")
    readback = {}
    for name, (passed, checksum) in GPU_LOGS.items():
        log, result = tests(saved_log(root, name))
        require(result["passed"] == passed and result["ignored"] == 0 and result["log_sha256"] == checksum
                and "backend: Metal" in log and "Apple M1 Max" in log, "Expected original Metal gate")
        readback[name] = result
    images = root / "gpu-fixed"
    require(digest(images / "SHA256SUMS") == GPU_SHA, "Wrong original Metal manifest")
    pngs = path_entries(images / "SHA256SUMS", allow_dot_prefix=True)
    verify_entries(images, pngs)
    actual = {path.name for path in images.iterdir()}
    require(len(pngs) == 21 and {name for name in actual if name.endswith(".png")} == pngs.keys(),
            "Wrong Metal PNG inventory")
    for name in pngs:
        decode_png(images / name)
    return {"workspace": {"wasm_checks": wasm_completions, **workspace},
            "offscreen": {"backend": "Metal", "adapter": "Apple M1 Max", "tests": 18,
                          "original_pngs": len(pngs), "runs": readback}}


def build_info(root, source, *, builder=None):
    result = data(root / BUILD_RECEIPT)
    require(result["source"] == source, "Build receipt has the wrong corrected source")
    pins = {name: digest(root / name) for name in BUILD_MANIFESTS}
    require(result["artifact_manifests"] == pins
            and pins[BUILD_MANIFESTS[0]] != R3_NATIVE and pins[BUILD_MANIFESTS[2]] != R3_WEB,
            "Build pins differ or reuse R3 native or Web producers")
    require(result["logs"] == {**{
                name: sha256(raw_log(saved_log(root, name))).hexdigest() for name in BUILD_LOGS},
                "toolchain.txt": digest(root / "toolchain.txt")}, "Wrong build logs or toolchain")
    native, runners, web = (path_entries(root / name) for name in BUILD_MANIFESTS)
    require(native.keys() == {"surface-material-gallery", "surface-devtools"}
            and runners.keys() == {"surface-e2e"}
            and len(web) == 4 and {"index.html", "surface-web.json"} <= web.keys()
            and any(name.endswith(".wasm") for name in web)
            and any(name.endswith(".js") for name in web), "Wrong producer manifest inventory")
    if builder is not None:
        require(builder["build_record"](root, source) == result, "Live producer bytes differ")
    return result


def native_info(root, build, tools):
    captured = data(root / NATIVE_RECEIPT)
    binaries = build / BUILD_MANIFESTS[0]
    require(captured["source"] == {"version": "0.1.76", "manifest_sha256": SOURCE_SHA,
                                   "tree": TREE, "file_count": 703}
            and captured["binaries"]["files"] == path_entries(binaries), "Wrong corrected native producer")
    require(digest(root / "SHA256SUMS") == R4_NATIVE_RESULT, "Wrong exact R4 native manifest")
    result = run_json([tools / "native-tools/replay-vertical-choice-menu.py", "--readback", root,
                       "--evidence-sha", R4_NATIVE_RESULT, "--source-sha", SOURCE_SHA,
                       "--binary-sha", digest(binaries)])
    require(result["source_tree"] == TREE and result["original_pngs"] == 8 and result["repeats"] == 2
            and result["reaped"] == 6 and result["visible_windows"] == 0
            and result["window_samples"] == 46 and result["transports"] == NATIVE_TRANSPORTS,
            "Wrong corrected native runtime proof")
    return result


def runtime_report(root):
    tools, build = root / "tools", root / "build"
    source = source_info(tools)
    r5_source = r5.source_info(root / "r5", tools / "source.sha256")
    pins = r5.input_pins(root / "r5", r5_source, build, source)
    return {
        "status": "verified R4 native product and R5 test-only Chrome replay",
        "prior_r3_source": ORIGINAL,
        "source": source,
        "build": build_info(build, source),
        "rust": rust_info(root / "rust"),
        "native": native_info(root / "native", build, tools),
        "r4_browser_failure": r5.failure_info(root / "r4-browser-failure", build, source),
        "r5_source": r5_source,
        "r5_browser_inputs": pins,
        "browser": r5.browser_info(root / "browser", build, root / "r5-tools", pins),
    }


def preflight(build, native, browser):
    source = source_info(TOOLS)
    with import_from(TOOLS):
        builder = run_path(str(TOOLS / "pins.py"))
    require(builder["check_source"](TOOLS) == source, "Live corrected R4 source differs from its recipe")
    build_result = build_info(build, source, builder=builder)
    r5_source = r5.source_info(r5.R5, TOOLS / "source.sha256", original=True)
    pins = r5.input_pins(r5.R5, r5_source, build, source, original=True)
    failed = r5.failure_info(TOOLS / "guarded-stock-material", build, source, original=True)
    result = {
        "status": "preflight",
        "source": source,
        "build": build_result,
        "rust": rust_info(SOURCE),
        "native": native_info(native, build, TOOLS),
        "r4_browser_failure": failed,
        "r5_source": r5_source,
        "r5_browser_inputs": pins,
        "pending": [],
    }
    if (browser / "invocation.json").is_file():
        require(data(browser / "invocation.json")["inputs"] == pins,
                "Browser invocation has wrong R5 inputs")
        result["r5_invocation_verified"] = True
    r5.guard_members(r5.R5_TOOLS)
    if not (browser / r5.TERMINAL).is_file():
        if (browser / "inputs-after.json").is_file() and (browser / r5.WINDOW).is_file():
            window = data(browser / r5.WINDOW)
            require(window["stock_exit"] == 0 and not window["problems"],
                    "R5 runtime failed before a terminal acceptance receipt")
        result["pending"].append("exact R5 stock browser terminal receipt")
    else:
        result["browser"] = r5.browser_info(browser, build, r5.R5_TOOLS, pins, original=True)
    return result


def collect(out, build, native, browser):
    checked = preflight(build, native, browser)
    require(not checked["pending"], "Archive refused. Awaiting " + ", ".join(checked["pending"]))
    temp = Path("/tmp").resolve()
    require(out.is_relative_to(temp) and out != temp and not out.exists()
            and all(not out.is_relative_to(root.resolve()) for root in (SOURCE, TOOLS, r5.R5, r5.R5_TOOLS,
                                                                        build, native, browser)),
            "Use a fresh independent archive under /tmp")

    with TemporaryDirectory(prefix="surface-v0176-r5-evidence-stage-", dir=out.parent) as scratch:
        archive = Path(scratch) / "runtime"
        copy_members(HERE, archive, ("collect.py", "r5.py", "evidence_common.py", "README.md"))
        copy_members(TOOLS, archive / "tools", ("SHA256SUMS", *path_entries(TOOLS / "SHA256SUMS")))
        copy_members(TOOLS / "native-tools", archive / "tools/native-tools",
                     path_entries(TOOLS / "native-tools/SHA256SUMS"))
        copy_members(build, archive / "build", (*BUILD_MANIFESTS, "toolchain.txt", BUILD_RECEIPT))
        for name in BUILD_LOGS:
            compressed(build / name, archive / "build" / (name + ".gz"))
        copy_members(SOURCE, archive / "rust", ("full-fixed-workspace-passed", "gpu-fixed-passed"))
        for name in (WORKSPACE, *GPU_LOGS):
            compressed(SOURCE / name, archive / "rust" / (name + ".gz"))
        gpu = SOURCE / "gpu-fixed"
        copy_members(gpu, archive / "rust/gpu-fixed", ("SHA256SUMS", *path_entries(gpu / "SHA256SUMS",
                                                                                   allow_dot_prefix=True)))
        copy_members(native, archive / "native", ("SHA256SUMS", *path_entries(native / "SHA256SUMS")))
        r5.stage_failure(TOOLS / "guarded-stock-material", archive / "r4-browser-failure")
        r5.stage_source(r5.R5, SOURCE / "source", archive / "r5")
        copy_members(r5.R5_TOOLS, archive / "r5-tools", ("SHA256SUMS", *r5.guard_members(r5.R5_TOOLS)))
        ledger = tuple(path.relative_to(browser).as_posix() for path in (browser / "guard-ledger").glob("*.json"))
        copy_members(browser, archive / "browser", ("runner.log", "invocation.json", "inputs-after.json",
                                                     r5.WINDOW, r5.TERMINAL, *ledger))
        copy_members(browser / "artifacts", archive / "browser/artifacts", checked["browser"]["menus"])

        portable = runtime_report(archive)
        sections = ("source", "build", "rust", "native", "r4_browser_failure",
                    "r5_source", "r5_browser_inputs", "browser")
        require(all(portable[key] == checked[key] for key in sections),
                "Archived readback differs from original R4 or R5 evidence")
        json_file(archive / "verified.json", portable)
        manifest = archive / "SHA256SUMS"
        manifest.write_text("".join(f"{digest(path)}  {path.relative_to(archive).as_posix()}\n"
                                    for path in sorted(archive.rglob("*")) if path.is_file()))
        checksum = digest(manifest)
        files = len(seal(archive, "SHA256SUMS", checksum, exact=True))
        archive.rename(out)
    return {"archive": str(out), "manifest_sha256": checksum, "files": files,
            "browser_steps": portable["browser"]["steps"], "browser_checks": portable["browser"]["checks"],
            "native_pngs": portable["native"]["original_pngs"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--preflight", action="store_true")
    mode.add_argument("--collect", type=Path)
    mode.add_argument("--verify", type=Path)
    parser.add_argument("--sha")
    parser.add_argument("--build", type=Path, default=TOOLS / "outputs")
    parser.add_argument("--native", type=Path, default=TOOLS / "guarded-native")
    parser.add_argument("--browser", type=Path, default=r5.R5 / "guarded-stock-material")
    args = parser.parse_args()
    if args.verify:
        root = args.verify.resolve(strict=True)
        seal(root, "SHA256SUMS", args.sha, exact=True)
        result = runtime_report(root)
        require(result == data(root / "verified.json"), "Recomputed corrected summary differs")
    else:
        build, native, browser = (path.resolve() for path in (args.build, args.native, args.browser))
        result = (preflight(build, native, browser) if args.preflight
                  else collect(args.collect.resolve(), build, native, browser))
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit(f"Corrected Surface v0.1.76 evidence stopped. {error}")
