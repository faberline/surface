#!/usr/bin/env python3
"""Recompute the Material label motion evidence from original Chrome artifacts."""

from hashlib import sha256
import json
import math
from pathlib import Path
import re
from runpy import run_path

from PIL import Image

PREFIX = "material-text-field-label-motion"
TITLE = "專案名稱"
HELP = "請輸入中文或英文名稱。"
LONG_TITLE = "輸入團隊成員在共用專案清單中容易辨認的完整名稱"
DPR = 2
HEADER = 44
ROOT = Path(__file__).resolve().parent
FOCUSED = Path("/tmp/surface-md3-label-motion-e2e.PywRddEC")
FOCUSED_SEAL = "a77ef3675e2743cbc7030a918f0b26a7ac61c00539439fa8f6a2fe2c9f0ff8db"
FOCUSED_RECEIPT = "3f693f657ca2d61268b54e3e1fe5a9092261954b365276a9e901cdc41ad1372b"
FOCUSED_VERIFIER = "eceb8f1b5490883f3d496ee4927346304c899c0720641a7ed90050099ba5b6b2"
FOCUSED_SOURCE = "9a3755f9e41a87318692674089be2d705e6a09a13d305330a2df092026e584da"
FOCUSED_RUNNER = "1db87fe350d4051190d1212d75a41eebe98098181def36ab24308502352ced3b"
FOCUSED_WEB = "5ee0d1175d5c590d39b1c4d4438e241457e391a4abc7711fcf9b73334802ca43"
EXACT_SOURCE = "4aa53964b34f033b59b5ab723e58deb612d5db8ce1975bd3cf2115e687479271"
FEATURE_SOURCE = "76a8a1726a04f0fdee80aeb27b24f784c38222db73b24cc475ff0c6c89950daa"
UPDATER_FLAG = "--disable-updater-scheduler"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    return sha256(path.read_bytes()).hexdigest()


def color(palette, token):
    rgba = palette[token]
    require(len(rgba) == 4 and rgba[3] == 255 and all(type(n) is int and 0 <= n <= 255 for n in rgba),
            f"{token}: invalid opaque theme color")
    return tuple(rgba[:3])


def image_for(path, compact):
    require(path.is_file() and not path.is_symlink(), f"{path.name}: no original")
    expected = (640, 2008) if compact else (1920, 1728)
    with Image.open(path) as original:
        require(original.format == "PNG" and original.size == expected,
                f"{path.name}: expected original PNG {expected}")
        original.verify()
    with Image.open(path) as original:
        return original.convert("RGB")


def px(image, x, y):
    return image.getpixel((math.floor(x * DPR), math.floor((y + HEADER) * DPR)))


def region(image, rect, ink):
    x, y, width, height = rect
    # Coordinates use positive logical Canvas positions at DPR 2.
    def rounded(n):
        return math.floor(n * DPR + .5)

    box = tuple(map(rounded, (x, y + HEADER, x + width, y + HEADER + height)))
    require(0 <= box[0] < box[2] <= image.width and 0 <= box[1] < box[3] <= image.height,
            "paint crop out of the original PNG")
    crop = image.crop(box)
    counts = crop.getcolors(crop.width * crop.height)
    return next((count for count, pixel in counts if pixel == ink), 0)


def check_contains(rect, compact):
    x, y, width, height = rect
    viewport = (320, 960) if compact else (960, 820)
    require(width > 0 and height > 0 and 0 <= x < x + width <= viewport[0]
            and 0 <= y < y + height <= viewport[1], f"outside Canvas: {rect}")


