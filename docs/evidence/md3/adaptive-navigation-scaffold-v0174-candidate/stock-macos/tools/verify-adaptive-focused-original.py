"""Read the four original q74 screenshots and headless run record."""

import hashlib
import json
from pathlib import Path
import re
import sys

from PIL import Image


def read(path):
    return json.loads(path.read_text())


def require(condition, detail):
    if not condition:
        raise ValueError(detail)


def checksum(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_frame(path, record, size):
    require(path.name == record["image"], "Unexpected screenshot name")
    width, height = size
    with Image.open(path) as png:
        require(png.format == "PNG" and png.width == width * 2, f"Bad PNG size: {path.name}")
        image = png.convert("RGB")
    header = image.height - 2 * height
    require(header == 88, f"Unexpected gallery header: {path.name}")
    x, y, _, _ = record["bounds"]
    require(image.getpixel((round(2 * (x + 3)), header + round(2 * (y + 2))))
            == tuple(record["surface"]), f"Navigation surface color differs: {path.name}")
    left, top, box_width, box_height = record["current_bounds"]
    region = image.crop((round(2 * left), header + round(2 * top),
                         round(2 * (left + box_width)), header + round(2 * (top + box_height))))
    colors = region.getcolors(maxcolors=region.width * region.height)
    palette_counts = {color: count for count, color in colors}
    pixels = palette_counts.get(tuple(record["selected"]), 0)
    require(record["selected_pixels"] == pixels and pixels > 20,
            f"Selected pixel proof differs: {path.name}")
    return {"sha256": checksum(path), "size": image.size, "selected_pixels": pixels}


def verify(root):
    windows = read(root / "headless-window-observation.json")
    events = windows["guard_events"]
    require(windows["stock_exit"] == 0 and not any(windows[key] for key in
            ("visible_owned_windows", "remaining_observed_processes", "problems")),
            "Visible or surviving processes, or unsuccessful runner")
    require(len(events) == 1 and events[0]["allowed"] and events[0]["role"] == "chrome-primary"
            and events[0]["argv"].count("--headless=new") == 1, "Wrong guarded Chrome launch")
    log = (root / "runner.log").read_text()
    require(re.findall(r"^  \[(M\d+)\]", log, re.M) == ["M366", "M367", "M368", "M369"]
            and re.search(r"^case material \.\.\. ok \(4 steps, 200 checks,", log, re.M)
            and re.search(r"^e2e result: 1 passed; 0 failed; 200 checks;", log, re.M),
            "Wrong original ordered checks")
    artifacts = root / "artifacts"
    report_path = artifacts / "material-adaptive-navigation.json"
    report = read(report_path)
    require(report["draft"] == {"value": "draft", "runtime": [5, 3], "browser": ["draft", 3, 5, "backward"]},
            "Original draft and backward selection differ")
    dark = report["dark"]
    require(dark["rail_scroll"] == [0.0, 100.0] and dark["page_scroll"] == 0
            and dark["paint"]["bounds"] == [744, 0, 96, 300]
            and report["bars"]["compact"]["bounds"] == [0, 556, 599, 64]
            and report["bars"]["medium"]["bounds"] == [0, 556, 600, 64]
            and report["rail"]["paint"]["bounds"] == [0, 0, 96, 620],
            "Original responsive layout or independent scroll proof differs")
    profiles = ((report["bars"]["compact"], (599, 620)), (report["bars"]["medium"], (600, 620)),
                (report["rail"]["paint"], (840, 620)), (dark["paint"], (840, 300)))
    require({record["image"] for record, _ in profiles} == {p.name for p in artifacts.glob("*.png")},
            "Original screenshot set differs")
    pictures = {record["image"]: verify_frame(artifacts / record["image"], record, size)
                for record, size in profiles}
    return {"checks": 200, "window_samples": windows["launch_observations"]["chrome-primary"]["samples"],
            "visible_windows": 0, "remaining_processes": 0, "runner_log_sha256": checksum(root / "runner.log"),
            "report_sha256": checksum(report_path), "pictures": pictures, "draft": report["draft"],
            "rail_scroll": dark["rail_scroll"], "page_scroll": dark["page_scroll"]}


if __name__ == "__main__":
    print(json.dumps(verify(Path(sys.argv[1])), indent=2, ensure_ascii=False))
