"""Check the actual q75 Chrome Menu measurements and PNGs."""
import math
import re

from PIL import Image, ImageChops
from evidence_common import data, digest, require, clean_windows, verify_ledgers

STEPS = ["M267", "M268", "M269", "M370", "M371", "M372"]
WINDOW = "headless-window-observation.json"
REPORTS = ("browser-material-command-menu.json", "browser-material-vertical-menu.json")
SUFFIXES = {"command-menu-wide": (1920, 1688), "command-menu-compact": (640, 1048),
            "command-menu-compact-footer": (640, 1048), "vertical-menu-standard": (1920, 2328),
            "vertical-menu-vibrant": (1920, 2328), "vertical-menu-dark-rtl": (640, 1048)}
PNG = tuple(f"material-{name}.png" for name in SUFFIXES)


def load(root, name):
    path = root / f"material-{name}.png"
    require(path.is_file() and not path.is_symlink(), f"Missing original capture {path.name}")
    with Image.open(path) as image:
        require(image.size == SUFFIXES[name] and image.format == "PNG", f"Wrong size or format {path.name}")
        return image.convert("RGB")


def sample(image, x, y):
    return list(image.getpixel((math.floor(x * 2), math.floor((y + 44) * 2))))


def contains(outer, inner, tolerance=0.2):
    x, y, w, h = outer
    a, b, c, d = inner
    return x <= a + tolerance and y <= b + tolerance and a + c <= x + w + tolerance and b + d <= y + h + tolerance


def gaps(groups, gap):
    require(len(groups) == 3
            and all(abs(right[1] - left[1] - left[3] - gap) <= 0.2 for left, right in zip(groups, groups[1:])),
            f"Wrong measured {gap} dp gaps")


