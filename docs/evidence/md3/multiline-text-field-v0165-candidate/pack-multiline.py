#!/usr/bin/env python3
"""Validate and export native, Chrome and Rust evidence for Surface v0.1.65."""

import argparse
from contextlib import contextmanager
import gzip
from hashlib import file_digest, sha1, sha256
import json
import os
from pathlib import Path, PurePosixPath
import re
from runpy import run_path
import shutil
import subprocess
import sys
import tomllib

STAGE = Path("/tmp/surface-md3-text-field-multiline-v0165.ZRjH7vPS")
EXACT = STAGE / "exact-candidate"
BASE = Path("/tmp/surface-md3-text-field-counter-v0164.zL1rFusV")
NATIVE = Path("/tmp/surface-md3-multiline-native.cZ38fMkb")
FOCUSED = Path("/tmp/surface-md3-multiline-e2e.imcVWpWb")
STOCK = Path("/tmp/surface-md3-multiline-stock-v0165.C31fd1AG")
SOURCE = "6a420b56fc090457f9841fc8fda7c1bf0602e27138f586ca3f6b251652b580f2"
TREE = "2341dc506a206fe1e892c35a32d51386702958b3"
BASE_SOURCE = "fe7349b293de3078ca44fd6b1fa1885a5fa156ba559503c795cd9d6daab5e4a3"
BASE_TREE = "eeef2dc049409f7df3a67773d8e72e76b298faf6"
PATCH = "dbfd7299de8ddb80f22baf543a8c93e952ba187b06f39964ae993f05e2ff08df"
PATCH_NAME = "multiline-v0165.patch"
FOCUSED_SEAL = "c324f020b717a72dc9c0760ef22a1b4d12cc122b6dc5ceb1959b3728186573bf"
NATIVE_HARNESS = "c842ea6ac04a2428ac239bf4ec6357d2912603f9cd5b3decd1029a9e0b1b1f35"
NATIVE_BIN = "85afe8106d75c9c71213b9b098e86e54f8095a34449e089f35bad13072e5cdce"
NATIVE_PROOF = "d5fe9fccdb4214455fd0281c2cfc576dc80eb8b07cfd1d28fb54638819064fd6"
NATIVE_COMPARISON = "579091e823334ca36c04b9d3024efd60b6a0e0b714f406132c8bd8ed1d95ee01"
STOCK_HARNESS = "ebc28ad807afa006f250c67c427103587a0a89f7d5bcd65d9843b4ea6971937f"
RUST_LOGS = ("workspace-v0165-exact.log", "gpu-v0165-exact.log", "gpu-v0165-supplemental.log")
RUST_LOG_PINS = (
    "662b1fc6a69c4de0246ee15d3022c602e388c83ec154f343cebf00850519dd9a",
    "5c2ca203bfefd01485f4ac77f291f57e866319d8dccede8b9f58a1dd97a78d6d",
    "cd685ff340e05ceebb34348cb2936055872d672d3c90424a3c1cf56ab54302f1",
)
REPORT = "verified-multiline-v0165.json"
RAW_FEATURE = "browser-material-multiline-text-field.json"
STOCK_RESULT = "verified-multiline-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
WINDOW = "headless-window-observation.json"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
ADAPTER = {"vendor": "apple", "architecture": "metal-3", "isFallbackAdapter": False}
STOCK_PINS = (
    "browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256", "release-pin.json",
    "source-origins.json", "build-markers.json", "sealed-inputs.sha256", "inputs-before.json",
    "approved-runner.sha256",
)
RETAINED = (
    "browser-ime.json", "browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json",
    "browser-smoke.json", "browser-material-side-sheet.json", "bridge-web-result.json", "devtools-headless-result.json",
    "material-standard-slider.json", "material-slider-forms.json", "material-range-slider.json",
    "material-segmented.json", "material-buttons.json", "material-vertical-centered-slider.json",
    "material-slider-inset.json",
)
PIXEL_MODULES = {
    "multiline_text_field": ("verify_multiline.py", RAW_FEATURE),
    "text_field_counter": ("verify_counter.py", "material-text-field-counter.json"),
    "text_field_adornments": ("verify_adorned_field.py", "material-text-field-adornments.json"),
    "filled_text_field": ("verify_filled_field.py", "material-filled-text-field.json"),
    "outlined_text_field": ("verify_outlined_field.py", "material-outlined-text-field.json"),
}
FOCUS_FILES = (
    "README.md", "build-integrated.py", "run-integrated.py", "verify-development.py", "web-integrated.sha256",
    "focused-integrated-r1/runner.sha256", "focused-integrated-r1/scope-selector.rs",
    "verified-integrated-development.json", "integrated-r1/runner.log", "integrated-r1/invocation.json",
    f"integrated-r1/{WINDOW}", f"integrated-r1/artifacts/{RAW_FEATURE}",
    "source/e2e/src/material.rs", "source/e2e/src/material_multiline_text_field.rs",
)
REQUIRED_RUST = (
    "multiline_text_field_steps_are_required_by_the_material_case",
    "multiline_route_retains_two_editors_and_usable_nested_scroll_space",
    "growing_multiline_preserves_first_row_and_measures_preedit_and_committed_lines",
    "narrow_rtl_multiline_relocates_action_and_keeps_disabled_and_read_only_values",
    "multiline_text_projects_value_description_and_focus_by_mode",
    "release_can_take_the_axis_and_capture_loss_restores_both_previews",
    "panning_consumes_inner_then_outer_and_reverses_from_contact_origin",
    "text_field_counter_steps_are_required_by_the_material_case",
)
HEX = re.compile(r"[a-f0-9]{64}")


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def data(path):
    return json.loads(path.read_text())


