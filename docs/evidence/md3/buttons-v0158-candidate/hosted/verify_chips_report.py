"""Replay the four Chips steps from the typed report and original WebGPU PNGs."""

from hashlib import file_digest, sha256
import json
from math import floor, isfinite
import re

from PIL import Image


STEM = "material-chips"
REPORT = f"{STEM}.json"
STEPS = ("M290", "M291", "M292", "M293")
MODES = ("wide-light", "wide-focus", "wide-pressed", "compact-dark-rtl-200")
IMAGES = {f"{STEM}-{mode}.png": (640, 2128) if mode == MODES[-1] else (1920, 1688) for mode in MODES}
PREFIX = "chip-demo-"
VARIANTS = (("assist", "assist-flat"), ("assist-elevated", "assist-elevated"), ("news", "filter-flat"),
            ("images", "filter-elevated"), ("chen", "input"), ("suggest", "suggestion-flat"),
            ("suggest-elevated", "suggestion-elevated"))
ACTIONS = (
    ("assist", "排程", "button", "動作與建議", None, False),
    ("assist-elevated", "查看行程", "button", "動作與建議", None, False),
    ("disabled", "停用", "button", "動作與建議", None, True),
    ("news", "新聞", "checkbox", "搜尋篩選", "true", False),
    ("images", "圖片", "checkbox", "搜尋篩選", "false", False),
    ("filter-type", "依檔案類型篩選", "checkbox", "搜尋篩選", "false", False),
    ("filter-type-menu", "開啟檔案類型選單", "button", "搜尋篩選", None, False),
    ("chen", "小陳", "button", "收件人", None, False),
    ("chen-remove", "移除小陳", "button", "收件人", None, False),
    ("mei", "移除小美", "button", "收件人", None, False),
    ("lin", "林小姐的工作用電子郵件", "button", "收件人", None, False),
    ("lin-remove", "移除林小姐", "button", "收件人", None, False),
)


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def name(part):
    return PREFIX + part


def rect(value, description):
    require(isinstance(value, list) and len(value) == 4
            and all(type(item) in (int, float) and isfinite(item) for item in value)
            and min(value[2:]) > 0, f"invalid Chips {description} rectangle")
    return value


def rgba(value, count):
    require(isinstance(value, list) and len(value) == count
            and all(type(item) is int and 0 <= item <= 255 for item in value), "invalid Chips color")
    return value[:3]


def ax(action, expected):
    id_, label, role, group, state, disabled = expected
    require((action["id"], action["name"], action["role"], action["grid"], action["checked"]) ==
            (name(id_), label, role, group, state) and action["disabled"] is disabled
            and action["ancestry"] == ["gridcell", "row", "grid"], f"wrong Chips AX action or ancestry: {id_}")


def edge(box, axis):
    return box[axis] + box[axis + 2]


def center(box, axis):
    return box[axis] + box[axis + 2] / 2


def contains(outer, inner):
    return all(outer[axis] <= inner[axis] + 0.1 and edge(outer, axis) >= edge(inner, axis) - 0.1 for axis in (0, 1))


def capture(data, images, mode, viewport):
    path = f"{STEM}-{mode}.png"
    require(data["capture"] == path, f"wrong Chips capture relation: {mode}")
    if viewport:
        require(data["canvas"] == viewport, f"wrong Chips canvas for {mode}")
    return images[path]


def pixel(image, x, y):
    # All captures include the 44 CSS px host header at device scale 2.
    px, py = floor(2 * x), floor(2 * (y + 44))
    require(0 <= px < image.width and 0 <= py < image.height, "Chips sample outside the image")
    return list(image.getpixel((px, py)))


def colors(image, x, y, width, height):
    def round_px(value):
        return floor(2 * value + 0.5)

    left, top = round_px(x), round_px(y + 44)
    right, bottom = round_px(x + width), round_px(y + height + 44)
    require(0 <= left < right <= image.width and 0 <= top < bottom <= image.height,
            "Chips region outside the image")
    return {color: count for count, color in image.crop((left, top, right, bottom)).getcolors((right-left)*(bottom-top))}


