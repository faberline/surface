#!/usr/bin/env python3
"""Verify the Flexible Navigation Bar's focused or full-stock Chrome evidence."""

import argparse
from hashlib import sha256
import json
from math import floor, isfinite
from pathlib import Path
import re

from PIL import Image, ImageChops


STEM = "material-flexible-navigation-bar"
NAV_STEPS = [f"M{index}" for index in range(270, 274)]
STOCK_STEPS = [f"M{index}" for index in range(1, 274) if index != 23]
MAC_CASES = ["smoke", "ime", "material", "bridge-web", "devtools-headless"]
LINUX_CASES = ["smoke", "ime", "material", "bridge-native", "bridge-native-compact",
               "bridge-web", "devtools-native", "devtools-headless"]
ADAPTER_FIELDS = ["vendor", "architecture", "isFallbackAdapter"]
SWIFTSHADER = {"vendor": "google", "architecture": "swiftshader", "isFallbackAdapter": True}
IDS = ["navigation-bar-demo-" + name for name in ["home", "explore", "saved", "updates", "settings"]]
WIDE = (960, 800)
COMPACT = (320, 480)
SHOTS = {"light": WIDE, "dark": WIDE, "rtl-large": COMPACT, "rtl-large-scrolled": COMPACT}


def need(condition, description):
    if not condition:
        raise ValueError(description)


def read_json(path):
    result = json.loads(path.read_text())
    need(isinstance(result, dict), f"{path.name} must contain a JSON object")
    return result


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def runner_text(raw):
    """Strip optional prefixes from raw GitHub Actions logs and preserve indentation."""
    return re.sub(
        r"^(?:(?:[^\t\n]+\t){2})?\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z ",
        "", raw, flags=re.M,
    )


def runner_result(log):
    output = runner_text(log)
    starts = re.findall(r"^case (\S+) \.\.\.$", output, re.M)
    cases = re.findall(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks, \d+(?:\.\d+)?s\)$",
                       output, re.M)
    steps = re.findall(r"^  \[(M\d+)\]", output, re.M)
    summary = re.findall(r"^e2e result: (\d+) passed; (\d+) failed; (\d+) checks;", output, re.M)
    browsers = set(re.findall(r"^browser: (\S+)", output, re.M))
    need(not re.search(r"^case \S+ \.\.\. FAILED", output, re.M), "runner reported a failed case")
    need(len(summary) == 1, "expected one completed E2E summary")
    return starts, cases, steps, tuple(map(int, summary[0])), browsers


def validate_log(log, platform, stock, stock_last_material=273):
    starts, cases, steps, summary, browsers = runner_result(log)
    expected_cases = (MAC_CASES if platform == "macos" else LINUX_CASES) if stock else ["material"]
    need(starts == [case[0] for case in cases] == expected_cases,
         f"wrong passing suite sequence for {platform}: {cases}")
    counts = {case: {"steps": int(count), "checks": int(checks)} for case, count, checks in cases}
    need(all(record["steps"] > 0 and record["checks"] > 0 for record in counts.values()),
         "a passing suite withheld all steps or checks")
    expected_steps = [f"M{index}" for index in range(1, stock_last_material + 1) if index != 23] if stock else NAV_STEPS
    need(steps == expected_steps and counts["material"]["steps"] == len(expected_steps),
         "missing, extra or out-of-order Material step")
    need(counts["material"]["checks"] >= 458, "required Material checks are missing")
    need(summary == (len(expected_cases), 0, sum(record["checks"] for record in counts.values())),
         "the passing suite totals do not match the runner summary")
    return counts, steps, summary[2], browsers


def adapter_info(data):
    need(isinstance(data, dict) and all(key in data for key in ADAPTER_FIELDS),
         "missing observed WebGPU adapter fields")
    adapter = {key: data[key] for key in ADAPTER_FIELDS}
    need(type(adapter["isFallbackAdapter"]) is bool
         and all(isinstance(adapter[key], str) for key in ADAPTER_FIELDS[:2]),
         "invalid observed WebGPU adapter fields")
    return adapter


def metadata(data):
    browser = data["browser"]
    need(isinstance(browser, str) and re.fullmatch(r"(?:Chrome|Chromium)/\d+(?:\.\d+)+", browser),
         "invalid observed Chrome version")
    return browser, adapter_info(data["adapter"])


