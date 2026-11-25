"""Recheck the Filled Text Field report against its original Chrome Canvas PNGs."""

import json
from math import floor

from PIL import Image

from verify_forms_pixels import checksum, require
from verify_outlined_field import (HELP, INVALID, Frame as OutlinedFrame,
                                   bottom, bounds, color, description, model_state, near, right)

PREFIX = "material-filled-text-field"
REPORT = f"{PREFIX}.json"
IDLE = f"{PREFIX}-idle.png"
HOVER = f"{PREFIX}-hover.png"
FOCUSED = f"{PREFIX}-focused.png"
PREEDIT = f"{PREFIX}-ime-preedit.png"
ERROR = f"{PREFIX}-error.png"
COMPACT = f"{PREFIX}-compact-dark-rtl-200.png"
DIMENSIONS = {IDLE: (1920, 1728), HOVER: (1920, 1728), FOCUSED: (1920, 1728),
              PREEDIT: (960, 1728), ERROR: (1920, 1728), COMPACT: (640, 2008)}


class Frame(OutlinedFrame):
    def __init__(self, path):
        with Image.open(path) as image:
            require(image.format == "PNG" and image.size == DIMENSIONS[path.name],
                    f"invalid Filled Text Field PNG size or type: {path.name}")
            image.verify()
        with Image.open(path) as image:
            image.load()
            self.image = image.convert("RGB")
        self.path = path

    def swatch(self, record, x, y, expected):
        self.trace([record], [(x, y, expected)])

    def bottom_indicator(self, record, face, width, ink, fill):
        require(record["width_dp"] == width, "incorrect Filled Text Field bottom indicator width")
        self.trace(record["samples"], [
            (face[0] + fraction * face[2], bottom(face) - depth, ink if depth < width else fill)
            for fraction in (0.62, 0.82) for depth in (0.5, 1.5, 2.5, 3.5, 4.5)
        ])

    def overlay(self, record, x, y, base, layer):
        base, layer = tuple(base), tuple(layer)
        require(tuple(record["base"]) == base and tuple(record["overlay"]) == layer,
                "Filled Text Field blend base or token differs")
        predictions = {space: blend(base, layer, space == "linear") for space in ("encoded", "linear")}
        require(all(tuple(record[f"{space}_mix"]) == prediction
                    for space, prediction in predictions.items()), "incorrect reported blend computation")
        rgb = tuple(record["rgb"])
        matches = {space for space, prediction in predictions.items()
                   if all(abs(a - b) <= 2 for a, b in zip(rgb, prediction, strict=True))}
        require(matches and rgb != base, "Filled Text Field overlay pixel differs from the Material token")
        self.swatch(record, x, y, rgb)
        return rgb, matches


def blend(base, layer, linear):
    def decode(channel):
        value = channel / 255
        if not linear:
            return value
        return value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4

    def encode(value):
        if linear:
            value = 12.92 * value if value <= 0.0031308 else 1.055 * value ** (1 / 2.4) - 0.055
        return floor(255 * value + 0.5)

    alpha = layer[3] / 255
    return tuple(encode(decode(background) * (1 - alpha) + decode(ink) * alpha)
                 for background, ink in zip(base, layer[:3], strict=True))


def entry(face, top, width, rtl=False):
    x = right(face) - 16 - width if rtl else face[0] + 16
    return [x, top, width, bottom(face) - top - 5]


def inside(parent, child):
    return parent[0] <= child[0] < right(child) <= right(parent) and parent[1] <= child[1] < bottom(child) <= bottom(parent)


