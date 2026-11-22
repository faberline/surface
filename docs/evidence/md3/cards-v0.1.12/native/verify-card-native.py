#!/usr/bin/env python3
"""Replay Material 3 cards through the headless native Gallery."""

import argparse
import ctypes
import json
from pathlib import Path

from gallery_transport import metadata, png_payload, read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box


KINDS = ("elevated", "filled", "outlined")
PRIMARY = "card-primary"
SECONDARY = "card-secondary"
DISABLED = "card-disabled"
NESTED = "card-nested-action"
MOVE = "card-move"
SCREENSHOT = {"scale": 1, "max_side": 1200}
PALETTE = {
    "light": {"surface": (253, 247, 255), "elevated": (248, 242, 250),
              "filled": (230, 224, 233), "outlined": (253, 247, 255)},
    "dark": {"surface": (20, 18, 24), "elevated": (29, 27, 32),
             "filled": (54, 52, 58), "outlined": (20, 18, 24)},
}


def onscreen_windows(pid):
    """Ask CoreGraphics about the live child only. Do not capture the desktop."""
    cg = ctypes.CDLL("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics")
    cf = ctypes.CDLL("/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation")
    pointer = ctypes.c_void_p
    cg.CGWindowListCopyWindowInfo.argtypes = [ctypes.c_uint32, ctypes.c_uint32]
    cg.CGWindowListCopyWindowInfo.restype = pointer
    cf.CFArrayGetCount.argtypes = [pointer]
    cf.CFArrayGetCount.restype = ctypes.c_long
    cf.CFArrayGetValueAtIndex.argtypes = [pointer, ctypes.c_long]
    cf.CFArrayGetValueAtIndex.restype = pointer
    cf.CFDictionaryGetValue.argtypes = [pointer, pointer]
    cf.CFDictionaryGetValue.restype = pointer
    cf.CFNumberGetValue.argtypes = [pointer, ctypes.c_long, pointer]
    cf.CFNumberGetValue.restype = ctypes.c_bool
    cf.CFRelease.argtypes = [pointer]
    owner_key = pointer.in_dll(cg, "kCGWindowOwnerPID")
    windows = cg.CGWindowListCopyWindowInfo(1, 0)
    require(windows, "CoreGraphics did not return a window list")
    try:
        count = 0
        for index in range(cf.CFArrayGetCount(windows)):
            entry = cf.CFArrayGetValueAtIndex(windows, index)
            owner = cf.CFDictionaryGetValue(entry, owner_key)
            value = ctypes.c_int32()
            if owner and cf.CFNumberGetValue(owner, 3, ctypes.byref(value)) and value.value == pid:
                count += 1
        return count
    finally:
        cf.CFRelease(windows)


def find(frame, stable_id):
    for index, node in enumerate(frame["dom"]["nodes"]):
        if node.get("stable_id") == stable_id:
            return node, frame["layout"]["nodes"][index]
    raise RuntimeError(f"No node with stable ID {stable_id}")


def bounds(frame, stable_id):
    return find(frame, stable_id)[1]["bounds"]


def support(frame, card):
    return find(frame, card + "-supporting")[0]["text"]


def expect_focus(app, target):
    frame = app.dom()
    require(focused(frame) == target, f"Expected focus on {target}, got {focused(frame)}")
    return frame


def open_card(app):
    app.activate("section-3")
    app.activate("card-demo-show")
    return expect_focus(app, "card-demo-back")


def near(actual, expected):
    return max(abs(a - b) for a, b in zip(actual, expected)) <= 1


