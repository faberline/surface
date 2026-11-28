"""Replay both q74 Navigation scroll proofs without a running browser."""

from math import isfinite
from pathlib import Path
import re
from runpy import run_path

from evidence_common import clean_windows, data, digest, normalized, path_entries, require, seal, verify_entries, verify_ledgers
ROOT = Path(__file__).resolve().parent
REGISTRY = "e2e/src/material.rs"
STEM = "material-adaptive-navigation"
PLACEMENT = {
    "compact": ((599, 620), [0, 556, 599, 64], [242, 236, 244], [232, 222, 248]),
    "medium": ((600, 620), [0, 556, 600, 64], [242, 236, 244], [232, 222, 248]),
    "rail": ((840, 620), [0, 0, 96, 620], [253, 247, 255], [232, 222, 248]),
    "dark-rtl-short-rail": ((840, 300), [744, 0, 96, 300], [20, 18, 24], [74, 68, 88]),
}
DRAFT = {"value": "draft", "runtime": [5, 3], "browser": ["draft", 3, 5, "backward"]}
SAVED, UPDATES, SETTINGS = ["adaptive-navigation-demo-" + suffix for suffix in ("saved", "updates", "settings")]


def tools():
    return run_path(str(ROOT / "verify-adaptive-focused-original.py"))


def rect(r):
    require(isinstance(r, list) and len(r) == 4 and all(type(x) in (float, int) and isfinite(x) for x in r)
            and min(r[2:]) > 0, "Invalid measured rectangle")
    return r


def inside(r, viewport):
    x, y, w, h = rect(r)
    left, top, span, height = rect(viewport)
    return left <= x and top <= y and x + w <= left + span and y + h <= top + height


def verify(artifacts, metadata):
    report = artifacts / f"{STEM}.json"
    proof = data(report)
    bars, rail, dark = proof["bars"], proof["rail"], proof["dark"]
    require(proof["draft"] == DRAFT and bars["current"] == bars["focus"] == SAVED
            and rail["current"] == UPDATES and rail["focus"] == dark["focus"] == SETTINGS,
            "Current page, retained focus or backward selection differs")
    require(rail["page"] == [96, 0, 744, 620] and dark["page"] == [0, 0, 744, 300]
            and inside(dark["editor"], dark["page"]), "The separate content viewport or editor differs")
    before, revealed = rect(dark["before"]), rect(dark["revealed"])
    offset = dark["rail_scroll"]
    require(dark["clip"] == dark["paint"]["bounds"] and offset == [0,100] and dark["page_scroll"] == 0
            and not inside(before, dark["clip"]) and inside(revealed, dark["clip"])
            and before[:1] + before[2:] == revealed[:1] + revealed[2:]
            and before[1] - revealed[1] == offset[1] - offset[0],
            "The focused row did not move into the rail clip by its own real scroll offset")
    records = {"compact": bars["compact"], "medium": bars["medium"], "rail": rail["paint"],
               "dark-rtl-short-rail": dark["paint"]}
    filenames = {f"{STEM}-{name}.png" for name in records}
    require({path.name for path in artifacts.glob(f"{STEM}-*.png")} == filenames, "Wrong adaptive Navigation image inventory")
    screenshots = {}
    checker = tools()["verify_frame"]
    for tag, record in records.items():
        path = artifacts / f"{STEM}-{tag}.png"
        viewport, nav, surface, selected = PLACEMENT[tag]
        cell = rect(record["current_bounds"])
        require(record["bounds"] == nav and record["surface"] == surface and record["selected"] == selected
                and inside(cell, nav) and min(cell[2:]) >= 48, "Wrong adaptive Navigation token or selected-cell geometry")
        if tag == "medium":
            require(cell[2:] == [112,64] and cell[0] + cell[2] / 2 == 300, "Wrong centred Medium cell")
        elif "rail" in tag:
            require(cell[0] == nav[0] and cell[2] == 96, "Wrong logical-start rail cell")
        screenshots[path.name] = checker(path, record, viewport)
    return normalized({"report_sha256": digest(report), "pngs": screenshots,
                       "draft": proof["draft"], "selected": [bars["current"], rail["current"]],
                       "rail_scroll": offset, "page_scroll": dark["page_scroll"], "scroll_witness": dark,
                       **metadata})

