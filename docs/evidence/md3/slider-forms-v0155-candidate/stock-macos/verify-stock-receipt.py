"""Recheck the stock headless Slider Forms run without loading a browser."""

import json
from pathlib import Path
from runpy import run_path
import sys

from verify_forms_pixels import checksum, require

ROOT = Path(__file__).resolve().parent
OBSERVER = run_path(str(ROOT / "observe-slider-forms.py"))
PREVIOUS = Path("/tmp/surface-md3-sliders-stock.lK6a9gKl/verify-existing.py")
EXPECTED_PREVIOUS = "eded5dfdfcf94dc901cb3608b83cc86a557fe4cb02b91122416c6a03e3568558"


def data(path):
    return json.loads(path.read_text())


def main(run):
    after = data(run / "inputs-after.json")
    args = data(run / "invocation.json")
    require(after == data(ROOT / "inputs-before.json") == args["inputs"],
            "source or executable pins differ across the run")
    require(args["suite_arguments"] == list(OBSERVER["SUITES"]) and args["injected_chrome_flags"] == [],
            "runner scope changed")
    require(after["release"]["version"] == "0.1.55" and after["release"]["last_material_step"] == 305,
            "wrong release proof")
    require(checksum(ROOT / "browser-source.sha256") == after["source_manifest_sha256"],
            "reported source manifest hash differs")
    receipt = run / "headless-window-observation.json"
    stock = OBSERVER["verify_run"](run, data(receipt), after["release"])
    require(json.loads(json.dumps(stock)) == data(run / "verified-slider-forms-stock.json"),
            "offline replay differs from the actual receipt")
    require(checksum(PREVIOUS) == EXPECTED_PREVIOUS, "retained verifier changed")
    prior = run_path(str(PREVIOUS))
    retained = prior["retained"](run, ROOT / "browser-source.sha256", after["source_manifest_sha256"], 305)
    pixels = prior["slider_pixels"](run / "artifacts")
    result = {**stock, "release": after["release"], "retained": retained, "standard_pixels": pixels,
              "window_receipt_sha256": checksum(receipt), "runner_log_sha256": checksum(run / "runner.log")}
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main(Path(sys.argv[1]).resolve(strict=True))
