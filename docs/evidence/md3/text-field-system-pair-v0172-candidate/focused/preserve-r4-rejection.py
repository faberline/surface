"""Keep the original guarded q72 R4 caret-edge rejection as offline evidence."""

import argparse
import json
from pathlib import Path
import re
from runpy import run_path

from PIL import Image

from evidence_common import copy, copy_members, data, digest, json_file, path_entries, require, seal, verify_ledgers

HERE = Path(__file__).resolve().parent
OLD = Path("/tmp/surface-md3-field-pair-focused-v0172-r4.tnCHn0zc")
LAUNCHER = "ca93fdef342b8a42554f5b5b80adf938c16c8dbb9b36e1219a22cba3d2bb9de9"
WEB = "84359bf859a0582b2da51875f0e6bd279641a0afe28935b0b38ecb2c0cb1f3b1"
SOURCE = "5c342f40941e6f5350411c7787faed8651356f8f739bd64f059bdb65622e8649"
STEPS = [f"M{i}" for i in (*range(241, 248), *range(357, 362))]
PREFIX = "browser-material-text-field-system-pair"
ARTIFACTS = (f"{PREFIX}.json", *(f"{PREFIX}-{tag}.png" for tag in
             ("real-light", "real-dark", "outlined-normal", "outlined-selected", "outlined-caret", "failure")))
RAW = ("runner.log", "headless-window-observation.json", "invocation.json", "inputs-after.json", "owned-shutdown.json")


def review(root):
    run = root / "development-r4"
    text = (run / "runner.log").read_text()
    report = data(run / "artifacts" / ARTIFACTS[0])
    require(re.findall(r"^  \[(M\d+)\]", text, re.M) == STEPS
            and "0 passed; 1 failed; 1669 checks" in text and "last step: M361" in text
            and "expected [249, 220, 126], got [248, 219, 126]" in report["failure"]
            and report["page_errors"] == [], "Wrong original R4 rejection")
    watch = data(run / RAW[1])
    require(watch["stock_exit"] == 1 and watch["visible_owned_windows"] == watch["remaining_observed_processes"]
            == watch["problems"] == data(run / RAW[4])["after_cleanup"] == [], "Wrong R4 process failure scope")
    verify_ledgers(run, watch, {"chrome-primary"})
    inputs = data(run / RAW[2])["inputs"]
    require(inputs == data(run / RAW[3]) and inputs["web_manifest_sha256"] == WEB
            and inputs["product_source_manifest_sha256"] == inputs["web_producer_manifest_sha256"] == SOURCE
            and inputs["runner_sha256"] == (root / "runner.sha256").read_text().strip(), "Original R4 inputs changed")
    with Image.open(run / "artifacts" / f"{PREFIX}-outlined-caret.png") as raw:
        require(raw.format == "PNG" and raw.width == 1920, "Wrong original R4 caret capture")
        image = raw.convert("RGB")
    row = [[x, list(image.getpixel((x, 344)))] for x in (151, 152, 153)]
    require(row == [[151, [248, 219, 126]], [152, [249, 220, 126]], [153, [19, 45, 47]]],
            "Original edge, solid interior and field samples differ")
    prior = run_path(str(HERE / "verify-field-contrast.py"))["verify_captures"](run / "artifacts")
    require(len(prior["png_sha256"]) == 15, "Retained GrayText contrast originals differ")
    return {"scope": "original R4 rejected caret sample", "run_exit": 1, "last_material_step": "M361", "checks": 1669,
            "visible_windows": 0, "remaining_owned_processes": [],
            "chrome_samples": watch["launch_observations"]["chrome-primary"]["samples"],
            "source_sha256": SOURCE, "web_sha256": WEB, "physical_row": 344, "physical_rgb_witness": row,
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
        require(actual == data(root / "verified-rejection.json"), "The moved R4 failure differs")
        result = {**actual, "archive_sha256": args.evidence_sha256}
    else:
        result = review(OLD)
        seal(OLD, "launcher.sha256", LAUNCHER)
        dest = args.out.resolve()
        require(dest.is_relative_to(HERE) and dest != HERE and not dest.exists(), "Use a fresh output under the R5 lane")
        dest.mkdir(mode=0o700)
        ledgers = [path.relative_to(OLD).as_posix() for path in (OLD / "development-r4/guard-ledger").glob("*.json")]
        copy_members(OLD, dest, ("runner.sha256", "launcher.sha256", "scope-pin.json", "original-source.sha256",
                                 "focused-source.sha256", "gallery-pin.json", *(f"development-r4/{name}" for name in RAW), *ledgers))
        for name in (*ARTIFACTS, "material-text-field-disabled-contrast.json", *result["retained_disabled_contrast"]["png_sha256"]):
            copy(OLD / "development-r4/artifacts" / name, dest / "development-r4/artifacts" / name)
        json_file(dest / "verified-rejection.json", result)
        manifest = dest / "SHA256SUMS"
        manifest.write_text("".join(f"{digest(path)}  {path.relative_to(dest).as_posix()}\n"
                                    for path in sorted(dest.rglob("*")) if path.is_file()))
        result = {**result, "archive_sha256": digest(manifest)}
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
