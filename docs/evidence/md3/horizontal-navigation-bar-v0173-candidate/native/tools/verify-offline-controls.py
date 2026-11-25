#!/usr/bin/env python3
"""Exercise the replay verifier with synthetic inputs only."""

from copy import deepcopy
import json
from pathlib import Path
from runpy import run_path
import struct
from tempfile import TemporaryDirectory
import zlib

replay = run_path(str(Path(__file__).with_name("replay-horizontal-nav.py")))
profiles = replay["PROFILES"]
items = replay["ITEMS"]
labels = replay["LABELS"]


def box(x, y, width, height):
    return dict(x=x, y=y, width=width, height=height)


def sample_layout(profile, count):
    width, height, rtl, _, _ = profiles[profile]
    bar_height = 104 if rtl else 64
    top = height - bar_height
    layout = {"preview": box(0, 0, width, height), "scroll": box(0, 80, width, top - 80),
              "bar": box(0, top, width, bar_height), "items": {}}
    start = (width - 112 * count) / 2
    for i, item in enumerate(items[:count]):
        col = count - i - 1 if rtl else i
        hit_x = start + 112 * col
        text = labels[i]
        label_width = min(52, len(text) * (24 if rtl else 12))
        label_height = 64 if rtl and len(text) > 2 else 32 if rtl else 16
        pill_width = label_width + 60
        pill_height = max(24, label_height) + 16
        pill_x = hit_x + (112 - pill_width) / 2
        mid = top + bar_height / 2
        icon_x = pill_x + pill_width - 40 if rtl else pill_x + 16
        label_x = pill_x + 16 if rtl else icon_x + 28
        layout["items"][item] = {"hit": box(hit_x, top, 112, bar_height),
                                "pill": box(pill_x, mid - pill_height / 2, pill_width, pill_height),
                                "icon": box(icon_x, mid - 12, 24, 24),
                                "label": box(label_x, mid - label_height / 2, label_width, label_height),
                                "text": text}
    return layout


def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


def image(path, profile, layout, palette, erase=None):
    width, height, _, current, _ = profiles[profile]
    bar, fill, ink, focus = (bytes(palette[part]) for part in ("bar", "fill", "ink", "focus"))
    rows = [bytearray(bar * width) for _ in range(height)]

    def put(x, y, color):
        ix, iy = int(x), int(y)
        rows[iy][ix * 4:ix * 4 + 4] = color

    item = layout["items"][current]
    pill = item["pill"]
    if erase != "face":
        put(pill["x"] + 6, pill["y"] + pill["height"] / 2, fill)
    if erase == "corner":
        put(pill["x"] + 5, pill["y"] + 2, fill)
    hit = layout["items"][replay["AFTER_SELECTED"][current]]["hit"]
    if erase != "focus":
        for offset in range(3):
            put(hit["x"] + hit["width"] / 2, hit["y"] + offset, focus)
    for part in ("icon", "label"):
        if part != erase:
            for i in range(6):
                for j in range(6):
                    put(item[part]["x"] + i + 2, item[part]["y"] + j + 2, ink)
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    payload = zlib.compress(b"".join(b"\0" + row for row in rows))
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header)
                     + chunk(b"IDAT", payload) + chunk(b"IEND", b""))


def rejects(action):
    try:
        action()
    except RuntimeError:
        return True
    raise RuntimeError("Verifier accepted corrupted synthetic input")


def main():
    checks = {}
    for profile in profiles:
        layout = sample_layout(profile, 5)
        width, _, rtl, current, _ = profiles[profile]
        for count in (3, 4, 5):
            replay["verify_layout"](sample_layout(profile, count), profile, count)
        wrong = deepcopy(layout)
        wrong["items"][current]["hit"]["width"] = 113
        checks[f"{profile}_target_width"] = rejects(lambda: replay["verify_layout"](wrong, profile, 5))
        wrong = deepcopy(layout)
        wrong["items"][current]["hit"]["x"] = 0 if rtl else width
        checks[f"{profile}_cell_order"] = rejects(lambda: replay["verify_layout"](wrong, profile, 5))
        wrong = replay["expected_semantics"](5, current)
        wrong["destinations"][replay["HOME"]]["current_page"] = True
        checks[f"{profile}_duplicate_current"] = rejects(
            lambda: replay["verify_semantics"](wrong, 5, current))
        with TemporaryDirectory(prefix="surface-nav-offline-") as tmp:
            path = Path(tmp) / "fixture.png"
            palette = {"bar": [233, 231, 245, 255], "fill": [194, 188, 229, 255],
                       "ink": [22, 17, 50, 255], "focus": [41, 41, 77, 255]}
            image(path, profile, layout, palette)
            assert replay["paint"](path, profile, layout, palette)["label_pixels"] == 36
            for defect in ("face", "corner", "icon", "label", "focus"):
                image(path, profile, layout, palette, erase=defect)
                checks[f"{profile}_missing_{defect}"] = rejects(
                    lambda: replay["paint"](path, profile, layout, palette))
    print(json.dumps({"scope": "synthetic verifier controls", "rejections": checks}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
