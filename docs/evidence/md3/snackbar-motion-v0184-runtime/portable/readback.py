#!/usr/bin/env python3
"""Offline integrity and visual readback for a portable q84 Snackbar package."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tempfile

HEX = re.compile(r"^[0-9a-f]{64}$")


def need(ok, message):
    if not ok:
        raise RuntimeError(message)


def sha(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def jread(path):
    return json.loads(Path(path).read_text())


def manifest(path):
    out = {}
    names = []
    for row in Path(path).read_text().splitlines():
        digest, name = row.split("  ", 1)
        rel = PurePosixPath(name)
        need(HEX.fullmatch(digest) and rel.as_posix() == name and not rel.is_absolute()
             and ".." not in rel.parts and name not in out, f"bad manifest row: {name}")
        out[name] = digest
        names.append(name)
    need(names == sorted(names) and names, f"empty or unsorted manifest: {path}")
    return out


def members(root, index, expected):
    root = Path(root).resolve()
    index = Path(index).resolve()
    need(sha(index) == expected, f"manifest seal differs: {index}")
    rows = manifest(index)
    for name, digest in rows.items():
        path = root / name
        need(path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root)
             and sha(path) == digest, f"manifest member differs: {path}")
    return rows


def inventory(root, index, expected):
    root = Path(root).resolve()
    rows = members(root, index, expected)
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
              if p.is_file() and p != Path(index).resolve()}
    need(actual == set(rows), "sealed inventory has missing or extra files")
    return rows


def verify_source(root, pins):
    bundle = root / "source/q84-source.bundle"
    need(sha(bundle) == pins["source"]["bundle_sha256"], "source bundle digest differs")
    with tempfile.TemporaryDirectory(prefix="surface-q84-readback-", dir="/tmp") as tmp:
        checkout = Path(tmp) / "source"
        subprocess.run(["git", "clone", "--quiet", "--no-checkout", str(bundle), str(checkout)],
                       check=True, capture_output=True, text=True)
        subprocess.run(["git", "-C", str(checkout), "switch", "--quiet", "--detach",
                        pins["source"]["commit"]], check=True, capture_output=True, text=True)
        commit = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True).strip()
        tree = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD^{tree}"], text=True).strip()
        need((commit, tree) == (pins["source"]["commit"], pins["source"]["tree"]),
             "bundled source commit/tree differs")
        entries = members(checkout, root / "source/source.sha256", pins["source"]["manifest_sha256"])
        actual = {p.relative_to(checkout).as_posix() for p in checkout.rglob("*")
                  if p.is_file() and ".git" not in p.relative_to(checkout).parts}
        need(actual == set(entries) and len(entries) == pins["source"]["files"],
             "bundled source manifest inventory differs")
    need(sha(root / "source/forward.patch") == pins["source"]["patch_sha256"],
         "forward patch digest differs")
    return len(entries)


def test_totals(path):
    raw = gzip.decompress(Path(path).read_bytes()) if str(path).endswith(".gz") else Path(path).read_bytes()
    text = raw.decode()
    rows = re.findall(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", text, re.M)
    need(rows and all(state == "ok" and failed == "0" for state, _, failed, _ in rows),
         f"test log failed: {path}")
    return {"passed": sum(int(p) for _, p, _, _ in rows),
            "ignored": sum(int(i) for _, _, _, i in rows), "sha256": hashlib.sha256(raw).hexdigest()}


def verify_payload(root):
    root = Path(root).resolve()
    pins = jread(root / "pins.json")
    files = verify_source(root, pins)
    need((root / "build/BUILD-COMPLETE").read_text().splitlines() ==
         [pins["source"]["tree"], pins["source"]["manifest_sha256"]], "build marker differs")
    need((root / "build/source-before.log").read_bytes() ==
         (root / "build/source-after.log").read_bytes(), "source changed during build")
    need(sha(root / "build/stock.sha256") == pins["build"]["stock_manifest_sha256"],
         "stock manifest digest differs")
    stock = manifest(root / "build/stock.sha256")
    need(sha(root / "build/native/SHA256SUMS") == pins["build"]["native_manifest_sha256"],
         "native binary manifest digest differs")
    native_bins = manifest(root / "build/native/SHA256SUMS")
    gpu = inventory(root / "build/gpu", root / "build/gpu.sha256", pins["build"]["gpu_manifest_sha256"])
    need(len(stock) == pins["build"]["stock_files"] and len(native_bins) == pins["build"]["native_binary_count"]
         and len(gpu) == pins["build"]["gpu_files"], "build inventory counts differ")
    check = test_totals(root / "build/logs/check.log")
    metal = {
        "offscreen": test_totals(root / "build/gpu/offscreen.log"),
        "text_selection": test_totals(root / "build/gpu/text-selection.log"),
    }
    need("Metal" in (root / "build/gpu/offscreen.log").read_text()
         and "Metal" in (root / "build/gpu/text-selection.log").read_text(), "Metal backend receipt missing")
    need(all(value["passed"] > 0 and value["ignored"] == 0 for value in metal.values()),
         "Metal receipts have no passing tests or contain ignored tests")

    sys.path.insert(0, str(root / "native/tools"))
    import snackbar_native
    native_results = []
    native_pids = []
    screenshot_hashes = []
    for index, run in enumerate(pins["native"]["runs"], 1):
        need(run["package_dir"] == f"native/run-{index}", "Native package path differs")
        native_root = root / run["package_dir"]
        result = snackbar_native.verify(native_root,
                                        run["archive_sha256"], run["config_sha256"])
        need(result == run["result"], f"native run receipt differs: {run['name']}")
        native_report = jread(native_root / "report.json")
        native_pids.extend(row["pid"] for row in native_report["runs"].values())
        screenshot_hashes.append({name: row["screenshot"]["sha256"]
                                  for name, row in native_report["runs"].items()})
        native_results.append(result)
    need(len(native_results) == 2 and len(native_pids) == len(set(native_pids)),
         "Native runs are not independent process runs")
    need(screenshot_hashes[0] == screenshot_hashes[1], "Native screenshots changed across repeats")

    sys.path.insert(0, str(root / "browser/tools"))
    import browser
    bpin = pins["browser"]
    need(sha(root / "browser/config.json") == bpin["config_sha256"], "Browser config digest differs")
    browser_config = jread(root / "browser/config.json")
    need((browser_config["source_commit"], browser_config["source_tree"],
          browser_config["source_sha256"], browser_config["source_files"],
          browser_config["build_sha256"]) == (
             pins["source"]["commit"], pins["source"]["tree"], pins["source"]["manifest_sha256"],
             pins["source"]["files"], pins["build"]["stock_manifest_sha256"]),
         "Browser config source/build pins differ")
    stock_rows = manifest(root / "build/stock.sha256")
    need(stock_rows.get("stock/debug/surface-e2e") == browser_config["runner_sha256"],
         "Browser runner hash is absent from stock manifest")
    run = root / "browser/run"
    invocation = jread(run / "invocation.json")
    input_pins = invocation["inputs"]
    for name, key in (("browser.py", "verifier_sha256"),
                      ("verify_snackbar_motion.py", "snackbar_reader_sha256"),
                      ("evidence_common.py", "common_helper_sha256"),
                      ("observe-focused-original.py", "guard_sha256")):
        need(sha(root / "browser/tools" / name) == input_pins[key],
             f"Browser reader input differs: {name}")
    need((input_pins["source_commit"], input_pins["source_tree"], input_pins["source_sha256"],
          input_pins["config_sha256"], input_pins["build_sha256"], input_pins["runner_sha256"]) == (
             pins["source"]["commit"], pins["source"]["tree"], pins["source"]["manifest_sha256"],
             bpin["config_sha256"], pins["build"]["stock_manifest_sha256"], browser_config["runner_sha256"]),
         "Browser invocation does not match packaged source/build pins")
    need(jread(run / "inputs-after.json") == input_pins, "Browser after-pins differ")
    need(jread(run / "verified-snackbar-stock.json") == bpin["result"], "Browser receipt pin differs")
    ledgers = [jread(path) for path in sorted((run / "guard-ledger").glob("*.json"),
                                               key=lambda path: path.name)]
    window = jread(run / "headless-window-observation.json")
    need(sorted(ledgers, key=lambda row: row["pid"]) ==
         sorted(window["guard_events"], key=lambda row: row["pid"]),
         "Browser process ledger differs from the original window observation")
    result = browser.verify_run(run, window, input_pins, Path(bpin["original_run_root"]))
    need(result == bpin["result"] and result["material_steps"] == 391,
         "full ordered Material Browser receipt differs")

    focused = {}
    for item in pins["focused_logs"]:
        path = root / "focused" / item["name"]
        need(sha(path) == item["sha256"], f"focused log digest differs: {item['name']}")
        focused[item["name"]] = item["sha256"]
    pilot = jread(root / "focused/pilot-receipt.json")
    steps = ["M28", "M29", "M30", "M31", "M391", "M392"]
    raw = (root / "focused/runner.log").read_text()
    need(re.findall(r"^  \[(M\d+)\]", raw, re.M) == steps == pilot["steps"]
         and pilot["status"] == "PASS" and pilot["source"]["source_commit"] == pins["source"]["commit"]
         and re.findall(r"^case material \.\.\. ok \(6 steps, (\d+) checks,", raw, re.M) == [str(pilot["checks"])],
         "Focused Snackbar result differs")
    return {"source_files": files, "stock_files": len(stock), "native_binary_count": len(native_bins),
            "check": check, "metal": metal, "gpu_pngs": pins["build"]["gpu_pngs"],
            "native_runs": native_results, "native_repeat_hashes": screenshot_hashes[0],
            "browser": result, "focused_checks": pilot["checks"], "focused_logs": focused}


def verify_package(root, seal):
    root = Path(root).resolve(strict=True)
    need(HEX.fullmatch(seal or "") and sha(root / "SHA256SUMS") == seal, "archive seal differs")
    count = len(inventory(root, root / "SHA256SUMS", seal))
    return {"files": count, "summary": verify_payload(root)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent)
    parser.add_argument("--seal", required=True)
    args = parser.parse_args()
    print(json.dumps(verify_package(args.root, args.seal), indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
