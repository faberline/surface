"""Independently verify real .69 paired Canvas pixels and the scoped receipt."""

import argparse
from hashlib import file_digest
import json
import math
from pathlib import Path
import re
from runpy import run_path

from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parent
PREFIX = "material-text-field-placeholder"
R2 = Path("/tmp/surface-md3-label-color-focused-v0168.Ebk8jXAK/development-r2")
R2_REPORT = "9a9ba2f1f0a450c39f8d4804bf6ede6bb796253832ab8039b1fe1ace139efa38"
UPDATER = "--disable-updater-scheduler"
HEADER, DPR = 44, 2
SHORT = "名稱"
ABOVE = "輸入團隊成員在共用專案清單中容易辨認的完整名稱"


def require(ok, context):
    if not ok:
        raise ValueError(context)


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def read(path):
    return json.loads(path.read_text())


def remain(k, ms):
    angle = math.sqrt(k) * ms / 1000
    return (1 + angle) * math.exp(-angle)


def resumed(blur_ms, resume_ms):
    w = math.sqrt(800)
    t = resume_ms / 1000
    offset = remain(3800, blur_ms) - 1
    velocity = -3800 * blur_ms / 1000 * math.exp(-math.sqrt(3800) * blur_ms / 1000)
    return 1 + (offset + (velocity + w * offset) * t) * math.exp(-w * t)


def spec():
    return {
        "outlined": [
            ("idle", 0, 0), ("focus-start", 0, 0), ("focus-overlap", 14, 0),
            ("focus-alpha-96", 96, 1 - remain(800, 96)),
            ("focus-alpha-144", 144, 1 - remain(800, 144)),
            ("focus-settled", 800, 1), ("blur-start", 800, 1),
            ("blur-alpha-12", 812, remain(3800, 12)),
            ("blur-alpha-20", 820, remain(3800, 20)),
            ("reverse-start", 820, remain(3800, 20)),
            ("reverse-alpha-6", 826, resumed(20, 6)), ("reduced", 826, 1),
        ],
        "filled-rtl": [
            ("focus-start", 0, 0), ("focus-overlap", 14, 0), ("focus-settled", 800, 1),
            ("marked", 800, 0), ("cancelled-marked", 800, 1), ("empty-composition", 800, 0),
            ("cancelled-empty", 800, 1), ("typed", 800, 0), ("cleared", 800, 1),
        ],
        "above-rtl": [
            ("idle-above", 0, 1), ("focused-above", 0, 1), ("blurred-above", 0, 1),
            ("nonempty", 800, 0), ("clear-start", 800, 0), ("clear-alpha-44", 844, 1 - remain(800, 44)),
            ("clear-alpha-96", 896, 1 - remain(800, 96)), ("clear-settled", 1600, 1),
        ],
    }


def image(path, compact):
    expected = (640, 2008) if compact else (1920, 1728)
    require(path.is_file() and not path.is_symlink(), f"{path.name}: missing original PNG")
    with Image.open(path) as original:
        require(original.format == "PNG" and original.size == expected,
                f"{path.name}: wrong original PNG geometry {original.size}")
        original.verify()
    with Image.open(path) as original:
        return original.convert("RGB")


def dot(bitmap, point):
    x, y = point
    return bitmap.getpixel((math.floor(x * DPR), math.floor((y + HEADER) * DPR)))


def inner(rect):
    x, y, w, h = rect
    return math.ceil(x * DPR), math.ceil((y + HEADER) * DPR), math.floor((x + w) * DPR), math.floor((y + HEADER + h) * DPR)


def inside(rect, rectangle):
    x, y, w, h = rect
    l, t, width, height = rectangle
    return w > 0 and h > 0 and l <= x < x + w <= l + width and t <= y < y + h <= t + height


def colors(under, over, alpha, linear):
    alpha = math.floor(alpha * 255 + .5) / 255

    def light(byte):
        unit = byte / 255
        return unit if not linear else unit / 12.92 if unit <= .04045 else ((unit + .055) / 1.055) ** 2.4

    def encode(n):
        srgb = n if not linear else n * 12.92 if n <= .0031308 else 1.055 * n ** (1 / 2.4) - .055
        return math.floor(srgb * 255 + .5)

    return tuple(encode(light(a) * (1 - alpha) + light(b) * alpha) for a, b in zip(under, over, strict=True))


