#!/usr/bin/env python3
"""Read the saved headless boundary evidence without starting Chrome."""

import argparse
from pathlib import Path
from runpy import run_path

from evidence_common import data, require

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("browser_evidence", type=Path)
args = parser.parse_args()
root = args.browser_evidence.resolve(strict=True)
browser = run_path(str(Path(__file__).with_name("browser.py")))
observation = data(root / "headless-window-observation.json")
result = browser["verify_run"](root, observation)
require(result == data(root / "verified-stock-material.json"), "Saved Browser result differs")
print(f"PASS {result['steps']} steps, {result['checks']} checks and {result['verified_png_samples']} screenshot samples")
