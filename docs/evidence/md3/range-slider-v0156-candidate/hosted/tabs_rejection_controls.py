"""Exercise the Tabs evidence verifier on altered local copies only."""

import json
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from verify_tabs_report import IMAGES, REPORT, STEM, digest, validate


def replace_json(path, change):
    edited = json.loads(path.read_text())
    change(edited)
    path.write_text(json.dumps(edited))


def controls(raw, log, metadata):
    results = {}

    def reject(name, reason, mutation):
        with TemporaryDirectory(prefix="surface-tabs-evidence-control.", dir="/tmp") as directory:
            scratch = Path(directory)
            for asset in (REPORT, *IMAGES):
                copyfile(raw / asset, scratch / asset)
            copyfile(log, scratch / "runner.log")
            mutation(scratch)
            try:
                validate(scratch, scratch / "runner.log", metadata)
            except ValueError as error:
                assert reason in str(error), (name, str(error))
                results[name] = str(error)
            else:
                raise AssertionError(f"accepted altered Tabs evidence: {name}")

    def changed_step(path):
        run = path / "runner.log"
        original = run.read_text()
        assert original.count("[M289]") == 1
        run.write_text(original.replace("[M289]", "[M285]", 1))

    reject("missing_m289", "missing, duplicate or out-of-order", changed_step)
    reject("hidden_panel_exposed", "bad Tabs mount or placeholder", lambda path: replace_json(
        path / REPORT, lambda report: report["fixed"]["before"][1].update(hidden=False)))
    reject("duplicate_target_id", "unbound or ambiguous Tabs relationship", lambda path: replace_json(
        path / REPORT, lambda report: report["fixed"]["after"][2].update(matches=2)))
    reject("wrong_chrome_adapter", "independent Chrome adapter mismatch", lambda path: replace_json(
        path / REPORT, lambda report: report["adapter"].update(isFallbackAdapter="true")))
    reject("page_error", "Tabs failure, page errors", lambda path: replace_json(
        path / REPORT, lambda report: report["page_errors"].append("synthetic error")))
    reject("fake_press_color", "held press pixels changed", lambda path: replace_json(
        path / REPORT, lambda report: report["automatic"].update(pressed_ink=[255, 0, 1])))
    reject("wrong_rtl_size", "wrong Tabs PNG dimensions", lambda path: copyfile(
        path / f"{STEM}-fixed-dark.png", path / f"{STEM}-rtl-320.png"))
    reject("duplicate_dark_frame", "duplicate Tabs PNG pixels", lambda path: copyfile(
        path / f"{STEM}-fixed-dark.png", path / f"{STEM}-dark-pressed.png"))
    return {"scope": "mutated local copies only", "original_report_sha256": digest(raw / REPORT), "rejected": results}