def paint_report(report, frames):
    idle, hover, focused = (frames[name] for name in (IDLE, HOVER, FOCUSED))
    face = bounds(report["face"], 960, 820)
    require(near(face[3], 56) and all(report[key] == face
            for key in ("container", "hover_layer", "indicator_bounds", "focused_face")),
            "the wide editor, overlays and hit rectangle differ")
    helper = bounds(report["description_bounds"], 960, 820)
    require(near(helper[1] - bottom(face), 4), "the supporting text is out of position")
    paper, fill, neutral, text, primary = (color(report, key) for key in
        ("background", "surface_container_highest", "on_surface_variant", "on_surface", "primary"))
    require(len({paper, fill, neutral, text, primary}) == 5, "light palette roles collapsed")
    layers = {alpha: (*text, stored) for alpha, stored in ((4, 10), (8, 20), (38, 97))}
    require(all(tuple(report["palette"][f"on_surface_alpha_{alpha}"]) == token
                for alpha, token in layers.items()), "incorrect Filled Field alpha tokens")
    x, y, w, _ = face
    middle_x = x + 0.75 * w
    left_corner = (x + 0.25, y + 0.25, paper)
    right_corner = (right(face) - 0.25, y + 0.25, paper)
    idle.trace(report["corners"], [left_corner, right_corner,
               (x + 4.5, y + 0.5, fill), (right(face) - 4.5, y + 0.5, fill),
               (x + 0.5, bottom(face) - 1.5, fill), (right(face) - 0.5, bottom(face) - 1.5, fill)])
    idle.swatch(report["idle"]["fill"], middle_x, y + 12.5, fill)
    idle.bottom_indicator(report["idle"]["indicator"], face, 1, neutral, fill)
    idle.ink([x + 16, y + 16, w - 32, 24], neutral, report["idle"]["label_pixels"], 13)
    idle.ink(helper, neutral, report["description_pixels"], 13)
    hover_fill, hover_space = hover.overlay(report["hover"]["fill"], middle_x, y + 12.5, fill, layers[8])
    hover.trace(report["hover"]["corners"], [left_corner, right_corner])
    hover.bottom_indicator(report["hover"]["indicator"], face, 1, text, hover_fill)
    focused.swatch(report["focused"]["fill"], middle_x, y + 12.5, fill)
    focused.bottom_indicator(report["focused"]["indicator"], face, 3, primary, fill)
    label = bounds(report["float_bounds"], 960, 820)
    require(inside(face, label) and [label[0], label[1], label[3]] == [x + 16, y + 8, 16],
            "the floating caption has wrong in-field geometry")
    focused.ink(label, primary, report["focused"]["float_pixels"], 13)
    body = entry(face, bottom(label), 100)
    require(report["focused"]["body_bounds"] == body, "wrong lower placeholder sample rectangle")
    focused.ink(body, neutral, report["focused"]["placeholder_pixels"], 13)
    require(focused.ink([body[0], y, 100, 24], neutral,
                        report["focused"]["upper_placeholder_pixels"], 0) == 0, "placeholder paints above its line")
    description(report["description"], HELP, False)

    disabled = report["disabled"]
    dx, dy, dw, dh = bounds(disabled["face"], 960, 820)
    require(dx == x and dw == w and dh == 56 and dy > bottom(face), "disabled peer moves or overlaps a field")
    point_x = dx + 0.75 * dw
    idle.swatch(disabled["outside"], point_x, dy - 2.5, paper)
    disabled_fill, disabled_space = idle.overlay(disabled["fill"], point_x, dy + 12.5, paper, layers[4])
    layer, above = disabled["indicator"]
    _, indicator_space = idle.overlay(layer, point_x, dy + dh - 0.5, disabled_fill, layers[38])
    idle.swatch(above, point_x, dy + dh - 1.5, disabled_fill)
    spaces = hover_space & disabled_space & indicator_space
    require(spaces, "Filled Text Field alpha probes disagree on the WebGPU blend space")
    return face, label, helper, sorted(spaces)


def composition_report(record, paints, face, frame):
    after = bounds(record["after_bounds"], 480, 820)
    require(record["before_bounds"] == face and near(after[3], 56) and after[2] < face[2]
            and record["before_selection"] == record["after_selection"] == ["中文", 2, 2]
            and record["method"] == "CDP synthetic composition and trusted Enter keys",
            "composition or selection was lost on actual reflow")
    require(record["enter_keys"] == [{"trusted": True, "dom_composing": True},
                                      {"trusted": True, "dom_composing": False}],
            "missing separate trusted composing and noncomposing Enter events")
    fallback = record["insert_text_needed"]
    require(type(fallback) is bool, "missing synthetic CDP commit path")
    for key, value, edits, submits in (
        ("first_enter", "" if fallback else "中文", 0 if fallback else 1, 0),
        ("settled", "中文", 1, 0), ("second_enter", "中文", 1, 1),
    ):
        model_state(record[key], width=480, value=value, edits=edits, submits=submits)
        require(record[key]["palette"] == paints["palette"], "composition changed the palette")
    body = entry(after, after[1] + 24, 100)
    frame.ink(body, color(paints, "on_surface"), record["composing_pixels"], 13)
    require(frame.ink(body, color(paints, "on_surface_variant"), record["placeholder_pixels"], 0) == 0,
            "placeholder duplicates the composing value")
    return {"first_enter_submits": 0, "second_enter_submits": 1, "insert_text_needed": fallback,
            "selection_preserved": True, "enter_keys": record["enter_keys"]}


