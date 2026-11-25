"""Calibrate Counter R2 against the actual stored native preview and R1 failure."""

from collections import Counter
import json
from pathlib import Path
from runpy import run_path

from gallery_transport import read_png
from native_runtime import digest

R1 = Path("/tmp/surface-md3-counter-native.jGyZbVdk")
PREVIEW = Path("/tmp/surface-md3-counter-preview-v0164.cy5DF1If")
LANE = Path(__file__).resolve().parent
OLD = run_path(str(R1 / "replay-text-field-counter.py"))
NEW = run_path(str(LANE / "replay-text-field-counter.py"))
FIELD = "text-field-counter-demo-editable-counter"
FAILURE_PNG = R1 / "development-1/replay-1/wide-outlined-seed.png"


def rejects(action, expected):
    try:
        action()
    except RuntimeError as error:
        if expected not in str(error):
            raise
        return str(error)
    raise AssertionError(f"Unexpected acceptance: {expected}")


def check():
    preview = json.loads((PREVIEW / "preview.json").read_text())
    failure = digest(FAILURE_PNG)
    assert failure == preview["wide"]["sha256"] == digest(PREVIEW / "wide.png")
    measured = {}
    for mode, report in preview.items():
        image = PREVIEW / report["name"]
        sha = digest(image)
        assert sha == report["sha256"]
        width, height, rows = read_png(image.read_bytes())
        assert (width, height) == (report["width"], report["height"])
        target = report["ink"][FIELD]
        rect, authored = target["bounds"], target["rgba"]
        samples = NEW["sample_pixels"](rows, rect)
        background = Counter(samples).most_common(1)[0][0]
        assert background == ((253, 247, 255, 255) if mode == "wide" else (20, 18, 24, 255))
        old_endpoint = sum(NEW["color_distance"](pixel, authored) <= 3 for pixel in samples)
        verified = NEW["painted_ink"](rows, rect, authored, background)
        wrong_hue = (186, 26, 26, 255) if mode == "wide" else (255, 180, 171, 255)
        bad_hue = rejects(lambda: NEW["painted_ink"](rows, rect, wrong_hue, background), "authored hue and opacity")
        bad_alpha = rejects(lambda: NEW["painted_ink"](rows, rect, (*authored[:3], 97), background), "authored hue and opacity")
        if mode == "wide":
            assert old_endpoint == 0 and verified["peak_coverage"] > .95 and verified["ink_pixels"] >= 30
            measured["original_r1_rejection"] = rejects(
                lambda: OLD["painted_ink"](rows, rect, authored, background), "Text lacks original pixels")
        measured[mode] = {"sha256": sha, "bounds": rect, "background": list(background), "old_endpoint_hits": old_endpoint,
                          "r2_original_pixel_check": verified, "erroneous_palette_rejected": bad_hue,
                          "erroneous_38_percent_alpha_rejected": bad_alpha}
        assert digest(image) == sha
    assert digest(FAILURE_PNG) == failure
    return {"scope": "previously captured original PNG calibration", "failed_run_png_sha256": failure,
            "preview_wide_byte_identical": True, **measured}


if __name__ == "__main__":
    print(json.dumps(check(), ensure_ascii=False, indent=2))
