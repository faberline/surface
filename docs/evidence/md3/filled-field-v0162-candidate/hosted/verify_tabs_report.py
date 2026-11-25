"""Replay the pinned Tabs journey from its typed report and four PNGs."""

from hashlib import file_digest, sha256
import json
from math import floor, isfinite
import re

from PIL import Image

STEM = "material-tabs"
REPORT = f"{STEM}.json"
IMAGES = {f"{STEM}-{mode}.png": size for mode, size in (
    ("fixed-light", (1920, 1688)), ("fixed-dark", (1920, 1688)),
    ("dark-pressed", (1920, 1688)), ("rtl-320", (640, 1608)),
)}
STEPS = ("M286", "M287", "M288", "M289")
KEYS = ("overview", "lock", "activity", "all", "private", "unread")


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def rect(value, name):
    require(isinstance(value, list) and len(value) == 4
            and all(type(item) in (int, float) and isfinite(item) for item in value)
            and value[2] > 0 and value[3] > 0, f"invalid Tabs rectangle in {name}")
    return value


def near(a, b):
    return abs(a - b) <= 0.75


def middle(rect):
    return rect[0] + rect[2] / 2


def right(rect):
    return rect[0] + rect[2]


def bottom(rect):
    return rect[1] + rect[3]


def contains(rect, child):
    return (rect[0] <= child[0] + 0.75 and rect[1] <= child[1] + 0.75
            and right(rect) + 0.75 >= right(child) and bottom(rect) + 0.75 >= bottom(child))


def sample(image, x, y):
    # The pinned Gallery has a 44 CSS px header and a 2x device viewport.
    px, py = floor(x * 2), floor((y + 44) * 2)
    require(0 <= px < image.width and 0 <= py < image.height, "Tabs sample outside original PNG")
    return image.getpixel((px, py))


def panel_links(links, primary):
    require(isinstance(links, list) and len(links) == len(KEYS), "wrong Tabs relationship count")
    targets = []
    for key, link in zip(KEYS, links, strict=True):
        stable = f"tabs-demo-{key}"
        source = "surface-accessibility-tab-" + stable.encode().hex()
        panel = f"{stable}-panel"
        target = "surface-accessibility-tabpanel-" + panel.encode().hex()
        active = key in (primary, "unread")
        require(link["source"] == link["label"] == source and link["target"] == target
                and link["role"] == "tabpanel" and type(link["matches"]) is int and link["matches"] == 1,
                f"unbound or ambiguous Tabs relationship for {key}")
        require(link["hidden"] is (not active) and link["empty"] is (not active)
                and link["live"] == (panel if active else None), f"bad Tabs mount or placeholder for {key}")
        targets.append(target)
    return targets


def fixed(data, image, mode):
    require(data["capture"] == mode, f"wrong Tabs fixed capture for {mode}")
    top, low = [rect(value, "bar") for value in data["bars"]]
    cells = [rect(value, "cell") for value in data["cells"]]
    primary, secondary = [rect(value, "mark") for value in data["marks"]]
    divider = rect(data["divider"], "divider")
    require(near(top[3], 64) and near(low[3], 48) and len(cells) == 3 and all(
        near(cell[0], top[0] + index * top[2] / 3) and near(cell[1], top[1])
        and near(cell[2], top[2] / 3) and near(cell[3], 64) for index, cell in enumerate(cells)),
        "Tabs fixed row geometry changed")
    require(near(primary[3], 3) and 24 <= primary[2] < cells[0][2]
            and near(secondary[3], 2) and near(secondary[2], low[2] / 3)
            and near(bottom(top), bottom(primary)) and near(bottom(low), bottom(secondary))
            and near(divider[3], 1) and near(divider[2], top[2]) and near(bottom(divider), bottom(top)),
            "Tabs indicator or divider geometry changed")
    base = sample(image, middle(cells[1]), cells[1][1] + 4)
    ink = sample(image, middle(primary), primary[1] + 1)
    edge = sample(image, middle(cells[1]), divider[1] + 0.5)
    require(ink == sample(image, middle(secondary), secondary[1] + 1)
            and len({base, ink, edge}) == 3
            and sample(image, primary[0] + 0.25, primary[1] + 0.25) != ink,
            f"Tabs pixels differ from the fixed marks, rounded corner or divider in {mode}")
    return {"base": base, "primary": ink, "divider": edge}


