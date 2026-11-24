def validate(data):
    assert not data["errors"], data["errors"]
    tokens = data["tokens"]
    assert tokens["order"] == ["banqiao", "taipei-north", "taipei-station", "taoyuan", "hsinchu"]
    assert tokens["face"][2] >= 120 and tokens["face"][3] == 200
    assert len(tokens["glyph_counts"]) == 2 and min(tokens["glyph_counts"]) > 8
    assert_choice(tokens["ax"], 1, "板橋")

    prefix = data["prefix"]
    assert prefix["query"] == ["桃", 1, 1]
    assert prefix["id"] == "taoyuan" and prefix["commits"] == 1
    assert_helper(prefix["suggestion"], "桃園", False)
    assert_choice(prefix["ax"], 4, "桃園")

    validation = data["validation"]
    assert validation["id"] == "taipei-station" and validation["commits"] == 1
    assert validation["resolved_query"] == "台北車站"
    assert validation["unchanged_duplicate_commits"] == 0
    expected = [("台北", "有多個相符地點，請輸入完整名稱"), ("無", "找不到地點"), ("", "請輸入地點")]
    assert len(validation["errors"]) == len(expected)
    for error, (raw, copy) in zip(validation["errors"], expected):
        assert error["raw"] == raw and error["kind"]
        assert_helper(error["ax"], copy, True)
    assert_choice(validation["ax"], 3, "台北", "車站")

    catalog = data["catalog"]
    assert catalog["order"] == tokens["order"][::-1] and catalog["id_retained"] == "banqiao"
    assert catalog["external_commits"] == 0 and catalog["commits"] == 1
    assert catalog["derived_previous_id"] == "taipei-north"
    assert_choice(catalog["reordered"], 5, "板橋")
    assert_choice(catalog["renamed"], 5, "板橋站")
    assert_helper(catalog["rematched_suggestion"], "板橋站", False)
    assert_choice(catalog["previous"], 4, "台北", "北門")

    gestures = data["gestures"]
    face = gestures["face"]
    upper, lower = gestures["outside"]
    assert face[3] == 120 and upper[1] < face[1] < face[1] + face[3] < lower[1]
    assert all(zone[3] >= 48 for zone in gestures["zones"])
    assert gestures["tap_ids"] == ["taipei-north", "banqiao"] and abs(gestures["held_offset"] - 1.23) < .03
    assert gestures["cancel_restored_id"] == "banqiao" and gestures["stale_release_ignored"]
    assert_choice(gestures["before_ax"], 1, "板橋")
    assert_choice(gestures["after_replacement_ax"], 5, "板橋")
    assert gestures["disabled_inert"] and gestures["reduced_motion_id"] == "taipei-north" and gestures["commits"] == 3
    kinds = {(event["device"], event["type"]) for event in gestures["trusted_contacts"]}
    assert {("mouse", "pointerup"), ("touch", "pointerup"), ("touch", "pointercancel")} <= kinds

    compact = data["compact"]
    assert compact["viewport"] == [320, 820] and compact["rtl"] and compact["text_scale"] == 2
    x, y, w, h = compact["editor"]
    assert x >= 0 and x+w <= 320 and y >= 0 and y+h <= 820 and w >= 48 and h >= 48
    assert compact["editor_ink"] > 16
    assert compact["preedit_survived_resize"] and compact["raw"] == "桃" and compact["first_enter_commits"] == 0
    assert compact["committed_id"] == "taoyuan" and compact["commits"] == 1 and compact["restored_editor_focus"]
    assert_helper(compact["description"], "桃園", False)
    assert_choice(compact["roomy_ax"], 4, "桃園")


def assert_choice(evidence, ordinal, *words):
    dom = evidence["dom"]
    assert dom["role"] == "spinbutton" and dom["name"] == "地點"
    assert [dom["min"], dom["max"], dom["now"]] == [1, 5, ordinal]
    assert dom["descendants"] == 0 and dom["wheel_count"] == 1
    assert all(word in dom["text"] for word in words)
    value = evidence["chrome_value"]
    description = evidence["chrome_description"]
    assert isinstance(description, str) and "可輸入部分地名" in description
    assert ((isinstance(value, str) and all(word in value for word in words))
            or (value == ordinal and isinstance(description, str) and all(word in description for word in words)))


def assert_helper(helper, text, invalid):
    assert text in helper["description"] and helper["invalid"] == invalid
    assert all(side > 0 for side in helper["bounds"][2:])