def mac_observation(path, stock):
    observed = read_json(path)
    if stock:
        need(observed.get("stock_exit") == 0 and observed.get("visible_owned_windows") == []
             and observed.get("remaining_observed_processes") == [],
             "stock window observer reports a failure, a window or a leftover process")
        method = observed.get("method")
        need(isinstance(method, str) and "CoreGraphics" in method, "missing CoreGraphics observer method")
        observations = observed["observations"]
        browser = stock_samples(observations["chrome"])
        native = stock_samples(observations["native_gallery"])
    else:
        need(observed.get("exit") == 0 and observed.get("visible_windows") == []
             and observed.get("remaining_chrome") == [],
             "focused window observer reports a failure, a window or a leftover process")
        browser = observed["samples"]
        need(type(browser) is int and browser > 0, "no focused Chrome process samples")
        native = None
    return {"chrome_process_samples": browser, "native_gallery_process_samples": native,
            "visible_windows": 0, "remaining_owned_processes": 0, "sha256": digest(path)}


def stock_samples(series):
    samples, pids = series["samples"], series["pids"]
    need(type(samples) is int and isinstance(pids, list) and 0 < len(pids) <= samples
         and all(type(pid) is int and pid > 0 for pid in pids) and len(set(pids)) == len(pids),
         "invalid stock process samples")
    return samples


def rect(value):
    need(isinstance(value, list) and len(value) == 4
         and all(type(number) in (int, float) and isfinite(number) for number in value)
         and min(value[2:]) > 0, f"invalid bounds: {value!r}")
    return value


def bottom(bounds):
    return bounds[1] + bounds[3]


def contains(outer, inner):
    return (outer[0] <= inner[0] and outer[1] <= inner[1]
            and inner[0] + inner[2] <= outer[0] + outer[2] and bottom(inner) <= bottom(outer))


def geometry(report, count, compact):
    preview, bar, scroll = (rect(report[key]) for key in ("preview", "bar", "scroll"))
    need(preview == ([0, 0, 320, 480] if compact else [240, 0, 480, 800]),
         "wrong Canvas preview rectangle")
    need(bar == ([0, 368, 320, 112] if compact else [240, 736, 480, 64])
         and bottom(scroll) == bar[1] and bottom(bar) == bottom(preview)
         and contains(preview, scroll) and scroll[0] == preview[0] and scroll[2] == preview[2],
         "the bar or independent scroller is misaligned")
    rows = report["destinations"]
    need([row["id"] for row in rows] == IDS[:count], "wrong authored destination order")
    cell = bar[2] / count
    for index, row in enumerate(rows):
        hit, pill, label = (rect(row[key]) for key in ("hit", "pill", "label"))
        physical = count - 1 - index if compact else index
        need(hit == [bar[0] + cell * physical, bar[1], cell, bar[3]] and min(hit[2:]) >= 48,
             "wrong equal width, visual order or hit area")
        need(pill[2:] == [56, 32] and contains(hit, pill) and contains(hit, label)
             and pill[0] + pill[2] / 2 == label[0] + label[2] / 2 == hit[0] + hit[2] / 2
             and pill[1] - hit[1] == 6 and label[1] - bottom(pill) == 4
             and bottom(hit) - bottom(label) >= 6,
             "the pill or label lost its specified geometry or clipped its own cell")
    return {"preview": preview, "bar": bar, "scroll": scroll, "destinations": rows}


def decode(path, viewport):
    with Image.open(path) as png:
        need(png.format == "PNG" and png.mode in ("RGB", "RGBA"), f"wrong PNG pixel format: {path.name}")
        png.verify()
    with Image.open(path) as png:
        png.load()
        width, height = viewport
        need(png.size == (width * 2, (height + 44) * 2), f"wrong PNG dimensions: {path.name}")
        return png.convert("RGB")


def px(x, y):
    return floor(x * 2), floor((y + 44) * 2)


def crop(image, bounds):
    x, y, w, h = bounds
    return image.crop((*px(x, y), *px(x + w, y + h)))


def rgb(value):
    need(isinstance(value, list) and len(value) == 3
         and all(type(channel) is int and 0 <= channel <= 255 for channel in value), "invalid report RGB")
    return tuple(value)


def ink_count(image, ink):
    return sum(count for count, color in image.getcolors(image.width * image.height) if color == ink)


def colors(image, evidence, layout, index, stage):
    selected = layout["destinations"][index]
    cell_x, y, cell_w, _ = selected["hit"]
    x, py, _, _ = selected["pill"]
    surface, pill, corner, ink = (rgb(evidence[key]) for key in ("surface", "pill", "corner", "icon_ink"))
    need(evidence["stage"] == stage and surface == corner and len({surface, pill, ink}) == 3,
         "stage or active colors did not distinguish the pill and ink")
    need(image.getpixel(px(cell_x + cell_w / 2, y + 2)) == surface
         and image.getpixel(px(x + 7, py + 16)) == pill and image.getpixel(px(x, py)) == corner,
         "decoded bar, pill or rounded corner pixels disagree with the report")
    count = ink_count(crop(image, [x + 16, py + 4, 24, 24]), ink)
    need(count > 24 and count == evidence["icon_pixels"], "decoded icon ink disagrees with the report")
    return {"surface_rgb": surface, "pill_rgb": pill, "corner_rgb": corner, "icon_rgb": ink,
            "icon_pixels": count}