def wide_check(data, image):
    require(len(data["variants"]) == len(VARIANTS), "incorrect Chips style matrix")
    faces = []
    for row, (id_, kind) in zip(data["variants"], VARIANTS, strict=True):
        require((row["id"], row["kind"]) == (name(id_), kind), "incorrect Chips style identity")
        face, hit, chip, label = (rect(row[key], key) for key in ("face", "hit", "chip", "label"))
        require(abs(face[3] - 32) <= 0.75 and abs((face[1] - hit[1]) - 8) <= 0.75 and min(hit[2:]) >= 48
                and contains(chip, hit) and contains(hit, label) and contains([0, 0, 960, 800], chip),
                f"Chips face height, hit area or label bounds differ for {id_}")
        faces.append(face)
    require(data["artwork"] == [[18, 18], [18, 18], [18, 18], [24, 24]], "Chips icon/check/image/avatar dimensions differ")
    require(len(data["ax"]) == len(ACTIONS), "wrong Chips AX action count")
    for action, expected in zip(data["ax"], ACTIONS, strict=True):
        ax(action, expected)
    palette = {key: rgba(data["palette"][key], 4) for key in ("surface_container_low", "surface", "secondary_container", "outline_variant", "secondary")}
    fill = lambda face: pixel(image, face[0] + 3, center(face, 1))
    base, selected = fill(faces[0]), fill(faces[2])
    elevated = [fill(faces[i]) for i in (1, 3, 6)]
    border = pixel(image, faces[0][0] + 0.5, center(faces[0], 1))
    require(base == data["surface_pixel"] == palette["surface"] and selected == data["selected_fill_pixel"] == palette["secondary_container"]
            and elevated == data["elevated_fill_pixels"] == [palette["surface_container_low"]] * 3
            and border == data["outline_pixel"] == palette["outline_variant"], "Chips light palette pixels differ")
    shadows = []
    for index in (0, 1, 3, 6):
        face = faces[index]
        box = colors(image, face[0] + 12., edge(face, 1) + 0.5, face[2] - 24., 4.)
        shadows.append(sum(count for color, count in box.items() if list(color) != base))
    require(shadows == data["shadow_samples"] and shadows[0] == 0 and min(shadows[1:]) > 0,
            "Chips elevated-specific shadows differ")
    return faces[0], base, palette["secondary"], shadows


def keyboard_check(data, image, ink):
    require(data["stops"] == [name(id_) for id_ in ("assist", "assist-elevated", "suggest-elevated", "news",
             "assist", "news", "images", "chen", "chen-remove")] and data["multi"] == [True, True, False]
            and data["single"] == [False, True, False] and data["wide_menu_trigger"] == name("filter-type-menu")
            and data["focus"] == name("chen-remove"), "Chips roving, independent menu or manual selection differs")
    box = rect(data["focus_rect"], "keyboard focus")
    focus = colors(image, *box).get(tuple(ink), 0)
    require(all(abs(edge - 28) <= 0.75 for edge in box[2:]) and focus == data["focus_pixels"] > 20,
            "Chips independent remove focus pixels differ")
    return focus


def pointer_check(data, image, face, base):
    pressed = pixel(image, face[0] + 3, center(face, 1))
    require(pressed == data["held"] != data["before"] == base, "Chips pressed pixel differs")
    body, tail = (rect(data[key], key) for key in ("body_target", "remove_target"))
    require(min(*body[2:], *tail[2:]) >= 48 and edge(body, 0) <= tail[0] + 0.1 and body[1] == tail[1],
            "Chips recipient hit targets overlap or are too small")
    for key, target in (("body_point", body), ("remove_point", tail)):
        x, y = data[key]
        require(target[0] < x < edge(target, 0) and target[1] < y < edge(target, 1), "Chips input missed its hit target")
    journeys = ("after_body", "after_canceled_touch", "after_remove", "after_delete", "after_backspace")
    for phase, removed, last in zip(journeys, (0, 0, 1, 2, 3), ("chen", "chen", "chen", "mei", "lin"), strict=True):
        state = data[phase]
        require((state["actions"], state["edits"], state["removals"], state["last"]) == (1, 1, removed, name(last))
                and state["removed"] == [index < removed for index in range(3)], f"Chips mouse or touch change differs: {phase}")
    require(data["restored_focus"] == [name("mei"), name("lin")], "Chips focus replacement differs")
    return pressed


