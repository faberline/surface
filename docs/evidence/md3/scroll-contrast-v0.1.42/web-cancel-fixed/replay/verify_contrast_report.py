"""Check real browser contrast reports and preedit pixels from the stock runner."""

from hashlib import file_digest
import json
from pathlib import Path
import re
from struct import unpack
import subprocess
import sys
from zlib import crc32, decompress


def json_file(root, name):
    return json.loads((root / name).read_text())


def png_record(path, size):
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    offset = 8
    size_read = None
    saw_idat = False
    while offset < len(data):
        length = unpack(">I", data[offset:offset + 4])[0]
        chunk = data[offset + 4:offset + 8 + length]
        tag = chunk[:4]
        checksum = unpack(">I", data[offset + 8 + length:offset + 12 + length])[0]
        assert crc32(chunk) == checksum, (path, tag)
        offset += 12 + length
        if tag == b"IHDR":
            size_read = list(unpack(">II", chunk[4:12]))
        if tag == b"IDAT":
            saw_idat = True
        if tag == b"IEND":
            break
    assert saw_idat and tag == b"IEND" and offset == len(data) and size_read == size, (path, size_read, size)
    with path.open("rb") as file:
        digest = file_digest(file, "sha256").hexdigest()
    return {"physical_size": size, "bytes": len(data), "sha256": digest}


def validate_boundary(bound):
    assert bound["border_present"] is True
    assert bound["exterior"] == bound["interior"] == bound["expected_fill"]
    assert bound["expected_ink"] != bound["expected_fill"] and bound["expected_ink"] in bound["edge"]


def validate_selection(lane):
    assert lane["device_pixel_ratio"] == 1.25 and lane["physical_size"] == [400, 500]
    assert lane["errors"] == []
    assert lane["selected_range"] == [1, 3] and lane["selected_text"] == "68" and lane["raw"] == "1688"
    assert lane["editing"]["browser_selection"] == [1, 3]
    assert lane["editing"]["quantity"]["value"] == 1200 and lane["editing"]["quantity"]["commits"] == 0
    assert lane["editing"]["unit"]["id"] == "ml" and lane["editing"]["unit"]["commits"] == 0
    paint = lane["paint"]
    assert paint["comparison_pixels"] > 1000 and paint["mismatches"] == 0
    assert all(paint[key] > 0 for key in ("selected_ink_pixels", "left_unselected_ink_pixels", "right_unselected_ink_pixels"))
    assert lane["normal_restoration"]["selected_pixels_changed_from_clear"] > 100
    assert lane["normal_restoration"]["restore_mismatches"] == 0
    return paint




