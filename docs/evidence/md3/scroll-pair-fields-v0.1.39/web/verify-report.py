QUANTITY = "scroll-pair-demo-quantity"
UNIT = "scroll-pair-demo-unit"


def validate(data):
    assert not data["errors"]

    semantics = data["semantics"]
    assert semantics["group"] == {
        "role": "group", "name": "容量", "tabindex": None, "action": None, "names": ["數量", "單位"]
    }
    numeric(semantics["quantity"], 1200)
    choice(semantics["unit"], 1, "毫升")
    assert len(semantics["faces"]) == 2 and min(semantics["paint_colors"]) > 8

    edits = data["edits"]
    assert edits["quantity_values"] == [1200, 1202, 1208]
    assert edits["unit_ids"] == ["ml", "liter", "fluid-ounce"] and edits["commits"] == [2, 2]
    assert edits["rejected_quantity"] == "1201" and edits["error"] == "請輸入偶數"
    assert edits["alias"] == "fl oz"
    assert edits["unit_error"] == "找不到單位"
    numeric(edits["quantity_ax"], 1208)
    choice(edits["liter_ax"], 2, "公升")
    choice(edits["unit_ax"], 3, "液量盎司")

    mirror = data["mirror"]
    q0, u0 = mirror["ltr"]
    q1, u1 = mirror["rtl"]
    assert mirror["viewport"] == [960, 820]
    assert q0[0]+q0[2] < u0[0] and u1[0]+u1[2] < q1[0]
    first, second = mirror["gaps"]
    assert first > 0 and second > 0 and abs(first-second) <= .75
    assert mirror["tab_order"] == [
        f"{QUANTITY}-adjustable", f"{QUANTITY}-use-input", f"{UNIT}-adjustable", f"{UNIT}-use-input"
    ]

    disabled = data["disabled"]
    assert disabled["quantity_disabled_attributes"] == disabled["unit_disabled_attributes"] == ["true", "true"]
    assert disabled["enabled_tab_stops"] == [f"{UNIT}-adjustable", f"{QUANTITY}-adjustable"]
    assert disabled["quantity_when_unit_changes"] == 1200 and disabled["unit_when_quantity_changes"] == "liter"
    assert disabled["final_quantity"] == 1198 and disabled["commits"] == [1, 1]

    compact = data["compact"]
    assert compact["viewport"] == [320,820] and compact["rtl"] and compact["text_scale"] == 2
    assert compact["quantity"] == 1202 and compact["quantity_commits"] == 1
    assert compact["id"] == "liter" and compact["unit_commits"] == 1
    assert compact["raw"] == "公升" and compact["first_enter_commits"] == 0
    assert compact["preedit_survived_resize"] and compact["restored_editor_focus"]
    assert compact["hint_scroll"][1] > compact["hint_scroll"][0]
    assert min(compact["paint_colors"]) > 8
    qty, unit = compact["editors"]
    assert qty[1]+qty[3] <= unit[1]
    for rect in (qty, unit):
        assert min(rect[2:]) >= 48 and contains([0,0,320,820], rect)
    choice(compact["unit_ax"], 2, "公升")

    short = data["short"]
    assert short["viewport"] == [320,360] and short["commits"] == [0,0]
    assert short["back_focus"] == "scroll-pair-demo-show"
    assert short["focus_order"] == [f"{QUANTITY}-editor", f"{UNIT}-editor", f"{QUANTITY}-editor"]
    assert not contains(short["scroller"], short["clipped_unit"])
    assert all(contains(short["scroller"],r) for r in short["focused_editors"])
    start, qty, unit, back = short["scroll_offsets"]
    assert start <= qty < unit and back < unit


def contains(outer, inner):
    x, y, w, h = outer
    a, b, c, d = inner
    return x-.75 <= a and y-.75 <= b and a+c <= x+w+.75 and b+d <= y+h+.75


def spin(evidence, name, lo, hi, now):
    dom = evidence["dom"]
    assert dom["role"] == "spinbutton" and dom["name"] == name and dom["inside_group"]
    assert [dom["min"], dom["max"], dom["now"]] == [lo, hi, now]
    return dom["text"], evidence["chrome_value"], evidence["chrome_description"]


def numeric(evidence, now):
    text, value, help = spin(evidence, "數量", -1800, 1800, now)
    assert text.replace(",", "") == str(now) and value == now
    assert "每次增加或減少 2" in help and all(unit not in help for unit in ("毫升", "公升", "液量盎司"))


def choice(evidence, now, text):
    actual, value, help = spin(evidence, "單位", 1, 3, now)
    assert actual == text and "可輸入單位名稱" in help
    assert value == text or (value == now and text in help)
