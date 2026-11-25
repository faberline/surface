"""Reject altered copies of original Icon Button evidence."""

import json
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from PIL import Image

from verify_icon_buttons_report import COMPACT, HELD, IMAGES, REPORT, WIDE, center, digest, validate


def update(path, change):
    report = json.loads(path.read_text())
    change(report)
    path.write_text(json.dumps(report))


def controls(raw, log, metadata):
    results = {}

    def reject(label, message, change):
        with TemporaryDirectory(prefix="surface-icon-button-control.", dir="/tmp") as dir_name:
            scratch = Path(dir_name)
            for name in (REPORT, *IMAGES):
                copyfile(raw / name, scratch / name)
            runner = scratch / "runner.log"
            copyfile(log, runner)
            change(scratch)
            try:
                validate(scratch, runner, metadata)
            except ValueError as error:
                assert message in str(error), (label, str(error))
                results[label] = str(error)
            else:
                raise AssertionError(f"accepted modified Icon Button evidence: {label}")

    def lost_step(folder):
        log_path = folder / "runner.log"
        text = log_path.read_text()
        assert text.count("[M297]") == 1
        log_path.write_text(text.replace("[M297]", "[M293]"))

    def recolor_pixel(folder):
        report = json.loads((folder / REPORT).read_text())
        x, _ = center(report["wide"]["geometry"][0]["face"])
        y = report["wide"]["geometry"][0]["face"][1] + 3
        with Image.open(folder / WIDE) as opened:
            image = opened.convert("RGB")
        image.putpixel((int(x * 2), int((y + 44) * 2)), (0, 0, 0))
        image.save(folder / WIDE)

    reject("missing_m297", "missing, duplicate or out-of-order Icon Button step", lost_step)
    reject("wrong_capture_shape", "wrong Icon Button screenshot dimensions", lambda p: copyfile(p / WIDE, p / COMPACT))
    reject("duplicate_capture_pixels", "duplicate Icon Button screenshot pixels", lambda p: copyfile(p / WIDE, p / HELD))
    reject("command_claims_checked", "wrong named Chrome AX button", lambda p: update(
        p / REPORT, lambda r: r["wide"]["ax"][0].update(pressed="true")))
    reject("disabled_claims_enabled", "wrong named Chrome AX button", lambda p: update(
        p / REPORT, lambda r: r["wide"]["ax"][2].update(disabled=False)))
    reject("lost_artwork_motion", "wrong Icon Button rest, held, release or Reduce Motion contours", lambda p: update(
        p / REPORT, lambda r: r["pointer"]["corner_pixels"].__setitem__(1, r["pointer"]["corner_pixels"][0])))
    reject("outside_release_activates", "wrong Icon Button cancellation", lambda p: update(
        p / REPORT, lambda r: r["pointer"].update(requests_after_cancel=1)))
    reject("missing_tooltip_description", "wrong or overlapping compact Icon Button tooltip", lambda p: update(
        p / REPORT, lambda r: r["compact"].update(description="")))
    reject("buffer_pixel_changed", "decoded Icon Button command_fill color differs", recolor_pixel)
    return {"scope": "mutated local copies only", "original_report_sha256": digest(raw / REPORT), "rejected": results}
