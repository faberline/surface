#!/usr/bin/env python3
"""Offline readback for the R5 package and its separately scoped history."""
import hashlib
import json
import runpy
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
R5_SOURCE = "11b2891b47a85681ac0dd5b56274ad98cf9a412abdb6dfac110baf2253b567c3"
R5_TREE = "957b77b0c35e535f9c8191277bfc20673d937c60"
R5_COMMIT = "6c856eb5cdfef69f8fa10949be5abc2d44d40266"
R5_PATCH = "da28567bb5f820abb1b3f1b8a51b3efb7a0c41fc505447da24f5540a7640d780"
R5_BUNDLE = "18c4e7216465ec74a46784ba6ebe2f7be2a8f17bb3da18e0a081e9a31cb25645"
R5_TEST_PATCH = "f93bd9d9b16d2afbcbae95e82c56d6e878f98279f6c9cab6f2201b7b3c9db721"
BASE_COMMIT = "f5beae439e76f141395932156c9dd84d1b037e96"
BASE_TREE = "b61630995f470772785605a944e3515a45b6d553"
R3_SOURCE = "a78b09d904211968f9fff598179af986a0153c940112801155bc6d49a46a1e93"
R3_TREE = "f958f418eb642c2205f9e10fe7a313837b6580be"
R4_SOURCE = "5b2ca5d0b0975d7a014705bdd28fcef6d98ad30f2b5083ec06c806edd22dd8a6"
R4_PRODUCT_SOURCE = "2df44969de81b86ef857592452f56e28e7db206a295d855e3bf8c332df507a38"
R3_BUILD = "37d103c2a2cb801d38501aa8039e0c5c92d5b692b1d6a003ae608b522e678a57"
R4_RUNNER = "99a39eb9176275e8040b78a318507074be886680edd457d648bab216f983d0ff"
R4_RUNNER_MANIFEST = "e6a0ee36db892ee30c5d07807a8e05c957432980c811eb7adcf7dd0e49266505"

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def need(ok, message):
    if not ok:
        raise SystemExit("READBACK FAILED: " + message)

rows = {}
for line in (ROOT / "SHA256SUMS").read_text().splitlines():
    digest, name = line.split("  ", 1)
    p = Path(name)
    need(name not in rows and not p.is_absolute() and ".." not in p.parts, "unsafe package path")
    rows[name] = digest
for name, digest in rows.items():
    need((ROOT / name).is_file() and sha(ROOT / name) == digest, "package hash mismatch: " + name)
actual = {p.relative_to(ROOT).as_posix() for p in ROOT.rglob("*") if p.is_file() and p != ROOT / "SHA256SUMS"}
need(actual == set(rows), "package inventory differs")

meta = json.loads((ROOT / "verified-summary.json").read_text())
need(meta["source_commit"] == R5_COMMIT and meta["source_tree"] == R5_TREE
     and meta["source_sha256"] == R5_SOURCE and meta["patch_sha256"] == R5_PATCH
     and meta["r3_to_r5_test_patch_sha256"] == R5_TEST_PATCH
     and meta["base_commit"] == BASE_COMMIT and meta["base_tree"] == BASE_TREE
     and meta["source_bundle"]["sha256"] == R5_BUNDLE
     and meta["source_bundle"]["size_bytes"] == 19448533
     and meta["source_bundle"]["ref"] == "refs/heads/codex/md3-baseline-submenu"
     and meta["source_bundle"]["clone_manifest_files_verified"] == 706
     and meta["source_bundle"]["applied_manifest_files_verified"] == 706,
     "R5 source pins differ")
need(sha(ROOT / "source/source.sha256") == R5_SOURCE
     and sha(ROOT / "source/forward-from-v0176.patch") == R5_PATCH
     and sha(ROOT / "source/r3-r5-focus-test.patch") == R5_TEST_PATCH
     and (ROOT / "source/base.commit").read_text().strip() == BASE_COMMIT
     and (ROOT / "source/base.tree").read_text().strip() == BASE_TREE
     and sha(ROOT / "source/r5-source.bundle") == R5_BUNDLE,
     "R5 source seals differ")

# R3 remains a scoped failure record, with clean process cleanup.
h = ROOT / "history/r3-negative-control"
log = (h / "runner-tail.txt").read_text()
inv = json.loads((h / "invocation.json").read_text())
config = json.loads((h / "browser-config.json").read_text())
window = json.loads((h / "headless-window-observation.json").read_text())
need("last step: M377" in log and "timed out after 30s" in log
     and window["stock_exit"] == 1 and not window["problems"]
     and not window["visible_owned_windows"] and not window["remaining_observed_processes"],
     "R3 negative-control result differs")
need(inv["inputs"]["source_sha256"] == R3_SOURCE and inv["inputs"]["reviewed_source_tree"] == R3_TREE
     and sha(h / "source-r3.sha256") == R3_SOURCE
     and (h / "source-r3.tree").read_text().strip() == R3_TREE
     and sha(h / "browser-config.json") == inv["inputs"]["config_sha256"]
     and config["source_tree"] == R3_TREE, "R3 source/config receipt differs")

