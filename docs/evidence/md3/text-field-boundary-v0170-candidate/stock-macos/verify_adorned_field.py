"""Decode original Canvas captures and check the Adorned Text Field browser receipt."""

import json

from PIL import Image

from verify_forms_pixels import checksum, require
from verify_outlined_field import Frame as OutlinedFrame, bottom, bounds, color, near, right

PREFIX = "material-text-field-adornments"
REPORT = f"{PREFIX}.json"
IDLE = f"{PREFIX}-outlined-idle.png"
OUTLINED = f"{PREFIX}-outlined-focused.png"
FILLED = f"{PREFIX}-filled-focused.png"
PREEDIT = f"{PREFIX}-ime-preedit.png"
COMPACT = f"{PREFIX}-compact-dark-rtl-200.png"
FALLBACK = f"{PREFIX}-fallback-112.png"
DIMENSIONS = {IDLE: (1920, 1728), OUTLINED: (1920, 1728), FILLED: (1920, 1728),
              PREEDIT: (960, 1728), COMPACT: (640, 2008), FALLBACK: (640, 2008)}
EDITOR = "text-field-adornments-demo-editable"
ACTION_NAME = "套用建議金額"
HELP = "請輸入不含單位的金額。"
INVALID = "請輸入大於零的金額。"
DEPTHS = (0.5, 1.5, 2.5, 3.5)


class Frame(OutlinedFrame):
    def __init__(self, path):
        with Image.open(path) as decoded:
            require(decoded.format == "PNG" and decoded.size == DIMENSIONS[path.name],
                    f"invalid Adorned Text Field PNG size or type: {path.name}")
            decoded.verify()
        with Image.open(path) as decoded:
            decoded.load()
            self.image = decoded.convert("RGB")
        self.path = path

    def side_outline(self, record, slots, width, ink, paper):
        x, y, w, h = slots["face"]
        require(record["width_dp"] == width, "wrong outline width")
        samples = [(x + w - depth, y + h / 2, ink if depth < width else paper) for depth in DEPTHS]
        if width == 3:
            label = slots["floating"]
            require(near(label[0] - x, 16) and label[1] < y < bottom(label), "incorrect outline caption")
            samples.extend((axis, y + 0.5, shade) for axis, shade in (
                (label[0] - 6, ink), (label[0] - 2, paper),
                (right(label) + 2, paper), (right(label) + 6, ink)))
        self.trace(record["samples"], samples)

    def indicator(self, record, face, ink, fill):
        require(record["width_dp"] == 3, "wrong bottom indicator width")
        self.trace(record["samples"], [
            (x, bottom(face) - depth, ink if depth < 3 else fill)
            for x in (face[0] + 6.5, right(face) - 6.5) for depth in DEPTHS
        ])


def inside(outer, inner):
    return (outer[0] <= inner[0] < right(inner) <= right(outer)
            and outer[1] <= inner[1] < bottom(inner) <= bottom(outer))


def logical(rect, rtl):
    return (-right(rect), -rect[0]) if rtl else (rect[0], right(rect))


def validate_inline(slots, width, height, rtl=False):
    face, editor, action, leading, trailing, prefix, suffix, value, floating = (
        bounds(slots[key], width, height) for key in
        ("face", "editor", "action", "leading", "trailing", "prefix", "suffix", "value", "floating"))
    start, end = logical(face, rtl)
    rest_start, rest_end = logical(editor, rtl)
    action_start, action_end = logical(action, rtl)
    require(near(face[3], 96 if rtl else 56) and editor[1::2] == face[1::2]
            and action[2:] == [48, 48] and inside(face, action)
            and near(rest_start, start) and near(rest_end, action_start) and near(action_end, end)
            and near(action[1] + 24, face[1] + face[3] / 2),
            "editor and trailing target must occupy separate parts of one full face")
    require(inside(editor, leading) and inside(action, trailing)
            and all(rect[2:] == [24, 24] and near(rect[1] + 12, face[1] + face[3] / 2)
                    for rect in (leading, trailing))
            and near(logical(leading, rtl)[0] - start, 12)
            and near(end - logical(trailing, rtl)[1], 12), "independent icon geometry differs")
    prefix_start, prefix_end = logical(prefix, rtl)
    value_start, value_end = logical(value, rtl)
    suffix_start, suffix_end = logical(suffix, rtl)
    require(value[2] >= 48 and all(inside(editor, part) for part in (prefix, value, suffix))
            and prefix[1::2] == value[1::2] == suffix[1::2]
            and near(prefix_start - start, 52) and near(end - suffix_end, 52)
            and near(value_start - prefix_end, 2) and near(suffix_start - value_end, 2),
            "affixes do not separate from the value lane")
    return face, floating


def description(data, invalid=False):
    names = ["歐元", "每月", INVALID if invalid else HELP]
    require(data["dom"] == {"names": names, "unique": True, "invalid": "true" if invalid else None}
            and data["dom"]["unique"] is True and data["ax_description"] == " ".join(names)
            and data["action_role"] == "button" and data["action_name"] == ACTION_NAME,
            "ordered AX description or separate named action differs")


