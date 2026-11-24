"""Verify a stock Material command menu with the native headless renderer."""

import argparse
import json
import math
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, focused, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows


PREFIX = "menu-demo-"
SHOW = PREFIX + "show"
BACK = PREFIX + "back"
OPENER = PREFIX + "opener"
PANEL = PREFIX + "panel"
FIRST = PREFIX + "inbox"
DISABLED = PREFIX + "archive"
FIND = PREFIX + "find"
LAST = PREFIX + "details"
STATUS = PREFIX + "last"
OUTSIDE = PREFIX + "list"
BOTTOM = PREFIX + "bottom"
BODY = PREFIX + "preview-scroll"
LARGE_TEXT = PREFIX + "large-text"
LAST_RESULT = "已執行「顯示詳細資料」"
PROFILES = (("wide-light-ltr", (960, 800), False), ("compact-dark-rtl-200", (320, 480), True))


def right(rect):
    return rect["x"] + rect["width"]


def bottom(rect):
    return rect["y"] + rect["height"]


def absent(frame, target):
    require(find(frame, target)[1] is None, f"{target} is still mounted")


def open_menu(app):
    frame = app.dom()
    if focused(frame) != OPENER:
        focus_by_tab(app, OPENER)
    frame = settle(app)
    require(node(frame, OPENER).get("expanded") is False, "Closed trigger has an incorrect hint")
    absent(frame, PANEL)
    frame = key(app, "enter")
    focus(frame, FIRST)
    require(node(frame, OPENER).get("expanded") is True, "Open trigger has an incorrect hint")
    require(node(frame, PANEL)["role"] == "Menu" and node(frame, PANEL)["name"] == "檔案動作",
            "Open panel lost its authored menu role or name")
    return frame


def assert_closed(frame, expected_focus):
    absent(frame, PANEL)
    require(node(frame, OPENER).get("expanded") is False, "Trigger stays expanded after dismissal")
    focus(frame, expected_focus)


def menu_geometry(frame, rtl):
    trigger, panel = (visible(frame, target) for target in (OPENER, PANEL))
    viewport = frame["layout"]["viewport"]
    require(same_box(viewport, trigger), f"Trigger leaves viewport: {trigger}")
    safe = dict(x=8., y=8., width=viewport["width"] - 16., height=viewport["height"] - 16.)
    require(same_box(safe, panel) and panel["width"] == 240.,
            f"Menu exceeds safe viewport or measured width: {panel}")
    gap = trigger["y"] - bottom(panel) if rtl else panel["y"] - bottom(trigger)
    require(abs(gap - 4.) <= 0.5,
            f"Menu does not keep the 4 dp {'above' if rtl else 'below'} gap: {panel}, {trigger}")
    expected_edge = right(trigger) if rtl else trigger["x"]
    require(abs((right(panel) if rtl else panel["x"]) - expected_edge) <= 0.5,
            f"Panel does not align to the trigger's logical start: {panel}, {trigger}")
    first, icon = (visible(frame, target) for target in (FIRST, FIRST + "-leading"))
    require(first["height"] >= 48. and abs(first["y"] - panel["y"] - 8.) <= 0.5,
            f"First row lost its 48 dp minimum or 8 dp panel padding: {first}, {panel}")
    require((icon["width"], icon["height"]) == (24., 24.), f"Menu icon is not 24 dp: {icon}")
    icon_inset = right(first) - right(icon) if rtl else icon["x"] - first["x"]
    require(abs(icon_inset - 12.) <= 0.5, f"Incorrect logical menu icon inset: {icon_inset}")
    divider = layout(frame, PREFIX + "divider")["bounds"]
    require(divider["height"] == 1. and divider["width"] == panel["width"],
            f"Menu separator is not a full-width 1 dp rule: {divider}")
    rename = layout(frame, PREFIX + "rename")["bounds"]
    if rtl:
        content = layout(frame, PANEL + "-content")["bounds"]
        require(rename["height"] > 48. and content["height"] > panel["height"],
                "The 200% text menu neither wraps nor creates a scroll range")
    return {"trigger": trigger, "panel": panel, "first": first, "leading_icon": icon,
            "separator": divider, "rename": rename, "side": "above" if rtl else "below"}


def rgba_at(rows, x, y):
    return list(rows[y][4 * x:4 * (x + 1)])


