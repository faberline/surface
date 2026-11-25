"""Cross-check the browser fixture's golden hues and the existing PNG crops."""

from hashlib import sha256
import json
from math import cbrt, exp, floor, sqrt
from pathlib import Path
import sys

from PIL import Image

ROOT = Path("/tmp/surface-md3-label-motion-e2e.PywRddEC/development-r3/artifacts")
BASE_REPORT = ROOT / "material-text-field-label-motion.json"
PREVIOUS_REPORT_SHA = "31139899f13b63a7a8c178ce02d79505d2621f71de47b44c49808a4fb57be1d1"


def transform(matrix, vector):
    return tuple(sum(row[i] * vector[i] for i in range(3)) for row in matrix)


def to_linear(byte):
    x = byte / 255
    return x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4


def to_byte(linear):
    x = 12.92 * linear if linear <= 0.0031308 else 1.055 * linear ** (1 / 2.4) - 0.055
    return floor(0.5 + max(0, min(1, x)) * 255)


def from_srgb(rgb):
    lms = transform(((.4122214708, .5363325363, .0514459929),
                     (.2119034982, .6806995451, .1073969566),
                     (.0883024619, .2817188376, .6299787005)), tuple(map(to_linear, rgb)))
    return transform(((.2104542553, .7936177850, -.0040720468),
                      (1.9779984951, -2.4285922050, .4505937099),
                      (.0259040371, .7827717662, -.8086757660)), tuple(map(cbrt, lms)))


def from_lab(lab):
    lms = transform(((1, .3963377774, .2158037573),
                     (1, -.1055613458, -.0638541728),
                     (1, -.0894841775, -1.2914855480)), lab)
    rgb = transform(((4.0767416621, -3.3077115913, .2309699292),
                     (-1.2684380046, 2.6097574011, -.3413193965),
                     (-.0041960863, -.7034186147, 1.7076147010)), tuple(x ** 3 for x in lms))
    return tuple(map(to_byte, rgb))


def pose(rgb):
    return from_srgb(rgb), (0, 0, 0)


def sample(state, color, time):
    at, velocity = state
    end = from_srgb(color)
    omega = sqrt(3800)
    time /= 1000
    decay = exp(-omega * time)
    delta = tuple(x - y for x, y in zip(at, end))
    change = tuple(v + d * omega for d, v in zip(delta, velocity))
    return (
        tuple(y + (d + t * time) * decay for y, d, t in zip(end, delta, change)),
        tuple((v - omega * t * time) * decay for v, t in zip(velocity, change)),
    )


def calibrate(report, palette):
    title = report["label"]
    pixel_box = [int(v * 2) for v in (title[0], title[1] + 44, title[0] + title[2], title[1] + 44 + title[3])]
    x0, y0, x1, y1 = pixel_box
    with Image.open(ROOT / report["png"]) as decoded:
        image = decoded.convert("RGB")
    color = tuple(palette[:3])
    points = []
    for x in range(x0, x1):
        if points and x < points[-1][0] + 6:
            continue
        for y in range(y0, y1):
            if all(image.getpixel((x + dx, y)) == color for dx in (-1, 0, 1)):
                points.append((x, y))
                break
        if len(points) == 4:
            break
    assert len(points) == 4
    return points


def main():
    if len(sys.argv) != 1:
        raise SystemExit("This check reads only the sealed v0.1.67 development fixture.")
    assert sha256(BASE_REPORT.read_bytes()).hexdigest() == PREVIOUS_REPORT_SHA
    fixture = json.loads(BASE_REPORT.read_text())
    palette = fixture["outlined"]["palette"]
    idle, focus, error = (tuple(palette[key][:3]) for key in ("on_surface_variant", "primary", "error"))
    dark = fixture["compact"]["palette"]
    dark_idle, dark_focus = (tuple(dark[key][:3]) for key in ("on_surface_variant", "primary"))
    focus_mid = sample(pose(idle), focus, 44)
    dark_mid = from_lab(sample(pose(dark_idle), dark_focus, 44)[0])
    error_mid = from_lab(sample(focus_mid, error, 44)[0])
    reset_error_mid = from_lab(sample(pose(from_lab(focus_mid[0])), error, 44)[0])
    first = from_lab(sample(pose(idle), focus, 14)[0])
    assert first == (79, 73, 92) and from_lab(focus_mid[0]) == (94, 81, 127)
    assert error_mid != reset_error_mid
    assert dark_mid != dark_idle and dark_mid != dark_focus
    print(json.dumps({
        "scope": "golden math and legacy endpoint probe geometry only",
        "light_14ms": first, "light_44ms": from_lab(focus_mid[0]),
        "light_error_44ms_with_velocity": error_mid, "light_error_44ms_fresh_negative": reset_error_mid,
        "light_clearing_44ms": from_lab(sample(pose(error), focus, 44)[0]),
        "dark_44ms": dark_mid,
        "legacy_outlined_four_solid_pixels": calibrate(fixture["outlined"]["frames"][-1], palette["primary"]),
        "legacy_filled_four_solid_pixels": calibrate(fixture["filled"]["frames"][-1], palette["primary"]),
    }, indent=2))


if __name__ == "__main__":
    main()