def state_report(record, paints, face, label, helper, frame):
    description(record["description"], INVALID, True)
    require(record["assertive_messages"] == [INVALID, INVALID] and record["live_priority"] == "assertive",
            "assertive invalid state did not publish once per transition")
    require(record["readonly"] is True and record["disabled"] is True
            and record["tab_skipped_disabled_to"] == "filled-text-field-demo-long-label",
            "read-only selection, disabled handling or sequential focus differs")
    model = record["model"]
    model_state(model, width=960, value="中文", edits=1, submits=1, invalid=True)
    require(model["palette"] == paints["palette"] and record["error_bounds"] == helper
            and record["label_bounds"] == label, "validation paint reused the wrong helper or label")
    red, fill = (color(model, key) for key in ("error", "surface_container_highest"))
    frame.bottom_indicator(record["indicator"], face, 3, red, fill)
    frame.ink(helper, red, record["error_pixels"], 13)
    frame.ink(label, red, record["label_pixels"], 13)


def compact_report(record, paints, frame):
    model = record["model"]
    model_state(model, width=320, value="", edits=0, submits=0, compact=True, invalid=True)
    description(record["description"], INVALID, True)
    require(record["back_focus"] == "filled-text-field-demo-show", "compact Back focus was not restored")
    short, inside_label, long, above = (bounds(record[key], 320, 960)
        for key in ("short_face", "short_label", "long_face", "long_label"))
    require(short[0] == long[0] and short[2:] == long[2:] and near(long[3], 96)
            and inside(short, inside_label) and inside_label[1] - short[1] == 8 and inside_label[3] == 32
            and right(short) - right(inside_label) == 16 and above[3] >= 64
            and above[0] == long[0] + 16 and right(above) == right(long) - 16
            and bottom(above) + 4 == long[1], "wrong dark RTL 200 percent caption or face geometry")
    paper, fill, neutral, text, red = (color(model, key) for key in
        ("background", "surface_container_highest", "on_surface_variant", "on_surface", "error"))
    require(paper != color(paints, "background") and fill != color(paints, "surface_container_highest")
            and len({paper, fill, neutral, text, red}) == 5, "compact dark palette did not change")
    frame.swatch(record["short_fill"], short[0] + 8.5, short[1] + 10.5, fill)
    frame.swatch(record["long_fill"], long[0] + 8.5, long[1] + 10.5, fill)
    frame.bottom_indicator(record["short_indicator"], short, 3, red, fill)
    frame.bottom_indicator(record["long_indicator"], long, 1, neutral, fill)
    frame.ink(inside_label, red, record["short_label_pixels"], 31)
    frame.ink(above, neutral, record["long_label_pixels"], 41)
    lower_right = entry(long, long[1] + 40, 105, True)
    lower_left = entry(long, long[1] + 40, 52)
    require(record["right_value_bounds"] == lower_right and record["left_value_bounds"] == lower_left,
            "wrong measured compact lower ink sample")
    right_count = frame.ink(lower_right, text, record["right_value_pixels"], 31)
    require(frame.ink(lower_left, text, record["left_value_pixels"], 0) == 0
            and frame.ink([lower_right[0], long[1], 105, 40], text, record["upper_value_pixels"], 0) == 0,
            "compact editor value escaped the lower right entry area")
    return {"face_height_dp": long[3], "above_label_height_dp": above[3],
            "lower_right_value_pixels": right_count, "back_focus": record["back_focus"]}


def verify(artifacts, metadata):
    report_path = artifacts / REPORT
    report = json.loads(report_path.read_text())
    require(set(report) == {"browser", "page_errors", "paint", "composition", "states", "compact"}
            and report["browser"] == metadata["browser"] and report["page_errors"] == [],
            "failed or mismatched Filled Text Field report")
    require({p.name for p in artifacts.glob(f"{PREFIX}-*.png")} == set(DIMENSIONS),
            "expected exactly six original Filled Text Field PNGs")
    frames = {name: Frame(artifacts / name) for name in DIMENSIONS}
    paints, composition, states, narrow = (report[key] for key in ("paint", "composition", "states", "compact"))
    require(paints["captures"] == [IDLE, HOVER, FOCUSED] and composition["capture"] == PREEDIT
            and states["capture"] == ERROR and narrow["capture"] == COMPACT, "wrong original screenshot mapping")
    face, label, helper, spaces = paint_report(paints, frames)
    synthetic = composition_report(composition, paints, face, frames[PREEDIT])
    state_report(states, paints, face, label, helper, frames[ERROR])
    compact = compact_report(narrow, paints, frames[COMPACT])
    return {"report_sha256": checksum(report_path), "browser": report["browser"],
            "pngs": {name: {"size": list(frame.image.size), "sha256": checksum(frame.path)}
                     for name, frame in frames.items()},
            "indicator_dp": {"idle": 1, "hover": 1, "focused": 3, "invalid_focused": 3},
            "blend_spaces": spaces, "compact": compact, "synthetic_composition": synthetic,
            "assertive_messages": states["assertive_messages"], "readonly_preserved": True, "disabled_skipped": True}
