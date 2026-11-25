"""Read original q72 Canvas images and scoped headless receipts offline."""

import argparse
import json
from math import floor
from pathlib import Path
import re
from runpy import run_path

from PIL import Image

from evidence_common import clean_windows, data, digest, require, verify_ledgers

ROOT = Path(__file__).resolve().parent
PREFIX = "browser-material-text-field-system-pair"
DRAFT = "1688"
FIELD = {"background": [19, 45, 47, 255], "foreground": [249, 220, 126, 255]}
WIDE = ("normal", "selected", "caret", "restored")
COMPACT = ("normal", "distinct", "malformed-fallback", "restored")
STEPS = [f"M{i}" for i in (*range(241, 248), *range(357, 363))]
SUFFIXES = {"real-light", "real-dark", *(f"{tag}-{stage}" for tag in ("outlined", "filled") for stage in WIDE),
            *(f"compact-{stage}" for stage in COMPACT)}
RETAINED_ORACLE = "442ede58283ac8f39cfc3a7ff355e94c53f0639a1130b34bd75a67d789f909eb"


def name(suffix):
    return f"{PREFIX}-{suffix}.png"


def crop(image, rect):
    x, y, w, h = rect
    box = tuple(max(0, floor(2 * v + 0.5)) for v in (x, y, x + w, y + h))
    left, top, right, bottom = box
    require(0 <= left < right <= image.width and 0 <= top < bottom <= image.height,
            "A measured field region is outside the original Canvas")
    return image.crop(box)


def pixel(image, witness):
    x, y = (floor(2 * value) for value in witness["point"])
    require(0 <= x < image.width and 0 <= y < image.height
            and list(image.getpixel((x, y))) == witness["rgb"], "Exact Canvas point disagrees with the receipt")


def ink(image, witness):
    expected = tuple(witness["ink"])
    count = sum(color == expected for color in crop(image, witness["rect"]).getdata())
    require(count == witness["count"] and count >= witness["minimum"] > 0,
            "Exact Canvas glyph ink disagrees with the receipt")


def witnesses(image, item):
    if isinstance(item, list):
        return sum(witnesses(image, value) for value in item)
    if not isinstance(item, dict):
        return 0
    if set(item) == {"point", "rgb"}:
        pixel(image, item)
        return 1
    if set(item) == {"rect", "ink", "count", "minimum"}:
        ink(image, item)
        return 1
    return sum(witnesses(image, value) for value in item.values())


def verify_editor(editor, anchor, cursor):
    direction = "backward" if anchor > cursor else "none"
    require(editor == {"active": True, "dom": DRAFT, "value": DRAFT, "composing": False, "caller": DRAFT,
                       "browser": [min(anchor, cursor), max(anchor, cursor), direction], "runtime": [anchor, cursor]},
            "The actual browser and Runtime editor range differ")


def rgba(rgb):
    return [*rgb, 255]


def source_over(source, base, mode):
    require(len(source) == 4 and len(base) == 3 and all(type(channel) is int and 0 <= channel <= 255
            for channel in source + base), "Expected an RGBA source over an opaque RGB base")
    require(mode in ("byte-space source-over", "linear-light source-over"), "Unknown original Canvas blend mode")
    alpha = source[3] / 255
    if mode == "byte-space source-over":
        return [floor(s * alpha + b * (1 - alpha) + 0.5) for s, b in zip(source, base)]

    def decode(channel):
        value = channel / 255
        return value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4

    def encode(value):
        return 12.92 * value if value <= 0.0031308 else 1.055 * value ** (1 / 2.4) - 0.055

    return [floor(255 * encode(decode(s) * alpha + decode(b) * (1 - alpha)) + 0.5) for s, b in zip(source, base)]


def verify_highlight(selection, source, base):
    blend = selection["highlight_composite"]
    require(set(blend) == {"mode", "source_rgba", "base_rgb", "result_rgb"}
            and blend["source_rgba"] == source and blend["base_rgb"] == base
            and source_over(source, base, blend["mode"]) == blend["result_rgb"] == selection["highlight"]["ink"],
            "Highlight is not the exact witnessed source over Field")


