"""Check Standard Slider browser behavior and exact stock WebGPU screenshot pixels."""
import json
import re
from hashlib import file_digest, sha256
from math import floor, isfinite

from PIL import Image

STEM = "material-standard-slider"
REPORT = f"{STEM}.json"
WIDE = f"{STEM}-wide-light.png"
KEYBOARD = f"{STEM}-keyboard.png"
HELD = f"{STEM}-live-drag.png"
COMPACT = f"{STEM}-compact-dark-rtl-200.png"
IMAGES = {WIDE: (1920, 2168), KEYBOARD: (1920, 2168), HELD: (1920, 2168), COMPACT: (640, 1848)}
STEPS = ("M298", "M299", "M300", "M301")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as handle:
        return file_digest(handle, "sha256").hexdigest()


def validate(root, log, metadata):
    path = root / REPORT
    report = json.loads(path.read_text())
    verify_report(report, metadata)
    require(tuple(re.findall(r"^\s*\[(M298|M299|M300|M301)\]", log.read_text(), re.M)) == STEPS,
            "missing, duplicate or out-of-order Standard Slider step")
    require({image.name for image in root.glob(f"{STEM}-*.png")} == set(IMAGES), "wrong Standard Slider PNG set")
    captures, images = {}, {}
    for name, dimensions in IMAGES.items():
        original = root / name
        with Image.open(original) as opened:
            require(opened.format == "PNG" and opened.mode in {"RGB", "RGBA"} and opened.size == dimensions,
                    f"wrong Standard Slider screenshot dimensions: {name}")
            opened.verify()
        with Image.open(original) as opened:
            opened.load()
            decoded = opened.convert("RGB")
        images[name] = decoded
        captures[name] = {"size": list(dimensions), "sha256": digest(original),
                          "decoded_rgb_sha256": sha256(decoded.tobytes()).hexdigest()}
    require(len({item["decoded_rgb_sha256"] for item in captures.values()}) == len(IMAGES),
            "duplicate Standard Slider screenshot pixels")
    return {"steps": list(STEPS), "report_sha256": digest(path), "pngs": captures,
            "page_errors": 0, "specimens": len(report["wide"]["geometry"]),
            "pointer_requests": report["pointer"]["requests"], "compact_back_focus": report["compact"]["back_focus"],
            "pixels": measure_pixels(report, images)}


def slider_rect(box, name):
    require(isinstance(box, list) and len(box) == 4
            and all(type(n) in (int, float) and isfinite(n) for n in box)
            and box[2] > 0 and box[3] > 0, f"bad Slider bounds for {name}")
    return box


def slider_near(actual, expected, tolerance=0.75):
    return type(actual) in (int, float) and isfinite(actual) and abs(actual - expected) <= tolerance


def slider_contains(parent, child):
    px, py, pw, ph = parent
    x, y, w, h = child
    return px - 0.75 <= x and py - 0.75 <= y and x + w <= px + pw + 0.75 and y + h <= py + ph + 0.75


def slider_geometry(entry, suffix, viewport, track, handle, fraction, engaged, rtl=False):
    require(entry["id"] == f"slider-demo-{suffix}", "wrong Slider specimen")
    hit, rail, active, inactive, grip = [slider_rect(entry[kind], kind)
                                        for kind in ("hit", "rail", "active", "inactive", "handle")]
    require(slider_contains([0, 0, *viewport], hit) and all(
        slider_contains(hit, part) for part in (rail, active, inactive, grip)
    ), "Slider target or artwork goes outside its viewport")
    require(hit[2] >= 48 and slider_near(hit[3], max(48, handle))
            and slider_near(rail[0] - hit[0], 12) and slider_near(hit[2] - rail[2], 24)
            and slider_near(grip[2], 2 if engaged else 4) and slider_near(grip[3], handle)
            and slider_near(active[3], track) and slider_near(inactive[3], track),
            "Slider track, handle or target dimensions differ")
    left, right = (inactive, active) if rtl else (active, inactive)
    center = grip[0] + grip[2] / 2
    ratio = (center - rail[0]) / rail[2]
    require(slider_near(ratio, 1 - fraction if rtl else fraction, 0.005)
            and slider_near(grip[0] - (left[0] + left[2]), 6)
            and slider_near(right[0] - (grip[0] + grip[2]), 6),
            "Slider value position or logical direction differs")
    return entry