def check_model(data, width, value, edits, actions, submits, last=None, compact=False, read_only_actions=0):
    flags = {"visible": True, "dark": compact, "rtl": compact, "show_error": compact, "narrow": False}
    counts = {"edits": edits, "actions": actions, "submits": submits, "disabled_action_requests": 0,
              "locked_edit_requests": 0, "read_only_actions": read_only_actions}
    require(all(data[key] is expected for key, expected in flags.items())
            and all(type(data[key]) is int and data[key] == expected for key, expected in counts.items())
            and data["width"] == width and data["value"] == value and data["last_submit"] == last
            and data["text_scale"] == (2 if compact else 1) and data["appearance"] == "filled"
            and data["fixture_width"] is None, "unexpected caller-owned field state")


def verify_wide(presentation, frames):
    idle, focus, filled = (presentation[key] for key in ("idle", "outlined", "filled"))
    for record, name in ((idle, IDLE), (focus, OUTLINED), (filled, FILLED)):
        require(record["capture"] == name, "wrong wide capture mapping")
        validate_inline(record["slots"], 960, 820)
    require(idle["slots"] == focus["slots"], "focus changes stable Outlined hit geometry")
    paper, neutral, primary, outline, fill = (color(presentation, key) for key in
        ("background", "on_surface_variant", "primary", "outline", "surface_container_highest"))
    require(len({paper, neutral, primary, outline, fill}) == 5, "distinct light color roles collapsed")
    frames[IDLE].side_outline(idle["outline"], idle["slots"], 1, outline, paper)
    frames[OUTLINED].side_outline(focus["outline"], focus["slots"], 3, primary, paper)
    for part, key, minimum in (("leading", "leading_ink", 20), ("trailing", "trailing_ink", 20),
                               ("prefix", "prefix_region_ink", 0), ("suffix", "suffix_ink", 0)):
        frames[IDLE].ink(idle["slots"][part], neutral, idle[key], minimum)
    require(idle["suffix_ink"] == 0 and idle["prefix_region_ink"] != focus["prefix_ink"],
            "idle suffix is visible or focus fails to swap its prefix region")
    for state, name in ((focus, OUTLINED), (filled, FILLED)):
        for affix in ("prefix", "suffix"):
            frames[name].ink(state["slots"][affix], neutral, state[f"{affix}_ink"], 13)
    face, label = (filled["slots"][key] for key in ("face", "floating"))
    require(near(label[0] - face[0], 52) and near(label[1] - face[1], 8)
            and near(filled["slots"]["value"][1] - face[1], 24), "wrong Filled vertical slots")
    frames[FILLED].indicator(filled["indicator"], face, primary, fill)
    frames[FILLED].trace([filled["fill"]], [(face[0] + 6.5, face[1] + 10.5, fill)])
    description(focus["description"])
    selection = {"value": "中文42", "display_value": "中文42", "selection_anchor": 0,
                 "selection_cursor": 8, "composing": False, "preedit_range": None}
    require(presentation["retained_editor"] == {"id": EDITOR, "before": selection, "after": selection},
            "the editor or selected UTF-8 byte range changed during appearance change")


def verify_actions(actions):
    trusted = actions["trusted_events"]
    require(trusted["keys"] == [["Enter", True], ["Enter", True], [" ", True]]
            and trusted["pointer"] == [["pointerdown", True], ["pointerup", True]]
            and all(event[1] is True for group in trusted.values() for event in group),
            "missing trusted keyboard or pointer sequence")
    for key, value, count in (("editor_submit", "中文42", 0), ("action_enter", "24", 1),
                              ("action_space", "24", 2), ("held", "24", 2), ("released", "24", 3)):
        check_model(actions[key], 960, value, 1, count, 1, "中文42")


def verify_composition(data, frame):
    require(data["capture"] == PREEDIT and data["method"] == "CDP synthetic composition with trusted click, Escape and Enter"
            and type(data["cancel_needed_cdp"]) is bool and type(data["commit_needed_insert_text"]) is bool,
            "invalid disclosed composition transport")
    before = bounds(data["editor_before"], 960, 820)
    after = bounds(data["editor_after"], 480, 820)
    value, suffix = (bounds(data[key], 480, 820) for key in ("value_box", "suffix_box"))
    require(after[2] < before[2] and inside(after, value) and inside(after, suffix)
            and near(suffix[0] - right(value), 2), "preedit resize lost separate value and affix lanes")
    require(data["preedit"] == {"value": "24", "display_value": "ㄓㄨㄥ", "selection_anchor": 0,
                              "selection_cursor": 2, "composing": True, "preedit_range": {"start": 0, "end": 9}}
            and data["preedit"]["composing"] is True, "wrong live preedit text or range")
    for key in ("protected", "cancelled"):
        check_model(data[key], 480, "24", 1, 3, 1, "中文42")
    first_native_commit = not data["commit_needed_insert_text"]
    check_model(data["first_enter"], 480, "中文" if first_native_commit else "24",
                2 if first_native_commit else 1, 3, 1, "中文42")
    check_model(data["committed"], 480, "中文", 2, 3, 1, "中文42")
    check_model(data["second_enter"], 480, "中文", 2, 3, 2, "中文")
    frame.ink(value, color(data["protected"], "on_surface"), data["preedit_ink"], 13)
    frame.ink(suffix, color(data["protected"], "on_surface_variant"), data["suffix_ink"], 13)


