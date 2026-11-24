"""Check the two archived native Switch replays and decode their PNGs."""

import hashlib
import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
PROFILES = (
    ("wide-light.png", (960, 800), False, 1),
    ("compact-dark-rtl-200.png", (320, 640), True, 2),
)


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify(path, profile, filename, size, rtl, scale):
    require(tuple(profile["viewport"]) == size, "Incorrect profile viewport")
    require(profile["rtl"] is rtl and profile["text_scale"] == scale, "Wrong direction or text scale")
    require(profile["windows"] == [0, 0, 0], "The profile is missing its zero-window samples")
    require(profile["checked"] == [True, False, False, False, True], "Unexpected checked states")

    geometry = profile["geometry"]
    require(len(geometry) == 5, "Five Switch rows were expected")
    require(len({row["track"]["x"] for row in geometry.values()}) == 1, "Tracks are not aligned")
    for row in geometry.values():
        target = row["target"]
        track = row["track"]
        require(target["width"] >= 48 and target["height"] >= 48, "Switch target is too small")
        require((track["width"], track["height"]) == (52, 32), "Unexpected track geometry")
        if rtl:
            require(track["x"] == target["x"] + 5, "The track missed the logical trailing edge")
        else:
            require(track["x"] + track["width"] == target["x"] + target["width"] - 5,
                    "The track missed the logical trailing edge")
    if rtl:
        require(geometry["switch-demo-both-icons"]["label"]["height"] >= 96, "Large label did not wrap")

    info = profile["png"]
    require(info["name"] == filename and sha(path) == info["sha256"], "PNG digest mismatch")
    with Image.open(path) as image:
        image.verify()
    with Image.open(path) as image:
        image.load()
        require(image.format == "PNG" and image.mode == "RGBA" and image.size == size, "Wrong PNG format")
        sample = info["handle_pixel"]
        require(list(image.getpixel(tuple(sample["point"]))) == sample["rgba"], "Handle pixel differs")
    return info["sha256"]


def main():
    reports = [ROOT / name / "native-gallery-report.json" for name in ("final-1", "final-2")]
    require(reports[0].read_bytes() == reports[1].read_bytes(), "Replay reports differ")
    report = json.loads(reports[0].read_text())
    require(report["schema"] == "surface.test.switch.native.v1", "Unexpected report schema")
    require(report["rendering"] == "headless", "Headless flag was missing")
    require(report["transports"] == ["Page.screenshot", "CLI", "MCP"], "Incomplete transports")
    require(len(report["profiles"]) == len(PROFILES), "Missing or extra profile")
    for profile, (filename, size, rtl, scale) in zip(report["profiles"], PROFILES, strict=True):
        paths = [root.parent / filename for root in reports]
        for path in paths:
            verify(path, profile, filename, size, rtl, scale)
        require(paths[0].read_bytes() == paths[1].read_bytes(), "PNG bytes differ across replays")
        print(f"{filename} {profile['png']['sha256']}")
    print("Two repeatable native profiles passed. All twelve visible-window samples are zero.")


if __name__ == "__main__":
    main()
