"""Decode the stock Divider screenshots and verify their physical line pixels."""

from hashlib import sha256
import json
from pathlib import Path
import re
import sys

from PIL import Image

STEPS = ("M264", "M265", "M266")
HEIGHT = 800
SCALE = 2
HEADER = 44
PROFILES = {
    "wide": ("wide-light-ltr", 960, (202, 196, 207), 16, 1),
    "compact": ("compact-dark-rtl-200", 320, (73, 69, 78), 0, 1),
    "wide_heavy": ("wide-light-ltr-heavy", 960, (202, 196, 207), 16, 8),
    "compact_heavy": ("compact-dark-rtl-200-heavy", 320, (73, 69, 78), 0, 8),
}


def read_profile(run, raw, expected):
    name, width, ink, leading, thickness = expected
    assert raw["profile"] == name and raw["viewport"] == [width, HEIGHT]
    assert raw["accessible_groups"] is True and raw["decorative_focus"] is False
    assert raw["thickness_dp"] == thickness
    png = run / f"material-divider-{name}.png"
    with Image.open(png) as original:
        assert original.format == "PNG"
        assert original.size == (width * SCALE, (HEIGHT + HEADER) * SCALE)
        frame = original.convert("RGB")

    lines = ("full", "inset", "middle", "vertical")
    full, inset, middle, vertical = (raw[line]["bounds"] for line in lines)
    assert [full[3], inset[3], middle[3], vertical[2], vertical[3]] == [thickness] * 4 + [64]
    assert [inset[0] - full[0], full[2] - inset[2], middle[0] - full[0], full[2] - middle[2]] == [leading, 16, 16, 32]

    pixels = {}
    for name in lines:
        line = raw[name]
        assert tuple(line["rgb"]) == ink
        x, y, width_dp, height_dp = line["bounds"]
        assert 0 <= x < x + width_dp <= width and 0 <= y < y + height_dp <= HEIGHT
        coordinates = [SCALE * x, SCALE * (y + HEADER), SCALE * (x + width_dp), SCALE * (y + height_dp + HEADER)]
        assert all(float(number).is_integer() for number in coordinates), coordinates
        x0, y0, x1, y1 = map(int, coordinates)
        if name == "vertical":
            body = (x0, y0 + SCALE, x1, y1 - SCALE)
            adjacent = [(x0 - 1, body[1], x0, body[3]), (x1, body[1], x1 + 1, body[3])]
        else:
            body = (x0 + SCALE, y0, x1 - SCALE, y1)
            adjacent = [(body[0], y0 - 1, body[2], y0), (body[0], y1, body[2], y1 + 1)]
        stripe = frame.crop(body).get_flattened_data()
        assert stripe and all(pixel == ink for pixel in stripe), (raw["profile"], name)
        for region in adjacent:
            assert ink not in frame.crop(region).get_flattened_data(), (raw["profile"], name, region)
        pixels[name] = {
            "bounds_dp": line["bounds"],
            "solid_pixels_checked": len(stripe),
            "physical_thickness_px": SCALE * thickness,
        }
    return {"file": png.name, "sha256": sha256(png.read_bytes()).hexdigest(), "lines": pixels}


def validate(attempt, browser, adapter, *, log=None):
    run = Path(attempt)
    log = (run / "runner.log").read_text() if log is None else log
    steps = tuple(step for step in re.findall(r"^  \[(M\d+)\]", log, re.M) if step in STEPS)
    assert steps == STEPS, steps
    raw = json.loads((run / "browser-material-divider.json").read_text())
    assert raw["errors"] == [] and "failure" not in raw and raw["focus_restored"] is True
    expected_adapter = {key: adapter[key] for key in ("vendor", "architecture", "isFallbackAdapter")}
    assert isinstance(expected_adapter["isFallbackAdapter"], bool)
    assert raw["adapter"] == expected_adapter and raw["browser"] == browser
    return {
        "browser": browser, "adapter": expected_adapter, "steps": list(steps),
        "page_errors": 0, "focus_restored": True,
        "profiles": {key: read_profile(run, raw[key], expected) for key, expected in PROFILES.items()},
    }


if __name__ == "__main__":
    path, host = map(Path, sys.argv[1:3])
    observed = json.loads(host.read_text())
    print(json.dumps(validate(path, observed["browser"], observed["adapter"]), indent=2))
