"""Recheck original .70 Canvas PNGs, inherited pixel suites and scoped guard receipts."""

import argparse
from dataclasses import dataclass
from hashlib import file_digest
import json
from math import cos, exp, floor, isclose, sin, sqrt
from pathlib import Path
import re
from runpy import run_path

ROOT = Path(__file__).resolve().parent
PRIOR = Path("/tmp/surface-md3-field-placeholder-focused-v0169-r2.8AArzw7l")
PREFIX = "material-text-field-boundary-motion"
DPR = 2
SCOPE = "q70 focused M325–M356 development"
UPDATER = "--disable-updater-scheduler"
OFFSETS = (-.375, -.125, .125, .375)
TITLE = "專案名稱"
HELP = "請輸入中文或英文名稱。"
ERROR_HELP = "請輸入有效的專案名稱。"
LONG = "輸入團隊成員在共用專案清單中容易辨認的完整名稱"


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def read(path):
    return json.loads(path.read_text())


for name, digest in {
    "verify-label-color-original.py": "62c1a0238a740ccac4a24677b48f4842cfa4de4639fd62353bfdc4dfdebcf8e1",
    "pixel-oracle.py": "50e0b193f7b10158725ac2021bd1564ff3b62c648d559a1368aac5c3ea8b1949",
    "verify-placeholder.py": "6217077832e5ead6d1e314641348f2d8b1c138a59853a5f2cd6fe402f84c8b74",
}.items():
    require(sha(PRIOR / name) == digest, f"the sealed inherited verifier changed: {name}")
require(sha(ROOT / "observe-tabs.py") == "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7",
        "the sealed original guard differs")
OBS = run_path(str(ROOT / "observe-tabs.py"))
PREP = run_path(str(ROOT / "prepare-focused.py"))
LABEL = run_path(str(ROOT / "verify-inherited-label.py"))
PLACEHOLDER = run_path(str(PRIOR / "verify-placeholder.py"))
ORACLE = run_path(str(PRIOR / "pixel-oracle.py"))
rgb, image_for = LABEL["color"], LABEL["image_for"]
blend = LABEL["blend"]


@dataclass(frozen=True)
class Phase:
    tag: str
    ms: int
    width: float
    ink: tuple
    alpha: int = 255
    layer: int = 0


def motion(form, start, target, elapsed, velocity=0):
    """Analytic position and velocity. Keep an unrounded pose for retarget."""
    mass_rate, damping = (sqrt(3800), 1) if form == "outlined" else (sqrt(1400), .9)
    t = elapsed / 1000
    atten = mass_rate * damping
    d = start - target
    fade = exp(-atten * t)
    momentum = velocity + atten * d
    if damping == 1:
        displacement = d + momentum * t
        derivative = momentum
    else:
        sine_rate = sqrt(mass_rate ** 2 - atten ** 2)
        angle = t * sine_rate
        displacement = cos(angle) * d + sin(angle) * momentum / sine_rate
        derivative = cos(angle) * momentum - sin(angle) * d * sine_rate
    return target + fade * displacement, fade * (derivative - atten * displacement)


def opening(form, palette):
    idle = rgb(palette, "outline" if form == "outlined" else "on_surface_variant")
    primary = rgb(palette, "primary")
    return [
        Phase("wide-idle", 1600, 1, idle),
        Phase("focus-start", 1600, 1, idle),
        Phase("focus-14", 1614, motion(form, 1, 3, 14)[0], blend(idle, primary, 14)),
        Phase("focus-44", 1644, motion(form, 1, 3, 44)[0], blend(idle, primary, 44)),
        Phase("focus-end", 2400, 3, primary),
    ]


def palettes(form, palette):
    idle = rgb(palette, "outline" if form == "outlined" else "on_surface_variant")
    hover, primary, error, invalid_hover = (
        rgb(palette, role) for role in ("on_surface", "primary", "error", "on_error_container")
    )
    surface_hover = 20 if form == "filled" else 0
    surface_disabled, disabled_ink = (10, 97) if form == "filled" else (0, 31)
    return [
        Phase("hover-44", 44, 1, blend(idle, hover, 44), layer=surface_hover),
        Phase("hover-end", 800, 1, hover, layer=surface_hover),
        Phase("focus-over-hover", 1600, 3, primary),
        Phase("invalid-44", 1644, motion(form, 3, 1, 44)[0], blend(primary, error, 44)),
        Phase("invalid-idle", 2400, 1, error),
        Phase("invalid-hover", 3200, 1, invalid_hover, layer=surface_hover),
        Phase("invalid-focus-over-hover", 4000, 3, error),
        Phase("disabled", 4000, 1, hover, disabled_ink, surface_disabled),
    ]


