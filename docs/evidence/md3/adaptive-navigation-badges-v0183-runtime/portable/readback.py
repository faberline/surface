"""Independent offline verification of portable q83 adaptive-navigation evidence."""
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
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def data(path):
    return json.loads(Path(path).read_text())


def parse_manifest(path):
    names, entries = [], {}
    for line in Path(path).read_text().splitlines():
        digest, name = line.split("  ", 1)
        rel = PurePosixPath(name)
        assert HEX.fullmatch(digest) and rel.as_posix() == name
        assert not rel.is_absolute() and ".." not in rel.parts and name not in entries
        entries[name] = digest
        names.append(name)
    assert names == sorted(names) and names
    return entries


def verify_members(root, manifest, expected_sha):
    root, manifest = Path(root).resolve(), Path(manifest).resolve()
    assert sha(manifest) == expected_sha
    entries = parse_manifest(manifest)
    for name, expected in entries.items():
        path = root / name
        assert path.is_file() and not path.is_symlink()
        assert path.resolve().is_relative_to(root) and sha(path) == expected
    return entries


def verify_inventory(root, manifest, expected_sha):
    root, manifest = Path(root).resolve(), Path(manifest).resolve()
    entries = verify_members(root, manifest, expected_sha)
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
              if p.is_file() and p != manifest}
    assert actual == set(entries), (actual - set(entries), set(entries) - actual)
    return entries


def test_counts(text):
    rows = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text, re.M)
    assert rows and "test result: FAILED" not in text
    return sum(int(row[0]) for row in rows), sum(int(row[1]) for row in rows)


def verify_source_bundle(root, source):
    bundle = root / "source/q83-source.bundle"
    assert sha(bundle) == source["bundle_sha256"]
    with tempfile.TemporaryDirectory(prefix="surface-q83-source-readback-", dir="/tmp") as tmp:
        checkout = Path(tmp) / "source"
        subprocess.run(["git", "clone", "--quiet", "--no-checkout", str(bundle), str(checkout)],
                       check=True, capture_output=True, text=True)
        subprocess.run(["git", "-C", str(checkout), "bundle", "verify", str(bundle)], check=True,
                       capture_output=True, text=True)
        subprocess.run(["git", "-C", str(checkout), "switch", "--quiet", "--detach", source["commit"]],
                       check=True, capture_output=True, text=True)
        commit = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD"],
                                         text=True).strip()
        tree = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD^{tree}"],
                                       text=True).strip()
        assert (commit, tree) == (source["commit"], source["tree"])
        manifest = root / "source/source.sha256"
        assert sha(manifest) == source["manifest_sha256"]
        entries = verify_members(checkout, manifest, source["manifest_sha256"])
        actual = {p.relative_to(checkout).as_posix() for p in checkout.rglob("*")
                  if p.is_file() and ".git" not in p.relative_to(checkout).parts}
        assert actual == set(entries) and len(entries) == source["files"] == 717
    return 717