def compact_check(data, image):
    first, neighbor, down, label, remove = (rect(data[key], key) for key in (
        "first_row", "next_in_row", "following_row", "long_label", "removal"))
    require(data["rtl"] is True and data["text_scale"] == 2 and first[0] > neighbor[0]
            and abs(center(first, 1) - center(neighbor, 1)) <= 0.75 and down[1] >= edge(neighbor, 1) - 1,
            "Chips mirrored compact measured row geometry differs")
    focus = data["focus_path"]
    require(len(focus) == 4 and focus[:2] == [name("assist"), name("assist-elevated")]
            and focus[2] in (name("suggest"), name("suggest-elevated"))
            and focus[3] == min(zip(focus[:2], (first, neighbor), strict=True), key=lambda node: abs(
                center(node[1], 0) - center(down, 0)))[0], "Chips RTL nearest-row focus path differs")
    require(data["menu_body"] == name("filter-type") and data["back_focus"] == name("show")
            and data["selection_after_menu"] == [True, False, True], "Chips whole-body menu selection/focus differs")
    ax(data["menu_ax"], ("filter-type", "依檔案類型篩選", "button", "搜尋篩選", None, False))
    require(edge(remove, 0) <= label[0] + 0.1 and min(remove[2:]) >= 48 and label[3] >= 80
            and all(contains([0, 0, 320, 1020], box) for box in (first, neighbor, down, label, remove)),
            "Chips full compact label or RTL removal target is clipped")
    x, y, width, height = label
    ink = tuple(rgba(data["label_ink"], 3))
    lines = [colors(image, x, y + offset, width, min(40, height - offset)).get(ink, 0)
             for offset in range(0, round(height), 40)]
    require(len(lines) >= 2 and min(lines) > 20 and lines == data["label_line_ink"], "Chips wrapped foreground pixels differ")
    return lines


def validate(root, log, metadata):
    report = json.loads((root / REPORT).read_text())
    require(set(report) == {"browser", "adapter", "wide", "keyboard", "pointer", "compact", "page_errors"}
            and report["browser"] == metadata["browser"] and report["adapter"] == metadata["adapter"]
            and type(report["adapter"]["isFallbackAdapter"]) is bool and report["page_errors"] == [],
            "Chips failure, page errors or Chrome adapter mismatch")
    require(tuple(re.findall(r"^\s*\[(M29[0-3])\]", log.read_text(), re.M)) == STEPS,
            "missing, duplicate or out-of-order Chips step")
    require({item.name for item in root.glob(f"{STEM}-*.png")} == set(IMAGES), "wrong Chips PNG set")
    images, receipts = {}, {}
    for path, size in IMAGES.items():
        with Image.open(root / path) as image:
            require(image.format == "PNG" and image.size == size, f"wrong Chips screenshot dimensions: {path}")
            decoded = image.convert("RGB")
        images[path] = decoded
        receipts[path] = {"size": list(size), "sha256": digest(root / path), "decoded_rgb_sha256": sha256(decoded.tobytes()).hexdigest()}
    require(len({item["decoded_rgb_sha256"] for item in receipts.values()}) == 4, "duplicate Chips screenshot pixels")
    wide, keys, pointer, compact = (report[key] for key in ("wide", "keyboard", "pointer", "compact"))
    face, base, focus_rgb, shadows = wide_check(wide, capture(wide, images, MODES[0], [960, 800]))
    focus = keyboard_check(keys, capture(keys, images, MODES[1], None), focus_rgb)
    pressed = pointer_check(pointer, capture(pointer, images, MODES[2], None), face, base)
    lines = compact_check(compact, capture(compact, images, MODES[3], [320, 1020]))
    return {"steps": list(STEPS), "report_sha256": digest(root / REPORT), "pngs": receipts,
            "wide_variants": len(VARIANTS), "ax_actions": len(ACTIONS), "shadow_pixels": shadows,
            "independent_focus_ink_pixels": focus, "pressed_pixel": pressed, "compact_line_ink": lines,
            "page_errors": 0}
