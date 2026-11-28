import math
def color(palette, token):
    rgba = palette[token]
    require(len(rgba) == 4 and rgba[3] == 255 and all(type(n) is int and 0 <= n <= 255 for n in rgba),
            f"{token}: invalid opaque theme color")
    return tuple(rgba[:3])

def image_for(path, compact):
    require(path.is_file() and not path.is_symlink(), f"{path.name}: no original")
    expected = (640, 2008) if compact else (1920, 1728)
    with Image.open(path) as original:
        require(original.format == "PNG" and original.size == expected,
                f"{path.name}: expected original PNG {expected}")
        original.verify()
    with Image.open(path) as original:
        return original.convert("RGB")

def colors(under, over, alpha, linear):
    alpha = math.floor(alpha * 255 + .5) / 255

    def light(byte):
        unit = byte / 255
        return unit if not linear else unit / 12.92 if unit <= .04045 else ((unit + .055) / 1.055) ** 2.4

    def encode(n):
        srgb = n if not linear else n * 12.92 if n <= .0031308 else 1.055 * n ** (1 / 2.4) - .055
        return math.floor(srgb * 255 + .5)

    return tuple(encode(light(a) * (1 - alpha) + light(b) * alpha) for a, b in zip(under, over, strict=True))

rgb = color
def blend(start, target, ms):
    return ORACLE["from_lab"](ORACLE["sample"](ORACLE["pose"](start), target, ms)[0])
