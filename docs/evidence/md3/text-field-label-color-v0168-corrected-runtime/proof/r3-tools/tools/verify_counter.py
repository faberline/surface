"""Verify the Character Counter receipt against the original Canvas images."""

import json

from PIL import Image

from verify_forms_pixels import checksum, require
from verify_filled_field import blend
from verify_outlined_field import Frame as OutlinedFrame, bottom, bounds, color, near, right

PREFIX = "material-text-field-counter"
REPORT = f"{PREFIX}.json"
NORMAL = f"{PREFIX}-wide-outlined-normal.png"
OVER_LIMIT = f"{PREFIX}-wide-outlined-overflow.png"
MANUAL_ERROR = f"{PREFIX}-wide-filled-manual.png"
PREEDIT = f"{PREFIX}-ime-preedit.png"
COMPACT = f"{PREFIX}-compact-rtl-200.png"
NARROW_OUTLINED = f"{PREFIX}-fallback-outlined-112.png"
NARROW_FILLED = f"{PREFIX}-fallback-filled-112.png"
DIMENSIONS = {NORMAL: (1920, 1728), OVER_LIMIT: (1920, 1728), MANUAL_ERROR: (1920, 1728),
              PREEDIT: (960, 1728), COMPACT: (640, 2568), NARROW_OUTLINED: (640, 2568),
              NARROW_FILLED: (640, 2568)}
EDITOR = "text-field-counter-demo-editable"
INITIAL = "中a\u0301👍🏽"
AT_LIMIT = INITIAL + "🇹🇼👩‍💻"
LONGER = AT_LIMIT + "文字"
HELP = "請輸入 5 個字以內的標籤名稱。英文字母、中文或表情符號都可以使用。"
OVERFLOW = "請輸入最多 5 個字。"
MANUAL = "這個標籤已存在。"
ANNOUNCED = [OVERFLOW, MANUAL]


class Frame(OutlinedFrame):
    def __init__(self, path):
        with Image.open(path) as decoded:
            require(decoded.format == "PNG" and decoded.size == DIMENSIONS[path.name],
                    f"invalid Character Counter PNG size or type: {path.name}")
            decoded.verify()
        with Image.open(path) as decoded:
            decoded.load()
            self.image = decoded.convert("RGB")
        self.path = path


def inside(outer, inner):
    return (outer[0] <= inner[0] < right(inner) <= right(outer)
            and outer[1] <= inner[1] < bottom(inner) <= bottom(outer))


def check_model(data, width, text, edits, submits, *, appearance="outlined", last=None,
                actions=0, compact=False, narrow=False, manual=False):
    flags = {"visible": True, "dark": compact, "rtl": compact, "narrow": narrow, "show_error": manual}
    counters = {"limit": 5, "edits": edits, "actions": actions, "submits": submits, "locked_edit_requests": 0}
    require(all(data[key] is expected for key, expected in flags.items())
            and all(type(data[key]) is int and data[key] == expected for key, expected in counters.items())
            and data["width"] == width and data["value"] == text and data["last_submit"] == last
            and data["text_scale"] == (2 if compact else 1) and data["appearance"] == appearance
            and data["fixture_width"] == (112 if narrow else None), "unexpected caller-owned counter model")


def description(data, count, help_text=HELP, invalid=False):
    names = ["井字號", f"字數，已輸入 {count} 個字，上限 5 個字。", help_text]
    dom = data["dom"]
    require(data["ratio"] == f"{count}/5"
            and dom == {"names": names, "unique": True, "invalid": "true" if invalid else None, "passive": True}
            and dom["unique"] is True and dom["passive"] is True and data["ax_description"] == " ".join(names),
            "ordered passive AX count, limit or helper differs")