def slider_color(color, channels):
    return isinstance(color, list) and len(color) == channels and all(type(c) is int and 0 <= c <= 255 for c in color)


def verify_report(report, metadata):
    require(set(report) == {"browser", "adapter", "wide", "keyboard", "pointer", "compact", "page_errors"}
            and report["browser"] == metadata["browser"] and report["adapter"] == metadata["adapter"]
            and type(report["adapter"]["isFallbackAdapter"]) is bool,
            "Slider Chrome or WebGPU adapter disagrees")
    require(report["page_errors"] == [],
            "Slider browser report contains a failure")
    wide, keyboard, pointer, compact = (report[k] for k in ("wide", "keyboard", "pointer", "compact"))
    require([part["capture"] for part in (wide, keyboard, pointer, compact)] == list(IMAGES),
            "wrong Slider screenshot paths")
    require(wide["viewport"] == [960, 1040] and compact["viewport"] == [320, 880]
            and compact["rtl"] is True and compact["text_scale"] == 2,
            "Slider CSS viewports, RTL or text scaling differ")

    require(len(wide["geometry"]) == 5, "missing wide Slider size tokens")
    for entry, suffix, track, handle, fraction in zip(
        wide["geometry"], ("xs", "s", "m", "l", "xl"),
        (16, 24, 40, 56, 96), (44, 44, 52, 68, 108), (0.25, 0.5, 0.5, 0.5, 0.5), strict=True,
    ):
        slider_geometry(entry, suffix, wide["viewport"], track, handle, fraction, False)
    require(wide["ax"] == ["25", "25％"] and wide["disabled_ax"] == ["50", "50％"]
            and keyboard["ax"] == ["10", "10％"] and compact["ax"] == ["30", "30％"],
            "Slider numeric or localized accessibility value differs")
    colors = wide["colors"]
    require(len(colors) == 3 and all(slider_color(color, 3) for color in colors)
            and colors[0] == colors[2] != colors[1], "Slider track/handle colors differ")
    for kind in ("stop", "tick"):
        dot = slider_rect(wide[kind], kind)
        require(all(slider_near(side, 4) for side in dot[2:]), f"wrong Slider {kind} dimensions")
    require(slider_contains(wide["geometry"][0]["inactive"], wide["stop"])
            and slider_contains(wide["geometry"][1]["active"], wide["tick"]),
            "Slider stop or tick falls outside its track")

    g = slider_geometry(keyboard["geometry"], "xs", wide["viewport"], 16, 44, 0.1, True)
    first = wide["geometry"][0]
    label = slider_rect(keyboard["value_label"], "keyboard indicator")
    ring = slider_rect(keyboard["ring"], "keyboard focus")
    require(g["hit"] == first["hit"] and g["rail"] == first["rail"]
            and slider_near(ring[0], g["handle"][0] - 5) and slider_near(ring[1], g["handle"][1] - 5)
            and slider_near(ring[2], 12) and slider_near(ring[3], 54)
            and label[3] >= 44 and slider_near(label[1] + label[3] + 4, ring[1])
            and type(keyboard["focus_ink"]) is int and keyboard["focus_ink"] > 15
            and type(keyboard["label_ink"]) is int and keyboard["label_ink"] > 15
            and type(keyboard["continuous_requests"]) is int and keyboard["continuous_requests"] == 4
            and keyboard["discrete"] == 62.5, "Slider keys, focus ring or indicator differ")

    drag = slider_rect(pointer["handle"], "held mouse handle")
    require(slider_near(pointer["tap"], 33.3, 0.4)
            and slider_near(pointer["held"], 73.1, 0.4)
            and pointer["canceled"] == pointer["held"]
            and slider_near(drag[2], 2) and slider_near(drag[3], 44)
            and slider_near(drag[0] + drag[2] / 2,
                            first["rail"][0] + first["rail"][2] * pointer["held"] / 100)
            and pointer["touch"] == 75 and pointer["requests"] == [2, 2, 0, 0, 0, 0]
            and all(type(n) is int for n in pointer["requests"]),
            "Slider continuous drag, cancellation, discrete touch or disabled counts differ")

    g = slider_geometry(compact["geometry"], "xs", compact["viewport"], 16, 44, 0.3, True, True)
    label = slider_rect(compact["label"], "compact indicator")
    require(slider_contains([0, 0, *compact["viewport"]], label)
            and label[3] >= 64 and slider_near(label[1] + label[3] + 9, g["handle"][1])
            and type(compact["label_ink"]) is int and compact["label_ink"] > 40
            and compact["back_focus"] == "slider-demo-show", "compact Slider paint or return focus differs")

    keys = {"background", "primary", "on_primary", "secondary", "secondary_container",
            "on_secondary_container", "inverse_surface", "inverse_on_surface"}
    for part in (wide, keyboard, compact):
        palette = part.get("palette")
        require(isinstance(palette, dict) and keys <= palette.keys()
                and all(slider_color(palette[key], 4) and palette[key][3] == 255 for key in keys),
                "missing or invalid Slider palette")
    require(wide["palette"] == keyboard["palette"] != compact["palette"]
            and colors == [wide["palette"][key][:3] for key in ("primary", "secondary_container", "primary")],
            "Slider light and dark palettes disagree")