def capture_panel(app, plan, profile, stage, frame, sample):
    panel = visible(frame, PANEL)
    sample_id = sample + "-label"
    shot = capture(app, plan, f"{profile}-{stage}", frame=frame, sample_targets=(sample_id,))
    width, height, rows = read_png((plan.out / shot["name"]).read_bytes())
    viewport = frame["layout"]["viewport"]
    require((width, height, shot["scale"]) == (viewport["width"], viewport["height"], 1.),
            f"Screenshot dimensions do not match the 1× logical viewport: {shot}")
    panel_token = node(frame, PANEL)["style"]["background"]
    page_token = node(frame, "surface-page")["style"]["background"]
    dark = profile.startswith("compact")
    require(panel_token != page_token and panel_token[3] == page_token[3] == 255,
            "Menu and surrounding page lost their separate opaque surface colors")
    if dark:
        require(0 < min(panel_token[:3]) <= max(panel_token[:3]) < 64 and
                sum(panel_token[:3]) > sum(page_token[:3]),
                f"Wrong dark Surface Container tone: {panel_token}, {page_token}")
    else:
        require(panel_token == [242, 236, 244, 255] and page_token == [253, 247, 255, 255],
                f"Wrong light Surface Container palette: {panel_token}, {page_token}")

    # This interior stripe misses text and icons, rounded corners, and the
    # two-pixel focus stroke. A separator may cross it.
    first_x = math.ceil(panel["x"] + 5.)
    first_y = math.ceil(panel["y"] + 6.)
    last_y = math.floor(bottom(panel) - 6.)
    points = [(x, y) for x in range(first_x, first_x + 3)
              for y in range(first_y, last_y)]
    matches = sum(rgba_at(rows, x, y) == panel_token for x, y in points)
    require(matches >= 0.95 * len(points),
            f"Panel interior did not paint the expected tone ({matches}/{len(points)})")
    require(rgba_at(rows, 2, 2) == page_token, "The PNG page corner did not paint the Surface tone")
    target = focused(frame)
    row = visible(frame, target, 48.)
    focus_token = node(frame, target)["focus_color"]
    outline_points = [(x, y) for x in range(math.ceil(row["x"] + 16.), math.floor(right(row) - 16.))
                      for y in range(math.ceil(row["y"]), math.floor(row["y"] + 2.))]
    outline_pixels = sum(rgba_at(rows, x, y) == focus_token for x, y in outline_points)
    require(outline_pixels >= len(outline_points) * 0.5 and len(outline_points) >= 32,
            f"Missing focus stroke on {target}: {outline_pixels}/{len(outline_points)} pixels")
    shot["outline"] = {"target": target, "bounds": row, "rgba": focus_token,
                       "matching_pixels": outline_pixels, "samples": len(outline_points)}
    shot["palette"] = {"panel": panel_token, "page": page_token, "solid_panel_pixels": matches,
                       "total_panel_samples": len(points), "page_point": [2, 2]}
    return shot


def blocked_keys(app, frame):
    focus(frame, DISABLED)
    command = node(frame, DISABLED)
    require(command["role"] == "MenuItem" and command["name"] == "封存" and command["disabled"],
            "Disabled item lost its authored accessible name and state")
    require(node(frame, DISABLED + "-label")["style"]["foreground"][3] == 97,
            "The disabled item lost its 38% label ink")
    visible(frame, DISABLED, 48.)
    actions = []
    for name in ("enter", "space"):
        result = app.wire("Action.key", {"key": name})
        require(result["focused"] == DISABLED and result["changed"] is False,
                f"{name} activated a disabled command: {result}")
        current = settle(app)
        focus(current, DISABLED)
        visible(current, PANEL)
        absent(current, STATUS)
        actions.append(name)
    reply = app.wire("Action.activate", {"id": DISABLED}, expected_error="disabled")
    require(reply == "disabled", "Direct activation did not reject the disabled item")
    return {"focus_kept": DISABLED, "keys_rejected": actions, "direct_error": reply}


def dark_setup(app):
    frame = app.dom()
    for suffix in ("theme", "rtl", "large-text", "bottom"):
        frame = activate(app, PREFIX + suffix)
    for suffix in ("rtl", "large-text", "bottom"):
        require(node(frame, PREFIX + suffix).get("selected") is True, f"{suffix} mode did not toggle")


def intersection(first, second):
    x, y = max(first["x"], second["x"]), max(first["y"], second["y"])
    return {"x": x, "y": y, "width": max(0., min(right(first), right(second)) - x),
            "height": max(0., min(bottom(first), bottom(second)) - y)}


