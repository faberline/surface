#!/usr/bin/env python3
"""Source-bound headless native proof for the q79 Divider Gallery fixture."""
import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import capture, find, focus, key, layout, node, settle
from window_check import onscreen_windows

REPORT = "native-divider-report.json"
SCHEMA = "surface.test.divider-composition.native.v1"
TRANSPORTS = ["Page.screenshot", "CLI", "MCP"]
SHA = re.compile(r"[0-9a-f]{64}")
ROOT = Path(__file__).resolve().parent


def pinned_entries(root, manifest, expected):
    require(SHA.fullmatch(expected or "") and manifest.is_file()
            and not manifest.is_symlink() and digest(manifest) == expected,
            "Reviewed manifest hash differs")
    entries = {}
    for line in manifest.read_text().splitlines():
        value, name = line.split("  ", 1)
        relative = Path(name)
        target = root / relative
        require(SHA.fullmatch(value) and not relative.is_absolute()
                and ".." not in relative.parts and name not in entries
                and target.is_file() and not target.is_symlink()
                and target.resolve().is_relative_to(root) and digest(target) == value,
                "Invalid manifest entry: " + name)
        entries[name] = value
    require(entries, "Manifest is empty")
    return entries


def load_config(path, expected_sha, source_only=False):
    require(SHA.fullmatch(expected_sha or "") and digest(path) == expected_sha,
            "Reviewed config hash differs")
    config = json.loads(path.read_text())
    fields = {"source", "source_manifest", "source_sha256", "source_commit",
              "source_tree", "source_files", "binary_manifest", "binary_sha256",
              "ids", "profiles"}
    require(set(config) == fields, "Config fields differ")
    require(SHA.fullmatch(config["source_sha256"])
            and re.fullmatch(r"[0-9a-f]{40}", config["source_commit"])
            and re.fullmatch(r"[0-9a-f]{40}", config["source_tree"])
            and type(config["source_files"]) is int and config["source_files"] > 0,
            "Source pins are incomplete")
    source = Path(config["source"]).resolve(strict=True)
    source_manifest = Path(config["source_manifest"]).resolve(strict=True)
    source_files = pinned_entries(source, source_manifest, config["source_sha256"])
    actual = {p.relative_to(source).as_posix() for p in source.rglob("*")
              if p.is_file() and p.relative_to(source).parts[0] != ".git"}
    require(set(source_files) == actual and len(source_files) == config["source_files"],
            "Source inventory differs from the reviewed manifest")
    head = subprocess.run(["git", "-C", str(source), "rev-parse", "HEAD"],
                          check=True, text=True, capture_output=True).stdout.strip()
    tree = subprocess.run(["git", "-C", str(source), "rev-parse", "HEAD^{tree}"],
                          check=True, text=True, capture_output=True).stdout.strip()
    status = subprocess.run(["git", "-C", str(source), "status", "--porcelain"],
                            check=True, text=True, capture_output=True).stdout
    require(head == config["source_commit"] and tree == config["source_tree"]
            and not status, "Source commit, Git tree, or clean worktree differs")
    if source_only:
        return config, source_files, None, None, None
    require(SHA.fullmatch(config["binary_sha256"]),
            "Reviewed native binary manifest hash is required for this mode")
    binary_manifest = Path(config["binary_manifest"]).resolve(strict=True)
    binaries = pinned_entries(binary_manifest.parent, binary_manifest, config["binary_sha256"])
    require(set(binaries) == {"surface-material-gallery", "surface-devtools"},
            "Expected only the Gallery and DevTools binaries")
    gallery = binary_manifest.parent / "surface-material-gallery"
    cli = binary_manifest.parent / "surface-devtools"
    require(os.access(gallery, os.X_OK) and os.access(cli, os.X_OK),
            "A reviewed executable is not executable")
    ids = config["ids"]
    required_ids = {
        "section", "show", "back", "theme", "rtl", "large_text",
        "related_list", "related_a", "related_b", "related_rule",
        "full", "heavy_group", "heavy", "heavy_before", "heavy_after",
        "sheet_open", "sheet", "sheet_list", "sheet_a", "sheet_b", "sheet_rule",
        "activation_status",
    }
    require(set(ids) == required_ids and all(isinstance(x, str) and x for x in ids.values()),
            "Gallery ID map differs")
    profiles = config["profiles"]
    require(set(profiles) == {"wide-light", "compact-dark-rtl-200"},
            "Expected wide and compact profiles")
    require(profiles["wide-light"] == {"width": 960, "height": 960, "settings": []},
            "Wide profile must be 960x960 light LTR")
    require(profiles["compact-dark-rtl-200"] == {
        "width": 320, "height": 1360, "settings": ["theme", "rtl", "large_text"]},
        "Compact profile must be 320x1360 dark RTL at 200 percent")
    return config, source_files, binaries, gallery, cli


