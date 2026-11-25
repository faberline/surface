"""Read the original Range Slider captures and compare the resolved paint."""

from hashlib import sha256
import json
from math import floor
from pathlib import Path
import sys

from PIL import Image

from verify_forms_pixels import checksum, ink, require, sample

REPORT = "material-range-slider.json"
CAPTURES = {
    "wide": ("material-range-slider-wide-light.png", (960, 1160)),
    "overlap": ("material-range-slider-overlap-focus.png", (960, 1160)),
    "capture_loss": ("material-range-slider-held-loss.png", (960, 1160)),
    "compact": ("material-range-slider-compact-dark-rtl-200.png", (320, 880)),
}


def decoded(artifacts, report):
    expected = {name for name, _ in CAPTURES.values()}
    require({file.name for file in artifacts.glob("material-range-slider-*.png")} == expected,
            "missing or unexpected Range Slider original")
    captures = {}
    images = {}
    for section, (name, viewport) in CAPTURES.items():
        part = report[section]
        require(part["viewport"] == list(viewport) and part["capture"] == name,
                f"{section}: viewport or capture name differs")
        width, height = viewport
        size = (width * 2, (height + 44) * 2)
        path = artifacts / name
        with Image.open(path) as image:
            require(image.format == "PNG" and image.size == size, f"{section}: PNG dimensions differ")
            image.load()
            color = image.convert("RGB")
        images[section] = color
        captures[name] = {"size": list(size), "sha256": checksum(path),
                          "decoded_rgb_sha256": sha256(color.tobytes()).hexdigest()}
    return captures, images


def near(left, right):
    return abs(left - right) < 0.76


def midpoint(box):
    return box[0] + box[2] / 2


def track_sample(image, rect):
    x, y, width, height = rect
    return list(image.getpixel((floor(2 * (x + width / 2)), floor(2 * (44 + y + 0.3 * height)))))


def validate_axes(payload, limits, values, disabled=False):
    thumbs = payload["thumbs"]
    require(len(thumbs) == 2 and thumbs[0]["node"] != thumbs[1]["node"],
            "Chrome must retain separate accessible thumbs")
    for index, thumb in enumerate(thumbs):
        require(thumb["id"].endswith(("-lower", "-upper")[index]) and
                thumb["limits"] == limits[index] and thumb["value"] == values[index] and
                thumb["disabled"] is disabled and bool(thumb["name"]), "wrong Range AX descriptor")
        expected = [*limits[index], values[index]]
        require([float(value) for value in thumb["attributes"][:3]] == expected and
                bool(thumb["attributes"][3]) and
                thumb["attributes"][4] == ("true" if disabled else None),
                "projected Range attributes differ from the Chrome tree")


def validate_wide(part, image):
    palette = part["palette"]
    expected = [([0, 100], [25, 75]), ([-10, 90], [15, 65]), ([0, 100], [50, 50]), ([0, 100], [25, 75])]
    require(len(part["shapes"]) == len(part["colors"]) == len(part["ax"]) == 4,
            "four distinct Range fixtures are required")
    for index, (shape, receipt, axes) in enumerate(zip(part["shapes"], part["colors"], part["ax"], strict=True)):
        handles, targets = shape["handles"], shape["targets"]
        require(len(handles) == len(targets) == 2, "each thumb needs a handle and target")
        for handle, target in zip(handles, targets):
            require(near(target[2], 48) and target[3] >= 48 and near(midpoint(handle), midpoint(target)),
                    "each wide target must align with the painted handle")
        rail = shape["rail"]
        owner = shape["owner"]
        require(near(rail[0] - owner[0], 24) and near(owner[2] - rail[2], 48), "the shared rail is not inset")
        active = shape["active"]
        if index == 2:
            require(active[2] == 0 and targets[0] == targets[1], "overlap needs distinct controls at one position")
        else:
            require(near(active[0] - handles[0][0] - handles[0][2], 6) and
                    near(handles[1][0] - active[0] - active[2], 6), "active segment gaps differ")
        decoded_colors = {
            "active": track_sample(image, active) if active[2] > 0 else None,
            "inactive": [track_sample(image, item) for item in shape["inactive"]],
            "handles": [sample(image, item) for item in handles],
            "stops": [sample(image, item) for item in shape["stops"]],
        }
        require(decoded_colors == receipt, "wide decoded paint differs")
        if index == 3:
            require(all(color not in (palette["primary"][:3], palette["background"][:3])
                        for color in decoded_colors["handles"]), "disabled handles must paint in muted ink")
        else:
            require(decoded_colors["inactive"] == [palette["secondary_container"][:3]] * 2 and
                    decoded_colors["handles"] == [palette["primary"][:3]] * 2 and
                    decoded_colors["stops"] == [palette["on_secondary_container"][:3]] * 2 and
                    decoded_colors["active"] in (None, palette["primary"][:3]), "wide palette paint differs")
        (low, high), values = expected[index]
        validate_axes(axes, [[low, values[1]], [values[0], high]], values, index == 3)
    return {"fixture_count": 4, "handle_count": 8, "active_colors": [row["active"] for row in part["colors"]]}


