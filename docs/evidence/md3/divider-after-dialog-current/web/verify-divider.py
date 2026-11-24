"""Decode the exact focused Divider receipts and verify the physical line pixels."""

import argparse
from hashlib import sha256
import json
from pathlib import Path
import re
import subprocess

from PIL import Image


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def read_profile(run, raw, key, expected, old):
    name, width, ink, leading = expected
    profile = raw[key]
    assert profile["profile"] == name and profile["viewport"] == [width, 640]
    assert profile["accessible_groups"] is True and profile["decorative_focus"] is False
    png = run / f"material-divider-{name}.png"
    with Image.open(png) as original:
        assert original.format == "PNG" and original.size == (width * 2, 1368)
        frame = original.convert("RGB")
    ids = ("full", "inset", "middle", "vertical")
    full, inset, middle, vertical = (profile[identifier]["bounds"] for identifier in ids)
    assert [full[3], inset[3], middle[3], vertical[2], vertical[3]] == [1, 1, 1, 1, 64]
    assert [inset[0]-full[0], full[2]-inset[2], middle[0]-full[0], full[2]-middle[2]] == [leading, 16, 16, 32]
    pixels = {}
    for identifier in ids:
        line = profile[identifier]
        assert tuple(line["rgb"]) == ink
        x, y, w, h = line["bounds"]
        assert x >= 0 and y >= 0 and x+w <= width and y+h <= 640
        # The Web harness sets DPR 2 and a 44 CSS px host bar. Sample both
        # physical pixels of each 1 dp line and a full adjacent stripe.
        coordinates = [2*x, 2*(y+44), 2*(x+w), 2*(y+h+44)]
        assert all(float(number).is_integer() for number in coordinates)
        x0, y0, x1, y1 = map(int, coordinates)
        if identifier == "vertical":
            body = (x0, y0+2, x1, y1-2)
            beside = [(x0-1, y0+2, x0, y1-2), (x1, y0+2, x1+1, y1-2)]
        else:
            body = (x0+2, y0, x1-2, y1)
            beside = [(x0+2, y0-1, x1-2, y0), (x0+2, y1, x1-2, y1+1)]
        line_pixels = frame.crop(body).get_flattened_data()
        assert line_pixels and all(pixel == ink for pixel in line_pixels), identifier
        for region in beside:
            assert ink not in frame.crop(region).get_flattened_data(), identifier
        pixels[identifier] = {"bounds_dp": line["bounds"], "solid_pixels_checked": len(line_pixels),
                              "physical_thickness_px": 2, "adjacent_matching_ink_pixels": 0}
    return {"file": png.name, "width": frame.width, "height": frame.height, "sha256": digest(png),
            "ink_rgb": ink, "lines": pixels, "same_png_as_pre_retarget": png.read_bytes() == (old/png.name).read_bytes(),
            "accessible_groups": profile["accessible_groups"], "decorative_focus": profile["decorative_focus"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    parser.add_argument("old", type=Path)
    args = parser.parse_args()
    run, old = args.run.resolve(), args.old.resolve()
    assert (run/"exit.txt").read_bytes() == b"runner=0 tee=0\n"
    for kind in ("runtime", "source"):
        assert (run/f"{kind}-before.log").read_bytes() == (run/f"{kind}-after.log").read_bytes()
    log = (run/"runner.log").read_text()
    assert re.findall(r"^  \[(M\d+)\]", log, re.M) == ["M264", "M265"]
    match = re.search(r"case material \.\.\. ok \(2 steps, (\d+) checks, (\d+(?:\.\d+)?)s\)", log)
    assert match and f"e2e result: 1 passed; 0 failed; {match[1]} checks" in log
    raw = json.loads((run/"browser-material-divider.json").read_text())
    assert raw["errors"] == [] and "failure" not in raw
    assert raw["adapter"] == {"vendor": "apple", "architecture": "metal-3", "isFallbackAdapter": False}
    assert raw["focus_restored"] is True
    profiles = [
        read_profile(run, raw, "wide", ("wide-light-ltr", 960, (202,196,207), 16), old),
        read_profile(run, raw, "compact", ("compact-dark-rtl-200", 320, (73,69,78), 0), old),
    ]
    procs = subprocess.check_output(["/bin/ps", "-axww", "-o", "pid=,command="], text=True)
    profile = str(run/"browser-temp")
    binary = str(run.parent/"pins/surface-e2e")
    owned = []
    for line in procs.splitlines():
        pid, _, command = line.strip().partition(" ")
        if command.startswith(binary) or (command.startswith("/Applications/Google Chrome.app/") and profile in command):
            owned.append(int(pid))
    assert owned == [], owned
    outcome = {"browser": raw["browser"], "adapter": raw["adapter"], "headless": True,
               "steps": ["M264", "M265"], "checks": int(match[1]), "elapsed_seconds": float(match[2]),
               "page_errors": 0, "remaining_owned_processes": owned, "focus_restored": True, "profiles": profiles}
    (run/"verified-divider.json").write_text(json.dumps(outcome, ensure_ascii=False, indent=2)+"\n")
    print(f"Verified {match[1]} browser checks and both decoded PNGs. All eight lines paint exactly 2 px with no adjacent ink.")


if __name__ == "__main__":
    main()
