"""Keep q68 R2 failure and join the R3 test-only runner to the R2 product."""

from pathlib import Path
import re
from runpy import run_path

from PIL import Image

from evidence_common import (
    clean_windows, copy, copy_members, data, digest, import_from, path_entries,
    require, run_json, seal, source_tree, verify_entries, verify_ledgers,
)

R3 = Path("/tmp/surface-md3-label-color-v0168-browser-clock.ectN4rAV")
DIAGNOSTIC = Path("/tmp/surface-md3-label-color-v0168-diagnostic.ADfuCWLi")
DIAGNOSTIC_SHA = "4fe0ef3446420e4dc458432fb5a317dd74ca40ab0a337f6c1cee72e1c0f4bb35"
COMMIT = "26758a7313527c9aab7c2d5e677ec880866638da"
TREE = "9d3ce235f8efde1459fd41862cd918367fc85207"
SOURCE = "4bced93c36065b03879ea12060d36d0ce7eb88ebfa43ce4a139fc28200219bd8"
R2_COMMIT = "e0b2a032f46a397265fee70136b471fbb5bcfd57"
R2_TREE = "72ae6c3eb56e9554532f428b3308aadd8358a242"
R2_SOURCE = "086f06d98a7c2f3728b67c9524b15ed8b380fb078d42c48bb80b1f082c24580c"
STOCK_SHA = "bcf203bdc573567a3cfa66919491d5e4ae849b3f5f881f16e48e41a8d1d0df46"
NATIVE_SHA = "4e68692d2b6de8ed7c854b5a427dcb45b264334909d819b14bdff162b3d5f4f5"
PATCH = "settle-compact-focus.patch"
PATCH_SHA = "de0899c86af932cbf6c12ed9c7ee30d7d4454856a502e83f7b3cc345de50a84b"
TEST_PATH = "e2e/src/material_text_field_label_motion.rs"
RUNNER_SHA = "915e4ea70968e34f8724e80341cb69f5b679d6d7795c0f66d13a9d6f1ce61096"
RUNNER_MANIFEST_SHA = "3b61a424352e6be5f22675093b988720074b1e5a4a2afc0a9916a4b941178a81"
RUNNER_LOG_SHA = "70218a21feabebe34334f4c3d437e98cdb219efc214ec7bdec629266906effd4"
GUARD = Path("/tmp/surface-md3-label-color-v0168-r3-stock.DhTi2bwy")
HARNESS_SHA = "5e883b18779244e515fa1e07d619676982e5585471a7ace2cd669602389610ba"
FOCUSED_SHA = "78deb69187e673d0cd40d38859126141564c19d539ff88dcf89aa770dfbf4efb"
GUARD_SHA = "e8c6594e746be387ff27aa049ebaf8bbd97982d934375ba3453aeac3477b1c34"
TERMINAL = "verified-r3-stock.json"
WINDOW = "headless-window-observation.json"
LABEL_REPORT = "material-text-field-label-motion.json"
SUITES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
ROLES = {"chrome-primary", "chrome-bridge", "native"}
M350 = "the wrapped fallback changes only ink at the same glyph positions"
ABOVE = (80, 1544, 560, 1736)
FAILURE_FILES = {
    "runner.log": "e385e366a4c3be86bbdd90e599a1647c42f572dbddb82fe724d67c64fb80d157",
    "invocation.json": "c8650169a76f6804c54df135c942bb2223e1a4390f97865271414a34381071d8",
    "inputs-after.json": "590630991ba5fca4110bfb4b8e5d87dfdbd2ec6be07fff520884f664033c615b",
    WINDOW: "779f9f59e4ffaa5588c35f3a07b2e40776fce108af934c6eb04834c5e11cde66",
    f"artifacts/{LABEL_REPORT}": "2e9e82e0e27ecbc7231a89417ba6d62f26f2ec46ec748806d9237ecaaa850781",
}
R2_FRAMES = {
    "fresh-rest.png": "material-text-field-label-motion-rtl-above-label-rest.png",
    "fresh-middle.png": "material-text-field-label-motion-rtl-above-label-middle.png",
    "fresh-failure.png": "material-text-field-label-motion-failure.png",
}