def json_file(path, content):
    path.write_text(json.dumps(content, indent=2, ensure_ascii=False, sort_keys=True) + "\n")


def normalized(value):
    return json.loads(json.dumps(value))


def path_entries(path):
    entries = {}
    for line in path.read_text().splitlines():
        value, name = line.split("  ", 1)
        relative = PurePosixPath(name)
        require(HEX.fullmatch(value) and relative.as_posix() == name and not relative.is_absolute()
                and ".." not in relative.parts and name not in entries and name != ".",
                f"Invalid manifest path: {name}")
        entries[name] = value
    require(entries, f"Empty manifest: {path}")
    return entries


def verify_entries(root, entries):
    for name, expected in entries.items():
        path = root / name
        require(path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root.resolve())
                and digest(path) == expected, f"Different or missing pinned file: {path}")


def seal(root, name, expected, *, exact=False):
    require(HEX.fullmatch(expected or "") and digest(root / name) == expected, f"Wrong seal for {root / name}")
    entries = path_entries(root / name)
    verify_entries(root, entries)
    if exact:
        files = {str(file.relative_to(root)) for file in root.rglob("*") if file.is_file() and file != root / name}
        require(files == entries.keys(), "Portable proof has unsealed files")
    return entries


def object_id(kind, content):
    return sha1(f"{kind} {len(content)}\0".encode() + content).digest()


def source_tree(root, entries):
    folders = {}
    for name, checksum in entries.items():
        *parents, filename = PurePosixPath(name).parts
        folder = folders
        for parent in parents:
            folder = folder.setdefault(parent, {})
        path = root / name
        payload = path.read_bytes()
        require(sha256(payload).hexdigest() == checksum, f"Source differs during tree verification: {name}")
        folder[filename] = (b"100755" if path.stat().st_mode & 0o100 else b"100644", object_id("blob", payload))

    def tree(folder):
        items = []
        for name, child in sorted(folder.items(), key=lambda item: (item[0]+("/" if isinstance(item[1], dict) else "")).encode()):
            mode, oid = (b"40000", tree(child)) if isinstance(child, dict) else child
            items.append(mode + b" " + name.encode() + b"\0" + oid)
        return object_id("tree", b"".join(items))

    return tree(folders).hex()


