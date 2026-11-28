#!/usr/bin/env python3
"""Collect actual corrected Surface v0.1.75 test receipts without starting apps."""

import argparse
from hashlib import sha256
import json
from pathlib import Path
from runpy import run_path
import sys

from PIL import Image
from evidence_common import (compressed, copy, copy_members, data, digest, import_from, json_file, path_entries,
                             raw_log, require, run_json, seal, tests, verify_entries)
from menu_browser import PNG, REPORTS, WINDOW, run_evidence

HERE = Path(__file__).resolve().parent
STAGE = Path("/tmp/surface-md3-disabled-menu-click.qfrcXZ1r")
REPLAY = Path("/tmp/surface-md3-disabled-menu-corrected-replay.ZIfGJCvi")
TREE = "8046e25a630284411e1ebf9f9d16d1004b02651b"
SOURCE = "8a2037d8b5fb11afcb226f5b27b41656d7c3dd428b09db9c4aaf2d711cacf2d0"
BASE_TREE = "1a8f21971d18d2782882fc8e8e57424bdb9cf301"
BASE = "4ab84a94bbc6923a313a66e6be5f68d43d03834e18d8f962bc8f95413b439b87"
PATCH = "4f1fb9d57083d7f36b7eec8900e657c1db42f182c80bd7351b74444c63a69de4"
TOOLS = "4a991e493e1de0b21e065002cd817eeac8873217ccfa5251c7558c94dbee77ef"
NATIVE_TOOLS = "074b61aa872bf54d11ff9a0a3fa97c33bf5e129774327a9b7ea8304a63fcd0b0"
OBSERVER = "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7"
NATIVE_RUN = "923b7ae661f26b4f27724f83ba87b59c634b7ef01c2257eeb09fae1b520d41ad"
BUILD = "9fce19cb3b2b341cc394e262f6c7a46e603bacd688a740b98484596c7d54f846"
WORKSPACE = "207b84a557e22a8de8909bbf27553d189ec9ca0c3e735f472fa54c29a0a4d3a1"
METAL_PNGS = "c7fb34d394d7702a9e9f6e8c617d552486dfb52bf766fce7bc4c8259440c409f"
METAL_LOGS = {
    "gpu-fixed-primary.log": (16, "679d886df793ad6e2a014c01d7d1166b2f1db5fcbe4700502b6b24c6086580e1"),
    "gpu-fixed-supplemental.log": (2, "ab166f3ea9096a2e927c526da350f30c1eb4349ae08c1e93085049c476e2b9f8"),
}
REGRESSION_LOGS = {
    "red.log": "5e69214f3be4fc1b983c0ae36b509d48ba2e1df6ef5cb2e6ba0cc772c37af002",
    "green.log": "108cd3deb891352ecad88aa974bc0a20be917f038cb33127e0c960a57fb01122",
    "surface-app.log": "70da294ff498e0835dd3ede90cc45e94c59ffaaa45845cf966766da79280d506",
}
REGRESSION = "disabled_click_stays_in_a_separated_menu_and_outside_click_dismisses"
CHANGES = ["crates/surface-app/src/controls.rs", "crates/surface-material/tests/vertical_command_menu.rs"]
BUILD_FILES = {
    "native/SHA256SUMS": "b8324865ffb88b7847f3d2eadab7a64707a5056dffcc68187ce0cee73a503fde",
    "runners/SHA256SUMS": "1b7795b43384c8dde6939c0fefcfec72a04e184014b220d88b8acb2205ccdfae",
    "web/SHA256SUMS": "06e9122eee2e6633e71a3e65a651c9325c726d61c424fdcf16f888feae31a464",
}
NATIVE_RECEIPT = "native-vertical-menu-report.json"
BROWSER_RECEIPT = "verified-stock-material.json"


def require_tree(path):
    require(path.read_text() == TREE + "\n", f"Wrong corrected tree marker at {path.name}")


def verifier(tools, name):
    with import_from(tools):
        return run_path(str(tools / name))


