"""Read painted hint and caret at one captured Search frame."""

import math

from gallery_transport import read_png, require


def combined_editor(path, editor, foreground, hint, rtl):
    _, _, rows = read_png(path.read_bytes())
    require(abs(hint["mid_y"] - (editor["y"] + editor["height"] / 2)) <= 4,
            "The hint lost vertical centering during motion")
    right = editor["x"] + editor["width"]
    gap = right - hint["right"] - 1 if rtl else hint["left"] - editor["x"]
    require(2 <= gap <= 18, f"The hint lost its logical start inset: {gap}")
    caret_x = round(right - 6 if rtl else editor["x"] + 4)
    bounds = (math.ceil(editor["y"] + 6), math.floor(editor["y"] + editor["height"] - 6))
    best = (0, 0, 0)
    for x in range(caret_x - 1, caret_x + 2):
        start = None
        for y in range(bounds[0], bounds[1] + 1):
            if y < bounds[1] and rows[y][4*x:4*x+4] == bytes(foreground):
                start = y if start is None else start
            elif start is not None:
                if y - start > best[2]:
                    best = (x, start, y - start)
                start = None
    x, y, height = best
    require(height >= 12, f"No painted caret at logical start x={caret_x} in {path.name}")
    require(abs(y + height / 2 - (editor["y"] + editor["height"] / 2)) <= 5,
            "The caret lost vertical centering during motion")
    return {"editor": editor, "hint_ink": hint, "logical_hint_gap": gap,
            "caret_ink": {"x": x, "y": y, "height": height, "rgba": foreground}}
