"""Verify stock Search geometry, Chrome AX, and independent Canvas pixels."""

import json
from math import floor
from pathlib import Path

from verify_contrast_report import png_record, png_rows

PREFIX = "material-web-contrast-search-"
WIDE = [1920, 1600]
PHONE = [640, 800]

def chrome_property(node, name):
    return next((item["value"]["value"] for item in node["properties"] if item["name"] == name), None)

def pixel(root, frame, x, y):
    path = root / f"{PREFIX}{frame}.png"
    py = floor(y * 2)
    return list(png_rows(path, {py})[py][floor(x * 2)])

def expected_frames():
    sizes = {
        "docked-rest": WIDE,
        "compact-rest": PHONE,
        "empty-chrome-ax": WIDE,
        "live-centered-editor": WIDE,
        "ltr-start-reference": WIDE,
        "resize-pending-composition": [1200, 1440],
    }
    for style in ("contained", "divided"):
        sizes[f"docked-light-{style}"] = WIDE
        sizes[f"compact-dark-rtl-{style}"] = PHONE
        sizes[f"caret-ltr-{style}"] = WIDE
        sizes[f"caret-rtl-{style}"] = PHONE
        for mode in ("normal", "forced", "forced-typed", "normal-restored"):
            sizes[f"docked-light-{style}-{mode}"] = WIDE
            sizes[f"full-rtl-dark-{style}-{mode}"] = PHONE
    for name in ("empty-hint", "empty-preedit", "typed-query", "pointer-selection", "selected-preedit", "clear-restores-hint"):
        sizes[f"rtl-{name}"] = PHONE
    for frame in ("rest", "entering", "expanded", "exiting", "reduced"):
        sizes[f"motion-docked-contained-{frame}"] = WIDE
        sizes[f"motion-fullscreen-divided-{frame}"] = PHONE
    assert len(sizes) == 46
    return sizes

