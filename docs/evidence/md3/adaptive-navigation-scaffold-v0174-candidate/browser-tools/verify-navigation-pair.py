#!/usr/bin/env python3
"""Read the seven Navigation captures and the local scoped-run receipts offline."""

import argparse
from copy import deepcopy
from hashlib import sha256
import json
from math import floor
from pathlib import Path
from runpy import run_path
from tempfile import TemporaryDirectory

from PIL import Image, ImageChops

from evidence_common import clean_windows, seal, verify_ledgers


ROOT = Path(__file__).resolve().parent
LAUNCHER_SHA = "14f92282fe4ef079c5dbf88aef59bba6ad733cb2b12d9d0842d93d328a9dd998"
LEGACY = ROOT / "verify-compact-predecessor.py"
LEGACY_SHA = "53eb0fa5d730a1736be585fc8c78da290a5d17c919f5de6ec0cd11793adc22f2"
if sha256(LEGACY.read_bytes()).hexdigest() != LEGACY_SHA:
    raise SystemExit("the predecessor Compact verifier differs from its pin")
OLD = run_path(str(LEGACY))
PREP = run_path(str(ROOT / "prepare-focused.py"))
need, read_json, digest = OLD["need"], OLD["read_json"], OLD["digest"]
rect, bottom, contains = OLD["rect"], OLD["bottom"], OLD["contains"]
decode, px, rgb, ink_count = OLD["decode"], OLD["px"], OLD["rgb"], OLD["ink_count"]
IDS = OLD["IDS"]
STEM = "material-horizontal-navigation-bar"
COMPACT = f"browser-{OLD['STEM']}.json"
SHOTS = {"light": (839, 760), "rtl-large": (720, 350), "rtl-large-scrolled": (720, 350)}


def right(rectangle):
    return rectangle[0] + rectangle[2]


def near(actual, expected):
    return abs(actual - expected) <= 0.75


def geometry(layout, width, height, count, rtl=False):
    bar = rect(layout["bar"])
    scroll = rect(layout["scroll"])
    need(bar[0] == scroll[0] == 0 and bar[2] == scroll[2] == width
         and bottom(bar) == height and bottom(scroll) == bar[1]
         and (bar[3] > 64 if rtl else bar[3] == 64), "wrong page scroller or anchored bar")
    items = layout["destinations"]
    need([item["id"] for item in items] == IDS[:count], "wrong authored order")
    start = (width - 112 * count) / 2
    for index, row in enumerate(items):
        hit, pill, icon, label = (rect(row[key]) for key in ("hit", "pill", "icon", "label"))
        visual_index = count - index - 1 if rtl else index
        need(all(near(actual, expected) for actual, expected in
                 zip(hit, (start + visual_index * 112, bar[1], 112, bar[3]), strict=True))
             and min(hit[2:]) >= 48, "wrong width or physical ordering for a destination target")
        need(contains(hit, pill) and contains(pill, label) and contains(pill, icon)
             and near(pill[0] + pill[2] / 2, hit[0] + hit[2] / 2)
             and near(pill[1] + pill[3] / 2, hit[1] + hit[3] / 2)
             and near(pill[2], 60 + label[2]) and 0 < label[2] <= 52
             and near(pill[3], max(label[3], 24) + 16),
             "the hit, pill and label rectangles lost measured alignment")
        gap = icon[0] - right(label) if rtl else label[0] - right(icon)
        need(icon[2:] == [24, 24] and near(gap, 4)
             and near(icon[0], right(pill) - 40 if rtl else pill[0] + 16)
             and near(icon[1], pill[1] + (pill[3] - 24) / 2),
             "the leading icon or its gap does not follow writing direction")
    return layout