def verify_frame(artifacts, scene, frame, suffix, ms, appearance, focused):
    require(appearance in ("outlined", "filled", "compact"), "wrong appearance")
    compact = appearance == "compact"
    filename = f"{PREFIX}-{suffix}.png"
    require(frame["png"] == filename and frame["ms"] == ms, f"{suffix}: incorrect timestamp or capture")
    path = artifacts / filename
    image = image_for(path, compact)
    palette = scene["palette"]
    ink = color(palette, "primary" if focused else "on_surface_variant")
    label = frame["label"]
    check_contains(label, compact)
    require(frame["label_pixels"] > 12 and region(image, label, ink) >= frame["label_pixels"],
            f"{suffix}: reported label missing in the original PNG")
    if appearance in ("outlined", "compact"):
        paper = color(palette, "background")
        border = color(palette, "primary" if focused else "outline")
        x, y, width, height = scene["face"]
        left, _, span, _ = scene["floated"]
        anchor = scene.get("anchor", left)
        x_padding = anchor + (2 if compact else -2)
        actual_notch = [px(image, x_padding, y + .25), px(image, x_padding, y + 2.5)]
        actual_sides = [
            px(image, x + .5, y + height / 2),
            px(image, x + width - .5, y + height / 2),
            px(image, x + width / 2, y + height - .5),
        ]
        columns = range(math.ceil((left - 4) * DPR), math.floor((left + span + 4) * DPR))
        gap = sum(px(image, (column + .5) / DPR, y + .25) == paper for column in columns)
        require(actual_notch == list(map(tuple, frame["notch"]))
                and actual_sides == list(map(tuple, frame["outline_sides"])) == [border] * 3
                and gap == frame["gap_pixel_columns"], f"{suffix}: original notch/sides differ")
    if appearance != "compact" and focused:
        x, inline_top, _, height = scene["inline"]
        _, float_top, span, float_height = scene["floated"]
        top = inline_top if appearance == "outlined" else float_top + float_height
        clip = [x, top, span * 1.5 + 24, height]
        placeholder = region(image, clip, color(palette, "on_surface_variant"))
        require(placeholder == frame["placeholder_pixels"],
                f"{suffix}: placeholder pixels differ from the original capture")
    return sha(path)


def read(path):
    return json.loads(path.read_text())


def manifest(path):
    entries = {}
    for line in path.read_text().splitlines():
        digest, name = line.split("  ", 1)
        relative = Path(name)
        require(re.fullmatch(r"[0-9a-f]{64}", digest) and not relative.is_absolute()
                and ".." not in relative.parts and name not in entries, f"bad manifest input {name}")
        entries[name] = digest
    return entries


