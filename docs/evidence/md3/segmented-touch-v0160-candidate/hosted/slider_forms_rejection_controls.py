"""Reject altered copies of the four stock Slider Forms specimens."""

import json
from math import floor
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from PIL import Image

from verify_forms_pixels import CAPTURES, checksum
from verify_slider_forms_report import IMAGES, REPORT, validate


def update(root, change):
    path = root / REPORT
    receipt = json.loads(path.read_text())
    change(receipt)
    path.write_text(json.dumps(receipt))


def controls(raw, log, metadata):
    result = {}

    def reject(label, expected, modify):
        with TemporaryDirectory(prefix="surface-slider-forms-control.", dir="/tmp") as folder:
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
                raise AssertionError(f"accepted altered Forms evidence: {label}")

    def omit_step(root):
        path = root / "runner.log"
        log_text = path.read_text()
        assert log_text.count("[M305]") == 1
        path.write_text(log_text.replace("[M305]", "[M304]"))

    def stop_pixel(root):
        report = json.loads((root / REPORT).read_text())
        x, y, width, height = report["centered"]["sliders"][0]["stops"][0]
        path = root / CAPTURES["centered"][0]
        with Image.open(path) as image:
            altered = image.convert("RGB")
        point = floor(2 * (x + width / 2)), floor(2 * (44 + y + height / 2))
        r, g, b = altered.getpixel(point)
        altered.putpixel(point, (255 - r, 255 - g, 255 - b))
        altered.save(path)

    reject("m305_dropped", "missing, repeated or out-of-order", omit_step)
    reject("wrong_gpu", "WebGPU adapter disagrees", lambda p: update(p, lambda r: r["adapter"].update(vendor="wrong")))
    reject("wrong_capture_dimensions", "PNG dimensions differ", lambda p: copyfile(
        p / CAPTURES["centered"][0], p / CAPTURES["compact"][0]))
    reject("localized_vertical_value_changed", "wrong localized vertical accessible values", lambda p: update(
        p, lambda r: r["compact"]["aria"].__setitem__(3, "25")))
    reject("value_lost_during_resize", "vertical held value or resize cancel differs", lambda p: update(
        p, lambda r: r["vertical_mouse"].update(value_after_resize=50)))
    reject("wrong_compact_thumb_geometry", "Forms hit or grip size differs", lambda p: update(
        p, lambda r: r["compact"]["geometry"]["rail"].__setitem__(3, 210)))
    reject("one_recolored_centered_stop_pixel", "inactive stop ink differs", stop_pixel)
    return {"scope": "mutated local copies only", "original_report_sha256": checksum(raw / REPORT), "rejected": result}
