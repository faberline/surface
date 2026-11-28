#!/usr/bin/env python3
"""Collect and recheck corrected Surface v0.1.68 macOS evidence without launching apps."""
import argparse
import json
from pathlib import Path
import re
from runpy import run_path
import sys
from tempfile import TemporaryDirectory

import r3

from evidence_common import (compressed, copy, copy_members, data, digest,
                             import_from, json_file, path_entries, raw_log, require,
                             run_json, seal, source_tree, tests, verify_entries)

HERE = Path(__file__).resolve().parent
STAGE = Path("/tmp/surface-md3-label-color-v0168-corrected.6WxZjn4h")
KIT = Path("/tmp/surface-md3-label-color-v0168-corrected-replay.2X3oZ1AB")
DIAG = Path("/tmp/surface-md3-label-color-v0168-contrast.CqYLtpEr")
HISTORICAL = Path("/tmp/surface-md3-label-color-v0168-fresh-replay.AwmrhZ8K")
SOURCE = "086f06d98a7c2f3728b67c9524b15ed8b380fb078d42c48bb80b1f082c24580c"
COMMIT = "e0b2a032f46a397265fee70136b471fbb5bcfd57"
TREE = "72ae6c3eb56e9554532f428b3308aadd8358a242"
KIT_SHA = "fcfa8650d87f531ed43c1a44e5c1d31bbf307ac379de35feeea74efae94d7ac0"
HIST_SHA = "5e19c8384a44e5ed31e5d61d1a7f03e14234476960b2f78ba6b78ba23fc37811"
GPU_SHA = "c7fb34d394d7702a9e9f6e8c617d552486dfb52bf766fce7bc4c8259440c409f"
OLD_NATIVE_SHA = "44d5bb4a88a83612b4b855b1f9bc42b0915ce39ced62a898a0eba00c44214fcd"
NATIVE_RUNTIME_SHA = "8ad648b73d3df7a3877844d6fcbedbed98825a84f3353ac86fa7e3e163ca0577"
DIAG_COMMIT = "7541bb6474e536945643b84badd84c604a4859fd"
NATIVE_TOOLS = "c842ea6ac04a2428ac239bf4ec6357d2912603f9cd5b3decd1029a9e0b1b1f35"
LABEL_REPORT = "material-text-field-label-motion.json"
ADJACENT_REPORTS = ("browser-smoke.json", "browser-ime.json", "browser-ime-cancel-redraw.json",
                    "browser-ime-cancel-selected-redraw.json", "bridge-web-result.json",
                    "devtools-headless-result.json")
STOCK_TOOLS_SHA = "09e5c4da3ea68a60ac5bca784268ecbe013b492139dcec50258205b90f938d0d"
STOCK_TOOLS = ("harness.sha256", "verify_label_color.py", "label-color-oracle.py", "observe-focused-original.py")
BUILD_LOGS = ("toolchain.txt", "web-build.log", "native-build.log", "stock-build.log")
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")


def marker(path):
    require(path.read_text().splitlines() == [COMMIT, TREE], f"Wrong terminal source marker: {path}")


def workspace(path):
    body, result = tests(path)
    wasm = re.findall(r"^\s*Finished `dev` profile.*$", body[body.rfind("test result:"):], re.M)
    require((result["passed"], result["ignored"], len(wasm)) == (1575, 20, 3),
            "Wrong corrected workspace or Wasm totals")
    return {**result, "wasm_checks": len(wasm)}


def gpu_entries(captures):
    manifest = captures / "SHA256SUMS"
    require(digest(manifest) == GPU_SHA, "Wrong original GPU manifest")
    rows = [re.fullmatch(r"([0-9a-f]{64})  \./(gpu-[a-z0-9-]+\.png)", line)
            for line in manifest.read_text().splitlines()]
    require(all(rows), "Invalid original GPU manifest entry")
    entries = {row[2]: row[1] for row in rows}
    require(len(rows) == len(entries) == 21 and {p.name for p in captures.iterdir()} == set(entries) | {"SHA256SUMS"},
            "Wrong Metal captures")
    verify_entries(captures, entries)
    return entries


def metal(primary, supplemental, captures):
    name = 'name: "Apple M1 Max"'
    backend = "backend: Metal"
    first, primary_stats = tests(primary)
    second, supplemental_stats = tests(supplemental)
    require((primary_stats["passed"], supplemental_stats["passed"]) == (16, 2) and
            all(any(name in line and backend in line for line in text.splitlines()) for text in (first, second)),
            "Missing actual Apple Metal primary or supplemental acceptance")
    table = gpu_entries(captures)
    from PIL import Image
    for name in table:
        with Image.open(captures / name) as image:
            require(image.format == "PNG" and bool(image.width and image.height), "Invalid Metal PNG")
            image.load()
    return {"adapter": "Apple M1 Max / Metal", "primary": primary_stats,
            "supplemental": supplemental_stats, "sealed_captures": len(table), "manifest_sha256": GPU_SHA}


