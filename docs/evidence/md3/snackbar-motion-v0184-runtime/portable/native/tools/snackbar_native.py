#!/usr/bin/env python3
"""Headless native q84 Snackbar smoke replay. Live geometry is observational."""
import argparse, hashlib, json, os, re, subprocess, time
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import capture, layout, node, settle
from window_check import onscreen_windows

SHA = re.compile(r"[0-9a-f]{64}")
WIDTH, HEIGHT = 320, 720
PROFILES = (("light-320-rtl-200", False, "snackbar-show-undo"),
            ("dark-320-rtl-200", True, "snackbar-show-long"))
TRANSPORTS = ["Page.screenshot", "CLI", "MCP"]


def pinned(root, manifest, expected):
    root = root.resolve(strict=True)
    require(SHA.fullmatch(expected or "") and digest(manifest) == expected,
            "manifest SHA differs")
    values = {}
    for line in manifest.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        relative = Path(name)
        path = root / relative
        require(SHA.fullmatch(checksum) and not relative.is_absolute()
                and ".." not in relative.parts and name not in values
                and path.is_file() and not path.is_symlink()
                and path.resolve().is_relative_to(root)
                and digest(path) == checksum, "invalid manifest entry")
        values[name] = checksum
    return values


def inputs(path, expected, source_only=False):
    require(SHA.fullmatch(expected or "") and digest(path) == expected,
            "config SHA differs")
    cfg = json.loads(path.read_text())
    require(set(cfg) == {"source", "source_manifest", "source_sha256",
                         "source_commit", "source_tree", "source_files",
                         "binary_manifest", "binary_sha256"},
            "wrong config schema")
    source = Path(cfg["source"]).resolve(strict=True)
    source_files = pinned(source, Path(cfg["source_manifest"]), cfg["source_sha256"])
    actual = {p.relative_to(source).as_posix() for p in source.rglob("*")
              if (p.is_file() or p.is_symlink())
              and p.relative_to(source).parts[0] != ".git"}
    require(len(source_files) == cfg["source_files"]
            and set(source_files) == actual, "source inventory differs")
    env = dict(os.environ, GIT_OPTIONAL_LOCKS="0")
    revision = subprocess.check_output(
        ["git", "-C", str(source), "rev-parse", "HEAD", "HEAD^{tree}"],
        text=True, env=env).splitlines()
    status = subprocess.check_output(
        ["git", "-C", str(source), "status", "--porcelain"],
        text=True, env=env)
    require(revision == [cfg["source_commit"], cfg["source_tree"]] and not status,
            "source pin or clean worktree differs")
    if source_only:
        return cfg, None
    manifest = Path(cfg["binary_manifest"]).resolve(strict=True)
    binary_files = pinned(manifest.parent, manifest, cfg["binary_sha256"])
    require(set(binary_files) == {"surface-material-gallery", "surface-devtools"},
            "native binary inventory differs")
    return cfg, manifest.parent


def sample_window(app, windows, label):
    require(app.proc.poll() is None, "Gallery exited before cleanup")
    windows[label] = onscreen_windows(app.proc.pid)
    require(windows[label] == 0, f"Visible Gallery window at {label}")


def panel_for(frame):
    ids = [node.get("stable_id", "") for node in frame["dom"]["nodes"]]
    panels = [item for item in ids
              if isinstance(item, str) and re.fullmatch(r"gallery-snackbar-[0-9]+", item)]
    require(len(panels) == 1, f"Expected one visible Snackbar, found {panels}")
    return panels[0]


def bounds(frame, stable_id):
    return {key: float(value) for key, value in
            layout(frame, stable_id)["bounds"].items()
            if key in ("x", "y", "width", "height")}


def observe_motion(app, stable_id, wait_for_removal=False):
    started = time.monotonic()
    samples = []
    previous = None
    stable = 0
    while time.monotonic() - started < 2.5:
        frame = app.dom()
        try:
            measured = bounds(frame, stable_id)
        except RuntimeError:
            if wait_for_removal:
                return {"samples": samples,
                        "removed_after_ms": round((time.monotonic() - started) * 1000, 2),
                        "frame": frame}
            raise
        elapsed = round((time.monotonic() - started) * 1000, 2)
        samples.append({"elapsed_ms": elapsed, **measured})
        if previous and all(abs(measured[key] - previous[key]) < 0.1 for key in measured):
            stable += 1
        else:
            stable = 0
        previous = measured
        if not wait_for_removal and stable >= 3:
            return {"samples": samples, "settled_after_ms": elapsed, "frame": frame}
        time.sleep(0.025)
    raise RuntimeError(f"Snackbar did not {'leave' if wait_for_removal else 'settle'}")