def source_info(root, r2_manifest, *, original=False):
    expected = [COMMIT, TREE]
    require((root / "prepared.txt").read_text().splitlines() == expected
            and (root / "runner-built.txt").read_text().splitlines() == expected
            and digest(root / "source.sha256") == SOURCE and digest(root / PATCH) == PATCH_SHA
            and (root / "fmt.log").stat().st_size == 0,
            "Wrong R3 source, runner marker or test-only patch")
    before, after = path_entries(r2_manifest), path_entries(root / "source.sha256")
    require(digest(r2_manifest) == R2_SOURCE and len(after) == 685 and before.keys() == after.keys()
            and [name for name in after if before[name] != after[name]] == [TEST_PATH],
            "R3 must change only the compact label E2E fixture")
    if original:
        source = root / "surface"
        files = set()
        for folder, directories, names in source.walk():
            if folder == source and ".git" in directories:
                directories.remove(".git")
            files.update((folder / name).relative_to(source).as_posix() for name in names if name != ".git")
        verify_entries(source, after)
        require(files == after.keys() and source_tree(source, after) == TREE,
                "Live R3 bytes or inventory differ")
        require(digest(root / "runner/surface-e2e") == RUNNER_SHA, "Wrong built R3 runner bytes")
    else:
        require(digest(root / "before.rs") == before[TEST_PATH] and digest(root / "after.rs") == after[TEST_PATH],
                "Preserved R2 and R3 fixture source differs")
    require(path_entries(root / "runner/SHA256SUMS") == {"surface-e2e": RUNNER_SHA}
            and digest(root / "runner/SHA256SUMS") == RUNNER_MANIFEST_SHA
            and digest(root / "runner-build.log") == RUNNER_LOG_SHA
            and re.search(r"Finished `dev` profile", (root / "runner-build.log").read_text()),
            "Wrong R3 runner manifest or terminal build log")
    return {
        "commit": COMMIT, "tree": TREE, "manifest_sha256": SOURCE, "source_files": 685,
        "changed_path": TEST_PATH, "unchanged_paths": 684, "patch_sha256": PATCH_SHA,
        "runner_sha256": RUNNER_SHA, "runner_manifest_sha256": RUNNER_MANIFEST_SHA,
        "runner_build_log_sha256": RUNNER_LOG_SHA,
    }


def expected_product_files(build):
    stock, native = build / "stock/SHA256SUMS", build / "native/SHA256SUMS"
    require(digest(stock) == STOCK_SHA and digest(native) == NATIVE_SHA,
            "R3 must use the verified R2 native/Web bundle")
    return {**path_entries(stock), **{f"debug/{name}": checksum for name, checksum in path_entries(native).items()}}


def proof_via_stock_ledger(run, watch):
    verify_ledgers(run, watch, ROLES)
    args = data(run / "invocation.json")
    require(args["suite_arguments"] == list(SUITES)
            and args["injected_chrome_flags"] == ["--disable-updater-scheduler"],
            "Wrong stock suite or Chrome launch flag")