def verify_compact(compact, frames):
    inline, narrow, locked = (compact[key] for key in ("compact", "fallback", "locked"))
    require(inline["capture"] == COMPACT and narrow["capture"] == FALLBACK
            and compact["back_focus"] == "text-field-adornments-demo-show", "wrong compact captures or return focus")
    check_model(inline["model"], 320, "", 0, 0, 0, compact=True)
    check_model(locked["model"], 320, "", 0, 0, 0, compact=True, read_only_actions=1)
    require(locked["readonly"] is True and locked["disabled"] is True
            and locked["readonly_action"] == "text-field-adornments-demo-read-only-trailing-action"
            and bounds(locked["disabled_action"], 320, 960)[2:] == [48, 48], "read-only or disabled proof differs")
    slots = inline["slots"]
    face, label = validate_inline(slots, 320, 960, rtl=True)
    require(inside(slots["editor"], label) and near(right(face) - right(label), 52),
            "the enlarged caption leaves its logical start")
    model = inline["model"]
    error, neutral, fill = (color(model, key) for key in ("error", "on_surface_variant", "surface_container_highest"))
    require(len({error, neutral, fill}) == 3, "distinct dark color roles collapsed")
    frames[COMPACT].indicator(inline["indicator"], face, error, fill)
    frames[COMPACT].ink(slots["trailing"], error, inline["trailing_error_ink"], 21)
    for name, pixels in zip(("prefix", "suffix"), inline["affix_ink"], strict=True):
        frames[COMPACT].ink(slots[name], neutral, pixels, 13)
    description(inline["description"], invalid=True)

    slots = narrow["slots"]
    face, action, value, trailing = (bounds(slots[key], 320, 960)
        for key in ("face", "action", "value", "trailing"))
    label, prefix, suffix = (bounds(narrow[key], 320, 960) for key in ("above_label", "prefix", "suffix"))
    require(face == slots["editor"] and face[2:] == [112, 96] and inside(face, value)
            and value[2] >= 48 and action[2:] == [48, 48] and action[0] == face[0]
            and trailing[2:] == [24, 24] and inside(action, trailing)
            and bottom(label) <= face[1] < bottom(face) < action[1] < bottom(action) < prefix[1]
            and bottom(prefix) < suffix[1], "112 dp fallback lost the full editor or logical reading order")
    lead = slots["leading"]
    require(all(slots[key] == lead for key in ("prefix", "suffix", "floating"))
            and lead == [face[0], label[1], 0, 0], "removed inline slots still have geometry")
    gutter = [lead[0], lead[1], min(24, label[0] - lead[0]), 24]
    require(gutter[2] >= 8 and right(gutter) <= label[0] and narrow["omitted_icon"]["gutter"] == gutter,
            "incorrect leading icon gutter")
    frames[FALLBACK].indicator(narrow["indicator"], face, error, fill)
    frames[FALLBACK].ink(gutter, neutral, narrow["omitted_icon"]["ink"], 0)
    require(narrow["omitted_icon"]["ink"] == 0, "ghost leading icon ink remains")
    for rect, pixels in zip((prefix, suffix), narrow["wrapped_ink"], strict=True):
        frames[FALLBACK].ink(rect, neutral, pixels, 13)
    description(narrow["description"], invalid=True)


def verify(artifacts, metadata):
    report_path = artifacts / REPORT
    report = json.loads(report_path.read_text())
    require(set(report) == {"browser", "adapter", "page_errors", "presentations", "actions", "composition", "compact"}
            and report["page_errors"] == [] and report["browser"] == metadata["browser"]
            and report["adapter"] == metadata["adapter"], "failed or mismatched Adorned Text Field report")
    require({path.name for path in artifacts.glob(f"{PREFIX}-*.png")} == set(DIMENSIONS),
            "expected exactly six original Adorned Text Field PNGs")
    frames = {name: Frame(artifacts / name) for name in DIMENSIONS}
    verify_wide(report["presentations"], frames)
    verify_actions(report["actions"])
    verify_composition(report["composition"], frames[PREEDIT])
    verify_compact(report["compact"], frames)
    return {"report_sha256": checksum(report_path), "pngs": {
                name: {"sha256": checksum(frame.path), "size": list(frame.image.size)}
                for name, frame in frames.items()},
            "description_cases": 3, "ghost_icon_pixels": 0, "trusted_action_events": 5,
            "synthetic_composition": {"cancel_needed_cdp": report["composition"]["cancel_needed_cdp"],
                                      "commit_needed_insert_text": report["composition"]["commit_needed_insert_text"]}}
