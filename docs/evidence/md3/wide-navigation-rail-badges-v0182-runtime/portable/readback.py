"""Independent offline validation for the staged q82 evidence package."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent
HEX = re.compile(r"^[0-9a-f]{64}$")


def sha(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as f:
        for block in iter(lambda: f.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def data(path):
    return json.loads(Path(path).read_text())


def verify_inventory(root, manifest, expected):
    root = Path(root).resolve()
    manifest = Path(manifest).resolve()
    assert sha(manifest) == expected
    names = set()
    for line in manifest.read_text().splitlines():
        digest, name = line.split("  ", 1)
        rel = PurePosixPath(name)
        assert HEX.fullmatch(digest) and str(rel) == name
        assert not rel.is_absolute() and ".." not in rel.parts and name not in names
        target = root / name
        assert target.is_file() and not target.is_symlink() and target.resolve().is_relative_to(root)
        assert sha(target) == digest
        names.add(name)
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
              if p.is_file() and p != manifest}
    assert names == actual, (names - actual, actual - names)
    return len(names)


def verify_manifest_members(root, manifest, expected):
    root = Path(root).resolve()
    manifest = Path(manifest).resolve()
    assert sha(manifest) == expected
    names = set()
    for line in manifest.read_text().splitlines():
        digest, name = line.split("  ", 1)
        rel = PurePosixPath(name)
        assert HEX.fullmatch(digest) and str(rel) == name
        assert not rel.is_absolute() and ".." not in rel.parts and name not in names
        target = root / name
        assert target.is_file() and not target.is_symlink() and target.resolve().is_relative_to(root)
        assert sha(target) == digest
        names.add(name)
    return len(names)


def test_counts(text):
    rows = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text, re.M)
    assert rows and "test result: FAILED" not in text
    return sum(int(r[0]) for r in rows), sum(int(r[1]) for r in rows)


def verify_payload(root):
    root = Path(root)
    pins = data(root / "pins.json")
    n = pins["native"]
    b = pins["browser"]
    s = pins["source"]
    build = pins["build"]
    assert sha(root / "source/source.sha256") == s["manifest_sha256"]
    assert sha(root / "source/q82-source.bundle") == s["bundle_sha256"]
    bundle = root / "source/q82-source.bundle"
    with tempfile.TemporaryDirectory(prefix="surface-q82-bundle-check-") as tmp:
        checkout = Path(tmp) / "source"
        subprocess.run(["git", "clone", "--quiet", "--no-checkout", str(bundle), str(checkout)], check=True,
                       capture_output=True, text=True)
        subprocess.run(["git", "-C", str(checkout), "bundle", "verify", str(bundle)], check=True,
                       capture_output=True, text=True)
        subprocess.run(["git", "-C", str(checkout), "switch", "--quiet", "--detach", s["commit"]], check=True,
                       capture_output=True, text=True)
        commit = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True).strip()
        tree = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD^{tree}"], text=True).strip()
        assert (commit, tree) == (s["commit"], s["tree"])
        source_count = verify_manifest_members(checkout, root / "source/source.sha256", s["manifest_sha256"])
        actual = {p.relative_to(checkout).as_posix() for p in checkout.rglob("*")
                  if p.is_file() and p.relative_to(checkout).parts[0] != ".git"}
        expected = {line.split("  ", 1)[1] for line in (root / "source/source.sha256").read_text().splitlines()}
        assert actual == expected
        assert source_count == s["files"] == 716
    assert (root / "build/BUILD-COMPLETE").read_text().splitlines() == [s["tree"], s["manifest_sha256"]]
    assert sha(root / "build/stock.sha256") == build["stock_manifest_sha256"]
    assert sha(root / "build/native/SHA256SUMS") == build["native_manifest_sha256"]
    gpu_count = verify_inventory(root / "build/gpu", root / "build/gpu.sha256", build["gpu_manifest_sha256"])
    assert gpu_count == build["gpu_files"]
    assert sha(root / "native/config.json") == n["config_sha256"]
    assert sha(root / "browser/config.json") == b["config_sha256"]

    sys.path.insert(0, str(root / "native/tools"))
    from wide_rail_replay import verify as native_verify
    native = native_verify(root / "native/originals", n["archive_sha256"], n["config_sha256"],
                           data(root / "native/config.json"))
    assert native == n["result"]
    assert native["captures"] == 10 and native["reaped"] == 8 and native["zero_window_samples"] == 18

    sys.path.insert(0, str(root / "browser/tools"))
    from browser import verify_log, verify_badges
    browser = data(root / "browser/verified-stock-material.json")
    invocation = data(root / "browser/invocation.json")
    inputs = data(root / "browser/inputs-after.json")
    assert invocation["inputs"] == inputs
    assert inputs["verifier_sha256"] == sha(root / "browser/tools/browser.py")
    assert inputs["observer_sha256"] == sha(root / "browser/tools/observe-focused-original.py")
    assert (inputs["source_commit"], inputs["source_tree"], inputs["source_sha256"],
            inputs["config_sha256"], inputs["build_sha256"]) == (
                s["commit"], s["tree"], s["manifest_sha256"], b["config_sha256"], build["stock_manifest_sha256"])
    artifacts = root / "browser/artifacts"
    badge = verify_badges(artifacts)
    assert browser["badge_evidence"] == badge
    checks = verify_log((root / "browser/runner.log").read_text())
    assert browser["checks"] == checks and browser["material_steps"] == 387
    base = data(artifacts / "browser-material.json")
    assert not base["errors"] and base["browser"] == browser["browser"]
    window = data(root / "browser/headless-window-observation.json")
    assert window["stock_exit"] == 0 and not window["problems"]
    assert not window["visible_owned_windows"] and not window["remaining_observed_processes"]
    launches = window["guard_events"]
    assert len(launches) == 1 and launches[0]["role"] == "chrome-primary" and launches[0]["allowed"]
    assert launches[0]["argv"].count("--headless=new") == 1
    assert browser["chrome_samples"] == window["launch_observations"]["chrome-primary"]["samples"] > 0

    rust, ignored = test_counts((root / "build/check.log").read_text())
    metal_offscreen = test_counts((root / "build/gpu/offscreen.log").read_text())
    metal_selection = test_counts((root / "build/gpu/text-selection.log").read_text())
    assert "backend: Metal" in (root / "build/gpu/offscreen.log").read_text()
    assert "backend: Metal" in (root / "build/gpu/text-selection.log").read_text()
    assert metal_offscreen == (16, 0) and metal_selection == (2, 0)
    run = data(root / "native/originals/report.json")["runs"]["1"]["collapsed-dark-rtl-200-320"]
    icon = run["captures"]["initial"]["paint"]["owners"]["wide-navigation-badges-demo-inbox"]["icon"]
    assert icon["ink_pixels"] >= 95
    return {"rust": {"passed": rust, "ignored": ignored}, "metal": {
                "passed": metal_offscreen[0] + metal_selection[0], "offscreen": metal_offscreen,
                "text_selection": metal_selection, "pngs": build["gpu_pngs"], "manifest_files": gpu_count},
            "native": native, "browser": browser, "source_files": source_count,
            "collapsed_dark_mailbox_ink_pixels": icon["ink_pixels"]}


def verify_archive(root, seal):
    return {"files": verify_inventory(root, Path(root) / "SHA256SUMS", seal)}


def render_readme(pins, summary):
    s = pins["source"]
    return ("# Surface q82 wide navigation rail Badge runtime evidence\n\n"
            f"Source commit `{s['commit']}`, tree `{s['tree']}`, manifest SHA-256 `{s['manifest_sha256']}`.\n\n"
            f"Rust passed {summary['rust']['passed']} tests; {summary['rust']['ignored']} ignored. "
            f"Metal passed {summary['metal']['passed']} targeted GPU tests. "
            f"Native replay captured {summary['native']['captures']} original images twice, with "
            f"{summary['native']['zero_window_samples']} zero-window checks and {summary['native']['reaped']} reaped processes.\n\n"
            f"Headless Chrome verified {summary['browser']['material_steps']} Material steps and "
            f"{summary['browser']['checks']} checks. The independent screenshot verifier checked four q82 Badge images.\n\n"
            "The 320 dp dark RTL native capture keeps the collapsed mailbox visible at 200% text. "
            f"The original PNG contains {summary['collapsed_dark_mailbox_ink_pixels']} independently sampled icon ink pixels. "
            "The browser run verifies arrow focus in the short 320 × 720 dp rail and checks that "
            "the adjacent page retains its position.\n\n"
            "The build and binary manifests are original hash receipts. Compiled binaries are omitted. "
            "The source bundle contains the exact reproducible source.\n\n"
            "This package records local verification. It does not claim physical-screen or VoiceOver coverage.\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest-sha", required=True)
    args = ap.parse_args()
    result = {**verify_archive(ROOT, args.manifest_sha), **verify_payload(ROOT)}
    print(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