def compact(palette):
    idle = rgb(palette, "outline")
    primary = rgb(palette, "primary")
    falling, velocity = motion("outlined", 3, 1, 44)
    falling_color = ORACLE["sample"](ORACLE["pose"](primary), idle, 44)
    intermediate = ORACLE["from_lab"](falling_color[0])
    reversed_color = ORACLE["from_lab"](ORACLE["sample"](falling_color, primary, 20)[0])
    return [
        Phase("rtl-focus", 800, 3, primary),
        Phase("rtl-falling", 844, falling, intermediate),
        Phase("rtl-retarget", 844, falling, intermediate),
        Phase("rtl-retarget-20", 864, motion("outlined", falling, 3, 20, velocity)[0], reversed_color),
        Phase("rtl-reduced-focus", 864, 3, primary),
        Phase("rtl-reduced-blur", 864, 1, idle),
        Phase("rtl-above-label-44", 908, motion("outlined", 1, 3, 44)[0], blend(idle, primary, 44)),
    ]


class Render:
    def __init__(self):
        self.modes = {False, True}
        self.pixels = 0

    def expect(self, actual, base, ink, alpha, message):
        self.pixels += 1
        if alpha == 0:
            require(actual == base, f"{message}: expected unpainted {base}, got {actual}")
            return
        if alpha == 1:
            require(actual == ink, f"{message}: expected full ink {ink}, got {actual}")
            return
        self.modes = {
            linear for linear in self.modes
            if all(abs(a - b) <= 2 for a, b in zip(actual, PLACEHOLDER["colors"](base, ink, alpha, linear), strict=True))
        }
        require(self.modes, f"{message}: no common composite convention yields {actual} from {base}, {ink}, {alpha}")


def pixel(image, canvas, point):
    x, y = (floor((origin + value) * DPR) for origin, value in zip(canvas[:2], point, strict=True))
    require(0 <= x < image.width and 0 <= y < image.height, f"probe outside the original: {point}")
    return image.getpixel((x, y))


def inside_width(depth, width, start=0):
    # A path fragment projects four hardware sample locations onto the normal.
    return sum(start <= depth + offset / DPR < width for offset in OFFSETS)


def verify_strip(image, canvas, report, segment, phase, base, paper, render, start=0):
    axis, edge, along, inside = segment
    require(report["edge_dp"] == edge and report["normal_axis"] == axis and report["inward"] == inside
            and report["ink_starts_at_dp"] == start, f"{phase.tag}: wrong stripe geometry")
    anchor = floor((canvas[axis] + edge) * DPR)
    expected = []
    for relative, item in zip(range(-2, 13), report["samples"], strict=True):
        index = anchor + relative if inside > 0 else anchor - 1 - relative
        absolute = (index + .5) / DPR - canvas[axis]
        point = [along, absolute] if axis == 1 else [absolute, along]
        depth = inside * (absolute - edge)
        coverage = inside_width(depth, phase.width, start)
        actual = pixel(image, canvas, point)
        require(item == {"point": point, "depth_dp": depth, "coverage_quarters": coverage, "rgb": list(actual)},
                f"{phase.tag}: typed sample disagrees with independent PNG geometry at {point}")
        render.expect(actual, paper if depth < 0 else base, phase.ink, coverage * phase.alpha / (4 * 255),
                      f"{phase.tag} {point} ({depth} dp)")
        expected.append(coverage)
    return expected