def footer_status(frame, enlarged):
    expected_bottom = frame["layout"]["viewport"]["height"] - 28.
    expected_right = frame["layout"]["viewport"]["width"] - 28.
    trigger, body = (visible(frame, target) for target in (OPENER, BODY))
    require(bottom(trigger) == expected_bottom and right(trigger) == expected_right,
            f"Footer trigger left its anchored edge: {trigger}")
    require(trigger["height"] >= 48. and trigger["y"] - bottom(body) == 8.,
            f"Body does not clear the actual trigger bounds: body={body} trigger={trigger}")
    require(node(frame, BODY)["style"]["clip"] is True, "Footer body lost its scroll clip")
    require(node(frame, LARGE_TEXT)["selected"] is enlarged, "Unexpected Gallery text size")
    status = node(frame, STATUS)
    require(status["role"] == "Text" and status["text"] == LAST_RESULT,
            "The chosen command was not kept in the original status node")
    arranged = layout(frame, STATUS)
    require(arranged["clip"] == body, f"Status has the wrong body clip: {arranged}")
    shown = intersection(arranged["bounds"], arranged["clip"])
    if shown["height"] == 0 or shown["width"] == 0:
        shown = None
    if shown is not None:
        require(same_box(body, shown) and bottom(shown) <= trigger["y"] - 8.,
                f"Visible status reaches the footer: status={shown}, trigger={trigger}")
    if not enlarged:
        require(shown == arranged["bounds"], "100% status should fit entirely in the preview")
    return {"trigger": trigger, "body": body, "result": status["text"], "status": arranged,
            "visible_status": shown, "text_percent": 200 if enlarged else 100}


def capture_footer(app, plan, profile, stage, frame, evidence):
    fully_visible = evidence["visible_status"] == evidence["status"]["bounds"]
    sample = (STATUS,) if fully_visible else ()
    shot = capture(app, plan, f"{profile}-{stage}", frame=frame, sample_targets=sample)
    width, height, rows = read_png((plan.out / shot["name"]).read_bytes())
    require((width, height, shot["scale"]) == (320, 480, 1.), "Wrong footer screenshot dimensions")
    trigger, body = evidence["trigger"], evidence["body"]
    page_token = node(frame, "surface-page")["style"]["background"]
    points = [(x, y) for x in range(math.ceil(body["x"]), math.floor(right(body)))
              for y in range(math.ceil(bottom(body)), math.floor(trigger["y"]))]
    clear_pixels = sum(rgba_at(rows, x, y) == page_token for x, y in points)
    require(clear_pixels == len(points) and len(points) >= 8 * 100,
            f"Gap has paint from outside the body clip: {clear_pixels}/{len(points)} clean pixels")
    shot["footer"] = {"clear_pixels": clear_pixels, "expected_pixels": len(points),
                      "page": page_token, "status_fully_visible": fully_visible}
    return shot


