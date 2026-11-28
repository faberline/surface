#!/usr/bin/env python3
"""Test verifier decisions with synthetic bytes, without starting a Gallery."""

from copy import deepcopy
import json
import math
from pathlib import Path
from runpy import run_path
from tempfile import TemporaryDirectory

from PIL import Image

p = run_path(str(Path(__file__).with_name("replay-vertical-choice-menu.py")))


def box(x, y, width, height):
    return dict(x=x, y=y, width=width, height=height)


def fixture(profile, end=False):
    compact = profile.compact
    x, y = 50, 8 if compact else 148
    gutter = 0 if compact else 2
    header = 40 if compact else 28
    heights = {name: 56 if compact else 48 for name in p["LABELS"]}
    if compact:
        heights[p["SIZE"]] = 88
    content_h = sum(heights.values()) + 3 * (header + 8) + 2 * gutter
    viewport_h = 380 if compact else content_h
    offset = content_h - viewport_h if end else 0
    viewport = box(x, y, 240, viewport_h)
    origin, items, groups = y - offset, {}, {}
    for group, members in p["MEMBERS"].items():
        group_h = header + sum(heights[name] for name in members) + 8
        groups[group] = box(x, origin, 240, group_h)
        row_y = origin + 4 + header
        for name in members:
            items[name] = {"bounds": box(x + 4, row_y, 232, heights[name]), "clip": viewport}
            row_y += heights[name]
        origin += group_h + gutter
    thumb = None
    if compact:
        h = round((viewport_h - 8) * viewport_h / content_h)
        thumb = box(x + 2, y + 4 + (viewport_h - 8 - h if end else 0), 4, h)
    return {"trigger": box(55, y + viewport_h + 4 if compact else 96, 140, 48),
            "panel": viewport, "viewport": viewport, "contents": box(x, y - offset, 240, content_h),
            "groups": groups, "items": items, "thumb": thumb}


def semantics():
    return {"menu": ["Menu", p["TITLE"]], "groups": {k: ["Group", v] for k, v in p["GROUPS"].items()},
            "items": {name: {"role": "MenuItem", "name": label, "disabled": name in p["DISABLED"],
                             "owner": p["OWNERS"][name], "checked": name in p["INITIAL"], "radio": name in p["RADIOS"]}
                      for name, label in p["LABELS"].items()}}


def reject(test):
    try:
        test()
    except RuntimeError:
        return True
    raise AssertionError("The verifier accepted an altered synthetic input")


def paint_control(directory, profile, geometry, rejected):
    focus, selected, unchecked = p["LOCKED"], p["SIZE"], p["KIND"]
    mark_ink, fill, page, ring = (61, 31, 51, 255), (239, 184, 200, 255), (22, 18, 27, 255), (207, 190, 208, 255)
    target = geometry["items"][focus]["bounds"]
    face = geometry["items"][selected]["bounds"]
    empty = geometry["items"][unchecked]["bounds"]
    mid = lambda rect: rect["y"] + rect["height"] // 2
    mark = box(p["right"](face) - 36, mid(face) - 10, 20, 20)
    baseline = box(p["right"](empty) - 36, mid(empty) - 10, 20, 20)
    label = box(face["x"] + 16, mid(face) - 22, 150, 44)
    thumb = geometry["thumb"]
    art = {"focus": focus, "selected": selected, "unchecked": unchecked, "ring": ring,
           "selected_face": face, "unchecked_face": empty,
           "selected_fill": fill, "unchecked_fill": (0, 0, 0, 0), "selected_label": label,
           "mark": mark, "mark_ink": mark_ink, "label_ink": mark_ink, "empty_mark": baseline,
           "surface": profile.surface, "page": page, "thumb_ink": ring}
    png = directory / "synthetic.png"
    im = Image.new("RGBA", (profile.width, profile.height), profile.surface)
    im.putpixel((2, 2), page)
    selected_sample = (int(face["x"] + 6), int(mid(face)))
    im.putpixel(selected_sample, fill)
    for y in range(2):
        for x in range(8):
            im.putpixel((int(mark["x"] + x), int(mark["y"] + y)), mark_ink)
            im.putpixel((int(label["x"] + x), int(label["y"] + y)), mark_ink)
    x_middle = math.floor(target["x"] + target["width"] / 2)
    for x in range(x_middle - 5, x_middle + 6):
        im.putpixel((x, int(target["y"])), ring)
    thumb_sample = (int(thumb["x"] + 2), int(mid(thumb)))
    im.putpixel(thumb_sample, ring)
    im.save(png)
    stats = p["paint"](png, profile, geometry, art)
    assert stats["mark_pixels"] == stats["label_pixels"] == 16 and stats["empty_mark_pixels"] == 0

    im.putpixel((int(baseline["x"]), int(baseline["y"])), mark_ink)
    im.save(png)
    rejected["ghost_check"] = reject(lambda: p["paint"](png, profile, geometry, art))
    im.putpixel((int(baseline["x"]), int(baseline["y"])), profile.surface)
    im.putpixel(thumb_sample, profile.surface)
    im.save(png)
    rejected["invisible_thumb"] = reject(lambda: p["paint"](png, profile, geometry, art))
    im.putpixel(thumb_sample, ring)
    im.putpixel(selected_sample, profile.surface)
    im.save(png)
    rejected["wrong_selected_fill"] = reject(lambda: p["paint"](png, profile, geometry, art))


def main():
    rejected = {}
    expected = semantics()
    p["verify_semantics"](expected, p["INITIAL"])
    changed = deepcopy(expected)
    changed["items"][p["NAME"]]["checked"] = True
    rejected["two_checked_radios"] = reject(lambda: p["verify_semantics"](changed, p["INITIAL"]))
    changed = deepcopy(expected)
    changed["items"][p["KIND"]]["radio"] = True
    rejected["checkbox_as_radio"] = reject(lambda: p["verify_semantics"](changed, p["INITIAL"]))
    changed = deepcopy(expected)
    changed["items"][p["COMFORTABLE"]]["owner"] = p["OWNERS"][p["RECENT"]]
    rejected["merged_groups"] = reject(lambda: p["verify_semantics"](changed, p["INITIAL"]))
    for profile in p["PROFILES"].values():
        p["verify_geometry"](fixture(profile), profile, p["RECENT"])
    compact = p["PROFILES"]["compact"]
    before, after = fixture(compact), fixture(compact, end=True)
    p["verify_geometry"](after, compact, p["LOCKED"])
    assert p["verify_scroll"](before, after) == before["contents"]["height"] - before["viewport"]["height"]
    changed = deepcopy(after)
    changed["thumb"]["y"] = before["thumb"]["y"]
    rejected["stationary_thumb"] = reject(lambda: p["verify_scroll"](before, changed))
    changed = deepcopy(after)
    changed["items"][p["LOCKED"]]["clip"] = box(0, 0, 1, 1)
    rejected["unrevealed_last_choice"] = reject(lambda: p["verify_geometry"](changed, compact, p["LOCKED"]))
    with TemporaryDirectory(prefix="choice-menu-offline-controls-") as tmp:
        paint_control(Path(tmp), compact, after, rejected)
    print(json.dumps({"scope": "synthetic checks only", "positive_profiles": list(p["PROFILES"]),
                      "rejected": rejected}, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