def changed(left, right):
    hist = ImageChops.difference(left, right).getcolors(left.width * left.height)
    return sum(n for n, rgb in hist if rgb != (0, 0, 0))


def point_in(x, y, rectangle):
    l, t, w, h = rectangle
    return l <= x < l + w and t <= y < t + h


def compare_case(artifacts, name, block, stages):
    compact = name.endswith("-rtl")
    viewport = (0, 0, 320, 960) if compact else (0, 0, 960, 820)
    layout = block["layout"]
    face, crop, label = (layout[key] for key in ("editor", "crop", "label"))
    require(layout["rtl"] is compact and layout["filled"] is (name == "filled-rtl")
            and inside(face, viewport) and inside(crop, face) and inside(label, viewport),
            f"{name}: invalid viewport or direction")
    if name == "above-rtl":
        require(label[3] >= 64 and label[1] + label[3] + 3.5 <= face[1],
                "wrapped label inside its own area above the editor")
    probes = block["probe_points"]
    require(len(probes) == 4 and all(point_in(*point, crop) for point in probes), "wrong solid hint probe mask")
    peer_modes = [True, True]
    expect_names = {phase for phase, _, _ in stages}
    rows = block["frames"]
    require(set(rows) == expect_names, f"{name}: incomplete or extra phase inventory")
    original = {}
    retained = {}
    paper, ink = (tuple(layout[key]) for key in ("paper", "foreground"))
    for phase, ms, alpha in stages:
        row = rows[phase]
        require(row["ms"] == ms and math.isclose(row["opacity"], alpha, abs_tol=1e-11),
                f"{name}/{phase}: wrong independently computed alpha or clock")
        pictures = []
        for kind in ("hint", "control"):
            exact_name = f"{PREFIX}-{name}-{kind}-{phase}.png"
            require(row[f"{kind}_png"] == exact_name, f"{name}/{phase}: original {kind} filename differs")
            path = artifacts / exact_name
            bitmap = image(path, compact)
            pictures.append(bitmap)
            original[exact_name] = {"sha256": sha(path), "size": list(bitmap.size)}
        left, right = pictures
        left_cut, right_cut = (img.crop(inner(crop)) for img in pictures)
        count = changed(left_cut, right_cut)
        actual = [dot(left, point) for point in probes]
        require(count == row["changed_pixels"] and list(map(list, actual)) == row["actual_probe_rgb"],
                f"{name}/{phase}: report differs from original PNG bytes")
        if alpha == 0:
            require(count == 0 and row["expected_composite_rgb"] == [list(paper)] * 2,
                    f"{name}/{phase}: visible ink in exhaustive no-hint crop")
        else:
            expected = [colors(paper, ink, alpha, mode) for mode in (False, True)]
            require(count > 32 and all(dot(right, point) == paper for point in probes)
                    and row["expected_composite_rgb"] == list(map(list, expected)),
                    f"{name}/{phase}: true control/ink/color blend differs")
            for mode, color in enumerate(expected):
                peer_modes[mode] &= all(all(abs(a - b) <= 2 for a, b in zip(rgb, color)) for rgb in actual)
            require(any(peer_modes), f"{name}/{phase}: no one renderer color space fits the actual timed pixels")
            if alpha == 1:
                require(actual == [ink] * 4, f"{name}/{phase}: not settled against the exact token")
                exact_ink = []
                x0, y0, _, _ = inner(crop)
                for index, (pixel, baseline) in enumerate(zip(left_cut.get_flattened_data(),
                                                              right_cut.get_flattened_data(), strict=True)):
                    if pixel == ink and baseline == paper:
                        exact_ink.append(index)
                require(len(exact_ink) > 80, f"{name}/{phase}: no real settled hint mask")
                xs = [(x0 + (index % left_cut.width) + .5) / DPR for index in exact_ink]
                ink_edge = max(xs) if compact else min(xs)
                field_edge = face[0] + (face[2] - 16 if compact else 16)
                require(abs(ink_edge - field_edge) <= 10, f"{name}/{phase}: true ink is not at logical start")
        if phase in ("marked", "empty-composition"): 
            retained[phase] = left_cut
        if phase == "idle-above":
            area = inner(label)
            totals = [sum(sample == ink for sample in pic.crop(area).get_flattened_data()) for pic in pictures]
            require(totals == [block["idle_above_label_pixels"]] * 2 and totals[0] > 40,
                    "above caption and its idle hint must be simultaneously visible")
    require(peer_modes == block["accepted_blend_models"], "recorded blend model choice differs")
    if name == "filled-rtl":
        require(block["marked_ink_pixels"] > 12 and changed(retained["marked"], retained["empty-composition"]) > 12,
                "the PNG must show nonempty actual marked glyph ink")
    return original


