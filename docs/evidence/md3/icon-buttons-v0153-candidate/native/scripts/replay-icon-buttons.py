"""Exercise real icon buttons in the windowless Surface Gallery."""
import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, focus, key, node, visible
from window_check import onscreen_windows


PREFIX = "icon-button-demo-"
SHOW, BACK, THEME, RTL, LARGE_TEXT, COMMAND, TOGGLE, DISABLED, MEDIUM = (
    PREFIX + suffix for suffix in ("show", "back", "theme", "rtl", "large-text",
                                   "command", "toggle", "disabled", "medium")
)
TOOLTIP = PREFIX + "tooltip"


def geometry(frame, id_, expected):
    action, face, art, outline = (
        visible(frame, id_ + suffix) for suffix in ("", "-face", "-icon", "-focus")
    )
    target_width, target_height, width, height, icon = expected
    for box, dimensions in ((action, (target_width, target_height)),
                            (face, (width, height)), (art, (icon, icon)),
                            (outline, (width + 10, height + 10))):
        require((box["width"], box["height"]) == dimensions,
                f"Wrong recorded dimensions for {id_}: {box}")
        for axis, length in (("x", "width"), ("y", "height")):
            require(abs(box[axis] + box[length] / 2 - action[axis] - action[length] / 2) < 0.51,
                    f"The face, icon, and focus must share a center: {id_}")
    require(node(frame, id_)["role"] == "Button", f"Missing action role for {id_}")
    return {"action": action, "face": face, "icon": art, "focus": outline}


def rgba(rows, x, y):
    return list(rows[y][4 * x:4 * (x + 1)])


def canvas_check(plan, shot, frame, dims):
    width, height, rows = read_png((plan.out / shot["name"]).read_bytes())
    samples = {}
    for id_, dims_ in dims.items():
        face, art = dims_["face"], dims_["icon"]
        point = (int(face["x"] + 2), int(face["y"] + face["height"] / 2))
        require(0 <= point[0] < width and 0 <= point[1] < height, "Face sample outside the PNG")
        paint = rgba(rows, *point)
        color = node(frame, id_ + "-face")["style"]["background"]
        # Token background is sampled from the unoccluded left edge.
        # Keyboard focus can tint that background, so sample the other face.
        if id_ == COMMAND:
            require(paint == color, f"Command face {paint} does not match {color}")
        art_colors = {
            tuple(rgba(rows, x, y))
            for y in range(int(art["y"]), int(art["y"] + art["height"]))
            for x in range(int(art["x"]), int(art["x"] + art["width"]))
        }
        require(len(art_colors) > 1, f"No visible artwork inside {id_}")
        samples[id_] = {"face_point": point, "face_rgba": paint,
                        "art_color_count": len(art_colors)}
    return samples


def run_profile(plan, name, width, height, compact):
    with launch(plan, name, width, height) as app:
        windows = [onscreen_windows(app.proc.pid)]
        focus_by_tab(app, SHOW)
        focus(key(app, "enter"), BACK)
        if compact:
            for control in (THEME, RTL, LARGE_TEXT):
                focus(activate(app, control), control)
        else:
            for control in (THEME, RTL, LARGE_TEXT):
                focus(key(app, "tab"), control)
        command_frame = key(app, "tab")
        focus(command_frame, COMMAND)
        tip = visible(command_frame, TOOLTIP)
        action = visible(command_frame, COMMAND, 48.)
        ring = visible(command_frame, COMMAND + "-focus")
        require(0 <= tip["x"] and tip["x"] + tip["width"] <= width + 0.5,
                "Tooltip escaped the logical viewport")
        require(tip["y"] + tip["height"] <= min(action["y"], ring["y"])
                or tip["y"] >= max(action["y"] + action["height"], ring["y"] + ring["height"]),
                "The tooltip overlaps the command hit area or focus ring")
        tooltip_png = capture(app, plan, "compact-tooltip", frame=command_frame) if compact else None
        frame = key(app, "tab")
        focus(frame, TOGGLE)
        require(node(frame, TOGGLE)["checked"] is False, "Toggle must start off")
        require(node(frame, DISABLED)["disabled"] is True, "Disabled icon is interactive")
        before = geometry(frame, TOGGLE, (52, 48, 52, 40, 24))
        focused_name = node(frame, TOGGLE)["name"]
        frame = key(app, "space")
        focus(frame, TOGGLE)
        require(node(frame, TOGGLE)["checked"] is True, "Space did not toggle on")
        require(node(frame, TOGGLE)["name"] == focused_name, "Toggle changed its action label")
        after = geometry(frame, TOGGLE, (52, 48, 52, 40, 24))
        require(before == after, "Toggling changed the hit or focus geometry")
        dims = {
            COMMAND: geometry(frame, COMMAND, (48, 48, 28, 32, 20)),
            TOGGLE: after,
        }
        image = capture(app, plan, name, frame=frame)
        image["samples"] = canvas_check(plan, image, frame, dims)
        windows.append(onscreen_windows(app.proc.pid))

        frame = key(app, "enter")
        require(node(frame, TOGGLE)["checked"] is False, "Enter did not turn toggle off")
        app.wire("Action.activate", {"id": DISABLED}, expected_error="disabled")
        focus(key(app, "tab"), MEDIUM)
        focus(key(app, "tab", reverse=True), TOGGLE)
        focus(activate(app, BACK), SHOW)
        windows.append(onscreen_windows(app.proc.pid))
        require(windows == [0, 0, 0], "The Gallery made a visible window")
    require(app.proc.returncode is not None, "The Gallery did not exit")
    return {"viewport": [width, height], "rtl": compact, "text_scale": 2 if compact else 1,
            "windows": windows, "geometry": dims, "name": focused_name, "png": image,
            "tooltip": {"bounds": tip, "action": action, "focus_ring": ring, "png": tooltip_png}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ("gallery", "cli", "out"):
        parser.add_argument(f"--{field}", required=True, type=Path)
    args = parser.parse_args()
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use a private directory under /tmp")
    out.mkdir()
    plan = SimpleNamespace(gallery=args.gallery.resolve(strict=True),
                           cli=args.cli.resolve(strict=True), out=out)
    paths = {"gallery": plan.gallery, "cli": plan.cli}
    pins = {key: digest(path) for key, path in paths.items()}
    profiles = [
        run_profile(plan, "wide-light-icon-buttons", 960, 800, False),
        run_profile(plan, "compact-dark-rtl-200", 320, 760, True),
    ]
    require({key: digest(path) for key, path in paths.items()} == pins, "A binary was replaced")
    result = {"schema": "surface.test.icon-buttons.native.v1", "rendering": "headless",
              "transports": ["Page.screenshot", "CLI", "MCP"], "binaries": pins, "profiles": profiles}
    (out / "native-gallery-report.json").write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    print("Wide light and compact dark RTL passed with matching PNG bytes and no visible windows.")


if __name__ == "__main__":
    main()
