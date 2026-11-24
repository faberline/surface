"""Validate both native Checkbox replays and independently decode the PNGs."""

import hashlib
import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SPECS = (
    ("wide-light.png", (960, 800), False, 1),
    ("compact-dark-rtl-200.png", (320, 720), True, 2),
)
BEFORE = ["mixed", "checked", "unchecked", "mixed", "unchecked"]
AFTER = ["unchecked", "unchecked", "unchecked", "mixed", "checked"]


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_box(row, rtl):
    target = row["target"]
    box = row["box"]
    label = row["label"]
    ring = row["ring"]
    require((box["width"], box["height"]) == (18, 18), "Wrong 18 dp visual")
    require((ring["width"], ring["height"]) == (50, 50), "Wrong focus ring extent")
    require(min(target["width"], target["height"]) >= 48, "The target is too small")
    for part in (box, label, ring):
        require(all(target[k] <= part[k] and part[k] + part[d] <= target[k] + target[d]
                    for k, d in (("x", "width"), ("y", "height"))),
                "Artwork or label lies outside the target")
    if rtl:
        require(label["x"] + label["width"] <= box["x"], "RTL art belongs at the right")
    else:
        require(box["x"] + box["width"] <= label["x"], "LTR art belongs at the left")


def verify_profile(profile, png, expected_size, rtl, scale, copies):
    require(tuple(profile["viewport"]) == expected_size, "Viewport changed")
    require(profile["rtl"] is rtl and profile["text_scale"] == scale, "Wrong direction or text scale")
    require(profile["before"] == BEFORE and profile["after"] == AFTER, "Checkbox states differ")
    require(profile["windows"] == [0, 0, 0], "Missing zero-window observations")
    rows = profile["geometry"]
    require(len(rows) == 5 and len({row["box"]["x"] for row in rows.values()}) == 1, "Rows drifted")
    for row in rows.values():
        verify_box(row, rtl)
    if rtl:
        require(rows["checkbox-demo-consent"]["label"]["height"] >= 96, "Consent label did not wrap")
    receipt = profile["png"]
    require(receipt["name"] == png, "Wrong PNG name")
    color = receipt["error_pixel"]["rgba"]
    require(color == receipt["ink"]["checkbox-demo-error-hint"]["rgba"], "Error colors differ")
    paths = [copy / png for copy in copies]
    for path in paths:
        require(sha(path) == receipt["sha256"], "PNG digest mismatch")
        with Image.open(path) as image:
            image.verify()
        with Image.open(path) as image:
            image.load()
            require(image.format == "PNG" and image.mode == "RGBA" and image.size == expected_size,
                    "Incorrect native PNG format")
            require(list(image.getpixel(tuple(receipt["error_pixel"]["point"]))) == color,
                    "The decoded error border pixel differs")
    require(paths[0].read_bytes() == paths[1].read_bytes(), "Replay PNG bytes differ")
    print(f"{png} {receipt['sha256']}")


def main():
    copies = [ROOT / name for name in ("exact-1", "exact-2")]
    reports = [(copy / "native-gallery-report.json").read_bytes() for copy in copies]
    require(reports[0] == reports[1], "Native replay reports differ")
    report = json.loads(reports[0])
    require(report["schema"] == "surface.test.checkbox.native.v1", "Incorrect report schema")
    require(report["rendering"] == "headless", "Missing headless rendering")
    require(report["transports"] == ["Page.screenshot", "CLI", "MCP"], "Incomplete PNG transports")
    require(len(report["profiles"]) == len(SPECS), "Incorrect profile count")
    for profile, spec in zip(report["profiles"], SPECS, strict=True):
        verify_profile(profile, *spec, copies)
    print("Both light and dark native replays match. All twelve visible-window samples are zero.")


if __name__ == "__main__":
    main()