def validate(root: Path, browser: str, adapter: dict) -> dict:
    report = json_file(root, "browser-material-web-contrast.json")
    assert report["errors"] == [] and report["browser"] == browser
    assert adapter == {"vendor": "apple", "architecture": "metal-3", "isFallbackAdapter": False}
    assert report["preference_source"] == "CDP-emulated browser preference"
    assert report["paint_source"] == "headless Chrome WebGPU Canvas-only PNG"
    assert report["real_cases_used_palette_injection"] is False

    preferences = report["preferences"]
    assert preferences["modes"] == ["forced", "increased", "normal", "forced"]
    assert preferences["probe_hidden_and_ax_excluded"] and preferences["probe_removed_on_exit"]
    assert preferences["normal_theme"] != preferences["increased_theme"]
    assert all(set(preferences[when]["css"]["pairs"]) == {"canvas", "button", "highlight"} for when in ("startup", "ending"))

    popup = report["popup_paint"]
    assert popup["viewport"] == [320, 400] and popup["rtl"] is True and popup["text_scale"] == 2
    assert popup["authored_dark"] and [item["mode"] for item in popup["samples"]] == ["normal", "forced", "restored"]
    normal, forced, restored = popup["samples"]
    assert forced["css"]["pairs"] == forced["accepted"]["system_colors"]
    for role in ("root", "panel", "field", "button", "focus_top", "focus_bottom", "focus_gap"):
        assert normal["paint"][role] == restored["paint"][role]
    for role in ("root", "field", "focus_top"):
        assert normal["paint"][role] != forced["paint"][role]

    validate_boundary(report["popup_edge"])
    assert report["popup_edge"] == json_file(root, "material-web-contrast-popup-boundary.json")
    wheel = report["wheel_edge"]
    validate_boundary(wheel["outline"])
    assert wheel["outline"] == json_file(root, "material-web-contrast-wheel-boundary.json")
    assert wheel["disabled_aria"] and all(label["ink_pixels"] > 0 for label in wheel["labels"])
    assert wheel["disabled_ink_sample"]["disabled_exact_pixels"] < wheel["disabled_ink_sample"]["enabled_exact_pixels"]

    preedit = report["preedit"]
    assert preedit["composition"] == "CDP synthetic preedit"
    assert preedit["close_during_preedit_ignored"] and preedit["quantity_commits"] == 1
    assert preedit["recovered_quantity"] == 1206 and preedit["returned_focus"] == "scroll-popup-demo-opener"
    assert all(ax["chrome_focused"] and "請輸入偶數" in ax["chrome_description"] for ax in preedit["chrome_modes"])

    selection = report["selection"]
    assert selection == json_file(root, "browser-material-web-contrast-selection.json")
    browser_lane = selection["browser"]
    assert browser_lane == json_file(root, "browser-material-web-contrast-selection-browser.json")
    assert browser_lane["palette_source"] == "CDP-emulated browser system colors" and browser_lane["preedit"]
    paint = validate_selection(browser_lane)
    native_arrow = browser_lane["native_arrow_redraw"]
    delayed_arrow = browser_lane["delayed_select_redraw"]
    for phase, measured, delayed in (
        ("native-arrow", native_arrow, False),
        ("delayed-select-arrow", delayed_arrow, True),
    ):
        assert measured == json_file(root, f"material-web-contrast-selection-browser-{phase}.json")
        assert measured["keydown_event"] == {"key": "ArrowRight", "trusted": True, "prevented": False}
        assert measured["scoped_select_delay"] is delayed
        assert measured["before"]["browser"] == measured["before"]["runtime"] == [3, 3]
        for name in ("after_key_down", "after_forced_redraw", "after_key_up"):
            state = measured[name]
            assert state["browser"] == [4, 4] and state["runtime"] in ([3, 3], [4, 4])
            assert state["value"] == "1688" and state["composing"] is False and state["focused"] is True
        if delayed:
            assert measured["after_key_down"]["runtime"] == measured["after_forced_redraw"]["runtime"] == [3, 3]
        assert measured["after_key_up"]["runtime"] == [4, 4]
        assert measured["restored_normal"] == measured["after_key_up"]
        assert measured["normal_canvas_sample"] != measured["forced_canvas_sample"]
        forced_path = root / f"material-web-contrast-selection-browser-{phase}-forced.png"
        # Surface samples the CSS point at floor(point × 1.25 DPR).
        assert list(png_rows(forced_path, {496})[496][3]) == measured["forced_canvas_sample"]
        assert measured["css_canvas"] == [*measured["forced_canvas_sample"], 255]
    reverse = browser_lane["backward_range"]
    assert reverse["browser"] == [3, 4] and reverse["runtime"] == [4, 3] and reverse["direction"] == "backward"
    distinguishable = browser_lane["selected_foreground_distinguished"]
    assert distinguishable == selection["browser_foreground_pair_distinguishable"] == paint["foreground_roles_distinguishable"]
    if distinguishable:
        assert selection["foreground_evidence"] == "browser" and selection["synthetic_fallback"] is None
    else:
        assert selection["foreground_evidence"] == "separate synthetic contrast pair"
        synthetic = selection["synthetic_fallback"]
        assert synthetic == json_file(root, "browser-material-web-contrast-selection-injected.json")
        assert synthetic["palette_source"] == "synthetic injected system-probe colors"
        assert synthetic["preedit"] is None and synthetic["selected_foreground_distinguished"]
        assert validate_selection(synthetic)["repeat_blend_discriminable"]

    underline = json_file(root, "browser-material-web-contrast-preedit-underline.json")
    assert underline == json_file(root, "material-web-contrast-preedit-underline-measurements.json")
    assert underline["source"] == "CDP synthetic composition; Chrome-resolved CSS colors"
    assert underline["device_pixel_ratio"] == 2 and underline["physical_size"] == [640, 800]
    assert underline["normal_pixel_mismatches"] == 0
    assert underline["preedit_clear"]["textarea_selection"] == [4, 4]
    assert underline["preedit_partial"]["textarea_selection"] == [1, 3]
    assert underline["preedit_partial"]["runtime"]["value"] == "1680"
    assert underline["preedit_partial"]["runtime"]["display_value"] == "1688"
    assert underline["preedit_partial"]["runtime"]["preedit_range"] == {"start": 0, "end": 4}
    assert underline["stroke_row"] > underline["selection_witness_row"]
    assert underline["highlighted_columns"] >= 12
    assert underline["ordinary_ink_partial"][1] == 0
    assert underline["ordinary_ink_partial"][0] >= underline["clear_outline_pixels"][0]
    assert underline["ordinary_ink_partial"][2] >= underline["clear_outline_pixels"][2]
    assert underline["selected_ink_middle"] >= underline["clear_outline_pixels"][1]
    verify_underline_png_pixels(root, underline)

    sizes = {
        "popup-normal": [640, 800], "popup-forced": [640, 800], "popup-restored": [640, 800],
        "popup-boundary": [640, 800], "popup-synthetic-preedit": [640, 800],
        "wheel-resting": [1920, 1640], "wheel-focused": [1920, 1640], "wheel-disabled": [1920, 1640],
        "selection-browser-forced-preedit": [400, 500],
        "selection-browser-native-arrow-forced": [400, 500],
        "selection-browser-delayed-select-arrow-forced": [400, 500],
    }
    lanes = ("browser",) if distinguishable else ("browser", "injected")
    variants = ("normal-clear", "normal-partial", "forced-partial", "forced-clear", "forced-all", "forced-prefix", "forced-suffix", "normal-restored")
    sizes.update({f"selection-{lane}-{variant}": [400, 500] for lane in lanes for variant in variants})
    frames = {name: png_record(root / f"material-web-contrast-{name}.png", size) for name, size in sizes.items()}
    underline_frames = (
        "plain-clear", "plain-all", "plain-prefix", "plain-suffix", "plain-partial",
        "preedit-clear", "preedit-normal-before", "preedit-partial", "preedit-normal-restored",
    )
    frames.update({
        f"preedit-underline-{name}": png_record(root / f"material-web-contrast-preedit-underline-{name}.png", [640, 800])
        for name in underline_frames
    })
    return {
        "browser": report["browser"],
        "adapter": adapter,
        "forced_system_pairs": forced["css"]["pairs"],
        "foreground_evidence": selection["foreground_evidence"],
        "selection_seam_pixels": paint["comparison_pixels"],
        "native_arrow_redraw": native_arrow,
        "delayed_select_redraw": delayed_arrow,
        "backward_range": reverse,
        "preedit_underline": underline,
        "popup_border": report["popup_edge"],
        "wheel_border": wheel["outline"],
        "pngs": frames,
    }


