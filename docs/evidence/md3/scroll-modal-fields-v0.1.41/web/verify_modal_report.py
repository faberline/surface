"""Check the exact receipt from the named modal Scroll Field browser case."""

from pathlib import Path
from struct import unpack

OPENER = "scroll-modal-demo-opener"
OUTSIDE = "scroll-modal-demo-outside"
QUANTITY = "scroll-modal-demo-quantity"
UNIT = "scroll-modal-demo-unit"
CANCEL = "scroll-modal-demo-cancel"
SAVE = "scroll-modal-demo-save"
COMPACT_CAPTURES = {
    "compact-dark-rtl": 400,
    "compact-quantity-focus": 400,
    "shortest-editor": 218,
}


def right(rect):
    return rect[0] + rect[2]


def bottom(rect):
    return rect[1] + rect[3]


def covers(outer, inner):
    return (
        outer[0] - 0.75 <= inner[0]
        and outer[1] - 0.75 <= inner[1]
        and right(inner) <= right(outer) + 0.75
        and bottom(inner) <= bottom(outer) + 0.75
    )


def overlap(first, second):
    return (
        first[0] < right(second)
        and second[0] < right(first)
        and first[1] < bottom(second)
        and second[1] < bottom(first)
    )


def verify_compact_png(path: Path, viewport_height=400):
    with path.open("rb") as screenshot:
        header = screenshot.read(24)
    assert header[:8] == b"\x89PNG\r\n\x1a\n" and header[12:16] == b"IHDR", path
    dimensions = unpack(">II", header[16:24])
    assert dimensions == (640, (viewport_height + 44) * 2), (path, dimensions)
    return dimensions


def verify_compact_pngs(root: Path):
    return {
        name: verify_compact_png(root / f"material-scroll-modal-fields-{name}.png", height)
        for name, height in COMPACT_CAPTURES.items()
    }


def verify_ring_ink(pixels):
    assert pixels["top"] == pixels["bottom"] and pixels["top"] != pixels["gap"]
    assert len(pixels["top"]) == 3 and max(pixels["top"]) > 0


