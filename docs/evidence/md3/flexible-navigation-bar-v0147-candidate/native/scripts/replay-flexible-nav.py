"""Exercise the stock Navigation bar Gallery without creating a window."""

import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

PREFIX = "navigation-bar-demo-"
SHOW, BACK, PREVIEW, SCROLL, BAR, PAGE = (
    PREFIX + name for name in ("show", "back", "preview", "scroll", "bar", "page-title")
)
ITEMS = tuple(PREFIX + name for name in ("home", "explore", "saved", "updates", "settings"))
HOME, EXPLORE, SAVED, UPDATES, SETTINGS = ITEMS
CONTROL_SETTINGS = [PREFIX + name for name in ("theme", "rtl", "large-text", "count-3", "count-4", "count-5")]


def check_current(frame, chosen, count):
    for target in ITEMS[:count]:
        item = node(frame, target)
        require(item["role"] == "Button" and item["current_page"] is (target == chosen),
                f"{target} lost current page semantics")
    require(node(frame, EXPLORE)["disabled"], "the disabled route became enabled")
    require(node(frame, SAVED)["name"] == "收藏的地點", "the destination lost its full accessible name")
    require(node(frame, BAR)["role"] == "Navigation" and node(frame, BAR)["name"] == "主要導覽",
            "the bar lost its navigation landmark")


def check_layout(frame, width, height, count, rtl=False):
    preview = visible(frame, PREVIEW)
    bar = visible(frame, BAR, 48)
    scroller = visible(frame, SCROLL)
    expected_width = min(width, 480)
    require(preview["x"] == (width - expected_width) / 2 and preview["width"] == expected_width,
            f"the preview is not centered: {preview}")
    require(bar["width"] == expected_width and bar["x"] == preview["x"]
            and bar["height"] >= 64 and bar["y"] + bar["height"] == height,
            f"the compact bar is not anchored below content: {bar}")
    require(scroller["y"] + scroller["height"] == bar["y"], "the content must stop above the bar")
    items = {}
    for index, target in enumerate(ITEMS):
        if index >= count:
            require(find(frame, target)[1] is None, f"the {count} destination fixture still has {target}")
            continue
        item = visible(frame, target, 48)
        order = count - index - 1 if rtl else index
        approximate_x = preview["x"] + order * expected_width / count
        require(abs(item["x"] - approximate_x) <= 1 and abs(item["width"] - expected_width / count) <= 1,
                f"wrong equal allocation for {target}: {item}")
        require(item["height"] == bar["height"], f"{target} does not meet the shared item extent")
        indicator = visible(frame, target + "-indicator")
        require((indicator["width"], indicator["height"]) == (56, 32), f"indicator for {target} drifted")
        label = visible(frame, target + "-label")
        require(label["y"] - indicator["y"] - indicator["height"] == 4., f"wrong label gap for {target}")
        items[target] = {"target": item, "label": label, "indicator": indicator}
    return {"preview": preview, "bar": bar, "scroll": scroller, "items": items}


def open_route(app, *, compact=False):
    activate(app, "section-3")
    if compact:
        focus_by_tab(app, SHOW)
        frame = key(app, "enter")
    else:
        frame = activate(app, SHOW)
    focus(frame, BACK)
    return frame


def wide(plan):
    with launch(plan, "wide", 960, 800) as app:
        windows = [onscreen_windows(app.proc.pid)]
        frame = open_route(app)
        original = check_layout(frame, 960, 800, 5)
        require(original["bar"]["height"] == 64, "one-line 100% label changed the 64 dp default")
        check_current(frame, HOME, 5)
        for target in [*CONTROL_SETTINGS, HOME, SAVED]:
            frame = key(app, "tab")
            focus(frame, target)
            check_current(frame, HOME, 5)
        frame = key(app, "enter")
        focus(frame, SAVED)
        check_current(frame, SAVED, 5)
        require(node(frame, PAGE)["name"] == "收藏", "the selected page heading did not change")
        denied = app.wire("Action.activate", {"id": EXPLORE}, expected_error="disabled")
        require(denied == "disabled", "a disabled destination activated")
        image = capture(app, plan, "wide-light-focused", frame=frame, sample_targets=[SAVED + "-label"])
        counts = {}
        for count in (3, 4, 5):
            frame = activate(app, f"{PREFIX}count-{count}")
            check_current(frame, SAVED, count)
            counts[str(count)] = check_layout(frame, 960, 800, count)
        windows.append(onscreen_windows(app.proc.pid))
        focus(activate(app, BACK), SHOW)
        windows.append(onscreen_windows(app.proc.pid))
        require(windows == [0] * 3, "wide Gallery had an on-screen window")
    return {"viewport": [960, 800], "windows": windows, "counts": counts, "shot": image}


def compact(plan):
    with launch(plan, "compact", 320, 400) as app:
        windows = [onscreen_windows(app.proc.pid)]
        open_route(app, compact=True)
        for target in CONTROL_SETTINGS[:3]:
            activate(app, target)
        frame = settle(app)
        geometry = check_layout(frame, 320, 400, 5, rtl=True)
        require(geometry["bar"]["height"] > 64, "large text did not grow the bar")
        require(geometry["items"][UPDATES]["label"]["height"] >= 64., "large CJK label did not wrap")
        check_current(frame, HOME, 5)
        focus_by_tab(app, HOME)
        frame = key(app, "tab")
        focus(frame, SAVED)
        frame = key(app, "space")
        focus(frame, SAVED)
        check_current(frame, SAVED, 5)
        require(node(frame, PAGE)["name"] == "收藏", "the compact page heading did not change")
        visible(frame, BACK, 48.)
        visible(frame, PAGE)
        caption_targets = [PAGE, SAVED + "-label", UPDATES + "-label"]
        image = capture(app, plan, "compact-dark-rtl-200", frame=frame, sample_targets=caption_targets)
        page_position = layout(frame, PAGE)
        windows.append(onscreen_windows(app.proc.pid))
        focus(activate(app, BACK), SHOW)
        windows.append(onscreen_windows(app.proc.pid))
        require(windows == [0] * 3, "compact Gallery had an on-screen window")
    return {"viewport": [320, 400], "rtl": True, "text_scale": 2, "windows": windows,
            "geometry": geometry, "page_position": page_position, "shot": image}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ("gallery", "cli", "out"):
        parser.add_argument(f"--{field}", required=True, type=Path)
    args = parser.parse_args()
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Use a private /tmp directory")
    out.mkdir()
    plan = SimpleNamespace(gallery=args.gallery.resolve(strict=True), cli=args.cli.resolve(strict=True), out=out)
    binaries = {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}
    report = {"schema": "surface.test.flexible-nav-gallery-native.v1", "binaries": binaries,
              "headless": True, "transport": ["Page.screenshot", "CLI", "MCP"],
              "profiles": [wide(plan), compact(plan)]}
    require(binaries == {"gallery": digest(plan.gallery), "cli": digest(plan.cli)}, "Binaries changed")
    (out / "native-gallery-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print("Both Gallery profiles passed, two decoded PNGs agree across API, CLI and MCP, no visible windows.")


if __name__ == "__main__":
    main()
