#!/usr/bin/env python3
"""Decode the seven original Slider Refinements Canvas PNGs without a browser."""

from hashlib import sha256
import json
from math import isclose
from pathlib import Path
import sys

from PIL import Image

from verify_forms_pixels import checksum, ink, rect, require, sample

CENTERED = "material-vertical-centered-slider.json"
INSET = "material-slider-inset.json"
CENTERED_IMAGES = {
    "wide": ("material-vertical-centered-light.png", (1920, 2408)),
    "keys": ("material-vertical-centered-keyboard.png", (1920, 2408)),
    "compact": ("material-vertical-centered-dark-rtl-200.png", (640, 1848)),
}
INSET_IMAGES = {
    "wide": ("material-slider-inset-light.png", (1920, 2168)),
    "muted": ("material-slider-inset-muted.png", (1920, 2168)),
    "quiet": ("material-slider-inset-quiet.png", (1920, 2168)),
    "compact": ("material-slider-inset-dark-rtl-200.png", (640, 1848)),
}
INSET_IDS = (
    "horizontal-medium", "horizontal-large", "horizontal-xl", "short",
    "vertical-medium", "vertical-large", "vertical-xl", "disabled",
)


def decoded(artifacts, sections, expected, prefix):
    require({path.name for path in artifacts.glob(f"{prefix}*.png")} == {name for name, _ in expected.values()},
            f"Missing, unexpected or failed {prefix} capture")
    captures, images = {}, {}
    for role, (name, size) in expected.items():
        require(sections[role]["capture"] == name, f"{role} capture name differs")
        path = artifacts / name
        with Image.open(path) as picture:
            require(picture.format == "PNG" and picture.size == size, f"{role} PNG format or size differs")
            picture.load()
            image = picture.convert("RGB")
        captures[name] = {"size": list(size), "sha256": checksum(path),
                          "decoded_rgb_sha256": sha256(image.tobytes()).hexdigest()}
        images[role] = image
    return captures, images


def near(actual, expected):
    return isclose(actual, expected, rel_tol=0, abs_tol=0.75)


def end_y(box):
    return box[1] + box[3]


def centered_geometry(shape, value):
    target, rail, handle = (rect(shape[key]) for key in ("target", "rail", "handle"))
    low, high = map(rect, shape["inactive"])
    seam = rail[1] + rail[3] / 2
    require(target[2] >= 48 and near(target[3], 240) and near(rail[1] - target[1], 12)
            and near(target[3] - rail[3], 24)
            and near(handle[1] + handle[3] / 2, rail[1] + rail[3] * (100 - value) / 200),
            "Centered handle and measured bottom-to-top rail differ")
    active = shape["active"]
    if value == 0:
        require(active[3] <= 0.75 and near(low[1] - end_y(high), handle[3] + 12),
                "Vertical Centered neutral seam differs")
    elif value < 0:
        require(rect(active)[3] > 0 and near(active[1], seam + 2) and near(end_y(high), seam - 2)
                and near(handle[1] - end_y(active), 6) and near(low[1] - end_y(handle), 6),
                "Negative side or neutral cut differs")
    else:
        require(rect(active)[3] > 0 and near(end_y(active), seam - 2) and near(low[1], seam + 2)
                and near(active[1] - end_y(handle), 6) and near(handle[1] - end_y(high), 6),
                "Positive side or neutral cut differs")


def centered_pixels(report, images):
    wide, sections = report["wide"], ("keys", "compact")
    seeds = [-50, 0, 50, -25, 50]
    labels = ["−50％", "0％", "+50％", "−25％", "+50％"]
    require(wide["viewport"] == [960, 1160] and len(wide["samples"]) == len(seeds), "Wrong wide Centered fixture")
    wide_ink = []
    palette, image = wide["palette"], images["wide"]
    for index, (entry, value, label) in enumerate(zip(wide["samples"], seeds, labels, strict=True)):
        shape = entry["geometry"]
        centered_geometry(shape, value)
        require(entry["value"] == value and entry["aria"] == [str(value), label, "vertical"],
                "Wrong localized Centered seed receipt")
        measured = [sample(image, box) for box in (shape["handle"], *shape["inactive"])]
        require(measured == entry["colors"], "Recorded Centered track pixels differ")
        if index == 4:
            require(measured[0] not in [palette["primary"][:3], palette["background"][:3]],
                    "Disabled vertical Centered handle lacks muted paint")
            continue
        require(measured[0] == palette["primary"][:3] and all(near(track[2], 24) for track in shape["inactive"]),
                "Vertical Centered Small track or handle token differs")
        for track, stop in zip(shape["inactive"], shape["stops"], strict=True):
            require(near(stop[2], 4) and near(stop[3], 4) and sample(image, stop) == palette["on_secondary_container"][:3]
                    and ink(image, track, palette["secondary_container"]) > 80, "Vertical Centered end stop or inactive ink differs")
        active = ink(image, shape["active"], palette["primary"]) if value else 0
        require(value == 0 or active > 50, "Signed vertical active paint is missing")
        wide_ink.append(active)

    focused = {}
    for key in sections:
        entry, image = report[key], images[key]
        palette, paint, geometry = (entry[name] for name in ("palette", "paint", "geometry"))
        centered_geometry(geometry, 25)
        require(near(geometry["handle"][3], 2) and entry["aria"] == ["25", "+25％", "vertical"],
                "Wrong engaged vertical value, orientation or handle")
        label, ring = paint["label"], paint["ring"]
        counts = {"text_ink": ink(image, label, palette["inverse_on_surface"]),
                  "label_fill": ink(image, label, palette["inverse_surface"]),
                  "focus_ink": ink(image, ring, palette["secondary"])}
        require(all(value == paint[name] for name, value in counts.items()) and counts["text_ink"] > 20
                and counts["label_fill"] > 100 and counts["focus_ink"] > 20,
                f"Vertical Centered indicator pixels differ in {key}")
        values = [sample["value"] for sample in entry["values"]]
        requests = [sample["requests"] for sample in entry["values"]]
        if key == "keys":
            require(entry["viewport"] == [960, 1160] and values == [100, 0, 50, 25, 50]
                    and requests == [2, 0, 0, 2, 0] and near(label[0] - ring[0] - ring[2], 4),
                    "Vertical Centered keyboard or LTR label receipt differs")
        else:
            require(entry["viewport"] == [320, 880] and entry["dark"] and entry["rtl"] and entry["text_scale"] == 2
                    and values == [-50, 0, 50, 25, 50] and requests == [0, 0, 0, 4, 0] and label[3] >= 63
                    and near(ring[0] - label[0] - label[2], 4) and entry["back_focus"] == "vertical-centered-slider-demo-show",
                    "Vertical Centered dark RTL or touch receipt differs")
        focused[key] = counts
    return {"signed_active_pixels": wide_ink, "indicators": focused}