def failure_info(run, diagnostic, build, *, original=False):
    verify_entries(run, FAILURE_FILES)
    seal(diagnostic, "SHA256SUMS", DIAGNOSTIC_SHA)
    checked = data(diagnostic / "pixel-receipt.json")
    require(run_json([diagnostic / "verify.py"]) == checked, "Stored R2 pixels do not match the diagnostic")
    require(checked["same_glyph_positions"] == 12125
            and checked["observed_mid_colour"] == [202, 196, 207]
            and checked["historical_mid_colour"] == [206, 191, 243]
            and checked["former_readonly_field_focus_mid_pixels"] == 2289,
            "Wrong R2 label-clock diagnostic")
    if original:
        for saved, filename in R2_FRAMES.items():
            require(digest(diagnostic / saved) == digest(run / "artifacts" / filename),
                    f"Wrong original R2 failure PNG: {filename}")
    require(digest(diagnostic / "fresh-result.json") == digest(run / "artifacts" / LABEL_REPORT),
            "Wrong original R2 browser label report")

    invocation = data(run / "invocation.json")
    pins = data(run / "inputs-after.json")
    require(invocation["inputs"] == pins and pins["source_manifest_sha256"] == R2_SOURCE
            and pins["release"]["source_tree"] == R2_TREE
            and pins["stock_files"] == expected_product_files(build), "Wrong original R2 stock inputs")
    watch = data(run / WINDOW)
    require(watch["stock_exit"] == 1 and not watch["problems"]
            and not watch["visible_owned_windows"] and not watch["remaining_observed_processes"],
            "Wrong R2 failure termination")
    proof_via_stock_ledger(run, watch)
    log = (run / "runner.log").read_text()
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == list(SUITES)
            and re.findall(r"^  \[M(\d+)\]", log, re.M) == [str(i) for i in range(1, 351) if i != 23]
            and f"case material ... FAILED" in log and "last step: M350" in log and M350 in log
            and "e2e result: 4 passed; 1 failed; 25264 checks;" in log,
            "Wrong original R2 failure step or stock terminal counts")
    report = data(run / "artifacts" / LABEL_REPORT)
    require(report["failure"] == M350 and report["page_errors"] == [] and "compact" not in report,
            "Expected real R2 missing 44 ms color")
    if not original:
        for name in ("fresh-middle", "historical-middle"):
            with Image.open(diagnostic / f"{name}.png") as raw, Image.open(diagnostic / f"{name}-label-crop.png") as crop:
                expected = raw.convert("RGB").crop(ABOVE)
                require(expected.size == crop.size and expected.tobytes() == crop.convert("RGB").tobytes(),
                        "Lossless label crop differs from the original")
    return {"status": "failed at compact M350 color", "material_steps_reached": 349,
            "cases_passed": 4, "cases_failed": 1, "checks_before_stop": 25264,
            "left_visible_windows": 0, "left_owned_processes": 0, "pixels": checked}


def stage_failure(run, diagnostic, target):
    ledgers = [path.relative_to(run).as_posix() for path in (run / "guard-ledger").glob("*.json")]
    copy_members(run, target / "run", (*FAILURE_FILES, *ledgers))
    copy_members(diagnostic, target / "pixels", ("SHA256SUMS", *path_entries(diagnostic / "SHA256SUMS")))
    for name in ("fresh-middle", "historical-middle"):
        root = target / "pixels"
        with Image.open(root / f"{name}.png") as raw:
            raw.convert("RGB").crop(ABOVE).save(root / f"{name}-label-crop.png")


def stage_source(r3, r2, out):
    copy_members(r3, out, ("prepared.txt", "runner-built.txt", "fmt.log", "source.sha256",
                           PATCH, "runner/SHA256SUMS", "runner-build.log"))
    copy(r2 / TEST_PATH, out / "before.rs")
    copy(r3 / "surface" / TEST_PATH, out / "after.rs")


def m350_pixels(artifacts):
    raw = data(artifacts / LABEL_REPORT)
    require(raw["page_errors"] == [] and "failure" not in raw, "R3 has a label or browser failure")
    compact = raw["compact"]["fallback"]
    idle, middle = compact["color_frames"]
    expected = [206, 191, 243]
    require(idle["ms"] == 0 and idle["label_rgb"] == [202, 196, 207] and idle["label_pixels"] > 40
            and middle["ms"] == 44 and middle["label_rgb"] == expected and middle["label_pixels"] > 40
            and middle["solid_probe_rgb"] == [expected] * 4 and compact["label_rgb"] == [207, 189, 254]
            and compact["ax"]["focused"] and compact["ax"]["named_textboxes"] == 1,
            "R3 does not prove actual M350 timing, colors and focus")
    def capture(frame):
        with Image.open(artifacts / frame["png"]) as png:
            require(png.format == "PNG" and png.size == (640, 2008), "Wrong compact original")
            return png.convert("RGB")

    def point(x, y):
        return int(x * 2), int((y + 44) * 2)

    probes = [point(*value) for value in compact["probe_points"]]
    idle_image, middle_image = capture(idle), capture(middle)
    require(all(list(idle_image.getpixel(p)) == idle["label_rgb"]
                and list(middle_image.getpixel(p)) == expected for p in probes),
            "Real R3 PNG pixels do not match the four fixed M350 probes")
    return {"milliseconds": 44, "probe_pixels": [list(p) for p in probes],
            "idle_rgb": idle["label_rgb"], "middle_rgb": expected,
            "intermediate_png_sha256": digest(artifacts / middle["png"]),
            "probes_at_golden": len(probes)}