def png_rows(path: Path, needed_rows: set[int]):
    """Decode the requested RGB(A) rows without relying on the E2E decoder."""
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    width, height, bits, kind, compression, filtering, interlace = unpack(">IIBBBBB", data[16:29])
    assert bits == 8 and kind in (2, 6)
    assert (compression, filtering, interlace) == (0, 0, 0)
    assert min(needed_rows) >= 0 and max(needed_rows) < height
    channels = 3 if kind == 2 else 4
    stride = width * channels
    chunks = []
    offset = 8
    while offset < len(data):
        length = unpack(">I", data[offset:offset + 4])[0]
        if data[offset + 4:offset + 8] == b"IDAT":
            chunks.append(data[offset + 8:offset + 8 + length])
        offset += length + 12
    pixels = decompress(b"".join(chunks))
    assert len(pixels) == height * (stride + 1)
    rows = {}
    previous = bytearray(stride)

    def predictor(left, above, up_left):
        p = left + above - up_left
        dl, da, du = abs(p - left), abs(p - above), abs(p - up_left)
        return left if dl <= da and dl <= du else above if da <= du else up_left

    for row_number in range(max(needed_rows) + 1):
        offset = row_number * (stride + 1)
        method = pixels[offset]
        row = bytearray(pixels[offset + 1:offset + 1 + stride])
        if method not in range(5):
            raise ValueError(f"Unknown PNG filter {method} in {path}")
        if method:
            for index, value in enumerate(row):
                left = row[index - channels] if index >= channels else 0
                above = previous[index]
                up_left = previous[index - channels] if index >= channels else 0
                estimate = (0, left, above, (left + above) // 2, 0)[method]
                if method == 4:
                    estimate = predictor(left, above, up_left)
                row[index] = (value + estimate) & 255
        if row_number in needed_rows:
            rows[row_number] = [tuple(row[i:i + 3]) for i in range(0, len(row), channels)]
        previous = row
    return rows


def verify_underline_png_pixels(root, underline):
    stroke, witness = underline["stroke_row"], underline["selection_witness_row"]
    ranges = [range(*span) for span in underline["physical_spans"]]
    ordinary = tuple(underline["expected_ink"]["canvas_text"])
    selected = tuple(underline["expected_ink"]["highlight_text"])
    assert ordinary != selected
    backgrounds = underline["system_colors"]["pairs"]
    assert tuple(backgrounds["canvas"]["foreground"][:3]) == ordinary
    assert tuple(backgrounds["highlight"]["foreground"][:3]) == selected
    canvas = tuple(backgrounds["canvas"]["background"][:3])

    def rows(name, *indices):
        path = root / f"material-web-contrast-preedit-underline-{name}.png"
        return png_rows(path, set(indices))

    plain = rows("plain-clear", stroke)[stroke]
    clear = rows("preedit-clear", stroke)[stroke]
    assert [
        sum(plain[x] == canvas and clear[x] == ordinary for x in span)
        for span in ranges
    ] == underline["clear_outline_pixels"]
    partial = rows("preedit-partial", stroke, witness)
    assert [sum(partial[stroke][x] == ordinary for x in span) for span in ranges] == underline["ordinary_ink_partial"]
    assert sum(partial[stroke][x] == selected for x in ranges[1]) == underline["selected_ink_middle"]
    # The ordinary partial frame was generated using trusted keys; it is
    # independent of the synthetic composition and the Core underline path.
    selected_plain = rows("plain-partial", witness)[witness]
    assert sum(partial[witness][x] == selected_plain[x] for x in ranges[1]) >= underline["highlighted_columns"]
