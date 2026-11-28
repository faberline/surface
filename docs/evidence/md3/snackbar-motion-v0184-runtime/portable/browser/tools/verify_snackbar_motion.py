"""Read original Snackbar motion PNGs and the geometry from Chrome."""

import json
import math
from pathlib import Path

from PIL import Image
from evidence_common import digest, require

REPORT = "material-snackbar-motion.json"
STEPS = ["M391", "M392"]
PNG_NAMES = [f"material-snackbar-motion-{name}.png"
             for name in ("light-rest", "light-exit", "dark-rtl-200")]


def rect(value):
    return len(value) == 4 and all(
        type(n) in (int, float) and math.isfinite(n) for n in value
    ) and min(value[2:]) > 0


def sample(file, canvas, panel):
    with Image.open(file) as image:
        require(image.format == "PNG", f"not an original PNG: {file.name}")
        scale = image.width / canvas[2]
        x = math.floor((canvas[0] + panel[0] + 8) * scale)
        y = math.floor((canvas[1] + panel[1] + 8) * scale)
        return list(image.convert("RGB").getpixel((x, y)))


def verify_motion(artifacts, browser):
    artifacts = Path(artifacts)
    report_path = artifacts / REPORT
    report = json.loads(report_path.read_text())
    require(report["browser"] == browser and report["steps"] == STEPS
            and report["errors"] == [], "Snackbar report browser, steps or errors changed")
    rest, leave, dark = (report[key] for key in ("light_rest", "light_exit", "dark_rtl_200"))
    canvases = report["canvases"]
    require(len(canvases) == 3 and all(rect(bounds) for bounds in [*canvases, rest, leave, dark]),
            "invalid Snackbar or canvas bounds")
    require(canvases[0] == canvases[1] and canvases[0][2] == 960 and canvases[2][2] == 320,
            "Snackbar profiles did not exercise wide and 320 dp canvases")
    require(rest[0] == leave[0] and rest[2:] == leave[2:] and rest[1] < leave[1],
            "leaving changed the Snackbar size or moved it away from the lower edge")
    require(0 < report["exit_progress"] < 1, "leaving capture is not an intermediate pose")
    require(0 <= dark[0] and 0 <= dark[1]
            and dark[0] + dark[2] <= canvases[2][2]
            and dark[1] + dark[3] <= canvases[2][3], "narrow Snackbar escapes the canvas")
    state = report["state"]
    require(state["rtl"] is True and state["text_scale"] == 2 and state["dismissed"] == 2
            and state["visible"] is None and state["presented_id"] is None
            and state["motion_progress"] == 0, "final reduced-motion exit did not settle")
    png = [artifacts / name for name in PNG_NAMES]
    light_fill = sample(png[0], canvases[0], rest)
    dark_fill = sample(png[2], canvases[2], dark)
    require(light_fill == report["light_fill"] and dark_fill == report["dark_fill"]
            and light_fill != dark_fill, "original PNG theme colors differ")
    require(sample(png[1], canvases[1], leave) == light_fill
            and sample(png[1], canvases[1], rest) != light_fill,
            "original PNG does not show the departing Snackbar at its sampled pose")
    return {"report_sha256": digest(report_path),
            "screenshots": {file.name: digest(file) for file in png},
            "light_fill": light_fill, "dark_fill": dark_fill, "exit_progress": report["exit_progress"]}
