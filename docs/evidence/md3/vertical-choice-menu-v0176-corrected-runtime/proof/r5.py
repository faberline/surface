"""Read-only checks for the R4 Chrome failure and the test-only R5 replay."""

from pathlib import Path
import re
from runpy import run_path
import shutil

from PIL import Image

from evidence_common import (
    clean_windows, copy_members, data, digest, import_from, path_entries, require, seal,
    source_tree, verify_ledgers,
)

R5 = Path("/tmp/surface-md3-choice-menu-v0176-browser-corner.3ixT0mHe")
R5_TOOLS = Path("/tmp/surface-md3-choice-menu-v0176-r5-guard.7wvCho7W")
R5_GUARD_SHA = "bebaa53d211e87e73ea0d4b1dc232034be8ff5e5f299aea5581c9e584a92b0db"
TREE = "b6dc18c4baf8e6b65bddfbc6f59b7b07b4a0b5b9"
SOURCE_SHA = "28c5bf44e9618f47ab4b541851d50c0d1d3c7df4748aac5518103d9f24e92893"
PATCH_SHA = "4239e9da0541b1794b4fc15f26ddef035d97d0a6f0f35572de5cccb0039c7b39"
TEST_PATH = "e2e/src/material_choice_menu.rs"
PATCH = "rtl-corner-probe.patch"
OBSERVER = "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7"
WEB_SHA = "3c471440987193713a55394e334f0b8a5078da7bad8dc98016c9dca1985d9ec4"
RUNNER_MANIFEST = "da2f8b4115489f006de691c2fe4b7c934f3dc7dd2c2c699a1bc109cbbbe96fa0"
RUNNER_LOG = "e4ea67c2cb91957c7b7f79ed3e2cfd715f4b5dee7a11f32c00ad1cfe1ff71026"
RUNNER = "cd10e21a67723e8f07844d5a5d861e17845a5b4486e5b4e7f5e6c37108d19e3a"
R4_RUNNER = "94e09c6ecebab8458fa796a4ec72946c2f9eab5ecd6ed820ef2422b7d18c0868"
FAILURE = "M375: the selected corner leaves the actual group paint visible: expected [99, 59, 72], got [255, 217, 227]"
STEP_IDS = [str(step) for step in range(1, 376) if step != 23]
REPORT = "browser-material-choice-menu.json"
COMPACT = "material-choice-menu-compact-dark-rtl.png"
FAILURE_PNG = "material-choice-menu-failure.png"
CROP = "compact-corner-and-scroll-thumb-crop.png"
CROP_BOX = (104, 496, 584, 522)
TERMINAL = "verified-stock-material.json"
WINDOW = "headless-window-observation.json"

R4_FILES = {
    "runner.log": "8fc5fcbd61a456c36af031ff9023a4c44a85c91c00c717492666d40f5993e18c",
    "invocation.json": "e8f9ed5dd20ca5aeebb49cb39431f5fc9128c88c9d9fa3f8983521230dc30ba8",
    WINDOW: "490e2cc6d38da454c6f01877fe29e865a6978daca0332375f2ea4a2ee2107bad",
    f"artifacts/{REPORT}": "3264433f803bfe3f20cea000ffe10f5e0b95172afda5d1afb2da08a4234fd10d",
    f"artifacts/{COMPACT}": "3ece7fa6fb0eb1c39b4cedd08bac7ee704460f6ee3e7bbe00c18bb0da7eacd06",
}
PIXELS = {
    "old_left_corner": ((114, 506), [255, 217, 227]),
    "mirrored_right_corner": ((574, 506), [99, 59, 72]),
    "mirrored_right_interior": ((564, 510), [239, 184, 200]),
    "selected_center": ((344, 512), [239, 184, 200]),
}
DISABLED = {
    "open": True,
    "focus": "menu-demo-choice-owner",
    "sort": "menu-demo-choice-recent",
    "density": "menu-demo-choice-comfortable",
    "checked": ["menu-demo-choice-kind", "menu-demo-choice-locked"],
    "changes": 0,
}


def source_info(root, r4_source_manifest, *, original=False):
    require((root / "source.tree").read_text() == TREE + "\n"
            and digest(root / "source.sha256") == SOURCE_SHA
            and digest(root / PATCH) == PATCH_SHA, "Wrong R5 tree, manifest or patch")
    before, after = path_entries(r4_source_manifest), path_entries(root / "source.sha256")
    require(len(after) == 703 and before.keys() == after.keys()
            and [name for name in after if before[name] != after[name]] == [TEST_PATH],
            "R5 must differ only in the menu E2E probe")
    if original:
        source = root / "source"
        require({path.relative_to(source).as_posix() for path in source.rglob("*") if path.is_file()}
                == after.keys() and source_tree(source, after) == TREE,
                "Live R5 tree or source inventory differs")
    else:
        require(digest(root / "before.rs") == before[TEST_PATH]
                and digest(root / "after.rs") == after[TEST_PATH],
                "Preserved original and corrected E2E source bytes differ")
    return {"tree": TREE, "source_sha256": SOURCE_SHA, "source_files": len(after),
            "only_changed_path": TEST_PATH, "test_only_patch_sha256": PATCH_SHA,
            "unchanged_paths": len(after) - 1}


