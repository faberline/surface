"""Check that the exact Navigation verifier rejects altered copies of passing evidence."""

import json
from pathlib import Path
import re
from shutil import copy2
from tempfile import TemporaryDirectory

from verify_navigation_bar import SHOTS, STEM, validate

REPORT = f"browser-{STEM}.json"
METADATA = "browser-material-side-sheet.json"


def replace_json(path, change):
    value = json.loads(path.read_text())
    change(value)
    path.write_text(json.dumps(value, ensure_ascii=False))


def replace_text(path, old, new):
    value = path.read_text()
    assert old in value
    path.write_text(value.replace(old, new, 1))


def copy_fixture(source, log, destination, platform):
    files = [REPORT, METADATA, "browser-smoke.json", *(f"{STEM}-{name}.png" for name in SHOTS)]
    if platform == "macos":
        files.extend(("headless-window-observation.json", "attempt.json"))
    for name in files:
        copy2(source / name, destination / name)
    copy2(log, destination / "runner.log")


def controls(source, log, *, platform, stock_last_material=273):
    source, log = Path(source), Path(log)
    arguments = {"platform": platform, "scope": "stock", "expect_swiftshader": platform == "linux",
                 "stock_last_material": stock_last_material}
    baseline = validate(source, log=log, expected_metadata=source / METADATA, **arguments)
    result = {}

    def rejected(case, expected, mutate):
        with TemporaryDirectory(dir="/tmp", prefix="surface-nav-evidence-control.") as temp:
            run = Path(temp)
            copy_fixture(source, log, run, platform)
            mutate(run)
            try:
                validate(run, expected_metadata=run / METADATA, **arguments)
            except (ValueError, OSError, SyntaxError) as error:
                assert expected in str(error), (case, str(error))
                result[case] = str(error)
            else:
                raise AssertionError(f"incorrectly accepted {case}")

    rejected("missing_m273", "missing, extra or out-of-order Material step",
             lambda run: replace_text(run / "runner.log", "[M273]", "[M274]"))
    total = baseline["total_checks"]
    rejected("incorrect_total", "passing suite totals",
             lambda run: replace_text(run / "runner.log", f"{total} checks;", f"{total + 1} checks;"))
    rejected("wrong_independent_adapter", "differs from the expected runtime metadata",
             lambda run: replace_json(run / METADATA, lambda doc: doc["adapter"].update(architecture="wrong-adapter")))
    rejected("nonboolean_fallback", "invalid observed WebGPU adapter fields",
             lambda run: replace_json(run / REPORT, lambda doc: doc["adapter"].update(isFallbackAdapter="true")))
    rejected("wrong_theme_capture", "decoded bar, pill or rounded corner pixels disagree",
             lambda run: copy2(run / f"{STEM}-light.png", run / f"{STEM}-dark.png"))
    rejected("unscrolled_capture", "wheel screenshots did not show changing content",
             lambda run: copy2(run / f"{STEM}-rtl-large.png", run / f"{STEM}-rtl-large-scrolled.png"))
    rejected("wrong_activation_count", "wrong keyboard or pointer activation",
             lambda run: replace_json(run / REPORT, lambda doc: doc["actions"].update(activations=4)))
    if platform == "linux":
        def replace_all_adapters(run):
            for name in (REPORT, METADATA, "browser-smoke.json"):
                replace_json(run / name, lambda doc: doc["adapter"].update(
                    vendor="apple", architecture="metal-3", isFallbackAdapter=False))
        rejected("wrong_linux_adapter", "did not match the SwiftShader pin", replace_all_adapters)
    else:
        rejected("visible_window", "stock window observer reports a failure, a window or a leftover process",
                 lambda run: replace_json(run / "headless-window-observation.json",
                                          lambda doc: doc["visible_owned_windows"].append(1)))
    assert len(result) == 8
    return {"platform":platform,"role":"mutated copies are rejection tests only",
            "original_report_sha256":baseline["report_sha256"],"rejected":result}
