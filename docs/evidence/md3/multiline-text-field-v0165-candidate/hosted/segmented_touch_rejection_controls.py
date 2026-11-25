"""Rejection controls derived from original Segmented Button touch artifacts."""

from copy import deepcopy
import json
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from verify_segmented_pixels import IMAGES, REPORT, TOUCH_LTR, TOUCH_RTL, verify


def rejected(operation, expected):
    try:
        operation()
    except (ValueError, RuntimeError) as error:
        if expected not in str(error):
            raise AssertionError(f"Unexpected rejection: {error}") from error
        return str(error)
    raise AssertionError(f"Altered touch evidence passed: {expected}")


def controls(raw, log, observed):
    report = json.loads((raw / REPORT).read_text())
    with TemporaryDirectory(prefix="surface-hosted-segmented-touch-controls.", dir="/tmp") as temporary:
        root = Path(temporary)
        for name in IMAGES:
            copyfile(raw / name, root / name)
        target = root / REPORT
        failures = {}

        def alter(name, edit, expected):
            changed = deepcopy(report)
            edit(changed)
            target.write_text(json.dumps(changed, ensure_ascii=False))
            failures[name] = rejected(lambda: verify(root, observed, log=log), expected)

        alter("foreign_owner", lambda value: value["touch_capture_loss"].update(owner=value["touch_capture_loss"]["owner"] + 1),
              "Capture ownership or trusted event order")
        alter("reversed_capture", lambda value: value["touch_capture_loss"]["events"].reverse(),
              "Capture ownership or trusted event order")
        alter("stale_release", lambda value: value["touch_capture_loss"]["after_up"].__setitem__(0, value["touch_capture_loss"]["after_up"][0] + 5),
              "Capture loss or stale release")
        alter("cross_axis_parent", lambda value: value["touch_overflow"]["parent"]["end"].__setitem__(0, value["touch_overflow"]["parent"]["start"][0] + 5),
              "Vertical touch changed the horizontal offset")
        alter("duplicate_tap", lambda value: value["touch_overflow"]["directions"][0]["tap"].update(activations=2),
              "exact activation count")
        copyfile(raw / REPORT, target)
        copyfile(raw / TOUCH_RTL, root / TOUCH_LTR)
        failures["duplicated_touch_png"] = rejected(lambda: verify(root, observed, log=log),
                                                   "Reused Segmented Button Canvas captures")
        return failures
