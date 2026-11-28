"""Shared native Gallery replay checks."""
import math

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focused, same_box

SHOT = {"scale": 1, "max_side": 1200}


def find(frame, target):
    return next(
        ((i, node) for i, node in enumerate(frame["dom"]["nodes"]) if node.get("stable_id") == target),
        (None, None),
    )


def node(frame, target):
    _, match = find(frame, target)
    require(match is not None, f"Missing DOM node {target}")
    return match


def layout(frame, target):
    index, match = find(frame, target)
    require(match is not None and frame.get("layout") is not None, f"Missing layout for {target}")
    return frame["layout"]["nodes"][index]


def settle(app):
    prior = app.dom()
    for _ in range(24):
        current = app.dom()
        if prior == current:
            return current
        prior = current
    raise RuntimeError("The Gallery layout did not settle")


def activate(app, target):
    app.activate(target)
    return settle(app)


def key(app, name, *, reverse=False):
    app.wire("Action.key", {"key": name, "extend": reverse})
    return settle(app)


def focus(frame, target):
    require(focused(frame) == target, f"Expected focus {target}. Got {focused(frame)}")


def visible(frame, target, minimum=1.):
    measured = layout(frame, target)
    bounds = measured["bounds"]
    require(bounds["width"] >= minimum and bounds["height"] >= minimum, f"Small target {target}: {bounds}")
    require(same_box(measured["clip"], bounds), f"Clipped target {target}: {measured}")
    return bounds


def ink_sample(frame, rows, target):
    bounds = visible(frame, target)
    color = bytes(node(frame, target)["style"]["foreground"])
    left = max(0, math.ceil(bounds["x"]))
    right = min(len(rows[0]) // 4, math.floor(bounds["x"] + bounds["width"]))
    top = max(0, math.ceil(bounds["y"]))
    bottom = min(len(rows), math.floor(bounds["y"] + bounds["height"]))
    tolerance = 64

    def near_ink(pixel):
        return pixel[3] == color[3] and max(abs(a - b) for a, b in zip(pixel[:3], color[:3])) <= tolerance

    points = [(x, y) for y in range(top, bottom) for x in range(left, right)
              if near_ink(rows[y][4*x:4*x+4])]
    require(len(points) >= 16, f"No readable ink in {target}")
    xs, ys = zip(*points)
    ink = [min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1]
    require(ink[2] >= 4 and ink[3] >= 6, f"Too little visible ink in {target}: {ink}")
    return {"bounds": bounds, "ink_bounds": ink, "rgba": list(color),
            "rgb_tolerance": tolerance, "ink_pixels": len(points)}


def capture(app, plan, title, *, frame=None, sample_targets=()):
    path = plan.out / f"{title}.png"
    shot = SHOT.copy()
    viewport_dimensions = None
    if frame is not None:
        viewport = frame["layout"]["viewport"]
        viewport_dimensions = (int(viewport["width"]), int(viewport["height"]))
        shot["max_side"] = max(viewport_dimensions)
    cli = app.cli(["page", "screenshot", "--scale", "1", "--max-side",
                   str(shot["max_side"]), "--out", path])
    api = app.wire("Page.screenshot", shot)
    mcp, mcp_metadata, _ = app.mcp(shot)
    encoded = path.read_bytes()
    require(encoded == png_payload(api) == mcp, f"PNG bytes differ for {title}")
    require(metadata(cli) == metadata(api) == mcp_metadata, f"PNG metadata differs for {title}")
    width, height, rows = read_png(encoded)
    require((width, height) == (cli["width"], cli["height"]), "Decoded PNG dimensions disagree")
    if viewport_dimensions is not None:
        require((width, height) == viewport_dimensions and cli["scale"] == 1,
                "Divider pixel proof needs one output pixel per logical unit")
    result = {"name": path.name, "sha256": digest(path), **metadata(cli)}
    if sample_targets:
        result["ink"] = {target: ink_sample(frame, rows, target) for target in sample_targets}
    return result
