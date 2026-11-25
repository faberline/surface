"""Reject wrong refinement steps, typed state and painted evidence."""

import json
from pathlib import Path
import re
from shutil import copyfile
from tempfile import TemporaryDirectory

from PIL import Image

from verify_refinement_pixels import CENTERED, INSET, CENTERED_IMAGES, verify
from verify_forms_pixels import checksum

STEPS = ("M310", "M311", "M312", "M313", "M314", "M315")


def verify_steps(text):
    actual = re.findall(r"^\s*\[(M\d+)\]", text, re.M)
    if tuple(step for step in actual if step in STEPS) != STEPS:
        raise AssertionError("Slider Refinements steps are missing, repeated or out of order")


def replay_steps(log):
    verify_steps(log.read_text())


def reject(incorrect, message):
    try:
        incorrect()
    except (ValueError, AssertionError, RuntimeError) as error:
        if message not in str(error):
            raise
        return True
    raise AssertionError(f"Refinement rejection control passed: {message}")


def with_changed_report(raw, temp, name, change):
    payload = json.loads((raw / name).read_text())
    change(payload)
    (temp / name).write_text(json.dumps(payload))


def bad_orientation(report):
    report["wide"]["samples"][0]["aria"][2] = "horizontal"


def extra_request(report):
    report["keys"]["values"]["requests"] = 4


def controls(raw, log, measured):
    replay_steps(log)
    failed_order = reject(
        lambda: verify_steps(log.read_text().replace("[M315]", "[M314]")),
        "missing, repeated or out of order",
    )
    expected = measured["slider_refinements"]
    originals = {name: checksum(raw / name) for name in (CENTERED, INSET, *expected["captures"])}
    with TemporaryDirectory(prefix="surface-slider-refinements-controls.", dir="/tmp") as folder:
        temp = Path(folder)
        for name in originals:
            copyfile(raw / name, temp / name)
        with_changed_report(raw, temp, CENTERED, bad_orientation)
        orientation = reject(lambda: verify(temp, measured), "Wrong localized Centered seed")
        copyfile(raw / CENTERED, temp / CENTERED)
        with_changed_report(raw, temp, INSET, extra_request)
        wrong_requests = reject(lambda: verify(temp, measured), "contour or keyboard receipt differs")
        copyfile(raw / INSET, temp / INSET)
        wide = CENTERED_IMAGES["wide"][0]
        with Image.open(raw / wide) as original:
            corrupt = Image.new("RGB", original.size, (255, 0, 255))
        corrupt.save(temp / wide)
        wrong_pixels = reject(lambda: verify(temp, measured), "Recorded Centered track pixels differ")

    if {name: checksum(raw / name) for name in originals} != originals:
        raise AssertionError("Rejection controls modified the original receipts")
    return {"out_of_order_step": failed_order, "wrong_orientation": orientation,
            "extra_keyboard_request": wrong_requests, "wrong_valid_png": wrong_pixels}
