#!/usr/bin/env python3
"""Verify the Material Gallery boundary cases in headless Chrome on reviewed q79 inputs."""

import argparse
import json
import math
import os
from pathlib import Path
import re
from runpy import run_path
import shlex
import struct
import subprocess
import sys
from time import monotonic, sleep

from PIL import Image

from evidence_common import data, digest, json_file, require

ROOT = Path(__file__).resolve().parent
GUARD = ROOT / "observe-focused-original.py"
GUARD_SHA = "ae8dab806e95653212495348ba78ea8e4bb0ac396047da3cd3c967b9f9e3eca7"
STEPS = [str(i) for i in range(1, 383) if i != 23]
UPDATER_ARG = "--disable-updater-scheduler"
DISPLAY_VARS = {"DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"}
MENU_REPORTS = ["browser-material-command-menu.json", "browser-material-vertical-menu.json", "browser-material-choice-menu.json"]
MENU_IMAGES = ["material-command-menu-wide.png", "material-command-menu-compact.png", "material-command-menu-compact-footer.png", "material-vertical-menu-standard.png", "material-vertical-menu-vibrant.png", "material-vertical-menu-dark-rtl.png", "material-choice-menu-standard.png", "material-choice-menu-vibrant.png", "material-choice-menu-compact-dark-rtl.png"]
DIVIDER_REPORT = "browser-material-divider-composition.json"
DIVIDER_IMAGES = [f"material-divider-composition-{profile}-{scene}.png"
                  for profile in ("wide-light", "compact-dark-rtl-200")
                  for scene in ("before", "forced", "after", "sheet")]
OVERRIDES = {"SURFACE_CHROME", "SURFACE_CHROME_ARGS", "CHROME_PATH",
             "SURFACE_BRIDGE_SESSION", "SURFACE_DEVTOOLS_SESSION"}


def verify_m373(report):
    standard = report["standard"]
    expected = {
        "open": True,
        "sort": "menu-demo-choice-recent",
        "density": "menu-demo-choice-comfortable",
        "checked": ["menu-demo-choice-kind", "menu-demo-choice-locked"],
        "changes": 0,
        "focus": "menu-demo-choice-owner",
    }
    require(standard["disabled_mouse"] == standard["disabled"] == expected,
            "M373 must show the disabled pointer click preserving the open menu, focus, and choices")
    return expected


def verify_m375(report, screenshot):
    compact = report["compact"]
    paint = compact["paint"]
    tokens = paint["tokens"]
    expected_group, selected = [99, 59, 72], [239, 184, 200]
    require(tokens["group"] == paint["group_rgb"] == paint["corner_rgb"] == expected_group
            and tokens["selected_face"] == paint["fill"] == paint["inner_rgb"] == selected
            and paint["corners"] == [12, 12, 12, 12] and paint["face_contrast"] >= 3
            and paint["mark_pixels"] > 8, "M375 must prove the dark rounded leading corner and selected face")
    expected = {
        "sort": "menu-demo-choice-name",
        "density": "menu-demo-choice-compact",
        "checked": ["menu-demo-choice-kind", "menu-demo-choice-locked"],
        "changes": 6,
    }
    require(compact["toggled"] == {**expected, "open": True, "focus": "menu-demo-choice-size"}
            and compact["dismissed"] == {**expected, "open": False, "focus": "menu-demo-opener"}
            and compact["scroll"][1] > compact["scroll"][0]
            and compact["full_name"] == "檔案大小與可用空間的詳細資料",
            "M375 must prove scroll, wrapped label, unchecked state and Escape focus")
    x, y, width, _ = paint["face"]
    samples = [(width - 1, 1, expected_group), (width - 6, 3, selected), (width / 2, 4, selected)]
    with Image.open(screenshot) as png:
        require(png.format == "PNG" and png.size == (640, 1048), "Wrong exact M375 original PNG size")
        rgb = png.convert("RGB")
        for dx, dy, expected_pixel in samples:
            point = (math.floor((x + dx) * 2), math.floor((y + dy + 44) * 2))
            require(list(rgb.getpixel(point)) == expected_pixel, f"M375 original PNG pixel differs at {point}")
    return {"corner_rgb": paint["corner_rgb"], "inner_rgb": paint["inner_rgb"],
            "selected_face_rgb": selected, "group_rgb": expected_group,
            "focus_after_escape": compact["dismissed"]["focus"],
            "checked_after_escape": compact["dismissed"]["checked"]}


