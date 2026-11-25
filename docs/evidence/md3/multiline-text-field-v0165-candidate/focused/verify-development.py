#!/usr/bin/env python3
"""Read back the focused multiline development reports and original Chrome PNGs."""

import argparse
import hashlib
import json
from pathlib import Path
import re
from PIL import Image, ImageChops


def require(value, reason):
    if not value:
        raise RuntimeError(reason)


def report_json(path):
    return json.loads(path.read_text())


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    parser.add_argument("--wheel-only", action="store_true")
    args = parser.parse_args()
    run = args.run
    art = run / "artifacts"
    report = report_json(art / "browser-material-multiline-text-field.json")
    watch = report_json(run / "headless-window-observation.json")
    log = (run / "runner.log").read_text()
    steps = [f"M{step}" for step in range(341, 345 if args.wheel_only else 346)]
    require(re.findall(r"^  \[(M\d+)\]", log, re.M) == steps, "wrong focused coverage")
    total = re.search(r"e2e result: 1 passed; 0 failed; (\d+) checks;", log)
    require(total and "case material ... ok" in log, "the exact runner did not pass")
    require(not report.get("failure") and report["page_errors"] == [], "page failure")
    require(report["adapter"] == {
        "vendor": "apple", "architecture": "metal-3", "isFallbackAdapter": False,
    }, "development browser did not report Apple Metal")
    require(watch["stock_exit"] == 0 and watch["visible_owned_windows"] == []
            and watch["remaining_observed_processes"] == [] and watch["problems"] == []
            and watch["observations"]["chrome"]["samples"] > 0
            and len(watch["guard_events"]) == 1
            and watch["guard_events"][0]["role"] == "chrome-primary"
            and watch["guard_events"][0]["allowed"] is True, "headless host evidence is incomplete")
    profiles = [
        report["outlined"]["outlined"], report["filled_ime"]["capture"],
        report["compact"]["outlined"], report["compact"]["filled"],
    ]
    if not args.wheel_only:
        profiles.append(report["page_keys"]["paint"])
        require(report["locked_ax"]["skipped_disabled"] is True
                and report["locked_ax"]["active"]["multiline"] is True,
                "locked Chrome AX proof is incomplete")
    names = {profile["screenshot"]: profile["canvas"][2:] for profile in profiles}
    wheel = report["wheel"]
    require(wheel["scroll_offsets"] == [0, 0, 64, 0] and wheel["dpr"] == 2,
            "inner wheel, residual and reversal offsets differ")
    require(wheel["cdp_deltas"] == [96, 320, -416]
            and [event["dy"] for event in wheel["trusted"]] == [48,160,-208],
            "CDP and browser logical wheel receipts differ")
    names.update({name: wheel["canvas"][2:] for name in wheel["captures"]})
    if not args.wheel_only:
        touch = report["touch"]
        require(touch["outer"] == [0, 56, 0]
                and any(event["id"] == touch["owner"] and event["trusted"]
                        and event["kind"] == "lostpointercapture" for event in touch["trace"]),
                "actual touch capture loss or rollback evidence is absent")
        names.update({name: [960, 840] for name in touch["captures"]})
    require(len(names) == (8 if args.wheel_only else 12)
            and set(names) == {path.name for path in art.glob("*.png")}, "original PNG set differs")
    records = {}
    for name, (width, height) in names.items():
        path = art / name
        with Image.open(path) as image:
            image.load()
            expected = (int(width*2), int((height+44)*2))
            require(image.format == "PNG" and image.size == expected, f"wrong device raster: {name}")
            records[name] = {"dimensions": list(image.size), "sha256": sha(path)}

    def cropped(name, rect, top=0., height=None):
        x, y, width, available = rect
        height = available - top if height is None else height
        with Image.open(art / name) as image:
            return image.convert("RGB").crop((
                round(x*2), round((y+44+top)*2), round((x+width)*2), round((y+44+top+height)*2),
            ))

    before, inner, _, restored = wheel["captures"]
    height = wheel["body"][3] - 48
    require(ImageChops.difference(cropped(before, wheel["body"], 48, height),
                                  cropped(inner, wheel["body"], 0, height)).getbbox() is None,
            "original wheel overlap does not translate by the measured 48 dp")
    require(ImageChops.difference(cropped(before, wheel["body"]),
                                  cropped(restored, wheel["restored_body"])).getbbox() is None,
            "restored wheel capture differs in the original body")
    if not args.wheel_only:
        before, held, restored = touch["captures"]
        require(ImageChops.difference(cropped(before, touch["body"]),
                                      cropped(held, touch["held_body"])).getbbox() is not None,
                "the captured pan did not paint different rows")
        require(ImageChops.difference(cropped(before, touch["body"]),
                                      cropped(restored, touch["restored_body"])).getbbox() is None,
                "restored touch capture differs in the original body")
    print(json.dumps({
        "scope": "wheel-only development" if args.wheel_only else "integrated development",
        "steps": steps, "checks": int(total.group(1)), "adapter": report["adapter"],
        "chrome_observations": watch["observations"]["chrome"]["samples"],
        "report_sha256": sha(art / "browser-material-multiline-text-field.json"),
        "window_receipt_sha256": sha(run / "headless-window-observation.json"),
        "captures": records,
    }, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