def source_info(exact=EXACT, base=BASE):
    before = path_entries(base / "source.sha256")
    after = path_entries(exact / "source.sha256")
    for root, entries, count, expected_tree, expected_seal in (
        (base, before, 678, BASE_TREE, BASE_SOURCE), (exact, after, 681, TREE, SOURCE),
    ):
        require(len(entries) == count and digest(root / "source.sha256") == expected_seal, "Wrong source inventory")
        source = root / "source"
        verify_entries(source, entries)
        actual = {str(p.relative_to(source)) for p in source.rglob("*")
                  if p.is_file() and p.relative_to(source).parts[0] != ".git"}
        require(actual == entries.keys() and source_tree(source, entries) == expected_tree, "Wrong exact Git tree or path set")
    patch = STAGE / PATCH_NAME
    require(digest(patch) == PATCH and (STAGE / "source-tree").read_text().strip() == TREE
            and (STAGE / "exact-apply-checked").is_file(), "Wrong exact apply, patch or tree marker")
    touched = set(re.findall(r"^diff --git a/(\S+) b/\S+$", patch.read_text(), re.M))
    changed = {name for name in before.keys() | after.keys() if before.get(name) != after.get(name)}
    require(touched == changed and len(touched) == 31
            and sorted(touched) == sorted((STAGE / "owned-multiline.paths").read_text().splitlines()),
            "Wrong 31-path release scope")
    source = exact / "source"
    require(tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]["version"] == "0.1.65",
            "Wrong workspace version")
    registry = (source / "e2e/src/material.rs").read_text().split("pub const STEPS:", 1)[1].split("];", 1)[0]
    require([int(n) for n in re.findall(r'"M(\d+)"', registry)] == [n for n in range(1, 346) if n != 23],
            "Exact Material registry lost a step")
    for root, options in ((base / "source", []), (source, ["--reverse"])):
        subprocess.run(["git", "-C", str(root), "apply", "--check", "--whitespace=error-all", *options, str(patch)],
                       check=True, env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"})
    for relative in ("e2e/src/material.rs", "e2e/src/material_multiline_text_field.rs"):
        require(digest(FOCUSED / "source" / relative) == after[relative], "Focused Rust acceptance source differs")
    return {"version": "0.1.65", "source_sha256": SOURCE, "source_files": len(after), "tree": TREE,
            "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH,
            "changed_paths": sorted(touched)}


@contextmanager
def import_from(root):
    sys.path.insert(0, str(root))
    try:
        yield
    finally:
        sys.path.pop(0)


def run_json(argv):
    result = subprocess.run([sys.executable, "-E", "-B", *map(str, argv)], capture_output=True, text=True, check=False)
    require(result.returncode == 0, f"Offline verifier failed: {result.stderr.strip()}")
    return json.loads(result.stdout)


def raw_log(path):
    return gzip.decompress(path.read_bytes()) if path.suffix == ".gz" else path.read_bytes()


def tests(log):
    payload = raw_log(log)
    text = payload.decode()
    rows = re.findall(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", text, re.M)
    require(rows and all(state == "ok" and failed == "0" for state, _, failed, _ in rows),
            f"Failed or missing exact tests in {log.name}")
    return text, {"passed": sum(int(passed) for _, passed, _, _ in rows),
                  "ignored": sum(int(ignored) for _, _, _, ignored in rows),
                  "log_sha256": sha256(payload).hexdigest()}


def workspace_info(log):
    text, receipt = tests(log)
    tail = text.rsplit("\ntest result:", 1)[-1]
    require((receipt["passed"], receipt["ignored"]) == (1561, 20)
            and tail.count("Finished \x60dev\x60 profile") == 3
            and "Doc-tests surface_material_gallery" in text
            and all(f"{name} ... ok" in text for name in REQUIRED_RUST)
            and re.search(r"^provenance: .+ self-test cases passed\)", text, re.M),
            "Missing the exact workspace, provenance or three final Wasm checks")
    return {**receipt, "wasm_checks": 3}


def gpu_info(primary, supplemental):
    text, receipt = tests(primary)
    require(receipt["passed"] == 16 and receipt["ignored"] == 0 and "backend: Metal" in text
            and 'name: "Apple M1 Max"' in text, "Missing actual primary Apple Metal GPU checks")
    raw, extra = tests(supplemental)
    names = ("right_aligned_wrapped_selection_agrees_with_shaped_geometry",
             "paired_selection_matches_single_draw_references_at_fractional_clip_edges")
    require(extra["passed"] == 2 and extra["ignored"] == 0 and raw.count("backend: Metal") == 2
            and all(f"test {name} ... ok" in raw for name in names), "Missing two supplemental Metal tests")
    return {"metal": {**receipt, "backend": "Metal"},
            "supplemental": {**extra, "backend": "Metal", "tests": list(names)}, "metal_total_passed": 18}


def rust_info(root, *, archived=False):
    logs = [root / (f"{name}.gz" if archived else name) for name in RUST_LOGS]
    for log, expected in zip(logs, RUST_LOG_PINS, strict=True):
        require(sha256(raw_log(log)).hexdigest() == expected, f"Wrong exact Rust or GPU log bytes: {log.name}")
    workspace, primary, supplemental = logs
    return {"workspace": workspace_info(workspace), **gpu_info(primary, supplemental)}


def native_info(root):
    seal(root / "harness", "harness.sha256", NATIVE_HARNESS)
    require(digest(root / "native-binaries.sha256") == NATIVE_BIN, "Wrong exact native executable pin")
    receipt = run_json([root / "harness/verify-stored-multiline.py", root / "proof",
                        "--manifest-sha", NATIVE_PROOF, "--source-manifest-sha", SOURCE,
                        "--binary-manifest-sha", NATIVE_BIN])
    require(receipt == data(root / "verified.json") and receipt["original_pngs"] == 14
            and receipt["observed_window_samples"] == 24 and receipt["reaped_gallery_processes"] == 4
            and receipt["visible_owned_windows"] == 0
            and root.joinpath("terminal.log").read_text().startswith("2 out of 2 headless replays passed at "),
            "Exact native parity, pixel or zero-window receipt differs")
    comparison = data(root / "pixel-comparison.json")
    require(digest(root / "pixel-comparison.json") == NATIVE_COMPARISON and comparison["images_equal"] == 14
            and comparison["behavior_equal"] and comparison["native_binaries_equal"]
            and comparison["exact_source"] == {"files": 681, "manifest_sha256": SOURCE},
            "Exact native cross-build comparison differs")
    return {**receipt, "transports": ["Page.screenshot", "CLI", "MCP"],
            "development_comparison_sha256": NATIVE_COMPARISON}


def clean_windows(watch):
    require(watch["stock_exit"] == 0 and watch["visible_owned_windows"] == []
            and watch["remaining_observed_processes"] == [] and watch["problems"] == [],
            "Guard recorded an error, visible window or surviving process")


def verify_ledgers(run, watch, roles):
    ledger = [data(file) for file in (run / "guard-ledger").glob("*.json")]
    events = watch["guard_events"]
    require(len(events) == len(roles) and {event["role"] for event in events} == roles
            and len({event["pid"] for event in events}) == len(roles)
            and sorted(ledger, key=lambda event: event["pid"]) == sorted(events, key=lambda event: event["pid"]),
            "Wrong original guard ledger or launch scope")
    for event in events:
        group = watch["launch_observations"][event["role"]]
        require(event["allowed"] is True and group["pids"] == [event["pid"]] and group["samples"] > 0,
                "Incomplete guarded process samples")
        if event["role"] == "native":
            require(event["argv"] == ["--headless", "--width", "720", "--height", "640"] and event["profile"] is None,
                    "Native launch lost exact offscreen flags")
        else:
            headless = [flag for flag in event["argv"] if flag == "--headless" or flag.startswith("--headless=")]
            profiles = [flag for flag in event["argv"] if flag.startswith("--user-data-dir=")]
            require(headless == ["--headless=new"] and profiles == [f'--user-data-dir={event["profile"]}']
                    and "/tmp/" in event["profile"], "Chrome launch lost exact headless flags or private profile")


def focused_info(root, feature_verifier):
    run = root / "integrated-r1"
    watch = data(run / WINDOW)
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary"})
    args = data(run / "invocation.json")
    require(args["argv"][-1:] == ["material"] and "development" in args["scope"]
            and args["runner_sha256"] == (root / "focused-integrated-r1/runner.sha256").read_text().split()[0],
            "Development invoked a different focused runner")
    assets = path_entries(root / "web-integrated.sha256")
    require(len(assets) == 4 and args["web_files"] == {PurePosixPath(name).name: sha for name, sha in assets.items()},
            "Wrong integrated development Web files")
    scoped = run_json([root / "verify-development.py", run])
    require(scoped == data(root / "verified-integrated-development.json")
            and scoped["checks"] == 247 and scoped["steps"] == [f"M{i}" for i in range(341, 346)]
            and scoped["adapter"] == ADAPTER and len(scoped["captures"]) == 12
            and scoped["chrome_observations"] == 56, "Wrong integrated development scope")
    log = (run / "runner.log").read_text()
    browser = re.findall(r"^browser: (Chrome/\S+) ", log, re.M)
    require(len(browser) == 1 and re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"],
            "Unscoped focused E2E log")
    feature = normalized(feature_verifier(run / "artifacts", {"adapter": ADAPTER, "browser": browser[0]}))
    require(set(scoped["captures"]) == set(feature["pngs"]), "Focused independent raster proof differs")
    return {**scoped, "browser": browser[0], "web_files": args["web_files"], "runner_sha256": args["runner_sha256"],
            "multiline_text_field": feature}


def stock_tools(root):
    with import_from(root):
        observer = run_path(str(root / "observe-multiline.py"))
        pixels = {key: run_path(str(root / module))["verify"] for key, (module, _) in PIXEL_MODULES.items()}
    require(observer["SOURCE_SHA"] == SOURCE and observer["SOURCE_FILES"] == 681
            and observer["SOURCE_TREE"] == TREE and observer["LAST_MATERIAL"] == 345,
            "Stock observer targets a different candidate")
    return observer, pixels


def stock_info(run, pins, observer, pixels):
    verdict = data(run / STOCK_RESULT)
    offline = data(pins / OFFLINE)
    require(all(offline[key] == value for key, value in verdict.items()), "Live and offline verdicts differ")
    watch = data(run / WINDOW)
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary", "chrome-bridge", "native"})
    require(watch["launch_observations"]["chrome-bridge"]["detached_samples"] > 0, "Missing detached Bridge Chrome samples")
    log = raw_log(run / ("runner.log" if (run / "runner.log").exists() else "runner.log.gz"))
    cases, checks = observer["verify_runner_log"](log.decode(), 345)
    require(cases == verdict["cases"] and checks == verdict["total_checks"] and cases["material"]["steps"] == 344
            and verdict["page_errors"] == 0 and digest(run / WINDOW) == offline["window_receipt_sha256"]
            and sha256(log).hexdigest() == offline["runner_log_sha256"], "Incomplete exact stock result")
    expected_pin = {"version": "0.1.65", "source_sha256": SOURCE, "source_files": 681, "source_tree": TREE,
                    "last_material_step": 345, "wide_canvas_height": 800, "rtl_canvas_height": 1020}
    before, after = data(pins / "inputs-before.json"), data(run / "inputs-after.json")
    invocation = data(run / "invocation.json")
    runner = (pins / "approved-runner.sha256").read_text().strip()
    entries = path_entries(pins / "browser-binary-sha256.txt")
    require(before == after == invocation["inputs"] and tuple(invocation["suite_arguments"]) == SUITES
            and invocation["injected_chrome_flags"] == [] and (after["release"] == offline["release"]
            == data(pins / "release-pin.json") == expected_pin)
            and after["source_manifest_sha256"] == digest(pins / "browser-source.sha256") == SOURCE
            and after["source_files_checked_per_origin"] == 681 and after["stock_files"] == entries
            and after["bundle_files_checked"] == len(entries) == 16
            and HEX.fullmatch(runner) and runner == entries["debug/surface-e2e"] == after["runner_sha256"]
            and digest(pins / "native-binaries.sha256") == NATIVE_BIN,
            "Wrong exact stock source, frozen binaries or suite arguments")
    seal(pins, "sealed-inputs.sha256", after["app_bundle_seal_sha256"])
    require(verdict["adapter"] == ADAPTER and verdict["guard_and_window_checks"] == "passed",
            "Stock browser did not report Apple Metal or complete guard checks")
    artifacts = run / "artifacts"
    metadata = {"browser": verdict["browser"], "adapter": ADAPTER}
    retained = {name: digest(artifacts / name) for name in RETAINED}
    smoke, ime, side, bridge, native = (data(artifacts / name) for name in (
        "browser-smoke.json", "browser-ime.json", "browser-material-side-sheet.json",
        "bridge-web-result.json", "devtools-headless-result.json"))
    require(smoke["errors"] == ime["errors"] == side["errors"] == []
            and smoke["browser"] == side["browser"] == verdict["browser"]
            and side["adapter"] == {key: smoke["adapter"][key] for key in ADAPTER} == ADAPTER
            and ime["committed"] == "繁體中文、简体中文" and bridge["webgpu_ready"] is True
            and bridge["owned_browser_exited"] is True and native["action_rendered"] is True
            and native["screenshot_transports_match"] is True
            and (native["width"], native["height"], native["scale"]) == (720, 640, 1),
            "Raw stock smoke, IME, Bridge or DevTools receipt differs")
    select_all = data(artifacts / "browser-ime-cancel-redraw.json")
    selected = data(artifacts / "browser-ime-cancel-selected-redraw.json")
    require(select_all["before"]["browser"] == select_all["after"]["browser"] == [0, 2]
            and select_all["before"]["trustedSelectAll"] is True
            and select_all["after"]["frames"] > select_all["before"]["frames"]
            and select_all["replacement"] == ["中👨‍👩‍👧‍👦é", "中👨‍👩‍👧‍👦é", "31", "31", "false"]
            and selected["canceled"]["direction"] == "backward" and selected["moved"]["trustedRight"] is True
            and selected["unpainted"]["value"] == selected["repainted"]["value"] == "台文X"
            and selected["unpainted"]["frames"] == selected["canceled"]["frames"] < selected["repainted"]["frames"],
            "Stock archive lost either retained I11 case")
    editors = {key: normalized(tool(artifacts, metadata)) for key, tool in pixels.items()}
    require(len(editors["multiline_text_field"]["pngs"]) == 12
            and all(value == verdict[key] for key, value in editors.items()), "Edited field stock pixel proof differs")
    gallery = {PurePosixPath(name).name: sha for name, sha in entries.items()
               if PurePosixPath(name).parent == PurePosixPath("web/release/material-gallery")}
    require(len(gallery) == 4, "Stock omitted canonical Gallery assets")
    return {**metadata, "cases": cases, "total_checks": checks, "visible_windows": 0, "surviving_processes": 0,
            "page_errors": 0, "runner_sha256": runner, "offline_receipt_sha256": digest(pins / OFFLINE),
            "gallery_web_files": gallery, "retained_reports": retained, **editors}


def copy(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)


def compressed(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(gzip.compress(source.read_bytes(), mtime=0))


def copy_members(source, target, names):
    for name in sorted(set(names)):
        copy(source / name, target / name)


def archive_native(target):
    copy_members(NATIVE, target / "harness", (*path_entries(NATIVE / "harness.sha256"), "harness.sha256"))
    copy(NATIVE / "EXACT-R2.md", target / "harness/EXACT-R2.md")
    copy(EXACT / "binaries/SHA256SUMS", target / "native-binaries.sha256")
    files = {"exact-1.log": "terminal.log", "exact-1-verified.json": "verified.json",
             "exact-1-pixel-comparison.json": "pixel-comparison.json", "compare-exact-pngs.py": "compare-exact-pngs.py"}
    for original, name in files.items():
        copy(NATIVE / original, target / name)
    copy_members(NATIVE / "exact-1", target / "proof",
                 (*path_entries(NATIVE / "exact-1/SHA256SUMS"), "SHA256SUMS"))


def native_preflight():
    seal(NATIVE, "exact-proof.sha256", "987d5041a6a58a30f46207217e105842f159e6c34b9ec97d14946e88dc402db5")
    seal(EXACT / "binaries", "SHA256SUMS", NATIVE_BIN)
    result = run_json([NATIVE / "verify-stored-multiline.py", NATIVE / "exact-1",
                       "--manifest-sha", NATIVE_PROOF, "--source-manifest-sha", SOURCE,
                       "--binary-manifest-sha", NATIVE_BIN])
    require(result == data(NATIVE / "exact-1-verified.json") and result["original_pngs"] == 14
            and result["observed_window_samples"] == 24 and result["reaped_gallery_processes"] == 4,
            "Exact native receipt does not independently recheck")
    return {**result, "transports": ["Page.screenshot", "CLI", "MCP"],
            "development_comparison_sha256": NATIVE_COMPARISON}


def focus_preflight():
    seal(FOCUSED, "SHA256SUMS", FOCUSED_SEAL)
    seal(STOCK, "harness.sha256", STOCK_HARNESS)
    _, pixels = stock_tools(STOCK)
    result = focused_info(FOCUSED, pixels["multiline_text_field"])
    verify_entries(FOCUSED / "web-integrated/material-gallery", result["web_files"])
    return result


def archive_focused(target, result):
    copy_members(FOCUSED, target, (*FOCUS_FILES, *(
        f"integrated-r1/artifacts/{name}" for name in result["captures"])))
    copy_members(FOCUSED / "integrated-r1", target / "integrated-r1", (
        str(name.relative_to(FOCUSED / "integrated-r1"))
        for name in (FOCUSED / "integrated-r1/guard-ledger").glob("*.json")))


def archive_stock(target, result, harness_digest):
    live = STOCK / "run"
    harness = seal(STOCK, "harness.sha256", harness_digest)
    inputs = path_entries(STOCK / "sealed-inputs.sha256")
    copy_members(STOCK, target, (*STOCK_PINS, *harness, *inputs, "harness.sha256", OFFLINE))
    copy_members(live, target, (WINDOW, STOCK_RESULT, "invocation.json", "inputs-after.json",
                                *(str(file.relative_to(live)) for file in (live / "guard-ledger").glob("*.json"))))
    compressed(live / "runner.log", target / "runner.log.gz")
    files = (*RETAINED, *(name for key, (_, report) in PIXEL_MODULES.items()
                          for name in (report, *result[key]["pngs"])))
    copy_members(live / "artifacts", target / "artifacts", files)


def export(out, receipt):
    out = out.resolve()
    require(out != Path("/tmp").resolve() and out.is_relative_to(Path("/tmp").resolve()),
            "Use a fresh private /tmp evidence path")
    out.mkdir(mode=0o700)
    copy(EXACT / "source.sha256", out / "source.sha256")
    copy(BASE / "source.sha256", out / "base-source.sha256")
    copy(STAGE / "owned-multiline.paths", out / "owned-multiline.paths")
    compressed(STAGE / PATCH_NAME, out / f"{PATCH_NAME}.gz")
    copy(Path(__file__), out / "pack-multiline.py")
    for name in RUST_LOGS:
        compressed(STAGE / name, out / "rust" / f"{name}.gz")
    archive_native(out / "native")
    archive_focused(out / "focused", receipt["focused"])
    archive_stock(out / "stock-macos", receipt["stock_macos"], receipt["stock_harness_sha256"])
    json_file(out / REPORT, receipt)
    (out / "README.md").write_text(f"""# Surface v0.1.65 Multiline Text Field candidate proof

This archive covers local verification of the exact candidate tree
{TREE}. The source manifest pins 681 files. The 31-path patch
applies to the v0.1.64 tree {BASE_TREE}.

- The workspace passed {receipt['rust']['workspace']['passed']:,} Rust tests and
  three WebAssembly checks. {receipt['rust']['metal_total_passed']} GPU tests ran
  on Apple Metal.
- Native headless Gallery produced seven originals in each of two
  runs. Page.screenshot, CLI and MCP matched. The window observer saw
  zero visible windows in 24 checks. All four Gallery processes exited.
- The separate integrated development Chrome run passed 247 checks from
  M341 through M345. It recorded 12 original PNGs. Its four Gallery Web
  files match the exact stock files.
- Exact stock macOS passed {receipt['stock_macos']['total_checks']:,} checks
  in five headless suites. It ran all 344 Material steps through M345
  and both retained IME I11 paths. The archive includes 12 multiline PNGs
  plus the earlier Counter, Adornments, Filled and Outlined editor images.
  Chrome used Apple Metal, and the guard found zero visible windows or
  surviving processes.

The native harness uses semantic DevTools keys. The Chrome harness uses
CDP input and verifies trusted pointer and wheel events plus a simulated
composition. Speech and physical input methods require separate tests.

## Offline recheck

Use Python 3.12 with Pillow to read the archived files.

~~~sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-multiline.py --verify-package . --package-sha "$manifest_sha"
~~~

The gzip files keep their original log or patch bytes. This command
does not launch the Gallery or Chrome.
""")
    files = sorted(file for file in out.rglob("*") if file.is_file())
    (out / "SHA256SUMS").write_text("".join(f"{digest(file)}  {file.relative_to(out)}\n" for file in files))
    return digest(out / "SHA256SUMS")


def verify_package(root, checksum):
    seal(root, "SHA256SUMS", checksum, exact=True)
    receipt = data(root / REPORT)
    source = receipt["source"]
    patch = raw_log(root / f"{PATCH_NAME}.gz")
    patch_paths = sorted(re.findall(r"^diff --git a/(\S+) b/\S+$", patch.decode(), re.M))
    require(receipt["status"] == "verified" and source["version"] == "0.1.65" and source["tree"] == TREE
            and source["source_sha256"] == SOURCE and source["patch_sha256"] == PATCH and source["base_tree"] == BASE_TREE
            and source["source_files"] == 681 and source["base_source_sha256"] == BASE_SOURCE
            and digest(root / "source.sha256") == SOURCE and len(path_entries(root / "source.sha256")) == 681
            and digest(root / "base-source.sha256") == BASE_SOURCE and len(path_entries(root / "base-source.sha256")) == 678
            and sha256(patch).hexdigest() == PATCH
            and source["changed_paths"] == sorted((root / "owned-multiline.paths").read_text().splitlines())
            == patch_paths
            and len(source["changed_paths"]) == 31, "Archived exact release source evidence differs")
    stock = root / "stock-macos"
    require(receipt["stock_harness_sha256"] == STOCK_HARNESS, "The sealed stock harness changed")
    seal(stock, "harness.sha256", STOCK_HARNESS)
    observer, pixels = stock_tools(stock)
    focused = focused_info(root / "focused", pixels["multiline_text_field"])
    full = stock_info(stock, stock, observer, pixels)
    require(focused == receipt["focused"] and full == receipt["stock_macos"]
            and focused["web_files"] == full["gallery_web_files"], "Archived stock or focused evidence differs")
    output_entries = path_entries(root / "source.sha256")
    for relative in ("e2e/src/material.rs", "e2e/src/material_multiline_text_field.rs"):
        require(digest(root / "focused/source" / relative) == output_entries[relative], "Archived development source differs")
    require(native_info(root / "native") == receipt["native"], "Archived exact native evidence differs")
    require(rust_info(root / "rust", archived=True) == receipt["rust"], "Archived workspace or Metal rows differ")
    return receipt


def terminal_receipts():
    paths = (STAGE / "workspace-v0165-passed", STAGE / "gpu-v0165-passed", EXACT / "native-final-built",
             EXACT / "stock-canonical-built", STOCK / "stock-mac-v0165-passed")
    return [str(path) for path in paths if not path.is_file()]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-sha")
    parser.add_argument("--tree")
    parser.add_argument("--stock-harness-sha")
    parser.add_argument("--stock-offline-sha")
    parser.add_argument("--stock-runner-sha")
    parser.add_argument("--package-sha")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check-source", action="store_true")
    mode.add_argument("--preflight", action="store_true")
    mode.add_argument("--out", type=Path)
    mode.add_argument("--verify-package", type=Path)
    args = parser.parse_args()
    if args.verify_package:
        result = verify_package(args.verify_package.resolve(strict=True), args.package_sha)
    else:
        require(args.source_sha == SOURCE and args.tree == TREE, "Supply the reviewed candidate tree and source seal")
        source = source_info()
        if args.check_source:
            result = source
        else:
            result = {"status": "preflight only", "source": source, "native": native_preflight(),
                      "focused": focus_preflight(), "pending_markers": terminal_receipts()}
            if (STAGE / "workspace-v0165-passed").is_file() and (STAGE / "gpu-v0165-passed").is_file():
                result["rust"] = rust_info(STAGE)
        if args.out:
            require(not result["pending_markers"], "Pending exact gates:\n" + "\n".join(result["pending_markers"]))
            require(args.stock_harness_sha == STOCK_HARNESS, "Supply the reviewed exact stock harness digest")
            seal(STOCK, "harness.sha256", STOCK_HARNESS)
            require(HEX.fullmatch(args.stock_offline_sha or "") and HEX.fullmatch(args.stock_runner_sha or "")
                    and digest(STOCK / OFFLINE) == args.stock_offline_sha
                    and (STOCK / "approved-runner.sha256").read_text().strip() == args.stock_runner_sha,
                    "Exact stock verifier or runner digests differ")
            live = run_json([STOCK / "verify-stock-receipt.py", STOCK / "run"])
            require(live == data(STOCK / OFFLINE), "Actual exact stock replay did not reproduce the stock verdict")
            observer, pixels = stock_tools(STOCK)
            stock = stock_info(STOCK / "run", STOCK, observer, pixels)
            require(result["focused"]["web_files"] == stock["gallery_web_files"],
                    "Integrated focused Gallery Web files differ from exact canonical stock")
            result = {key: value for key, value in result.items() if key != "pending_markers"}
            result.update({"status": "verified", "stock_macos": stock, "stock_harness_sha256": args.stock_harness_sha})
            destination = args.out.resolve()
            marker = destination.with_name(destination.name + "-verified.json")
            require(not marker.exists(), "Use a fresh portable verification marker path")
            checksum = export(destination, result)
            require(verify_package(destination, checksum) == result, "Fresh portable archive validation differs")
            output = {"path": str(destination), "manifest_sha256": checksum,
                      "files": len(path_entries(destination / "SHA256SUMS"))}
            json_file(marker, {**output, "source_sha256": SOURCE, "tree": TREE, "runner_sha256": args.stock_runner_sha,
                               "stock_offline_sha256": args.stock_offline_sha,
                               "report_sha256": digest(destination / REPORT)})
            result = {**result, "output": output, "marker": str(marker)}
    print(json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, RuntimeError, KeyError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(f"Multiline evidence stopped. {error}")