def pixel_check(frame, stable_id, image_path):
    panel = node(frame, stable_id)
    status = node(frame, stable_id + "-text")
    action = node(frame, stable_id + "-action")
    require(status.get("role") == "Status" and status.get("text"),
            "Snackbar status is missing")
    require(action.get("role") == "Button", "Snackbar action is missing")
    box = bounds(frame, stable_id)
    require(box["x"] >= 0 and box["y"] >= 0
            and box["x"] + box["width"] <= WIDTH
            and box["y"] + box["height"] <= HEIGHT
            and abs(HEIGHT - box["y"] - box["height"] - 16) <= 1.5,
            f"Snackbar placement differs: {box}")
    color = panel["style"]["background"]
    require(len(color) == 4 and color[3] == 255, "Snackbar color is missing")
    png_w, png_h, rows = read_png(image_path.read_bytes())
    require((png_w, png_h) == (WIDTH, HEIGHT), "Screenshot viewport differs")
    x = round(box["x"] + 2)
    y = round(box["y"] + box["height"] / 2)
    sample = list(rows[y][x * 4:x * 4 + 4])
    require(sample[3] == color[3]
            and max(abs(sample[index] - color[index]) for index in range(3)) <= 1,
            "Snackbar center-edge pixel differs from its live style")
    return {"bounds": box, "status": status["text"], "surface_rgba": color,
            "sample_pixel": sample, "sample_point": [x, y]}


def run_profile(plan, name, dark, opener):
    windows = {}
    pid = None
    process = None
    with launch(plan, name, WIDTH, HEIGHT) as app:
        process = app.proc
        pid = process.pid
        frame = settle(app)
        sample_window(app, windows, "ready")
        app.activate("section-2")
        frame = settle(app)
        for control in ("snackbar-direction", "snackbar-scale"):
            app.activate(control)
            frame = settle(app)
        if dark:
            app.activate("theme")
            frame = settle(app)
        require(node(frame, "theme").get("checked") is dark, "dark mode differs")
        for control, label in (("snackbar-direction", "由左至右"),
                               ("snackbar-scale", "一般文字")):
            require(node(frame, control).get("name") == label,
                    f"{control} toggle did not expose its new action")
        app.activate(opener)
        panel = panel_for(app.dom())
        entry = observe_motion(app, panel)
        frame = entry.pop("frame")
        shot = capture(app, plan, name, frame=frame)
        pixel = pixel_check(frame, panel, plan.out / shot["name"])
        sample_window(app, windows, "visible")
        action = panel + "-action"
        tab_trace = focus_by_tab(app, action)
        require(app.dom()["interaction"]["focused"] == action,
                "Tab did not focus the Snackbar action")
        app.key("enter")
        exit_motion = observe_motion(app, panel, wait_for_removal=True)
        dismissed = exit_motion.pop("frame")
        focus = dismissed["interaction"]["focused"]
        require(focus == opener, f"Focus did not return to {opener}: {focus}")
        require(not any(item.get("stable_id") == panel
                        for item in dismissed["dom"]["nodes"]),
                "Dismissed Snackbar remains in the DOM")
        sample_window(app, windows, "dismissed")
    require(process is not None and process.returncode is not None,
            "Gallery process was not reaped")
    return {
        "profile": name, "viewport": [WIDTH, HEIGHT], "dark": dark,
        "direction": "rtl", "text_scale": 2, "opener": opener,
        "pid": pid, "reaped": True, "windows": windows,
        "panel_id": panel, "frame": frame, "screenshot": shot,
        "pixel_readback": pixel, "entry_geometry": entry,
        "dismissal": {"focus": focus, "action": action,
                      "tab_trace": tab_trace, "exit_geometry": exit_motion},
    }


