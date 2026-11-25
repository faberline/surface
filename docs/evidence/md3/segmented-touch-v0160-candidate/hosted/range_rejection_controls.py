"""Check that wrong Range Slider pixels, pointer state and step order are rejected."""

import json
from pathlib import Path
import re
from tempfile import TemporaryDirectory

from PIL import Image

from verify_range_pixels import REPORT, checksum, verify

STEPS = ("M306", "M307", "M308", "M309")


def replay_steps(log):
    actual = re.findall(r"^\s*\[(M\d+)\]", log.read_text(), re.M)
    if tuple(step for step in actual if step in STEPS) != STEPS:
        raise AssertionError("Range steps are missing, repeated or out of order")


def controls(raw, log, measured):
    expected = measured["range_slider"]
    before = checksum(raw / REPORT)
    replay_steps(log)
    with TemporaryDirectory(prefix="surface-range-hosted-controls.", dir="/tmp") as folder:
        temp = Path(folder)
        for name in expected["captures"]:
            target = (raw / name).resolve(strict=True)
            (temp / name).symlink_to(target)

        def reject(label, test):
            try:
                test()
            except (AssertionError, ValueError) as issue:
                if label not in str(issue):
                    raise
                return True
            raise AssertionError(f"Range rejection control passed: {label}")

        altered = temp / "runner.log"
        original = log.read_text()
        altered.write_text(original.replace("[M309]", "[M308]"))
        missing_step = reject("missing, repeated or out of order", lambda: replay_steps(altered))

        evidence = json.loads((raw / REPORT).read_text())
        invalid = temp / REPORT
        evidence["capture_loss"]["after"]["values"][0] += 1
        invalid.write_text(json.dumps(evidence))
        late_release = reject("foreign or late release applied an extra value", lambda: verify(temp, measured))
        invalid.write_bytes((raw / REPORT).read_bytes())

        name = "material-range-slider-wide-light.png"
        path = temp / name
        with Image.open(path) as original_image:
            image = original_image.convert("RGB")
        # Replace the full PNG with a valid but wrong image of the same size.
        # Requiring the first painted span to match provides the rejection.
        image.paste((255, 0, 255), (0, 0, image.width, image.height))
        path.unlink()
        image.save(path)
        bad_pixels = reject("wide decoded paint differs", lambda: verify(temp, measured))

    if checksum(raw / REPORT) != before:
        raise AssertionError("Range rejection controls modified originals")
    return {"out_of_order_step": missing_step, "late_release_value": late_release, "wrong_valid_png": bad_pixels}
