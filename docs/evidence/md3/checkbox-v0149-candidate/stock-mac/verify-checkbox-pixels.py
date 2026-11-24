"""Independently decode the four original Checkbox stock images."""

from hashlib import file_digest
import json
from math import floor
from pathlib import Path
import sys

from PIL import Image


ART = "material-checkbox-"


def need(condition, description):
    if not condition:
        raise ValueError(description)


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def read(path):
    return json.loads(path.read_text())


def color(state, role):
    rgba = state["palette"][role]
    need(len(rgba) == 4 and rgba[3] == 255, f"palette role {role} is not opaque")
    return tuple(rgba[:3])


def center(rect):
    x, y, w, h = rect
    return x + w / 2, y + h / 2


def inside(outer, inner):
    x, y, w, h = outer
    ix, iy, iw, ih = inner
    return ix >= x and iy >= y and ix + iw <= x + w and iy + ih <= y + h


def point(x, y):
    return floor(x * 2), floor((y + 44) * 2)


def pixels(image, bounds, ink):
    x, y, w, h = bounds
    def edge(px, py):
        return floor(px * 2 + 0.5), floor((py + 44) * 2 + 0.5)

    raster = image.crop((*edge(x, y), *edge(x + w, y + h)))
    return sum(count for count, rgb in raster.getcolors(raster.width * raster.height) if rgb == ink)


def valid_geometry(layout, *, rtl=False):
    row, art, mark, layer, ring, label = (layout[key] for key in ("row", "art", "mark", "layer", "ring", "label"))
    need(art[2:] == [18, 18] and mark == art and layer[2:] == [40, 40] and ring[2:] == [50, 50],
         "wrong Checkbox mark, box, state layer or focus bounds")
    need(center(art) == center(layer) == center(ring) and min(row[2:]) >= 48
         and inside(row, label) and inside(row, ring), "box, focus and text escaped the row")
    if rtl:
        need(row[0] + row[2] == ring[0] + ring[2] and label[0] + label[2] <= ring[0], "RTL artwork escaped its edge")
    else:
        need(row[0] == ring[0] and ring[0] + ring[2] <= label[0], "LTR artwork escaped its edge")


def main(run):
    stock = read(run / "verified-checkbox-stock.json")
    expect = stock["checkbox_pngs"]
    artifacts = run / "artifacts"
    path = artifacts / "browser-material-checkbox.json"
    raw = read(path)
    need(digest(path) == stock["checkbox_report_sha256"]
         and raw["errors"] == [] and stock["cases"]["material"]["steps"] == 280, "unmatched stock receipt")
    images = {}
    for name in ("wide-focused", "wide-held", "compact-invalid", "compact-resolved"):
        filename = ART + name + ".png"
        original = artifacts / filename
        need(digest(original) == expect[filename]["sha256"], f"raw bytes of {filename} changed")
        with Image.open(original) as encoded:
            encoded.load()
            need(list(encoded.size) == expect[filename]["size"], f"wrong dimensions in {filename}")
            images[name] = encoded.convert("RGB")
    keyboard = raw["keyboard"]
    need(keyboard["space"]["state"]["requests"] == 1
         and keyboard["enter"]["state"]["requests"] == 2, "keyboard action count changed")
    wide = raw["accessibility"]
    need(len(wide["layouts"]) == len(wide["nodes"]) == 5, "missing wide controls")
    for layout in wide["layouts"]:
        valid_geometry(layout)
    held = raw["pointer"]["pointer_held"]
    valid_geometry(held["layout"])
    need(held["state"]["requests"] == raw["pointer"]["mouse_release_outside"]["requests"]
         == raw["pointer"]["touch_cancel"]["requests"] == 3, "cancelled contact changed the request count")
    mixed = wide["layouts"][0]
    normal = images["wide-focused"]
    x, y = center(mixed["art"])
    samples = {
        "mixed_fill": normal.getpixel(point(x, y - 5)),
        "mixed_mark": normal.getpixel(point(x, y)),
        "focus_ring": normal.getpixel(point(x, mixed["ring"][1] + 1.5)),
        "focus_layer": normal.getpixel(point(x + 14, y)),
        "focus_background": normal.getpixel(point(x + 27, y)),
    }
    need(all(list(rgb) == wide["paint"][role] for role, rgb in samples.items())
         and samples["mixed_fill"] == color(wide["state"], "primary")
         and samples["mixed_mark"] == color(wide["state"], "on_primary")
         and samples["focus_ring"] == color(wide["state"], "secondary"),
         "decoded mixed, focus or layer pixels did not match the receipt")
    hx, hy = center(held["layout"]["art"])
    press = images["wide-held"].getpixel(point(hx + 14, hy))
    need(list(press) == held["layer_rgb"] and press != samples["focus_layer"], "held layer did not change")

    compact = raw["compact"]
    invalid, resolved = compact["invalid"], compact["resolved"]
    for status in (invalid, resolved):
        valid_geometry(status["layout"], rtl=True)
        need(inside(compact["clip"], status["layout"]["row"]), "compact layout escaped the clip")
    need(inside(compact["clip"], invalid["help"]) and invalid["layout"]["label"][3] >= 96,
         "compact help or wrapped label is clipped")
    invalid_semantics, resolved_semantics = invalid["semantics"], resolved["semantics"]
    need(invalid_semantics["chrome_invalid"] is True and invalid_semantics["chrome_description"] == "請勾選後再繼續。"
         and resolved_semantics["chrome_invalid"] is False and resolved_semantics["chrome_description"] == ""
         and resolved["host"] == "surface" and resolved["state"]["requests"] == 5
         and compact["focus_restored"] == "checkbox-demo-show", "checked error retirement or focus mismatch")
    error_image = images["compact-invalid"]
    error_ink = color(invalid["state"], "error")
    ix, _ = center(invalid["layout"]["art"])
    need(error_image.getpixel(point(ix, invalid["layout"]["art"][1] + 1)) == error_ink,
         "decoded invalid box lacks the Error stroke")
    help_pixels = pixels(error_image, invalid["help"], error_ink)
    need(help_pixels == invalid["help_ink_pixels"] and help_pixels > 24, "actual error help ink mismatch")
    lx, ly, lw, _ = invalid["layout"]["label"]
    lines = [pixels(error_image, [lx, ly + line * 48, lw, 48], color(invalid["state"], "on_surface"))
             for line in range(2)]
    need(lines == invalid["line_ink_pixels"] and min(lines) > 24, "actual wrapped label ink mismatch")
    final_image = images["compact-resolved"]
    ax, ay = center(resolved["layout"]["art"])
    need(final_image.getpixel(point(ax, ay - 5)) == color(resolved["state"], "primary"),
         "decoded checked box lacks the Primary fill")
    tick_pixels = pixels(final_image, resolved["layout"]["art"], color(resolved["state"], "on_primary"))
    need(tick_pixels == resolved["check_ink_pixels"] and tick_pixels > 16, "actual checked mark ink mismatch")
    return {"report_sha256": digest(path), "stock_receipt_sha256": digest(run / "verified-checkbox-stock.json"),
            "browser": raw["browser"], "adapter": raw["adapter"], "wide_colors": samples, "held_layer": press,
            "compact_help_pixels": help_pixels, "compact_label_pixels": lines, "checked_mark_pixels": tick_pixels,
            "keyboard_requests": 2, "final_requests": 5, "pngs": expect}


if __name__ == "__main__":
    print(json.dumps(main(Path(sys.argv[1]).resolve()), indent=2))