def crop(image, bounds):
    # Rust Raster::colors rounds positive coordinates to the nearest device pixel.
    x, y, w, h = bounds
    edges = tuple(floor(value * 2 + 0.5) for value in (x, y + 44, x + w, y + h + 44))
    need(0 <= edges[0] < edges[2] <= image.width
         and 0 <= edges[1] < edges[3] <= image.height, "crop falls outside the captured PNG")
    return image.crop(edges)


def paint(image, proof, layout, index):
    surface, fill, ink = (rgb(proof[key]) for key in ("surface", "indicator", "ink"))
    need(len({surface, fill, ink}) == 3, "bar, pill and ink must use different colours")
    row = layout["destinations"][index]
    x, y, _, h = row["pill"]
    need(image.getpixel(px(layout["bar"][0] + 3, y + h / 2)) == surface
         and image.getpixel(px(x + 6, y + h / 2)) == fill
         and image.getpixel(px(x, y)) == surface,
         "decoded Horizontal surface, pill fill or rounded corner disagrees with the report")
    icon, label = (ink_count(crop(image, row[key]), ink) for key in ("icon", "label"))
    need(icon > 24 and label > 16 and [icon, label] == [proof["icon_pixels"], proof["label_pixels"]],
         "decoded Horizontal icon or label ink counts disagree with the report")
    return {"surface_rgb": surface, "pill_rgb": fill, "ink_rgb": ink,
            "icon_pixels": icon, "label_pixels": label}


def focus_ring(image, proof, layout, expected):
    ink = rgb(proof["focus_ink"])
    need(ink == expected and ink != rgb(proof["surface"]),
         "Horizontal focus ink must use the inherited Secondary token")
    left, top, width, _ = layout["destinations"][4]["hit"]
    samples = [image.getpixel(px(left + width / 2, top + depth))
               for depth in (0.75, 1.75, 2.75)]
    need(samples == [ink] * 3, "decoded Settings does not show its 3 dp focus ring")
    return {"focused": IDS[4], "stroke_dp": 3, "samples_rgb": samples}


def medium(artifacts, compact_report):
    path = artifacts / f"{STEM}.json"
    report = read_json(path)
    layouts = [geometry(layout, width, 760, count) for (width, count), layout in zip(
        ((width, count) for width in (600, 839) for count in (3, 4, 5)),
        report["layout"]["layouts"], strict=True,
    )]
    interaction = report["interaction"]
    need(interaction["ax"] == compact_report["accessibility"]
         and interaction["widths"] == [839, 599, 600]
         and interaction["focus"] == IDS[2] and interaction["touch_cancelled"] is True,
         "Horizontal accessibility, contact cancellation or breakpoint focus proof differs")
    actions = interaction["actions"]
    compact_actions = compact_report["actions"]
    need(actions.keys() == compact_actions.keys()
         and all(value == compact_actions[key] for key, value in actions.items() if key != "visible_titles")
         and len(actions["visible_titles"]) == 3
         and all(contains([0, 0, 839, layouts[-1]["scroll"][1]], rect(title))
                 for title in actions["visible_titles"]),
         "Horizontal trusted action or visible heading proof differs")
    rtl = report["rtl"]
    tight = geometry(rtl["layout"], 720, 350, 5, rtl=True)
    need(rtl["current"] == IDS[3] and rtl["focus"] == IDS[4]
         and tight["destinations"][3]["label"][3] >= 64,
         "the dark selected label must wrap across at least two lines")

    files = {tag: artifacts / f"{STEM}-{tag}.png" for tag in SHOTS}
    need(set(artifacts.glob(f"{STEM}-*.png")) == set(files.values()), "wrong Horizontal PNG inventory")
    images = {tag: decode(file, SHOTS[tag]) for tag, file in files.items()}
    paints = {"light": report["layout"]["paint"], "rtl-large": rtl["initial"], "rtl-large-scrolled": rtl["scrolled"]}
    results = {}
    for tag, file in files.items():
        need(paints[tag]["image"] == file.name, "the raw report points to a different PNG")
        wide = tag == "light"
        colors = paint(images[tag], paints[tag], layouts[-1] if wide else tight, 0 if wide else 3)
        compact = compact_report["counts_and_colors"]["light" if wide else "dark"]
        need([colors[key] for key in ("surface_rgb", "pill_rgb", "ink_rgb")]
             == [rgb(compact[key]) for key in ("surface", "pill", "icon_ink")],
             "the Horizontal bar changed the inherited light or dark palette")
        if wide:
            need(paints[tag]["focus_ink"] is None, "unexpected separate focus witness in the light frame")
        else:
            colors["focus"] = focus_ring(images[tag], paints[tag], tight,
                                         rgb(compact_report["compact"]["focused_outline_rgb"]))
        results[tag] = {"size": images[tag].size, "sha256": digest(file), **colors}
    before, after = images["rtl-large"], images["rtl-large-scrolled"]
    need(ImageChops.difference(crop(before, tight["scroll"]), crop(after, tight["scroll"])).getbbox()
         is not None, "the independent PNGs do not show content moving under the wheel")
    x, y, width, height = tight["destinations"][3]["label"]
    ink = results["rtl-large"]["ink_rgb"]
    line_ink = [ink_count(crop(before, [x, y + 32 * index, width, 32]), ink)
                for index in range(floor(height / 32))]
    need(sum(count > 16 for count in line_ink) >= 2, "the RTL artwork must show at least two legible lines")
    return {"report_sha256": digest(path), "layouts": layouts, "rtl_layout": tight,
            "rtl_line_ink": line_ink, "interaction": interaction, "pngs": results}