def input_pins(root, proof, build, r4_source, *, original=False):
    runner = root / "runner"
    manifest = runner / "SHA256SUMS"
    binaries = path_entries(manifest)
    require(digest(manifest) == RUNNER_MANIFEST and binaries == {"surface-e2e": RUNNER}
            and RUNNER != R4_RUNNER and (root / "runner-built.tree").read_text() == TREE + "\n",
            "Expected the freshly built R5 test runner")
    log = root / "runner-build.log"
    require(digest(log) == RUNNER_LOG and re.search(r"Finished `dev` profile", log.read_text()),
            "Missing exact R5 Cargo build completion")
    if original:
        require(digest(runner / "surface-e2e") == binaries["surface-e2e"], "Live R5 runner bytes differ")
    web_manifest = build / "web/SHA256SUMS"
    require(digest(web_manifest) == WEB_SHA, "The R5 browser must use the verified R4 Web build")
    return {
        "source_tree": proof["tree"],
        "source_sha256": proof["source_sha256"],
        "web_producer_tree": r4_source["tree"],
        "web_producer_source_sha256": r4_source["source_sha256"],
        "web_manifest_sha256": WEB_SHA,
        "test_only_path": proof["only_changed_path"],
        "test_only_patch_sha256": proof["test_only_patch_sha256"],
        "runner_sha256": binaries["surface-e2e"],
        "runner_manifest_sha256": digest(manifest),
        "runner_build_log_sha256": digest(log),
        "observer_sha256": OBSERVER,
    }


def failure_info(root, build, r4_source, *, original=False):
    for name, expected in R4_FILES.items():
        require(digest(root / name) == expected, f"Wrong original R4 browser failure input: {name}")
    same = R4_FILES[f"artifacts/{COMPACT}"]
    if original:
        require(digest(root / "artifacts" / FAILURE_PNG) == same,
                "Original failure and compact screenshots differ")
    invoke = data(root / "invocation.json")
    pins = {"source_tree": r4_source["tree"], "source_sha256": r4_source["source_sha256"],
            "runner_sha256": R4_RUNNER, "web_manifest_sha256": WEB_SHA, "observer_sha256": OBSERVER}
    require(invoke["inputs"] == data(root / "inputs-after.json") == pins
            and path_entries(build / "runners/SHA256SUMS").get("surface-e2e") == R4_RUNNER,
            "R4 failure belongs to different build pins")
    checks = (root / "runner.log").read_text()
    require(re.findall(r"^  \[M(\d+)\]", checks, re.M) == STEP_IDS and f"1 failed; 30898 checks;" in checks
            and "last step: M375" in checks and FAILURE in checks,
            "R4 stock failure receipt has wrong terminal status or steps")
    window = data(root / WINDOW)
    require(window["stock_exit"] == 1 and not window["problems"]
            and not window["visible_owned_windows"] and not window["remaining_observed_processes"],
            "R4 failure did not shut down cleanly")
    verify_ledgers(root, window, {"chrome-primary"})
    report = data(root / "artifacts" / REPORT)
    require(report["failure"] == FAILURE and report["errors"] == [] and "compact" not in report
            and report["standard"]["disabled_mouse"] == report["standard"]["disabled"] == DISABLED
            and isinstance(report["vibrant"], dict), "Wrong R4 partial menu evidence")
    with Image.open(root / "artifacts" / COMPACT) as original_image:
        image = original_image.convert("RGB")
        require(image.size == (640, 1048), "Wrong R4 compact PNG size")
        for name, (point, rgb) in PIXELS.items():
            require(list(image.getpixel(point)) == rgb, f"Wrong measured R4 diagnostic pixel: {name}")
        if not original:
            with Image.open(root / CROP) as crop:
                require(crop.convert("RGB").tobytes() == image.crop(CROP_BOX).tobytes()
                        and crop.size == (480, 26), "Derived crop differs from the original pixels")
    return {"status": "failed at M375 pixel probe", "assertions_before_stop": 30898,
            "m373_disabled_mouse": DISABLED, "m374_completed": True,
            "visible_windows": 0, "surviving_processes": 0, "original_png_sha256": same,
            "crop_box": list(CROP_BOX),
            "samples": {name: {"pixel": list(point), "rgb": rgb} for name, (point, rgb) in PIXELS.items()}}