def inset_pixels(report, images):
    wide, keys, compact = (report[key] for key in ("wide", "keys", "compact"))
    palette = wide["palette"]
    sizes = [24, 24, 32, 0, 24, 24, 32, 24]
    require(len(wide["samples"]) == len(sizes), "Inset slider fixture count differs")
    glyphs = {}
    for index, (entry, suffix, size) in enumerate(zip(wide["samples"], INSET_IDS, sizes, strict=True)):
        require(entry["id"] == f"slider-inset-demo-{suffix}" and rect(entry["stop"])[2:] == [4, 4],
                "Inset identity or end stop changed")
        if size == 0:
            require(entry["glyph"] is None, "Short inset rail unexpectedly has a glyph")
            continue
        require(rect(entry["glyph"])[2:] == [size, size], "Inset glyph dimensions changed")
        if index == 7:
            require(entry["ink"] is None, "Disabled inset reading must remain descriptive")
            continue
        color = "on_secondary_container" if index in (1, 5) else "on_primary"
        painted = ink(images["wide"], entry["glyph"], palette[color])
        require(painted == entry["ink"] and painted > 30, f"Inset glyph paint differs for {suffix}")
        glyphs[suffix] = painted

    muted, quiet = keys["muted"], keys["quiet"]
    muted_ink = ink(images["muted"], muted["glyph"], palette["on_secondary_container"])
    quiet_ink = ink(images["quiet"], quiet["glyph"], palette["on_secondary_container"])
    require(muted["glyph"] == quiet["glyph"] and keys["full"][0] < quiet["glyph"][0]
            and muted_ink == muted["ink"] and quiet_ink == quiet["ink"] and min(muted_ink, quiet_ink) > 30
            and abs(muted_ink - quiet_ink) >= 8 and keys["values"]["value"] == 100 and keys["values"]["requests"] == 3,
            "Inset minimum-versus-quiet painted contour or keyboard receipt differs")

    palette, image = compact["palette"], images["compact"]
    counts = {"glyph_ink": ink(image, compact["vertical_icon"], palette["on_primary"]),
              "label_ink": ink(image, compact["label"], palette["inverse_on_surface"]),
              "ring_ink": ink(image, compact["ring"], palette["secondary"])}
    require(rect(compact["vertical_icon"])[2:] == [24, 24] and rect(compact["label"])[3] >= 63
            and compact["label"][0] + compact["label"][2] < compact["ring"][0]
            and all(hits == compact[name] and hits > 30 for name, hits in counts.items())
            and compact["back_focus"] == "slider-inset-demo-show", "Compact inset indicator, glyph or focus receipt differs")
    return {"enabled_wide_glyph_pixels": glyphs, "contours": {"muted": muted_ink, "quiet": quiet_ink},
            "compact": counts}


def verify(artifacts, metadata=None):
    centered_path, inset_path = artifacts / CENTERED, artifacts / INSET
    centered, inset = (json.loads(path.read_text()) for path in (centered_path, inset_path))
    require(all(report.get("page_errors") == [] and "failure" not in report for report in (centered, inset)),
            "A Slider Refinements report has a page error or failure")
    require(centered["browser"] == inset["browser"], "Slider Refinements browser metadata differs")
    if metadata is not None:
        require(centered["browser"] == metadata["browser"] and centered["adapter"] == metadata["adapter"],
                "Slider Refinements browser or adapter differs from stock")
    vc_captures, vc_images = decoded(artifacts, centered, CENTERED_IMAGES, "material-vertical-centered-")
    inset_sections = {"wide": inset["wide"], "muted": inset["keys"]["muted"],
                      "quiet": inset["keys"]["quiet"], "compact": inset["compact"]}
    inset_captures, inset_images = decoded(artifacts, inset_sections, INSET_IMAGES, "material-slider-inset-")
    captures = {**vc_captures, **inset_captures}
    require(len({row["sha256"] for row in captures.values()}) == 7, "A Slider Refinements PNG was reused")
    return {"report_sha256": {CENTERED: checksum(centered_path), INSET: checksum(inset_path)},
            "captures": captures, "vertical_centered": centered_pixels(centered, vc_images),
            "inset": inset_pixels(inset, inset_images)}


if __name__ == "__main__":
    print(json.dumps(verify(Path(sys.argv[1])), ensure_ascii=False, indent=2))