def source_info(tools):
    seal(tools, "SHA256SUMS", TOOLS)
    seal(tools / "native-tools", "SHA256SUMS", NATIVE_TOOLS)
    require_tree(tools / "source.tree")
    source, base = tools / "source.sha256", tools / "base.sha256"
    require(digest(source) == SOURCE and digest(base) == BASE and digest(tools / "disabled-menu.patch") == PATCH,
            "Corrected source or patch pin differs")
    new, old = path_entries(source), path_entries(base)
    changes = sorted(name for name in new.keys() | old.keys() if new.get(name) != old.get(name))
    require(len(new) == len(old) == 698 and changes == CHANGES, "Corrected two-path delta differs")
    proof = {"tree": TREE, "source_sha256": SOURCE, "patch_sha256": PATCH, "patch_paths": changes,
             "file_count": 698, "original_source_sha256": BASE, "original_tree": BASE_TREE}
    require(data(tools / "source-receipt.json") == proof, "Corrected source receipt differs")
    return proof


def log_at(directory, name):
    path = directory / name
    return path if path.is_file() else directory / (name + ".gz")


def log_sha(path):
    return sha256(raw_log(path)).hexdigest()


def workspace(folder):
    path = log_at(folder, "check.log")
    text, totals = tests(path)
    wasm = text.rsplit("\ntest result:", 1)[-1].count("Finished `dev` profile")
    require((totals["passed"], totals["ignored"], totals["log_sha256"], wasm) == (1607, 20, WORKSPACE, 3)
            and "provenance:" in text, "Missing the exact corrected workspace results and three Wasm checks")
    result = {"source_tree": TREE, "source_sha256": SOURCE,
              "command": ["sh", "scripts/check.sh"], "wasm_checks": wasm, **totals}
    require(data(folder / "verified-workspace.json") == result, "Corrected workspace terminal receipt differs")
    return result


def metal(folder, log_dir, marker):
    require_tree(marker)
    require(digest(folder / "SHA256SUMS") == METAL_PNGS, "Corrected Metal PNG manifest differs")
    files = path_entries(folder / "SHA256SUMS", allow_dot_prefix=True)
    require(len(files) == 21 and set(files) == {path.name for path in folder.glob("*.png")}
            and "gpu-offscreen.png" in files, "Expected 21 original corrected Metal PNGs")
    verify_entries(folder, files)
    for name in files:
        with Image.open(folder / name) as image:
            require(image.format == "PNG", f"Expected PNG: {name}")
            image.load()
    results = {}
    for name, (expected, sha) in METAL_LOGS.items():
        text, result = tests(log_at(log_dir, name))
        require((result["passed"], result["ignored"], result["log_sha256"]) == (expected, 0, sha)
                and "backend: Metal" in text, "Expected the exact corrected test results on Metal")
        results[name] = result
    return {"source_tree": TREE, "backend": "Metal", "passed": 18, "logs": results, "pngs": files}


def regression(folder):
    logs = {name: log_at(folder, name) for name in REGRESSION_LOGS}
    require({name: log_sha(path) for name, path in logs.items()} == REGRESSION_LOGS,
            "Original red/green and Runtime logs differ")
    red, green = (raw_log(logs[name]).decode() for name in ("red.log", "green.log"))
    require(f"test {REGRESSION} ... FAILED" in red and "a disabled row is inside the popup" in red
            and f"test {REGRESSION} ... ok" in green
            and tests(logs["green.log"])[1]["passed"] == 8 and tests(logs["surface-app.log"])[1]["passed"] == 372,
            "Original disabled pointer regression or neighboring results differ")
    return {"case": REGRESSION, "corrected_menu_tests": 8, "corrected_surface_app_tests": 372, "log_sha256": REGRESSION_LOGS}


def build_info(out, tools, *, live=False):
    receipt = out / "verified-build-inputs.json"
    require(digest(receipt) == BUILD, "The exact corrected build receipt is required")
    actual = data(receipt)
    require(actual["source"] == {"version": "0.1.75", "tree": TREE, "source_sha256": SOURCE,
                                 "source_file_count": 698, "patch_sha256": PATCH}
            and actual["artifact_manifests"] == BUILD_FILES
            and {name: digest(out / name) for name in BUILD_FILES} == BUILD_FILES
            and {name: log_sha(log_at(out, name)) for name in actual["logs"]} == actual["logs"],
            "Corrected build metadata, manifest or log differs")
    if live:
        pins = verifier(tools, "pins.py")
        require(pins["build_record"](out, pins["check_source"](tools)) == actual,
                "Actual corrected build artifacts or source bytes differ")
    dist_root = out / ("web/material-gallery" if live else "web")
    require(digest(dist_root / "surface-web.json") == path_entries(out / "web/SHA256SUMS")["surface-web.json"],
            "Wasm distribution producer metadata differs")
    return actual