def at(frame, stable_id):
    return find(frame, stable_id)[1]


def measured(frame, stable_id):
    index, item = find(frame, stable_id)
    require(item is not None and frame.get("layout") is not None, "Missing layout node " + stable_id)
    return frame["layout"]["nodes"][index]


def box(frame, stable_id):
    return measured(frame, stable_id)["bounds"]


def near(a, b, tolerance=0.5):
    return abs(a - b) <= tolerance


def prove_list(frame, ids, prefix, expected_name):
    list_id, first_id, second_id = (ids[prefix + suffix] for suffix in ("_list", "_a", "_b"))
    list_index, listing = find(frame, list_id)
    first_index, first = find(frame, first_id)
    second_index, second = find(frame, second_id)
    first_item_index, first_item = find(frame, first_id + "-item")
    second_item_index, second_item = find(frame, second_id + "-item")
    require(listing is not None and listing["role"] == "List"
            and listing["name"] == expected_name, "Named List role or label differs")
    require(first is not None and second is not None
            and first_item is not None and second_item is not None
            and first_item["role"] == "ListItem" and second_item["role"] == "ListItem"
            and first_item["parent"] == list_index and second_item["parent"] == list_index
            and first["parent"] == first_item_index and second["parent"] == second_item_index,
            "Action buttons must be inside their stable ListItem wrappers under the named List")
    require(first["role"] == "Button" and second["role"] == "Button",
            "List rows are not semantic action buttons")
    return {"list_id": list_id, "role": listing["role"], "name": listing["name"],
            "rows": [{"button": first_id, "listitem": first_id + "-item"},
                     {"button": second_id, "listitem": second_id + "-item"}],
            "parent_index": list_index}