def diagnostic(red, green):
    raw = raw_log(red).decode()
    _, passed = tests(green)
    require("Outlined label uses enabled ink on the first frame" in raw
            and "left: Color(29, 27, 32, 97)" in raw and "right: Color(186, 26, 26, 255)" in raw
            and "test result: FAILED. 0 passed; 1 failed;" in raw
            and passed["passed"] == 13 and passed["ignored"] == 0,
            "The targeted diagnostic did not reproduce old ink and pass all 13 after the fix")
    return {"observed_disabled_rgba": [29, 27, 32, 97], "expected_enabled_rgba": [186, 26, 26, 255],
            "green": passed, "archived_commit": DIAG_COMMIT}


def checked_preflight():
    seal(KIT, "SHA256SUMS", KIT_SHA)
    manifest = STAGE / "source.sha256"
    require(digest(manifest) == SOURCE and len(path_entries(manifest)) == 685
            and source_tree(STAGE / "surface", path_entries(manifest)) == TREE, "Wrong live R2 source")

    build = KIT / "outputs"
    with import_from(KIT):
        members = run_path(str(KIT / "pins.py"))["members"]
    receipt = data(build / "verified-build-inputs.json")
    require(receipt["source"]["tree"] == TREE and receipt["source"]["source_sha256"] == SOURCE
            and receipt["artifact_manifests"] == members(build)
            and receipt["logs"] == {name: digest(build / name) for name in BUILD_LOGS},
            "Wrong R2 live producer or build logs")
    marker(STAGE / "workspace-v0168-passed")
    marker(STAGE / "gpu-fixed-passed")
    require((STAGE / "contrast-proof-published").read_text().strip() == DIAG_COMMIT,
            "Wrong contrast evidence pin")
    return {"source": receipt["source"], "workspace": workspace(STAGE / "workspace-v0168.log"),
            "metal": metal(STAGE / "gpu-primary.log", STAGE / "gpu-supplemental.log", STAGE / "gpu-fixed"),
            "contrast": diagnostic(DIAG / "red.log", DIAG / "green.log")}


def native_verify(native, tools, binary_manifest):
    seal(tools, "harness.sha256", NATIVE_TOOLS, exact=True)
    require(digest(native / "SHA256SUMS") == NATIVE_RUNTIME_SHA,
            "Wrong original corrected R2 native manifest")
    receipt = run_json([tools / "verify-stored-multiline.py", native.resolve(),
                        "--manifest-sha", NATIVE_RUNTIME_SHA,
                        "--source-manifest-sha", SOURCE,
                        "--binary-manifest-sha", digest(binary_manifest), "--repeats", "2"])
    require((receipt["reaped_gallery_processes"], receipt["observed_window_samples"],
             receipt["visible_owned_windows"], receipt["original_pngs"]) == (4, 24, 0, 14),
            "Wrong exact native replay scope")
    return receipt