def verify(run):
    watch = read_json(run / "headless-window-observation.json")
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary"})
    need("CoreGraphics" in watch["method"] and read_json(run / "owned-shutdown.json")["after_cleanup"] == [],
         "missing CoreGraphics sampling or final process reaping")
    argv = watch["guard_events"][0]["argv"]
    extra = "--disable-updater-scheduler"
    need(argv.count(extra) == 1 and "--" not in argv, "the guarded Chrome launch flags differ")
    invocation = read_json(run / "invocation.json")
    pins = invocation["inputs"]
    seal(ROOT, "launcher.sha256", LAUNCHER_SHA)
    prep = {**PREP["check_scope"](), **PREP["check_gallery"]()}
    runner = ROOT / "pins/surface-e2e"
    need(invocation["scope"] == PREP["SCOPE"] and invocation["suite_arguments"] == ["material"]
         and invocation["injected_chrome_flags"] == [extra]
         and pins == read_json(run / "inputs-after.json")
         and pins == {**prep, "runner_sha256": digest(runner)}
         and (ROOT / "runner.sha256").read_text().strip() == pins["runner_sha256"]
         and pins["web_producer_manifest_sha256"] == PREP["R2_SHA"]
         and pins["web_producer_tree"] == PREP["R2_TREE"]
         and pins["web_compatible_test_only_delta"] == PREP["WEB_DELTA"],
         "the run scope, pre/post inputs or actual Gallery source differs")
    log = run / "runner.log"
    starts, cases, steps, summary, browsers = OLD["runner_result"](log.read_text())
    artifacts = run / "artifacts"
    compact_file = artifacts / COMPACT
    report = read_json(compact_file)
    browser, adapter = OLD["metadata"](report)
    need(starts == ["material"] and len(cases) == 1 and cases[0][:2] == ("material", "7")
         and steps == PREP["STEPS"] and summary == (1, 0, int(cases[0][2]))
         and summary[2] >= 458 and browsers == {browser},
         "the actual runner did not pass all seven ordered steps on the reported Chrome")
    need(report["errors"] == [] and not {"failure", "state_at_failure"} & report.keys()
         and report["reduced_motion"] is True, "Compact failed or had a browser error")
    return {
        "schema": "surface.test.focused-navigation-bars.v1",
        "scope": invocation["scope"], "steps": steps, "checks": summary[2],
        "browser": browser, "adapter": adapter, "inputs": pins, "runner_log_sha256": digest(log),
        "headless": {"sha256": digest(run / "headless-window-observation.json"),
                     "chrome_samples": watch["launch_observations"]["chrome-primary"]["samples"],
                     "visible_windows": 0, "remaining_owned_processes": 0},
        "compact": {"report_sha256": digest(compact_file),
                    **OLD["validate_navigation"](report, artifacts)},
        "horizontal": medium(artifacts, report),
    }


