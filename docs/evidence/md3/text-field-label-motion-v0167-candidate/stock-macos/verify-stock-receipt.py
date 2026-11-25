"""Recheck the stock headless text field label motion run without loading a browser."""

import json
from pathlib import Path
from runpy import run_path
import sys

from verify_forms_pixels import checksum, require
from verify_label_motion import focused_receipt

ROOT = Path(__file__).resolve().parent
OBSERVER = run_path(str(ROOT / "observe-multiline.py"))
PREVIOUS = Path("/tmp/surface-md3-sliders-stock.lK6a9gKl/verify-existing.py")
EXPECTED_PREVIOUS = "eded5dfdfcf94dc901cb3608b83cc86a557fe4cb02b91122416c6a03e3568558"


def data(path):
    return json.loads(path.read_text())


def main(run):
    after = data(run / "inputs-after.json")
    args = data(run / "invocation.json")
    require(after == data(ROOT / "inputs-before.json") == args["inputs"],
            "source or executable pins differ across the run")
    require(args["suite_arguments"] == list(OBSERVER["SUITES"]) and args["injected_chrome_flags"] == list(OBSERVER["CHROME_FLAGS"]),
            "runner scope or the only approved process-local Chrome option changed")
    bundle = ROOT / "bundle"
    guard_paths = {role: str(run / "guards" / role) for role in OBSERVER["GUARD_ROLES"]}
    command = OBSERVER["stock_command"](run, bundle / "debug/surface-e2e", bundle, guard_paths)
    require(args["argv"] == command, "recorded stock executable, asset and launch options differ")
    require(after["release"] == OBSERVER["read_release_pin"](ROOT)
            and after["source_manifest_sha256"] == OBSERVER["SOURCE_SHA"]
            and after["source_files_checked_per_origin"] == OBSERVER["SOURCE_FILES"], "wrong exact release proof")
    require(checksum(ROOT / "browser-source.sha256") == after["source_manifest_sha256"],
            "reported source manifest hash differs")
    exact_inputs = OBSERVER["verify_inputs"]((ROOT / "bundle/debug/surface-e2e").resolve(),
                                             after["runner_sha256"], ROOT)
    require(exact_inputs == after, "archived native, Web or source inputs differ")
    focused = focused_receipt(ROOT / "browser-source.sha256", ROOT / "bundle/web/release/material-gallery")
    require(focused == data(ROOT / "focused-proof.json")
            and after["focused_proof_sha256"] == checksum(ROOT / "focused-proof.json"),
            "independent focused headless receipt or canonical Web binding differs")
    receipt = run / "headless-window-observation.json"
    observation = data(receipt)
    ledger = [data(path) for path in (run / "guard-ledger").glob("*.json")]
    require(sorted(ledger, key=lambda event: event["pid"]) ==
            sorted(observation["guard_events"], key=lambda event: event["pid"]), "guard ledger differs")
    stock = OBSERVER["verify_run"](run, observation, after["release"])
    require(json.loads(json.dumps(stock)) == data(run / "verified-label-motion-stock.json"),
            "offline replay differs from the actual receipt")
    require(checksum(PREVIOUS) == EXPECTED_PREVIOUS, "retained verifier changed")
    prior = run_path(str(PREVIOUS))
    retained = prior["retained"](run, ROOT / "browser-source.sha256", after["source_manifest_sha256"], OBSERVER["LAST_MATERIAL"])
    pixels = prior["slider_pixels"](run / "artifacts")
    result = {**stock, "release": after["release"], "retained": retained, "standard_pixels": pixels,
              "focused": focused, "injected_chrome_flags": args["injected_chrome_flags"],
              "window_receipt_sha256": checksum(receipt), "runner_log_sha256": checksum(run / "runner.log")}
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main(Path(sys.argv[1]).resolve(strict=True))
