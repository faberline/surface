#!/usr/bin/env python3
"""Replay the Material 3 carousel gallery on the native GPU without a window."""
import argparse
import json
from pathlib import Path
import time

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box

ROOT = "carousel-demo-"
TRACK = "carousel-track"
VARIANTS = ("multi-browse", "uncontained", "mixed", "hero-start", "hero-center", "full-screen")
SCREENSHOT = {"scale": 1, "max_side": 1200}


def find(frame, stable_id):
    for index, node in enumerate(frame["dom"]["nodes"]):
        if node.get("stable_id") == stable_id:
            return node, frame["layout"]["nodes"][index]
    return None, None


def card(index):
    return f"carousel-card-{index}"


def visible(geometry):
    if geometry is None:
        return False
    bounds, clip = geometry["bounds"], geometry["clip"]
    return all(min(bounds[axis] + bounds[size], clip[axis] + clip[size])
               > max(bounds[axis], clip[axis]) for axis, size in (("x", "width"), ("y", "height")))


def until(app, condition, message):
    deadline = time.monotonic() + 5
    while True:
        frame = app.dom()
        if condition(frame):
            return frame
        require(time.monotonic() < deadline, message)
        time.sleep(0.03)


def expect_focus(app, key):
    return until(app, lambda frame: focused(frame) == key, f"Focus did not reach {key}")


def expect_item(app, index):
    return until(app, lambda frame: find(frame, "carousel-position")[0]["text"]
                 == f"{index + 1} / 6", f"Carousel did not reach item {index + 1}")


def capture(app, plan, name, frame, index):
    file = plan.out / (name + ".png")
    client = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", file])
    raw = app.wire("Page.screenshot", SCREENSHOT)
    image, mcp_metadata, _ = app.mcp(SCREENSHOT)
    data = file.read_bytes()
    require(data == png_payload(raw) == image, "The API, CLI and MCP returned different pixels")
    require(metadata(client) == metadata(raw) == mcp_metadata, "Screenshot metadata differed")
    width, height, rows = read_png(data)
    require((width, height) == (client["width"], client["height"]), "PNG metadata mismatch")
    track = find(frame, TRACK)[1]["bounds"]
    node, geom = find(frame, card(index))
    bounds = geom["bounds"]
    require(node["role"] == "Button" and f"第 {index + 1} 張，共 6 張" in node["name"],
            f"{node['name']} is missing its stable card position")
    require(visible(geom) and same_box(track, bounds), f"Card {index} has invalid bounds")
    surface = find(frame, ROOT + "page")[0]["style"]["background"][:3]
    clip = geom["clip"]
    left = max(bounds["x"], clip["x"])
    right = min(bounds["x"] + bounds["width"], clip["x"] + clip["width"])
    top = max(bounds["y"], clip["y"])
    bottom = min(bounds["y"] + bounds["height"], clip["y"] + clip["height"])
    samples = []
    for fraction in (0.25, 0.5, 0.75):
        x = int(left + (right - left) * fraction)
        y = int(top + (bottom - top) * fraction)
        samples.append(list(rows[y][x * 4 : x * 4 + 3]))
    require(all(len(sample) == 3 for sample in samples)
            and any(max(abs(a - b) for a, b in zip(sample, surface)) >= 9 for sample in samples),
            f"Card {index} did not paint inside the viewport")
    return {"file": file.name, "sha256": digest(file), "track": track, "bounds": bounds,
            "paint_rgb": samples, "transport_parity": True, **metadata(client)}


def set_fixture(app, compact):
    app.activate("section-1")
    app.activate(ROOT + "show")
    expect_focus(app, ROOT + "back")
    if compact:
        for control in ("carousel-theme", "carousel-direction", "carousel-large-text"):
            app.activate(control)


def desktop(app, plan):
    set_fixture(app, False)
    results = {}
    for variant in VARIANTS:
        if variant != VARIANTS[0]:
            app.activate("carousel-" + variant)
        frame = expect_item(app, 0)
        results[variant] = capture(app, plan, "light-" + variant, frame, 0)
    frame = app.dom()
    require(find(frame, "carousel-show-all")[0] is None, "Full-screen showed a Show All control")
    track, active = find(frame, TRACK)[1]["bounds"], find(frame, card(0))[1]["bounds"]
    require(track == active, "Full-screen activity did not fill its viewport")
    app.activate("carousel-next")
    expect_item(app, 1)
    focus_by_tab(app, card(1))
    app.key("down")
    expect_item(app, 2)
    expect_focus(app, card(2))
    app.key("end")
    expect_item(app, 5)
    expect_focus(app, card(5))
    app.key("home")
    expect_focus(app, card(0))
    app.activate("carousel-multi-browse")
    app.activate("carousel-show-all")
    expect_focus(app, "carousel-show-all-back")
    frame = app.dom()
    require(find(frame, "carousel-list")[0]["role"] == "List", "All cards did not open a list")
    app.activate("carousel-list-4")
    expect_focus(app, "carousel-show-all")
    expect_item(app, 4)
    app.activate(ROOT + "back")
    expect_focus(app, ROOT + "show")
    return results


def compact(app, plan):
    set_fixture(app, True)
    app.activate("carousel-hero-center")
    frame = expect_item(app, 0)
    first = find(frame, card(0))[1]["bounds"]
    second = find(frame, card(1))[1]["bounds"]
    require(first["x"] > second["x"], "RTL card order did not mirror")
    focus_by_tab(app, card(0))
    app.key("left")
    expect_focus(app, card(1))
    frame = expect_item(app, 1)
    for variant in VARIANTS:
        geometry = find(frame, "carousel-" + variant)[1]
        require(same_box(geometry["clip"], geometry["bounds"]),
                f"{variant} control does not fit at 320 dp and 200% text")
    first_result = capture(app, plan, "dark-rtl-200-hero-center", frame, 1)
    app.activate("carousel-mixed")
    expect_item(app, 0)
    focus_by_tab(app, card(0))
    frame = expect_focus(app, card(0))
    return {
        "hero_center": first_result,
        "mixed": capture(app, plan, "dark-rtl-200-mixed", frame, 0),
    }


def main():
    parser = argparse.ArgumentParser()
    for key in ("gallery", "cli", "out"):
        parser.add_argument("--" + key, type=Path, required=True)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True, exist_ok=False)
    receipt = {"source_manifest_sha256": plan.source}
    for name, width, height, run in (
        ("light", 960, 800, desktop),
        ("dark_rtl_200", 320, 580, compact),
    ):
        with launch(plan, name, width, height) as app:
            receipt[name] = run(app, plan)
        print("PASS", name, flush=True)
    (plan.out / "native-carousel-report.json").write_text(
        json.dumps(receipt, ensure_ascii=False, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