def inputs(args):
    require(digest(GUARD) == GUARD_SHA, "Different macOS observer source")
    config = data(args.config)
    require(re.fullmatch(r"[0-9a-f]{64}", args.config_sha256) and digest(args.config) == args.config_sha256, "Wrong reviewed config digest")
    require(set(config) == {"source", "source_manifest", "source_sha256", "source_tree", "source_files", "build_root", "build_manifest", "build_sha256", "runner", "runner_sha256", "web_root"}, "Wrong config fields")
    require(re.fullmatch(r"[0-9a-f]{40}", config["source_tree"]) and type(config["source_files"]) is int and config["source_files"] > 0, "Wrong source tree record or count")
    expected_source = Path("/tmp/surface-md3-v0179-divider.cOzU6oi6/surface").resolve(strict=True)
    expected_manifest = Path("/tmp/surface-md3-v0179-divider.cOzU6oi6/source.sha256").resolve(strict=True)
    require(Path(config["source"]).resolve(strict=True) == expected_source
            and Path(config["source_manifest"]).resolve(strict=True) == expected_manifest
            and config["source_sha256"] == "4ff1e8e17f1eb66d230ad43a8f3226bb126c31eea79791bda765f62c90b2b979"
            and config["source_tree"] == "c2470303ad4a11e356f93f6f66d361d8d78e13d5"
            and config["source_files"] == 708,
            "Config does not pin the exact q79 source")
    for key in ("source_sha256", "build_sha256", "runner_sha256"):
        require(re.fullmatch(r"[0-9a-f]{64}", config[key]), "Wrong SHA-256 in config")
    source = Path(config["source"]).resolve(strict=True)
    manifest = Path(config["source_manifest"]).resolve(strict=True)
    build = Path(config["build_root"]).resolve(strict=True)
    build_manifest = Path(config["build_manifest"]).resolve(strict=True)
    require(digest(manifest) == config["source_sha256"] and digest(build_manifest) == config["build_sha256"], "Source or build manifest differs")
    source_commit, source_tree_id = subprocess.check_output(
        ["git", "-C", str(source), "rev-parse", "HEAD", "HEAD^{tree}"], text=True
    ).splitlines()
    require(source_commit == "2c5003c521f518ed80c59035f2b74e86cf97ea73"
            and source_tree_id == config["source_tree"], "Source Git commit or tree differs")
    def read_entries(path, root):
        entries = {}
        for line in path.read_text().splitlines():
            sha, name = line.split("  ", 1)
            relative = Path(name)
            require(re.fullmatch(r"[0-9a-f]{64}",sha) and not relative.is_absolute() and ".." not in relative.parts and relative.as_posix() == name and name not in entries, "Invalid manifest entry")
            target = root / relative
            require(target.is_file() and not target.is_symlink() and target.resolve().is_relative_to(root) and digest(target) == sha, "Pinned file differs: " + name)
            entries[name] = sha
        return entries
    sources=read_entries(manifest, source)
    actual={p.relative_to(source).as_posix() for p in source.rglob("*") if (p.is_file() or p.is_symlink()) and p.relative_to(source).parts[0] != ".git"}
    require(len(sources) == config["source_files"] and set(sources) == actual, "Source inventory differs")
    builds=read_entries(build_manifest, build)
    runner=Path(config["runner"]).resolve(strict=True)
    web=Path(config["web_root"]).resolve(strict=True)
    require(runner.is_relative_to(build) and web.is_relative_to(build) and runner.relative_to(build).as_posix() in builds and
            builds[runner.relative_to(build).as_posix()] == config["runner_sha256"] and os.access(runner,os.X_OK), "Wrong built runner")
    require(any(name.startswith(web.relative_to(build).as_posix() + "/material-gallery/") for name in builds), "Web Gallery missing from build manifest")
    require((build/"BUILD-COMPLETE").is_file(), "Build is not sealed complete")
    args.runner, args.web_root = runner, web
    return {"config_sha256":args.config_sha256,"source_sha256":config["source_sha256"],"reviewed_source_tree":config["source_tree"],
            "source_files":len(sources),"runner_sha256":config["runner_sha256"],"build_manifest_sha256":config["build_sha256"],"observer_sha256":GUARD_SHA}


def verify_log(log):
    require(re.findall(r"^case (\S+) \.\.\.$", log, re.M) == ["material"],
            "Expected exactly one stock Material suite")
    require(re.findall(r"^  \[M(\d+)\]", log, re.M) == STEPS,
            "Expected all ordered M1–M382 steps except M23")
    rows = re.findall(r"^case material \.\.\. ok \((\d+) steps, (\d+) checks,", log, re.M)
    require(len(rows) == 1 and int(rows[0][0]) == len(STEPS) and int(rows[0][1]) >= len(STEPS),
            "Material result has invalid step or assertion totals")
    checks = rows[0][1]
    require(re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", log, re.M) == [checks],
            "Material result lacks the successful runner summary")
    return int(checks)


