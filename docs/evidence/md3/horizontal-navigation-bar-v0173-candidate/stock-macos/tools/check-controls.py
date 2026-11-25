"""Recheck the exact inputs and actual predecessor and scoped reports offline."""

from pathlib import Path
from runpy import run_path
from tempfile import TemporaryDirectory
import json
import subprocess
import sys

from evidence_common import data, digest, require, seal
from stock_config import NAV_FOCUS, Q72_FINAL, STOCK_RUNNER, reviewed, release_pin, validate_reviewed
from verify_navigation_stock import verify as navigation

ROOT = Path(__file__).resolve().parent
Q72 = Path("/tmp/surface-md3-field-pair-stock-v0172-final.qeJADEOL")
Q72_SEAL = "0a2c01309be1d4d5c0cde1153d8fc713f9ed2f0cb26dbd2f657aaf08b144be3c"
Q72_RECEIPT = "f4cbebb4e7dda593cb0109772f0aaafb08c8c629e12657028d3d30fe32ea57d7"
OBS = run_path(str(ROOT / "observe-multiline.py"))


def rejected(callback, phrase):
    try:
        callback()
    except (ValueError, RuntimeError) as error:
        require(phrase in str(error), f"Wrong offline rejection: {error}")
    else:
        raise ValueError(f"Accepted invalid evidence: {phrase}")


def main():
    OBS["self_test"]()
    exact = reviewed(ROOT / "authority-pins.json")
    require(data(ROOT / "reviewed-pins.json") == exact, "Wrong locally prepared review pins")
    runner = (ROOT / "bundle/debug/surface-e2e").resolve(strict=True)
    prepared = OBS["verify_inputs"](runner, STOCK_RUNNER, ROOT)
    require(prepared == data(ROOT / "inputs-before.json") == data(ROOT / "inputs-rechecked.json"),
            "The exact prepared stock files changed")
    seal(Q72, "harness.sha256", Q72_SEAL)
    report = Q72 / "stock-mac-exact-offline.json"
    require(digest(report) == Q72_RECEIPT, "Wrong original q72 stock receipt")
    previous = subprocess.run(
        [sys.executable, "-E", "-B", str(Q72 / "verify-stock-receipt.py"), str(Q72 / "run")],
        capture_output=True, check=True,
    ).stdout
    require(previous == report.read_bytes(), "The original q72 stock receipt did not reproduce")
    previous = json.loads(previous)
    require(previous["cases"]["material"]["steps"] == 361 and previous["total_checks"] == 29907,
            "Wrong preceding actual full-stock record")
    reject_prior = {**exact, "source_sha256": Q72_FINAL}
    rejected(lambda: validate_reviewed(reject_prior), "not the v0.1.73 candidate")
    rejected(lambda: OBS["verify_runner_log"]((Q72 / "run/runner.log").read_text()), "Material steps changed")
    rejected(lambda: navigation(Q72 / "run/artifacts", previous), "Both original stock Navigation reports")

    # This control pairs actual q72 stock data with actual q73 scoped Navigation.
    # It checks shell predicates only and cannot establish an exact q73 run.
    focus = data(ROOT / "focused-proof.json")
    nav = navigation(NAV_FOCUS / "development-r3/artifacts", focus)
    require(nav["horizontal"] == focus["horizontal"] and nav["compact"] == focus["compact"],
            "The stock decoder differs on the known R3 originals")
    proof = {**previous, "release": release_pin(exact), "focused": focus, "navigation_bars": nav,
             "cases": {**previous["cases"], "material": {**previous["cases"]["material"], "steps": 364}}}
    with TemporaryDirectory(dir=ROOT) as tmp:
        receipt = Path(tmp) / "receipt.json"

        def shell(candidate):
            receipt.write_text(json.dumps(candidate))
            return subprocess.run(["sh", ROOT / "check-receipt.sh",
                                   ROOT / "authority-pins.json", receipt], check=False).returncode

        require(shell(proof) == 0, "Known valid receipt fields failed the shell predicate")
        wrong = {**nav, "horizontal": {**nav["horizontal"], "pngs": {}}}
        require(shell({**proof, "navigation_bars": wrong}) != 0, "Missing Horizontal PNGs were accepted")
        require(shell({**proof, "focused": {**focus, "visible_windows": 1}}) != 0, "A visible window was accepted")
        require(shell({**proof, "release": previous["release"]}) != 0, "Predecessor scope was accepted")

    print(json.dumps({"scope": "offline controls only", "prepared_stock_bundle_files": prepared["bundle_files_checked"],
                      "q72_original_stock_checks": previous["total_checks"],
                      "q73_scoped_checks": focus["checks"], "q73_scoped_pngs": 7,
                      "rejections": ["q72 source", "q72 stock step list", "missing Horizontal original reports",
                                     "missing Horizontal originals", "visible window", "wrong release scope"]}, indent=2))


if __name__ == "__main__":
    main()
