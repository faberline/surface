"""Keep the original guarded q72 R5 Highlight-alpha rejection as offline evidence."""

import argparse
import json
from pathlib import Path
import re
from runpy import run_path

from PIL import Image

from evidence_common import copy, copy_members, data, digest, json_file, path_entries, require, seal, verify_ledgers

HERE = Path(__file__).resolve().parent
OLD = Path("/tmp/surface-md3-field-pair-focused-v0172-r5.bce6QeJB")
LAUNCHER = "dddb7a29b9b908e7dc65ec135caad94b8e9cfd516e61a6a8ddc2b3213943beb2"
WEB = "84359bf859a0582b2da51875f0e6bd279641a0afe28935b0b38ecb2c0cb1f3b1"
SOURCE = "80ae80c08efe8025bd1765154fa32343934cc4dcde8902a42a52cd2ce2ad5683"
PRODUCER = "5c342f40941e6f5350411c7787faed8651356f8f739bd64f059bdb65622e8649"
STEPS = [f"M{i}" for i in (*range(241, 248), *range(357, 362))]
PREFIX = "browser-material-text-field-system-pair"
ARTIFACTS = (f"{PREFIX}.json", *(f"{PREFIX}-{tag}.png" for tag in
             ("real-light", "real-dark", "outlined-normal", "outlined-selected", "outlined-caret", "failure")))
RAW = ("runner.log", "headless-window-observation.json", "invocation.json", "inputs-after.json", "owned-shutdown.json")


def review(root):
    run = root / "development-r5"
    text = (run / "runner.log").read_text()
    report = data(run / "artifacts" / ARTIFACTS[0])
    require(re.findall(r"^  \[(M\d+)\]", text, re.M) == STEPS
            and "0 passed; 1 failed; 1675 checks" in text and "last step: M361" in text
            and "the sampled system ink or backdrop is opaque: expected 255, got 204" in report["failure"]
            and report["page_errors"] == [], "Wrong original R5 rejection")
    watch = data(run / RAW[1])
    require(watch["stock_exit"] == 1 and watch["visible_owned_windows"] == watch["remaining_observed_processes"]
            == watch["problems"] == data(run / RAW[4])["after_cleanup"] == [], "Wrong R5 process failure scope")
    verify_ledgers(run, watch, {"chrome-primary"})
    inputs = data(run / RAW[2])["inputs"]
    require(inputs == data(run / RAW[3]) and inputs["web_manifest_sha256"] == WEB
            and inputs["product_source_manifest_sha256"] == SOURCE
            and inputs["web_producer_manifest_sha256"] == PRODUCER
            and inputs["web_compatible_test_only_delta"] == ["e2e/src/material_text_field_system_pair.rs"]
            and inputs["runner_sha256"] == (root / "runner.sha256").read_text().strip(), "Original R5 inputs changed")
    highlight = report["real"][0]["accepted"]["highlight"]["background"]
    field = [19, 45, 47]
    mode = "byte-space source-over"
    calculate = run_path(str(HERE / "verify-system-pair.py"))["source_over"]
    result = calculate(highlight, field, mode)
    require(highlight == [5, 0, 73, 204] and result == [8, 9, 68], "Wrong actual Highlight composition")
    with Image.open(run / "artifacts" / f"{PREFIX}-outlined-selected.png") as raw:
        require(raw.format == "PNG" and raw.width == 1920 and list(raw.convert("RGB").getpixel((116, 330))) == result,
                "The original rejected capture does not show the precise composited Highlight")
    with Image.open(run / "artifacts" / f"{PREFIX}-outlined-caret.png") as raw:
        require(list(raw.convert("RGB").getpixel((152, 344))) == [249, 220, 126],
                "The original corrected caret sample lost its exact FieldText pixel")
    prior = run_path(str(HERE / "verify-field-contrast.py"))["verify_captures"](run / "artifacts")
    require(len(prior["png_sha256"]) == 15, "Retained GrayText contrast originals differ")
    return {"scope": "original R5 rejected opaque Highlight assumption", "run_exit": 1, "last_material_step": "M361", "checks": 1675,
            "visible_windows": 0, "remaining_owned_processes": [],
            "chrome_samples": watch["launch_observations"]["chrome-primary"]["samples"],
            "source_sha256": SOURCE, "web_sha256": WEB, "sample_physical_xy": [116, 330],
            "highlight_source_rgba": highlight, "field_base_rgb": field, "mode": mode, "composited_rgb": result,
            "retained_disabled_contrast": prior}


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    mode = cli.add_mutually_exclusive_group(required=True)
    mode.add_argument("--out", type=Path)
    mode.add_argument("--verify", type=Path)
    cli.add_argument("--evidence-sha256")
    args = cli.parse_args()
    if args.verify:
        root = args.verify.resolve(strict=True)
        seal(root, "SHA256SUMS", args.evidence_sha256, exact=True)
        actual = review(root)
        require(actual == data(root / "verified-rejection.json"), "The moved R5 failure differs")
        result = {**actual, "archive_sha256": args.evidence_sha256}
    else:
        result = review(OLD)
        seal(OLD, "launcher.sha256", LAUNCHER)
        dest = args.out.resolve()
        require(dest.is_relative_to(HERE) and dest != HERE and not dest.exists(), "Use a fresh output under the R5 lane")
        dest.mkdir(mode=0o700)
        ledgers = [path.relative_to(OLD).as_posix() for path in (OLD / "development-r5/guard-ledger").glob("*.json")]
        copy_members(OLD, dest, ("runner.sha256", "launcher.sha256", "scope-pin.json", "original-source.sha256",
                                 "focused-source.sha256", "gallery-pin.json", *(f"development-r5/{name}" for name in RAW), *ledgers))
        for name in (*ARTIFACTS, "material-text-field-disabled-contrast.json", *result["retained_disabled_contrast"]["png_sha256"]):
            copy(OLD / "development-r5/artifacts" / name, dest / "development-r5/artifacts" / name)
        json_file(dest / "verified-rejection.json", result)
        manifest = dest / "SHA256SUMS"
        manifest.write_text("".join(f"{digest(path)}  {path.relative_to(dest).as_posix()}\n"
                                    for path in sorted(dest.rglob("*")) if path.is_file()))
        result = {**result, "archive_sha256": digest(manifest)}
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