def verify_submenus(artifacts):
    report_path = artifacts / "browser-material-command-menu-submenus.json"
    report = data(report_path)
    require(report.get("errors") == [] and report.get("steps") == ["M376", "M377", "M378", "M379"]
            and re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", report.get("browser", "")), "Wrong q77 typed report")
    keyboard = report["keyboard"]
    parent = keyboard["parent"]
    properties = {item["name"]: item["value"]["value"] for item in parent.get("properties", []) if "value" in item.get("value", {})}
    require(keyboard["child_items"] == 2 and keyboard["keyboard"] == "ArrowRight"
            and parent["role"]["value"] == "menuitem" and parent["name"]["value"] == "匯出選項"
            and properties.get("hasPopup") == "menu" and properties.get("expanded") is True, "Wrong submenu accessibility or keyboard state")
    pointer, dismissal = dict(report["pointer"]), dict(report["dismissal"])
    pointer_focus = pointer.pop("pointer_focus")
    outside_focus = dismissal.pop("outside_focus")
    require(pointer == {"disabled_actions": 0, "export_actions": 1} and
            dismissal == {"escape_layers": 2, "tab_target": "menu-demo-list", "outside_action": 0},
            "Wrong submenu activation or dismissal state")
    require(set(pointer_focus) == {"open", "disabled", "return", "reopen", "dismiss"} and
            all(value in ("canvas", "projected") for value in (*pointer_focus.values(), outside_focus)),
            "Pointer interactions did not preserve Browser focus within Surface")
    compact = report["compact"]
    require(compact["viewport"] == [320, 480] and compact["dark"] is True and compact["rtl"] is True, "Wrong compact profile")
    def bounds(rect, viewport):
        require(isinstance(rect, list) and len(rect) == 4 and all(type(x) in (int,float) and math.isfinite(x) for x in rect), "Invalid menu geometry")
        x,y,w,h = rect
        require(w > 0 and h > 0 and 8 <= x < x+w <= viewport[0]-8 and 8 <= y < y+h <= viewport[1]-8, "Menu escapes safe viewport")
        return rect
    parent_rect, child_rect = bounds(compact["menu"],(320,480)), bounds(compact["submenu"],(320,480))
    bounds(keyboard["menu"],(960,800)); bounds(keyboard["submenu"],(960,800))
    screenshots = {}
    for name, size, scene in (("material-command-menu-submenu.png", (1920,1688), keyboard),
                              ("material-command-menu-submenu-dark-rtl.png", (640,1048), compact)):
        path = artifacts/name
        with Image.open(path) as raw:
            require(raw.format == "PNG" and raw.size == size, "Wrong original submenu PNG dimensions")
            image = raw.convert("RGB")
        require(len(image.getcolors(maxcolors=image.width*image.height) or []) > 2, "Submenu screenshot has no visible content")
        screenshots[name] = {"sha256": digest(path), "size": list(size)}
        canvas=scene["canvas"]; dpr=scene["device_pixel_ratio"]; surface=scene["surface"]
        viewport = [320,480] if "dark-rtl" in name else [960,800]
        require(isinstance(canvas,list) and len(canvas)==4 and all(type(v) in (int,float) and math.isfinite(v) for v in canvas)
                and canvas[2:] == viewport and dpr == 2 and len(surface)==4 and
                all(type(v) is int and 0<=v<=255 for v in surface) and surface[3]==255, "Invalid canvas or surface witness")
        menu,child,first=scene["menu"],scene["submenu"],scene["first"]
        require(len(first)==4 and first[2]>0 and first[3]>0 and child[0]<=first[0] and child[1]<=first[1] and
                first[0]+first[2]<=child[0]+child[2] and first[1]+first[3]<=child[1]+child[3], "First row outside child menu")
        if "dark-rtl" not in name:
            require(child[0]+child[2]<=menu[0] or child[0]>=menu[0]+menu[2] or
                    child[1]+child[3]<=menu[1] or child[1]>=menu[1]+menu[3], "Wide child does not escape parent")
        x,y,w,h=child
        require(w>=32 and h>=24, "Child menu is too small for an independent paint probe")
        # Top padding is clear of text. The thin side bands are a fallback if
        # a focus belt or another layer covers part of that padding.
        regions=[(x+8,y+2,x+w-8,y+7),(x+3,y+10,x+7,y+h-3),(x+w-7,y+10,x+w-3,y+h-3)]
        found=[]
        for l,t,r,b in regions:
            count=0
            for py in range(math.ceil((canvas[1]+t)*dpr), math.floor((canvas[1]+b)*dpr)):
                for px in range(math.ceil((canvas[0]+l)*dpr), math.floor((canvas[0]+r)*dpr)):
                    require(0<=px<image.width and 0<=py<image.height, "Paint probe outside original PNG")
                    rgb=image.getpixel((px,py))
                    if all(abs(actual-expected)<=1 for actual,expected in zip(rgb,surface[:3])): count+=1
            found.append(count)
        require(max(found)>=16, "No expected child surface pixels in clear edge probes")
        screenshots[name]["child_surface_rgb"]=surface[:3]
        screenshots[name]["edge_probe_matches"]=found
        screenshots[name]["canvas"]=canvas
        screenshots[name]["child_bounds"]=child
    return {"report_sha256":digest(report_path), "browser":report["browser"], "compact_parent":parent_rect,
            "compact_child":child_rect, "screenshots":screenshots,
            "pointer_focus":pointer_focus, "outside_focus":outside_focus}



def verify_forced_menu(artifacts):
    report_path = artifacts / "browser-material-choice-menu-forced.json"
    report = data(report_path)
    require(report.get("errors") == [] and report.get("steps") == ["M380"] and report.get("mode") == "forced"
            and re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", report.get("browser", "")), "Wrong forced-color menu report")
    def rgba(value, opaque=True):
        require(isinstance(value,list) and len(value)==4 and all(type(x) is int and 0<=x<=255 for x in value)
                and (value[3] == 255 if opaque else value[3] > 0), "Invalid CSS color")
        return value
    def pair(value, opaque=True):
        require(isinstance(value,dict) and set(value)=={"background","foreground"}, "Invalid CSS color pair")
        return [rgba(value["background"], opaque), rgba(value["foreground"], opaque)]
    selected = report["selected_item"]
    highlight = report["highlight"]
    # CSS Highlight can be translucent. SelectedItem still needs opaque ink here.
    pair(highlight, opaque=False)
    css_button=pair(report["css_button"])
    css_canvas=pair(report["css_canvas"])
    if selected is not None: pair(selected)
    require(type(report["selected_item_equals_highlight"]) is bool and
            report["selected_item_equals_highlight"] == (selected==highlight), "SelectedItem/Highlight equality differs")
    normal=report["normal_palettes"]
    require(set(normal)=={"standard","vibrant"} and all(isinstance(normal[n],list) and len(normal[n])==2 for n in normal),
            "Missing authored palettes")
    for values in normal.values():
        for value in values: rgba(value)
    restored=report["restored_palettes"]
    require(restored==normal, "Restored palettes differ from authored palettes")
    def ax_check(item,role,name,disabled=False):
        props={p["name"]:p["value"]["value"] for p in item.get("properties",[]) if "value" in p.get("value",{})}
        require(item["role"]["value"]==role and item["name"]["value"]==name and props.get("checked")=="true" and
                (not disabled or props.get("disabled") is True), "Wrong checked or disabled accessibility state")
    images={}
    for name in ("standard","vibrant"):
        section=report[name]; semantics=section["semantics"]
        ax_check(semantics["radio"],"menuitemradio","最近修改")
        ax_check(semantics["checkbox"],"menuitemcheckbox","檔案類型")
        ax_check(semantics["disabled_checkbox"],"menuitemcheckbox","鎖定狀態",True)
        state=section["disabled_activation"]
        require(state["sort"]=="menu-demo-choice-recent" and state["density"]=="menu-demo-choice-comfortable" and
                state["checked"]==["menu-demo-choice-kind","menu-demo-choice-locked"] and state["changes"]==0,
                "Disabled activation changed the selected values")
        palette=state["palette"]
        require(isinstance(palette,list) and len(palette)==2, "Missing actual forced palette")
        for value in palette: rgba(value)
        expected_pair = pair(selected) if selected is not None else (css_button if name=="standard" else [css_canvas[1],css_canvas[0]])
        require(palette==expected_pair, "Actual palette differs from CSS SelectedItem or required fallback")
        paint=section["pixels"]; file=f"material-choice-menu-forced-{name}.png"
        require(paint["file"]==file and paint["device_pixel_ratio"]==2 and
                isinstance(paint["viewport"],list) and len(paint["viewport"])==2 and
                isinstance(paint["canvas"],list) and len(paint["canvas"])==4, "Wrong PNG capture geometry")
        dpr=paint["device_pixel_ratio"]; canvas=paint["canvas"]; viewport=paint["viewport"]
        require(all(type(v) in (int,float) and math.isfinite(v) for v in canvas+viewport) and
                all(v>0 for v in viewport) and canvas[2]>0 and canvas[3]>0, "Invalid Canvas geometry")
        path=artifacts/file
        with Image.open(path) as raw:
            require(raw.format=="PNG" and raw.size==(int(viewport[0]*dpr),int(viewport[1]*dpr)), "Wrong original PNG size")
            bitmap=raw.convert("RGB")
        rows=paint["rows"]
        require([row["id"] for row in rows]==["menu-demo-choice-recent","menu-demo-choice-kind"], "Missing radio or checkbox pixel witness")
        row_results={}
        for row in rows:
            face,mark=row["face"],row["mark"]
            require(all(isinstance(r,list) and len(r)==4 and all(type(v) in (int,float) and math.isfinite(v) for v in r) for r in (face,mark)), "Invalid face or mark bounds")
            x,y,w,h=face; mx,my,mw,mh=mark
            require(w>0 and h>0 and mw>0 and mh>0 and x<=mx<mx+mw<=x+w and y<=my<my+mh<=y+h,
                    "Mark lies outside selected face")
            fill_xy=(math.floor((canvas[0]+x+w/2)*dpr),math.floor((canvas[1]+y+4)*dpr))
            require(0<=fill_xy[0]<bitmap.width and 0<=fill_xy[1]<bitmap.height, "Fill point is outside screenshot")
            fill=list(bitmap.getpixel(fill_xy))
            require(fill==row["fill"]==palette[0][:3] and row["foreground"]==palette[1][:3], "PNG fill or reported ink differs from actual palette")
            left,top,right,bottom=(math.floor((n* dpr)+0.5) for n in (canvas[0]+mx,canvas[1]+my,canvas[0]+mx+mw,canvas[1]+my+mh))
            require(0<=left<right<=bitmap.width and 0<=top<bottom<=bitmap.height, "Mark crop outside screenshot")
            count=sum(pixel==tuple(palette[1][:3]) for pixel in bitmap.crop((left,top,right,bottom)).getdata())
            require(count==row["mark_pixels"] and count>8, "PNG mark pixels differ from typed report")
            row_results[row["id"]]={"face":face,"mark":mark,"fill":fill,"mark_pixels":count}
        images[name]={"sha256":digest(path),"size":list(bitmap.size),"canvas":canvas,"rows":row_results,"actual_palette":palette}
    return {"report_sha256":digest(report_path),"browser":report["browser"],"selected_item":selected,
            "highlight":highlight,"selected_item_equals_highlight":report["selected_item_equals_highlight"],
            "normal_palettes":normal,"restored_palettes":restored,"css_button":report["css_button"],
            "css_canvas":report["css_canvas"],"images":images}


def verify_divider_compositions(artifacts):
    report_path = artifacts / DIVIDER_REPORT
    report = data(report_path)
    require(report.get("errors") == [] and report.get("steps") == ["M381", "M382"]
            and re.fullmatch(r"Chrome/\d+(?:\.\d+){3}", report.get("browser", "")),
            "Wrong q79 Divider composition report")
    adapter = report.get("adapter", {})
    require(adapter.get("isFallbackAdapter") is False and adapter.get("vendor") == "apple"
            and adapter.get("architecture", "").startswith("metal-"),
            "Expected the reviewed Apple Metal WebGPU adapter")

    def rect(value, label):
        require(isinstance(value, list) and len(value) == 4
                and all(type(v) in (int, float) and math.isfinite(v) for v in value),
                f"Invalid {label} rectangle")
        x, y, width, height = value
        require(width > 0 and height > 0, f"Empty {label} rectangle")
        return value

    def rgba(value, label):
        require(isinstance(value, list) and len(value) == 4
                and all(type(v) is int and 0 <= v <= 255 for v in value)
                and value[3] == 255, f"Invalid opaque {label}")
        return value

    def ax_nodes(values, expected, label):
        require(isinstance(values, list) and len(values) == len(expected), f"Wrong {label} node count")
        for node, (role, name) in zip(values, expected):
            require(node.get("role", {}).get("value") == role
                    and node.get("name", {}).get("value") == name,
                    f"Wrong {label} role or name")

    main_ax = [
        ("group", "相關項目範例"),
        ("group", "不相關項目範例"),
        ("group", "主要區段分隔線範例"),
        ("list", "相關項目"),
        ("button", "草稿設定"),
        ("button", "草稿版本"),
        ("heading", "其他內容"),
        ("heading", "目前工作"),
        ("heading", "封存工作"),
    ]
    sheet_ax = [
        ("dialog", "分隔線細節"),
        ("list", "草稿細節"),
        ("button", "草稿名稱"),
        ("button", "草稿位置"),
        ("heading", "其他資料"),
    ]
    profiles = {}
    expected_profiles = {
        "wide": ("wide-light", [960, 960], [202, 196, 207, 255]),
        "compact": ("compact-dark-rtl-200", [320, 1360], [73, 69, 78, 255]),
    }
    expected_line_ids = [
        "divider-demo-related-rule",
        "divider-demo-unrelated-divider",
        "divider-demo-heavy-major-break",
    ]
    for key, (profile_name, viewport, authored) in expected_profiles.items():
        section = report.get(key)
        require(isinstance(section, dict) and section.get("profile") == profile_name,
                f"Missing {profile_name} profile")
        authored_rgba = rgba(section.get("authored_outline_variant"), f"{profile_name} authored outline")
        require(authored_rgba == authored, f"Wrong {profile_name} authored outline color")
        ax_nodes(section.get("accessibility"), main_ax, f"{profile_name} main accessibility")
        ax_nodes(section.get("sheet_accessibility"), sheet_ax, f"{profile_name} sheet accessibility")

        forced_appearance = section.get("forced_appearance", {})
        forced_colors = forced_appearance.get("system_colors")
        require(forced_appearance.get("mode") == "forced" and isinstance(forced_colors, dict)
                and isinstance(forced_colors.get("canvas"), dict),
                f"Missing forced Canvas colors for {profile_name}")
        css_canvas = section.get("css_canvas")
        require(isinstance(css_canvas, dict) and set(css_canvas) == {"background", "foreground"},
                f"Missing independent CSS Canvas pair for {profile_name}")
        canvas_text = rgba(css_canvas["foreground"], f"{profile_name} CSS CanvasText")
        require(forced_colors["canvas"] == css_canvas,
                f"Forced Canvas pair differs from direct CSS for {profile_name}")
        normal = section.get("normal_appearance", {})
        require(normal.get("system_colors") is None,
                f"System colors were not cleared for {profile_name}")

        geometry = section.get("related_geometry", {})
        first = rect(geometry.get("first"), f"{profile_name} first row")
        second = rect(geometry.get("second"), f"{profile_name} second row")
        related_list = rect(geometry.get("list"), f"{profile_name} related list")
        related = rect(geometry.get("rule"), f"{profile_name} related divider")
        before = rect(geometry.get("heavy_before"), f"{profile_name} heavy preceding heading")
        heavy = rect(geometry.get("heavy"), f"{profile_name} Heavy divider")
        after = rect(geometry.get("heavy_after"), f"{profile_name} heavy following heading")
        require(abs((related_list[2] - related[2]) - 16.) <= 0.01
                and abs((related[0] - related_list[0]) - (0. if key == "compact" else 16.)) <= 0.01,
                f"Wrong {profile_name} logical-leading inset geometry")
        require(first[1] + first[3] <= related[1] and related[1] + related[3] <= second[1],
                f"{profile_name} inset is not between its related rows")
        require(abs(heavy[3] - 8.) <= 0.01
                and abs((heavy[1] - before[1] - before[3]) - 16.) <= 0.01
                and abs((after[1] - heavy[1] - heavy[3]) - 16.) <= 0.01,
                f"Wrong {profile_name} Heavy line or caller spacing")

        line_images = {}
        for mode, expected_color in (("before", authored[:3]), ("forced", canvas_text[:3]),
                                     ("after", authored[:3])):
            capture = section.get(mode, {})
            filename = f"material-divider-composition-{profile_name}-{mode}.png"
            require(capture.get("file") == filename and capture.get("device_pixel_ratio") == 2,
                    f"Wrong {profile_name} {mode} PNG metadata")
            canvas = rect(capture.get("canvas"), f"{profile_name} {mode} canvas")
            require(canvas[2:] == viewport, f"Wrong {profile_name} {mode} viewport")
            lines = capture.get("lines")
            require(isinstance(lines, list) and [line.get("id") for line in lines] == expected_line_ids,
                    f"Wrong {profile_name} {mode} divider rows")
            sampled = []
            for line, expected_height in zip(lines, (1., 1., 8.)):
                bounds = rect(line.get("bounds"), f"{profile_name} {mode} {line['id']}")
                color = line.get("rgb")
                require(color == expected_color and abs(bounds[3] - expected_height) <= 0.01
                        and bounds[2] > 4., f"Wrong {profile_name} {mode} line color or geometry")
                sampled.append((line["id"], bounds, color))
            image_result = sample_png(artifacts / filename, canvas, capture["device_pixel_ratio"],
                                      viewport, sampled, f"{profile_name} {mode}")
            line_images[mode] = image_result

        sheet = section.get("sheet", {})
        sheet_file = f"material-divider-composition-{profile_name}-sheet.png"
        require(sheet.get("file") == sheet_file and sheet.get("device_pixel_ratio") == 2
                and sheet.get("focus_return") == "divider-demo-sheet-open",
                f"Wrong {profile_name} sheet report or focus handback")
        sheet_canvas = rect(sheet.get("canvas"), f"{profile_name} sheet canvas")
        require(sheet_canvas[2:] == viewport and sheet.get("rgb") == authored[:3],
                f"Wrong {profile_name} sheet canvas or restored color")
        panel = rect(sheet.get("panel"), f"{profile_name} sheet panel")
        sheet_list = rect(sheet.get("list"), f"{profile_name} sheet list")
        inset = rect(sheet.get("inset"), f"{profile_name} sheet inset")
        full = rect(sheet.get("full"), f"{profile_name} sheet full divider")
        require(abs((sheet_list[2] - inset[2]) - 16.) <= 0.01
                and abs((inset[0] - sheet_list[0]) - (0. if key == "compact" else 16.)) <= 0.01
                and contains(panel, inset) and contains(panel, full),
                f"Wrong {profile_name} sheet divider geometry")
        sheet_samples = [
            ("divider-demo-sheet-inset", inset, authored[:3]),
            ("divider-demo-sheet-full", full, authored[:3]),
        ]
        sheet_image = sample_png(artifacts / sheet_file, sheet_canvas, sheet["device_pixel_ratio"],
                                 viewport, sheet_samples, f"{profile_name} sheet")
        heading = sheet.get("heading", {})
        heading_bounds = rect(heading.get("bounds"), f"{profile_name} sheet heading")
        heading_rgba = rgba(heading.get("rgba"), f"{profile_name} heading color")
        require(heading_rgba == normal.get("theme", {}).get("on_surface")
                and contains(panel, heading_bounds), f"Wrong {profile_name} heading palette or geometry")
        require(type(heading.get("ink_pixels")) is int and heading["ink_pixels"] > 8,
                f"Missing {profile_name} heading ink witness")
        with Image.open(artifacts / sheet_file) as raw:
            image = raw.convert("RGB")
        hx, hy, hw, hh = heading_bounds
        sx, sy = sheet_canvas[:2]
        dpr = sheet["device_pixel_ratio"]
        left, top, right, bottom = (math.floor(v * dpr + 0.5) for v in
            (sx + hx, sy + hy, sx + hx + hw, sy + hy + hh))
        require(0 <= left < right <= image.width and 0 <= top < bottom <= image.height,
                f"{profile_name} heading crop is outside original PNG")
        count = sum(pixel == tuple(heading_rgba[:3]) for pixel in image.crop((left, top, right, bottom)).getdata())
        require(count == heading["ink_pixels"] and count > 8,
                f"{profile_name} heading pixel count differs from original PNG")
        sheet_image["heading"] = {"bounds": heading_bounds, "rgba": heading_rgba, "ink_pixels": count}
        profiles[key] = {
            "profile": profile_name,
            "authored_outline_variant": authored_rgba,
            "canvas_text": canvas_text,
            "related_geometry": {"first": first, "rule": related, "second": second,
                                 "list": related_list},
            "heavy_geometry": {"before": before, "line": heavy, "after": after},
            "sheet_focus_return": sheet["focus_return"],
            "images": {**line_images, "sheet": sheet_image},
        }
    return {"report_sha256": digest(report_path), "browser": report["browser"],
            "profiles": profiles, "images": DIVIDER_IMAGES}


def contains(outer, inner):
    x, y, width, height = outer
    a, b, c, d = inner
    return x <= a and y <= b and a + c <= x + width and b + d <= y + height


def sample_png(path, canvas, dpr, viewport, samples, label):
    with Image.open(path) as raw:
        require(raw.format == "PNG", f"{label} is not a PNG")
        expected_size = (int(round((canvas[0] + canvas[2]) * dpr)),
                         int(round((canvas[1] + canvas[3]) * dpr)))
        require(raw.size == expected_size, f"Wrong {label} PNG dimensions")
        image = raw.convert("RGB")
    require(canvas[2:] == viewport, f"Wrong {label} screenshot viewport")
    found = {}
    for name, bounds, expected in samples:
        require(contains([0., 0., viewport[0], viewport[1]], bounds),
                f"{label} {name} lies outside its viewport")
        points = []
        for x in (bounds[0] + 1.5, bounds[0] + bounds[2] / 2., bounds[0] + bounds[2] - 1.5):
            for depth in range(int(round(bounds[3]))):
                px = int(math.floor((canvas[0] + x) * dpr))
                py = int(math.floor((canvas[1] + bounds[1] + depth + 0.5) * dpr))
                require(0 <= px < image.width and 0 <= py < image.height,
                        f"{label} sample lies outside the PNG")
                actual = list(image.getpixel((px, py)))
                require(actual == expected, f"{label} {name} pixel differs at {(px, py)}")
                points.append([px, py])
        found[name] = {"bounds": bounds, "rgb": expected, "sample_points": points}
    return {"sha256": digest(path), "size": list(image.size), "canvas": canvas, "samples": found}


def verify_run(out, window):
    require(window["stock_exit"] == 0 and not window["problems"]
            and not window["visible_owned_windows"] and not window["remaining_observed_processes"],
            "The suite, window observation, or private process cleanup failed")
    launches = window["guard_events"]
    chrome = window["launch_observations"]["chrome-primary"]
    require(len(launches) == 1 and launches[0]["role"] == "chrome-primary"
            and launches[0]["allowed"] and chrome["samples"] > 0,
            "A single guarded Chrome launch must be observed")
    flags = launches[0]["argv"]
    require(flags.count("--headless=new") == flags.count(UPDATER_ARG) == 1 and "--" not in flags,
            "Guarded Chrome arguments differ")

    checks = verify_log((out / "runner.log").read_text())
    artifacts = out / "artifacts"
    artifacts = out / "artifacts"
    files = {}
    for name in MENU_REPORTS:
        path = artifacts / name
        report = data(path)
        require(report["errors"] == [] and "failure" not in report, f"Menu report has browser errors: {name}")
        fallback = report.get("adapter", {}).get("isFallbackAdapter")
        require(fallback is False and report["adapter"].get("vendor") == "apple" and
                report["adapter"].get("architecture", "").startswith("metal-"), f"Expected Apple Metal WebGPU: {name}")
        files[name] = {"sha256": digest(path), "webgpu_fallback": fallback}
        if name == "browser-material-choice-menu.json":
            files[name]["m373_disabled_mouse"] = verify_m373(report)
            files[name]["m375_rtl_leading_corner"] = verify_m375(
                report, artifacts / "material-choice-menu-compact-dark-rtl.png")
    for name in MENU_IMAGES:
        path = artifacts / name
        with path.open("rb") as stream:
            header = stream.read(24)
        require(len(header) == 24 and header[:8] == b"\x89PNG\r\n\x1a\n" and header[12:16] == b"IHDR",
                f"Invalid screenshot: {name}")
        width, height = struct.unpack(">II", header[16:24])
        require(width > 0 and height > 0, f"Invalid dimensions: {name}")
        files[name] = {"sha256": digest(path), "size": [width, height]}
    submenu=verify_submenus(artifacts)
    require(submenu["browser"] == data(artifacts / "browser-material-choice-menu.json")["browser"], "Browser identity differs between reports")
    forced=verify_forced_menu(artifacts)
    require(forced["browser"]==submenu["browser"], "Forced-color report uses a different browser")
    dividers=verify_divider_compositions(artifacts)
    require(dividers["browser"]==submenu["browser"], "Divider report uses a different browser")
    files[DIVIDER_REPORT] = {"sha256": dividers["report_sha256"]}
    for name, image in zip(DIVIDER_IMAGES,
                           [dividers["profiles"][key]["images"][scene]
                            for key in ("wide", "compact")
                            for scene in ("before", "forced", "after", "sheet")]):
        files[name] = {"sha256": image["sha256"], "size": image["size"]}
    return {"steps": len(STEPS), "checks": checks, "chromium_samples": chrome["samples"], "menus": files,
            "submenus": submenu, "forced_choice":forced, "divider_compositions":dividers}


def observe(args, out, pins):
    observer_code = run_path(str(GUARD))
    chrome = (args.chrome or observer_code["CHROME"]).resolve(strict=True)
    out.mkdir(mode=0o700)
    for name in ("guards", "guard-ledger", "tmp", "artifacts"):
        (out / name).mkdir(mode=0o700)
    guard = out / "guards" / "chrome"
    prefix = [sys.executable, "-B", str(GUARD), "__guard", "chrome", str(chrome)]
    guard.write_text("#!/bin/sh\nexec " + shlex.join(prefix) + ' "$@"\n')
    guard.chmod(0o700)
    command = [str(args.runner.resolve()), "--chrome", str(guard), "--chrome-arg", UPDATER_ARG,
               "--web-root", str(args.web_root.resolve()), "--artifacts", str(out / "artifacts"), "material"]
    excluded = DISPLAY_VARS | OVERRIDES | {observer_code["JET_MARKER"]}
    environment = {name: value for name, value in os.environ.items()
                   if name not in excluded and not (name.startswith("SURFACE_") and name.endswith("_FOCUSED"))}
    environment.update({"TMPDIR": str(out / "tmp"), observer_code["PRIVATE_ROOT"]: str(out)})
    json_file(out / "invocation.json", {"argv": command, "inputs": pins})
    windows = observer_code["WindowOwners"]()
    with (out / "runner.log").open("xb") as log:
        leader = subprocess.Popen(command, cwd=out, env=environment, start_new_session=True,
                                  stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
        observer = observer_code["Observer"](out, windows, "/usr/bin/false", chrome, leader.pid)
        deadline = monotonic() + args.timeout
        problems = []
        owned = {}
        try:
            while leader.poll() is None:
                owned = observer.sample()
                if observer.visible or any(not event["allowed"] for event in observer.guards.values()):
                    problems.append("Visible owned window or rejected pre-exec guard")
                    break
                if monotonic() > deadline:
                    problems.append("Stock Material suite timed out")
                    break
                sleep(0.05)
            if problems:
                observer_code["terminate_owned"](leader, owned)
            owned = observer.sample()
            cleanup = monotonic() + 8
            while owned and monotonic() < cleanup:
                sleep(0.1)
                owned = observer.sample()
            if owned:
                problems.append("Private process survived cleanup grace")
        finally:
            if leader.poll() is None or owned:
                observer_code["terminate_owned"](leader, owned)
            exit_status = leader.wait(timeout=8)
            window = observer.report(exit_status, observer.sample(), problems)
            json_file(out / "headless-window-observation.json", window)
    return window


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--config-sha256", required=True)
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--timeout", type=int, default=1800)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check-inputs", action="store_true")
    mode.add_argument("--verify", action="store_true")
    mode.add_argument("--recover-receipt", action="store_true")
    mode.add_argument("--run", action="store_true")
    args = parser.parse_args()
    pins = inputs(args)
    if args.check_inputs:
        print("PASS reviewed q79 runner and Web inputs. No executable started.")
        return
    require(args.out is not None, "Provide a private output under /tmp")
    out = args.out.resolve()
    temp = Path("/tmp").resolve()
    require(out.is_relative_to(temp) and out != temp, "Output must be private and under /tmp")
    if args.verify or args.recover_receipt:
        require(data(out / "invocation.json")["inputs"] == pins
                and data(out / "inputs-after.json") == pins, "Run used different inputs")
        result = verify_run(out, data(out / "headless-window-observation.json"))
        receipt = out / "verified-stock-material.json"
        if args.recover_receipt:
            require(not receipt.exists(), "Recovery cannot replace an existing receipt")
            json_file(receipt, result)
        elif receipt.exists():
            require(data(receipt) == json.loads(json.dumps(result)), "Stored Browser receipt differs")
    else:
        window = observe(args, out, pins)
        require(inputs(args) == pins, "Build inputs changed while the suite ran")
        json_file(out / "inputs-after.json", pins)
        result = verify_run(out, window)
        json_file(out / "verified-stock-material.json", result)
    print(f"PASS {result['steps']} Material steps and {result['checks']} checks. Receipt: {out}")


if __name__ == "__main__":
    main()
