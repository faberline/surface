"""Check the v0.1.65 multiline browser report against its original PNGs."""

import json
from math import floor, isfinite

from PIL import Image

from verify_forms_pixels import checksum, require
from verify_outlined_field import DPR, HEADER, Frame as OutlinedFrame, bottom, bounds, color, near, right

PREFIX = "material-multiline-text-field"
REPORT = f"browser-{PREFIX}.json"
WIDE = (960, 840)
COMPACT = (320, 900)
PROFILE_TAGS = ("wide-outlined", "wide-filled-preedit", "compact-outlined", "compact-filled", "page-down")
WHEEL_TAGS = ("wheel-before", "wheel-inner", "wheel-residual", "wheel-restored")
TOUCH_TAGS = ("touch-before", "touch-held", "touch-restored")
TAGS = PROFILE_TAGS + WHEEL_TAGS + TOUCH_TAGS
EDITOR = "surface-text-input"
INITIAL = "第一行"
FIXED = "第一行😀\n第二行 á\n第三行 ABC 中文文字\n第四行\n第五行\n第六行\n第七行\n第八行\n"
GROW_HELP = "請簡短記錄這次的變更。"
FIXED_HELP = "內容會在固定高度內捲動。"
LOCKED_HELP = "這段文字可以選取和複製。"
READONLY_VALUE = "已儲存的第一行\n已儲存的第二行\n"
DISABLED_VALUE = "停用的第一行\n停用的第二行"
REPORT_KEYS = {
    "browser", "adapter", "outlined", "filled_ime", "wheel", "compact",
    "page_keys", "touch", "locked_ax", "page_errors",
}


def filename(tag):
    return f"{PREFIX}-{tag}.png"


def dimensions(tag):
    return COMPACT if tag.startswith("compact-") else WIDE


def inside(outer, inner):
    return (outer[0] <= inner[0] < right(inner) <= right(outer)
            and outer[1] <= inner[1] < bottom(inner) <= bottom(outer))


class Frame(OutlinedFrame):
    def __init__(self, artifacts, tag):
        self.path = artifacts / filename(tag)
        self.canvas = dimensions(tag)
        w, h = self.canvas
        with Image.open(self.path) as decoded:
            require(decoded.format == "PNG" and decoded.size == (w * DPR, (h + HEADER) * DPR),
                    f"wrong Multiline PNG format or dimensions: {self.path.name}")
            decoded.verify()
        with Image.open(self.path) as decoded:
            decoded.load()
            self.image = decoded.convert("RGB")

    def sample(self, x, y):
        require(type(x) in (float, int) and type(y) in (float, int)
                and 0 <= x < self.canvas[0] and 0 <= y < self.canvas[1], "invalid Canvas sample")
        return self.image.getpixel((floor(x * DPR), floor((y + HEADER) * DPR)))


def delta_pixels(before, source, after, target):
    bounds(source, *before.canvas)
    bounds(target, *after.canvas)
    require(source[2:] == target[2:] and min(source[2:]) >= 8, "unequal compared text boxes")
    return sum(
        before.sample(source[0] + x + .5, source[1] + y + .5)
        != after.sample(target[0] + x + .5, target[1] + y + .5)
        for y in range(2, int(source[3]) - 2, 2)
        for x in range(2, int(source[2]) - 2, 2)
    )


def state(model, *, compact=False, appearance="outlined", grow=INITIAL, edits=0, submits=0):
    require(model["visible"] is True and model["dark"] is compact and model["rtl"] is compact
            and model["text_scale"] == (2 if compact else 1)
            and model["width"] == (COMPACT[0] if compact else WIDE[0])
            and model["appearance"] == appearance and model["locked_edit_requests"] == 0
            and model["fixed"] == {"edits": 0, "rows": 3, "value": FIXED}
            and model["grow"] == {"actions": 0, "edits": edits, "limit": 12, "value": grow}
            and model["submits"] == submits
            and model["last_submit"] == ({"fixed": FIXED, "grow": grow} if submits else None),
            "unexpected multiline caller state or separate submit count")