def verify_frame(artifacts, frame, form, phase, palette, render, narrow=False):
    name = f"{PREFIX}-{form}-{phase.tag}.png"
    require(frame["png"] == name and frame["clock_ms"] == phase.ms
            and isclose(frame["width_dp"], phase.width, abs_tol=1e-8, rel_tol=0)
            and frame["ink"] == list(phase.ink) and frame["ink_alpha"] == phase.alpha
            and frame["layer_alpha"] == phase.layer, f"{name}: independent phase pose differs")
    image = image_for(artifacts / name, narrow)
    canvas = [0, 44, 320, 960] if narrow else [0, 44, 960, 820]
    require(frame["canvas"] == canvas, f"{name}: canvas translation differs")
    x, y, w, h = frame["face"]
    clip_x, clip_y, clip_w, clip_h = frame["clip"]
    require(w >= 112 and h >= 48 and 0 <= x - 1 < x + w + 1 <= canvas[2]
            and 0 <= y - 1 < y + h + 1 <= canvas[3]
            and clip_x <= x <= x + w <= clip_x + clip_w
            and clip_y <= y <= y + h <= clip_y + clip_h, f"{name}: face geometry or clipping differs")
    paper = rgb(palette, "background")
    unlayered = paper if form == "outlined" or phase.layer == 10 else rgb(palette, "surface_container_highest")
    base = pixel(image, canvas, [x + w * .75, y + h - 8])
    render.expect(base, unlayered, rgb(palette, "on_surface"), phase.layer / 255, f"{name}: intact face")
    require(frame["base"] == list(base) and frame["paper"] == list(paper), f"{name}: invalid base samples")
    bottoms = [verify_strip(image, canvas, strip, (1, y + h, x + w * fraction, -1), phase, base, paper, render)
               for strip, fraction in zip(frame["bottom"], (.25, .5, .75), strict=True)]
    require(bottoms[0] == bottoms[1] == bottoms[2], f"{name}: three independent bottom stripe masks disagree")
    if form == "outlined":
        sides = [(0, x, y + h / 2, 1), (0, x + w, y + h / 2, -1), (1, y, x + w * (.25 if narrow else .75), 1)]
        for stripe, segment in zip(frame["other"], sides, strict=True):
            verify_strip(image, canvas, stripe, segment, phase, base, paper, render)
    else:
        points = ([x + 4, y + h / 2], [x + w - 4, y + h / 2], [x + w * .75, y + .25])
        for sample, point in zip(frame["other"], points, strict=True):
            require(sample == {"point": point, "rgb": list(base)} and pixel(image, canvas, point) == base,
                    f"{name}: Filled has false stroke outside the bottom")
    return image, {"sha256": sha(artifacts / name), "size": list(image.size), "clock_ms": phase.ms,
                   "width_dp": phase.width, "coverage_quarters_bottom": sum(bottoms[0]), "ink_rgb": list(phase.ink)}


def field_ax(item, label, help_text, value, focused, invalid):
    require(item["label"] == label and (item["value"] == value or (value == "" and item["value"] is None))
            and (help_text is None or item["description"] == help_text)
            and item["focused"] is focused and item["invalid"] is invalid
            and item["named_textboxes"] == 1 and type(item["backend"]) is int
            and item["backend"] > 0, "inconsistent Chrome textbox role, name, description, host or state")