def capture(app, plan, name, frame, theme, kind, focus_ring=False):
    file = plan.out / (name + ".png")
    cli_result = app.cli(["page", "screenshot", "--scale", "1", "--max-side", "1200", "--out", file])
    raw = app.wire("Page.screenshot", SCREENSHOT)
    mcp, mcp_metadata, _ = app.mcp(SCREENSHOT)
    encoded = file.read_bytes()
    require(encoded == png_payload(raw) == mcp, f"{name} API, CLI and MCP PNG bytes differ")
    require(metadata(cli_result) == metadata(raw) == mcp_metadata, f"{name} metadata differs")
    width, height, rows = read_png(encoded)
    require((width, height) == (cli_result["width"], cli_result["height"]), f"{name} bad dimensions")

    def pixel(x, y):
        x, y = int(x), int(y)
        require(0 <= x < width and 0 <= y < height, f"{name} pixel ({x}, {y}) is out of bounds")
        return tuple(rows[y][x * 4:x * 4 + 3])

    primary, geometry = find(frame, PRIMARY)
    box, clip = geometry["bounds"], geometry["clip"]
    require(same_box(clip, box), f"{name} primary card is partially clipped")
    for content in ("title", "supporting", "media"):
        require(same_box(box, bounds(frame, f"{PRIMARY}-{content}")),
                f"{name} primary {content} leaves the card")
    y = box["y"] + box["height"] * 0.5
    x = box["x"]
    surface = PALETTE[theme]["surface"]
    expected_face = PALETTE[theme][kind]
    face = pixel(x + 7, y)
    page = pixel(4, height - 8)
    require(near(page, surface), f"{name} page pixel {page} differs from {surface}")
    require(near(face, expected_face), f"{name} face pixel {face} differs from {expected_face}")
    require(tuple(primary["style"]["background"][:3]) == expected_face,
            f"{name} declared face does not match the palette")

    shadow = [pixel(x - distance, y) for distance in (1, 2, 3, 4)]
    if kind == "elevated":
        require(len(primary.get("box_shadows", [])) == 2, f"{name} does not declare both elevation layers")
        require(any(sum(sample) < sum(surface) - 3 for sample in shadow),
                f"{name} elevation did not darken the surface outside the card: {shadow}")
    else:
        require(not primary.get("box_shadows") and all(near(sample, surface) for sample in shadow),
                f"{name} flat card draws outside its bounds: {shadow}")
    layer = find(frame, PRIMARY + "-state")[0]
    border = pixel(x, y)
    if kind == "outlined":
        expected_border = tuple(layer["style"]["border_color"][:3])
        require(layer["style"]["border_width"] == 1 and not near(expected_border, expected_face),
                f"{name} outline declaration is missing")
        require(near(border, expected_border), f"{name} outline pixel {border} differs from {expected_border}")
    if focus_ring:
        expected_border = tuple(layer["focus_color"][:3])
        border = pixel(x + 1, y)
        require(focused(frame) == PRIMARY and near(border, expected_border),
                f"{name} keyboard focus pixel {border} differs from {expected_border}")
    return {"file": file.name, "sha256": digest(file), **metadata(cli_result),
            "transport_parity": True, "card_bounds": box, "surface_rgb": page,
            "face_rgb": face, "edge_rgb": border, "shadow_rgb": shadow}


def keyboard_journey(app):
    primary, _ = find(app.dom(), PRIMARY)
    passive, _ = find(app.dom(), SECONDARY)
    require(primary["role"] == "Button" and primary.get("checked") is None, "Primary press role changed")
    require(passive["role"] == "Group", "Passive card is an activation target")
    require(same_box(bounds(app.dom(), SECONDARY), bounds(app.dom(), NESTED)),
            "Independent action leaves its passive card")
    app.wire("Action.activate", {"id": SECONDARY}, "action_failed")
    app.activate(PRIMARY)
    pressed = expect_focus(app, PRIMARY)
    require(support(pressed, PRIMARY) == "行程已開啟。", "Primary action did not open the trip")
    nested_tab = focus_by_tab(app, NESTED)
    app.key("enter")
    nested = expect_focus(app, NESTED)
    require(support(nested, SECONDARY) == "已開啟行程草稿。", "Independent Enter did not open its trip")
    require(support(nested, PRIMARY) == support(pressed, PRIMARY), "Nested action reached primary")

    app.activate("card-toggle-mode")
    toggle_tab = focus_by_tab(app, PRIMARY)
    values = [find(expect_focus(app, PRIMARY), PRIMARY)[0]["checked"]]
    for key in ("enter", "space"):
        app.key(key)
        frame = expect_focus(app, PRIMARY)
        values.append(find(frame, PRIMARY)[0]["checked"])
        want = "已選取這段行程。" if key == "enter" else "行程已開啟。"
        require(support(frame, PRIMARY) == want, f"{key} did not apply the controlled toggle")
    require(values == [False, True, False], f"Unexpected toggle sequence {values}")

    move_tab = focus_by_tab(app, MOVE)
    order = []
    for key in (None, "enter", "space"):
        if key:
            app.key(key)
        frame = expect_focus(app, MOVE)
        order.append([bounds(frame, SECONDARY)["y"], bounds(frame, PRIMARY)["y"]])
        require(support(frame, SECONDARY) == "已開啟行程草稿。", "Move changed the independent action")
    require(order[0][0] < order[0][1] and order[1][0] > order[1][1]
            and order[2] == order[0], f"Keyboard Move did not reorder both ways: {order}")

    app.activate("card-disable")
    disabled = [PRIMARY, DISABLED]
    snapshot = app.dom()
    for target in disabled:
        require(find(snapshot, target)[0]["disabled"], f"{target} is enabled")
        app.wire("Action.activate", {"id": target}, "disabled")
        app.cli(["action", "activate", target], want_status=1)
    require(find(app.dom(), PRIMARY)[0]["checked"] is False, "A rejected press changed the toggle")
    start = focused(app.dom())
    tab = []
    for _ in range(30):
        app.key("tab")
        key = focused(app.dom())
        tab.append(key)
        if key == start:
            break
    require(tab[-1] == start and all(key not in tab for key in disabled)
            and all(key in tab for key in (NESTED, MOVE)), f"Unexpected disabled Tab route {tab}")
    app.activate("card-disable")
    require(not find(app.dom(), PRIMARY)[0]["disabled"], "Primary did not re-enable")
    app.activate("card-demo-back")
    expect_focus(app, "card-demo-show")
    return {"toggle_checked": values, "move_y_before_and_after": order,
            "disabled_activation": disabled, "disabled_tab_cycle": tab,
            "nested_tab": nested_tab, "toggle_tab": toggle_tab, "move_tab": move_tab,
            "focus_restored": "card-demo-show"}


