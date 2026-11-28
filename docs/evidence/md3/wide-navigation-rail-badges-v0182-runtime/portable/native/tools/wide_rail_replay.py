"""Headless Native Gallery evidence for application-owned wide rail badges."""
import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, focus, key, node, visible
from replay import descendant, inputs, pinned, sample, window

BASE = "wide-navigation-badges-demo"
IDS = tuple(f"{BASE}-{suffix}" for suffix in ("home", "inbox", "updates", "explore", "settings"))
NAMES = ("首頁", "收件匣，1,234 則未讀", "最新消息，有新內容", "探索路線，有新內容", "設定，無法使用")
INBOX = IDS[1]
INBOX_NAME = "收件匣"
PROFILES = (
    ("collapsed-light-840", 840, 720, False, False),
    ("expanded-light-840", 840, 720, True, False),
    ("collapsed-dark-rtl-200-320", 320, 880, False, True),
    ("expanded-dark-rtl-200-420", 420, 720, True, True),
)
ACK_PROFILE = PROFILES[-1][0]


def inside(parent, child):
    return same_box(parent, child)


def artwork(frame, index, owner_bounds, rows, label):
    nodes = frame["dom"]["nodes"]
    layouts = frame["layout"]["nodes"]
    require(inside(owner_bounds, layouts[index]["bounds"]), f"{label} bounds outside owner")
    ink = nodes[index]["style"]["foreground"]
    count = sample(rows, layouts[index]["bounds"], ink, tolerance=30)
    require(count >= 10, f"Label {label} has too little visible ink")
    return {"bounds": layouts[index]["bounds"], "ink": ink, "ink_pixels": count}


def analyze(frame, path, profile, acknowledged=False):
    _, width, height, expanded, dark = profile
    nodes = frame["dom"]["nodes"]
    layouts = frame["layout"]["nodes"]
    viewport = frame["layout"]["viewport"]
    require((viewport["width"], viewport["height"]) == (width, height), "Viewport differs")
    require(node(frame, BASE + "-rail")["role"] == "Navigation", "Rail landmark missing")
    rail = visible(frame, BASE + "-rail")
    width_ok = 219.5 <= rail["width"] <= 360.5 if expanded else abs(rail["width"] - 96) <= .5
    body = visible(frame, BASE + "-layout")
    expected_x = body["x"] + body["width"] - rail["width"] if dark else body["x"]
    require(width_ok and abs(expected_x - rail["x"]) <= .5, f"Rail measures incorrectly: {rail}")
    visible(frame, BASE + "-title")

    count_index = next((i for i, n in enumerate(nodes) if n.get("text") == "999+"), None)
    if acknowledged:
        require(count_index is None, "Acknowledged count is still in DOM")
        count_index = next(i for i, n in enumerate(nodes) if n.get("text") == "新")
    require(count_index is not None, "Count or localized label is missing")
    face_index = nodes[count_index]["parent"]
    error = nodes[face_index]["style"]["background"]
    w, h, rows = read_png(path.read_bytes())
    require((w, h) == (width, height), "PNG dimensions differ")

    owners = {}
    for i, stable_id in enumerate(IDS):
        target = node(frame, stable_id)
        name = INBOX_NAME if i == 1 and acknowledged else NAMES[i]
        require(target["role"] == "Button" and target["name"] == name, f"Wrong semantics for {stable_id}")
        require(target.get("disabled", False) == (i == 4), f"Wrong disabled state for {stable_id}")
        bounds = visible(frame, stable_id, 48)
        require(inside(rail, bounds), f"{stable_id} lies outside its rail")
        count = sample(rows, bounds, error)
        hidden = i in (0, 4) or (i == 1 and acknowledged)
        require(count == 0 if hidden else count >= (8 if i == 2 else 30),
                f"Badge face for {stable_id} has {count} pixels")
        owners[stable_id] = {"name": name, "bounds": bounds, "error_pixels": count}

    owner_index = next(i for i, n in enumerate(nodes) if n.get("stable_id") == INBOX)
    icon_index = next(i for i, n in enumerate(nodes)
                      if n.get("fill_contour") and descendant(nodes, i, owner_index))
    contour = nodes[icon_index]["fill_contour"]
    icon_bounds = layouts[icon_index]["bounds"]
    require(inside(owners[INBOX]["bounds"], icon_bounds), "Inbox icon outside its owner")
    icon_pixels = sample(rows, icon_bounds, contour["color"], tolerance=18)
    minimum = 95 if profile[0] == "collapsed-dark-rtl-200-320" else 10
    require(icon_pixels >= minimum, f"Inbox icon has {icon_pixels} visible pixels")
    owners[INBOX]["icon"] = {"bounds": icon_bounds, "ink": contour["color"], "ink_pixels": icon_pixels}
    badge_bounds = layouts[face_index]["bounds"]
    on_error = nodes[count_index]["style"]["foreground"]
    ink_pixels = sample(rows, layouts[count_index]["bounds"], on_error, tolerance=52, face=badge_bounds)
    require(ink_pixels >= 4, f"The readable badge text has {ink_pixels} pixels")
    if expanded:
        label_index = next(i for i, n in enumerate(nodes) if n.get("text") == INBOX_NAME
                           and descendant(nodes, i, owner_index))
        owners[INBOX]["text"] = artwork(frame, label_index, owners[INBOX]["bounds"], rows, INBOX_NAME)
    if acknowledged:
        require(node(frame, INBOX).get("selected") is True, "Acknowledged Inbox lost selection")
        focus(frame, INBOX)
    return {"rail": rail, "owners": owners, "error": error,
            "badge_text": {"bounds": layouts[count_index]["bounds"], "ink": on_error, "pixels": ink_pixels}}