def measure_pixels(report, images):
    wide, keyboard, pointer, compact = (report[name] for name in ("wide", "keyboard", "pointer", "compact"))

    def sample(image, box):
        x, y, width, height = box
        return list(image.getpixel((floor(2 * (x + width / 2)), floor(2 * (44 + y + height / 2)))))

    def ink(image, box, color):
        x, y, width, height = box
        bounds = [floor(2 * value + 0.5) for value in (x, 44 + y, x + width, 44 + y + height)]
        require(0 <= bounds[0] < bounds[2] <= image.width
                and 0 <= bounds[1] < bounds[3] <= image.height, "Slider sample is outside the original PNG")
        expected = tuple(color[:3])
        return sum(pixel == expected for pixel in image.crop(bounds).getdata())

    light, keys, held, dark = (images[name] for name in IMAGES)
    first = wide["geometry"][0]
    colors = [sample(light, first[kind]) for kind in ("active", "inactive", "handle")]
    held_color = sample(held, pointer["handle"])
    require(colors == wide["colors"] and held_color == colors[0], "Slider decoded track or held handle RGB differs")
    measured = {"track_and_handle_rgb": colors, "held_handle_rgb": held_color}
    primary = wide["palette"]
    light_value = keyboard["palette"]
    dark_value = compact["palette"]
    stop = sample(light, wide["stop"])
    tick = ink(light, wide["tick"], primary["on_primary"])
    focus = ink(keys, keyboard["ring"], light_value["secondary"])
    key_ink = ink(keys, keyboard["value_label"], light_value["inverse_on_surface"])
    key_fill = ink(keys, keyboard["value_label"], light_value["inverse_surface"])
    compact_ink = ink(dark, compact["label"], dark_value["inverse_on_surface"])
    compact_fill = ink(dark, compact["label"], dark_value["inverse_surface"])
    require(stop == primary["on_secondary_container"][:3] and tick > 0
            and focus == keyboard["focus_ink"] and key_ink == keyboard["label_ink"] and key_fill > 15
            and compact_ink == compact["label_ink"] and compact_fill > 15
            and sample(dark, compact["geometry"]["active"]) == dark_value["primary"][:3]
            and sample(dark, compact["geometry"]["inactive"]) == dark_value["secondary_container"][:3],
            "Slider decoded ring, dot, label or dark track paint differs")
    return {**measured, "stop_rgb": stop, "tick_pixels": tick, "focus_pixels": focus,
            "key_label_ink": key_ink, "key_label_fill": key_fill,
            "compact_label_ink": compact_ink, "compact_label_fill": compact_fill}