def desktop(app, plan):
    initial = open_card(app)
    require(bounds(initial, NESTED)["x"] < bounds(initial, MOVE)["x"], "LTR action order changed")
    frames = {}
    for theme in ("light", "dark"):
        if theme == "dark":
            app.activate("card-theme")
        for kind in KINDS:
            app.activate("card-" + kind)
            name = f"{theme}-{kind}"
            frames[name] = capture(app, plan, name, app.dom(), theme, kind)
    return {"captures": frames, "keyboard": keyboard_journey(app),
            "primary_title_height": bounds(initial, PRIMARY + "-title")["height"]}


def compact(app, plan, base_height):
    open_card(app)
    for key in ("card-theme", "card-direction", "card-large-text"):
        app.activate(key)
    frame = app.dom()
    title = bounds(frame, PRIMARY + "-title")
    card = bounds(frame, PRIMARY)
    require(title["height"] == 2 * base_height, f"Compact heading is not measured at 200%: {title}")
    require(abs(title["x"] + title["width"] - (card["x"] + card["width"] - 16)) < 0.51,
            "RTL title did not align to logical start")
    require(bounds(frame, NESTED)["x"] > bounds(frame, MOVE)["x"], "RTL actions did not mirror")
    for key in ("card-demo-back", "card-theme", "card-direction", "card-large-text",
                "card-toggle-mode", "card-disable", *("card-" + kind for kind in KINDS)):
        node, geometry = find(frame, key)
        require(same_box(geometry["clip"], geometry["bounds"]), f"Compact {key} is clipped")
        require(not node["disabled"], f"Compact {key} is disabled")
    tab = focus_by_tab(app, PRIMARY)
    frame = expect_focus(app, PRIMARY)
    return {"title_height": title["height"], "tab_to_primary": tab,
            "capture": capture(app, plan, "dark-rtl-200", frame, "dark", "elevated", focus_ring=True)}


def main():
    parser = argparse.ArgumentParser()
    for key in ("gallery", "cli", "out"):
        parser.add_argument("--" + key, type=Path, required=True)
    parser.add_argument("--source", required=True)
    plan = parser.parse_args()
    plan.out.mkdir(exist_ok=False)
    receipt = {"source_manifest_sha256": plan.source, "gallery_sha256": digest(plan.gallery),
               "cli_sha256": digest(plan.cli), "headless_renderer": "WGPU native macOS"}
    with launch(plan, "desktop", 960, 800) as app:
        require(onscreen_windows(app.proc.pid) == 0, "Desktop Gallery owns a visible window")
        receipt["desktop"] = desktop(app, plan)
        receipt["desktop"]["onscreen_windows"] = onscreen_windows(app.proc.pid)
        require(receipt["desktop"]["onscreen_windows"] == 0, "Desktop replay opened a window")
    print("PASS desktop cards, palettes, shadows, transport and keyboard", flush=True)
    with launch(plan, "compact", 320, 800) as app:
        require(onscreen_windows(app.proc.pid) == 0, "Compact Gallery owns a visible window")
        receipt["compact"] = compact(app, plan, receipt["desktop"]["primary_title_height"])
        receipt["compact"]["onscreen_windows"] = onscreen_windows(app.proc.pid)
        require(receipt["compact"]["onscreen_windows"] == 0, "Compact replay opened a window")
    print("PASS compact dark RTL 200% text, focus paint and transport", flush=True)
    (plan.out / "native-card-report.json").write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