def verify_distinct(colors, css):
    require(colors["field"] == FIELD and css["field"] == css["pairs"]["canvas"] != FIELD
            and all(colors[role] == css["pairs"][role] for role in ("canvas", "button", "highlight"))
            and colors["disabled_text"] == css["disabled_text"]["rgba"], "Incorrect real and synthetic CSS scope")


def verify_restored(initial, restored, rects, changes):
    require(changes == [0] * len(rects)
            and all(crop(initial, rect).tobytes() == crop(restored, rect).tobytes() for rect in rects),
            "Normal paint did not restore byte-for-byte in the tested regions")


def verify_captures(artifacts):
    report_path = artifacts / f"{PREFIX}.json"
    report = data(report_path)
    require(report_path.is_file() and not report_path.is_symlink() and report["page_errors"] == []
            and not {"failure", "failure_state", "failure_dom"} & report.keys() and report["dpr"] == 2
            and report["real_source"] == "CDP emulated system CSS with no palette override"
            and report["distinct_source"].startswith("synthetic unequal Field pair scoped to Surface optional probe; "),
            "The new q72 paint case did not finish in its approved scope")
    require({p.name for p in artifacts.glob(f"{PREFIX}*.png")} == {name(suffix) for suffix in SUFFIXES},
            "Expected 14 original q72 Canvas PNGs")
    hashes = {}

    def capture(suffix, size):
        path = artifacts / name(suffix)
        require(path.is_file() and not path.is_symlink(), "Expected an original Canvas capture")
        with Image.open(path) as raw:
            require(raw.format == "PNG" and list(raw.size) == size and raw.width == (640 if suffix.startswith("compact-") else 1920),
                    f"Wrong original Canvas dimensions for {suffix}")
            raw.load()
            image = raw.convert("RGB")
        hashes[path.name] = digest(path)
        return image

    probes = {}
    require([row["scheme"] for row in report["real"]] == ["light", "dark"], "Both genuine CSS palettes are required")
    for row in report["real"]:
        shade = 255 if row["scheme"] == "light" else 0
        pair = {"background": rgba([shade] * 3), "foreground": rgba([255 - shade] * 3)}
        suffix = "real-" + row["scheme"]
        require(row["png"] == name(suffix) and row["css"]["field"] == row["css"]["pairs"]["canvas"] == row["accepted"]["field"] == pair
                and row["quiet_face"]["rgb"] == pair["background"][:3] and row["value_ink"]["ink"] == pair["foreground"][:3],
                "Genuine CSS Field differs from the browser-supported pair")
        verify_editor(row["editing"], 4, 4)
        probes[suffix] = witnesses(capture(suffix, row["size"]), row)

    for tag in ("outlined", "filled"):
        section = report[tag]
        verify_distinct(section["accepted"], section["css_witness"])
        verify_editor(section["editing"], 4, 2)
        require(section["ax"] == {"role": "textbox", "name": "專案名稱", "value": DRAFT, "focused": True, "same_name_nodes": 1},
                "Focus did not keep its one named accessible textbox")
        require(section["images"] == [name(f"{tag}-{stage}") for stage in WIDE], "Wide report changed its capture order")
        initial, active, caret, restored = [capture(f"{tag}-{stage}", section["size"]) for stage in WIDE]
        selected = section["selected"]
        field, text = FIELD["background"][:3], FIELD["foreground"][:3]
        highlight = section["accepted"]["highlight"]
        verify_highlight(selected["selection"], highlight["background"], field)
        require([point["rgb"] for point in selected["focus_edge"]] == [text] * 3 + [field]
                and selected["selection"]["ordinary"]["ink"] == text
                and selected["selection"]["selected"]["ink"] == highlight["foreground"][:3]
                and section["caret"]["pixel"]["rgb"] == text and section["caret"]["trailing_face"]["rgb"] == field,
                "The selection, caret or 3 dp focus treatment uses the wrong paired ink")
        if tag == "outlined":
            require(len(selected["notch"]["samples"]) == 8 and selected["notch"]["rtl"] is False,
                    "Missing outline notch witnesses")
        else:
            require(selected["notch"] is None, "Filled field reported an outline notch")
        probes[tag] = witnesses(active, selected) + witnesses(caret, section["caret"])
        rects = [section["geometry"][key] for key in ("face", "label", "support")]
        verify_restored(initial, restored, rects, section["restored_changed_pixels"])

    compact = report["compact"]
    verify_distinct(compact["accepted"], compact["css_witness"])
    verify_editor(compact["editing"], 4, 4)
    require(compact["viewport"] == [320, 960] and compact["rtl"] is True and compact["text_scale"] == 2
            and compact["images"] == [name(f"compact-{stage}") for stage in COMPACT], "Wrong compact RTL field scope")
    initial, active, missing, restored = [capture(f"compact-{stage}", compact["size"]) for stage in COMPACT]
    fallback = compact["fallback"]
    require(fallback["malformed_foreground"] == fallback["missing_background"]
            and fallback["missing_background"].get("field") is None
            and compact["paint"]["floating_notch"]["rtl"] is True
            and len(compact["paint"]["floating_notch"]["samples"]) == 8
            and len(compact["paint"]["closed_edge"]) == 4
            and compact["paint"]["wrong_caption_field_text_pixels"] == compact["paint"]["unwanted_left_field_text_pixels"] == 0,
            "Compact incomplete-pair or closed-outline scope changed")
    probes["compact"] = witnesses(active, compact["paint"]) + witnesses(missing, fallback["paint"])
    rects = [compact["geometry"][key] for key in ("face", "label")] + [compact["long"][key] for key in ("face", "above_label")]
    verify_restored(initial, restored, rects, compact["restored_changed_pixels"])
    require(len(hashes) == 14 and all(probes[part] > 10 for part in ("outlined", "filled", "compact")),
            "Independent pixel witnesses are incomplete")
    return {"browser": report["browser"], "adapter": report["adapter"], "report_sha256": digest(report_path),
            "png_sha256": hashes, "decoded_probes": probes, "real_source": report["real_source"],
            "distinct_source": report["distinct_source"]}