def verify(artifacts):
    report_path = artifacts / f"{PREFIX}.json"
    require(report_path.is_file() and not report_path.is_symlink(), "missing the actual .70 typed boundary report")
    report = read(report_path)
    require(report["dpr"] == DPR and report["clock"] == "held performance.now"
            and report["page_errors"] == [] and "failure" not in report, "wrong clock or failed page")
    require(re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", report["browser"]), "missing actual Chrome version")
    adapter = report["adapter"]
    require(bool(adapter["vendor"]) and bool(adapter["architecture"])
            and type(adapter["isFallbackAdapter"]) is bool, "missing actual WebGPU adapter")
    render = Render()
    manifest = {}
    for form in ("outlined", "filled"):
        active = report["opening"][form]
        states = report["palette_states"][form]
        palette = active["palette"]
        for name, group, plans in (("opening", active, opening(form, palette)), ("palette", states, palettes(form, palette))):
            for frame, phase in zip(group["frames"], plans, strict=True):
                if phase.tag != "disabled":
                    require(frame["face"] == active["frames"][0]["face"]
                            and frame["clip"] == active["frames"][0]["clip"],
                            f"{form}/{name}: face or clip moved")
                else:
                    require(frame["face"][1] > active["frames"][0]["face"][1], "disabled probe is not the separate peer")
                _, proof = verify_frame(artifacts, frame, form, phase, palette, render)
                manifest[frame["png"]] = proof
        for ax, focused in ((active["rest_ax"], False), (active["focused_ax"], True)):
            field_ax(ax, TITLE, HELP, "草稿", focused, False)
        require(active["rest_ax"]["backend"] != active["focused_ax"]["backend"], "opening never changed host")
        for ax, focused in zip(states["invalid_ax"], (False, True), strict=True):
            field_ax(ax, TITLE, ERROR_HELP, "", focused, True)
        disabled = states["disabled_ax"]
        require(disabled["name"]["value"] == "停用名稱" and disabled["role"]["value"] == "textbox"
                and any(p["name"] == "disabled" and p["value"]["value"] is True for p in disabled["properties"]),
                f"{form}: disabled Chrome AX role differs")

    narrow = report["compact"]
    palette = narrow["palette"]
    expected = compact(palette)
    paper = rgb(palette, "background")
    for frame, phase in zip(narrow["frames"], expected[:-1], strict=True):
        require(frame["face"] == narrow["frames"][0]["face"]
                and frame["clip"] == narrow["frames"][0]["clip"], "compact retarget moved the hit box")
        image, proof = verify_frame(artifacts, frame, "outlined", phase, palette, render, True)
        manifest[frame["png"]] = proof
        if phase.tag == "rtl-focus":
            x, y, w, _ = frame["face"]
            left, span = narrow["floating"]
            require(span > 0 and x + w - left - span == 16, "RTL label lost its physical right inset")
            verify_strip(image, frame["canvas"], narrow["right_notch"], (1, y, left + span + 3, 1),
                         phase, paper, paper, render, phase.width)
    require(narrow["frames"][1]["bottom"] == narrow["frames"][2]["bottom"],
            "same-clock reversal differs in measured color or actual full stripes")
    fallback = narrow["fallback"]
    frame = fallback["frame"]
    image, proof = verify_frame(artifacts, frame, "outlined", expected[-1], palette, render, True)
    manifest[frame["png"]] = proof
    x, y, w, _ = frame["face"]
    above_x, above_y, above_w, above_h = fallback["above"]
    require(above_h >= 64 and 0 <= above_x < above_x + above_w <= 320 and 0 < above_y + above_h <= y,
            "compact wrapped label is not above the input")
    for stripe, fraction in zip(fallback["closed_top"], (.25, .5, .75), strict=True):
        verify_strip(image, frame["canvas"], stripe, (1, y, x + w * fraction, 1), expected[-1], paper, paper, render)
    field_ax(fallback["ax"], LONG, None, "共用名稱", True, False)

    require(len(manifest) == 33 and manifest.keys() == {path.name for path in artifacts.glob(f"{PREFIX}-*.png")},
            "wrong 33-file boundary PNG inventory")
    checks = {
        form: [manifest[f"{PREFIX}-{form}-focus-{phase}.png"]["coverage_quarters_bottom"] for phase in ("14", "44")]
        for form in ("filled", "outlined")
    }
    require(checks == {"filled": [10, 16], "outlined": [11, 20]}, "mid-frame stroke widths do not discriminate both springs")
    blend_modes = {"encoded": False in render.modes, "linear_light": True in render.modes}
    require(report["blend_modes"] == blend_modes, "the reported alpha compose convention disagrees with the raw PNGs")
    return {"report_sha256": sha(report_path), "browser": report["browser"], "adapter": adapter, "pngs": manifest,
            "coverage_quarters_at_14_and_44_ms": checks, "blend_modes": blend_modes, "decoded_probes": render.pixels}


def verify_run(out):
    artifacts = out / "artifacts"
    data = verify(artifacts)
    labels = LABEL["verify_artifacts"](artifacts)
    hints = PLACEHOLDER["verify"](artifacts)
    require(len(labels["captures"]) == 34 and len(hints["pngs"]) == 58
            and labels["browser"] == hints["browser"] == data["browser"]
            and labels["adapter"] == hints["adapter"] == data["adapter"],
            "prior captures or actual browser/adapter changed")
    run = run_path(str(ROOT / "run-focused.py"))
    sealed = run["inputs"]()
    invocation = read(out / "invocation.json")
    command = [str(ROOT / "pins/surface-e2e"), "--chrome", str(out / "guards/chrome"), "--chrome-arg", UPDATER,
               "--web-root", str(ROOT / "web"), "--artifacts", str(artifacts), "material"]
    require(invocation["inputs"] == sealed == read(out / "inputs-after.json") and invocation["argv"] == command
            and invocation["scope"] == SCOPE and invocation["suite_arguments"] == ["material"]
            and invocation["injected_chrome_flags"] == [UPDATER], "wrong actual focused invocation or product pin")
    logpath = out / "runner.log"
    log = logpath.read_text()
    counts = re.findall(r"^case material \.\.\. ok \(32 steps, (\d+) checks,", log, re.M)
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"] and
            re.findall(r"^  \[(M\d+)\]", log, re.M) == PREP["STEPS"] and
            len(counts) == 1 and int(counts[0]) > 2373 and
            re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", log, re.M) == counts,
            "ordered M325–M356 focused scope did not complete")
    guardpath = out / "headless-window-observation.json"
    guard = read(guardpath)
    event, = guard["guard_events"]
    primary = guard["launch_observations"]["chrome-primary"]
    require([read(path) for path in (out / "guard-ledger").glob("*.json")] == [event]
            and guard["stock_exit"] == 0 and not guard["problems"] and not guard["visible_owned_windows"]
            and not guard["remaining_observed_processes"] and primary["pids"] == [event["pid"]]
            and event["allowed"] is True and event["role"] == "chrome-primary" and primary["samples"] > 0
            and event["argv"].count(UPDATER) == 1 and "--" not in event["argv"]
            and OBS["guard_verdict"]("chrome", event["argv"], {}, out / "tmp") == ("chrome-primary", True, event["profile"])
            and all(guard["launch_observations"][kind]["samples"] == 0 for kind in ("chrome-bridge", "native"))
            and read(out / "owned-shutdown.json")["after_cleanup"] == [],
            "not a clean guarded headless run")
    return {**data, "inherited_label_motion": labels, "inherited_placeholder": hints, "steps": PREP["STEPS"],
            "checks": int(counts[0]), "raw_report_and_original_png_count": len(data["pngs"]),
            "retained_png_count": len(labels["captures"]) + len(hints["pngs"]), "visible_windows": 0,
            "remaining_owned_processes": [], "scope": SCOPE, "inputs": sealed, "window_receipt_sha256": sha(guardpath),
            "runner_log_sha256": sha(logpath), "headless_samples": primary["samples"]}


