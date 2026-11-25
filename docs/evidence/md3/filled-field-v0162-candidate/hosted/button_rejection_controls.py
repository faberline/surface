"""Reject wrong Common Buttons states and original Canvas evidence."""

import json
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from PIL import Image

from verify_button_pixels import CHECKED, EARLY, IMAGES, REPORT, REST, verify
from verify_forms_pixels import checksum


def rejected(action, expected):
    try:
        action()
    except (ValueError, AssertionError, RuntimeError) as error:
        if expected not in str(error):
            raise
        return True
    raise AssertionError(f"Common Buttons rejection control passed: {expected}")


def controls(raw, log, measured):
    names = (REPORT, *IMAGES)
    checksums = {name: checksum(raw / name) for name in names}
    with TemporaryDirectory(prefix="surface-buttons-controls.", dir="/tmp") as directory:
        temp = Path(directory)
        broken_log = temp / "missing-step.log"
        broken_log.write_text(log.read_text().replace("[M319]", "[M318]"))
        wrong_order = rejected(lambda: verify(raw, measured, log=broken_log), "wrong ordered Common Buttons checkpoints")
        for name in names:
            copyfile(raw / name, temp / name)

        payload = json.loads((raw / REPORT).read_text())
        payload["held"]["after_touch_cancel"] = 2
        (temp / REPORT).write_text(json.dumps(payload))
        wrong_cancel = rejected(lambda: verify(temp, measured, log=log), "mouse release or touch cancellation receipt differs")

        payload = json.loads((raw / REPORT).read_text())
        payload["checked"]["states"][0]["requests"] = 2
        (temp / REPORT).write_text(json.dumps(payload))
        duplicate = rejected(lambda: verify(temp, measured, log=log), "wrong toggle state or activation count")

        copyfile(raw / REPORT, temp / REPORT)
        copyfile(raw / REST, temp / EARLY)
        reused_frame = rejected(lambda: verify(temp, measured, log=log), "motion PNG does not prove an intermediate frame")
        copyfile(raw / EARLY, temp / EARLY)

        with Image.open(raw / CHECKED) as original:
            altered = original.convert("RGB")
        face = payload["checked"]["states"][0]["geometry"]["face"]
        point = (int(2 * (face[0] + face[2] / 2)), int(2 * (face[1] + 44 + 3.5)))
        altered.putpixel(point, (255, 0, 255))
        altered.save(temp / CHECKED)
        wrong_pixel = rejected(lambda: verify(temp, measured, log=log), "checked face or label paint differs")
    if {name: checksum(raw / name) for name in names} != checksums:
        raise AssertionError("Rejection controls modified original Common Buttons evidence")
    return {"wrong_step_order": wrong_order, "extra_touch_cancel_request": wrong_cancel,
            "duplicate_toggle_request": duplicate, "reused_motion_frame": reused_frame,
            "one_changed_checked_pixel": wrong_pixel}