def validate_navigation(report, run):
    accessibility = report["accessibility"]
    enabled = [IDS[0], *IDS[2:]]
    need(accessibility == {"landmark": "主要導覽", "roles": ["首頁", "探索路線", "收藏的地點", "最新消息", "設定"],
                           "disabled": IDS[1], "current": IDS[0], "tab_order": enabled,
                           "shift_tab_order": enabled[-2::-1], "preactivations": 0},
         "wrong accessibility landmark, current page, disabled state or Tab sequence")
    actions = report["actions"]
    need({key: actions[key] for key in ("enter", "space", "disabled_pointer", "contact_target", "activations")}
         == {"enter": IDS[2], "space": IDS[3], "disabled_pointer": IDS[1], "contact_target": IDS[4], "activations": 3}
         and len(actions["visible_titles"]) == 3
         and all(contains([240, 0, 480, 80], rect(title)) for title in actions["visible_titles"]),
         "wrong keyboard or pointer activation, or a title escaped the fixed header")

    wide = [geometry(layout, count, False) for count, layout in
            zip((3, 4, 5), report["counts_and_colors"]["layouts"], strict=True)]
    compact = report["compact"]
    layout = geometry(compact["geometry"], 5, True)
    need(compact["rtl"] is True and compact["text_scale"] == 2 and compact["selected"] == IDS[3]
         and compact["focused_after_wheel"] == IDS[4] and compact["activations"] == 4
         and compact["restored_focus"] == "navigation-bar-demo-show",
         "wrong compact options, controlled selection or restored focus")
    headline = rect(compact["heading"])
    entry_before, entry_after = (rect(compact[key]) for key in ("entry_before", "entry_after"))
    scroll = layout["scroll"]
    need(contains([0, 0, 320, scroll[1]], headline) and not contains(scroll, entry_before)
         and contains(scroll, entry_after) and entry_before[1] > entry_after[1]
         and entry_before[:1] == entry_after[:1] and entry_before[2:] == entry_after[2:],
         "the wheel did not reveal the full row while keeping the title pinned")

    files = {name: run / f"{STEM}-{name}.png" for name in SHOTS}
    need(set(run.glob(f"{STEM}-*.png")) == set(files.values()), "wrong PNG set or a failure screenshot exists")
    images = {name: decode(path, SHOTS[name]) for name, path in files.items()}
    paint = report["counts_and_colors"]
    samples = {name: colors(images[name], paint[name], wide[-1], 0, name) for name in ("light", "dark")}
    before, after = (images[name] for name in ("rtl-large", "rtl-large-scrolled"))
    for name, image, stage in (("rtl-large", before, "initial_paint"),
                               ("rtl-large-scrolled", after, "final_paint")):
        samples[name] = colors(image, compact[stage], layout, 3, name)
        need(samples[name]["surface_rgb"] == samples["dark"]["surface_rgb"],
             "the compact viewport did not keep the dark surface")
    need(sum(samples["light"]["surface_rgb"]) > sum(samples["dark"]["surface_rgb"]) + 192,
         "light and dark did not produce distinct surface pixels")
    need(crop(before, layout["bar"]).tobytes() == crop(after, layout["bar"]).tobytes()
         and crop(before, headline).tobytes() == crop(after, headline).tobytes(),
         "the bar or title pixels moved when the content scrolled")
    need(ImageChops.difference(crop(before, scroll), crop(after, scroll)).getbbox() is not None,
         "wheel screenshots did not show changing content")

    focus_x, focus_y, _, _ = layout["destinations"][4]["hit"]
    ink = rgb(compact["selected_label_rgb"])
    need(before.getpixel(px(focus_x + 1, focus_y + 2)) == rgb(compact["focused_outline_rgb"]) == ink,
         "decoded Settings outline did not show the reported focus ink")
    x, y, w, h = layout["destinations"][3]["label"]
    need(h >= 64, "selected label did not wrap to two visible lines")
    line_pixels = [ink_count(crop(before, [x, y + line * 32, w, 32]), ink) for line in range(2)]
    need(min(line_pixels) > 32 and line_pixels == compact["selected_line_pixels"],
         "the selected label did not paint its ink on both reported lines")
    return {
        "geometry": {"wide": wide, "compact": layout},
        "accessibility": accessibility,
        "actions": actions,
        "wheel": {"before": entry_before, "after": entry_after, "scroll_dp": entry_before[1] - entry_after[1],
                  "bar_and_title_pixels_unchanged": True, "focus_after": compact["focused_after_wheel"],
                  "focus_restored": compact["restored_focus"], "line_pixels": line_pixels},
        "pngs": {key: {"size": images[key].size, "sha256": digest(path), **samples[key]}
                 for key, path in files.items()},
    }