def profile(record, frame, *, tag, appearance="outlined", grow=INITIAL, edits=0, submits=0):
    compact = frame.canvas == COMPACT
    state(record["model"], compact=compact, appearance=appearance, grow=grow, edits=edits, submits=submits)
    require(record["screenshot"] == filename(tag) and record["canvas"] == [0, 0, *frame.canvas],
            "incorrect captured profile")
    fixed, body, face, outer, help_rect, gutter = (
        bounds(record[key], *frame.canvas)
        for key in ("fixed", "body", "face", "outer", "help", "below_body")
    )
    bounds(record["grow"], *frame.canvas)
    require(fixed == face and inside(outer, fixed) and inside(fixed, body) and inside(outer, help_rect)
            and near(outer[3], 480) and near(body[3], 144 if compact else 72)
            and near(body[0] - fixed[0], 16) and near(right(fixed) - right(body), 16)
            and near(help_rect[1] - bottom(face), 4)
            and near(help_rect[0], body[0]) and near(help_rect[2], body[2])
            and gutter == [body[0], bottom(body) + 1, body[2], 3], "fixed-row geometry or clip changed")
    ink = color(record["model"], "on_surface")
    paper = color(record["model"], "surface_container_highest" if appearance == "filled" else "background")
    point = [fixed[0] + 9, bottom(face) - 5]
    require(record["sample"] == point and tuple(record["surface"]) == paper == frame.sample(*point),
            "the captured fill does not match the appearance")
    require(frame.count(gutter, ink) == 0, "glyphs leaked below the inner viewport")
    actual = frame.ink(body, ink, record["glyphs"], 31)
    help_pixels = frame.count(help_rect, color(record["model"], "on_surface_variant"))
    require(help_pixels > 30, "the visible helper did not paint")
    return {"body": body, "help": help_rect, "surface": paper, "glyph_pixels": actual,
            "help_glyph_pixels": help_pixels}


def textbox(record, label, help_text):
    require(record["label"] == label and record["description"] == help_text
            and type(record["backend_dom_node"]) is int and record["backend_dom_node"] > 0
            and record["multiline"] is True, "multiline textbox name, description or DOM owner differs")


def verify_selection(record, value, anchor, cursor, units, *, direction=None, display=None, preedit=None):
    parts = record["web"]
    require(type(parts) is list and parts[:2] == units and len(parts) == 3
            and parts[2] in {"none", "forward", "backward"}
            and (direction is None or parts[2] == direction), "textarea selection offsets or direction differ")
    require(record["rust"] == {"value": value, "display_value": value if display is None else display,
                               "selection_anchor": anchor, "selection_cursor": cursor,
                               "composing": preedit is not None, "preedit_range": preedit},
            "Runtime selection, composition or committed text differs")


def verify_editing(outlined, filled, frames):
    lf = INITIAL + "\n"
    state(outlined["enter_model"], grow=lf, edits=1)
    state(outlined["submitted"], grow=lf, edits=1, submits=1)
    before, after = (bounds(outlined[key], *WIDE) for key in ("before", "after"))
    require(before[:3] == after[:3] and near(after[3] - before[3], 24)
            and after == outlined["outlined"]["grow"], "the empty terminal row did not add 24 dp")
    textbox(outlined["ax"], "多行備註", f"井字號 字數，已輸入 3 個字，上限 12 個字。 {GROW_HELP}")
    events = [event for event in outlined["trusted"] if event["id"] == EDITOR]
    require([event["key"] for event in events if event["key"] in {"Enter", "ArrowUp", "ArrowDown"}]
            == ["Enter", "ArrowUp", "ArrowDown"]
            and any(event["kind"] == "beforeinput" and event["input"] == "insertLineBreak" for event in events),
            "trusted Enter and visual line events are incomplete")
    verify_selection(outlined["up"], lf, 0, 0, [0, 0])
    verify_selection(outlined["down"], lf, len(lf.encode()), len(lf.encode()), [4, 4])
    paint = {
        "wide_outlined": profile(outlined["outlined"], frames["wide-outlined"],
                                 tag="wide-outlined", grow=lf, edits=1),
        "wide_filled_preedit": profile(filled["capture"], frames["wide-filled-preedit"],
                                       tag="wide-filled-preedit", appearance="filled", grow=lf, edits=1, submits=1),
    }
    textbox(filled["ax"], "多行備註", f"井字號 字數，已輸入 4 個字，上限 12 個字。 {GROW_HELP}")
    verify_selection(filled["before"], lf, 10, 6, [2, 4], direction="backward")
    verify_selection(filled["preedit"], lf, 10, 6, [2, 3], display="第一中文",
                     preedit={"start": 6, "end": 12})
    require(type(filled["used_cdp_cancel"]) is bool and filled["after_cancel"] == filled["before"],
            "composition cancellation did not restore the backward committed selection")
    gx, gy, gw, gh = filled["capture"]["grow"]
    ink = color(filled["capture"]["model"], "on_surface")
    marked = frames["wide-filled-preedit"].count([gx + 16, gy + 24, gw - 32, gh - 24], ink)
    require(marked > 12, "composed Grow text was not painted")
    return {"paint": paint, "new_row_dp": after[3] - before[3], "separate_submits": 1,
            "marked_grow_pixels": marked, "used_cdp_cancel": filled["used_cdp_cancel"]}