def verify_payload(root):
    root = Path(root).resolve()
    pins = data(root / "pins.json")
    source, build = pins["source"], pins["build"]
    native_pin, browser_pin = pins["native"], pins["browser"]

    source_files = verify_source_bundle(root, source)
    assert (root / "build/BUILD-COMPLETE").read_text().splitlines() == [
        source["tree"], source["manifest_sha256"]]
    assert sha(root / "build/stock.sha256") == build["stock_manifest_sha256"]
    assert sha(root / "build/native/SHA256SUMS") == build["native_manifest_sha256"]
    assert len(parse_manifest(root / "build/native/SHA256SUMS")) == build["native_binary_count"]
    gpu = verify_inventory(root / "build/gpu", root / "build/gpu.sha256",
                           build["gpu_manifest_sha256"])
    assert len(gpu) == build["gpu_files"]
    assert build["gpu_pngs"] == sorted(name for name in gpu if name.lower().endswith(".png"))
    assert (root / "build/build-source-before.log").read_bytes() == \
           (root / "build/build-source-after.log").read_bytes()

    assert sha(root / "native/config.json") == native_pin["config_sha256"]
    assert sha(root / "browser/config.json") == browser_pin["config_sha256"]
    sys.path.insert(0, str(root / "native/tools"))
    from adaptive_native import verify as verify_native
    native = verify_native(root / "native/originals", native_pin["archive_sha256"],
                           native_pin["config_sha256"], data(root / "native/config.json"))
    assert native == native_pin["result"]
    assert (native["captures"], native["reaped_processes"], native["zero_window_samples"]) == (12, 10, 22)

    sys.path.insert(0, str(root / "browser/tools"))
    import browser_q83
    browser_config = data(root / "browser/config.json")
    browser_root = root / "browser"
    invocation = data(browser_root / "invocation.json")
    browser_pins = invocation["inputs"]
    assert data(browser_root / "inputs-after.json") == browser_pins
    original_out = Path(browser_pin["run_root"])
    assert original_out.is_absolute()
    assert invocation["argv"][-3:] == ["--artifacts", str(original_out / "artifacts"), "material"]
    assert browser_pins["verifier_sha256"] == sha(root / "browser/tools/browser_q83.py")
    assert browser_pins["guard_sha256"] == sha(root / "browser/tools/observe-focused-original.py")
    assert browser_pins["common_helper_sha256"] == sha(root / "browser/tools/evidence_common.py")
    assert (browser_pins["source_commit"], browser_pins["source_tree"], browser_pins["source_sha256"],
            browser_pins["config_sha256"], browser_pins["build_sha256"]) == (
                source["commit"], source["tree"], source["manifest_sha256"],
                browser_pin["config_sha256"], build["stock_manifest_sha256"])
    assert (browser_config["source_commit"], browser_config["source_tree"],
            browser_config["source_sha256"], browser_config["build_sha256"],
            browser_config["runner_sha256"]) == (
                source["commit"], source["tree"], source["manifest_sha256"],
                build["stock_manifest_sha256"], browser_pin["runner_sha256"])
    assert browser_pins["runner_sha256"] == browser_pin["runner_sha256"]
    assert parse_manifest(root / "build/stock.sha256")["stock/debug/surface-e2e"] == \
           browser_pin["runner_sha256"]
    window = data(browser_root / "headless-window-observation.json")
    ledgers = [data(p) for p in sorted((browser_root / "guard-ledger").glob("*.json"))]
    assert sorted(ledgers, key=lambda row: row["pid"]) == sorted(window["guard_events"],
                                                                 key=lambda row: row["pid"])
    result = browser_q83.verify_run(browser_root, window, browser_pins, original_out)
    receipt = data(browser_root / "verified-q83-stock.json")
    assert receipt == json.loads(json.dumps(result)) == browser_pin["result"]
    assert result["material_steps"] == 389

    rust, ignored = test_counts((root / "build/check.log").read_text())
    offscreen_log = (root / "build/gpu/offscreen.log").read_text()
    selection_log = (root / "build/gpu/text-selection.log").read_text()
    assert "backend: Metal" in offscreen_log and "backend: Metal" in selection_log
    offscreen, selection = test_counts(offscreen_log), test_counts(selection_log)
    assert offscreen[0] > 0 and offscreen[1] == 0
    assert selection[0] > 0 and selection[1] == 0
    return {"source_files": source_files,
            "rust": {"passed": rust, "ignored": ignored},
            "metal": {"offscreen": offscreen, "text_selection": selection,
                      "passed": offscreen[0] + selection[0], "pngs": build["gpu_pngs"]},
            "native": native, "browser": result,
            "browser_badges": result["adaptive_badges"]}


def verify_archive(root, seal):
    return len(verify_inventory(root, Path(root) / "SHA256SUMS", seal))


def render_readme(pins, summary):
    source = pins["source"]
    browser = summary["browser"]
    native = summary["native"]
    metal = summary["metal"]
    return (
        "# Surface q83 adaptive navigation Badge runtime evidence\n\n"
        f"Source commit {source['commit']}, tree {source['tree']}, "
        f"manifest SHA-256 {source['manifest_sha256']}; {summary['source_files']} files.\n\n"
        f"Rust passed {summary['rust']['passed']} tests; {summary['rust']['ignored']} ignored. "
        f"Metal passed {metal['passed']} GPU tests ({metal['offscreen'][0]} offscreen and "
        f"{metal['text_selection'][0]} text-selection). Native replay verified "
        f"{native['captures']} captures, {native['zero_window_samples']} zero-window checks, "
        f"and {native['reaped_processes']} reaped processes.\n\n"
        f"Headless Chrome passed {browser['material_steps']} ordered Material steps and "
        f"{browser['checks']} checks. The verifier read back four original PNGs, responsive "
        "navigation geometry, accessible destination names, badge pixels, the guarded Chrome "
        "launch, zero visible owned windows, and process cleanup.\n\n"
        "The build and binary manifests are retained as original hash receipts. Compiled "
        "executables and Web binaries are omitted. The exact source bundle is included and "
        "checked by a detached local checkout.\n\n"
        "This evidence covers headless rendering and accessibility metadata. It does not claim "
        "VoiceOver speech or physical-display behavior.\n"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest-sha", required=True)
    args = parser.parse_args()
    files = verify_archive(ROOT, args.manifest_sha)
    summary = verify_payload(ROOT)
    print(json.dumps({"files": files, "summary": summary}, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()