def clip_diagnostic(path, profile_name, frame, stable_id):
    dom_nodes = frame["dom"]["nodes"]
    index, divider = find(frame, stable_id)
    require(divider is not None, "Cannot diagnose missing divider " + stable_id)

    def record(node_index):
        raw = dom_nodes[node_index]
        return {"index": node_index, "stable_id": raw.get("stable_id"),
                "role": raw.get("role"), "name": raw.get("name"),
                "parent": raw.get("parent"),
                "layout": frame["layout"]["nodes"][node_index]}

    chain = []
    cursor = index
    while cursor is not None and len(chain) < 12:
        chain.append(record(cursor))
        cursor = dom_nodes[cursor].get("parent")
    anchors = {}
    for anchor in ("divider-demo-scroll", "divider-demo-body",
                   "divider-demo-composition", "divider-demo-related-list",
                   "divider-demo-sheet-panel", "divider-demo-sheet-list"):
        anchor_index, anchor_node = find(frame, anchor)
        if anchor_node is not None:
            anchors[anchor] = record(anchor_index)
    value = {"scope": "strict divider visibility failure",
             "profile": profile_name,
             "viewport": frame["layout"]["viewport"],
             "interaction": frame["interaction"],
             "divider": {"stable_id": stable_id, "node": divider,
                         "layout": frame["layout"]["nodes"][index]},
             "ancestor_chain": chain, "scroller_and_composition": anchors}
    diagnostic = path.with_name(path.stem + "-clip-diagnostic.json")
    diagnostic.write_text(json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n")
    return diagnostic


def check_line_pixels(path, frame, ids, line_ids, profile_name):
    width, height, rows = read_png(path.read_bytes())
    viewport = frame["layout"]["viewport"]
    scale_x = width / viewport["width"]
    scale_y = height / viewport["height"]
    receipts = {}
    for stable_id in line_ids:
        line = at(frame, stable_id)
        rect = box(frame, stable_id)
        clip = measured(frame, stable_id)["clip"]
        # `clip` is the inherited clipping rectangle, not the divider's own
        # bounds. A child can be fully visible while occupying only part of
        # that rectangle. Require full containment in both the inherited clip
        # and the viewport; do not require the rectangles to be equal.
        viewport_rect = {"x": 0.0, "y": 0.0,
                         "width": viewport["width"],
                         "height": viewport["height"]}
        def contained(inner, outer):
            epsilon = 0.5
            return (inner["x"] >= outer["x"] - epsilon
                    and inner["y"] >= outer["y"] - epsilon
                    and inner["x"] + inner["width"]
                    <= outer["x"] + outer["width"] + epsilon
                    and inner["y"] + inner["height"]
                    <= outer["y"] + outer["height"] + epsilon)
        if not contained(rect, clip) or not contained(rect, viewport_rect):
            diagnostic = clip_diagnostic(path, profile_name, frame, stable_id)
            require(False, "Divider is clipped in the viewport: "
                    + stable_id + "; diagnostic: " + str(diagnostic))
        rgba = line.get("style", {}).get("background")
        require(isinstance(rgba, list) and len(rgba) == 4,
                "Divider has no RGBA background in DOM: " + stable_id)
        # Sample every output pixel whose center falls inside the displayed
        # line. This checks the complete rendered stroke, including its ends.
        x0 = max(0, int(rect["x"] * scale_x))
        x1 = min(width, int((rect["x"] + rect["width"]) * scale_x + 0.999999))
        y0 = max(0, int(rect["y"] * scale_y))
        y1 = min(height, int((rect["y"] + rect["height"]) * scale_y + 0.999999))
        x_samples = [x for x in range(x0, x1)
                     if rect["x"] <= (x + 0.5) / scale_x
                     < rect["x"] + rect["width"]]
        ys = [y for y in range(y0, y1)
              if rect["y"] <= (y + 0.5) / scale_y
              < rect["y"] + rect["height"]]
        coords = [[x, y] for x in x_samples for y in ys]
        actual = [list(rows[y][x * 4:x * 4 + 4]) for x, y in coords]
        require(actual and all(pixel == rgba for pixel in actual),
                "PNG divider pixels differ from DOM paint: " + stable_id)
        receipts[stable_id] = {"rgba": rgba, "sample_count": len(actual),
                               "bounds": rect, "sample_coords": coords,
                               "samples": actual}
    return receipts


def prove_wide(frame, ids, png, profile_name):
    related = prove_list(frame, ids, "related", "相關項目")
    a, b, rule = (box(frame, ids[k]) for k in
                  ("related_a", "related_b", "related_rule"))
    require(near(rule["height"], 1.) and near(rule["x"], a["x"] + 16.)
            and near(rule["width"], a["width"] - 16.),
            "LTR related-list inset must be 16 dp and 1 dp high")
    require(near(rule["y"], a["y"] + a["height"])
            and near(b["y"], rule["y"] + rule["height"]),
            "Related-list divider must sit between its two rows")
    full = box(frame, ids["full"])
    heavy = box(frame, ids["heavy"])
    before = box(frame, ids["heavy_before"])
    after = box(frame, ids["heavy_after"])
    group = box(frame, ids["heavy_group"])
    require(near(full["height"], 1.) and full["width"] > 100,
            "Full regular divider geometry differs")
    require(near(heavy["height"], 8.)
            and near(heavy["y"] - before["y"] - before["height"], 16.)
            and near(after["y"] - heavy["y"] - heavy["height"], 16.),
            "Heavy divider must be 8 dp with 16 dp before and after")
    require(heavy["x"] >= group["x"]
            and heavy["x"] + heavy["width"] <= group["x"] + group["width"] + 0.5,
            "Heavy divider leaves its group")
    for stable_id in (ids["related_rule"], ids["full"], ids["heavy"]):
        divider = at(frame, stable_id)
        require(divider["role"] == "Separator", "Decorative DOM node is not a separator")
    pixels = check_line_pixels(png, frame, ids,
                               (ids["related_rule"], ids["full"], ids["heavy"]),
                               profile_name)
    return {"list": related, "related": {"a": a, "divider": rule, "b": b},
            "full": full, "heavy_group": group, "heavy": heavy,
            "heavy_before": before, "heavy_after": after, "png_pixels": pixels}


# The native DevTools scope check rejects suppressed decorations before activation.
SEPARATOR_ACTIVATION_REJECTION = "outside_modal"


def reject_separator_activation(app, ids, keys):
    results = {}
    for key in keys:
        stable_id = ids[key]
        app.wire("Action.activate", {"id": stable_id},
                 expected_error=SEPARATOR_ACTIVATION_REJECTION)
        results[stable_id] = SEPARATOR_ACTIVATION_REJECTION
    return results


def prove_compact_sheet(frame, ids, png, profile_name):
    sheet = at(frame, ids["sheet"])
    require(sheet is not None and sheet["role"] == "Dialog"
            and sheet["name"] == "分隔線細節", "Modal SideSheet semantic node differs")
    listing = prove_list(frame, ids, "sheet", "草稿細節")
    a, b, rule = (box(frame, ids[k]) for k in ("sheet_a", "sheet_b", "sheet_rule"))
    require(near(rule["height"], 1.) and near(rule["x"], a["x"])
            and near(rule["width"], a["width"] - 16.),
            "RTL sheet inset must be 16 dp on logical trailing side")
    require(near(rule["y"], a["y"] + a["height"])
            and near(b["y"], rule["y"] + rule["height"]),
            "Sheet divider must sit between its two rows")
    divider = at(frame, ids["sheet_rule"])
    require(divider["role"] == "Separator",
            "SideSheet decorative DOM node is not a separator")
    pixels = check_line_pixels(png, frame, ids, (ids["sheet_rule"],), profile_name)
    return {"sheet": {"role": sheet["role"], "name": sheet["name"]},
            "list": listing, "a": a, "divider": rule, "b": b,
            "png_pixels": pixels}


def start_fixture(app, ids, settings):
    app.activate(ids["section"])
    frame = settle(app)
    app.activate(ids["show"])
    frame = settle(app)
    for setting in settings:
        app.activate(ids[setting])
        frame = settle(app)
    return frame


def sample_window(app, samples, name):
    require(app.proc.poll() is None, "Gallery exited before " + name)
    count = onscreen_windows(app.proc.pid)
    samples[name] = count
    require(count == 0, "Gallery owned a visible window at " + name)


def play(plan, config, profile_name, repeat_dir):
    profile = config["profiles"][profile_name]
    ids = config["ids"]
    windows = {}
    stage = "start"
    with launch(plan, profile_name, profile["width"], profile["height"]) as app:
        sample_window(app, windows, "ready")
        stage = "fixture"
        frame = start_fixture(app, ids, profile["settings"])
        require(frame["layout"]["viewport"]["width"] == profile["width"]
                and frame["layout"]["viewport"]["height"] == profile["height"],
                "Gallery viewport differs from requested size")
        require(node(frame, ids["theme"]).get("checked")
                is (profile_name != "wide-light"), "Theme setting differs")
        require(node(frame, ids["rtl"]).get("selected")
                is (profile_name != "wide-light"), "Layout direction differs")
        require(node(frame, ids["large_text"]).get("selected")
                is (profile_name != "wide-light"), "Text scale differs")
        if profile_name == "wide-light":
            stage = "wide list and Divider"
            if frame["interaction"]["focused"] != ids["related_a"]:
                focus_by_tab(app, ids["related_a"])
            frame = app.dom()
            focus(frame, ids["related_a"])
            frame = key(app, "down")
            focus(frame, ids["related_b"])
            shot = capture(app, plan, "wide-light-divider", frame=frame)
            geometry = prove_wide(frame, ids, plan.out / shot["name"], profile_name)
            status_before = at(frame, ids["activation_status"]).get("name", "")
            separator_activation = reject_separator_activation(
                app, ids, ("related_rule", "full", "heavy"))
            after_separator = app.dom()
            status_after = at(after_separator, ids["activation_status"]).get("name", "")
            require(status_after == status_before,
                    "Semantic separator activation changed list action state")
            frame = key(app, "enter")
            status = at(frame, ids["activation_status"])
            require(status is not None and "清單操作 1 次" in status.get("name", ""),
                    "Visible list action status did not record Enter activation")
            sample_window(app, windows, "captured")
            result = {"geometry": geometry,
                      "separator_semantic_activation": separator_activation,
                      "keyboard": {"arrow_down": ids["related_b"],
                                   "enter_activated": True},
                      "screenshot": shot}
        else:
            stage = "open modal SideSheet"
            app.activate(ids["sheet_open"])
            frame = settle(app)
            require(at(frame, ids["sheet"]) is not None, "SideSheet did not open")
            shot = capture(app, plan, "compact-dark-rtl-200-sheet", frame=frame)
            geometry = prove_compact_sheet(frame, ids, plan.out / shot["name"],
                                           profile_name)
            status_before = at(frame, ids["activation_status"]).get("name", "")
            separator_activation = reject_separator_activation(app, ids, ("sheet_rule",))
            after_separator = app.dom()
            status_after = at(after_separator, ids["activation_status"]).get("name", "")
            require(status_after == status_before,
                    "Semantic sheet-divider activation changed action state")
            frame = after_separator
            if frame["interaction"]["focused"] != ids["sheet_a"]:
                focus_by_tab(app, ids["sheet_a"])
            frame = app.dom()
            focus(frame, ids["sheet_a"])
            frame = key(app, "down")
            focus(frame, ids["sheet_b"])
            frame = key(app, "enter")
            frame = settle(app)
            status = at(frame, ids["activation_status"])
            require(status is not None and "面板操作 1 次" in status.get("name", ""),
                    "Visible sheet action status did not record row activation")
            frame = key(app, "escape")
            focus(frame, ids["sheet_open"])
            require(at(frame, ids["sheet"]) is None,
                    "Escape did not dismiss the modal SideSheet")
            sample_window(app, windows, "captured-and-closed")
            result = {"geometry": geometry,
                      "separator_semantic_activation": separator_activation,
                      "keyboard": {"arrow_down": ids["sheet_b"], "enter_activated": True,
                                   "escape_restored_focus": ids["sheet_open"]},
                      "screenshot": shot}
        sample_window(app, windows, "closed")
    require(app.proc.returncode is not None, "Gallery process was not reaped")
    result.update({"profile": profile_name, "pid": app.proc.pid,
                   "argv": ["--headless", "--width", str(profile["width"]),
                            "--height", str(profile["height"])],
                   "reaped": True,
                   "window_samples": windows})
    return result


def verify_output(out, evidence_sha, config_sha, source_sha, binary_sha):
    out = out.resolve(strict=True)
    files = pinned_entries(out, out / "SHA256SUMS", evidence_sha)
    actual = {p.relative_to(out).as_posix() for p in out.rglob("*")
              if p.is_file() and p.name != "SHA256SUMS"}
    require(set(files) == actual, "Evidence inventory differs")
    report = json.loads((out / REPORT).read_text())
    require(report["schema"] == SCHEMA
            and report["inputs"]["config_sha256"] == config_sha
            and report["inputs"]["source"]["manifest_sha256"] == source_sha
            and report["inputs"]["binaries"]["manifest_sha256"] == binary_sha
            and report["inputs_after"] == report["inputs"]
            and report["transports"] == TRANSPORTS
            and report["status"] == "shared DOM/focus pass; native pointer and macOS AX not verified"
            and report["pointer_reference"]["source_test"]
                == "crates/surface-material/tests/list.rs::decorative_divider_keeps_interactive_rows_in_one_named_list"
            and report["pointer_reference"]["verified_by_this_driver"] is False,
            "Evidence source pins or scope differ")
    ids = report["inputs"]["ids"]
    require(report["inputs"]["profiles"] == {
        "wide-light": {"width": 960, "height": 960, "settings": []},
        "compact-dark-rtl-200": {
            "width": 320, "height": 1360, "settings": ["theme", "rtl", "large_text"]}},
        "Profile receipt differs")
    require(report["pointer_reference"]["verified_by_this_driver"] is False
            and "no coordinate pointer method"
                in report["pointer_reference"]["headless_input_method"],
            "Pointer input scope differs")
    replays = report["replays"]
    require(set(replays) == {"1", "2"}, "Expected two repeats")
    for profile_name in ("wide-light", "compact-dark-rtl-200"):
        first = replays["1"][profile_name]
        second = replays["2"][profile_name]
        require(first["screenshot"]["sha256"] == second["screenshot"]["sha256"],
                "Repeated screenshot differs for " + profile_name)
        require(first["geometry"] == second["geometry"],
                "Repeated geometry differs for " + profile_name)
        pids = set()
        for record in (first, second):
            require(record["reaped"] is True and
                    set(record["window_samples"].values()) == {0},
                    "Window or process receipt differs")
            phases = ({"ready", "captured", "closed"}
                      if profile_name == "wide-light"
                      else {"ready", "captured-and-closed", "closed"})
            require(set(record["window_samples"]) == phases,
                    "Window sample phases differ")
            require(type(record["pid"]) is int and record["pid"] > 0
                    and record["pid"] not in pids, "Gallery process identity differs")
            pids.add(record["pid"])
            if profile_name == "wide-light":
                expected_rejections = {ids[key]: SEPARATOR_ACTIVATION_REJECTION
                                       for key in ("related_rule", "full", "heavy")}
                expected_rows = [
                    {"button": ids["related_a"], "listitem": ids["related_a"] + "-item"},
                    {"button": ids["related_b"], "listitem": ids["related_b"] + "-item"},
                ]
                require(record["separator_semantic_activation"] == expected_rejections
                        and record["geometry"]["list"]["rows"] == expected_rows
                        and record["keyboard"] == {
                            "arrow_down": ids["related_b"], "enter_activated": True},
                        "Wide separator or keyboard receipt differs")
            else:
                expected_rows = [
                    {"button": ids["sheet_a"], "listitem": ids["sheet_a"] + "-item"},
                    {"button": ids["sheet_b"], "listitem": ids["sheet_b"] + "-item"},
                ]
                require(record["separator_semantic_activation"]
                        == {ids["sheet_rule"]: SEPARATOR_ACTIVATION_REJECTION}
                        and record["geometry"]["list"]["rows"] == expected_rows
                        and record["keyboard"] == {
                            "arrow_down": ids["sheet_b"], "enter_activated": True,
                            "escape_restored_focus": ids["sheet_open"]},
                        "Compact sheet divider or keyboard receipt differs")
    for repeat, records in replays.items():
        for profile_name, record in records.items():
            screenshot = out / ("repeat-" + repeat) / record["screenshot"]["name"]
            require(digest(screenshot) == record["screenshot"]["sha256"],
                    "Screenshot digest differs")
            width, height, rows = read_png(screenshot.read_bytes())
            require((width, height) == (record["screenshot"]["width"],
                                        record["screenshot"]["height"]),
                    "Screenshot dimensions differ")
            for proof in record["geometry"]["png_pixels"].values():
                observed = [list(rows[y][x * 4:x * 4 + 4])
                            for x, y in proof["sample_coords"]]
                require(observed == proof["samples"] and observed
                        and all(pixel == proof["rgba"] for pixel in observed),
                        "Independent PNG pixel readback differs")
    require(len([p for p in files if p.endswith(".png")]) == 4,
            "Expected four original screenshots")
    return {"captures": 4, "gallery_processes_reaped": 4,
            "window_samples": sum(len(x["window_samples"])
                                  for pair in replays.values() for x in pair.values()),
            "evidence_sha256": evidence_sha,
            "status": "shared DOM/focus pass; native pointer and macOS AX not verified"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check-source", action="store_true")
    mode.add_argument("--check-inputs", action="store_true")
    mode.add_argument("--run", action="store_true")
    mode.add_argument("--verify", type=Path)
    parser.add_argument("--pins", type=Path, required=True)
    parser.add_argument("--pins-sha256", required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--evidence-sha256")
    args = parser.parse_args()
    config, source_files, binaries, gallery, cli = load_config(
        args.pins, args.pins_sha256, source_only=args.check_source)
    if args.check_source:
        print(json.dumps({"source_commit": config["source_commit"],
                          "source_tree": config["source_tree"],
                          "source_manifest_sha256": config["source_sha256"],
                          "source_files": len(source_files),
                          "worktree_clean": True},
                         indent=2, sort_keys=True))
        return
    inputs = {"config_sha256": args.pins_sha256,
              "source": {"commit": config["source_commit"],
                         "manifest_sha256": config["source_sha256"],
                         "tree": config["source_tree"], "files": len(source_files)},
              "binaries": {"manifest_sha256": config["binary_sha256"], "files": binaries},
              "ids": config["ids"], "profiles": config["profiles"]}
    if args.check_inputs:
        print(json.dumps(inputs, ensure_ascii=False, indent=2, sort_keys=True))
        return
    if args.verify:
        require(args.evidence_sha256, "--evidence-sha256 is required")
        result = verify_output(args.verify.resolve(strict=True),
                               args.evidence_sha256, args.pins_sha256,
                               config["source_sha256"], config["binary_sha256"])
        print(json.dumps(result, indent=2, sort_keys=True))
        return
    require(sys.platform == "darwin", "Native Metal and CoreGraphics checks require macOS")
    require(args.out is not None and args.out.resolve().is_relative_to(Path("/tmp").resolve())
            and (not args.out.exists() or
                 (args.out.is_dir() and not any(args.out.iterdir()))),
            "Use a new or empty output directory under /tmp")
    args.out = args.out.resolve()
    args.out.mkdir(mode=0o700, exist_ok=True)
    plan = SimpleNamespace(gallery=str(gallery), cli=str(cli), out=args.out)
    replays = {}
    for repeat in ("1", "2"):
        repeat_dir = args.out / ("repeat-" + repeat)
        repeat_dir.mkdir()
        repeat_plan = SimpleNamespace(gallery=str(gallery), cli=str(cli), out=repeat_dir)
        replays[repeat] = {}
        for profile_name in ("wide-light", "compact-dark-rtl-200"):
            replays[repeat][profile_name] = play(repeat_plan, config, profile_name, repeat_dir)
    config_again, source_files_again, binaries_again, gallery_again, cli_again = load_config(
        args.pins, args.pins_sha256)
    require(source_files_again == source_files and binaries_again == binaries
            and gallery_again == gallery and cli_again == cli,
            "Reviewed source or executable inputs changed during the run")
    report = {"schema": SCHEMA, "inputs": inputs, "transports": TRANSPORTS,
              "inputs_after": {"config_sha256": args.pins_sha256,
                               "source": {"commit": config_again["source_commit"],
                                          "manifest_sha256": config_again["source_sha256"],
                                          "tree": config_again["source_tree"],
                                          "files": len(source_files_again)},
                               "binaries": {"manifest_sha256": config_again["binary_sha256"],
                                            "files": binaries_again},
                               "ids": config_again["ids"], "profiles": config_again["profiles"]},
              "status": "shared DOM/focus pass; native pointer and macOS AX not verified",
              "pointer_reference": {
                  "source_test": "crates/surface-material/tests/list.rs::decorative_divider_keeps_interactive_rows_in_one_named_list",
                  "headless_input_method": "surface-devtools has no coordinate pointer method",
                  "verified_by_this_driver": False},
              "replays": replays}
    (args.out / REPORT).write_text(json.dumps(report, ensure_ascii=False,
                                               indent=2, sort_keys=True) + "\n")
    files = sorted(p for p in args.out.rglob("*") if p.is_file()
                   and p.name != "SHA256SUMS")
    (args.out / "SHA256SUMS").write_text("".join(
        f"{digest(p)}  {p.relative_to(args.out)}\n" for p in files))
    evidence_sha = digest(args.out / "SHA256SUMS")
    print(json.dumps(verify_output(args.out, evidence_sha, args.pins_sha256,
                                   config["source_sha256"],
                                   config["binary_sha256"]),
                     indent=2, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        sys.exit("Native Divider replay stopped: " + str(error))