def replay(plan, profile, size, compact):
    windows = []
    with launch(plan, profile, *size) as app:
        def sample_windows():
            count = onscreen_windows(app.proc.pid)
            require(count == 0, f"{profile} opened a window")
            windows.append(count)

        sample_windows()
        activate(app, "section-2")
        launch_path = focus_by_tab(app, SHOW)
        frame = key(app, "enter")
        focus(frame, BACK)
        if compact:
            dark_setup(app)

        frame = open_menu(app)
        geometry = menu_geometry(frame, compact)
        captures = [capture_panel(app, plan, profile, "opened", frame, FIRST)]
        sample_windows()
        focus_history = [FIRST]
        if compact:
            last_before = layout(frame, LAST)
            last_row = last_before["bounds"]
            require(not same_box(last_before["clip"], last_row) and bottom(last_row) > bottom(geometry["panel"]),
                    f"End demo needs an initially clipped last item: {last_before}")
            frame = key(app, "end")
            focus(frame, LAST)
            focus_history.append(LAST)
            last_after = visible(frame, LAST, 48.)
            require(last_after["y"] < last_row["y"] and same_box(geometry["panel"], last_after),
                    f"End did not scroll the final command into the panel: {last_after}")
            require(visible(frame, PANEL) == geometry["panel"], "Scrolling moved the menu panel")
            captures.append(capture_panel(app, plan, profile, "scrolled-end", frame, LAST))
            sample_windows()
            frame = key(app, "escape")
            assert_closed(frame, OPENER)
            absent(frame, STATUS)
            open_menu(app)
            frame = key(app, "end")
            focus(frame, LAST)
            frame = key(app, "enter")
            assert_closed(frame, OPENER)
            enlarged = footer_status(frame, True)
            require(enlarged["trigger"] == geometry["trigger"], "The chosen result moved the trigger")
            captures.append(capture_footer(app, plan, profile, "status-200", frame, enlarged))
            sample_windows()

            frame = key(app, "tab", reverse=True)
            focus(frame, OUTSIDE)
            after_tab = footer_status(frame, True)
            require(after_tab["trigger"] == enlarged["trigger"], "Body focus moved the footer")
            require(same_box(after_tab["body"], visible(frame, OUTSIDE, 48.)),
                    "The scroller's List control is not usable with keyboard focus")
            frame = activate(app, LARGE_TEXT)
            focus(frame, LARGE_TEXT)
            regular = footer_status(frame, False)
            require(regular["trigger"]["width"] < enlarged["trigger"]["width"],
                    "The screenshot fixture did not exercise a natural button reflow")
            captures.append(capture_footer(app, plan, profile, "status-100", frame, regular))
            sample_windows()

            frame = activate(app, LARGE_TEXT)
            restored = footer_status(frame, True)
            require(restored["trigger"] == enlarged["trigger"] and
                    restored["body"] == enlarged["body"], "200% footer geometry did not restore")
            frame = open_menu(app)
            visible(frame, FIRST, 48.)
            require(visible(frame, OPENER) == geometry["trigger"] and
                    bottom(visible(frame, PANEL)) + 4. == geometry["trigger"]["y"],
                    "The compact menu did not reopen above the stationary trigger")
            frame = key(app, "escape")
            assert_closed(frame, OPENER)
            require(footer_status(frame, True)["result"] == LAST_RESULT, "Escape erased the previous result")
            evidence = {"last_before": last_before, "last_after": last_after, "escape_focus": OPENER,
                        "status_200": enlarged, "status_100": regular, "restored_200": restored,
                        "body_control_focus": OUTSIDE}
        else:
            frame = key(app, "down")
            blocked = blocked_keys(app, frame)
            focus_history.append(DISABLED)
            captures.append(capture_panel(app, plan, profile, "disabled-focused", frame, FIRST))
            frame = key(app, "down")
            focus(frame, FIND)
            focus_history.append(FIND)
            captures.append(capture_panel(app, plan, profile, "find-focused", frame, FIND))
            sample_windows()
            frame = key(app, "enter")
            assert_closed(frame, OPENER)
            require(node(frame, STATUS)["text"] == "已執行「在清單中尋找」",
                    "Choosing the focused enabled command did not update the visible status")
            focus_history.append(OPENER)

            open_menu(app)
            frame = key(app, "tab")
            assert_closed(frame, OUTSIDE)
            focus_history.append(OUTSIDE)
            open_menu(app)
            frame = key(app, "tab", reverse=True)
            assert_closed(frame, BOTTOM)
            focus_history.append(BOTTOM)
            open_menu(app)
            frame = key(app, "escape")
            assert_closed(frame, OPENER)
            focus_history.append(OPENER)
            require(node(frame, STATUS)["text"] == "已執行「在清單中尋找」",
                    "Tab/Escape caused a repeated command")
            evidence = {"disabled": blocked, "choice": node(frame, STATUS)["text"],
                        "tab_focus": OUTSIDE, "reverse_tab_focus": BOTTOM, "escape_focus": OPENER}

        activate(app, BACK)
        focus(settle(app), SHOW)
        sample_windows()
    return {"profile": profile, "viewport": size, "navigation_to_route": launch_path,
            "menu_focus_history": focus_history, "geometry": geometry, "checks": evidence,
            "captures": captures, "restored_focus": SHOW, "visible_windows": windows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for argument in ("gallery", "cli", "out"):
        parser.add_argument("--" + argument, type=Path, required=True)
    args = parser.parse_args()
    plan = SimpleNamespace(
        gallery=args.gallery.resolve(strict=True),
        cli=args.cli.resolve(strict=True),
        out=args.out.resolve(),
    )
    require(plan.out.is_relative_to(Path("/tmp").resolve()), "Use a private /tmp output directory")
    plan.out.mkdir()
    pins = {name: digest(getattr(plan, name)) for name in ("gallery", "cli")}
    report = {
        "schema": "surface.test.material-command-menu-native.v2",
        "binaries": pins,
        "transport": ["Page.screenshot", "CLI", "MCP"],
        "profiles": [replay(plan, *profile) for profile in PROFILES],
    }
    require(pins == {name: digest(getattr(plan, name)) for name in pins}, "Binaries changed")
    (plan.out / "native-menu-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print("PASS 2 headless menu profiles, 7 PNGs matched on API, CLI and MCP, zero onscreen windows")


if __name__ == "__main__":
    main()
