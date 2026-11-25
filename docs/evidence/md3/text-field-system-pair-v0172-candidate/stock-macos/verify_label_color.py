"""Keep the stock label proof shape and verify the new inherited M347 outline."""

import math
from pathlib import Path

from evidence_common import data, digest, require
from verify_boundary_stock import tools

PREFIX = "material-text-field-label-motion"


def verify(artifacts, metadata):
    checks = tools()
    label = checks["LABEL"]
    raw = label["verify_artifacts"](artifacts)
    require(raw["browser"] == metadata["browser"] and raw["adapter"] == metadata["adapter"],
            "label color Chrome or adapter differs from the stock run")
    report = data(artifacts / f"{PREFIX}.json")
    pngs = {name: {"sha256": checksum, "size": [640, 2008] if name.startswith(f"{PREFIX}-rtl") else [1920, 1728]}
            for name, checksum in sorted(raw["captures"].items())}
    outlined = report["outlined"]
    filled = report["filled"]
    early = outlined["frames"][2]
    image = label["image_for"](artifacts / early["png"], False)
    shallow = outlined["early_shallow"]
    coverage = {"cut": [0, 0, 1], "uncut": [4, 4, 3]}
    modes = {False, True}
    paper = tuple(outlined["palette"]["background"][:3])
    rgb = tuple(early["outline_rgb"])
    for segment, quarters in coverage.items():
        points = shallow[f"{segment}_points"]
        pixels = [image.getpixel((math.floor(x * 2), math.floor((y + 44) * 2))) for x, y in points]
        require(list(map(list, pixels)) == shallow[f"{segment}_rgb"], "early shallow M347 probe receipt differs")
        for pixel, count in zip(pixels, quarters, strict=True):
            modes = {linear for linear in modes if all(
                abs(a - b) <= (0 if count in (0, 4) else 2)
                for a, b in zip(pixel, checks["PLACEHOLDER"]["colors"](paper, rgb, count / 4, linear), strict=True)
            )}
    require(modes and early["gap_pixel_columns"] >= 8 and early["ms"] == 14,
            "M347 must leave the measured quarter-coverage shallow cut and full-width control")
    return {
        "report_sha256": raw["report_sha256"], "clock": report["visible_clock"], "pngs": pngs,
        "outlined": {
            "geometry": {key: outlined[key] for key in ("face", "inline", "floated", "helper")},
            "frames": outlined["frames"], "inactive_ax": outlined["proxy"], "active_ax": outlined["active"],
            "early_shallow": {**shallow, "coverage_quarters": coverage,
                              "png_sha256": digest(artifacts / early["png"])},
        },
        "reversal": report["reversal"],
        "filled": {
            "frames": filled["frames"], "rest": filled["rest"], "active_ax": filled["ax"],
            "synthetic_cdp_preedit": filled["preedit"],
        },
        "compact_dark_rtl_200": report["compact"], "label_color": report["label_color"],
    }