def translated(body, shifted, restored, offsets, expected):
    require(type(offsets) is list and len(offsets) >= 3 and all(type(v) in (float, int) and isfinite(v) for v in offsets),
            "invalid scroll offsets")
    require(body == restored and body[0] == shifted[0] and body[2:] == shifted[2:]
            and near(offsets[-2] - offsets[0], expected) and near(offsets[-1], offsets[0])
            and near(body[1] - shifted[1], expected) and body[3] == 72,
            "scroll residual or restoration differs")


def verify_wheel(record, frames):
    body, residual, restored = (bounds(record[key], *WIDE)
                                for key in ("body", "body_after_residual", "restored_body"))
    require(record["canvas"] == [0, 0, *WIDE] and record["captures"] == list(map(filename, WHEEL_TAGS))
            and record["dpr"] == DPR and record["cdp_deltas"] == [96, 320, -416],
            "incorrect calibrated wheel geometry or capture list")
    require(len(record["scroll_offsets"]) == 4
            and all(event["kind"] == "wheel" and event["id"] == "surface" for event in record["trusted"])
            and [event["dy"] for event in record["trusted"]] == [48, 160, -208]
            and near(record["scroll_offsets"][0], record["scroll_offsets"][1]), "trusted wheel deltas or inner-first scroll differ")
    translated(body, residual, restored, record["scroll_offsets"], 64)
    before, inner, after, end = (frames[tag] for tag in WHEEL_TAGS)
    changes = [delta_pixels(before, body, inner, body), delta_pixels(inner, body, after, residual),
               delta_pixels(before, body, end, restored)]
    x, y, width, _ = body
    overlap = delta_pixels(before, [x, y + 48, width, 24], inner, [x, y, width, 24])
    require(changes == record["changed"] and changes[0] > 15 and changes[1] > 15 and changes[2] <= 2
            and overlap == record["copied_overlap_changes"] and overlap <= 2,
            "wheel PNGs do not reproduce the translated inner rows and restored view")
    state(record["unchanged"])
    return {"logical_deltas": [48, 160, -208], "cdp_deltas": record["cdp_deltas"],
            "offsets": record["scroll_offsets"], "overlap_pixel_changes": overlap, "pixel_changes": changes}


def verify_paging(keys, frames, wheel_body):
    fourth = FIXED.index("第四行")
    byte = len(FIXED[:fourth].encode())
    unit = len(FIXED[:fourth].encode("utf-16-le")) // 2
    verify_selection(keys["start"], FIXED, 0, 0, [0, 0])
    verify_selection(keys["down"], FIXED, byte, byte, [unit, unit])
    verify_selection(keys["backward"], FIXED, byte, 0, [0, unit], direction="backward")
    verify_selection(keys["up"], FIXED, 0, 0, [0, 0])
    require([event["key"] for event in keys["trusted"]] == ["PageDown", "PageUp", "PageUp"]
            and all(event["kind"] == "keydown" and event["id"] == EDITOR for event in keys["trusted"]),
            "trusted page keys did not all reach the textarea")
    paint = profile(keys["paint"], frames["page-down"], tag="page-down")
    require(wheel_body == paint["body"], "paging changed fixed-row bounds")
    before, after = frames["wheel-before"], frames["page-down"]
    changes = delta_pixels(before, wheel_body, after, paint["body"])
    x, y, width, height = wheel_body
    # Exclude the leftmost caret. The remaining overlapping rows must
    # move up exactly one line to bring the start of row four into view.
    overlap = delta_pixels(before, [x + 4, y + 24, width - 4, height - 24],
                           after, [x + 4, y, width - 4, height - 24])
    require(changes > 15 and overlap <= 2, "PageDown did not reveal row four by one line of paint")
    return {"fourth_row_utf8_byte": byte, "fourth_row_utf16_unit": unit, "backward": keys["backward"]["web"],
            "body": paint["body"], "pixel_changes": changes, "overlap_pixel_changes": overlap}