def trusted_order(events, device, names):
    first = next(event for event in events
                 if event["device"] == device and event["trusted"] is True and event["kind"] == names[0])
    owner = first["id"]
    actual = iter(event["kind"] for event in events if event["device"] == device
                  and event["trusted"] is True and event["id"] == owner)
    require(all(any(kind == expected for kind in actual) for expected in names),
            f"missing trusted events in order: {names}")
    return owner


def validate_events(report):
    overlap = report["overlap"]
    require(overlap["shape"]["active"][2] == 0, "coincident focused interval paints active track")
    validate_axes(overlap["initial_ax"], [[0, 50], [50, 100]], [50, 50])
    validate_axes(overlap["locked_ax"], [[0, 0], [0, 100]], [0, 0])
    require(overlap["extremes"]["values"] == [0, 100] and
            overlap["discrete"]["values"] == [65, 77.5], "peer limits changed the common stop lattice")

    loss = report["capture_loss"]
    owner = trusted_order(loss["loss_events"], "mouse",
                          ("pointerdown", "gotpointercapture", "pointermove", "lostpointercapture", "pointerup"))
    trusted_order(loss["normal_events"], "mouse",
                  ("pointerdown", "gotpointercapture", "pointermove", "pointerup", "lostpointercapture"))
    require(any(event["kind"] == "lostpointercapture" and event["trusted"] is False
                and event["id"] == owner for event in loss["loss_events"]), "synthetic-loss control absent")
    foreign = trusted_order(loss["foreign_events"], "touch", ("pointerdown", "pointerup"))
    require(foreign != owner and loss["held"]["values"] == loss["after"]["values"] and
            loss["held"]["requests"] == loss["after"]["requests"] == [1, 2] and
            loss["fresh"]["requests"] == [1, 4], "foreign or late release applied an extra value")

    compact = report["compact"]
    require(compact["rtl"] is True and compact["dark"] is True and compact["text_scale"] == 2,
            "compact dark RTL 200 percent receipt is required")
    require(compact["shape"]["handles"][0][0] > compact["shape"]["handles"][1][0] and
            compact["back_focus"] == "range-slider-demo-show", "mirrored geometry or Back focus differs")
    trusted_order(compact["touch_cancel"], "touch", ("pointerdown", "pointermove", "pointercancel"))
    trusted_order(compact["touch_fresh"], "touch", ("pointerdown", "pointerup"))
    require(compact["held"]["values"] == compact["after"]["values"] and
            compact["after"]["requests"] == [1, 0] and compact["fresh"]["requests"] == [1, 1],
            "touch cancel or rearm applied the wrong action")
    validate_axes(compact["ax"], [[-10, 65], [27.5, 90]], [27.5, 65])
    return {"mouse_owner": owner, "foreign_touch": foreign, "held": loss["held"]["values"],
            "after": loss["after"]["values"], "fresh": loss["fresh"]["values"]}


def read_indicator(part, image, name):
    palette = part["palette"]
    paint = part["paint"]
    label, ring = paint["label"], paint["ring"]
    text = ink(image, label, palette["inverse_on_surface"])
    fill = ink(image, label, palette["inverse_surface"])
    focus = ink(image, ring, palette["secondary"])
    require(text == paint["label_ink"] and fill == paint["label_fill"] and focus == paint["focus_ink"],
            f"{name}: label or ring pixels differ from the actual browser receipt")
    require(text > 20 and fill > 100, f"{name}: value label is not legible")
    if name != "capture_loss":
        require(focus > 20, f"{name}: visible focus required")
    return {"text": text, "fill": fill, "focus": focus}


def verify(artifacts, metadata=None):
    report_path = artifacts / REPORT
    report = json.loads(report_path.read_text())
    require(report.get("page_errors") == [] and "failure" not in report, "Range Slider was not clean")
    if metadata is not None:
        require(report["browser"] == metadata["browser"] and report["adapter"] == metadata["adapter"],
                "Range Slider browser or WebGPU adapter disagrees")
    captures, images = decoded(artifacts, report)
    wide = validate_wide(report["wide"], images["wide"])
    events = validate_events(report)
    indicators = {section: read_indicator(report[section], images[section], section)
                  for section in ("overlap", "capture_loss", "compact")}
    return {"report_sha256": checksum(report_path), "captures": captures, "wide": wide,
            "indicators": indicators, "events": events}


if __name__ == "__main__":
    print(json.dumps(verify(Path(sys.argv[1])), ensure_ascii=False, indent=2))