def native_info(run, tools, build):
    require(digest(run / "SHA256SUMS") == NATIVE_RUN, "Expected the actual corrected native terminal receipt")
    binary = build / "native/SHA256SUMS"
    summary = run_json([tools / "native-tools/replay-vertical-menu.py", "--readback", run,
                        "--source-sha", SOURCE, "--binary-sha", digest(binary), "--evidence-sha", NATIVE_RUN])
    report = data(run / NATIVE_RECEIPT)
    require(summary["source_tree"] == TREE and report["source"]["file_count"] == 698
            and report["binaries"]["files"] == path_entries(binary) and summary["repeats"] == 2
            and (summary["original_pngs"], summary["window_samples"], summary["reaped"], summary["visible_windows"])
            == (8, 46, 6, 0), "Native screenshot, transport or clean process receipts differ")
    return summary


def browser_info(run, tools, build, *, live=False):
    invocation, observation = data(run / "invocation.json"), data(run / WINDOW)
    require(invocation["inputs"] == data(run / "inputs-after.json") == {
        "source_tree": TREE, "source_sha256": SOURCE, "observer_sha256": OBSERVER,
        "runner_sha256": path_entries(build / "runners/SHA256SUMS")["surface-e2e"],
        "web_manifest_sha256": BUILD_FILES["web/SHA256SUMS"],
    }, "Chrome receipt did not use the exact corrected source, runner and Web assets")
    command = invocation["argv"]
    require(len(command) == 10 and command[1] == "--chrome"
            and command[3:6] == ["--chrome-arg", "--disable-updater-scheduler", "--web-root"]
            and command[7] == "--artifacts" and command[9] == "material", "Wrong stock Material invocation")
    if live:
        require(tuple(Path(command[i]).resolve() for i in (0, 2, 6, 8))
                == (build / "runners/surface-e2e", run / "guards/chrome", build / "web", run / "artifacts"),
                "Original receipt points at a different runner, guard, Web root or artifacts directory")
    guard = verifier(tools, "run-stock-material.py")
    summary = guard["verify_run"](run, observation)
    require(summary == data(run / BROWSER_RECEIPT), "Original corrected Chrome terminal receipt differs")
    measured = run_evidence(run, summary)
    return {**measured, "full_material_steps": summary["steps"], "checks": summary["checks"],
            "runner_sha256": invocation["inputs"]["runner_sha256"],
            "web_manifest_sha256": invocation["inputs"]["web_manifest_sha256"]}


def readiness(build, native, browser):
    paths = [(build / "verified-build-inputs.json", "corrected build pins"),
             (native / "SHA256SUMS", "corrected native terminal receipt"),
             (browser / BROWSER_RECEIPT, "corrected stock Chrome terminal receipt")]
    return [name for marker, name in paths if not marker.is_file()]


def preflight(build, nrun, brun):
    state = {"status": "preflight", "source": source_info(REPLAY), "regression": regression(STAGE),
             "workspace": workspace(STAGE / "verified-workspace"),
             "metal": metal(STAGE / "gpu-fixed", STAGE, STAGE / "gpu-fixed-passed"),
             "pending": readiness(build, nrun, brun)}
    if "corrected build pins" not in state["pending"]:
        state["build"] = build_info(build, REPLAY, live=True)
        if "corrected native terminal receipt" not in state["pending"]:
            state["native"] = native_info(nrun, REPLAY, build)
        if "corrected stock Chrome terminal receipt" not in state["pending"]:
            state["browser"] = browser_info(brun, REPLAY, build, live=True)
    return state