def pixels(root):
    base, vertical = (data(root / name) for name in REPORTS)
    adapter = vertical["adapter"]
    require(base["browser"] == vertical["browser"] and re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", vertical["browser"])
            and base["adapter"] == adapter and adapter["vendor"] == "apple"
            and adapter["architecture"].startswith("metal-") and adapter["isFallbackAdapter"] is False,
            "Expected actual Chrome with nonfallback Apple Metal")
    require(all(p["errors"] == [] and not {"failure", "state_at_failure"}.intersection(p) for p in (base, vertical)),
            "Browser report contains a failure or page errors")
    menu_captures = {
        path.name
        for prefix in ("material-command-menu-", "material-vertical-menu-")
        for path in root.glob(f"{prefix}*.png")
    }
    require(menu_captures == set(PNG), "Wrong original Baseline and Vertical Menu image names")

    wide = base["keyboard"]
    x, y, w, h = wide["panel"]
    capture = load(root, "command-menu-wide")
    require(wide["disabled_focus"] is True and w == 240. and y >= wide["trigger"][1] + wide["trigger"][3]
            and sample(capture, x + 3., y + h / 2.) == wide["center_rgb"]
            and sample(capture, x, y) == wide["corner_rgb"] != wide["center_rgb"], "Baseline wide focus or paint differs")
    require(base["dismissal"] == {"tab_forward": "menu-demo-list", "tab_backward": "menu-demo-bottom",
                                  "outside_action_suppressed": True}, "Baseline dismissal differs")

    compact = base["compact"]
    require(compact["wrapped_height"] > 48. and compact["state"]["rtl"] is True
            and compact["state"]["text_scale"] == 2 and compact["state"]["bottom_anchor"] is True
            and contains([8, 8, 304, 464], compact["panel"]) and contains(compact["panel"], compact["last"])
            and compact["panel"][1] + compact["panel"][3] <= compact["trigger"][1]
            and not contains(compact["scroller"], compact["status_before"]) and contains(compact["scroller"], compact["status_after"])
            and compact["scroller"][1] + compact["scroller"][3] + 8 <= compact["trigger"][1], "Compact Baseline measures differ")
    require(ImageChops.difference(load(root, "command-menu-compact"), load(root, "command-menu-compact-footer")).getbbox(),
            "Baseline menu and closed footer captures are identical")

    standard, vibrant, short = (vertical[key] for key in ("standard", "vibrant", "overflow"))
    require(standard["last"] == "顯示詳細資料" and standard["paint"]["group"] == standard["groups"][0]
            and vibrant["pointer_command"] == {"before_count": 1, "after_count": 2, "focus": "menu-demo-opener", "last": "建立副本"}
            and vibrant["reopen_focus"] == "menu-demo-inbox" and vibrant["tab_focus"] == "menu-demo-list"
            and vibrant["external_focus"] == {"element": "menu-demo-host-focus", "surface_focus": None},
            "Vertical menu command or focus handoff differs")
    for measured, gap in ((standard["groups"], 2), (short["groups"], 0), (short["restored_groups"], 2)):
        gaps(measured, gap)

    panel, viewport = short["panel"], short["viewport"]
    top, bottom = short["thumb_top"], short["thumb_bottom"]
    expected_height = max((viewport[3] - 8.) * viewport[3] / short["content"][3], 16.)
    require(contains([8, 8, 304, 464], panel) and panel[1] + panel[3] <= short["trigger"][1]
            and contains(viewport, short["last"]) and short["rename"][3] > 48.
            and all(contains(panel, thumb) and abs(thumb[3] - expected_height) <= 1. for thumb in (top, bottom))
            and top[0] < panel[0] + panel[2] / 2. and bottom[1] > top[1]
            and short["scroll"][1] > short["scroll"][0], "Compact RTL thumb, reflow or keyboard reveal differs")
    require(short["restored_thumb"] == [] and short["restored_scroll"] == 0 and short["restored_panel"] != panel
            and contains([8, 8, 944, 1104], short["restored_panel"])
            and short["restored_panel"][3] > panel[3], "Wide restored viewport or group gaps differ")

    colors = {}
    for name, paint, rtl in (("standard", standard["paint"], False), ("vibrant", vibrant, False),
                              ("dark-rtl", short["paint"], True)):
        image = load(root, f"vertical-menu-{name}")
        x, y, w, h = paint["group"]
        edge = sample(image, x + (w - 2 if rtl else 2), y + h / 2)
        require(edge == paint["color"] and sample(image, x + w / 2., y + h / 2.) == paint["center"],
                f"The original {name} group pixels differ from the recorded paint")
        colors[name] = edge
        if rtl:
            a, b, c, d = bottom
            require(sample(image, a + c / 2, b + d / 2) == paint["thumb_color"] and paint["thumb_color"] != edge,
                    "Actual mirrored thumb ink differs")
    require(len({tuple(value) for value in colors.values()}) == 3, "Expected three distinct Menu palette results")
    return {"browser": vertical["browser"], "adapter": adapter, "palette_pixels": colors, "gaps_dp": [2, 0, 2],
            "compact_panel": panel, "expanded_panel": short["restored_panel"],
            "original_artifacts": {name: digest(root / name) for name in (*REPORTS, *PNG)}}


def run_evidence(run, original):
    watch = data(run / WINDOW)
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary"})
    one, = watch["guard_events"]
    args = one["argv"]
    require(args.count("--headless=new") == args.count("--disable-updater-scheduler") == 1,
            "Headless Chrome arguments differ")
    log = (run / "runner.log").read_text()
    require([name for name in re.findall(r"^  \[(M\d+)\]", log, re.M) if name in STEPS] == STEPS
            and original["steps"] == 371 and original["chromium_samples"] > 0, "Wrong actual Material Menu scope")
    result = pixels(run / "artifacts")
    require(set(re.findall(r"\bbrowser: (Chrome/[0-9][0-9.]+)\b", log)) == {result["browser"]},
            "Chrome version differs from original runner transcript")
    return {**result, "steps": STEPS, "visible_windows": 0, "surviving_processes": 0,
            "window_samples": watch["launch_observations"]["chrome-primary"]["samples"]}
