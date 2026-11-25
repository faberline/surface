"""Verify the Outlined Text Field report and its original Chrome pixels."""

import json
from math import floor, isfinite

from PIL import Image

from verify_forms_pixels import checksum, require

PREFIX = "material-outlined-text-field"
REPORT = f"{PREFIX}.json"
IDLE = f"{PREFIX}-idle.png"
FOCUSED = f"{PREFIX}-focused.png"
PREEDIT = f"{PREFIX}-ime-preedit.png"
ERROR = f"{PREFIX}-error.png"
COMPACT = f"{PREFIX}-compact-dark-rtl-200.png"
DIMENSIONS = {IDLE: (1920, 1728), FOCUSED: (1920, 1728), PREEDIT: (960, 1728),
              ERROR: (1920, 1728), COMPACT: (640, 2008)}
DPR, HEADER = 2, 44
HELP = "請輸入中文或英文名稱。"
INVALID = "請輸入有效的專案名稱。"
TOL = 0.13


def near(a, b):
    return abs(a - b) <= TOL


def color(model, key):
    rgba = model["palette"][key]
    require(type(rgba) is list and len(rgba) == 4 and rgba[-1] == 255
            and all(type(value) is int and 0 <= value <= 255 for value in rgba),
            f"bad authored color {key}")
    return tuple(rgba[:3])


def bounds(rect, width, height):
    require(type(rect) is list and len(rect) == 4
            and all(type(value) in (int, float) and isfinite(value) for value in rect),
            "invalid logical rectangle")
    x, y, w, h = rect
    require(x >= 0 and y >= 0 and w > 0 and h > 0 and x + w <= width and y + h <= height,
            "logical rectangle leaves the captured canvas")
    return rect


def right(rect):
    return rect[0] + rect[2]


def bottom(rect):
    return rect[1] + rect[3]


