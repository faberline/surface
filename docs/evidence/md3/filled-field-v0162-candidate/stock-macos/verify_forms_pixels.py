"""Read back the four original Centered and vertical Slider PNGs."""

from hashlib import file_digest, sha256
import json
from math import floor, isfinite
from pathlib import Path
import sys

from PIL import Image

REPORT = "material-slider-forms.json"
CAPTURES = {
    "centered": ("material-slider-forms-centered-light.png", (1920, 2408)),
    "centered_keys": ("material-slider-forms-centered-keys.png", (1920, 2408)),
    "vertical_mouse": ("material-slider-forms-vertical-light.png", (1920, 2408)),
    "compact": ("material-slider-forms-vertical-compact-dark-rtl-200.png", (640, 1848)),
}


def require(condition, context):
    if not condition:
        raise ValueError(context)


def checksum(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def rect(box):
    require(isinstance(box, list) and len(box) == 4, f"not a rectangle: {box}")
    require(all(type(value) in (int, float) and isfinite(value) for value in box), "nonfinite geometry")
    require(box[2] > 0 and box[3] > 0, "empty paint region")
    return box


def sample(image, box):
    x, y, width, height = rect(box)
    return list(image.getpixel((floor(2 * (x + width / 2)), floor(2 * (44 + y + height / 2)))))


def ink(image, box, color):
    x, y, width, height = rect(box)
    region = [floor(2 * value + 0.5) for value in (x, 44 + y, x + width, 44 + y + height)]
    left, top, right, bottom = region
    require(0 <= left < right <= image.width and 0 <= top < bottom <= image.height, "sample outside PNG")
    rgb = tuple(color[:3])
    return sum(value == rgb for value in image.crop(region).get_flattened_data())


def verify(artifacts, metadata=None):
    report_path = artifacts / REPORT
    report = json.loads(report_path.read_text())
    require(report.get("page_errors") == [] and "failure" not in report, "Slider forms were not clean")
    if metadata is not None:
        require(report["browser"] == metadata["browser"] and report["adapter"] == metadata["adapter"],
                "Slider Forms browser or WebGPU adapter disagrees")
    expected_files = {name for name, _ in CAPTURES.values()}
    require({path.name for path in artifacts.glob("material-slider-forms-*.png")} == expected_files,
            "unexpected, failed or missing Slider Forms capture")
    evidence, images = {}, {}
    for section, (name, size) in CAPTURES.items():
        path = artifacts / name
        part = report[section]
        require(part["capture"] == name, f"{section} capture name differs")
        with Image.open(path) as image:
            require(image.format == "PNG" and image.size == size, f"{section} PNG dimensions differ")
            image.load()
            color = image.convert("RGB")
        images[section] = color
        evidence[name] = {"size": list(size), "sha256": checksum(path),
                          "decoded_rgb_sha256": sha256(color.tobytes()).hexdigest()}

    centered = report["centered"]
    image = images["centered"]
    palette = centered["palette"]
    values = [slider["value"] for slider in centered["sliders"]]
    require(values == [-50, 0, 50, -25] and centered["viewport"] == [960, 1160], "signed fixture changed")
    stops = 0
    for slider in centered["sliders"]:
        shape = slider["shape"]
        value = slider["value"]
        require(slider["aria"][:3] == ["-100", "100", str(int(value))] and slider["aria"][4] is None,
                "centered numeric or orientation receipt differs")
        require(sample(image, shape["handle"]) == palette["primary"][:3], "centered handle paint differs")
        if value == 0:
            require(shape["active"][2] == 0, "neutral case paints an active portion")
        else:
            require(ink(image, shape["active"], palette["primary"]) > 50, "missing signed active paint")
        require(len(slider["inactive"]) == len(slider["stops"]) == 2, "lost centered outer spans or stops")
        for track, stop in zip(slider["inactive"], slider["stops"]):
            require(ink(image, track, palette["secondary_container"]) > 80, "missing inactive paint")
            require(sample(image, stop) == palette["on_secondary_container"][:3], "inactive stop ink differs")
            stops += 1

    keys = report["centered_keys"]
    image = images["centered_keys"]
    palette = keys["palette"]
    ring = ink(image, keys["ring"], palette["secondary"])
    text = ink(image, keys["label"], palette["inverse_on_surface"])
    require(keys["values"] == [100, 0, 50, 50] and keys["requests"] == [2, 0, 0, 3],
            "centered endpoint or RTL request counts differ")
    require(keys["neutral"]["active"][2] == 0 and keys["geometry"]["handle"][2] == 2,
            "centered neutral or focused handle bounds differ")
    require(ring == keys["ring_ink"] > 20 and text == keys["label_ink"] > 20,
            "focused Centered PNG differs from the original report")

    mouse = report["vertical_mouse"]
    image = images["vertical_mouse"]
    palette = mouse["palette"]
    active = sample(image, mouse["held"]["active"])
    inactive = sample(image, mouse["inactive"])
    handle = sample(image, mouse["held"]["handle"])
    require([active, inactive, handle] == mouse["colors"] ==
            [palette["primary"][:3], palette["secondary_container"][:3], palette["primary"][:3]],
            "vertical drag paint differs")
    require(abs(mouse["value"] - 73) < 0.4 and mouse["value_after_resize"] == mouse["value"],
            "vertical held value or resize cancel differs")
    require(mouse["initial_ax"][:3] == ["0", "100", "25"] and mouse["initial_ax"][4] == "vertical" and
            mouse["disabled_ax"][:3] == ["0", "100", "50"] and mouse["disabled_ax"][4] == "vertical",
            "vertical or disabled accessible orientation differs")
    require(ink(image, mouse["label"], palette["inverse_on_surface"]) > 20, "missing upright value text")

    dark = report["compact"]
    image = images["compact"]
    palette = dark["palette"]
    fill = ink(image, dark["label"], palette["inverse_surface"])
    label = ink(image, dark["label"], palette["inverse_on_surface"])
    focus = ink(image, dark["ring"], palette["secondary"])
    require(dark["viewport"] == [320, 880] and dark["text_scale"] == 2 and dark["rtl"] and dark["dark"],
            "compact receipt lacks dark RTL and 200 percent text")
    require(dark["vertical"]["values"] == [25, 25, 50] and dark["vertical"]["requests"] == [0, 4, 0],
            "compact gesture did not prove exact cancellation and recapture")
    require(dark["back_focus"] == "slider-forms-demo-show" and dark["aria"][:3] == ["0", "100", "25"]
            and dark["aria"][4] == "vertical", "missing compact focus or accessibility evidence")
    require(fill == dark["label_fill"] > 100 and label == dark["label_ink"] > 40
            and focus == dark["focus_ink"] > 30, "compact dark PNG paint differs")
    return {"report_sha256": checksum(report_path), "captures": evidence, "centered_stops": stops,
            "centered_ring_pixels": ring, "centered_label_pixels": text,
            "wide_vertical_value": mouse["value"], "compact_dark_label_ink": label,
            "compact_dark_label_fill": fill, "compact_dark_focus_ink": focus}


if __name__ == "__main__":
    print(json.dumps(verify(Path(sys.argv[1])), indent=2))
