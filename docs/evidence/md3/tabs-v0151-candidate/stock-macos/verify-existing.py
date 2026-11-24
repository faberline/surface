#!/usr/bin/env python3
"""Check an existing guarded stock run without starting any process."""

import argparse
from hashlib import sha256
import json
from pathlib import Path
import re
from runpy import run_path
import sys

from PIL import Image


ROOT = Path(__file__).resolve().parent
OBS = run_path(str(ROOT / "observe-tabs.py"))
RETAINED = ROOT / "retained"
RETAINED_SHA = "5a9c99e2bf64a76375d81d5f16ab58228410be56ba32225b01930df949a0ac6f"
require = OBS["require"]
digest = OBS["sha"]
read_json = OBS["read_json"]


def retained(run, manifest, expected_sha, last_material):
    require(digest(RETAINED / "SHA256SUMS") == RETAINED_SHA, "wrong retained verifier manifest")
    OBS["verify_entries"](RETAINED, OBS["checksums"](RETAINED / "SHA256SUMS"))
    sys.path.insert(0, str(RETAINED))
    from verify_checkbox_report import validate as checkbox
    from verify_divider_report import validate as divider
    from verify_ime_cancel_report import validate as ime
    from verify_menu_report import validate as menu
    from verify_navigation_bar import mac_observation, validate_log, validate_navigation
    from verify_radio_report import validate as radio
    from verify_switch_report import validate as switch

    require(digest(manifest) == expected_sha, "wrong provenance manifest")
    source_count = len(OBS["checksums"](manifest))
    artifacts = run / "artifacts"
    log = run / "runner.log"
    raw_log = log.read_text()
    window = run / "headless-window-observation.json"
    side = read_json(artifacts / "browser-material-side-sheet.json")
    smoke = read_json(artifacts / "browser-smoke.json")
    info = {"browser": side["browser"], "adapter": side["adapter"]}
    require(side["errors"] == smoke["errors"] == [] and smoke["browser"] == info["browser"]
            and {key: smoke["adapter"][key] for key in info["adapter"]} == info["adapter"],
            "retained browser metadata disagrees")
    count, total = OBS["verify_runner_log"](raw_log, last_material)
    common = (artifacts, log, "stock-macos", "metal", manifest, expected_sha)
    events = {
        "radio": radio(*common, stock_last_material=last_material),
        "checkbox": checkbox(*common, stock_last_material=last_material, source_file_count=source_count),
        "switch": switch(*common, stock_last_material=last_material),
    }
    for component, indexes in (("radio", range(282, 286)), ("checkbox", range(278, 282)), ("switch", range(274, 278))):
        result = events[component]
        require(result["runner"]["cases"] == count and result["runner"]["total_checks"] == total
                and result["runner"]["browser"] == info["browser"]
                and result["runner"][f"{component}_steps"] == [f"M{i}" for i in indexes],
                f"retained {component} coverage disagrees")
        require(result.get("adapter", result.get("observed_adapter")) == info["adapter"],
                f"retained {component} adapter disagrees")

    cases, steps, checked, browsers = validate_log(raw_log, "macos", True, last_material)
    nav = read_json(artifacts / "browser-material-flexible-navigation-bar.json")
    require(cases == count and checked == total and browsers == {info["browser"]}
            and nav["browser"] == info["browser"] and nav["adapter"] == info["adapter"]
            and nav["errors"] == [] and nav["reduced_motion"] is True,
            "retained Navigation metadata differs")
    events["navigation"] = {"material_steps": len(steps), **validate_navigation(nav, artifacts)}
    events["menu"] = menu(artifacts, platform="macos", log=log, scope="stock",
                          window_receipt=window, expected_metadata=info, stock_material_steps=len(steps))
    events["divider"] = divider(artifacts, info["browser"], info["adapter"], log=raw_log)
    events["ime_cancellation"] = ime(artifacts)
    return {**info, "checks": total, "retained": events, "window_observation": mac_observation(window, True)}