# R4 is a focused diagnostic pilot, not a full-suite acceptance result.
p = ROOT / "pilot/r4-focused"
summary = json.loads((p / "diagnostic-summary.json").read_text())
runner_log = (p / "runner.log").read_text()
pinv = json.loads((p / "invocation.json").read_text())
pafter = json.loads((p / "inputs-after.json").read_text())
pwindow = json.loads((p / "headless-window-observation.json").read_text())
need(summary["scope"] == "focused diagnostic only; not stock acceptance"
     and summary["exit_status"] == 0
     and summary["steps_seen"] == ["M376", "M377", "M378", "M379"]
     and summary["window_samples"] == 34, "R4 pilot scope/result differs")
need("4 steps, 164 checks" in runner_log
     and "e2e result: 1 passed; 0 failed; 164 checks" in runner_log
     and pinv["inputs"] == pafter and pwindow["stock_exit"] == 0
     and not pwindow["problems"] and not pwindow["visible_owned_windows"]
     and not pwindow["remaining_observed_processes"], "R4 pilot or process receipt differs")
need(summary["pointer_focus"] == {"open": "canvas", "disabled": "canvas", "return": "canvas",
     "reopen": "canvas", "dismiss": "canvas"} and summary["outside_focus"] == "canvas",
     "R4 focus-state witness differs")
need(summary["inputs"]["diagnostic_manifest_sha256"] == R4_SOURCE
     and sha(p / "source-r4.sha256") == R4_SOURCE
     and sha(p / "r4-product-source.sha256") == R4_PRODUCT_SOURCE
     and summary["inputs"]["original_r3_source_tree"] == R3_TREE
     and summary["inputs"]["original_r3_build_manifest_sha256"] == R3_BUILD
     and sha(p / "original-r3-build-SHA256SUMS") == R3_BUILD
     and summary["inputs"]["runner_sha256"] == R4_RUNNER
     and summary["inputs"]["runner_manifest_sha256"] == R4_RUNNER_MANIFEST
     and sha(p / "runner-SHA256SUMS") == R4_RUNNER_MANIFEST, "R4 pilot source/build/runner pins differ")
sys.path.insert(0, str(ROOT / "browser-tools"))
browser = runpy.run_path(str(ROOT / "browser-tools/browser.py"))
pilot_pixels = browser["verify_submenus"](p / "artifacts")
saved_pixels = json.loads((p / "pixel-readback.json").read_text())
need(json.dumps(pilot_pixels, sort_keys=True) == json.dumps(saved_pixels, sort_keys=True),
     "R4 independent screenshot readback differs")
for name in ("material-command-menu-submenu.png", "material-command-menu-submenu-dark-rtl.png"):
    need(pilot_pixels["screenshots"][name]["edge_probe_matches"] == [3840, 1552, 1552],
         "R4 image geometry/color probe differs: " + name)

# Verify the original R5 native screenshots and the R5 full Browser run.
sys.path.insert(0, str(ROOT / "native-tools"))
native = runpy.run_path(str(ROOT / "native-tools/replay.py"))
pins_sha = (ROOT / "native/pins.sha256").read_text().strip()
pins = json.loads((ROOT / "native/pins.json").read_text())
binary_manifest = ROOT / "native/binary-SHA256SUMS"
need(sha(binary_manifest) == pins["binary_sha256"], "R5 native binary manifest differs")
binaries = {}
for line in binary_manifest.read_text().splitlines():
    digest, name = line.split("  ", 1)
    binaries[name] = digest
proof = {"source_sha256": pins["source_sha256"], "reviewed_tree": pins["source_tree"],
         "source_files": pins["source_files"], "binary_manifest_sha256": pins["binary_sha256"],
         "binaries": binaries}
native_report = json.loads((ROOT / "native/run/report.json").read_text())
native_result = native["verify"](ROOT / "native/run", pins, pins_sha,
    sha(ROOT / "native/run/SHA256SUMS"), proof)
need(native_report["transports"] == ["Page.screenshot", "CLI", "MCP"]
     and native_result["captures"] == 4, "R5 native replay differs")

browser_config = json.loads((ROOT / "browser/config.json").read_text())
browser_config_sha = (ROOT / "browser/config.sha256").read_text().strip()
browser_inv = json.loads((ROOT / "browser/run/invocation.json").read_text())
browser_after = json.loads((ROOT / "browser/run/inputs-after.json").read_text())
need(browser_inv["inputs"] == browser_after
     and browser_after["config_sha256"] == browser_config_sha
     and browser_config["source_sha256"] == R5_SOURCE
     and browser_config["source_tree"] == R5_TREE
     and browser_config["build_sha256"] == sha(ROOT / "build/SHA256SUMS"),
     "R5 Browser source/build pins differ")
browser_result = browser["verify_run"](ROOT / "browser/run",
    json.loads((ROOT / "browser/run/headless-window-observation.json").read_text()))
stored_result = json.loads((ROOT / "browser/run/verified-stock-material.json").read_text())
need(json.dumps(browser_result, sort_keys=True) == json.dumps(stored_result, sort_keys=True),
     "R5 saved Browser receipt differs after JSON normalization")
need(browser_result["steps"] == 378 and len(browser_result["submenus"]["screenshots"]) == 2
     and len(browser_result["menus"]) == 12, "R5 Browser artifact inventory differs")
print("PORTABLE READBACK PASS: R5 runtime evidence, separate R4 pilot, and R3 negative history")