def verify(artifacts, metadata):
    path = artifacts / f"{PREFIX}.json"
    report = read(path)
    require(report["browser"] == metadata["browser"] and report["adapter"] == metadata["adapter"],
            "label motion Chrome or WebGPU adapter differs from stock")
    require(re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", report["browser"]), "no actual browser version")
    require(report["dpr"] == DPR and report["visible_clock"] == "controlled performance.now"
            and report["page_errors"] == [] and "failure" not in report, "wrong clock or failed page")
    adapter = report["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal-")
            and adapter["isFallbackAdapter"] is False, "Apple Metal Chrome adapter missing")
    outlined, reversal, filled, compact = (report[key] for key in ("outlined", "reversal", "filled", "compact"))
    for appearance, scene in (("outlined", outlined), ("filled", filled), ("compact", compact)):
        check_contains(scene["face"], appearance == "compact")
        check_contains(scene["floated"], appearance == "compact")
    ch = {}
    groups = [
        (outlined, outlined, "outlined", ["rest", "first", "early", "middle", "settled"],
         [0, 0, 14, 44, 800], [False, True, True, True, True]),
        (outlined, reversal, "outlined", ["falling", "retarget", "reduced"],
         [844, 844, 844], [False, True, True]),
        (filled, filled, "filled", ["first", "middle", "settled"],
         [0, 44, 800], [True, True, True]),
        (compact, compact, "compact", ["rest", "middle"], [0, 44], [False, True]),
    ]
    for scene, series, appearance, suffixes, times, focused in groups:
        prefix = "rtl" if appearance == "compact" else appearance
        for frame, suffix, ms, active in zip(series["frames"], suffixes, times, focused, strict=True):
            ch[frame["png"]] = verify_frame(artifacts, scene, frame, f"{prefix}-{suffix}", ms, appearance, active)

    rest, first, early, middle, end = outlined["frames"]
    falling, retarget, reduced = reversal["frames"]
    paper = outlined["palette"]["background"][:3]
    focus_color = outlined["palette"]["primary"][:3]
    require(rest["label"] == first["label"] and rest["gap_pixel_columns"] == first["gap_pixel_columns"] == 0
            and first["placeholder_pixels"] == early["placeholder_pixels"] == 0
            and early["notch"] == [paper, focus_color] and middle["notch"] == end["notch"] == [paper, paper]
            and 8 <= early["gap_pixel_columns"] < middle["gap_pixel_columns"]
            and first["label"][1] > early["label"][1] > middle["label"][1] > end["label"][1],
            "the Outlined notch or label trajectory changed")
    require(falling["ms"] == retarget["ms"] == reduced["ms"]
            and falling["label"] == retarget["label"] and reduced["label"] == end["label"]
            and reduced["notch"] == end["notch"] and end["placeholder_pixels"] > 12,
            "reversal or reduced endpoint changed")
    trusted = reversal["trusted"]
    require([key["key"] for key in trusted] == ["Tab"] * 3
            and [key["shift"] for key in trusted] == [False, True, False]
            and trusted[1]["id"] == "surface-text-input", "trusted focus route changed")
    for actual, active in ((outlined["proxy"], False), (outlined["active"], True),
                           (reversal["blurred"], False), (filled["ax"], True)):
        require(actual["label"] == TITLE and actual["description"] == HELP and actual["focused"] is active,
                "textbox name, description or focus changed")
    require(outlined["proxy"]["backend"] != outlined["active"]["backend"],
            "AX projection did not move to the live textarea")

    fill_start, fill_mid, fill_end = filled["frames"]
    preedit = filled["preedit"]
    filename = f"{PREFIX}-filled-preedit.png"
    require(preedit["png"] == filename and preedit["ms"] == 44 and preedit["placeholder_pixels"] == 0
            and fill_start["label"][1] > fill_mid["label"][1] > fill_end["label"][1]
            and fill_start["placeholder_pixels"] == fill_mid["placeholder_pixels"] == 0
            and fill_end["placeholder_pixels"] > 12, "Filled progression or preedit changed")
    x, _, _, height = filled["inline"]
    _, top, width, floating_height = filled["floated"]
    clearance = [x, top + floating_height, width * 1.5 + 24, height]
    image = image_for(artifacts / filename, False)
    require(region(image, clearance, color(filled["palette"], "on_surface")) == preedit["ink_pixels"] > 12
            and region(image, clearance, color(filled["palette"], "on_surface_variant")) == 0,
            "preedit PNG needs actual marked glyphs and no overlapping placeholder")
    ch[filename] = sha(artifacts / filename)

    closed, moving = compact["frames"]
    x, _, width, _ = compact["face"]
    require(x + width - compact["anchor"] == 16 and closed["gap_pixel_columns"] == 0
            and moving["gap_pixel_columns"] > 12
            and moving["label"][1] < closed["label"][1] - 4
            and moving["label"][0] > closed["label"][0] + 3
            and abs(sum(moving["label"][::2]) - sum(closed["label"][::2])) <= 3,
            "the compact RTL label loses its physical right anchor")
    fallback = compact["fallback"]
    filename = f"{PREFIX}-rtl-above-label.png"
    image = image_for(artifacts / filename, True)
    check_contains(fallback["above"], True)
    check_contains(fallback["face"], True)
    ink = color(compact["palette"], "primary")
    require(fallback["png"] == filename and fallback["ms"] == 800 and fallback["float"][2:] == [0, 0]
            and fallback["above"][3] >= 64 and fallback["above"][1] + fallback["above"][3] + 3.5 <= fallback["face"][1]
            and region(image, fallback["above"], ink) == fallback["pixels"] > 40
            and px(image, *fallback["solid_top"]) == ink and fallback["ax"]["label"] == LONG_TITLE
            and fallback["ax"]["focused"] is True, "dark RTL above-label fallback changed")
    ch[filename] = sha(artifacts / filename)
    require(len(ch) == 15 and set(ch) == {p.name for p in artifacts.glob(f"{PREFIX}-*.png")},
            "missing, unexpected or duplicate label motion originals")
    require(all(type(node["backend"]) is int and node["backend"] > 0 for node in (
        outlined["proxy"], outlined["active"], reversal["blurred"], filled["ax"], fallback["ax"]
    )), "missing actual Chrome AX backend IDs")
    pngs = {name: {"sha256": digest, "size": [640, 2008] if name.startswith(f"{PREFIX}-rtl") else [1920, 1728]}
            for name, digest in sorted(ch.items())}
    return {
        "report_sha256": sha(path),
        "clock": report["visible_clock"],
        "pngs": pngs,
        "outlined": {
            "geometry": {key: outlined[key] for key in ("face", "inline", "floated", "helper")},
            "frames": outlined["frames"],
            "inactive_ax": outlined["proxy"],
            "active_ax": outlined["active"],
        },
        "reversal": reversal,
        "filled": {
            "frames": filled["frames"],
            "active_ax": filled["ax"],
            "synthetic_cdp_preedit": preedit,
        },
        "compact_dark_rtl_200": compact,
    }


