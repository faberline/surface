"""Collect and recheck the exact recovered v0.1.75 macOS gates. Starts no application."""
import argparse
from hashlib import sha256
import json
from pathlib import Path
import re
from runpy import run_path
import subprocess
import sys

from PIL import Image
from evidence_common import (compressed, copy, copy_members, data, digest, json_file, path_entries,
                             raw_log,
                             require, seal, source_tree, tests, verify_entries)
from menu_browser import PNG, REPORTS, WINDOW, run_evidence

HERE = Path(__file__).resolve().parent
DEFAULT = Path("/tmp/surface-md3-menu-v0175-recovered.uSvgC4AK")
VALIDATION = Path("/tmp/surface-md3-menu-validation-recovery.afLERMJj")
TREE = "1a8f21971d18d2782882fc8e8e57424bdb9cf301"
SOURCE = "4ab84a94bbc6923a313a66e6be5f68d43d03834e18d8f962bc8f95413b439b87"
BASE = "7aedeb9161baadca46aa6fdd83848b7e9c5498bccfac0e9a0e21e2782ec3f0bd"
BASE_TREE = "f7a92f77ecc9b37c91d81ace4877cd8038044d5c"
PATCH = "4783224d6fcc7fbcc952471d21e75d64a4083f019109c4e9500dd15d4de0104c"
RECOVERY_COMMIT = "4b7a41753fc00b996b3a1b70885e272876cb77ca"
NATIVE_TOOLS = "6adbb41f840b8d26a6c759d2cf327bca21c6f270dfa6311ed3ec49870c86a552"
GUARD = "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7"
WATCHER = "e7afbd343f966ec4d3a3d62b27ce707ede03d613df1dc26046cc7b6a83082c33"
WORKSPACE = "workspace-v0175-recovered.log"
PRODUCER = "build-v0175-recovered.log"
RUNNER_LOG = "stock-runner-build.log"
RUNNER_PIN = "stock-runner.sha256"
METAL = ("gpu-primary.log", "gpu-supplemental.log")
SOURCE_MARKERS = ("workspace-v0175-recovered-passed", "gpu-v0175-recovered-passed")
BUILD_MARKERS = ("web-built.tree", "native-built.tree", "stock-runner-built.tree")
BROWSER_RECEIPT = "verified-stock-material.json"
PATCH_NAME = "vertical-menu-v0175.patch"
NATIVE_REPORT = "native-vertical-menu-report.json"


def source_proof(base, exact):
    manifest = exact / "source.sha256"
    patch = exact / PATCH_NAME
    require(digest(base) == BASE and digest(manifest) == SOURCE and digest(patch) == PATCH
            and (exact / "source.tree").read_text() == TREE + "\n", "Recovered identity differs")
    before, after = path_entries(base), path_entries(manifest)
    changed = sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))
    require(len(before) == 696 and len(after) == 698 and len(changed) == 17
            and changed == sorted(re.findall(r"^diff --git a/(\S+) b/\S+$", patch.read_text(), re.M)),
            "Recovered exact patch scope differs")
    return {"base_tree": BASE_TREE, "base_manifest_sha256": BASE, "tree": TREE, "source_manifest_sha256": SOURCE,
            "files": 698, "changed_paths": changed, "patch_sha256": PATCH, "source_recovery_commit": RECOVERY_COMMIT}


def workspace(path):
    log, result = tests(path)
    require((result["passed"], result["ignored"]) == (1606, 20) and "provenance:" in log
            and log.rsplit("\ntest result:", 1)[-1].count(f"Finished {chr(96)}dev{chr(96)} profile") == 3,
            "Workspace did not record all expected tests, provenance and three Wasm checks")
    return {**result, "wasm_checks": 3}


def metal(folder, log_dir, ext=""):
    files = path_entries(folder / "SHA256SUMS", allow_dot_prefix=True)
    require(len(files) == 21 and set(files) == {p.name for p in folder.glob("*.png")} and "gpu-offscreen.png" in files,
            "Expected the 21 original Metal PNGs")
    verify_entries(folder, files)
    for name in files:
        with Image.open(folder / name) as image:
            require(image.format == "PNG", "Unexpected Metal screenshot format")
            image.load()
    run = {}
    for name, n in zip(METAL, (16, 2), strict=True):
        log, result = tests(log_dir / (name + ext))
        require(result["passed"] == n and result["ignored"] == 0 and "backend: Metal" in log,
                "Expected passing ignored tests on real Metal")
        run[name] = result
    return {"backend": "Metal", "passed": 18, "logs": run, "pngs": files}