def self_test(artifacts):
    # Actual predecessor Compact archives and deliberate rejections. Synthetic pixels
    # below are solely a negative/positive control for the independent Horizontal reader.
    report = read_json(artifacts / COMPACT)
    summary = OLD["validate_navigation"](report, artifacts)
    wrong = deepcopy(report)
    wrong["compact"]["geometry"]["destinations"].reverse()
    try:
        OLD["validate_navigation"](wrong, artifacts)
    except ValueError:
        pass
    else:
        raise ValueError("reversed Compact authored order was accepted")
    icon, label = [160, 696, 24, 24], [188, 699, 32, 18]
    row = {"pill": [144, 688, 92, 40], "icon": icon, "label": label}
    focus_hit = [392, 676, 112, 64]
    layout = {"bar": [0, 676, 600, 64], "destinations": [row, {}, {}, {}, {"hit": focus_hit}]}
    palette = {"surface": [249, 245, 251], "indicator": [223, 215, 236], "ink": [43, 32, 56]}
    image = Image.new("RGB", (1200, 1568), rgb(palette["surface"]))
    image.putpixel(px(150, 708), rgb(palette["indicator"]))
    for bounds, key in ((icon, "icon_pixels"), (label, "label_pixels")):
        shape = crop(image, bounds)
        palette[key] = shape.width * shape.height
        image.paste(rgb(palette["ink"]), (*px(bounds[0], bounds[1]),
                                        *px(right(bounds), bottom(bounds))))
    palette["focus_ink"] = [224, 181, 225]
    focus_x, focus_y, focus_width, _ = focus_hit
    focus_pixels = [px(focus_x + focus_width / 2, focus_y + depth)
                    for depth in (0.75, 1.75, 2.75)]
    for point in focus_pixels:
        image.putpixel(point, rgb(palette["focus_ink"]))
    with TemporaryDirectory(dir=ROOT) as scratch:
        file = Path(scratch) / "synthetic.png"
        image.save(file)
        sampled = decode(file, (600, 740))
        paint(sampled, palette, layout, 0)
        focus_ring(sampled, palette, layout, rgb(palette["focus_ink"]))
        wrong_focus = sampled.copy()
        wrong_focus.putpixel(focus_pixels[2], rgb(palette["surface"]))
        try:
            focus_ring(wrong_focus, palette, layout, rgb(palette["focus_ink"]))
        except ValueError:
            pass
        else:
            raise ValueError("incomplete Horizontal focus stroke passed")
        for point in (px(150, 708), px(161, 697)):
            wrong_image = sampled.copy()
            wrong_image.putpixel(point, rgb(palette["surface"]))
            try:
                paint(wrong_image, palette, layout, 0)
            except ValueError:
                continue
            raise ValueError(f"altered Horizontal control pixel passed at {point}")
    return {"control": "offline controls only", "compact_pngs": summary["pngs"],
            "rejected": ["reversed Compact authored order",
                         "incorrect Horizontal fill", "modified Horizontal ink count",
                         "incomplete Horizontal focus stroke"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    target = parser.add_mutually_exclusive_group(required=True)
    target.add_argument("--run", type=Path)
    target.add_argument("--compact-control", type=Path)
    args = parser.parse_args()
    payload = verify(args.run) if args.run else self_test(args.compact_control)
    print(json.dumps(payload, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