def automatic(data, fixed_dark, image, palette):
    before, after, snapped = [rect(data[key], key) for key in ("before", "after", "snapped")]
    samples = [rect(value, "samples") for value in data["samples"]]
    require(len(samples) == 50, "wrong number of Tabs motion samples")
    start, end = middle(samples[0]), middle(samples[-1])
    require(end > start + 24 and middle(after) > middle(before) + 24
            and any(start + 8 < middle(item) < end - 8 for item in samples)
            and all(near(bottom(item), bottom(samples[0])) and near(item[3], 3) for item in samples)
            and all(near(item[3], 3) for item in (before, after, snapped))
            and near(middle(snapped), middle(before)), "Tabs motion or Reduce Motion snap changed")
    cell = fixed_dark["cells"][0]
    actual = sample(image, cell[0] + 12, cell[1] + 8)
    expected = tuple((base * 229 + ink * 26) // 255 for base, ink in zip(palette["base"], palette["primary"]))
    require(isinstance(data["pressed_ink"], list) and len(data["pressed_ink"]) == 3
            and all(type(value) is int and 0 <= value <= 255 for value in data["pressed_ink"])
            and list(actual) == data["pressed_ink"]
            and all(abs(actual_channel - expected_channel) <= 2
                    for actual_channel, expected_channel in zip(actual, expected)), "Tabs held press pixels changed")
    return {"samples": len(samples), "travel": end - start, "snap": list(snapped), "pressed_ink": list(actual)}


def scrolling(data, image, dark_primary):
    begin, end, rtl, dragged, revealed, label, mark = [rect(data[key], key) for key in (
        "start", "wheel_end", "rtl_start", "dragged", "revealed", "label", "mark")]
    require(end[0] < begin[0] - 48 and dragged[0] > rtl[0] + 30
            and all(near(item[2], begin[2]) for item in (end, rtl, dragged))
            and all(near(item[1], begin[1]) for item in (end, rtl, dragged, revealed))
            and revealed[0] >= 0 and right(revealed) <= 320 and revealed[2] >= 90 and revealed[3] >= 48
            and label[3] >= 40 and contains(revealed, label) and contains(revealed, mark)
            and near(mark[3], 3) and near(bottom(mark), bottom(revealed))
            and near(mark[2], label[2]), "Tabs RTL scroll or scaled focus reveal changed")
    require(sample(image, middle(mark), mark[1] + 1) == dark_primary, "Tabs RTL screenshot indicator ink changed")
    return {"wheel_delta": end[0] - begin[0], "touch_delta": dragged[0] - rtl[0],
            "revealed": revealed, "label": label, "mark": mark}


def validate(root, log, metadata):
    source = root / REPORT
    report = json.loads(source.read_text())
    require(set(report) == {"browser", "adapter", "fixed", "automatic", "scroll", "page_errors"}
            and report["browser"] == metadata["browser"] and report["adapter"] == metadata["adapter"]
            and type(report["adapter"]["isFallbackAdapter"]) is bool and report["page_errors"] == [],
            "Tabs failure, page errors or independent Chrome adapter mismatch")
    ids = re.findall(r"^\s*\[(M(?:286|287|288|289))\]", log.read_text(), re.M)
    require(tuple(ids) == STEPS, "missing, duplicate or out-of-order Tabs step")
    require({path.name for path in root.glob(f"{STEM}-*.png")} == set(IMAGES), "wrong Tabs PNG set")
    images, records = {}, {}
    for name, size in IMAGES.items():
        with Image.open(root / name) as png:
            png.verify()
        with Image.open(root / name) as png:
            require(png.format == "PNG" and png.size == size, f"wrong Tabs PNG dimensions for {name}")
            image = png.convert("RGB")
        images[name] = image
        records[name] = {"size": list(size), "sha256": digest(root / name),
                         "decoded_rgb_sha256": sha256(image.tobytes()).hexdigest()}
    require(len({item["decoded_rgb_sha256"] for item in records.values()}) == len(IMAGES), "duplicate Tabs PNG pixels")
    fixed_report = report["fixed"]
    before, after = panel_links(fixed_report["before"], "overview"), panel_links(fixed_report["after"], "activity")
    require(before == after, "Tabs panel IDs changed across a mount and unmount")
    light = fixed(fixed_report["light"], images[f"{STEM}-fixed-light.png"], "fixed-light")
    dark = fixed(fixed_report["dark"], images[f"{STEM}-fixed-dark.png"], "fixed-dark")
    require(sum(light["base"]) > sum(dark["base"]) + 300 and light["primary"] != dark["primary"],
            "Tabs light and dark palettes did not change")
    motion = automatic(report["automatic"], fixed_report["dark"], images[f"{STEM}-dark-pressed.png"], dark)
    rtl = scrolling(report["scroll"], images[f"{STEM}-rtl-320.png"], dark["primary"])
    return {"steps": list(STEPS), "report_sha256": digest(source), "pngs": records,
            "reciprocal_panel_ids": before, "motion": motion, "scroll": rtl, "page_errors": 0}