def focused_receipt(exact_manifest, canonical=None):
    run = FOCUSED / "development-r3"
    seal = FOCUSED / "EVIDENCE.sha256"
    verifier = ROOT / "verify-label-motion-focused-original.py"
    summary = FOCUSED / "development-r3-verified.json"
    require(sha(seal) == FOCUSED_SEAL and sha(verifier) == FOCUSED_VERIFIER
            and sha(summary) == FOCUSED_RECEIPT, "the original scoped seal, verifier or receipt differs")
    entries = manifest(seal)
    require(len(entries) == 81, "wrong original scoped evidence inventory")
    for name, digest in entries.items():
        require(sha(FOCUSED / name) == digest, f"original scoped input differs: {name}")

    feature = "e2e/src/material_text_field_label_motion.rs"
    scoped_source = FOCUSED / "source.sha256"
    require(sha(exact_manifest) == EXACT_SOURCE and sha(scoped_source) == FOCUSED_SOURCE
            and manifest(exact_manifest)[feature] == manifest(scoped_source)[feature]
            == sha(FOCUSED / "source" / feature) == FEATURE_SOURCE,
            "the exact and scoped M347–M350 feature test sources differ")
    original = run_path(str(verifier))
    observed = original["verify_run"](run, False)
    require(observed == read(summary) and observed["checks"] == 1495 and observed["steps"] == 26
            and len(observed["captures"]) == 15 and observed["observed_headless_samples"] == 256
            and observed["visible_windows"] == 0 and observed["remaining_owned_processes"] == []
            and observed["guard_passed"] is True, "the actual R3 scoped and guard result differs")
    invocation = read(run / "invocation.json")
    event, = read(run / "headless-window-observation.json")["guard_events"]
    require(invocation["argv"].count(UPDATER_FLAG) == event["argv"].count(UPDATER_FLAG) == 1
            and invocation["argv"][-1] == "material" and event["argv"].count("--headless=new") == 1
            and read(run / "owned-shutdown.json")["after_cleanup"] == [],
            "the scoped process-local launch option or shutdown proof differs")
    require(invocation["runner_sha256"] == sha(FOCUSED / "pins/surface-e2e") == FOCUSED_RUNNER
            and invocation["web_manifest_sha256"] == sha(FOCUSED / "web.sha256") == FOCUSED_WEB,
            "the original scoped executable or Web pin differs")
    assets = invocation["web_files"]
    require(len(assets) == 4 and assets == manifest(FOCUSED / "web.sha256")
            == {file.name: sha(file) for file in (FOCUSED / "web/material-gallery").iterdir()},
            "scoped Gallery bytes differ")
    proof = verify(run / "artifacts", {key: observed[key] for key in ("browser", "adapter")})
    require({name: png["sha256"] for name, png in proof["pngs"].items()} == observed["captures"],
            "the stock-style decoder disagrees with the original scoped decoder")
    gallery = None if canonical is None else {file.name: sha(file) for file in canonical.iterdir() if file.is_file()}
    require(gallery is None or len(gallery) == 4, "wrong canonical Gallery pack inventory")
    return {
        **observed,
        "scope": "scoped M325–M350 development",
        "exact_source_sha256": EXACT_SOURCE,
        "same_m347_to_m350_source_sha256": FEATURE_SOURCE,
        "focused_evidence_manifest_sha256": FOCUSED_SEAL,
        "focused_receipt_sha256": FOCUSED_RECEIPT,
        "injected_chrome_flags": [UPDATER_FLAG],
        "runner_sha256": FOCUSED_RUNNER,
        "web_manifest_sha256": FOCUSED_WEB,
        "gallery_files": assets,
        "canonical_gallery_files": gallery,
        "canonical_gallery_match": None if gallery is None else gallery == assets,
    }