def paint(record, frame, *, overflow=False):
    model = record["model"]
    require(record["capture"] == frame.path.name and record["stacked"] is model["narrow"],
            "wrong capture or stack state")
    canvas = (frame.image.width // 2, frame.image.height // 2 - 44)
    face, editor, action, help_rect, count, semantic = (bounds(record[key], *canvas)
        for key in ("face", "editor", "action", "helper", "count", "count_description"))
    rtl = model["rtl"]
    filled = model["appearance"] == "filled"
    line = 16 * model["text_scale"]
    face_height = (96 if filled else 80) if rtl else 56
    help_start = right(face) - right(help_rect) if rtl else help_rect[0] - face[0]
    count_end = count[0] - face[0] if rtl else right(face) - right(count)
    require(near(face[3], face_height) and near(help_rect[1] - bottom(face), 4)
            and near(help_start, 16) and near(count_end, 16) and near(count[3], line)
            and count == semantic and action[2:] == [48, 48],
            "4 dp top gap, 16 dp outer insets or full action differs")
    if record["stacked"]:
        require(face[2] == 112 and editor == face and help_rect[2] == 80 and help_rect[3] >= 2 * line
                and near(count[1] - bottom(help_rect), 4) and near(action[1] - bottom(count), 4)
                and action[0] == face[0], "112 dp stack loses help, ratio or the relocated action")
    else:
        gap = help_rect[0] - right(count) if rtl else count[0] - right(help_rect)
        meet = right(action) == editor[0] if rtl else right(editor) == action[0]
        require(near(gap, 16) and near(count[1], help_rect[1]) and help_rect[2] >= 48
                and contains_separate_targets(face, editor, action) and meet,
                "parallel row needs a 16 dp gap and disjoint full editor and action")
    neutral, error = (color(model, role) for role in ("on_surface_variant", "error"))
    require(neutral != error, "normal and error tokens collapsed")
    invalid = model["show_error"] or overflow
    frame.ink(help_rect, error if invalid else neutral, record["help_ink"], 13)
    frame.ink(count, error if overflow else neutral, record["count_ink"], 13)
    frame.ink(count, neutral if overflow else error, record["wrong_count_ink"], 0)
    require(record["wrong_count_ink"] == 0, "wrong counter ink is present")
    stroke = color(model, "error" if invalid else "primary")
    x, y, _, height = face
    samples = ([(x + 6.5, bottom(face) - 0.5, stroke), (right(face) - 6.5, bottom(face) - 0.5, stroke),
                (x + 6.5, y + 10.5, color(model, "surface_container_highest"))]
               if filled else [(right(face) - 0.5, y + height / 2, stroke)])
    frame.trace(record["boundary"], samples)
    return {"help_height": help_rect[3], "count": count, "help_ink": record["help_ink"],
            "count_ink": record["count_ink"], "stacked": record["stacked"]}


def contains_separate_targets(face, editor, action):
    return (inside(face, editor) and inside(face, action) and editor[1::2] == face[1::2]
            and near(editor[2] + action[2], face[2]) and near(action[1] + 24, face[1] + face[3] / 2))


def verify_wide(authored, limits, frames):
    check_model(authored["initial"], 960, INITIAL, 0, 0)
    check_model(authored["limit_model"], 960, AT_LIMIT, 1, 0)
    require(authored["ordinary_live_messages"] == [], "ordinary count changes wrote to a live region")
    description(authored["description"], 3)
    description(authored["at_limit"], 5)
    normal = authored["paint"]
    over = limits["overflow_paint"]
    manual = limits["manual_paint"]
    require(normal["model"] == authored["initial"] and over["model"] == limits["overflow"],
            "the screenshot model does not match its editor state")
    check_model(limits["overflow"], 960, AT_LIMIT + "文", 2, 0)
    check_model(limits["longer"], 960, LONGER, 3, 0)
    check_model(limits["submitted"], 960, LONGER, 3, 1, last=LONGER)
    check_model(manual["model"], 960, AT_LIMIT, 5, 1, appearance="filled", last=LONGER, manual=True)
    for key, count, help_text in (("overflow_description", 6, OVERFLOW), ("longer_description", 7, OVERFLOW),
                                  ("precedence", 7, MANUAL), ("manual_description", 5, MANUAL)):
        description(limits[key], count, help_text, invalid=True)
    description(limits["restored_help"], 5)
    require(limits["live"] == {"overflow": [OVERFLOW], "continued": [OVERFLOW], "manual": ANNOUNCED,
                              "at_limit": ANNOUNCED, "final": ANNOUNCED}, "live validation repeated or changed order")
    end = len(AT_LIMIT.encode("utf-8"))
    selection = {"value": AT_LIMIT, "display_value": AT_LIMIT, "selection_anchor": end, "selection_cursor": end,
                 "composing": False, "preedit_range": None}
    require(limits["presentations"] == {"editor": EDITOR, "before": selection, "after": selection},
            "appearance change altered the retained editor or byte selection")
    return {NORMAL: paint(normal, frames[NORMAL]), OVER_LIMIT: paint(over, frames[OVER_LIMIT], overflow=True),
            MANUAL_ERROR: paint(manual, frames[MANUAL_ERROR])}


def verify_composition(data, frame):
    require(data["method"] == "CDP synthetic composition and trusted keyboard/pointer"
            and type(data["cancel_needed_cdp"]) is bool and type(data["commit_needed_insert_text"]) is bool,
            "wrong disclosed synthetic composition transport")
    enter_events = data["trusted_keys"]
    require(enter_events == [[key, True, "surface-text-input"] for key in ("Escape", "Enter", "Enter")]
            and all(event[1] is True for event in enter_events), "trusted separate IME Escape/Enter events missing")
    before = bounds(data["editor_before"], 960, 820)
    after = bounds(data["editor_after"], 480, 820)
    value_box = bounds(data["value_box"], 480, 820)
    require(after[2] < before[2] and after == data["paint"]["editor"] and inside(after, value_box),
            "preedit resize does not match the real value box")
    preedit = data["preedit"]
    require(preedit == {"value": AT_LIMIT, "display_value": "ㄓㄨㄥ", "selection_anchor": 0,
                       "selection_cursor": len(AT_LIMIT.encode("utf-8")), "composing": True,
                       "preedit_range": {"start": 0, "end": len("ㄓㄨㄥ".encode("utf-8"))}}
            and preedit["composing"] is True, "preedit changed the committed value or byte selection")
    require(data["paint"]["model"] == data["pending"], "preedit image model differs")
    for key in ("pending", "cancelled"):
        check_model(data[key], 480, AT_LIMIT, 5, 1, appearance="filled", last=LONGER)
    description(data["pending_description"], 5)
    first_committed = not data["commit_needed_insert_text"]
    check_model(data["first_enter"], 480, "中文" if first_committed else AT_LIMIT, 6 if first_committed else 5, 1,
                appearance="filled", last=LONGER)
    check_model(data["committed"], 480, "中文", 6, 1, appearance="filled", last=LONGER)
    description(data["committed_description"], 2)
    check_model(data["second_enter"], 480, "中文", 6, 2, appearance="filled", last="中文")
    check_model(data["cleared"], 480, "", 6, 2, appearance="filled", last="中文", actions=1)
    require(data["pending_live"] == data["final_live"] == ANNOUNCED, "IME or Clear produced a counter announcement")
    frame.ink(value_box, color(data["pending"], "on_surface"), data["preedit_ink"], 13)
    return paint(data["paint"], frame)


def verify_compact(data, frames):
    require(data["back_focus"] == "text-field-counter-demo-show", "wrong return focus")
    results = {}
    for key, name, filled, narrow in (
        ("normal", COMPACT, False, False),
        ("fallback_outlined", NARROW_OUTLINED, False, True),
        ("fallback_filled", NARROW_FILLED, True, True),
    ):
        record = data[key]
        check_model(record["model"], 320, INITIAL, 0, 0, appearance="filled" if filled else "outlined",
                    compact=True, narrow=narrow, manual=filled)
        description(data[f"{key}_description"], 3, MANUAL if filled else HELP, invalid=filled)
        results[name] = paint(record, frames[name])
    require(results[COMPACT]["help_height"] > 32, "320 dp help does not wrap")

    locked = data["locks"]
    check_model(locked["model"], 320, INITIAL, 0, 0, compact=True)
    require(locked["readonly"] is True and locked["disabled"] is True
            and locked["read_ratio"] == "2/5" and locked["disabled_ratio"] == "6/5"
            and locked["read_description"] == "字數，已輸入 2 個字，上限 5 個字。 已儲存的標籤。",
            "read-only or disabled AX count differs")
    read_rect, disabled_rect = (bounds(locked[key], 320, 1240) for key in ("read_bounds", "disabled_bounds"))
    require(read_rect[3] == disabled_rect[3] == 32
            and bottom(data["normal"]["helper"]) < read_rect[1] and bottom(read_rect) < disabled_rect[1],
            "wrong locked counter locations")
    frame, model = frames[COMPACT], locked["model"]
    neutral, paper, error, on_surface = (color(model, role) for role in
        ("on_surface_variant", "background", "error", "on_surface"))
    frame.ink(read_rect, neutral, locked["read_ink"], 13)
    frame.ink(disabled_rect, error, locked["disabled_error_ink"], 0)
    layer = model["palette"]["on_surface_alpha_38"]
    require(layer == [*on_surface, 97], "disabled count alpha was not 38% OnSurface")
    expected = [list(blend(paper, layer, linear)) for linear in (False, True)]
    require(locked["muted_candidates"] == expected and locked["disabled_error_ink"] == 0,
            "muted ratio candidate colors or error ink differ")
    muted = sum(any(all(abs(channel - expected_channel) <= 2
                        for channel, expected_channel in zip(pixel, candidate, strict=True)) for candidate in expected)
                for pixel in frame.region(disabled_rect).getdata())
    require(type(locked["muted_ink"]) is int and muted == locked["muted_ink"] and muted > 12,
            "no verified muted disabled ratio ink")
    return results


def verify(artifacts, metadata):
    path = artifacts / REPORT
    data = json.loads(path.read_text())
    require(set(data) == {"browser", "adapter", "page_errors", "authored", "limits", "composition", "compact"}
            and data["browser"] == metadata["browser"] and data["adapter"] == metadata["adapter"]
            and data["page_errors"] == [], "failed or mismatched Character Counter report")
    require({name.name for name in artifacts.glob(f"{PREFIX}-*.png")} == set(DIMENSIONS),
            "expected exactly seven original Character Counter PNGs")
    frames = {name: Frame(artifacts / name) for name in DIMENSIONS}
    measurements = verify_wide(data["authored"], data["limits"], frames)
    measurements[PREEDIT] = verify_composition(data["composition"], frames[PREEDIT])
    measurements.update(verify_compact(data["compact"], frames))
    return {"report_sha256": checksum(path), "pngs": {name: {
                "sha256": checksum(frame.path), "size": list(frame.image.size), **measurements[name]}
                for name, frame in frames.items()}, "validation_messages": ANNOUNCED,
            "synthetic_composition": {name: data["composition"][name]
                for name in ("cancel_needed_cdp", "commit_needed_insert_text")}}