def validate(run, *, platform, scope="stock", log=None, window_receipt=None,
             expected_metadata=None, expect_swiftshader=False, stock_last_material=273):
    run = Path(run)
    stock = scope == "stock"
    need(not expect_swiftshader or (stock and platform == "linux"), "SwiftShader pin requires Linux stock mode")
    log_path = Path(log) if log is not None else run / "runner.log"
    counts, steps, total, logged_browsers = validate_log(log_path.read_text(), platform, stock, stock_last_material=stock_last_material)
    report_path = run / f"browser-{STEM}.json"
    report = read_json(report_path)
    need(report["errors"] == [] and not {"failure", "state_at_failure"} & report.keys(),
         "Navigation Bar emitted a browser failure or page error")
    need(report["reduced_motion"] is True, "the static capture did not report Reduce Motion")
    browser, adapter = metadata(report)
    need(logged_browsers == {browser}, "the Navigation browser does not match the runner log")

    comparison = {}
    if stock:
        smoke_path = run / "browser-smoke.json"
        smoke = read_json(smoke_path)
        need(smoke["errors"] == [] and metadata(smoke) == (browser, adapter),
             "the Navigation browser or adapter differs from stock Smoke")
        comparison["stock_smoke_sha256"] = digest(smoke_path)
    if expected_metadata is not None:
        expected = Path(expected_metadata)
        need(metadata(read_json(expected)) == (browser, adapter),
             "the browser or adapter differs from the expected runtime metadata")
        comparison["expected_metadata_sha256"] = digest(expected)
    if expect_swiftshader:
        need(adapter == SWIFTSHADER, "the observed Linux WebGPU adapter did not match the SwiftShader pin")

    if platform == "macos":
        default = "headless-window-observation.json" if stock else "headless-observation.json"
        windows = mac_observation(Path(window_receipt) if window_receipt else run / default, stock)
        if stock:
            attempt = read_json(run / "attempt.json")
            need(attempt["runner_exit"] == attempt["tee_exit"] == 0 and attempt["headless_flag"] == "--headless=new"
                 and attempt["material_steps_started"] == steps and attempt["failure_dumps"] == []
                 and attempt["remaining_owned_processes"] == [], "the local stock runner attempt did not pass")
        else:
            need((run / "exit.txt").read_text() == "runner=0 tee=0\n", "the focused runner or its log failed")
    else:
        need(window_receipt is None, "CoreGraphics observation is only supported for macOS proof")
        windows = None

    return {
        "schema": "surface.test.flexible-navigation-portable-verification.v1",
        "platform": platform, "case_scope": scope, "navigation_steps": NAV_STEPS,
        "material_steps": len(steps), "case_results": counts, "total_checks": total,
        "browser": browser, "adapter": adapter, "metadata_comparison": comparison, "window_observation": windows,
        "page_errors": 0, "report_sha256": digest(report_path), "runner_log_sha256": digest(log_path),
        **validate_navigation(report, run),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path, help="Directory containing the original Chrome report and PNGs")
    parser.add_argument("--platform", required=True, choices=("macos", "linux"))
    parser.add_argument("--scope", choices=("stock", "focused"), default="stock")
    parser.add_argument("--log", type=Path, help="Raw runner log or GitHub Actions step log")
    parser.add_argument("--window-receipt", type=Path, help="Override the macOS CoreGraphics receipt path")
    parser.add_argument("--expected-metadata", type=Path, help="Independent same-run JSON with browser and adapter")
    parser.add_argument("--expect-swiftshader", action="store_true", help="Require google/swiftshader fallback in Linux stock")
    args = parser.parse_args()
    try:
        print(json.dumps(validate(args.run, platform=args.platform, scope=args.scope, log=args.log,
                                  window_receipt=args.window_receipt, expected_metadata=args.expected_metadata,
                                  expect_swiftshader=args.expect_swiftshader), indent=2))
    except (ValueError, KeyError, TypeError, IndexError, OSError, SyntaxError) as error:
        parser.exit(1, f"Flexible Navigation Bar proof rejected: {error}\n")


if __name__ == "__main__":
    main()