def verify(root, seal, config_sha):
    root = root.resolve(strict=True)
    require(SHA.fullmatch(seal or "") and digest(root / "SHA256SUMS") == seal,
            "output seal differs")
    entries = pinned(root, root / "SHA256SUMS", seal)
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
              if p.is_file() and p != root / "SHA256SUMS"}
    require(actual == set(entries), "output inventory differs")
    config_path = root / "config.json"
    require(SHA.fullmatch(config_sha or "") and digest(config_path) == config_sha,
            "archived config SHA differs")
    config = json.loads(config_path.read_text())
    require(set(config) == {"source", "source_manifest", "source_sha256",
                            "source_commit", "source_tree", "source_files",
                            "binary_manifest", "binary_sha256"},
            "archived config schema differs")
    require(digest(root / "source-manifest.sha256") == config["source_sha256"]
            and digest(root / "binary-manifest.sha256") == config["binary_sha256"],
            "archived source or binary manifest differs")
    report = json.loads((root / "report.json").read_text())
    require(report["config_sha256"] == config_sha
            and report["source_sha256"] == config["source_sha256"]
            and report["source_commit"] == config["source_commit"]
            and report["source_tree"] == config["source_tree"]
            and report["binary_sha256"] == config["binary_sha256"]
            and report["inputs_before"] == report["inputs_after"]
            and report["transports"] == TRANSPORTS
            and report["profiles"] == [profile[0] for profile in PROFILES],
            "run inputs or transport receipt differs")
    for name, dark, opener in PROFILES:
        result = report["runs"][name]
        require(result["dark"] is dark and result["viewport"] == [WIDTH, HEIGHT]
                and result["direction"] == "rtl" and result["text_scale"] == 2
                and result["opener"] == opener and result["reaped"] is True
                and set(result["windows"]) == {"ready", "visible", "dismissed"}
                and all(count == 0 for count in result["windows"].values()),
                f"profile or cleanup differs: {name}")
        require(result["dismissal"]["focus"] == opener
                and result["dismissal"]["action"] == result["panel_id"] + "-action",
                f"keyboard dismissal or focus return differs: {name}")
        image = root / "captures" / result["screenshot"]["name"]
        require(digest(image) == result["screenshot"]["sha256"],
                f"original image hash differs: {name}")
        require(result["screenshot"]["width"] == WIDTH
                and result["screenshot"]["height"] == HEIGHT,
                f"image dimensions differ: {name}")
        require(pixel_check(result["frame"], result["panel_id"], image)
                == result["pixel_readback"], f"independent pixel readback differs: {name}")
    return {"profiles": report["profiles"], "screenshots": 2,
            "transports": TRANSPORTS, "zero_window_samples": 6,
            "reaped_children": 2,
            "scope": "Settled screenshots and sampled DOM geometry only. No fixed animation curve or VoiceOver claim."}


def main():
    parser = argparse.ArgumentParser()
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--check-source", action="store_true")
    modes.add_argument("--check-inputs", action="store_true")
    modes.add_argument("--run", action="store_true")
    modes.add_argument("--verify", type=Path)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--config-sha256", required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--evidence-sha256")
    args = parser.parse_args()
    if args.verify:
        print(json.dumps(verify(args.verify, args.evidence_sha256,
                                args.config_sha256), indent=2))
        return
    cfg, binaries = inputs(args.config, args.config_sha256, args.check_source)
    if args.check_source or args.check_inputs:
        print("Source and build inputs verified" if binaries else "Source verified")
        return
    require(args.out is not None, "output path is required")
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()) and not out.exists(),
            "use a fresh path under /tmp")
    out.mkdir(mode=0o700)
    (out / "captures").mkdir()
    (out / "config.json").write_bytes(args.config.read_bytes())
    (out / "source-manifest.sha256").write_bytes(
        Path(cfg["source_manifest"]).read_bytes())
    (out / "binary-manifest.sha256").write_bytes(
        Path(cfg["binary_manifest"]).read_bytes())
    before = {"source": cfg["source_sha256"], "binary": cfg["binary_sha256"]}
    runs = {}
    for name, dark, opener in PROFILES:
        plan = SimpleNamespace(
            gallery=str(binaries / "surface-material-gallery"),
            cli=str(binaries / "surface-devtools"),
            out=out / "captures")
        runs[name] = run_profile(plan, name, dark, opener)
    inputs(args.config, args.config_sha256)
    report = {
        "config_sha256": args.config_sha256,
        "source_sha256": cfg["source_sha256"],
        "source_commit": cfg["source_commit"],
        "source_tree": cfg["source_tree"],
        "source_files": cfg["source_files"],
        "binary_sha256": cfg["binary_sha256"],
        "inputs_before": before, "inputs_after": before,
        "transports": TRANSPORTS,
        "profiles": [profile[0] for profile in PROFILES],
        "runs": runs,
    }
    (out / "report.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n")
    (out / "SHA256SUMS").write_text(
        "".join(f"{digest(path)}  {path.relative_to(out).as_posix()}\n"
                for path in sorted(out.rglob("*")) if path.is_file()))
    print(json.dumps(verify(out, digest(out / "SHA256SUMS"),
                            args.config_sha256), indent=2))


if __name__ == "__main__":
    main()

