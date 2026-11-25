#!/usr/bin/env python3
"""Validate and export Surface v0.1.66 evidence without starting an app."""

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

from PIL import Image

STAGE = Path("/tmp/surface-md3-textbox-values-v0166.l4950l9e")
EXACT = STAGE / "exact-candidate"
BASE = Path("/tmp/surface-md3-text-field-multiline-v0165.ZRjH7vPS")
FOCUSED = Path("/tmp/surface-md3-textbox-values-e2e.ohBgx2tj")
PAIR = Path("/tmp/surface-md3-textbox-values-proof.n472jbhl")
STOCK = Path("/tmp/surface-md3-textbox-values-stock-v0166.z81UZBVd")
NATIVE_HARNESS = Path("/tmp/surface-md3-multiline-native.cZ38fMkb")
SOURCE = "d4b0c5b7a1ae5daf1f7a9084e765b30b24f8cae6ecbda881efb9468daf730783"
TREE = "3de0c143c43d35911593cfc95d3ea5931016c308"
BASE_SOURCE = "6a420b56fc090457f9841fc8fda7c1bf0602e27138f586ca3f6b251652b580f2"
BASE_TREE = "2341dc506a206fe1e892c35a32d51386702958b3"
PATCH = "a5a09a61580a7ae186d0fc95477d40582bba8799b21773e387bc21c4e38760e1"
PATCH_NAME = "textbox-values-v0166.patch"
STOCK_HARNESS = "d1d73190c1bb5f08225369777b5a8a20530449ff98ebab40ffc44744df8f4ea9"
FOCUSED_RECEIPT = "62ee6008c8a701e3c58911277a86f6e9cd8ad9e8b968d4773b78d3dfd1e304ce"
PAIR_SEAL = "6911c95d68407c4703ac678eca3ef06d8b21b3224f8c7231b27aef9cfcc477c6"
NATIVE_BIN = "89fc80bebbc8af785eca14e568f06429755ea1acd15520379490dd721cc0a2d7"
NATIVE_SOURCE = "0675a92cea6b221f96ade99a9e7e5569094a313b64c1367879b75f905ae784cb"
NATIVE_PREVIEW = "6449ab327baaf4cc07050cba55b08952e42eb647dc31731440d146dcc3517e0a"
NATIVE_REPLAY_HARNESS = "c842ea6ac04a2428ac239bf4ec6357d2912603f9cd5b3decd1029a9e0b1b1f35"
NATIVE_EXACT = "aa5c88610e5390f677ed653faa5843312fa9f0f11f77edbc659d50fb195f0858"
NATIVE_EXACT_RESULT = "59d42a569845a41e3b90b983aabe643c512210dcca47107d0cf4b8d281a316f1"
NATIVE_PRIOR = "d5fe9fccdb4214455fd0281c2cfc576dc80eb8b07cfd1d28fb54638819064fd6"
REPORT = "verified-textbox-values-v0166.json"
FEATURE = "browser-material-multiline-text-field.json"
STOCK_RESULT = "verified-textbox-values-stock.json"
OFFLINE = "stock-mac-exact-offline.json"
WINDOW = "headless-window-observation.json"
ADAPTER = {"vendor": "apple", "architecture": "metal-3", "isFallbackAdapter": False}
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
RUST_LOGS = {
    "workspace-v0166-exact.log": "326ce77b28d285d8dfabfadba54fdecff3748e17b094fa5a4ab8d927298d4a6f",
    "gpu-v0166-exact.log": "6eb001738b21105ce65e9925071d91b461eb2341fb64a4fb078602a373e4d8a8",
    "gpu-v0166-supplemental.log": "fd64c7561f73aabe6477b74a2704eeb55ffedf7639d67a3e15379f0921480872",
}
STOCK_PINS = (
    "browser-source.sha256", "browser-binary-sha256.txt", "native-binaries.sha256", "release-pin.json",
    "source-origins.json", "build-markers.json", "sealed-inputs.sha256", "inputs-before.json",
    "approved-runner.sha256", "focused-proof.json",
)
RETAINED = (
    "browser-ime.json", "browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json",
    "browser-smoke.json", "browser-material-side-sheet.json", "bridge-web-result.json",
    "devtools-headless-result.json", "material-standard-slider.json", "material-slider-forms.json",
    "material-range-slider.json", "material-segmented.json", "material-buttons.json",
    "material-vertical-centered-slider.json", "material-slider-inset.json",
)
PIXEL_MODULES = {
    "text_field_counter": ("verify_counter.py", "material-text-field-counter.json"),
    "text_field_adornments": ("verify_adorned_field.py", "material-text-field-adornments.json"),
    "filled_text_field": ("verify_filled_field.py", "material-filled-text-field.json"),
    "outlined_text_field": ("verify_outlined_field.py", "material-outlined-text-field.json"),
}
HEX = re.compile(r"[a-f0-9]{64}")
NATIVE_IMAGES = {
    f"{scope}-{kind}-r1.png": (width, 900)
    for scope, width in (("wide", 960), ("compact", 320)) for kind in ("outlined", "filled")
}


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


