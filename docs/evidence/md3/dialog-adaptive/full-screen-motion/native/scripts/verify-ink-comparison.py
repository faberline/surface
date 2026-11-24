"""Check the saved-text occlusion count against the original negative PNG pair."""

from hashlib import sha256
import json
import math
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent / "native-scripts"))
from gallery_transport import read_png


def main():
    archive = Path(__file__).resolve().parents[1]
    case = archive / "negative" / "first-clearance"
    report = json.loads((case / "comparison.json").read_text())
    samples = report["samples"]
    images = {}
    for direction, sample in samples.items():
        shot = sample["shot"]
        data = (case / shot["name"]).read_bytes()
        assert sha256(data).hexdigest() == shot["sha256"]
        width, height, images[direction] = read_png(data)
        assert (width, height) == (shot["width"], shot["height"])
        assert sample["windows"] == 0
    assert samples["ltr"]["layout"] == samples["rtl"]["layout"]
    rtl = samples["rtl"]
    saved = rtl["layout"]["bounds"]
    fab = rtl["fab"]

    def overlap(axis, extent):
        first = math.ceil(max(saved[axis], fab[axis]))
        last = math.ceil(min(saved[axis] + saved[extent], fab[axis] + fab[extent]))
        return range(first, last)

    def near_ink(direction, x, y):
        pixel = images[direction][y][4 * x:4 * x + 3]
        return max(abs(a - b) for a, b in zip(pixel, rtl["ink"][:3])) <= 28

    missing = [
        (x, y)
        for y in overlap("y", "height")
        for x in overlap("x", "width")
        if near_ink("ltr", x, y) and not near_ink("rtl", x, y)
    ]
    xs, ys = zip(*missing)
    assert len(missing) == report["missing_ink_pixels"]
    assert [min(xs), min(ys), max(xs), max(ys)] == report["lost_bounds"]
    print(f"Verified {len(missing)} obscured foreground pixels")


if __name__ == "__main__":
    main()