def selftest():
    old = PRIOR / "development-r2-authorized/artifacts"
    current_labels = run_path(str(PRIOR / "verify-label-color-original.py"))["verify_artifacts"](old)
    hints = PLACEHOLDER["verify"](old)
    require(len(current_labels["captures"]) == 34 and len(hints["pngs"]) == 58,
            "the sealed .69 originals no longer replay")
    try:
        verify(old)
    except ValueError as error:
        require(str(error) == "missing the actual .70 typed boundary report", "wrong legacy evidence rejection")
    else:
        raise ValueError("a legacy fixture was admitted as .70")
    try:
        LABEL["verify_artifacts"](old)
    except ValueError as error:
        require("timed outline ink" in str(error), "wrong inherited label migration negative")
    else:
        raise ValueError("a legacy immediate label boundary was admitted as migrated")
    label_report = read(old / "material-text-field-label-motion.json")["outlined"]
    frame = label_report["frames"][1]
    x, y, w, h = label_report["face"]
    image = image_for(old / frame["png"], False)
    sampled = pixel(image, [0, 44, 960, 820], [x + w / 2, y + h - 2.25])
    paper = rgb(label_report["palette"], "background")
    primary = rgb(label_report["palette"], "primary")
    require(sampled == primary != paper and inside_width(2.25, 1) == 0,
            "real former immediate 3 dp frame did not discriminate the new neutral 1 dp startup")
    # At 44 ms the exact same depth is full in Outlined and clean in Filled.
    require(inside_width(2.25, motion("outlined", 1, 3, 44)[0]) == 4
            and inside_width(2.25, motion("filled", 1, 3, 44)[0]) == 0, "wrong curve indistinguishable")
    renderer = Render()
    tinted = PLACEHOLDER["colors"](paper, primary, .65, True)
    try:
        renderer.expect(tinted, paper, primary, 1, "synthetic faded outer pixel")
    except ValueError as error:
        require("expected full ink" in str(error), "wrong synthetic alpha rejection")
    else:
        raise ValueError("a color fade was accepted as fully covered boundary ink")
    return {"scope": "offline only, prior actual .69 originals and synthetic negative guards",
            "old_label_pngs": 34, "old_placeholder_pngs": 58, "expected_new_inventory": 33,
            "reject_missing_typed_boundary": True, "reject_old_immediate_outline_ink": True,
            "real_old_3dp_start_has_ink_at_new_background_depth": True, "selective_44ms_depth_discriminates_width": True,
            "reject_synthetic_foreground_alpha_as_covered_ink": True, "steps": PREP["STEPS"]}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--run", type=Path)
    action.add_argument("--artifacts", type=Path)
    action.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    result = selftest() if args.self_test else verify(args.artifacts) if args.artifacts else verify_run(args.run.resolve(strict=True))
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))