def archive_report(root):
    tools, build = root / "replay", root / "build"
    return {"status": "verified corrected local runtime", "source": source_info(tools),
            "regression": regression(root / "logs"), "workspace": workspace(root / "workspace"),
            "metal": metal(root / "metal", root / "logs", root / "metal/tree"),
            "build": build_info(build, tools),
            "native": native_info(root / "native", tools, build),
            "browser": browser_info(root / "browser", tools, build)}


def collect(build, nrun, brun, out):
    original = preflight(build, nrun, brun)
    require(not original["pending"], "Corrected evidence is awaiting " + ", ".join(original["pending"]))
    temp = Path("/tmp").resolve()
    require(out != temp and out.is_relative_to(temp) and all(not out.is_relative_to(path.resolve())
                for path in (STAGE, REPLAY, nrun, brun)), "Choose a fresh independent output directory under /tmp")
    out.mkdir(mode=0o700)
    copy_members(REPLAY, out / "replay", ("SHA256SUMS", *path_entries(REPLAY / "SHA256SUMS")))
    native_tools = REPLAY / "native-tools"
    copy_members(native_tools, out / "replay/native-tools", path_entries(native_tools / "SHA256SUMS"))
    copy_members(build, out / "build", ("verified-build-inputs.json", "toolchain.txt", *BUILD_FILES))
    copy(build / "web/material-gallery/surface-web.json", out / "build/web/surface-web.json")
    for name in original["build"]["logs"].keys() - {"toolchain.txt"}:
        compressed(build / name, out / "build" / (name + ".gz"))
    copy(STAGE / "verified-workspace/verified-workspace.json", out / "workspace/verified-workspace.json")
    compressed(STAGE / "verified-workspace/check.log", out / "workspace/check.log.gz")
    copy_members(STAGE / "gpu-fixed", out / "metal", ("SHA256SUMS", *original["metal"]["pngs"]))
    copy(STAGE / "gpu-fixed-passed", out / "metal/tree")
    for name in (*METAL_LOGS, *REGRESSION_LOGS):
        compressed(STAGE / name, out / "logs" / (name + ".gz"))
    copy_members(nrun, out / "native", ("SHA256SUMS", *path_entries(nrun / "SHA256SUMS")))
    ledger = tuple(path.relative_to(brun).as_posix() for path in (brun / "guard-ledger").glob("*.json"))
    copy_members(brun, out / "browser", ("runner.log", "invocation.json", "inputs-after.json",
                                          WINDOW, BROWSER_RECEIPT, *ledger))
    copy_members(brun / "artifacts", out / "browser/artifacts", (*PNG, *REPORTS))
    copy_members(HERE, out, ("collect.py", "evidence_common.py", "menu_browser.py", "README.md"))
    result = archive_report(out)
    require(all(original[k] == value for k, value in result.items() if k != "status"),
            "Portable corrected readback differs from original receipts")
    json_file(out / "verified.json", result)
    manifest = out / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(out).as_posix()}\n"
                                for path in sorted(out.rglob("*")) if path.is_file()))
    return {"archive": str(out), "manifest_sha256": digest(manifest), "files": len(path_entries(manifest)),
            "browser_steps": result["browser"]["full_material_steps"], "checks": result["browser"]["checks"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--preflight", action="store_true")
    mode.add_argument("--collect", type=Path)
    mode.add_argument("--verify", type=Path)
    parser.add_argument("--sha")
    parser.add_argument("--build", type=Path, default=REPLAY / "outputs")
    parser.add_argument("--native", type=Path, default=REPLAY / "guarded-native")
    parser.add_argument("--browser", type=Path, default=REPLAY / "guarded-stock-material")
    args = parser.parse_args()
    if args.verify:
        root = args.verify.resolve(strict=True)
        seal(root, "SHA256SUMS", args.sha, exact=True)
        result = archive_report(root)
        require(result == data(root / "verified.json"), "Archived corrected result differs from independent readback")
    elif args.preflight:
        result = preflight(args.build.resolve(), args.native.resolve(), args.browser.resolve())
    else:
        result = collect(args.build.resolve(), args.native.resolve(), args.browser.resolve(), args.collect.resolve())
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as exc:
        sys.exit(f"Corrected q75 collector stopped. {exc}")