def copy(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)


def compressed(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(gzip.compress(source.read_bytes(), mtime=0))


def copy_members(source, target, names):
    for name in sorted(set(names)):
        copy(source / name, target / name)


def source_info():
    before = path_entries(BASE / "exact-candidate/source.sha256")
    after = path_entries(EXACT / "source.sha256")
    for area, entries, sha, tree in ((BASE / "exact-candidate", before, BASE_SOURCE, BASE_TREE),
                                     (EXACT, after, SOURCE, TREE)):
        source = area / "source"
        require(digest(area / "source.sha256") == sha and len(entries) == 681, "Wrong candidate source inventory")
        verify_entries(source, entries)
        actual = {str(file.relative_to(source)) for file in source.rglob("*")
                  if file.is_file() and file.relative_to(source).parts[0] != ".git"}
        require(actual == entries.keys() and source_tree(source, entries) == tree,
                "Exact source tree or path inventory differs")
    patch = STAGE / PATCH_NAME
    paths = sorted(re.findall(r"^diff --git a/(\S+) b/\S+$", patch.read_text(), re.M))
    require(digest(patch) == PATCH and (STAGE / "exact-apply-checked").is_file()
            and (STAGE / "source-tree").read_text().strip() == TREE
            and (STAGE / "prior-tree").read_text().strip() == BASE_TREE and len(paths) == 8
            and paths == sorted((STAGE / "owned-textbox-values.paths").read_text().splitlines())
            == sorted(path for path in before.keys() | after.keys() if before.get(path) != after.get(path)),
            "Wrong eight-path patch or clean reapply proof")
    source = EXACT / "source"
    require(tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]["version"] == "0.1.66",
            "Wrong workspace release")
    steps = (source / "e2e/src/material.rs").read_text().split("pub const STEPS:", 1)[1].split("];", 1)[0]
    require([int(n) for n in re.findall(r'"M(\d+)"', steps)] == [n for n in range(1, 347) if n != 23],
            "Final Material registry differs")
    for path, reverse in ((BASE / "exact-candidate/source", []), (source, ["--reverse"])):
        subprocess.run(["git", "-C", str(path), "apply", "--check", "--whitespace=error-all", *reverse, str(patch)],
                       check=True, env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"})
    return {"version": "0.1.66", "source_sha256": SOURCE, "source_files": 681, "tree": TREE,
            "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE, "patch_sha256": PATCH, "changed_paths": paths}


def rust_info(root, *, archived=False):
    captured = {}
    for name, expected in RUST_LOGS.items():
        path = root / (name + ".gz" if archived else name)
        require(sha256(raw_log(path)).hexdigest() == expected,
                f"Wrong exact Rust or GPU log bytes: {name}")
        captured[name] = tests(path)
    ((workspace, record), (primary_log, primary), (extra_log, extra)) = captured.values()
    tail = workspace.rsplit("\ntest result:", 1)[-1]
    require((record["passed"], record["ignored"]) == (1561, 20)
            and tail.count("Finished \x60dev\x60 profile") == 3
            and re.search(r"^provenance: .+ self-test cases passed\)", workspace, re.M)
            and "Doc-tests surface_material_gallery" in workspace
            and all(f"{name} ... ok" in workspace for name in ("multiline_text_field_steps_are_required_by_the_material_case",
                    "multiline_text_projects_value_description_and_focus_by_mode",
                    "text_field_counter_steps_are_required_by_the_material_case")),
            "Missing exact retained workspace, provenance or three Wasm checks")
    require(primary["passed"] == 16 and primary["ignored"] == 0 and "backend: Metal" in primary_log
            and 'name: "Apple M1 Max"' in primary_log, "Actual exact Apple Metal evidence is incomplete")
    extra_names = ("right_aligned_wrapped_selection_agrees_with_shaped_geometry",
                   "paired_selection_matches_single_draw_references_at_fractional_clip_edges")
    require(extra["passed"] == 2 and extra["ignored"] == 0 and extra_log.count("backend: Metal") == 2
            and all(f"test {name} ... ok" in extra_log for name in extra_names),
            "The two supplemental GPU cases do not prove Apple Metal")
    return {"workspace": {**record, "wasm_checks": 3}, "metal": {**primary, "backend": "Metal"},
            "supplemental": {**extra, "backend": "Metal", "tests": list(extra_names)}, "metal_total_passed": 18}


def decoded(path, size):
    with Image.open(path) as image:
        image.load()
        require(image.format == "PNG" and image.mode == "RGBA" and image.size == size, f"Wrong native raster {path.name}")
        return {"size": list(image.size), "png_sha256": digest(path), "rgba_sha256": sha256(image.tobytes()).hexdigest()}


def preview_info(current, before, preview_manifest, source_manifest, binary_manifest):
    require(digest(preview_manifest) == NATIVE_PREVIEW and digest(source_manifest) == NATIVE_SOURCE
            and digest(binary_manifest) == NATIVE_BIN, "Wrong sealed native development input")
    entries = path_entries(preview_manifest)
    require(entries.keys() == {*NATIVE_IMAGES, "preview.json"}, "Wrong native preview inventory")
    verify_entries(current, entries)
    raw = data(current / "preview.json")
    require(raw["transport_bytes_match"] is True and len(raw["profiles"]) == 2, "Wrong transport or preview profile count")
    shots = {}
    for profile in raw["profiles"]:
        require(profile["reaped"] is True and profile["samples"] == [0, 0, 0, 0]
                and profile["row_height"] == (24 if profile["profile"] == "wide" else 48)
                and profile["grow_after_enter"] == 2 * profile["row_height"], "Wrong preview rows or no-window proof")
        for record in profile["shots"]:
            name = record["name"]
            shot = decoded(current / name, NATIVE_IMAGES[name])
            require(shot["png_sha256"] == record["sha256"] and shot == decoded(before / name, NATIVE_IMAGES[name]),
                    "Native PNG or decoded pixels differ from actual v0.1.65 preview")
            shots[name] = shot
    require(shots.keys() == NATIVE_IMAGES.keys(), "Missing native originals")
    return {"scope": "native development preview and prior-pixel comparator", "originals": shots,
            "captured": 4, "matching_v0165": 4, "window_observations": 8,
            "visible_windows": 0, "reaped_processes": 2, "transport_parity": True, "binary_manifest_sha256": NATIVE_BIN}


def native_preflight():
    root = STAGE / "native-development-r1"
    seal(root / "binaries", "SHA256SUMS", NATIVE_BIN)
    verify_entries(root / "source", path_entries(root / "source.sha256"))
    return preview_info(root / "preview", BASE / "native-development-r1/preview", root / "preview.sha256",
                        root / "source.sha256", root / "binaries/SHA256SUMS")


def exact_native(directory, tools, before, receipt, terminal):
    seal(tools, "harness.sha256", NATIVE_REPLAY_HARNESS)
    current = seal(directory, "SHA256SUMS", NATIVE_EXACT, exact=True)
    require(digest(before) == NATIVE_PRIOR and digest(receipt) == NATIVE_EXACT_RESULT
            and terminal.read_text().startswith("2 out of 2 headless replays passed at "),
            "Wrong actual prior native manifest or terminal receipt")
    result = run_json([tools / "verify-stored-multiline.py", directory, "--manifest-sha", NATIVE_EXACT,
                       "--source-manifest-sha", SOURCE, "--binary-manifest-sha", NATIVE_BIN])
    require((result["repeats"], result["original_pngs"], result["visible_owned_windows"],
             result["observed_window_samples"], result["reaped_gallery_processes"]) == (2, 14, 0, 24, 4),
            "Optional exact native run lost a repetition, image or no-window sample")
    old_pngs = {name: sha for name, sha in path_entries(before).items() if name.endswith(".png")}
    new_pngs = {name: sha for name, sha in current.items() if name.endswith(".png")}
    require(result == data(receipt) and len(old_pngs) == 14 and new_pngs == old_pngs,
            "Exact native original PNGs differ from the sealed v0.1.65 run")
    return {**result, "matching_v0165": 14, "transports": ["Page.screenshot", "CLI", "MCP"]}


def pair_info(root):
    entries = seal(root, "SHA256SUMS", PAIR_SEAL)
    require(len(entries) == 21, "Wrong paired browser probe inventory")
    proof = run_json([root / "compare.py"])
    require(proof == data(root / "comparison.json") and proof["old_values_missing"] == proof["corrected_values_exact"] == 6
            and proof["adapter"] == ADAPTER and proof["zero_visible_windows"] and proof["observed_survivors"] == 0
            and all(report["window_samples"] == 9 for report in proof["reports"].values()),
            "Paired Chrome AX controls differ")
    for name in proof["reports"]:
        watch = data(root / name / WINDOW)
        clean_windows(watch)
        verify_ledgers(root / name, watch, {"chrome-primary"})
    return {**proof, "manifest_sha256": PAIR_SEAL}


def stock_tools(root):
    with import_from(root):
        observer = run_path(str(root / "observe-multiline.py"))
        textbox = run_path(str(root / "verify_textbox_values.py"))
        pixels = {key: run_path(str(root / module))["verify"] for key, (module, _) in PIXEL_MODULES.items()}
    require(observer["SOURCE_SHA"] == SOURCE and observer["SOURCE_FILES"] == 681
            and observer["SOURCE_TREE"] == TREE and observer["LAST_MATERIAL"] == 346,
            "Stock observer source pin differs")
    return observer, textbox, pixels


def focus_info(root, textbox):
    known = root / "verified-focused.json"
    require(digest(known) == FOCUSED_RECEIPT, "Wrong independent scoped positive report")
    verdict = data(known)
    run = root / "development-r1"
    invocation = data(run / "invocation.json")
    require(verdict["steps"] == [f"M{step}" for step in range(341, 347)] and verdict["checks"] == 308
            and verdict["adapter"] == ADAPTER and verdict["window_samples"] == 65
            and verdict["visible_windows"] == verdict["survivors"] == 0
            and invocation["argv"][-1] == "material" and invocation["runner_sha256"] == verdict["runner_sha256"],
            "Wrong focused development scope")
    log = (run / "runner.log").read_text()
    require(re.findall(r"^  \[(M\d+)\]", log, re.M) == verdict["steps"]
            and re.findall(r"^e2e result: (.*?); artifacts", log, re.M) == ["1 passed; 0 failed; 308 checks"],
            "Wrong original scoped Rust runner log")
    observation = data(run / WINDOW)
    clean_windows(observation)
    verify_ledgers(run, observation, {"chrome-primary"})
    require(digest(run / WINDOW) == verdict["window_receipt_sha256"], "Wrong focus guard receipt seal")
    meta = {key: verdict[key] for key in ("browser", "adapter")}
    retained, actual = textbox["verify"](run / "artifacts", meta)
    claim = normalized(retained)
    del claim["report_sha256"]
    require(claim == verdict["retained_m341_to_m345"] and len(claim["pngs"]) == 12
            and actual["inactive_ax"] == verdict["inactive_ax"] and actual["report_sha256"] == verdict["report_sha256"]
            and len(actual["inactive_ax"]) == 6, "Focused Chrome original pixel or AX replay differs")
    return {**verdict, "gallery_files": invocation["web_files"], "active_handoff": actual["active_handoff"]}


def stock_info(run, pins, observer, textbox, pixels, focus):
    verdict, offline = data(run / STOCK_RESULT), data(pins / OFFLINE)
    require(all(offline[key] == value for key, value in verdict.items()), "Actual and offline stock reports disagree")
    watch = data(run / WINDOW)
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary", "chrome-bridge", "native"})
    require(watch["launch_observations"]["chrome-bridge"]["detached_samples"] > 0, "Detached Bridge Chrome was not observed")
    log = raw_log(run / ("runner.log" if (run / "runner.log").is_file() else "runner.log.gz"))
    cases, checks = observer["verify_runner_log"](log.decode(), 346)
    require(cases == verdict["cases"] and checks == verdict["total_checks"] and cases["material"]["steps"] == 345
            and verdict["page_errors"] == 0 and offline["window_receipt_sha256"] == digest(run / WINDOW)
            and offline["runner_log_sha256"] == sha256(log).hexdigest(), "Wrong complete stock scope or original log")
    before, after = data(pins / "inputs-before.json"), data(run / "inputs-after.json")
    launch = data(run / "invocation.json")
    binaries = path_entries(pins / "browser-binary-sha256.txt")
    runner = (pins / "approved-runner.sha256").read_text().strip()
    expected = {"version": "0.1.66", "source_sha256": SOURCE, "source_tree": TREE, "source_files": 681,
                "last_material_step": 346, "wide_canvas_height": 800, "rtl_canvas_height": 1020}
    require(before == after == launch["inputs"] and tuple(launch["suite_arguments"]) == SUITES
            and launch["injected_chrome_flags"] == [] and after["release"] == data(pins / "release-pin.json")
            == offline["release"] == expected
            and after["source_manifest_sha256"] == digest(pins / "browser-source.sha256") == SOURCE
            and after["source_files_checked_per_origin"] == 681 and after["stock_files"] == binaries
            and after["bundle_files_checked"] == len(binaries) == 16
            and HEX.fullmatch(runner) and runner == binaries["debug/surface-e2e"] == after["runner_sha256"]
            and digest(pins / "native-binaries.sha256") == NATIVE_BIN,
            "Wrong exact stock source, executable bundle or arguments")
    seal(pins, "sealed-inputs.sha256", after["app_bundle_seal_sha256"])
    gallery = {PurePosixPath(name).name: sha for name, sha in binaries.items()
               if PurePosixPath(name).parent == PurePosixPath("web/release/material-gallery")}
    bound = data(pins / "focused-proof.json")
    require(offline["focused"] == bound and after["focused_proof_sha256"] == digest(pins / "focused-proof.json")
            and bound["canonical_gallery_match"] is True and gallery == focus["gallery_files"] == bound["gallery_files"]
            and bound["focused_receipt_sha256"] == FOCUSED_RECEIPT
            and bound["exact_source_sha256"] == SOURCE and bound["checks"] == focus["checks"] == 308
            and bound["inactive_ax"] == focus["inactive_ax"] and len(gallery) == 4,
            "Scoped receipt differs from the actual four canonical Gallery assets")
    artifacts = run / "artifacts"
    require(verdict["adapter"] == ADAPTER and verdict["guard_and_window_checks"] == "passed",
            "Stock Chrome lacks actual Metal or guard proof")
    meta = {"browser": verdict["browser"], "adapter": ADAPTER}
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
            "Retained stock suites lack their raw result")
    select_all = data(artifacts / "browser-ime-cancel-redraw.json")
    selected = data(artifacts / "browser-ime-cancel-selected-redraw.json")
    require(select_all["before"]["browser"] == select_all["after"]["browser"] == [0, 2]
            and select_all["before"]["trustedSelectAll"] is True
            and select_all["after"]["frames"] > select_all["before"]["frames"]
            and select_all["replacement"] == ["中👨‍👩‍👧‍👦é", "中👨‍👩‍👧‍👦é", "31", "31", "false"]
            and selected["canceled"]["direction"] == "backward" and selected["moved"]["trustedRight"] is True
            and selected["unpainted"]["value"] == selected["repainted"]["value"] == "台文X"
            and selected["unpainted"]["frames"] == selected["canceled"]["frames"] < selected["repainted"]["frames"],
            "Retained I11 case is absent")
    multiline, values = textbox["verify"](artifacts, meta)
    editors = {key: normalized(tool(artifacts, meta)) for key, tool in pixels.items()}
    require(normalized(multiline) == verdict["multiline_text_field"] and len(multiline["pngs"]) == 12
            and values == verdict["textbox_values"] and values["inactive_ax"] == focus["inactive_ax"]
            and all(verdict[key] == value for key, value in editors.items()),
            "Original stock textbox, multiline or retained editor proof differs")
    return {**meta, "cases": cases, "total_checks": checks, "page_errors": 0, "visible_windows": 0,
            "surviving_processes": 0, "runner_sha256": runner, "offline_receipt_sha256": digest(pins / OFFLINE),
            "gallery_web_files": gallery, "retained_reports": retained, "textbox_values": values,
            "multiline_text_field": normalized(multiline), **editors}


def preflight(gate):
    seal(STOCK, "harness.sha256", STOCK_HARNESS)
    _, textbox, _ = stock_tools(STOCK)
    focused = focus_info(FOCUSED, textbox)
    actual = textbox["focused_receipt"](EXACT / "source.sha256")
    require(focused["gallery_files"] == actual["gallery_files"] and focused["checks"] == actual["checks"],
            "Replayed original focused result differs from its frozen binding")
    result = {"status": "preflight only", "source": gate, "focused": focused,
              "native_preview": native_preflight(), "paired_browser_probe": pair_info(PAIR)}
    if (STAGE / "native-v0166-passed").is_file():
        result["native"] = exact_native(STAGE / "native-exact-replay", NATIVE_HARNESS, NATIVE_HARNESS / "exact-1/SHA256SUMS",
                                        STAGE / "native-v0166-verified.json", STAGE / "native-v0166-exact.log")
    if (STAGE / "workspace-v0166-passed").is_file() and (STAGE / "gpu-v0166-passed").is_file():
        result["rust"] = rust_info(STAGE)
    paths = (STAGE / "workspace-v0166-passed", STAGE / "gpu-v0166-passed", STAGE / "native-v0166-passed",
             EXACT / "native-final-built", EXACT / "stock-canonical-built", STOCK / "stock-mac-v0166-passed")
    return {**result, "pending_markers": [str(path) for path in paths if not path.is_file()]}


def archive_native(root):
    preview = STAGE / "native-development-r1"
    copy_members(preview / "preview", root / "preview/current", (*NATIVE_IMAGES, "preview.json"))
    copy_members(BASE / "native-development-r1/preview", root / "preview/v0165", NATIVE_IMAGES)
    copy_members(preview, root / "preview", ("source.sha256", "preview.sha256"))
    copy(preview / "binaries/SHA256SUMS", root / "preview/native-binaries.sha256")
    copy(BASE / "preview-multiline-native.py", root / "preview/preview-multiline-native.py")
    copy(STAGE / "compare-native-preview.sh", root / "preview/compare-native-preview.sh")
    exact = STAGE / "native-exact-replay"
    copy_members(exact, root / "exact", (*path_entries(exact / "SHA256SUMS"), "SHA256SUMS"))
    copy_members(NATIVE_HARNESS, root / "harness", (*path_entries(NATIVE_HARNESS / "harness.sha256"), "harness.sha256"))
    copy(NATIVE_HARNESS / "exact-1/SHA256SUMS", root / "v0165-captures.sha256")
    copy(EXACT / "binaries/SHA256SUMS", root / "exact-native-binaries.sha256")
    copy_members(STAGE, root, ("native-v0166-exact.log", "native-v0166-verified.json"))


def archive_focused(root, focus):
    copy_members(FOCUSED, root, ("README.md", "verified-focused.json", "q66-source.sha256", "focused-source.sha256",
                                "scope-selector.patch", "runner.sha256", "web.sha256", "verify-focused.py"))
    live = FOCUSED / "development-r1"
    files = (WINDOW, "invocation.json", "runner.log", f"artifacts/{FEATURE}",
             *(f'artifacts/{name}' for name in focus["retained_m341_to_m345"]["pngs"]),
             *(str(path.relative_to(live)) for path in (live / "guard-ledger").glob("*.json")))
    copy_members(live, root / "development-r1", files)


def archive_stock(root, stock):
    live = STOCK / "run"
    harness = seal(STOCK, "harness.sha256", STOCK_HARNESS)
    inputs = path_entries(STOCK / "sealed-inputs.sha256")
    copy_members(STOCK, root, (*STOCK_PINS, *harness, *inputs, "harness.sha256", OFFLINE))
    copy_members(live, root, (WINDOW, STOCK_RESULT, "invocation.json", "inputs-after.json",
                              *(str(path.relative_to(live)) for path in (live / "guard-ledger").glob("*.json"))))
    compressed(live / "runner.log", root / "runner.log.gz")
    files = (*RETAINED, FEATURE, *stock["multiline_text_field"]["pngs"],
             *(name for key, (_, report) in PIXEL_MODULES.items() for name in (report, *stock[key]["pngs"])))
    copy_members(live / "artifacts", root / "artifacts", files)


def export(root, receipt):
    require(root != Path("/tmp").resolve() and root.is_relative_to(Path("/tmp").resolve()),
            "Use a fresh private /tmp package directory")
    root.mkdir(mode=0o700)
    copy(EXACT / "source.sha256", root / "source.sha256")
    copy(BASE / "exact-candidate/source.sha256", root / "base-source.sha256")
    copy(STAGE / "owned-textbox-values.paths", root / "owned-textbox-values.paths")
    compressed(STAGE / PATCH_NAME, root / f"{PATCH_NAME}.gz")
    copy(Path(__file__), root / "pack-textbox-values.py")
    for name in RUST_LOGS:
        compressed(STAGE / name, root / "rust" / f"{name}.gz")
    archive_native(root / "native")
    archive_focused(root / "focused", receipt["focused"])
    copy_members(PAIR, root / "paired-browser-probe", (*path_entries(PAIR / "SHA256SUMS"), "SHA256SUMS"))
    archive_stock(root / "stock-macos", receipt["stock_macos"])
    json_file(root / REPORT, receipt)
    (root / "README.md").write_text(f"""# Surface v0.1.66 textbox-value candidate proof

This archive records local verification of the exact candidate tree
{TREE}, with 681 source files and an eight-path patch.

- The exact workspace passed 1,561 Rust tests and three WebAssembly
  checks. The GPU gate ran 18 tests on Apple Metal.
- The exact native Gallery replay returned 14 original PNGs in two
  runs. All match the prior v0.1.65 captures. Page.screenshot, CLI and
  MCP matched. The guard saw zero visible windows across 24 samples.
  The separate native development preview supplies four matching PNGs.
- The paired Chrome probe found the six inactive values missing in
  v0.1.65. The same six values match exactly after the v0.1.66 change,
  including whitespace, a combining accent and a terminal newline.
- The focused run passed 308 checks from M341 through M346. It keeps
  the 12 original PNGs, six inactive accessibility values, and the
  active textarea handoff. All four Gallery Web files match stock.
- The exact stock run passed {receipt['stock_macos']['total_checks']:,} checks in five
  headless macOS suites. It ran 345 Material steps through M346, the
  retained I11 regressions, and verified the six inactive values
  including the single-line field. The guard found zero visible
  windows and no surviving processes.

The accessibility receipts cover the Chromium tree. Spoken output
requires a separate screen-reader test.

## Offline recheck

Use Python 3.12 with Pillow. Supply the reviewed manifest digest.

~~~sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-textbox-values.py --verify-package . --package-sha "$manifest_sha"
~~~

The gzip files reproduce the original patch and log bytes. This
command reads the original evidence and starts no application.
""")
    files = sorted(file for file in root.rglob("*") if file.is_file())
    (root / "SHA256SUMS").write_text("".join(f"{digest(file)}  {file.relative_to(root)}\n" for file in files))
    return digest(root / "SHA256SUMS")


def verify_package(root, checksum):
    seal(root, "SHA256SUMS", checksum, exact=True)
    receipt = data(root / REPORT)
    source = receipt["source"]
    patch = raw_log(root / f"{PATCH_NAME}.gz")
    patch_paths = sorted(re.findall(r"^diff --git a/(\S+) b/\S+$", patch.decode(), re.M))
    require(receipt["status"] == "verified" and source["version"] == "0.1.66" and source["tree"] == TREE
            and source["source_sha256"] == SOURCE and source["patch_sha256"] == PATCH and source["base_tree"] == BASE_TREE
            and source["source_files"] == 681 and source["base_source_sha256"] == BASE_SOURCE
            and digest(root / "source.sha256") == SOURCE and len(path_entries(root / "source.sha256")) == 681
            and digest(root / "base-source.sha256") == BASE_SOURCE and len(path_entries(root / "base-source.sha256")) == 681
            and sha256(patch).hexdigest() == PATCH
            and source["changed_paths"] == sorted((root / "owned-textbox-values.paths").read_text().splitlines())
            == patch_paths and len(patch_paths) == 8, "Archived exact source or patch differs")
    stock = root / "stock-macos"
    require(receipt["stock_harness_sha256"] == STOCK_HARNESS, "Exact stock harness receipt changed")
    seal(stock, "harness.sha256", STOCK_HARNESS)
    observer, textbox, pixels = stock_tools(stock)
    focus = focus_info(root / "focused", textbox)
    require(focus == receipt["focused"] and stock_info(stock, stock, observer, textbox, pixels, focus)
            == receipt["stock_macos"], "Archived stock or focused proof differs")
    module_path = "e2e/src/material_multiline_text_field.rs"
    require(path_entries(root / "focused/focused-source.sha256")[module_path]
            == path_entries(root / "source.sha256")[module_path],
            "Archived focused AX test differs from the exact stock module")
    pair_root = root / "paired-browser-probe"
    require(pair_info(pair_root) == receipt["paired_browser_probe"]
            and path_entries(pair_root / "v0166-r2-web.sha256") == focus["gallery_files"],
            "The paired probe lost original data or canonical Web parity")
    native = root / "native"
    preview = native / "preview"
    require(preview_info(preview / "current", preview / "v0165", preview / "preview.sha256", preview / "source.sha256",
                         preview / "native-binaries.sha256") == receipt["native_preview"]
            and digest(native / "exact-native-binaries.sha256") == NATIVE_BIN
            and exact_native(native / "exact", native / "harness", native / "v0165-captures.sha256",
                             native / "native-v0166-verified.json", native / "native-v0166-exact.log") == receipt["native"],
            "Archived native raster evidence differs")
    require(rust_info(root / "rust", archived=True) == receipt["rust"], "Archived Rust and actual GPU receipts differ")
    return receipt


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
        require(args.source_sha == SOURCE and args.tree == TREE, "Supply the reviewed exact candidate tree and source seal")
        source = source_info()
        result = source if args.check_source else preflight(source)
        if args.out:
            require(not result["pending_markers"], "Pending exact gates:\n" + "\n".join(result["pending_markers"]))
            require(args.stock_harness_sha == STOCK_HARNESS and HEX.fullmatch(args.stock_offline_sha or "")
                    and HEX.fullmatch(args.stock_runner_sha or "") and digest(STOCK / OFFLINE) == args.stock_offline_sha
                    and (STOCK / "approved-runner.sha256").read_text().strip() == args.stock_runner_sha,
                    "Exact stock observer, receipt or runner digests differ")
            live = run_json([STOCK / "verify-stock-receipt.py", STOCK / "run"])
            require(live == data(STOCK / OFFLINE), "Actual stock replay differs from the saved offline verdict")
            observer, textbox, pixels = stock_tools(STOCK)
            stock = stock_info(STOCK / "run", STOCK, observer, textbox, pixels, result["focused"])
            result = {name: value for name, value in result.items() if name != "pending_markers"}
            result.update({"status": "verified", "stock_harness_sha256": STOCK_HARNESS, "stock_macos": stock})
            root = args.out.resolve()
            marker = root.with_name(root.name + "-verified.json")
            require(not marker.exists(), "Use a fresh external verification marker")
            checksum = export(root, result)
            require(verify_package(root, checksum) == result, "Fresh archive recheck differs")
            output = {"path": str(root), "manifest_sha256": checksum, "files": len(path_entries(root / "SHA256SUMS"))}
            json_file(marker, {**output, "source_sha256": SOURCE, "tree": TREE, "runner_sha256": args.stock_runner_sha,
                               "stock_offline_sha256": args.stock_offline_sha, "report_sha256": digest(root / REPORT)})
            result = {**result, "output": output, "marker": str(marker)}
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, RuntimeError, KeyError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(f"Textbox-value evidence stopped. {error}")
