#!/usr/bin/env python3
"""Synthetic calibration for the offline geometry and decoded pixels."""

from copy import deepcopy
import json
from pathlib import Path
from runpy import run_path
from tempfile import TemporaryDirectory

from PIL import Image

p = run_path(str(Path(__file__).with_name("replay-vertical-menu.py")))
groups, commands = p["GROUPS"], p["COMMANDS"]


def box(x, y, width, height):
    return dict(x=x, y=y, width=width, height=height)


def fixture(mode, shift=0):
    profile = p["PROFILES"][mode]
    compact = profile.compact
    step = 0 if compact else 2
    y = 8 if compact else 148
    trigger = box(55, 392 if compact else 96, 140, 60 if compact else 48)
    spans = (3, 3, 1) if profile.groups == 3 else (7,)
    heights = [88 if compact and row == p["RENAME"] else 56 if compact else 48 for row in commands]
    records, owners = {}, {}
    content_y = y - shift
    row_index = 0
    for group, length in zip(groups[:profile.groups], spans, strict=True):
        start_y = content_y
        header = (40 if compact else 28) if profile.groups == 3 else 0
        content_y += 4 + header
        for row, height in zip(commands[row_index:row_index + length], heights[row_index:row_index + length], strict=True):
            records[row] = {"bounds": box(54, content_y, 232, height)}
            content_y += height
        row_index += length
        content_y += 4
        owners[group] = box(50, start_y, 240, content_y - start_y)
        content_y += step
    content = box(50, y - shift, 240, content_y - step - (y - shift))
    panel = box(50, y, 240, 380 if compact else content["height"])
    for record in records.values():
        record["clip"] = panel
    thumb = box(52, 12 + shift / 2, 4, 170) if compact else None
    return {"trigger": trigger, "panel": panel, "viewport": panel, "contents": content,
            "groups": owners, "commands": records, "thumb": thumb}


def reject(test):
    try:
        test()
    except RuntimeError:
        return True
    raise RuntimeError("The verifier accepted an altered synthetic input")


def main():
    checks = {}
    for mode in p["PROFILES"]:
        data = fixture(mode)
        profile = p["PROFILES"][mode]
        p["verify_geometry"](data, profile, p["FIRST"])
    before, after = fixture("compact"), fixture("compact", 186)
    profile = p["PROFILES"]["compact"]
    p["verify_geometry"](after, profile, p["LAST"])
    assert p["verify_scroll"](before, after) == 186
    altered = deepcopy(after)
    altered["thumb"]["y"] = before["thumb"]["y"]
    checks["stationary_thumb"] = reject(lambda: p["verify_scroll"](before, altered))
    altered = deepcopy(after)
    altered["commands"][p["LAST"]]["clip"] = box(0, 0, 10, 10)
    checks["out_of_view_command"] = reject(lambda: p["verify_geometry"](altered, profile, p["LAST"]))
    altered = deepcopy(before)
    altered["groups"][groups[-1]]["y"] += 2
    checks["compact_group_gap"] = reject(lambda: p["verify_geometry"](altered, profile, p["FIRST"]))

    with TemporaryDirectory(prefix="q75-menu-control-") as tmp:
        path = Path(tmp) / "frame.png"
        fg, surface, page, ring = ([237, 218, 238, 255], [74, 48, 78, 255], [22, 18, 27, 255], [207, 190, 208, 255])
        row = after["commands"][p["LAST"]]["bounds"]
        art = {"group": groups[-1], "focused": p["LAST"], "surface": surface, "page": page, "ink": fg,
               "ring": ring, "thumb": fg, "icon": None, "icon_ink": None,
               "label": box(row["x"] + 24, row["y"] + 16, 60, 24)}
        im = Image.new("RGBA", (profile.width, profile.height), tuple(page))
        im.putpixel((288, int(row["y"] + row["height"] / 2)), tuple(surface))
        mid = int(row["x"] + row["width"] / 2)
        for x in range(mid - 5, mid + 6):
            im.putpixel((x, int(row["y"])), tuple(ring))
        for dx in range(20):
            im.putpixel((int(art["label"]["x"]) + dx, int(art["label"]["y"])), tuple(fg))
        thumb = after["thumb"]
        thumb_sample = (int(thumb["x"] + 2), int(thumb["y"] + thumb["height"] / 2))
        im.putpixel(thumb_sample, tuple(fg))
        im.save(path)
        assert p["paint"](path, profile, after, art)["label_pixels"] == 20
        im.putpixel(thumb_sample, tuple(surface))
        im.save(path)
        checks["invisible_thumb"] = reject(lambda: p["paint"](path, profile, after, art))
        im.putpixel(thumb_sample, tuple(fg))
        im.putpixel((288, int(row["y"] + row["height"] / 2)), tuple(fg))
        im.save(path)
        checks["wrong_surface"] = reject(lambda: p["paint"](path, profile, after, art))
    print(json.dumps({"scope": "synthetic offline controls", "rejected": checks}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