def guard_members(root):
    files = seal(root, "SHA256SUMS", GUARD_SHA)
    assets = seal(root / "tools", "harness.sha256", HARNESS_SHA)
    return tuple(sorted({*files, *(f"tools/{path}" for path in assets)}))


def check_invocation(stock, build, *, terminal=True):
    release = {"version": "0.1.68", "last_material_step": 350, "source_files": 685,
               "source_tree": TREE, "source_sha256": SOURCE, "wide_canvas_height": 800,
               "rtl_canvas_height": 1020}
    before = expected_product_files(build)
    require(before["debug/surface-e2e"] != RUNNER_SHA, "Original R2 and R3 runner digests must differ")
    pins = {
        "product_source_sha256": R2_SOURCE, "product_tree": R2_TREE,
        "runner_source_sha256": SOURCE, "runner_tree": TREE,
        "test_only_path": TEST_PATH, "test_only_patch_sha256": PATCH_SHA,
        "runner_sha256": RUNNER_SHA, "runner_manifest_sha256": RUNNER_MANIFEST_SHA,
        "runner_build_log_sha256": RUNNER_LOG_SHA,
        "stock_files": {**before, "debug/surface-e2e": RUNNER_SHA},
        "original_r2_stock_manifest_sha256": STOCK_SHA,
        "native_binary_manifest_sha256": NATIVE_SHA,
        "harness_manifest_sha256": HARNESS_SHA, "release": release,
    }
    launch = data(stock / "run/invocation.json")
    require(launch["inputs"] == data(stock / "inputs-before.json") == pins
            and data(stock / "release-pin.json") == release
            and path_entries(stock / "browser-binary-sha256.txt") == pins["stock_files"],
            "Stock did not use the exact new R3 runner and unchanged R2 products")
    if terminal:
        require(data(stock / "run/inputs-after.json") == data(stock / TERMINAL)["inputs"] == pins,
                "R3 before/after inputs differ")
    return pins


def historical_comparison(historical, report):
    require(digest(historical) == FOCUSED_SHA, "Wrong historical label comparison pin")
    captures = data(historical)["captures"]
    current = {name: details["sha256"] for name, details in report["pngs"].items()}
    require(len(captures) == 34 and captures.keys() == current.keys(), "Wrong historical comparison scope")
    different = {name: {"historical": captures[name], "actual": sha}
                 for name, sha in sorted(current.items()) if captures[name] != sha}
    return {"compared": 34, "byte_identical": 34 - len(different), "differences": different}


def stage_stock(stock, out, adjacent, historical):
    copy_members(stock, out, ("inputs-before.json", "browser-binary-sha256.txt", "release-pin.json", TERMINAL))
    copy(historical, out / "archived-focus.json")
    run, target = stock / "run", out / "run"
    ledgers = [path.relative_to(run).as_posix() for path in (run / "guard-ledger").glob("*.json")]
    copy_members(run, target, ("inputs-after.json", "invocation.json", WINDOW, "runner.log",
                               "verified-label-color-stock.json", *ledgers))
    pngs = data(stock / TERMINAL)["text_field_label_motion"]["pngs"]
    copy_members(run / "artifacts", target / "artifacts", (LABEL_REPORT, *adjacent, *pngs))