def validate(report):
    assert not report["errors"], report["errors"]
    assert report["browser"].startswith("Chrome/"), report["browser"]
    opening = report["opening"]
    assert opening["scope"] == {"name": "編輯容量", "aria_modal": True, "count": 1}
    assert opening["quantity"] == 1200
    assert opening["unit"] == "毫升" or opening["unit"] == 1 and "毫升" in opening["unit_help"]
    assert opening["keyboard_initial_focus"] == opening["pointer_initial_focus"] == QUANTITY + "-adjustable"
    assert opening["pointer_tab_focus"] == QUANTITY + "-use-input"
    forward = [
        QUANTITY + "-use-input", UNIT + "-adjustable", UNIT + "-use-input",
        CANCEL, SAVE, QUANTITY + "-adjustable",
    ]
    assert opening["forward"] == forward
    assert opening["reverse"] == list(reversed(forward[:-1])) + [forward[-1]]
    assert covers(opening["layer"], opening["panel"])

    save = report["save"]
    assert save["locally_confirmed"] == [1204, "liter"]
    assert save["unsubmitted"] == [1206, "fluid-ounce"]
    assert save["saves"] == [1, 1] and save["early_saves"] == 0
    assert save["open_seed_is_clean"] and save["restored_focus"] == OPENER

    invalid = report["invalid"]
    assert invalid["before"] == [1200, "ml"]
    assert invalid["after"] == [1206, "fluid-ounce"] and invalid["saves"] == 1
    assert invalid["failed_saves"] == 2 and invalid["valid_peer_raw"] == "１２０６"
    assert invalid["first_invalid"] == ["quantity", "unit"]
    assert invalid["statuses"] == ["請修正數量和單位", "請修正單位"]
    assert len(invalid["errors"]) == 2
    for error, text in zip(invalid["errors"], ("請輸入偶數", "找不到單位")):
        assert error["dom"] == {"invalid": "true", "linked": True, "text": text}
        assert text in error["chrome_description"]
    wide_unit = invalid["wide_unit_reveal"]
    assert covers(wide_unit["body"], wide_unit["ring"])
    assert covers(wide_unit["ring"], wide_unit["editor"])
    assert covers(wide_unit["body"], wide_unit["error"])
    assert min(wide_unit["editor"][2:]) >= 48 and wide_unit["error"][3] > 0
    verify_ring_ink(wide_unit["ring_ink"])

    dismissals = report["dismissals"]
    assert dismissals["routes"] == ["cancel", "escape", "scrim"]
    probe = [*dismissals["scrim_point"], 0, 0]
    assert covers(dismissals["outside_target"], probe) and not covers(opening["panel"], probe)
    assert dismissals["outside_presses_during_dismissal"] == 0
    assert dismissals["outside_presses_after_close"] == 1
    assert dismissals["accepted"] == [1200, "ml"] and dismissals["saves"] == 0
    assert dismissals["restored_focus"] == OPENER

    ime = report["composition"]
    assert ime["blocked_pointer"] == ime["blocked_projected"] == ["save", "cancel"]
    assert ime["first_enter_saves"] == ime["field_enter_saves"] == 0
    assert ime["projected_save_saves"] == 1 and ime["accepted"] == [1200, "liter"]
    assert ime["escape_order"] == ["preedit", "editor", "modal"]

    compact = report["compact"]
    assert compact["viewport"] == [320, 400] and compact["rtl"] and compact["text_scale"] == 2
    panel, body, footer = [compact[key] for key in ("panel", "body", "footer")]
    assert covers([0, 0, 320, 400], panel) and covers(panel, body) and covers(panel, footer)
    cancel, save_button = compact["actions"]
    assert cancel[0] > save_button[0] and abs(cancel[1] - save_button[1]) <= 0.75
    assert not overlap(cancel, save_button)
    assert all(min(rect[2:]) >= 48 and covers(footer, rect) and not overlap(body, rect)
               for rect in compact["actions"])
    assert len(compact["editors"]) == 2 and len(compact["rings"]) == 3
    assert all(covers(body, ring) for ring in compact["rings"])
    assert all(covers(ring, editor) and min(editor[2:]) >= 48
               for ring, editor in zip(compact["rings"], compact["editors"]))
    assert len(compact["ring_ink"]) == 2
    for pixels in compact["ring_ink"]:
        verify_ring_ink(pixels)
    initial, forward_scroll, reverse_scroll = compact["body_scroll_top"]
    assert forward_scroll > initial and reverse_scroll < forward_scroll
    assert compact["failed_save_kept_footer"]
    assert compact["invalid_summary"] == "請修正數量和單位" and compact["invalid_focus"] == SAVE

    short = compact["short_view"]
    assert (short["hidden_height"], short["restored_height"]) == (217, 218)
    short_body, short_editor, short_ring = [short[key] for key in ("body", "editor", "ring")]
    assert min(short_editor[2:]) >= 48 and min(short_body[2:]) >= 48
    assert covers([0, 0, 320, 218], short["panel"]) and covers(short["panel"], short_body)
    assert covers(short_body, short_ring) and covers(short_ring, short_editor)
    verify_ring_ink(short["ring_ink"])
    assert len(short["actions"]) == 2 and not overlap(*short["actions"])
    assert all(min(action[2:]) >= 48 and covers(short["panel"], action) and not overlap(short_body, action)
               for action in short["actions"])
    assert short["error"][1] >= bottom(short_body) - 0.75 and short["error"][3] > 0
    label = short["error_description"]
    assert label["dom"]["invalid"] == "true"
    assert "請輸入偶數" in label["dom"]["described_by"]
    assert "請輸入偶數" in label["chrome_description"]
    assert short["retained_raw"] == ["1203", "無"]
    assert compact["saves"] == 0 and compact["accepted"] == [1200, "ml"]
    assert compact["restored_focus"] == OPENER and compact["back_focus"] == "scroll-modal-demo-show"