def bind_native(report, tools, manifest):
    seal(tools, "SHA256SUMS", NATIVE_TOOLS)
    binary_sha = digest(manifest)
    proc = subprocess.run([sys.executable, "-E", "-B", str(tools / "replay-vertical-menu.py"), "--readback",
                           str(report.resolve(strict=True)), "--source-sha", SOURCE, "--binary-sha", binary_sha,
                           "--evidence-sha", digest(report / "SHA256SUMS")], text=True, capture_output=True)
    require(proc.returncode == 0, "Native offline readback failed: " + proc.stderr.strip())
    result = json.loads(proc.stdout)
    raw = data(report / NATIVE_REPORT)
    require(raw["source"]["file_count"] == 698 and raw["binaries"]["files"] == path_entries(manifest)
            and result["repeats"] == 2 and result["original_pngs"] == 8 and result["window_samples"] == 46
            and result["reaped"] == 6 and result["visible_windows"] == 0, "Native receipts do not bind exact inputs")
    return result


def watch_tools(tools):
    require(digest(tools / "run-stock-material.py") == WATCHER
            and digest(tools / "observe-focused-original.py") == GUARD, "Unreviewed browser observer")
    return run_path(str(tools / "run-stock-material.py"))


def runner_pin(path):
    text = path.read_text()
    match = re.fullmatch(r"([a-f0-9]{64})  (/[^\n]+/surface-e2e)\n", text)
    require(match is not None, "Missing exact stock runner executable pin")
    return match.groups()


def bound_browser(run, web_manifest, stock_pin, tools, original=False):
    call = data(run / "invocation.json")
    pins = call["inputs"]
    runner_sha, runner_path = runner_pin(stock_pin)
    require(pins == data(run / "inputs-after.json") and pins["source_tree"] == TREE and pins["source_sha256"] == SOURCE
            and pins["runner_sha256"] == runner_sha
            and pins["web_manifest_sha256"] == digest(web_manifest) and pins["observer_sha256"] == GUARD,
            "Browser receipt does not bind exact recovered source and Web build")
    argv = call["argv"]
    require(len(argv) == 10 and argv[1] == "--chrome" and argv[3:6] == ["--chrome-arg", "--disable-updater-scheduler", "--web-root"]
            and argv[7] == "--artifacts" and argv[9] == "material", "Wrong original stock command")
    if original:
        require(Path(argv[8]).resolve() == (run / "artifacts").resolve() and digest(Path(argv[0])) == pins["runner_sha256"],
                "Actual browser runner or artifact directory differs")
        require(Path(argv[0]).resolve() == Path(runner_path).resolve(), "Original command used a different stock runner")
        verify_entries(Path(argv[6]).resolve() / "material-gallery", path_entries(web_manifest, allow_dot_prefix=True))
    observed = watch_tools(tools)["verify_run"](run, data(run / WINDOW))
    require(observed == data(run / BROWSER_RECEIPT), "Original stock observer readback differs")
    independent = run_evidence(run, observed)
    return {**independent, "full_material_steps": observed["steps"], "checks": observed["checks"],
            "runner_sha256": pins["runner_sha256"], "web_manifest_sha256": pins["web_manifest_sha256"]}


def preflight(stage, browser, native):
    exact = stage / "exact-candidate"
    source = source_proof(stage / "base.sha256", exact)
    manifest = path_entries(exact / "source.sha256")
    require(source_tree(exact / "source", manifest) == TREE and set(manifest) == {
        path.relative_to(exact / "source").as_posix() for path in (exact / "source").rglob("*") if path.is_file()
    }, "Fresh recovered source bytes or inventory differ")
    seal(VALIDATION / "q75-native", "SHA256SUMS", NATIVE_TOOLS)
    watch_tools(VALIDATION / "browser")
    expected = {stage / name: TREE for name in SOURCE_MARKERS} | {exact / name: TREE for name in BUILD_MARKERS}
    for path, value in expected.items():
        if path.is_file():
            require(path.read_text() == value + "\n", f"Gate marker has the wrong tree: {path.name}")
    pending = [path.name for path in expected if not path.is_file()]
    if not (browser / BROWSER_RECEIPT).is_file():
        pending.append("guarded stock Material")
    if native is None or not (native / "SHA256SUMS").is_file():
        pending.append("native transport replay")
    result = {"status": "preflight", "source": source, "pending": pending}
    if (stage / SOURCE_MARKERS[0]).is_file():
        result["workspace"] = workspace(stage / WORKSPACE)
    if (stage / SOURCE_MARKERS[1]).is_file():
        gpu = metal(stage / "gpu", stage)
        result["metal"] = {"backend": gpu["backend"], "passed": gpu["passed"], "original_pngs": len(gpu["pngs"])}
    if (exact / BUILD_MARKERS[0]).is_file():
        manifest = exact / "web/SHA256SUMS"
        verify_entries(manifest.parent / "material-gallery", path_entries(manifest, allow_dot_prefix=True))
        result["web_manifest_sha256"] = digest(manifest)
    if (exact / BUILD_MARKERS[1]).is_file():
        manifest = exact / "binaries/SHA256SUMS"
        verify_entries(manifest.parent, path_entries(manifest))
        result["native_manifest_sha256"] = digest(manifest)
    if (exact / BUILD_MARKERS[2]).is_file():
        expected, executable = runner_pin(exact / RUNNER_PIN)
        require(digest(Path(executable)) == expected, "Actual stock runner build changed")
        result["stock_runner_sha256"] = expected
    return result


