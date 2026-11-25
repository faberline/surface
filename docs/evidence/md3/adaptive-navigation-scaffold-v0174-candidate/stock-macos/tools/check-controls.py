"""Check exact q74 source and known q73/R4 originals without app builds or launches."""

from argparse import Namespace
from pathlib import Path
from runpy import run_path
from tempfile import TemporaryDirectory
import json
import subprocess
import sys

from evidence_common import data, digest, require, seal
import stock_config as config
from verify_adaptive_navigation_stock import scoped, verify as adaptive

ROOT = Path(__file__).resolve().parent
Q73 = Path("/tmp/surface-md3-horizontal-nav-stock-v0173.5buwihck")
Q73_HARNESS = "1bc8c5e7d6d84ec27f8bea662172879502b872f614b507ca73f20f8f22039da7"
Q73_STOCK = "c224fd0156c540998b85eb01d5cbd540c4ed4da7a3fc2f8e816551f69f2bc250"
OBS = run_path(str(ROOT / "observe-multiline.py"))
PREP = run_path(str(ROOT / "prepare-pins.py"))


def rejected(callback, expected):
    try:
        callback()
    except (ValueError, RuntimeError, FileNotFoundError) as error:
        require(expected in str(error), f"Wrong negative result: {error}")
    else:
        raise ValueError(f"Invalid evidence accepted: {expected}")


def q73_originals():
    verify = Q73 / "verify-stock-receipt.py"
    seal(Q73, "harness.sha256", Q73_HARNESS)
    receipt = Q73 / "stock-mac-exact-offline.json"
    require(digest(receipt) == Q73_STOCK, "Wrong prior actual stock receipt")
    previous = subprocess.run(
        [sys.executable, "-E", "-B", str(verify), str(Q73 / "run")], check=True, capture_output=True,
    ).stdout
    require(previous == receipt.read_bytes(), "The original q73 full stock receipt did not reproduce")
    previous = json.loads(previous)
    require(previous["cases"]["material"]["steps"] == 364 and previous["total_checks"] == 30490
            and len(previous["navigation_bars"]["horizontal"]["pngs"]) == 3,
            "Wrong preceding actual stock proof")
    return previous


def receipts(previous, focus, new, pins):
    # A synthetic envelope around the two actual runs tests the final shell guard.
    proof = {**previous, "release": config.release_pin(pins), "focused": {**focus, "canonical_gallery_match": True},
             "adaptive_navigation": new, "cases": {**previous["cases"],
                                                  "material": {**previous["cases"]["material"], "steps": 368}}}
    with TemporaryDirectory(dir=ROOT) as tmp:
        r, p = (Path(tmp) / name for name in ("receipt.json", "pins.json"))
        p.write_text(json.dumps(pins))

        def check(report):
            r.write_text(json.dumps(report))
            return subprocess.run(["sh", ROOT / "check-receipt.sh", p, r], check=False,
                                  capture_output=True).returncode

        require(check(proof) == 0, "The shell refused the fully formed control")
        require(check({**proof, "adaptive_navigation": {**new, "page_scroll": 1}}) != 0,
                "The shell accepted page scroll in the rail-only test")
        require(check({**proof, "adaptive_navigation": {**new, "pngs": {}}}) != 0,
                "The shell accepted missing adaptive screenshots")
        require(check({**proof, "focused": {**proof["focused"], "checks": 199}}) != 0,
                "The shell accepted incomplete scoped checks")


def main():
    OBS["self_test"]()
    previous = q73_originals()
    pins = PREP["source_pins"]()
    exact = config.ADAPTIVE_STAGE / "exact-candidate"
    args = Namespace(source_manifest=exact / "source.sha256", patch=exact / config.PATCH_NAME,
                     native_source=exact / "source", web_source=exact / "source")
    source, _ = PREP["source_inputs"](args, pins)
    focus = scoped(args.source_manifest)
    require(focus["canonical_gallery_match"] is None, "Preflight cannot claim unbuilt exact Gallery")
    gallery = config.ADAPTIVE_STAGE / "web-development/material-gallery"
    require(len(PREP["canonical_dist"](gallery, "material-gallery", [])) == 4,
            "Known actual R4 Gallery descriptor is incomplete")
    rejected(lambda: PREP["canonical_dist"](gallery, "material-gallery", ["devtools"]),
             "wrong canonical Web inventory or profile")
    rejected(lambda: OBS["verify_runner_log"]((Q73 / "run/runner.log").read_text()), "Material steps changed")
    rejected(lambda: adaptive(Q73 / "run/artifacts", previous), "material-adaptive-navigation.json")
    scoped_run = config.ADAPTIVE_FOCUS / "development-r4"
    new = adaptive(scoped_run / "artifacts", {"browser": previous["browser"], "adapter": previous["adapter"]})
    require(new["pngs"] == focus["pictures"], "Stock decoder disagrees on real R4 originals")
    receipts(previous, focus, new, pins)
    print(json.dumps({"scope": "offline controls only", "source_files": len(source), "source_tree": pins["source_tree"],
                      "retained_stock_checks": previous["total_checks"], "focused_checks": focus["checks"],
                      "focused_steps": focus["steps"], "focused_original_pngs": len(focus["pictures"]),
                      "rail_scroll": new["rail_scroll"], "page_scroll": new["page_scroll"],
                      "future_canonical_gallery_checked": False, "future_exact_stock_launched": False,
                      "rejected": ["wrong canonical profile", "q73 step sequence", "missing adaptive raw report",
                                   "changed page scroll", "missing screenshots", "wrong focused checks"]}, indent=2))


if __name__ == "__main__":
    main()