def verify_touch(record, frames, wheel_body):
    owner = record["owner"]
    trace = record["trace"]
    ordered = ["pointerdown", "gotpointercapture", "pointermove", "lostpointercapture", "pointerup"]
    require(type(owner) is int and owner > 0 and type(trace) is list and all(
        event["id"] == owner and event["trusted"] is True and type(event["capture"]) is bool
        and type(event["y"]) in (int, float) and isfinite(event["y"]) for event in trace),
        "untrusted or foreign touch event")
    events = [event["kind"] for event in trace]
    require(events and events[0] == "pointerdown" and events[-1] == "pointerup"
            and set(events) == set(ordered)
            and all(events.count(kind) == 1 for kind in ordered if kind != "pointermove")
            and [events.index(kind) for kind in ordered] == sorted(events.index(kind) for kind in ordered)
            and all(event["capture"] is (index < events.index("lostpointercapture"))
                    for index, event in enumerate(trace))
            and any(event["kind"] == "pointermove" and near(trace[0]["y"] - event["y"], 200) for event in trace),
            "actual touch capture loss must precede the noncaptured release")
    body, held, restored = (bounds(record[key], *WIDE) for key in ("body", "held_body", "restored_body"))
    require(record["captures"] == list(map(filename, TOUCH_TAGS)) and body == wheel_body
            and len(record["outer"]) == 3, "incorrect touch capture list or fixed body")
    translated(body, held, restored, record["outer"], 56)
    before, down, end = (frames[tag] for tag in TOUCH_TAGS)
    changes = [delta_pixels(before, body, down, held), delta_pixels(before, body, end, restored)]
    require(changes == record["changed"] and changes[0] > 15 and changes[1] <= 2,
            "touch PNGs do not show movement and restoration")
    return {"id": owner, "events": events, "offsets": record["outer"], "pixel_changes": changes}


def verify_locked(data, compact):
    readonly, disabled = (data[key] for key in ("readonly_proxy", "disabled_proxy"))
    for node, label, state_name in ((readonly, "唯讀內容", "readonly"), (disabled, "停用內容", "disabled")):
        props = node["properties"]
        states = {prop["name"]: prop["value"].get("value") for prop in props}
        require(len(states) == len(props) and node["ignored"] is False and node["role"]["value"] == "textbox"
                and node["name"]["value"] == label and states.get("multiline") is True
                and states.get(state_name) is True and type(node["backendDOMNodeId"]) is int
                and node["backendDOMNodeId"] > 0, "Chrome proxy does not project the locked multiline state")
    require(readonly["description"]["value"] == LOCKED_HELP, "read-only proxy lost its description")
    textbox(data["active"], "唯讀內容", LOCKED_HELP)
    textbox(compact["fixed_ax"], "詳細說明", FIXED_HELP)
    require(data["active"]["backend_dom_node"] != readonly["backendDOMNodeId"]
            and data["skipped_disabled"] is True and data["readonly_value"] == READONLY_VALUE
            and compact["readonly_disabled"] == {"read_only_value": READONLY_VALUE, "disabled": True,
                                                   "disabled_value": DISABLED_VALUE},
            "locked editor focus or committed newlines differ")
    return {"readonly": True, "disabled": True, "active_textarea": True, "skipped_disabled": True}


def verify(artifacts, metadata):
    path = artifacts / REPORT
    report = json.loads(path.read_text())
    require(set(report) == REPORT_KEYS and report["browser"] == metadata["browser"]
            and report["adapter"] == metadata["adapter"] and report["page_errors"] == [],
            "wrong browser/adapter or failed multiline report")
    require({file.name for file in artifacts.glob(f"{PREFIX}-*.png")} == set(map(filename, TAGS)),
            "Multiline output must contain exactly the twelve original success images")
    frames = {tag: Frame(artifacts, tag) for tag in TAGS}
    editing = verify_editing(report["outlined"], report["filled_ime"], frames)
    compact = report["compact"]
    compact_outlined = profile(compact["outlined"], frames["compact-outlined"], tag="compact-outlined")
    compact_filled = profile(compact["filled"], frames["compact-filled"], tag="compact-filled", appearance="filled")
    action = bounds(compact["clear_action"], *COMPACT)
    grow = bounds(compact["grow_body"], *COMPACT)
    require(action[2:] == [48, 48] and right(action) <= grow[0]
            and inside(compact["filled"]["outer"], grow) and inside(compact["filled"]["outer"], action),
            "compact RTL edit and Clear targets overlap or leave the scroller")
    wheel = verify_wheel(report["wheel"], frames)
    keys = verify_paging(report["page_keys"], frames, report["wheel"]["body"])
    touch = verify_touch(report["touch"], frames, report["wheel"]["body"])
    locked = verify_locked(report["locked_ax"], compact)
    return {"report_sha256": checksum(path), "editing": editing, "wheel": wheel, "page_keys": keys,
            "touch": touch, "locked_ax": locked, "compact": {"outlined": compact_outlined,
            "filled": compact_filled, "clear_action": action}, "pngs": {
                frame.path.name: {"sha256": checksum(frame.path), "size": frame.image.size}
                for frame in frames.values()
            }}