def stage_failure(source, dest):
    names = (*R4_FILES, "inputs-after.json")
    ledger = tuple(path.relative_to(source).as_posix() for path in (source / "guard-ledger").glob("*.json"))
    copy_members(source, dest, (*names, *ledger))
    with Image.open(dest / "artifacts" / COMPACT) as image:
        image.convert("RGB").crop(CROP_BOX).save(dest / CROP)


def stage_source(source, r4_source, dest):
    copy_members(source, dest, ("source.tree", "source.sha256", PATCH, "fmt.log", "runner-build.log",
                                 "runner-built.tree", "runner/SHA256SUMS"))
    shutil.copyfile(r4_source / TEST_PATH, dest / "before.rs")
    shutil.copyfile(source / "source" / TEST_PATH, dest / "after.rs")


def guard_members(tools):
    return seal(tools, "SHA256SUMS", R5_GUARD_SHA)


def check_m375(report):
    compact = report["compact"]
    paint = compact["paint"]
    require(paint["id"] == "menu-demo-choice-size" and paint["corners"] == [12.0] * 4
            and paint["corner_rgb"] == [99, 59, 72]
            and paint["fill"] == paint["inner_rgb"] == [239, 184, 200]
            and paint["group_rgb"] == [99, 59, 72] and paint["mark_pixels"] > 8
            and paint["face_contrast"] >= 3 and compact["scroll"][1] > compact["scroll"][0],
            "M375 did not prove the dark RTL rounded mark and real scrolling")
    require(compact["dismissed"]["open"] is False
            and compact["dismissed"]["checked"] == ["menu-demo-choice-kind", "menu-demo-choice-locked"]
            and compact["toggled"]["open"] is True
            and compact["toggled"]["checked"] == compact["dismissed"]["checked"],
            "M375 did not retain the expected checked values on Escape")


def browser_info(root, build, tools, pins, *, original=False):
    invocation = data(root / "invocation.json")
    require(invocation["inputs"] == data(root / "inputs-after.json") == pins,
            "Browser did not run the exact R5 test runner with the R4 Web producer")
    argv = invocation["argv"]
    require(len(argv) == 10 and argv[1] == "--chrome"
            and argv[3:6] == ["--chrome-arg", "--disable-updater-scheduler", "--web-root"]
            and argv[7] == "--artifacts" and argv[9] == "material",
            "Expected the guarded stock Material invocation")
    if original:
        require(tuple(Path(argv[i]).resolve() for i in (0, 2, 6, 8)) ==
                ((R5 / "runner/surface-e2e").resolve(), root / "guards/chrome",
                 build / "web", root / "artifacts"), "R5 browser paths bypassed the new runner or R4 Web bytes")
    watched = data(root / WINDOW)
    clean_windows(watched)
    verify_ledgers(root, watched, {"chrome-primary"})
    guard_members(tools)
    with import_from(tools):
        guard = run_path(str(tools / "run-stock-material.py"))
    result = guard["verify_run"](root, watched)
    require(result == data(root / TERMINAL) and result["steps"] == 374
            and len(guard["MENU_REPORTS"]) == 3 and len(guard["MENU_IMAGES"]) == 9
            and set(result["menus"]) == set(guard["MENU_REPORTS"] + guard["MENU_IMAGES"]),
            "Expected genuine R5 terminal proof and all original menu artifacts")
    for name in guard["MENU_IMAGES"]:
        with Image.open(root / "artifacts" / name) as image:
            image.load()
            require(image.format == "PNG" and list(image.size) == result["menus"][name]["size"],
                    f"Original Chrome PNG differs: {name}")
    reports = [data(root / "artifacts" / name) for name in guard["MENU_REPORTS"]]
    adapters = [report["adapter"] for report in reports]
    require(all(adapter == adapters[0] and adapter["isFallbackAdapter"] is False for adapter in adapters),
            "R5 lacks a shared nonfallback WebGPU adapter")
    report = data(root / "artifacts" / REPORT)
    proof = result["menus"][REPORT]
    require(report["standard"]["disabled_mouse"] == report["standard"]["disabled"] == DISABLED
            and proof["m373_disabled_mouse"] == DISABLED,
            "M373 did not prove disabled pointer focus and unchanged values")
    check_m375(report)
    require(proof["m375_rtl_leading_corner"]["focus_after_escape"] == "menu-demo-opener",
            "M375 did not restore focus to the opener")
    return {**result, "adapter": adapters[0], "visible_windows": 0, "surviving_processes": 0}
