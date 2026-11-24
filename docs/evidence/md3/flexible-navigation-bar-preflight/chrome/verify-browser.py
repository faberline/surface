"""Verify the focused flexible navigation browser receipt and decoded PNGs."""

from hashlib import sha256
import json
from math import floor
from pathlib import Path
import re
import sys

from PIL import Image


root = Path(sys.argv[1])
stem = "material-flexible-navigation-bar"
report = root / f"browser-{stem}.json"
observer = json.loads((root / "headless-observation.json").read_text())
raw = json.loads(report.read_text())
log = (root / "runner.log").read_text()
steps = re.findall(r"^\s+\[(M\d+)\]", log, flags=re.M)
assert steps == ["M270", "M271", "M272", "M273"]
case = re.search(r"case material \.\.\. ok \(4 steps, (\d+) checks,", log)
assert case is not None and "1 passed; 0 failed" in log
assert (root / "exit.txt").read_text() == "runner=0 tee=0\n"
assert raw["errors"] == [] and "failure" not in raw
assert type(raw["adapter"]["isFallbackAdapter"]) is bool
assert raw["reduced_motion"] is True
assert observer["samples"] > 0 and observer["exit"] == 0
assert observer["visible_windows"] == [] and observer["remaining_chrome"] == []


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def png(stage, wide):
    path = root / f"{stem}-{stage}.png"
    with Image.open(path) as stored:
        assert stored.format == "PNG"
        assert stored.size == ((1920, 1688) if wide else (640, 1048)), stored.size
        return path, stored.convert("RGB")


def browser_sample(image, x, y):
    return list(image.getpixel((floor(x * 2), floor((44 + y) * 2))))


def pixel_slice(image, rect):
    x, y, w, h = rect
    return image.crop((round(x * 2), round((44 + y) * 2),
                       round((x + w) * 2), round((44 + y + h) * 2)))


def color_proof(stage, wide, layout, ink, selected):
    path, image = png(stage, wide)
    y = layout["bar"][1]
    hit = next(item for item in layout["destinations"] if item["id"] == selected)
    x = hit["hit"][0] + hit["hit"][2] / 2
    px, py, w, h = hit["pill"]
    assert (w, h) == (56, 32)
    assert browser_sample(image, x, y + 2) == ink["surface"]
    assert browser_sample(image, px + 7, py + 16) == ink["pill"]
    assert browser_sample(image, px, py) == ink["corner"]
    icon = pixel_slice(image, [px + 16, py + 4, 24, 24])
    assert icon.getcolors(24 * 24 * 4) is not None
    count = next(number for number, rgb in icon.getcolors(24 * 24 * 4) if list(rgb) == ink["icon_ink"])
    assert count == ink["icon_pixels"] and count > 24
    return {"sha256": digest(path), "size": image.size, "icon_pixels": count}, image


geometries = raw["counts_and_colors"]["layouts"]
assert [len(layout["destinations"]) for layout in geometries] == [3, 4, 5]
for layout in geometries:
    assert layout["bar"] == [240, 736, 480, 64]
    assert all(item["hit"][2] == 480 / len(layout["destinations"])
               for item in layout["destinations"])
colors = raw["counts_and_colors"]
summary = {}
for stage in ["light", "dark"]:
    summary[stage], _ = color_proof(stage, True, geometries[-1], colors[stage], "navigation-bar-demo-home")
selector = raw["accessibility"]
assert selector["landmark"] == "主要導覽" and selector["disabled"] == "navigation-bar-demo-explore"
assert selector["preactivations"] == 0
assert selector["tab_order"] == ["navigation-bar-demo-" + item
                                 for item in ["home", "saved", "updates", "settings"]]
assert raw["actions"]["activations"] == 3

compact = raw["compact"]
layout = compact["geometry"]
assert compact["rtl"] and compact["text_scale"] == 2
assert compact["selected"] == "navigation-bar-demo-updates"
assert compact["focused_after_wheel"] == "navigation-bar-demo-settings"
assert compact["activations"] == 4
assert compact["entry_after"][1] < compact["entry_before"][1]
assert sum(compact["entry_after"][i] for i in (1, 3)) <= sum(layout["scroll"][i] for i in (1, 3))
assert layout["bar"] == [0, 368, 320, 112]
assert [item["hit"][0] for item in layout["destinations"]] == [256, 192, 128, 64, 0]
assert len(compact["selected_line_pixels"]) == 2 and min(compact["selected_line_pixels"]) > 32
summary["rtl-large"], before = color_proof("rtl-large", False, layout, compact["initial_paint"], compact["selected"])
summary["rtl-large-scrolled"], after = color_proof(
    "rtl-large-scrolled", False, layout, compact["final_paint"], compact["selected"],
)
bar_top = round((44 + layout["bar"][1]) * 2)
assert before.crop((0, bar_top, before.width, before.height)).tobytes() == after.crop(
    (0, bar_top, after.width, after.height)
).tobytes(), "the actual bar paint differs after the wheel"
x, y, w, h = layout["destinations"][3]["label"]
assert h >= 64
ink = tuple(compact["selected_label_rgb"])
focus_x, focus_y, _, _ = layout["destinations"][4]["hit"]
assert browser_sample(before, focus_x + 1, focus_y + 2) == compact["focused_outline_rgb"]
assert compact["focused_outline_rgb"] == compact["selected_label_rgb"]
for index, expected in enumerate(compact["selected_line_pixels"]):
    line = pixel_slice(before, [x, y + 32 * index, w, 32])
    actual = sum(number for number, rgb in line.getcolors(line.width * line.height) if rgb == ink)
    assert actual == expected and actual > 32, (index, actual, expected)
for name, expected in [("source", 594), ("runtime", 5)]:
    before = (root / f"{name}-before.log").read_bytes()
    assert before == (root / f"{name}-after.log").read_bytes()
    lines = before.splitlines()
    assert len(lines) == expected and all(line.endswith(b": OK") for line in lines)
result = {
    "schema": "surface.test.flexible-navigation-browser-verification.v1",
    "steps": steps,
    "checks": int(case[1]),
    "browser": raw["browser"],
    "adapter": raw["adapter"],
    "window_samples": observer["samples"],
    "visible_windows": 0,
    "remaining_browser_processes": 0,
    "bar_pixels_unchanged_by_wheel": True,
    "selected_line_pixels": compact["selected_line_pixels"],
    "source_manifest_sha256": digest(root.parent / "source.sha256"),
    "gallery_source_manifest_sha256": digest(root.parent / "gallery-source.sha256"),
    "focused_source_manifest_sha256": digest(root.parent / "focused-source.sha256"),
    "runtime_manifest_sha256": digest(root.parent / "runtime.sha256"),
    "report_sha256": digest(report),
    "runner_log_sha256": digest(root / "runner.log"),
    "window_observation_sha256": digest(root / "headless-observation.json"),
    "pngs": summary,
}
print(json.dumps(result, indent=2) + "\n", end="")