def stock_log(log):
    body = raw_log(log).decode()
    begins = re.findall(r"^case (\S+) \.\.\.$", body, re.M)
    cases = re.findall(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks,", body, re.M)
    actual = {name: {"steps": int(steps), "checks": int(checks)} for name, steps, checks in cases}
    require(tuple(begins) == tuple(name for name, _, _ in cases) == SUITES
            and not re.search(r"^case \S+ \.\.\. (?:FAILED|skipped)", body, re.M),
            "Wrong stock pass scope or order")
    total = sum(case["checks"] for case in actual.values())
    require(re.findall(r"^e2e result: (\d+) passed; (\d+) failed; (\d+) checks;", body, re.M)
            == [("5", "0", str(total))], "Wrong stock terminal result")
    material = [int(step) for step in re.findall(r"^  \[M(\d+)\]", body, re.M)]
    require(material == [i for i in range(1, 351) if i != 23] and actual["material"]["steps"] == 349,
            "Wrong stock Material registry")
    ime = [int(step) for step in re.findall(r"^  \[I(\d+)\]", body, re.M)]
    require(ime == list(range(1, 21)) and actual["ime"] == {"steps": 20, "checks": 77},
            "Wrong stock IME regression scope")
    return actual, total


def verify_payload(root):
    from hashlib import sha256

    from gzip import decompress

    marker(root / "workspace/terminal-source.txt")
    marker(root / "metal/terminal-source.txt")
    inputs = root / "inputs"
    original_kit_manifest = inputs / "SHA256SUMS"
    require(digest(original_kit_manifest) == KIT_SHA, "Wrong reviewed replay kit manifest")
    kit_pins = path_entries(original_kit_manifest)
    carried = ("offline-repin.log", "base-source.sha256", "fix-only.patch",
               "regression-only.patch", "text-field-label-color-v0168.patch.gz")
    verify_entries(inputs, {name: kit_pins[name] for name in carried})
    meta = data(root / "inputs/verified-build-inputs.json")
    require(meta["source"]["commit"] == COMMIT and meta["source"]["tree"] == TREE
            and meta["source"]["source_sha256"] == digest(root / "source.sha256") == SOURCE
            and meta["source"]["source_file_count"] == len(path_entries(root / "source.sha256")) == 685,
            "Wrong corrected source provenance")
    require(set(meta["logs"]) == set(BUILD_LOGS)
            and meta["source"]["base_source_sha256"] == digest(inputs / "base-source.sha256")
            and meta["source"]["correction_patches"] == {name: kit_pins[name] for name in
                ("fix-only.patch", "regression-only.patch")}
            and meta["source"]["patch_sha256"] == sha256(decompress(
                (inputs / "text-field-label-color-v0168.patch.gz").read_bytes())).hexdigest(),
            "Source patch or build log scope differs")
    for name, value in meta["logs"].items():
        require(sha256(raw_log(root / "inputs/build" / f"{name}.gz")).hexdigest() == value,
                f"Wrong build log: {name}")
    for name, value in meta["artifact_manifests"].items():
        require(digest(root / "inputs/build" / name) == value, f"Wrong compiled asset manifest: {name}")
    require(set(meta["artifact_manifests"]) == {"native/SHA256SUMS", "stock/SHA256SUMS"},
            "Missing separate native and stock manifests")
    artifact_manifests = root / "inputs/build"
    native = native_verify(root / "native/replay", root / "native/harness",
                          artifact_manifests / "native/SHA256SUMS")
    native_archive_manifest = root / "native/archived-endpoints.sha256"
    require(digest(native_archive_manifest) == OLD_NATIVE_SHA, "Wrong historical comparison native seal")
    fresh_endpoints = path_entries(root / "native/replay/SHA256SUMS")
    old_endpoints = path_entries(native_archive_manifest)
    require({name: sha for name, sha in fresh_endpoints.items() if name.endswith(".png")} ==
            {name: sha for name, sha in old_endpoints.items() if name.endswith(".png")},
            "Fresh native PNG byte parity differs")
    return {"source": meta["source"], "workspace": workspace(root / "workspace/workspace-v0168.log.gz"),
            "metal": metal(root / "metal/primary.log.gz", root / "metal/supplemental.log.gz", root / "metal/png"),
            "contrast": diagnostic(root / "contrast/red.log.gz", root / "contrast/green.log.gz"), "native": native}


def stock_result(stock, build, tools, historical=None):
    from hashlib import sha256

    run = stock / "run"
    observation = data(run / r3.WINDOW)
    r3.clean_windows(observation)
    r3.proof_via_stock_ledger(run, observation)
    actual = data(run / "verified-label-color-stock.json")
    terminal = data(stock / r3.TERMINAL)
    cases, count = stock_log(saved_log(run, "runner.log"))
    require(all(terminal[key] == value for key, value in actual.items())
            and actual["cases"] == terminal["cases"] == cases
            and actual["total_checks"] == terminal["total_checks"] == count
            and actual["page_errors"] == terminal["page_errors"] == 0
            and actual["adapter"] == terminal["adapter"]
            and actual["adapter"]["isFallbackAdapter"] is False
            and actual["adapter"]["vendor"] == "apple" and actual["adapter"]["architecture"].startswith("metal"),
            "R3 requires five actual complete headless stock suites on Apple WebGPU")
    for name in ADJACENT_REPORTS[:2]:
        require(data(run / "artifacts" / name)["errors"] == [], f"Chrome errors in {name}")
    ime = data(run / "artifacts/browser-ime.json")
    bridge = data(run / "artifacts/bridge-web-result.json")
    transport = data(run / "artifacts/devtools-headless-result.json")
    require(ime["committed"] == "繁體中文、简体中文" and bridge["webgpu_ready"] is True
            and bridge["owned_browser_exited"] is True and transport["screenshot_transports_match"] is True
            and transport["action_rendered"] is True, "Incomplete actual R3 browser/IME/headless proof")
    tool_manifest = tools / "harness.sha256"
    require(digest(tool_manifest) == STOCK_TOOLS_SHA, "Wrong original label pixel verifier seal")
    tool_pins = path_entries(tool_manifest)
    verify_entries(tools, {name: tool_pins[name] for name in STOCK_TOOLS if name != "harness.sha256"})
    with import_from(tools):
        verifier = run_path(str(tools / "verify_label_color.py"))
        png_report = verifier["verify"](run / "artifacts", {key: actual[key] for key in ("browser", "adapter")})
    require(png_report == actual["text_field_label_motion"] == terminal["text_field_label_motion"]
            and len(png_report["pngs"]) == 34, "R3 original label pixel or typed result differs")
    mid = r3.m350_pixels(run / "artifacts")
    m350 = terminal["m350"]
    require(m350["ms"] == mid["milliseconds"] == 44
            and m350["expected_rgb"] == mid["middle_rgb"]
            and m350["actual_probe_rgb"] == [mid["middle_rgb"]] * 4
            and m350["original_png_sha256"] == mid["intermediate_png_sha256"],
            "R3 terminal M350 oracle differs from the four actual pixels")
    history = r3.historical_comparison(historical or stock / "archived-focus.json", png_report)
    require(history == terminal["historical_byte_comparison"],
            "Historical hash comparison differs; live pass is measured separately")
    pins = r3.check_invocation(stock, build)
    require(terminal["window_receipt_sha256"] == digest(run / r3.WINDOW)
            and terminal["runner_log_sha256"] == sha256(raw_log(saved_log(run, "runner.log"))).hexdigest(),
            "Wrong R3 terminal result log hashes")
    return {"adapter": actual["adapter"], "browser": actual["browser"], "cases": cases,
            "checks": count, "label_pngs": len(png_report["pngs"]), "m350": mid, "inputs": pins,
            "historical_byte_comparison": history,
            "page_errors": 0, "visible_owned_windows": 0, "remaining_owned_processes": 0}


def saved_log(root, name):
    path = root / name
    return path if path.is_file() else root / (name + ".gz")


def archive_report(root):
    result = verify_payload(root)
    result["r3_source"] = r3.source_info(root / "r3", root / "source.sha256")
    result["r2_browser_failure"] = r3.failure_info(root / "r2-browser-failure/run",
                                                   root / "r2-browser-failure/pixels",
                                                   root / "inputs/build")
    r3.guard_members(root / "r3-tools")
    result["stock"] = stock_result(root / "stock", root / "inputs/build", root / "stock-tools")
    return result


def preflight(stock):
    result = checked_preflight()
    build = KIT / "outputs"
    result["native"] = native_verify(KIT / "guarded-native", HISTORICAL / "historical/native/harness",
                                      build / "native/SHA256SUMS")
    result["r3_source"] = r3.source_info(r3.R3, STAGE / "source.sha256", original=True)
    result["r2_browser_failure"] = r3.failure_info(KIT / "stock/run", r3.DIAGNOSTIC, build, original=True)
    result["pending"] = []
    r3.guard_members(r3.GUARD)
    if (stock / "run/invocation.json").is_file():
        result["r3_invocation_verified"] = True
        r3.check_invocation(stock, build, terminal=False)
    if not (stock / r3.TERMINAL).is_file():
        run = stock / "run"
        if (run / r3.WINDOW).is_file():
            window = data(run / r3.WINDOW)
            require(window["stock_exit"] == 0 and not window["problems"],
                    "R3 stock stopped before an accepted terminal result")
        result["pending"].append("exact R3 five-suite terminal browser result")
    else:
        stopped = run_json([r3.GUARD / "stock-r3.py", "--verify", stock])
        require(stopped == data(stock / r3.TERMINAL), "Live R3 wrapper reread differs")
        result["stock"] = stock_result(stock, build, KIT / "stock-tools",
                                       HISTORICAL / "historical/stock-macos/focused-proof.json")
    return result


def write_manifest(out):
    paths = sorted(p for p in out.rglob("*") if p.is_file() and p != out / "SHA256SUMS")
    (out / "SHA256SUMS").write_text("".join(f"{digest(p)}  {p.relative_to(out).as_posix()}\n" for p in paths))


def stage_product(out):
    build, native = KIT / "outputs", KIT / "guarded-native"
    copy_members(HERE, out, ("collect.py", "r3.py", "evidence_common.py", "README.md"))
    copy(KIT / "source.sha256", out / "source.sha256")
    copy_members(KIT, out / "inputs", ("offline-repin.log", "base-source.sha256", "fix-only.patch",
                                      "regression-only.patch", "text-field-label-color-v0168.patch.gz", "SHA256SUMS"))
    copy(build / "verified-build-inputs.json", out / "inputs/verified-build-inputs.json")
    copy_members(build, out / "inputs/build", ("native/SHA256SUMS", "stock/SHA256SUMS"))
    for name in BUILD_LOGS:
        compressed(build / name, out / "inputs/build" / f"{name}.gz")
    compressed(STAGE / "workspace-v0168.log", out / "workspace/workspace-v0168.log.gz")
    copy(STAGE / "workspace-v0168-passed", out / "workspace/terminal-source.txt")
    for log, dest in (("gpu-primary.log", "primary.log.gz"), ("gpu-supplemental.log", "supplemental.log.gz")):
        compressed(STAGE / log, out / "metal" / dest)
    copy(STAGE / "gpu-fixed-passed", out / "metal/terminal-source.txt")
    gpu = STAGE / "gpu-fixed"
    copy_members(gpu, out / "metal/png", (*gpu_entries(gpu), "SHA256SUMS"))
    for name in ("red.log", "green.log"):
        compressed(DIAG / name, out / "contrast" / f"{name}.gz")
    copy_members(DIAG, out / "contrast", ("red-reproduced", "green-passed"))
    harness = HISTORICAL / "historical/native/harness"
    copy_members(harness, out / "native/harness", (*path_entries(harness / "harness.sha256"), "harness.sha256"))
    copy_members(native, out / "native/replay", (*path_entries(native / "SHA256SUMS"), "SHA256SUMS"))
    copy(HISTORICAL / "historical/native/replay/SHA256SUMS", out / "native/archived-endpoints.sha256")
    copy_members(KIT, out / "inputs/replay-tools", ("README.md", "run-native.sh", "build-pinned.sh"))


def collect(out, stock):
    out = out.resolve()
    temp = Path("/tmp").resolve()
    require(out != temp and out.is_relative_to(temp) and not out.exists()
            and all(not out.is_relative_to(p.resolve()) for p in (STAGE, KIT, DIAG, HISTORICAL, r3.R3)),
            "Choose a fresh /tmp evidence output away from source and producer lanes")
    checked = preflight(stock)
    require(not checked["pending"], "Archive refused. Awaiting " + ", ".join(checked["pending"]))
    with TemporaryDirectory(prefix="surface-v0168-r3-proof-", dir=out.parent) as stage:
        archive = Path(stage) / "runtime"
        stage_product(archive)
        r3.stage_source(r3.R3, STAGE / "surface", archive / "r3")
        r3.stage_failure(KIT / "stock/run", r3.DIAGNOSTIC, archive / "r2-browser-failure")
        copy_members(r3.GUARD, archive / "r3-tools", ("SHA256SUMS", *r3.guard_members(r3.GUARD)))
        copy_members(KIT / "stock-tools", archive / "stock-tools", STOCK_TOOLS)
        r3.stage_stock(stock, archive / "stock", ADJACENT_REPORTS,
                       HISTORICAL / "historical/stock-macos/focused-proof.json")
        result = archive_report(archive)
        require(all(checked[key] == value for key, value in result.items()),
                "Portable archive differs from original R2/R3 readback")
        json_file(archive / "verified.json", result)
        write_manifest(archive)
        checksum = digest(archive / "SHA256SUMS")
        seal(archive, "SHA256SUMS", checksum, exact=True)
        archive.rename(out)
    return {"archive": str(out), "portable_manifest_sha256": checksum, **result}


def verify(out, checksum):
    seal(out, "SHA256SUMS", checksum, exact=True)
    result = archive_report(out)
    require(result == data(out / "verified.json"), "Portable result does not match recorded verified.json")
    return {"portable_manifest_sha256": checksum, **result}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--preflight", action="store_true")
    mode.add_argument("--collect", type=Path)
    mode.add_argument("--verify", type=Path)
    parser.add_argument("--sha")
    parser.add_argument("--stock", type=Path, default=r3.GUARD / "stock")
    args = parser.parse_args()
    stock = args.stock.resolve()
    if args.collect:
        result = collect(args.collect, stock)
    elif args.verify:
        result = verify(args.verify.resolve(strict=True), args.sha)
    else:
        result = preflight(stock)
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as exc:
        sys.exit(f"Surface v0.1.68 mixed evidence stopped. {exc}")