class Frame:
    def __init__(self, path):
        with Image.open(path) as decoded:
            require(decoded.format == "PNG" and decoded.size == DIMENSIONS[path.name],
                    f"invalid PNG size or type {path.name}")
            decoded.verify()
        with Image.open(path) as decoded:
            decoded.load()
            self.image = decoded.convert("RGB")
        self.path = path

    def region(self, rect):
        x, y, w, h = bounds(rect, self.image.width // DPR, self.image.height // DPR - HEADER)
        return self.image.crop(tuple(floor(DPR * value + 0.5)
                               for value in (x, y + HEADER, x + w, y + h + HEADER)))

    def count(self, rect, ink):
        return sum(pixel == ink for pixel in self.region(rect).getdata())

    def trace(self, actual, expected):
        require(type(actual) is list and len(actual) == len(expected), "incomplete pixel trace")
        for record, (x, y, ink) in zip(actual, expected):
            require(len(record["point"]) == 2 and all(near(a, b) for a, b in zip(record["point"], (x, y))),
                    "incorrect logical sample location")
            rgb = self.image.getpixel((floor(record["point"][0] * DPR),
                                       floor((record["point"][1] + HEADER) * DPR)))
            require(tuple(record["rgb"]) == rgb == ink, f"unverified pixel trace in {self.path.name}")

    def stroke(self, record, face, ratio, width, ink, paper):
        require(record["width_dp"] == width, "wrong outline width")
        self.trace(record["samples"], [
            (face[0] + ratio * face[2], face[1] + depth, ink if depth < width else paper)
            for depth in (0.5, 1.5, 2.5, 3.5)
        ])

    def notch(self, record, face, ink, paper, rtl):
        require(record["face"] == face, "notch does not belong to the named editor")
        label = bounds(record["label"], self.image.width // DPR, self.image.height // DPR - HEADER)
        leading = right(face) - right(label) if rtl else label[0] - face[0]
        require(near(leading, 16) and label[1] < face[1] < bottom(label), "incorrect floating label placement")
        self.trace(record["samples"], [
            (label[0] - 6, face[1] + 0.5, ink),
            (label[0] - 2, face[1] + 0.5, paper),
            (right(label) + 2, face[1] + 0.5, paper),
            (right(label) + 6, face[1] + 0.5, ink),
        ])
        return label

    def ink(self, rect, token, reported, minimum):
        actual = self.count(rect, token)
        require(type(reported) is int and actual == reported and actual >= minimum,
                f"claimed glyph count disagrees with {self.path.name}")
        return actual


def description(actual, text, invalid):
    require(actual["dom"] == {"text": text, "invalid": "true" if invalid else None, "linked": True}
            and text in actual["ax_description"], "editor description or invalid state lost")


def model_state(model, *, width, value, edits, submits, compact=False, invalid=False):
    require(model["visible"] is True and model["width"] == width
            and model["dark"] is compact and model["rtl"] is compact
            and model["show_error"] is invalid and model["text_scale"] == (2 if compact else 1)
            and model["value"] == value and model["edits"] == edits and model["submits"] == submits
            and model["locked_edit_requests"] == model["long_edits"] == 0
            and model["long_value"] == "共用名稱", "unexpected caller-owned editor state")


def body(rect):
    x, y, w, h = rect
    return [x, y + 2, min(w, 145), h - 4]


def verify(artifacts, metadata):
    report_path = artifacts / REPORT
    report = json.loads(report_path.read_text())
    require(set(report) == {"browser", "page_errors", "paint", "composition", "states", "compact"}
            and report["page_errors"] == [] and report["browser"] == metadata["browser"],
            "failed or mismatched Outlined Text Field report")
    require({path.name for path in artifacts.glob(f"{PREFIX}-*.png")} == set(DIMENSIONS),
            "expected exactly five original Outlined Text Field PNGs")
    frames = {name: Frame(artifacts / name) for name in DIMENSIONS}
    paints, composition, states, compact = (report[key] for key in ("paint", "composition", "states", "compact"))
    idle, focus, preedit, error, narrow = (frames[name] for name in DIMENSIONS)
    require(paints["captures"] == [IDLE, FOCUSED] and composition["capture"] == PREEDIT
            and states["capture"] == ERROR and compact["capture"] == COMPACT,
            "incorrect original capture mapping")

    face = bounds(paints["face"], 960, 820)
    inline = bounds(paints["inline"], 960, 820)
    helper = bounds(paints["description_bounds"], 960, 820)
    require(near(face[3], 56) and near(helper[1] - bottom(face), 4)
            and inline[0] >= face[0] and inline[1] >= face[1]
            and right(inline) <= right(face) and bottom(inline) <= bottom(face), "wide measured geometry changed")
    paper, outline, neutral, primary, text, red = [color(paints, token) for token in
        ("background", "outline", "on_surface_variant", "primary", "on_surface", "error")]
    require(len({paper, outline, neutral, primary, text, red}) == 6, "wide authored paint roles collapsed")
    idle.stroke(paints["idle"], face, 0.72, 1, outline, paper)
    focus.stroke(paints["focused"], face, 0.72, 3, primary, paper)
    float_label = focus.notch(paints["notch"], face, primary, paper, False)
    idle.ink(body(inline), neutral, paints["label_pixels"], 13)
    idle.ink(helper, neutral, paints["description_pixels"], 13)
    focus.ink(body(inline), neutral, paints["placeholder_pixels"], 13)
    focus.ink(float_label, primary, paints["floating_pixels"], 13)
    x, y, _, h = inline
    idle_changed = [(x + dx + 0.25, y + dy + 0.25) for dx in range(72) for dy in range(2, int(h) - 2)]
    changed = 0
    for x, y in idle_changed:
        point = (floor(x * DPR), floor((y + HEADER) * DPR))
        before, after = idle.image.getpixel(point), focus.image.getpixel(point)
        changed += before != after and neutral in (before, after)
    require(changed == paints["glyph_difference_samples"] and changed > 16,
            "idle label and focused placeholder glyphs lost their observed difference")
    description(paints["description"], HELP, False)

    after = bounds(composition["after_bounds"], 480, 820)
    require(composition["before_bounds"] == face and near(after[3], 56) and after[2] < face[2]
            and composition["before_selection"] == composition["after_selection"] == ["中文", 2, 2]
            and composition["method"] == "CDP synthetic composition and trusted Enter keys",
            "synthetic preedit or its offsets did not survive actual reflow")
    first = composition["first_enter"]
    injected = composition["insert_text_needed"]
    require(type(injected) is bool, "missing CDP commit ownership")
    for record, value, edits, submits in (
        (first, "" if injected else "中文", 0 if injected else 1, 0),
        (composition["settled"], "中文", 1, 0),
        (composition["second_enter"], "中文", 1, 1),
    ):
        model_state(record, width=480, value=value, edits=edits, submits=submits)
        require(record["palette"] == paints["palette"], "unexpected paint role during preedit")
    input_body = [after[0] + (inline[0] - face[0]), after[1] + (inline[1] - face[1]),
                  after[2] - (face[2] - inline[2]), inline[3]]
    preedit.ink(body(input_body), text, composition["composing_pixels"], 13)
    require(preedit.ink(body(input_body), neutral, composition["placeholder_pixels"], 0) == 0,
            "placeholder still paints during preedit")

    description(states["description"], INVALID, True)
    require(states["assertive_messages"] == [INVALID, INVALID],
            "assertive validation deliveries did not deduplicate and resume after clear")
    require(states["readonly"] is True and states["disabled"] is True
            and states["tab_skipped_disabled_to"] == "outlined-text-field-demo-long-label",
            "read-only, disabled, or sequential focus evidence missing")
    model_state(states["model"], width=960, value="中文", edits=1, submits=1, invalid=True)
    require(states["model"]["palette"] == paints["palette"] and states["error_bounds"] == helper,
            "validation ink or description geometry changed unexpectedly")
    error.stroke(states["stroke"], face, 0.72, 3, red, paper)
    error.notch(states["notch"], face, red, paper, False)
    error.ink(helper, red, states["error_pixels"], 13)

    model = compact["model"]
    model_state(model, width=320, value="", edits=0, submits=0, compact=True, invalid=True)
    description(compact["description"], INVALID, True)
    require(compact["back_focus"] == "outlined-text-field-demo-show", "compact Back focus was not restored")
    short = bounds(compact["short_notch"]["face"], 320, 960)
    long_label = bounds(compact["long_label"], 320, 960)
    long_face = [long_label[0] - 16, bottom(long_label) + 4, long_label[2] + 32, 80]
    bounds(long_face, 320, 960)
    require(near(short[3], 80) and long_label[3] >= 64 and short[0] == long_face[0]
            and short[2] == long_face[2], "compact short/long measured geometry changed")
    dark_paper, dark_outline, dark_neutral, dark_text, dark_red = [color(model, token) for token in
        ("background", "outline", "on_surface_variant", "on_surface", "error")]
    require(dark_paper != paper and len({dark_paper, dark_outline, dark_neutral, dark_text, dark_red}) == 5,
            "compact dark authored paint roles collapsed")
    narrow.stroke(compact["short_border"], short, 0.3, 3, dark_red, dark_paper)
    narrow.notch(compact["short_notch"], short, dark_red, dark_paper, True)
    narrow.stroke(compact["long_border"], long_face, 0.5, 1, dark_outline, dark_paper)
    narrow.ink(long_label, dark_neutral, compact["long_label_pixels"], 41)
    midline = long_face[1] + 20
    right_value = [right(long_face) - 94, midline, 78, 40]
    left_value = [long_face[0] + 16, midline, 52, 40]
    require(compact["right_value_bounds"] == right_value and compact["left_value_bounds"] == left_value
            and narrow.ink(right_value, dark_text, compact["right_value_pixels"], 31) > 0
            and narrow.ink(left_value, dark_text, compact["left_value_pixels"], 0) == 0,
            "compact editor does not paint its value on the logical right")
    return {"report_sha256": checksum(report_path),
            "browser": report["browser"],
            "pngs": {name: {"size": list(frame.image.size), "sha256": checksum(frame.path)}
                     for name, frame in frames.items()},
            "outline_dp": {"idle": 1, "focused": 3, "invalid_focused": 3},
            "synthetic_composition": {"first_enter_submits": 0, "insert_text_needed": injected,
                                      "second_enter_submits": 1, "selection_preserved": True},
            "assertive_messages": states["assertive_messages"],
            "disabled_skipped": True, "readonly_preserved": True, "compact_back_focus": compact["back_focus"]}