def verify_run(run):
    artifacts = run / "artifacts"
    actual = verify_captures(artifacts)
    retained = ROOT / "verify-field-contrast.py"
    require(digest(retained) == RETAINED_ORACLE, "Retained disabled-field oracle changed")
    prior = run_path(str(retained))["verify_captures"](artifacts)
    require(all(prior[key] == actual[key] for key in ("browser", "adapter"))
            and len(prior["png_sha256"]) == 15 and actual["adapter"]["vendor"] == "apple"
            and actual["adapter"]["architecture"].startswith("metal")
            and actual["adapter"]["isFallbackAdapter"] is False, "Headless macOS WebGPU proof is incomplete")
    log = (run / "runner.log").read_text()
    checks = re.findall(r"^case material \.\.\. ok \(13 steps, (\d+) checks,", log, re.M)
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"]
            and re.findall(r"^  \[(M\d+)\]", log, re.M) == STEPS
            and len(checks) == 1 and int(checks[0]) >= 13
            and re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", log, re.M) == checks,
            "Wrong actual scoped step ledger")
    watch = data(run / "headless-window-observation.json")
    clean_windows(watch)
    verify_ledgers(run, watch, {"chrome-primary"})
    inputs = data(run / "invocation.json")
    event = watch["guard_events"][0]
    require("--" not in event["argv"] and event["argv"].count("--disable-updater-scheduler") == 1
            and not data(run / "owned-shutdown.json")["after_cleanup"]
            and inputs["suite_arguments"] == ["material"]
            and inputs["injected_chrome_flags"] == ["--disable-updater-scheduler"]
            and inputs["inputs"] == data(run / "inputs-after.json"), "The guarded input or process lifecycle changed")
    return {**actual, "retained_disabled_contrast": prior, "checks": int(checks[0]), "steps": STEPS,
            "chrome_samples": watch["launch_observations"]["chrome-primary"]["samples"], "visible_owned_windows": 0,
            "log_sha256": digest(run / "runner.log"), "inputs_sha256": digest(run / "inputs-after.json"),
            "window_receipt_sha256": digest(run / "headless-window-observation.json")}


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("run", type=Path)
    print(json.dumps(verify_run(cli.parse_args().run), ensure_ascii=False, sort_keys=True, indent=2))