def ax_state(item, label, committed, active):
    require(item["name"] == label and item["value"] == committed and item["active"] is active
            and type(item["backend"]) is int and item["backend"] > 0,
            "Chrome AX value, name or active editing host differs")


def verify(artifacts):
    report_path = artifacts / f"{PREFIX}.json"
    require(report_path.is_file(), "missing the actual .69 typed placeholder report")
    report = read(report_path)
    require(report["dpr"] == DPR and report["clock"] == "controlled performance.now"
            and report["stiffness"] == {"slow": 800, "fast": 3800} and report["damping_ratio"] == 1
            and report["page_errors"] == [] and "failure" not in report, "wrong or failed placeholder report")
    require(re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", report["browser"]), "missing real Chrome build")
    adapter = report["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal-")
            and adapter["isFallbackAdapter"] is False, "expected actual local Metal adapter")
    files = {}
    for name, stages in spec().items():
        block = report[name.replace("-", "_")]
        files.update(compare_case(artifacts, name, block, stages))
        for side in ("hint_ax", "control_ax"):
            ax = block[side]
            if name == "outlined":
                ax_state(ax["idle"], SHORT, "", False)
                ax_state(ax["focused"], SHORT, "", True)
                require(ax["idle"]["backend"] != ax["focused"]["backend"], "inactive proxy stayed active")
            elif name == "filled-rtl":
                require(ax["synthetic_empty_begin"] == {"trusted": False, "data": ""}
                        and ax["actual_cdp_marked"] == "中文", "IME diagnostic scope changed")
                ax_state(ax["host_after_cancel"], SHORT, "", True)
            else:
                ax_state(ax["idle"], ABOVE, "", False)
                ax_state(ax["seeded"], ABOVE, "範例", False)
                ax_state(ax["cleared"], ABOVE, "", False)
    require(len(files) == 58 and set(files) == {p.name for p in artifacts.glob(f"{PREFIX}-*.png")},
            "wrong 58-file paired PNG inventory")
    return {"report_sha256": sha(report_path), "browser": report["browser"], "adapter": adapter,
            "pngs": files, "expected_alpha": spec(),
            "negative_masks": [f"{name}/{phase}" for name, stages in spec().items() for phase, _, alpha in stages
                               if alpha == 0]}


