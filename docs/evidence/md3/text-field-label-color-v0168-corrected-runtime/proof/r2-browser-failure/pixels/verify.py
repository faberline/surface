"""Replay q68 M350's original-pixel comparison without a browser."""

import hashlib
import json
from pathlib import Path

from PIL import Image, ImageChops

SHA = {
    "fresh-rest": "bdcebb405b6382408a432e7d47a63d589febebf45043b8e7a5e6cd792a57b4fc",
    "fresh-middle": "2a96b671de3fe8a4d9eca515ad39afbfd73cd31c266fe121d669e2f160f639f4",
    "fresh-failure": "2a96b671de3fe8a4d9eca515ad39afbfd73cd31c266fe121d669e2f160f639f4",
    "historical-rest": "bdcebb405b6382408a432e7d47a63d589febebf45043b8e7a5e6cd792a57b4fc",
    "historical-middle": "e73d42ca05c7f1cc8ad251ba58b52a5a49800c0bcac3b35ba4a933dd83bb3f14",
    "historical-settled": "67f7aa634920a0bcf5d97fcc17ca5458ae60aa82a9fbbbd7147dad68d1a14e15",
}
ABOVE = (80, 1544, 560, 1736)
READ_ONLY = (80, 1092, 560, 1544)
PROBES = ((84, 1562), (90, 1562), (96, 1562), (102, 1569))
IDLE = (202, 196, 207)
MIDDLE = (206, 191, 243)
FOCUS = (207, 189, 254)

root = Path(__file__).resolve().parent
frames = {}
for name, sha in SHA.items():
    path = root / f"{name}.png"
    assert hashlib.sha256(path.read_bytes()).hexdigest() == sha, f"{name} digest differs"
    with Image.open(path) as image:
        assert image.size == (640, 2008)
        frames[name] = image.convert("RGB")

fresh = frames["fresh-middle"]
rest = frames["fresh-rest"]
historical = frames["historical-middle"]
assert ImageChops.difference(fresh.crop(ABOVE), rest.crop(ABOVE)).getbbox() is None
assert fresh.getpixel((320, 1744)) == historical.getpixel((320, 1744)) == FOCUS
assert rest.getpixel((320, 1744)) == (148, 143, 153)

def mask(image, box, rgb):
    return {i for i, value in enumerate(image.crop(box).get_flattened_data()) if value == rgb}

old_middle = mask(historical, ABOVE, MIDDLE)
assert old_middle == mask(rest, ABOVE, IDLE) == mask(fresh, ABOVE, IDLE)
assert len(old_middle) == 12125 and not mask(fresh, ABOVE, MIDDLE)
assert all(historical.getpixel(p) == MIDDLE and fresh.getpixel(p) == IDLE for p in PROBES)
former_focus = mask(fresh, READ_ONLY, MIDDLE)
assert len(former_focus) == 2289 and former_focus == mask(historical, READ_ONLY, IDLE)
print(json.dumps({"above_label_region_device_pixels": ABOVE, "probe_device_pixels": PROBES,
                  "same_glyph_positions": len(old_middle), "historical_mid_colour": MIDDLE,
                  "observed_mid_colour": IDLE, "focus_outline_is_primary": FOCUS,
                  "former_readonly_field_focus_mid_pixels": len(former_focus)}, indent=2))
