"""Exercise Chips receipt rejections on disposable copies of original files."""

import json
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from verify_chips_report import IMAGES, REPORT, STEM, digest, validate


def update(path, change):
    edited = json.loads(path.read_text())
    change(edited)
    path.write_text(json.dumps(edited))


def controls(raw, log, metadata):
    results = {}

    def reject(label, reason, change):
        with TemporaryDirectory(prefix="surface-chips-evidence-control.", dir="/tmp") as folder:
            scratch = Path(folder)
            for name in (REPORT, *IMAGES):
                copyfile(raw / name, scratch / name)
            runner = scratch / "runner.log"
            copyfile(log, runner)
            change(scratch)
            try:
                validate(scratch, runner, metadata)
            except ValueError as error:
                assert reason in str(error), (label, str(error))
                results[label] = str(error)
            else:
                raise AssertionError(f"accepted altered Chips evidence: {label}")

    def missing_step(scratch):
        path = scratch / "runner.log"
        original = path.read_text()
        assert original.count("[M293]") == 1
        path.write_text(original.replace("[M293]", "[M289]", 1))

    reject("missing_m293", "missing, duplicate or out-of-order", missing_step)
    reject("false_capture_shape", "wrong Chips screenshot dimensions", lambda path: copyfile(
        path / f"{STEM}-wide-light.png", path / f"{STEM}-compact-dark-rtl-200.png"))
    reject("duplicate_capture_pixels", "duplicate Chips screenshot pixels", lambda path: copyfile(
        path / f"{STEM}-wide-light.png", path / f"{STEM}-wide-pressed.png"))
    reject("wrong_adapter_type", "Chrome adapter mismatch", lambda path: update(
        path / REPORT, lambda item: item["adapter"].update(isFallbackAdapter="true")))
    reject("page_error", "Chips failure, page errors", lambda path: update(
        path / REPORT, lambda item: item["page_errors"].append("synthetic error")))
    reject("missing_ax_cell", "wrong Chips AX action or ancestry", lambda path: update(
        path / REPORT, lambda item: item["wide"]["ax"][0].update(ancestry=["row", "grid"])))
    reject("touch_cancel_commits", "Chips mouse or touch change differs", lambda path: update(
        path / REPORT, lambda item: item["pointer"]["after_canceled_touch"].update(removals=1)))
    reject("wrapped_ink_disagrees", "Chips wrapped foreground pixels differ", lambda path: update(
        path / REPORT, lambda item: item["compact"].update(label_line_ink=[25, 25])))
    reject("broken_compact_nearest_focus", "Chips RTL nearest-row focus path differs", lambda path: update(
        path / REPORT, lambda item: item["compact"]["focus_path"].__setitem__(3, "chip-demo-disabled")))
    return {"scope": "mutated local copies only", "original_report_sha256": digest(raw / REPORT), "rejected": results}