def verify_run(out):
    artifacts = out / "artifacts"
    raw = verify(artifacts)
    old = PAST["verify_artifacts"](artifacts)
    require(len(old["captures"]) == 34 and raw["browser"] == old["browser"] and raw["adapter"] == old["adapter"],
            "the retained label and placeholder views used different Chrome or adapters")
    scope = PREP["check_scope"]()
    gallery = PREP["check_gallery"](scope)
    runner = ROOT / "pins/surface-e2e"
    runner_sha = (ROOT / "runner.sha256").read_text().strip()
    require(sha(runner) == runner_sha
            and (ROOT / "pins/focused-source-manifest.sha256").read_text().strip() == scope["focused_source_manifest_sha256"],
            "the focused runner pin differs")
    invocation = read(out / "invocation.json")
    inputs = {**scope, **gallery, "runner_sha256": runner_sha}
    require(inputs == invocation["inputs"] == read(out / "inputs-after.json")
            and invocation["scope"] == "q69 focused M325–M353 development"
            and invocation["suite_arguments"] == ["material"] and invocation["injected_chrome_flags"] == [UPDATER],
            "full source, assets, or actual scope changed")
    command = [str(runner), "--chrome", str(out / "guards/chrome"), "--chrome-arg", UPDATER,
               "--web-root", str(ROOT / "web"), "--artifacts", str(artifacts), "material"]
    require(invocation["argv"] == command, "recorded scoped invocation changed")
    log = out / "runner.log"
    text = log.read_text()
    require(re.findall(r"^case (\S+) \.\.\.$", text, re.M) == ["material"]
            and re.findall(r"^  \[(M\d+)\]", text, re.M) == PREP["STEPS"],
            "scoped run did not exercise all 29 ordered old and new steps")
    count = re.findall(r"^case material \.\.\. ok \(29 steps, (\d+) checks,", text, re.M)
    require(len(count) == 1 and int(count[0]) >= 1798
            and re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", text, re.M) == count,
            "scoped result is incomplete")
    guard_path = out / "headless-window-observation.json"
    guard = read(guard_path)
    event, = guard["guard_events"]
    observed = guard["launch_observations"]["chrome-primary"]
    require([read(path) for path in (out / "guard-ledger").glob("*.json")] == [event],
            "raw scoped guard ledger differs")
    require(guard["stock_exit"] == 0 and not guard["problems"] and not guard["visible_owned_windows"]
            and not guard["remaining_observed_processes"] and observed["pids"] == [event["pid"]]
            and event["allowed"] is True and event["role"] == "chrome-primary" and observed["samples"] > 0
            and event["argv"].count(UPDATER) == 1 and "--" not in event["argv"]
            and OBS["guard_verdict"]("chrome", event["argv"], {}, out / "tmp")
            == ("chrome-primary", True, event["profile"])
            and all(guard["launch_observations"][type_]["samples"] == 0 for type_ in ("chrome-bridge", "native"))
            and read(out / "owned-shutdown.json")["after_cleanup"] == [],
            "observations did not prove a single clean headless Chrome")
    return {
        **raw, "steps": PREP["STEPS"], "checks": int(count[0]), "scope": invocation["scope"],
        "retained_label_motion": old, "inputs": inputs, "headless_samples": observed["samples"],
        "visible_windows": 0, "remaining_owned_processes": [],
        "window_receipt_sha256": sha(guard_path), "runner_log_sha256": sha(log),
    }


def self_test():
    past = PAST["verify_artifacts"](R2 / "artifacts")
    require(past["report_sha256"] == R2_REPORT and len(past["captures"]) == 34, "prior real proof differs")
    try:
        verify(R2 / "artifacts")
    except ValueError as error:
        require(str(error) == "missing the actual .69 typed placeholder report", "wrong expected prior rejection")
    else:
        raise ValueError("actual .68 was incorrectly accepted as real .69 coverage")
    paper = (253, 247, 255)
    ink = (73, 69, 78)
    correct = colors(paper, ink, 1 - remain(800, 96), False)
    difference = [colors(paper, ink, 1 - remain(3800, 96), model) for model in (False, True)]
    require(all(any(abs(a - b) > 2 for a, b in zip(guess, correct)) for guess in difference),
            "96 ms cannot distinguish the wrong curve")
    blank = Image.new("RGB", (3, 3), paper)
    painted = blank.copy()
    painted.putpixel((1, 1), correct)
    require(changed(blank, painted) == 1, "the no-hint comparator missed one translucent pixel")
    return {
        "scope": "offline prep, prior real .68 replay and synthetic negative controls only",
        "prior_real_label_pngs": len(past["captures"]), "expected_new_paired_pngs": 58,
        "reject_old_receipt": True, "uses_same_phase_colour_space": True,
        "slow_sample_discriminates_wrong_fast_curve": True, "single_translucent_pixel_detected": True,
        "closed_scope_steps": PREP["STEPS"],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--run", type=Path, help="verify finished scope and all raw PNGs")
    group.add_argument("--artifacts", type=Path, help="verify new raw typed receipt and PNGs alone")
    group.add_argument("--self-test", action="store_true", help="run strictly offline controls")
    args = parser.parse_args()
    if args.self_test:
        result = self_test()
    elif args.artifacts:
        result = verify(args.artifacts.resolve(strict=True))
    else:
        result = verify_run(args.run.resolve(strict=True))
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))
