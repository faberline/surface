"""Verify the exact release stock pin and compare its two settled frames."""

import argparse
import json
from pathlib import Path
import subprocess
import sys

from gallery_transport import require
from native_runtime import digest


def manifest(path):
    entries = [line.split(None, 1) for line in path.read_text().splitlines()]
    result = {name: hash_value for hash_value, name in entries}
    require(len(result) == len(entries), f"Duplicate entries in {path.name}")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pin-stage", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use private /tmp evidence")
    out.mkdir(parents=True)
    stage = args.pin_stage.resolve(strict=True)
    root = Path(__file__).resolve().parent.parent
    expected_source = root / "stock-source.sha256"
    exact_source = stage / "browser-source.sha256"
    incoming = stage / "incoming-source.sha256"
    exact_source_digest = digest(exact_source)
    require(incoming.read_bytes() == expected_source.read_bytes()
            and manifest(exact_source) == manifest(expected_source),
            "The stock pin was built from different sealed source inputs")
    receipt = stage / "browser-binary-sha256.txt"
    receipt_digest = digest(receipt)
    pins = manifest(receipt)
    paths = {kind: stage / "bundle" / name for kind, name in (
        ("gallery", "debug/surface-material-gallery"), ("cli", "debug/surface-devtools"))}
    expected = {kind: pins[str(path.relative_to(stage / "bundle"))] for kind, path in paths.items()}
    require({kind: digest(path) for kind, path in paths.items()} == expected,
            "Pinned stock Gallery or CLI digest mismatch")

    script = root / "native-scripts" / "replay-stock-settled.py"
    stock = out / "stock"
    subprocess.run([sys.executable, script, "--gallery", paths["gallery"],
                    "--cli", paths["cli"], "--out", stock], check=True)
    motion = root / "exact-motion" / "motion-1"
    diagnostic = json.loads((motion / "native-search-motion-report.json").read_text())
    baseline = json.loads((stock / "stock-search-settled-report.json").read_text())
    captures = []
    for actual in diagnostic["profiles"]:
        name = actual["name"]
        reference = next(profile for profile in baseline["profiles"] if profile["profile"] == name)
        snapped = stock / reference["shot"]["name"]
        settled = motion / actual["pngs"]["opening-settled"]["name"]
        require(snapped.read_bytes() == settled.read_bytes(), f"{name} settled PNG changed")
        require(reference["editor_evidence"] == actual["frames"]["opening-settled"]["editor_evidence"],
                f"{name} settled ink bounds changed")
        captures.append({"profile": name, "sha256": digest(snapped), "api_cli_mcp_match": True,
                         "editor_evidence": reference["editor_evidence"], "bytes_match_stock": True})

    require({kind: digest(path) for kind, path in paths.items()} == expected
            and digest(receipt) == receipt_digest and digest(exact_source) == exact_source_digest
            and incoming.read_bytes() == expected_source.read_bytes(),
            "Stock pins or source manifest changed")
    comparison = {
        "schema": "surface.test.search-v0143-native-stock-comparison.v1",
        "stock_binary_manifest_sha256": receipt_digest,
        "stock_source_manifest_sha256": exact_source_digest,
        "incoming_source_manifest_sha256": digest(incoming),
        "stock_report_sha256": digest(stock / "stock-search-settled-report.json"),
        "diagnostic_report_sha256": digest(motion / "native-search-motion-report.json"),
        "stock_binaries": expected,
        "stock_clock_reply": baseline["test_clock"],
        "matched": captures,
    }
    (out / "exact-stock-comparison.json").write_text(json.dumps(comparison, indent=2) + "\n")
    print("PASS both motion end states and editor ink match the exact v0.1.43 stock pin byte for byte")


if __name__ == "__main__":
    main()