def snap(app, plan, name, frame, profile, acknowledged=False):
    shot = capture(app, plan, name, frame=frame)
    return {"shot": shot, "frame": frame,
            "paint": analyze(frame, plan.out / shot["name"], profile, acknowledged)}


def run_profile(plan, profile):
    name, width, height, expanded, dark = profile
    samples = {}
    traces = {}
    with launch(plan, name, width, height) as app:
        window(app, samples, "ready")
        activate(app, "section-1")
        frame = activate(app, BASE + "-show")
        focus(frame, BASE + "-back")
        if dark:
            for suffix in ("theme", "rtl", "large-text"):
                frame = activate(app, BASE + "-" + suffix)
        if expanded:
            frame = activate(app, BASE + "-expand")
        captures = {"initial": snap(app, plan, name, frame, profile)}
        window(app, samples, "captured")
        if name == ACK_PROFILE:
            traces["tab"] = focus_by_tab(app, IDS[0])
            frame = key(app, "down")
            focus(frame, INBOX)
            require(node(frame, IDS[0]).get("selected") is True, "Arrow committed a page")
            frame = key(app, "enter")
            captures["acknowledged"] = snap(app, plan, name + "-acknowledged", frame, profile, True)
            before = captures["initial"]["paint"]["owners"][INBOX]["bounds"]
            after = captures["acknowledged"]["paint"]["owners"][INBOX]["bounds"]
            require(before == after, "Acknowledgement moved the hit target")
            window(app, samples, "acknowledged")
        pid = app.proc.pid
    require(app.proc.returncode is not None, "The headless child must be reaped")
    return {"pid": pid, "reaped": True, "windows": samples, "captures": captures, "keys": traces}


def verify(root, seal, cfg_sha, cfg):
    files = pinned(root, root / "SHA256SUMS", seal)
    payload = {p.relative_to(root).as_posix() for p in root.rglob("*")
               if p.is_file() and p != root / "SHA256SUMS"}
    require(set(files) == payload, "Unexpected evidence files")
    report = json.loads((root / "report.json").read_text())
    require(report["config_sha"] == cfg_sha and report["source_sha"] == cfg["source_sha256"]
            and report["binary_sha"] == cfg["binary_sha256"]
            and report["transports"] == ["Page.screenshot", "CLI", "MCP"], "Wrong evidence inputs")
    count = 0
    samples = 0
    children = set()
    for profile in PROFILES:
        name = profile[0]
        initial = {}
        for repeat in ("1", "2"):
            run = report["runs"][repeat][name]
            phases = {"initial", "acknowledged"} if name == ACK_PROFILE else {"initial"}
            require(set(run["captures"]) == phases, "Missing capture")
            checks = {"ready", "captured", "acknowledged"} if name == ACK_PROFILE else {"ready", "captured"}
            require(run["reaped"] and run["pid"] not in children and set(run["windows"]) == checks
                    and all(observed == 0 for observed in run["windows"].values()), "Headless conditions differ")
            children.add(run["pid"])
            samples += len(checks)
            for phase, received in run["captures"].items():
                path = root / ("repeat-" + repeat) / received["shot"]["name"]
                sha = digest(path)
                require(received["shot"]["sha256"] == sha, "Capture hash differs")
                paint = analyze(received["frame"], path, profile, phase == "acknowledged")
                require(received["paint"] == paint, "Capture paint differs independently")
                if repeat == "1":
                    initial[phase] = (sha, paint)
                else:
                    require(initial[phase] == (sha, paint), "PNG replay differs")
                count += 1
    return {"captures": count, "reaped": len(children), "zero_window_samples": samples}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--config-sha", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--verify-sha")
    args = parser.parse_args()
    config, bins = inputs(args.config, args.config_sha)
    out = args.out.resolve()
    if args.verify_sha:
        print(json.dumps(verify(out, args.verify_sha, args.config_sha, config)))
        return
    require(out.is_relative_to(Path("/tmp").resolve()), "Use a new output under /tmp")
    out.mkdir(mode=0o700)
    runs = {}
    for repeat in ("1", "2"):
        folder = out / ("repeat-" + repeat)
        folder.mkdir()
        plan = SimpleNamespace(gallery=str(bins / "surface-material-gallery"),
                               cli=str(bins / "surface-devtools"), out=folder)
        runs[repeat] = {p[0]: run_profile(plan, p) for p in PROFILES}
    inputs(args.config, args.config_sha)
    report = {"config_sha": args.config_sha, "source_sha": config["source_sha256"],
              "binary_sha": config["binary_sha256"],
              "transports": ["Page.screenshot", "CLI", "MCP"], "runs": runs}
    (out / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    manifest = out / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(p)}  {p.relative_to(out).as_posix()}\n"
                                for p in sorted(out.rglob("*")) if p.is_file()))
    seal = digest(manifest)
    print(json.dumps({"seal": seal, **verify(out, seal, args.config_sha, config)}))


if __name__ == "__main__":
    main()
