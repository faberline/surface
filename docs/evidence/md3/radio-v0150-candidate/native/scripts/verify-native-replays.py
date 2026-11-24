"""Decode the two native Radio Gallery replays and check their receipts."""

import hashlib
import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
PROFILES = (
    ("wide-light.png", (960, 800), False, 1, "right"),
    ("compact-dark-rtl-200.png", (320, 720), True, 2, "left"),
)


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def measured(rect, side):
    return (rect["width"], rect["height"]) == (side, side)


def verify_profile(receipt, copies, png, size, rtl, scale, arrow):
    require(tuple(receipt["viewport"]) == size, "Wrong viewport")
    require(receipt["rtl"] is rtl and receipt["text_scale"] == scale, "Wrong profile settings")
    require(receipt["windows"] == [0, 0, 0], "Missing zero-window observations")
    require(receipt["checked_at_entry"] == "radio-demo-weekly", "Tab missed the checked radio")
    require(receipt["physical_next"] == arrow, "Expected physical direction changed")
    require(receipt["no_selection_forward"] == "radio-demo-daily"
            and receipt["no_selection_reverse"] == "radio-demo-monthly", "No-selection entry changed")
    rows = receipt["geometry"]
    require(len(rows) == 4 and len({row["ring"]["x"] for row in rows.values()}) == 1, "Radio rows drifted")
    require(all(measured(row["ring"], 20) and measured(row["focus_ring"], 30)
                and measured(row["state_layer"], 40) for row in rows.values()), "Radio artwork dimensions changed")
    weekly = rows["radio-demo-weekly"]
    require(measured(weekly["dot"], 10), "Selected dot changed")
    if rtl:
        require(weekly["label"]["height"] >= 96, "Large weekly label did not wrap")
    info = receipt["png"]
    paths = [copy / png for copy in copies]
    raw = [path.read_bytes() for path in paths]
    require(info["name"] == png and raw[0] == raw[1], "Replay PNGs differ")
    require(hashlib.sha256(raw[0]).hexdigest() == info["sha256"], "PNG digest mismatch")
    for path in paths:
        with Image.open(path) as image:
            image.verify()
        with Image.open(path) as image:
            image.load()
            require(image.format == "PNG" and image.mode == "RGBA" and image.size == size, "Wrong PNG format")
            require(list(image.getpixel(tuple(info["dot_pixel"]["point"]))) == info["dot_pixel"]["rgba"],
                    "Decoded selected-dot color differs")
    print(f"{png} {info['sha256']}")


def main():
    copies = [ROOT / sub for sub in ("exact-1", "exact-2")]
    reports = [(copy / "native-gallery-report.json").read_bytes() for copy in copies]
    require(reports[0] == reports[1], "Replay reports differ")
    report = json.loads(reports[0])
    require(report["schema"] == "surface.test.radio.native.v1", "Wrong report schema")
    require(report["rendering"] == "headless", "Missing headless rendering")
    require(report["transports"] == ["Page.screenshot", "CLI", "MCP"], "Missing transport")
    require(len(report["profiles"]) == len(PROFILES), "Wrong profile count")
    for profile, spec in zip(report["profiles"], PROFILES, strict=True):
        verify_profile(profile, copies, *spec)
    print("Both Radio profiles match across replays. All twelve window observations are zero.")


if __name__ == "__main__":
    main()