def validate(root: Path, browser: str, adapter: dict):
    data = json.loads((root / "browser-material-search-overlay.json").read_text())
    assert data["browser"] == browser and data["errors"] == []
    assert data["adapter"] == adapter
    for group in ("docked", "compact"):
        profiles = data[group]["profiles"]
        assert data[group]["errors"] == []
        assert [p["style"] for p in profiles] == ["contained","divided"]
        for profile in profiles:
            assert profile["input"][3] == profile["back"][3] == 48
            if group == "compact":
                assert profile["dialog"] == [0, 0, 320, 400]
                assert profile["back"][0] > profile["input"][0]
                assert profile["bar"][3] == (56 if profile["style"] == "contained" else 72)
            else:
                assert 360 <= profile["dialog"][2] <= 720 and profile["dialog"][3] >= 240
                assert all(a > b for a, b in zip(profile["outside_before"],profile["outside_after"]))

    adaptive = data["adaptive"]
    assert adaptive["scroll_before"] == adaptive["scroll_after"] > 0
    assert adaptive["selection"] == {
        "same": True, "value":"設計", "start": 1, "end": 2, "direction":"backward"}
    assert adaptive["preedit"]["composing"] is True
    assert adaptive["preedit"]["display_value"] == "設ㄐㄧ" and adaptive["browser_preedit"]["same"] is True
    assert adaptive["browser_preedit"]["start"] == 1 and adaptive["browser_preedit"]["end"] == 3
    trace = adaptive["web_trace"]
    assert any(e["type"] == "compositionend" and e.get("data") == "定" for e in trace)
    assert not any("\n" in e["value"] for e in trace)
    assert [(e["key"], e["isComposing"]) for e in trace if e["type"] == "keydown"] == [
        ("Enter", True), ("Enter", False)]
    assert adaptive["docked"][1] > 0 and adaptive["docked"][2] < 600

    interaction = data["interaction"]
    assert chrome_property(interaction["dialog"], "modal") is True
    assert chrome_property(interaction["textbox"], "focused") is True
    assert chrome_property(interaction["disabled"], "disabled") is True
    assert interaction["stable_result"] == "search-doc-review"
    assert interaction["empty_status"] == "找不到相符文件"

    nav = data["navigation"]
    assert nav["direct_first"] == "search-doc-design"
    assert all(nav[item] is True for item in ("disabled_only_held","empty_results_held","modified_held"))
    assert nav["preedit"]["composing"] is True and nav["glyph_pixels"] > 20
    assert abs(nav["glyph_center"] - (nav["input"][1] + nav["input"][3]/2)) <= 5

    rtl = data["rtl"]
    assert rtl["hint_pixels"] > 20 and rtl["empty_input"][3] == 48
    assert rtl["pointer"]["selection_anchor"] == 0 and rtl["pointer"]["selection_cursor"] == 6
    assert rtl["preedit"]["value"] == "設計" and rtl["preedit"]["display_value"] == "H"
    assert rtl["preedit"]["composing"] is True
    assert len(rtl["caret_clearance"]) == 4
    for label, case in rtl["caret_clearance"].items():
        assert case["gap"][1] == case["background"] and case["gap"][1] != case["ring"][1]
        assert case["ring"][1] == case["caret_ink"][1]
        frame = f"caret-{label.replace('_','-')}"
        expected = png_rows(root / f"{PREFIX}{frame}.png", {floor(case["scan_y"] * 2)})
        row = expected[floor(case["scan_y"] * 2)]
        for sample in ("ring","gap","caret_ink"):
            x, rgb = case[sample]
            assert list(row[floor(x*2)]) == rgb, (label,sample,rgb)

    forced = data["forced"]
    assert forced["real_cdp_forced_colors"] is True
    assert forced["fit"]["short"] == [0,0,600,280]
    assert forced["fit"]["docked"][2] < 600 and forced["fit"]["draft"]["value"] == "設"
    forced_profiles = forced["profiles"]
    assert [p["profile"] for p in forced_profiles] == [
        "docked-light-contained","docked-light-divided","full-rtl-dark-contained","full-rtl-dark-divided"]
    for p in forced_profiles:
        pairs = p["browser_palette"]["pairs"]
        assert set(pairs) == {"canvas","button","highlight"}
        bg, fg = (pairs["canvas"][side][:3] for side in ("background","foreground"))
        assert p["fill"] == bg and p["ring"] == [fg, fg] and fg != bg
        assert p["normal_color"] != bg and p["hint_pixels"] > 20 and p["result_ink_pixels"] > 20
        if p["profile"].endswith("divided"):
            assert p["detail"]["divider"] == fg
        x,y,w,h = p["input"]
        assert pixel(root,f"{p['profile']}-forced",x+w/2,y+0.75) == fg

    motion = data["motion"]
    assert motion["manual_performance_time"] is True
    assert [p["presentation"] for p in motion["profiles"]] == ["docked-contained","fullscreen-divided"]
    for p in motion["profiles"]:
        assert p["initial"][3] < p["halfway"][3] < p["final"][3]
        assert p["initial"][3] < p["leaving"][3] < p["final"][3]
        assert p["entry"]["moving"] is True and p["exit"]["moving"] is True
        assert 0 < p["entry"]["progress"] < 1 and 0 < p["exit"]["progress"] < 1
        assert p["reduced_exact_endpoints"] is True
        assert p["unfocused_persistent_web_textarea_ax"] is None
        lifecycle = p["editor_lifecycle"]
        assert lifecycle["open"] == lifecycle["reopened"]
        assert lifecycle["open"] == {"same":True,"hidden":False,"rects":1,"label":"搜尋文件","focused":True}
        for key in ("rest","exit"):
            assert lifecycle[key]["same"] and lifecycle[key]["hidden"]
            assert lifecycle[key]["rects"] == 0 and lifecycle[key]["focused"] is False
        label = p["presentation"]
        width, height = (960,800) if label.startswith("docked") else (320,400)
        for pose in ("rest","entering","expanded","exiting"):
            assert pixel(root,f"motion-{label}-{pose}",width-4,height-4) == p["corner"][pose]
        assert pixel(root,f"motion-{label}-reduced",width-4,height-4) == p["corner"]["expanded"]

    expected = expected_frames()
    actual = {path.stem.removeprefix(PREFIX) for path in root.glob(f"{PREFIX}*.png")}
    assert actual == set(expected), {"extra":sorted(actual-set(expected)), "missing":sorted(set(expected)-actual)}
    pngs = {label: png_record(root / f"{PREFIX}{label}.png", size) for label,size in expected.items()}
    return {"browser":browser,"adapter":adapter,"documents":interaction["stable_result"],
            "adaptive":adaptive,"caret_clearance":rtl["caret_clearance"],
            "forced":forced,"motion":motion,"pngs":pngs}