def existing(run, stock):
    pin = OBS["read_release_pin"](stock)
    manifest = stock / "browser-source.sha256"
    require(digest(manifest) == pin["source_sha256"]
            and len(OBS["checksums"](manifest)) == pin["source_files"], "source manifest is not the run pin")
    OBS["verify_entries"](stock, OBS["checksums"](stock / "sealed-inputs.sha256"))
    invocation = read_json(run / "invocation.json")
    inputs = invocation["inputs"]
    require(inputs == read_json(stock / "inputs-before.json") == read_json(run / "inputs-after.json"),
            "runtime sealed inputs differ before or after the run")
    require(inputs["release"] == pin
            and inputs["app_bundle_seal_sha256"] == digest(stock / "sealed-inputs.sha256")
            and inputs["stock_files"] == OBS["checksums"](stock / "browser-binary-sha256.txt")
            and inputs["runner_sha256"] == (stock / "approved-runner.sha256").read_text().strip(),
            "different source, binary or runner authorization")
    require(tuple(invocation["suite_arguments"]) == OBS["SUITES"]
            and tuple(invocation["argv"][-5:]) == OBS["SUITES"]
            and invocation["injected_chrome_flags"] == [], "wrong explicit headless invocation")
    observation = read_json(run / "headless-window-observation.json")
    ledger = [read_json(path) for path in sorted((run / "guard-ledger").glob("*.json"))]
    require(sorted(ledger, key=lambda event: event["pid"]) ==
            sorted(observation["guard_events"], key=lambda event: event["pid"]), "original guard ledger differs")
    for event in ledger:
        if event["role"] == "native":
            require(event["argv"] == ["--headless", "--width", "720", "--height", "640"], "native not headless")
        else:
            require([arg for arg in event["argv"] if arg == "--headless" or arg.startswith("--headless=")]
                    == ["--headless=new"], "Chrome not exclusively headless")

    stock_receipt = OBS["verify_run"](run, observation, pin)
    require(json.loads(json.dumps(stock_receipt)) == read_json(run / "verified-tabs-stock.json"),
            "the original stock receipt disagrees")
    previous = retained(run, manifest, pin["source_sha256"], OBS["LAST_MATERIAL"])
    require(previous["checks"] == stock_receipt["total_checks"], "retained stock totals differ")
    captured = {}
    for name, expected in stock_receipt["tabs_pngs"].items():
        path = run / "artifacts" / name
        with Image.open(path) as opened:
            image = opened.convert("RGB")
            require(image.size == expected["size"] and digest(path) == expected["sha256"],
                    f"Tabs PNG differs from stock receipt: {name}")
            extrema = image.getextrema()
            require(any(low < high for low, high in extrema), f"Tabs PNG is blank: {name}")
            captured[name] = {**expected, "decoded_rgb_sha256": sha256(image.tobytes()).hexdigest()}
    require(len({entry["decoded_rgb_sha256"] for entry in captured.values()}) == 4, "Tabs PNGs reuse decoded pixels")
    return {"release": pin, "cases": stock_receipt["cases"], "total_checks": stock_receipt["total_checks"],
            "browser": stock_receipt["browser"], "adapter": stock_receipt["adapter"], "tabs_pngs": captured,
            "tabs_report_sha256": stock_receipt["tabs_report_sha256"], "retained": previous,
            "window_receipt_sha256": digest(run / "headless-window-observation.json"),
            "runner_log_sha256": digest(run / "runner.log"),
            "stock_receipt_sha256": digest(run / "verified-tabs-stock.json")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--stock", type=Path, default=ROOT)
    parser.add_argument("--retained-only", action="store_true", help="read an older actual proof as a control")
    parser.add_argument("--last-material", type=int)
    parser.add_argument("--source-manifest", type=Path)
    parser.add_argument("--source-sha256")
    args = parser.parse_args()
    if args.retained_only:
        if not (args.source_manifest and args.last_material and args.source_sha256
                and re.fullmatch(r"[a-f0-9]{64}", args.source_sha256)):
            parser.error("Retained control requires --source-manifest, --source-sha256 and --last-material.")
        result = retained(args.run, args.source_manifest, args.source_sha256, args.last_material)
    else:
        if any((args.last_material, args.source_manifest, args.source_sha256)):
            parser.error("The final run uses only its recorded release and source pins.")
        result = existing(args.run, args.stock)
    print(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
