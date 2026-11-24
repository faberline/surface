def validate(data):
    assert not data["errors"], data["errors"]

    keyboard = data["keyboard"]
    assert keyboard["boundary_values"] == [10, 2, 10]
    assert keyboard["home_end"] == [2, 10] and keyboard["commits"] == 4
    start, midpoint, end = keyboard["forward"]
    assert abs(start-4) < .02 and 4 < midpoint < 5 and end == 0
    start, midpoint, end = keyboard["reverse"]
    assert abs(start) < .02 and -1 < midpoint < 0 and end == 4
    for receipt, value in zip(keyboard["ax"], keyboard["boundary_values"]):
        numeric(receipt, value)

    drag = data["drag"]
    assert drag["pitch"] >= 48 and drag["held_ink"] > 8
    assert abs(drag["held"]-10.23) < .03 and abs(drag["canceled_phase"]-6.23) < .03
    start, midpoint, end = drag["settling"]
    assert abs(start-drag["held"]) < .02 and 10 < midpoint < start and end == 0
    assert drag["committed"] == 2 and drag["commits"] == 1
    assert {("pointerup", "mouse"), ("pointercancel", "touch")} <= contacts(drag)
    numeric(drag["ax_held"], 10)
    numeric(drag["ax"], 2)

    taps = data["taps"]
    assert taps["face"][3] == 120 and all(zone[3] >= 48 for zone in taps["zones"])
    upper, lower = taps["outside"]
    assert upper[1] < taps["face"][1] < sum(taps["face"][1::2]) < lower[1]
    assert taps["tap_values"] == [2, 10] and taps["commits"] == 2
    assert taps["page_scroll"][0] < taps["page_scroll"][1]
    assert {("pointerup", "mouse"), ("pointerup", "touch"), ("wheel", None)} <= contacts(taps)
    assert taps["back_focus"] == "scroll-loop-numeric-show"

    direction = data["direction"]
    assert direction["peek_labels"] == ["台北 車站"] * 2
    assert len(direction["peek_ink"]) == 2 and min(direction["peek_ink"]) > 8
    assert 0 < direction["upper_phase"] < 1 and -1 < direction["lower_phase"] < 0
    assert direction["accepted_id"] == "taipei-station" and direction["commits"] == 3
    choice(direction["ax"][0], 1, "板橋")
    for receipt in direction["ax"][1:]:
        choice(receipt, 2, "台北 車站")

    catalog = data["catalog"]
    assert abs(catalog["held"]-1.23) < .03 and catalog["order"] == ["taipei-station", "banqiao"]
    assert ("pointerup", "touch") in contacts(catalog)
    assert catalog["stale_release_ignored"] and catalog["external_commits"] == 0
    assert catalog["id_retained"] == "banqiao" and catalog["rematched_suggestion"] == "板橋站"
    assert catalog["wrapped_id"] == "taipei-station" and catalog["commits"] == 1
    assert catalog["back_focus"] == "scroll-loop-choice-show"
    for receipt, (ordinal, text) in zip(catalog["ax"], [(2, "板橋"), (2, "板橋站"), (1, "台北 車站")]):
        choice(receipt, ordinal, text)

    direct = data["compact"]["numeric"]
    assert direct["invalid_input"] == "12" and direct["visible_error"]
    assert direct["committed"] == 2 and direct["commits"] == 1
    numeric(direct["ax"], 2)
    fallback = data["compact"]["choice"]
    assert fallback["viewport"] == [320, 820] and fallback["text_scale"] == 2 and fallback["rtl"]
    x, y, w, h = fallback["editor"]
    assert 0 <= x <= x+w <= 320 and 0 <= y <= y+h <= 820 and min(w, h) >= 48
    assert fallback["ink_count"] > 16 and fallback["preedit_survived_resize"]
    assert fallback["raw"] == "台北車站" and fallback["first_enter_commits"] == 0
    assert fallback["id"] == "taipei-station" and fallback["commits"] == 1 and fallback["restored_editor_focus"]
    choice(fallback["ax"], 2, "台北 車站")


def contacts(receipt):
    return {(item["type"], item["device"]) for item in receipt["contacts"]}


def spin(receipt, label, lo, hi, now, text):
    dom = receipt["dom"]
    assert dom["role"] == "spinbutton" and dom["name"] == label
    assert [dom["min"], dom["max"], dom["now"], dom["text"]] == [lo, hi, now, text]
    assert dom["wheel_count"] == 1 and dom["descendants"] == 0


def numeric(receipt, now):
    spin(receipt, "數量", 2, 10, now, f"{now} 件")
    assert receipt["chrome_value"] == now


def choice(receipt, now, text):
    spin(receipt, "地點", 1, 2, now, text)
    assert "可輸入部分地名" in receipt["chrome_description"]
    assert receipt["chrome_value"] == text or (receipt["chrome_value"] == now and text in receipt["chrome_description"])
