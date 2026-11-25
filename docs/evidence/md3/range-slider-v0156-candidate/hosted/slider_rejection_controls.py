"""Reject deliberately altered copies of actual Standard Slider evidence."""

import json
from math import floor
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from PIL import Image

from verify_sliders_report import COMPACT, IMAGES, KEYBOARD, REPORT, WIDE, digest, validate


def update(path, change):
    report = json.loads(path.read_text())
    change(report)
    path.write_text(json.dumps(report))


def controls(raw, log, metadata):
    result = {}

    def reject(label, expected, modify):
        with TemporaryDirectory(prefix="surface-slider-control.", dir="/tmp") as folder:
            scratch = Path(folder)
            for name in (REPORT, *IMAGES):
                copyfile(raw / name, scratch / name)
            runner = scratch / "runner.log"
            copyfile(log, runner)
            modify(scratch)
            try:
                validate(scratch, runner, metadata)
            except ValueError as error:
                assert expected in str(error), (label, str(error))
                result[label] = str(error)
            else:
                raise AssertionError(f"accepted modified Standard Slider evidence: {label}")

    def lose_final_step(folder):
        path = folder / "runner.log"
        text = path.read_text()
        assert text.count("[M301]") == 1
        path.write_text(text.replace("[M301]", "[M297]"))

    def recolor_stop(folder):
        report = json.loads((folder / REPORT).read_text())
        x, y, w, h = report["wide"]["stop"]
        path = folder / WIDE
        with Image.open(path) as original:
            pixel_copy = original.convert("RGB")
        pixel_copy.putpixel((floor(2 * (x + w / 2)), floor(2 * (44 + y + h / 2))), (0, 0, 0))
        pixel_copy.save(path)

    reject("missing_m301", "missing, duplicate or out-of-order Standard Slider step", lose_final_step)
    reject("wrong_capture_dimensions", "wrong Standard Slider screenshot dimensions", lambda p: copyfile(p / WIDE, p / COMPACT))
    reject("duplicate_capture", "duplicate Standard Slider screenshot pixels", lambda p: copyfile(p / WIDE, p / KEYBOARD))
    reject("false_ax_value_text", "numeric or localized accessibility value differs", lambda p: update(
        p / REPORT, lambda r: r["compact"].update(ax=["30", "30"])))
    reject("bad_hardware_geometry", "track, handle or target dimensions differ", lambda p: update(
        p / REPORT, lambda r: r["wide"]["geometry"][4]["active"].__setitem__(3, 95)))
    reject("break_cancel", "continuous drag, cancellation, discrete touch or disabled counts differ", lambda p: update(
        p / REPORT, lambda r: r["pointer"].update(canceled=0)))
    reject("disabled_activates", "continuous drag, cancellation, discrete touch or disabled counts differ", lambda p: update(
        p / REPORT, lambda r: r["pointer"]["requests"].__setitem__(5, 1)))
    reject("back_focus_lost", "compact Slider paint or return focus differs", lambda p: update(
        p / REPORT, lambda r: r["compact"].update(back_focus="canvas")))
    reject("recolor_one_actual_stop_pixel", "decoded ring, dot, label or dark track paint differs", recolor_stop)
    return {"scope": "mutated local copies only", "original_report_sha256": digest(raw / REPORT), "rejected": result}
