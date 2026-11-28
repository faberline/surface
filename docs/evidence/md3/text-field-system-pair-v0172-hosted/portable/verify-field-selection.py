#!/usr/bin/env python3
"""Check q72 Field/FieldText selection screenshots from the original CI ZIP."""
from hashlib import sha256
import json
from pathlib import Path
from PIL import Image

PREFIX = "material-web-contrast-selection-browser"
PNG_SUFFIXES = (
    "delayed-select-arrow-forced", "forced-all", "forced-clear",
    "forced-partial", "forced-preedit", "forced-prefix", "forced-suffix",
    "native-arrow-forced", "normal-clear", "normal-partial", "normal-restored",
)
JSON_NAMES = (
    "browser-material-web-contrast-selection.json",
    "browser-material-web-contrast-selection-browser.json",
    "material-web-contrast-selection-browser-native-arrow.json",
    "material-web-contrast-selection-browser-delayed-select-arrow.json",
)

def require(ok, message):
    if not ok:
        raise ValueError(message)

def data(path):
    return json.loads(path.read_text())

def sha(path):
    return sha256(path.read_bytes()).hexdigest()

def verify(root):
    expected_pngs = {f"{PREFIX}-{suffix}.png" for suffix in PNG_SUFFIXES}
    actual_pngs = {p.name for p in root.glob(f"{PREFIX}-*.png")}
    require(actual_pngs == expected_pngs, "Original Field/FieldText PNG inventory differs")
    require({p.name for p in root.glob("browser-material-web-contrast-selection*.json")}
            | {p.name for p in root.glob("material-web-contrast-selection-browser-*.json")} == set(JSON_NAMES),
            "Original Field/FieldText report inventory differs")
    summary = data(root / JSON_NAMES[0])
    report = data(root / JSON_NAMES[1])
    lane = report
    palette = lane["palette"]
    field = palette["field"]
    pairs = palette["pairs"]
    require(lane["palette_source"] == "CDP-emulated browser system colors"
            and field == pairs["canvas"]
            and palette["field_raw"]["background"] == palette["raw"]["canvas"]["background"]
            and palette["field_raw"]["foreground"] == palette["raw"]["canvas"]["foreground"],
            "Field paint is not linked to the original system Canvas pair")
    require(lane["device_pixel_ratio"] == 1.25 and lane["physical_size"] == [400, 500]
            and lane["raw"] == "1688" and lane["selected_range"] == [1, 3]
            and lane["selected_text"] == "68" and lane["editing"]["browser_selection"] == [1, 3]
            and lane["editing"]["editor"]["value"] == "1688"
            and lane["editing"]["focus"] == "scroll-popup-demo-quantity-editor"
            and lane["editing"]["popup_open"] is True,
            "FieldText editor, selection, or focus receipt differs")
    paint = lane["paint"]
    require(paint["mismatches"] == 0 and paint["comparison_pixels"] > 1000
            and paint["selected_ink_pixels"] > 0 and paint["left_unselected_ink_pixels"] > 0
            and paint["right_unselected_ink_pixels"] > 0
            and paint["foreground_roles_distinguishable"] is True
            and lane["selected_foreground_distinguished"] is True
            and lane["normal_restoration"]["restore_mismatches"] == 0
            and lane["errors"] == [], "System Field and selected FieldText color proof failed")
    x0, x1 = paint["comparison_x"]
    left, right = paint["seams"]
    y0 = paint["seam_row"]
    require(0 <= x0 < left < right < x1 <= 400 and 0 <= y0 < 500
            and paint["comparison_pixels"] % (x1 - x0) == 0,
            "Reported pixel comparison range is invalid")
    rows = paint["comparison_pixels"] // (x1 - x0)
    require(y0 + rows <= 500, "Reported FieldText comparison is outside the PNG")
    images = {}
    for name in expected_pngs:
        path = root / name
        with Image.open(path) as raw:
            require(raw.format == "PNG" and raw.size == (400, 500),
                    f"Wrong original screenshot dimensions: {name}")
            raw.load()
            images[name] = raw.convert("RGB")
    partial = images[f"{PREFIX}-forced-partial.png"]
    all_text = images[f"{PREFIX}-forced-all.png"]
    clear = images[f"{PREFIX}-forced-clear.png"]
    checks = 0
    for y in range(y0, y0 + rows):
        for x in range(x0, x1):
            reference = all_text if left <= x < right else clear
            require(partial.getpixel((x, y)) == reference.getpixel((x, y)),
                    "Original FieldText selection pixels do not match Field and selected-text references")
            checks += 1
    require(checks == paint["comparison_pixels"], "Pixel comparison count differs")
    require(summary["browser_foreground_pair_distinguishable"] is True
            and summary["synthetic_fallback"] is None
            and summary["foreground_evidence"] == "browser"
            and summary["browser"] == lane,
            "FieldText selection did not use the original browser system pair")
    arrows = [data(root / name) for name in JSON_NAMES[2:]]
    require([v["scoped_select_delay"] for v in arrows] == [False, True]
            and all(v["keydown_event"] == {"key": "ArrowRight", "trusted": True, "prevented": False}
                    and v["intercepted_select_events"] == 0
                    and v["after_key_down"]["browser"] == [4, 4]
                    and v["after_key_down"]["runtime"] == [3, 3]
                    and v["after_key_up"]["runtime"] == [4, 4]
                    and v["restored_normal"] == v["after_key_up"] for v in arrows),
            "Native or delayed select redraw receipt differs")
    return {"field_reports": len(JSON_NAMES), "field_selection_pngs": len(expected_pngs),
            "selection_pixels_read": checks,
            "png_sha256": {name: sha(root / name) for name in sorted(expected_pngs)},
            "field_text_selection": "system Field and selected FieldText verified"}