def archive_report(root):
    for name in (*SOURCE_MARKERS, *BUILD_MARKERS):
        require((root / "gates" / name).read_text() == TREE + "\n", f"Wrong archived marker {name}")
    return {"status": "verified exact recovered local evidence", "source": source_proof(root / "base.sha256", root),
            "workspace": workspace(root / "logs" / (WORKSPACE + ".gz")),
            "producer_log_sha256": {name: sha256(raw_log(root / "logs" / (name + ".gz"))).hexdigest()
                                    for name in (PRODUCER, RUNNER_LOG)},
            "metal": metal(root / "gpu", root / "logs", ".gz"),
            "browser": bound_browser(root / "chrome", root / "web.sha256", root / RUNNER_PIN, root / "browser-tools"),
            "native": bind_native(root / "native", root / "native-tools", root / "binaries.sha256")}


def collect(stage, browser, native, root):
    state = preflight(stage, browser, native)
    require(state["pending"] == [], "Awaiting real results: " + ", ".join(state["pending"]))
    exact = stage / "exact-candidate"
    binary = exact / "binaries/SHA256SUMS"
    web = exact / "web/SHA256SUMS"
    require(set(path_entries(binary)) == {"surface-material-gallery", "surface-devtools"}, "Wrong native build files")
    verify_entries(binary.parent, path_entries(binary))
    metal_result = metal(stage / "gpu", stage)
    native_result = bind_native(native, VALIDATION / "q75-native", binary)
    browser_result = bound_browser(browser, web, exact / RUNNER_PIN, VALIDATION / "browser", original=True)

    root.mkdir()
    copy(stage / "base.sha256", root / "base.sha256")
    copy_members(exact, root, ("source.sha256", "source.tree", PATCH_NAME, RUNNER_PIN))
    copy_members(stage, root / "gates", SOURCE_MARKERS)
    copy_members(exact, root / "gates", BUILD_MARKERS)
    copy(binary, root / "binaries.sha256")
    copy(web, root / "web.sha256")
    for name in (WORKSPACE, PRODUCER, RUNNER_LOG, *METAL):
        compressed(stage / name, root / "logs" / (name + ".gz"))
    copy_members(stage / "gpu", root / "gpu", ("SHA256SUMS", *metal_result["pngs"]))
    copy_members(native, root / "native", ("SHA256SUMS", *path_entries(native / "SHA256SUMS")))
    tools = VALIDATION / "q75-native"
    copy_members(tools, root / "native-tools", ("SHA256SUMS", *path_entries(tools / "SHA256SUMS")))
    copy_members(VALIDATION / "browser", root / "browser-tools", ("run-stock-material.py", "observe-focused-original.py"))
    copy_members(browser, root / "chrome", ("runner.log", "invocation.json", "inputs-after.json", WINDOW, BROWSER_RECEIPT,
                                           *(p.relative_to(browser).as_posix() for p in (browser / "guard-ledger").glob("*.json"))))
    copy_members(browser / "artifacts", root / "chrome/artifacts", (*REPORTS, *PNG))
    copy_members(HERE, root, ("package.py", "menu_browser.py", "evidence_common.py", "README.md"))
    result = archive_report(root)
    require(result["browser"] == browser_result and result["native"] == native_result and result["metal"] == metal_result,
            "Archive differs from original readbacks")
    json_file(root / "verified.json", result)
    output = root / "SHA256SUMS"
    output.write_text("".join(f"{digest(path)}  {path.relative_to(root).as_posix()}\n"
                              for path in sorted(root.rglob("*")) if path.is_file()))
    return {"archive": str(root), "manifest_sha256": digest(output), "files": len(path_entries(output)),
            "steps": browser_result["full_material_steps"], "checks": browser_result["checks"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--preflight", action="store_true")
    mode.add_argument("--collect", type=Path)
    mode.add_argument("--verify", type=Path)
    parser.add_argument("--stage", type=Path, default=DEFAULT)
    parser.add_argument("--browser", type=Path)
    parser.add_argument("--native", type=Path)
    parser.add_argument("--sha")
    args = parser.parse_args()
    browser = args.browser or args.stage / "guarded-stock-material"
    native = args.native or args.stage / "guarded-native"
    if args.preflight:
        result = preflight(args.stage, browser, native)
    elif args.collect:
        path = args.collect.resolve()
        require(path.is_relative_to(Path("/tmp").resolve()) and path.parent.is_dir() and not path.exists(),
                "Choose a fresh output under /tmp")
        result = collect(args.stage, browser, native, path)
    else:
        root = args.verify.resolve(strict=True)
        seal(root, "SHA256SUMS", args.sha, exact=True)
        result = archive_report(root)
        require(result == data(root / "verified.json"), "Portable verification differs from sealed receipt")
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
