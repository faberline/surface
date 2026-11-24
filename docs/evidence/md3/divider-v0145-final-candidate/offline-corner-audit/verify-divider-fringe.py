#!/usr/bin/env python3
"""Replay Divider endpoint and thickness checks on the original Linux screenshots."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import zipfile

from PIL import Image


ROOT = Path(__file__).resolve().parent
DRAWINGS = ("full", "inset", "middle", "vertical")
REPORT = "browser-material-divider.json"
REGULAR = "material-divider-compact-dark-rtl-200.png"
HEAVY = "material-divider-compact-dark-rtl-200-heavy.png"
FAILURE = "material-divider-failure.png"
BACKGROUND = (20, 18, 24)
OUTLINE = (73, 69, 78)
CORNER = (65, 61, 70)


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def decoded_rgb(path):
    with Image.open(path) as image:
        image.verify()
    with Image.open(path) as image:
        image.load()
        assert image.mode == "RGB", (path, image.mode)
        return image.copy()


def check_archive(provenance, archive_path):
    archived = provenance["original_zip"]
    for name in (REPORT, REGULAR, HEAVY):
        assert sha256(ROOT / "raw" / name) == archived["members"][name]["sha256"], name
    assert archived["members"][FAILURE]["sha256"] == archived["members"][HEAVY]["sha256"]
    if archive_path is None:
        return
    assert archive_path.stat().st_size == archived["bytes"]
    assert sha256(archive_path) == archived["sha256"]
    with zipfile.ZipFile(archive_path) as original:
        for name, record in archived["members"].items():
            info = original.getinfo(record["zip_path"])
            assert info.file_size == record["bytes"] and f"{info.CRC:08x}" == record["crc32"], name
            assert hashlib.sha256(original.read(info)).hexdigest() == record["sha256"], name


def fixture_points(rect, vertical, end_inset):
    x, y, width, height = rect
    for offset in range(8):
        depth = offset + 0.5
        if vertical:
            yield from ((x + depth, y + end_inset),
                        (x + depth, y + height / 2),
                        (x + depth, y + height - end_inset))
        else:
            yield from ((x + end_inset, y + depth),
                        (x + width / 2, y + depth),
                        (x + width - end_inset, y + depth))


def outside_fixture_points(rect, vertical):
    x, y, width, height = rect
    if vertical:
        return ((x - 0.5, y + height / 2), (x + width + 0.5, y + height / 2))
    return ((x + width / 2, y - 0.5), (x + width / 2, y + height + 0.5))


def mismatches(image, points, expected):
    return [{"device_xy": [x, y], "rgb": list(pixel)}
            for x, y in points if (pixel := image.getpixel((x, y))) != expected]


def physical_stripes(bounds, vertical):
    left, top, right, bottom = bounds
    start, end = (top, bottom) if vertical else (left, right)
    for length_pixel in (start + 4, (start + end) // 2, end - 5):
        if vertical:
            yield ([(x, length_pixel) for x in range(left, right)],
                   [(left - 1, length_pixel), (right, length_pixel)])
        else:
            yield ([(length_pixel, y) for y in range(top, bottom)],
                   [(length_pixel, top - 1), (length_pixel, bottom)])


def border(bounds):
    left, top, right, bottom = bounds
    yield from ((x, y) for x in range(left - 1, right + 1) for y in (top - 1, bottom))
    yield from ((x, y) for y in range(top, bottom) for x in (left - 1, right))


def interior(bounds):
    left, top, right, bottom = bounds
    return ((x, y) for y in range(top, bottom) for x in range(left, right))


def verify(provenance, archive_path):
    check_archive(provenance, archive_path)
    report = json.loads((ROOT / "raw" / REPORT).read_text())
    capture_regular = decoded_rgb(ROOT / "raw" / REGULAR)
    capture_heavy = decoded_rgb(ROOT / "raw" / HEAVY)
    regular = report["compact"]
    assert regular["viewport"] == [320, 800] and regular["thickness_dp"] == 1
    assert capture_heavy.size == capture_regular.size == (640, 1688)
    assert report["errors"] == [] and report["state_at_failure"]["dark"] is True
    assert report["adapter"] == {"isFallbackAdapter": True, "vendor": "google", "architecture": "swiftshader"}
    assert report["failure"] == "M266: PNG uses outline variant: expected [73, 69, 78], got [65, 61, 70]"

    # Rust Raster::sample floors physical coordinates after adding the Canvas origin.
    # The original viewport is 320×800; the 640×1688 capture includes a 44-dp header.
    dpr = capture_heavy.width / regular["viewport"][0]
    header = capture_heavy.height / dpr - regular["viewport"][1]
    assert (dpr, header) == (2, 44)

    def physical(x, y):
        return math.floor(x * dpr), math.floor((y + header) * dpr)

    def device_rect(rect):
        x, y, width, height = rect
        return (*physical(x, y), *physical(x + width, y + height))

    results = {}
    for i, name in enumerate(DRAWINGS):
        x, y, width, height = regular[name]["bounds"]
        assert tuple(regular[name]["rgb"]) == OUTLINE
        assert not mismatches(capture_regular, interior(device_rect((x, y, width, height))), OUTLINE), name
        vertical = name == "vertical"
        assert (width if vertical else height) == 1
        # Previous horizontal rows add seven dp each. Keep the same layout widths
        # and the vertical ruler's 64-dp length; confirm the result by full scan.
        rect = (x, y + 7 * i, 8 if vertical else width, height if vertical else 8)
        assert (rect[0] >= 0 and rect[1] >= 0 and rect[0] + rect[2] <= regular["viewport"][0]
                and rect[1] + rect[3] <= regular["viewport"][1]), rect
        bounds = device_rect(rect)
        _, _, right, bottom = bounds
        corner_pixels = [(right - 1, bottom - 2), (right - 2, bottom - 1), (right - 1, bottom - 1)]
        expected_corner = [{"device_xy": list(point), "rgb": list(CORNER)} for point in corner_pixels]
        assert mismatches(capture_heavy, interior(bounds), OUTLINE) == expected_corner, name
        assert not mismatches(capture_heavy, border(bounds), BACKGROUND), name

        original_points = [physical(*point) for point in fixture_points(rect, vertical, 1.0)]
        revised_points = [physical(*point) for point in fixture_points(rect, vertical, 1.5)]
        original_mismatch = mismatches(capture_heavy, original_points, OUTLINE)
        assert len(original_points) == len(revised_points) == 24, name
        assert original_mismatch == [expected_corner[0 if vertical else 1]], (name, original_mismatch)
        assert not mismatches(capture_heavy, revised_points, OUTLINE), name
        outside = [physical(*point) for point in outside_fixture_points(rect, vertical)]
        assert not mismatches(capture_heavy, outside, BACKGROUND), name

        stripes = list(physical_stripes(bounds, vertical))
        for line, controls in stripes:
            assert len(line) == 16 and len(controls) == 2, name
            assert not mismatches(capture_heavy, line, OUTLINE), name
            assert not mismatches(capture_heavy, controls, BACKGROUND), name
        results[name] = {
            "logical_bounds_dp": rect,
            "physical_bounds_xyxy_exclusive": bounds,
            "interior_device_pixels": (bounds[2] - bounds[0]) * (bounds[3] - bounds[1]),
            "exact_interior_token_pixels": (bounds[2] - bounds[0]) * (bounds[3] - bounds[1]) - 3,
            "three_corner_coverage_pixels": expected_corner,
            "original_1dp_sample_mismatch": original_mismatch[0],
            "corrected_1_5dp_token_samples": len(revised_points),
            "unchanged_exterior_control_samples": len(outside),
            "independent_full_depth_stripe_samples": sum(len(line) for line, _ in stripes),
            "independent_background_controls": sum(len(controls) for _, controls in stripes),
            "exact_adjacent_background_perimeter_pixels": len(list(border(bounds))),
        }

    # Model quad.wgsl's corner distance derivatives under the observed pattern.
    # dFdx and dFdy have 0.5 dp magnitudes in the corner's DPR-2 fragment quad.
    distance, edge = -0.25, 1.0
    t = (distance + edge / 2) / edge
    coverage = 1 - t * t * (3 - 2 * t)
    modeled = tuple(round(background + (ink - background) * coverage)
                    for background, ink in zip(BACKGROUND, OUTLINE))
    assert coverage == 0.84375 and modeled == CORNER

    # A one-channel, one-count defect on a known interior point must still fail.
    full = results["full"]["logical_bounds_dp"]
    full_bounds = results["full"]["physical_bounds_xyxy_exclusive"]
    center = physical(full[0] + full[2] / 2, full[1] + 3.5)
    assert capture_heavy.getpixel(center) == OUTLINE
    changed = capture_heavy.copy()
    changed.putpixel(center, (OUTLINE[0] - 1, *OUTLINE[1:]))
    revised_points = [physical(*point) for point in fixture_points(full, False, 1.5)]
    all_stripe_points = [point for line, _ in physical_stripes(full_bounds, False) for point in line]
    corrected_negative = mismatches(changed, revised_points, OUTLINE)
    independent_negative = mismatches(changed, all_stripe_points, OUTLINE)
    assert corrected_negative == independent_negative == [{"device_xy": list(center), "rgb": [72, 69, 78]}]

    return {
        "scope": "offline verification of immutable original Linux captures; no new browser or CI replay",
        "original_workflow_run": provenance["original_hosted_push"]["id"],
        "original_browser_job": provenance["original_browser_job"]["id"],
        "original_member_sha256": provenance["original_zip"]["members"][HEAVY]["sha256"],
        "browser": report["browser"],
        "adapter": report["adapter"],
        "page_errors": report["errors"],
        "device_pixel_ratio": dpr,
        "canvas_origin_dp": [0, header],
        "image_size_pixels": capture_heavy.size,
        "outline_variant_rgb": OUTLINE,
        "adjacent_surface_rgb": BACKGROUND,
        "rectangles": results,
        "shader_consistency_model": {
            "status": "inference from WGSL and pixel pattern; no derivatives were instrumented",
            "distance_dp": distance,
            "fwidth_dp": edge,
            "coverage": coverage,
            "blended_rgb": modeled,
        },
        "in_memory_negative_control": {
            "change": "decreased the red channel by one in one interior pixel",
            "corrected_fixture_rejected": corrected_negative,
            "independent_stripe_rejected": independent_negative,
        },
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--original-zip", type=Path, help="also check the archive and exact archived member bytes")
    args = parser.parse_args()
    if not __debug__:
        raise SystemExit("Run without -O to keep verification assertions enabled.")
    provenance = json.loads((ROOT / "raw" / "provenance.json").read_text())
    print(json.dumps(verify(provenance, args.original_zip), indent=2))
