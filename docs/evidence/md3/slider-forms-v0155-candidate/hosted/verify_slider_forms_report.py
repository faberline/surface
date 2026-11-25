"""Check the four Slider Forms captures and their typed stock observations."""

import re
from math import isfinite

from verify_forms_pixels import CAPTURES, REPORT, rect, require, verify as verify_pixels

STEM = "material-slider-forms"
IMAGES = {name: size for name, size in CAPTURES.values()}
STEPS = ("M302", "M303", "M304", "M305")


def exact(actual, expected):
    return type(actual) in (int, float) and isfinite(actual) and abs(actual - expected) <= 0.75


def enclosed(outer, child):
    x, y, w, h = child
    px, py, pw, ph = outer
    return px - 0.75 <= x and py - 0.75 <= y and x + w <= px + pw + 0.75 and y + h <= py + ph + 0.75


def shape_geometry(shape, viewport, vertical, center, grip_breadth, track_expected):
    require(set(shape) == {"target", "rail", "active", "handle"}, "wrong Forms geometry fields")
    target, rail, handle = (rect(shape[field]) for field in ("target", "rail", "handle"))
    axis = 1 if vertical else 0
    cross = 1 - axis
    require(enclosed([0, 0, *viewport], target)
            and enclosed(target, rail) and enclosed(target, handle), "clipped Forms target or rail")
    require(exact(rail[axis] - target[axis], 12) and exact(target[axis + 2] - rail[axis + 2], 24)
            and exact(target[cross + 2], 48) and exact(handle[axis + 2], grip_breadth)
            and exact(handle[cross + 2], 44), "Forms hit or grip size differs")
    require(exact(handle[axis] + handle[axis + 2] / 2, rail[axis] + rail[axis + 2] * center),
            "Forms current value position differs")
    active = shape["active"]
    require(isinstance(active, list) and len(active) == 4
            and all(type(value) in (int, float) and isfinite(value) for value in active)
            and exact(active[cross + 2], 24) and active[axis + 2] >= 0,
            "Forms active track geometry differs")
    require(enclosed(rail, active) and all(exact(actual, expected) for actual, expected in
            zip((active[axis], active[axis] + active[axis + 2]), track_expected, strict=True)),
            "Forms active span does not join the exact neutral or value edge")
    return target, rail, handle


def centers(shape, value, rtl=False):
    rail, grip = shape["rail"], shape["handle"]
    midpoint = rail[0] + rail[2] / 2
    if value == 0:
        return (grip[0] + grip[2],) * 2
    sign = -value if rtl else value
    if sign < 0:
        return grip[0] + grip[2] + 6, midpoint - 2
    return midpoint + 2, grip[0] - 6


def label_geometry(label, ring, handle, viewport, *, vertical, rtl=False, scale=1):
    rect(label)
    rect(ring)
    require(enclosed([0, 0, *viewport], label) and enclosed([0, 0, *viewport], ring),
            "clipped Forms focus or value label")
    require(all(exact(actual, expected) for actual, expected in zip(
        ring, (handle[0] - 5, handle[1] - 5, handle[2] + 10, handle[3] + 10), strict=True)),
        "Forms focus ring must surround the handle")
    require(label[3] >= 44 + 20 * (scale - 1), "Forms value label lacks its measured height")
    if vertical:
        clearance = ring[0] - (label[0] + label[2]) if rtl else label[0] - (ring[0] + ring[2])
    else:
        clearance = ring[1] - (label[1] + label[3])
    require(exact(clearance, 4), "Forms value label is on the wrong physical side")


def validate(artifacts, log, metadata):
    import json

    report = json.loads((artifacts / REPORT).read_text())
    require(set(report) == {"browser", "adapter", "centered", "centered_keys", "vertical_mouse", "compact", "page_errors"},
            "unexpected Slider Forms report schema")
    require(tuple(re.findall(r"^\s*\[(M30[2-5])\]", log.read_text(), re.M)) == STEPS,
            "missing, repeated or out-of-order Slider Forms step")
    require(type(report["adapter"]["isFallbackAdapter"]) is bool, "invalid Forms WebGPU metadata")
    pixels = verify_pixels(artifacts, metadata)
    require(len({record["decoded_rgb_sha256"] for record in pixels["captures"].values()}) == len(IMAGES),
            "duplicate Forms screenshot pixels")

    centered = report["centered"]
    wide = centered["viewport"]
    suffixes = ("negative", "neutral", "positive", "discrete")
    expected_descriptions = ("−50％", "0％", "+50％", "−25％")
    for specimen, suffix, text in zip(centered["sliders"], suffixes, expected_descriptions, strict=True):
        require(specimen["id"] == f"slider-forms-demo-centered-{suffix}" and specimen["aria"][3] == text,
                "wrong Centered specimen or localized value")
        value, shape = specimen["value"], specimen["shape"]
        _, rail, grip = shape_geometry(shape, wide, False, (value + 100) / 200, 4, centers(shape, value))
        for track, stop in zip(specimen["inactive"], specimen["stops"], strict=True):
            require(enclosed(rail, rect(track)) and enclosed(track, rect(stop))
                    and exact(stop[2], 4) and exact(stop[3], 4), "Centered exterior stop leaves its track")
    keys = report["centered_keys"]
    _, _, grip = shape_geometry(keys["geometry"], wide, False, 0.25, 2, centers(keys["geometry"], 50, True))
    label_geometry(keys["label"], keys["ring"], grip, wide, vertical=False)

    mouse, compact = (report[section] for section in ("vertical_mouse", "compact"))
    require(mouse["initial_ax"] == ["0", "100", "25", "25％", "vertical"]
            and mouse["disabled_ax"] == ["0", "100", "50", "50％", "vertical"]
            and compact["aria"] == mouse["initial_ax"], "wrong localized vertical accessible values")
    require(compact["vertical"]["engaged"] == [False, True, False]
            and compact["rtl"] is True and compact["dark"] is True, "wrong compact capture owner")
    for shape, viewport, value, label, ring, rtl in (
        (mouse["initial"], wide, 25, None, None, False),
        (mouse["held"], wide, mouse["value"], mouse["label"], mouse["ring"], False),
        (compact["geometry"], compact["viewport"], 25, compact["label"], compact["ring"], True),
    ):
        rail, grip = shape["rail"], shape["handle"]
        _, _, grip = shape_geometry(shape, viewport, True, 1 - value / 100, 2,
                                    (grip[1] + grip[3] + 6, rail[1] + rail[3]))
        if label is not None:
            label_geometry(label, ring, grip, viewport, vertical=True, rtl=rtl, scale=2 if rtl else 1)
    return {"steps": list(STEPS), "page_errors": 0, **pixels}
