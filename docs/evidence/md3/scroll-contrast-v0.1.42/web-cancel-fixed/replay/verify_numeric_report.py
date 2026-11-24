def validate(data):
    assert not data["errors"], data["errors"]
    tokens = data["tokens"]
    assert [size["choice"] for size in tokens["sizes"]] == ["small", "medium", "large"]
    assert [size["face"][2:] for size in tokens["sizes"]] == [[100, 200], [120, 200], [186, 200]]
    assert all(pixels >= 8 for pixels in tokens["ink_counts"])
    assert tokens["hover"] != tokens["face_color"]
    assert tokens["focus"]["gap"] == tokens["corner"]
    assert tokens["focus"]["bounds"][2:] == [130, 210]
    assert tokens["ax"]["dom"]["text"] == "14 件"
    taps = data["taps"]
    assert taps["face"][3] == 120
    face = taps["face"]
    assert all(zone[3] >= 48 for zone in taps["zones"])
    assert taps["outside_points"][0][1] < face[1] < face[1] + face[3] < taps["outside_points"][1][1]
    assert taps["commits"] == [16, 14] and taps["disabled_inert"] and taps["center_opens_editor"]
    assert taps["ancestor_scroll"][0] < taps["ancestor_scroll"][1]
    assert taps["disabled_scroll"][0] < taps["disabled_scroll"][1]
    editor = data["editor"]
    assert editor["values"] == [14, 16, 14, 22] and editor["commits"] == 3
    assert editor["editor_focus_restored"] and editor["aria_invalid_cleared_on_edit"]
    assert len({error["kind"] for error in editor["errors"]}) == 3
    assert [error["raw"] for error in editor["errors"]] == ["15", "999", "abc"]
    assert all(error["description"] for error in editor["errors"])
    ime = data["ime"]
    assert ime["first_enter_commits"] == 0 and ime["explicit_confirm_commits"] == 1
    assert ime["mode_press_preserved_preedit"] and ime["value"] == 22
    gesture = data["gestures"]
    assert gesture["canceled_to"] == 14 and gesture["accepted"] == 16 and gesture["commits"] == 2
    first, middle, end = gesture["spring"]
    assert first > middle > end == 8
    first, middle, end = gesture["painted_glyph_y"]
    assert first > middle > end
    assert gesture["reduced_value"] == 18 and abs(gesture["reduced_offset"] - 9) < 0.001
    contacts = gesture["trusted_contacts"]
    mouse = [event for event in contacts if event["device"] == "mouse"]
    assert [event["kind"] for event in mouse] == [
        "pointerdown", "gotpointercapture", "lostpointercapture", "pointerup",
    ], mouse
    assert len({event["id"] for event in mouse}) == 1 and mouse[2]["buttons"] == 1, mouse
    assert any(event["kind"] == "pointercancel" and event["device"] == "touch" for event in contacts)
    compact = data["compact"]
    assert compact["viewport"] == [320, 820] and compact["rtl"] and compact["text_scale"] == 2
    assert compact["short_range_scroll"][3] >= 224
    assert all(side >= 48 for side in compact["fallback_editor"][2:])
    assert compact["draft_preserved"] == "1300" and compact["accepted"] == 1300 and compact["commits"] == 1
    assert compact["pointer_focus"] == {"runtime": "scroll-field-demo-value-adjustable", "browser": "CANVAS"}
    assert compact["roomy_ax"]["dom"]["text"].replace(",", "") == "1300 件"
    assert compact["resize_restores_editor"] and compact["preedit_survived_resize"]
