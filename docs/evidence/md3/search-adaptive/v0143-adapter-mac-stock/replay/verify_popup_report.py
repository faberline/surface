OPENER = "scroll-popup-demo-opener"
CLOSE = "scroll-popup-demo-close"
HIDE = "scroll-popup-demo-hide-opener"
QUANTITY = "scroll-popup-demo-quantity-adjustable"


def validate(data):
    assert not data["errors"], data["errors"]
    opening = data["opening"]
    assert opening["side"] == "below" and opening["focus"] == QUANTITY
    assert opening["expanded"] == [False, True] and abs(opening["gap"]-4) <= .75
    assert abs(opening["dialog"][0]-opening["anchor"][0]) <= .75
    assert abs(opening["dialog"][1]-bottom(opening["anchor"])-4) <= .75
    ax = opening["ax"]
    dom = ax["dom"]
    assert dom["role"] == "dialog" and dom["name"] == "編輯容量" and dom["modal"] != "true"
    assert dom["options"] == ["數量", "單位"] and dom["outside"] == "外部 0 次" and dom["close"] == "關閉"
    assert ax["chrome_modal"] in (None, False) and ax["quantity"] == 1200 and "毫升" in ax["unit_description"]

    modeless = data["modeless"]
    assert modeless["pointer_open_focus"] == OPENER and modeless["outside_focus"] == "scroll-popup-demo-outside"
    assert modeless["tab_focus"] in (QUANTITY,CLOSE) and modeless["outside_presses"] == 1
    assert modeless["outside_label"] == "外部 1 次"
    assert modeless["outside_escape_leaves_open"] and modeless["close_focus"] == OPENER

    edits = data["edits"]
    assert edits["quantity"] == 1202 and edits["unit"] == "fluid-ounce" and edits["alias"] == "Fl Oz"
    assert edits["commits"] == [1,1] and edits["rejected"] == ["1203","無"]
    for receipt, text in zip(edits["errors"], ("請輸入偶數","找不到單位")):
        assert receipt["dom"] == {"invalid":"true","linked":True,"text":text}
        assert text in receipt["description"]
    assert edits["discarded_both"] and edits["restored_focus"] == OPENER

    escape = data["escape"]
    assert escape["preedit_escape_preserved_popup"] and escape["draft_escape_preserved_popup"]
    assert escape["discarded"] == "1206" and escape["escapes_from"] == [QUANTITY,CLOSE]
    assert escape["restored_focus"] == OPENER and escape["commits"] == [0,0]

    anchor = data["anchor"]
    for placement in [anchor["ltr"],anchor["rtl"]]:
        assert abs(placement["gap"]-4) <= .75 and abs(placement["anchor"][1]-bottom(placement["popup"])-4) <= .75
    assert abs(anchor["ltr"]["popup"][0]-anchor["ltr"]["anchor"][0]) <= .75
    assert abs(right(anchor["rtl"]["popup"])-right(anchor["rtl"]["anchor"])) <= .75
    assert anchor["closed_without_anchor"] and anchor["retained_outside_focus"] == HIDE
    assert anchor["reopened_clean"] and anchor["back_focus"] == "scroll-popup-demo-show"

    compact = data["compact"]
    assert compact["viewport"] == [320,400] and compact["rtl"] and compact["text_scale"] == 2
    dialog, body, close = [compact[key] for key in ("dialog","body","close")]
    assert contains([0,0,320,400],dialog) and contains(dialog,body) and contains(dialog,close)
    assert min(close[2:]) >= 48 and abs(compact["anchor"][1]-bottom(dialog)-4) <= .75
    outside = compact["outside"]
    assert min(outside[2:]) >= 48 and contains([0,0,320,400],outside)
    assert outside[1] >= bottom(dialog) and compact["outside_label"] == "外部 1 次"
    assert len(compact["editors"]) == len(compact["rings"]) == 3
    for editor, ring in zip(compact["editors"],compact["rings"]):
        assert min(editor[2:]) >= 48 and contains(body,ring) and contains(ring,editor)
    top, bottom_ink = compact["unit_ring_ink"]
    assert top == bottom_ink and max(bottom_ink) > 0
    start, down, up = compact["scroll_offsets"]
    assert down > start and up < down
    assert compact["covered_setting"] in ("scroll-popup-demo-theme","scroll-popup-demo-large-text")
    assert contains(dialog,[*compact["shielded_point"],0,0])
    assert compact["fallback_escape_discards_draft"] and compact["close_focus"] == OPENER
    assert compact["shielded_outside_presses"] == 0 and compact["outside_presses"] == 1 and compact["commits"] == [0,0]


def right(rect):
    return rect[0]+rect[2]


def bottom(rect):
    return rect[1]+rect[3]


def contains(outer, inner):
    return (outer[0]-.75 <= inner[0] and outer[1]-.75 <= inner[1]
            and right(inner) <= right(outer)+.75 and bottom(inner) <= bottom(outer)+.75)
