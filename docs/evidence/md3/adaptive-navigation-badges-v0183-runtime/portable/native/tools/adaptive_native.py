#!/usr/bin/env python3
"""Native headless proof for q83 adaptive navigation badges."""
import argparse
import json
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, focus, key, layout, node, settle, visible
from replay_base import descendant, inputs, pinned, sample, window

SHOW = "adaptive-navigation-badges-demo-show"
BACK = "adaptive-navigation-demo-back"
THEME = "adaptive-navigation-demo-theme"
RTL = "adaptive-navigation-demo-rtl"
LARGE = "adaptive-navigation-demo-large"
NAV = "adaptive-navigation-demo-bar"
HOME = "adaptive-navigation-demo-home"
EXPLORE = "adaptive-navigation-demo-explore"
SAVED = "adaptive-navigation-demo-saved"
UPDATES = "adaptive-navigation-demo-updates"
SETTINGS = "adaptive-navigation-demo-settings"
IDS = (HOME, EXPLORE, SAVED, UPDATES, SETTINGS)
NAMES = ("首頁", "探索路線", "收藏，有新項目", "最新消息，1,234 則未讀", "設定，有新功能")
PROFILES = (
    ("599-light", 599, 720, False),
    ("600-light", 600, 720, False),
    ("839-light", 839, 720, False),
    ("840-light", 840, 720, False),
    ("320-dark-rtl-200", 320, 720, True),
)
ACK_PROFILE = "320-dark-rtl-200"


def within(outer, inner, tolerance=.5):
    return same_box(outer, inner, tolerance)


def paint_sample(rows, bounds, color, tolerance=8):
    return sample(rows, bounds, color, tolerance=tolerance)


def indexed_bounds(frame, index):
    return frame["layout"]["nodes"][index]["bounds"]


def verify_profile(frame, path, profile):
    name, width, height, dark = profile
    dom = frame["dom"]["nodes"]
    viewport = frame["layout"]["viewport"]
    require((viewport["width"], viewport["height"]) == (width, height),
            f"{name}: viewport differs: {viewport}")
    require(node(frame, BACK)["role"] == "Button", f"{name}: Back control missing")
    for control in (THEME, RTL, LARGE):
        require(node(frame, control).get("selected", False) is dark,
                f"{name}: display control {control} differs")

    nav_node = node(frame, NAV)
    require(nav_node["role"] == "Navigation" and nav_node["name"] == "主要導覽",
            f"{name}: navigation semantics differ")
    nav = visible(frame, NAV)
    measured_nav = layout(frame, NAV)["bounds"]
    if width < 840:
        require(abs(nav["width"] - width) <= .5, f"{name}: bar width differs: {nav}")
    else:
        require(abs(nav["width"] - 96.) <= .5 and abs(nav["x"]) <= .5,
                f"{name}: leading rail geometry differs: {nav}")
    require(within(nav, measured_nav), f"{name}: navigation clip differs")

    owners = {}
    rows = read_png(path.read_bytes())[2]
    for index, stable_id in enumerate(IDS):
        item = node(frame, stable_id)
        expected_name = NAMES[index]
        require(item["role"] == "Button" and item["name"] == expected_name,
                f"{name}: semantic destination differs for {stable_id}: {item}")
        require(item.get("disabled", False) is (index == 1),
                f"{name}: disabled state differs for {stable_id}")
        require(item.get("selected", False) is (index == 0),
                f"{name}: selection differs for {stable_id}")
        bounds = visible(frame, stable_id, 48.)
        require(within(nav, bounds), f"{name}: {stable_id} is outside the navigation clip")
        owners[stable_id] = {"name": item["name"], "bounds": bounds}

    updates_index = next(i for i, item in enumerate(dom) if item.get("stable_id") == UPDATES)
    count_index = next((i for i, item in enumerate(dom) if item.get("text") == "999+"), None)
    require(count_index is not None, f"{name}: visible 999+ count is missing")
    count = dom[count_index]
    require(descendant(dom, count_index, updates_index),
            f"{name}: count text is outside the Updates owner")
    face_index = count.get("parent")
    require(face_index is not None and descendant(dom, face_index, updates_index),
            f"{name}: count face is outside the Updates owner")
    require(count["role"] != "Button" and count.get("name") != "999+",
            f"{name}: decorative count became a separate semantic control")
    require(count["name"] == "", f"{name}: decorative count acquired an accessible name")
    error = dom[face_index]["style"]["background"]
    on_error = count["style"]["foreground"]
    require(error[3] == on_error[3] == 255 and error != on_error,
            f"{name}: count face and ink tokens are invalid")
    require(max(abs(a - b) for a, b in zip(error[:3], on_error[:3])) > 56,
            f"{name}: count fill and ink are too close for independent sampling")
    face_box = indexed_bounds(frame, face_index)
    count_box = indexed_bounds(frame, count_index)
    owner_box = owners[UPDATES]["bounds"]
    require(within(owner_box, face_box) and within(owner_box, count_box),
            f"{name}: count face or text escaped its stable owner")
    face_pixels = paint_sample(rows, face_box, error)
    text_pixels = paint_sample(rows, count_box, on_error, tolerance=52)
    require(face_pixels >= 30 and text_pixels >= 4,
            f"{name}: count paint is not readable ({face_pixels}, {text_pixels})")

    dot_index = next((i for i, item in enumerate(dom)
                      if item.get("role") == "Group" and item.get("stable_id") is None
                      and item.get("style", {}).get("background") == error
                      and descendant(dom, i, next(j for j, n in enumerate(dom)
                                                     if n.get("stable_id") == SAVED))), None)
    require(dot_index is not None, f"{name}: saved dot badge is missing")
    dot_box = indexed_bounds(frame, dot_index)
    require(within(owners[SAVED]["bounds"], dot_box), f"{name}: dot escaped its owner")
    dot_pixels = paint_sample(rows, dot_box, error)
    require(dot_pixels >= 8, f"{name}: dot has too few painted pixels: {dot_pixels}")

    settings_index = next(i for i, item in enumerate(dom) if item.get("stable_id") == SETTINGS)
    label_index = next((i for i, item in enumerate(dom)
                       if item.get("text") == "新" and descendant(dom, i, settings_index)), None)
    require(label_index is not None, f"{name}: localized Settings badge is missing")
    label_face = dom[label_index].get("parent")
    require(label_face is not None and dom[label_face]["style"]["background"] == error,
            f"{name}: localized label is outside its Error badge")
    label_box = indexed_bounds(frame, label_face)
    require(within(owners[SETTINGS]["bounds"], label_box),
            f"{name}: localized label badge escaped its owner")
    label_pixels = paint_sample(rows, label_box, error)
    require(label_pixels >= 8, f"{name}: localized label has too few Error pixels")
    label_ink = dom[label_index]["style"]["foreground"]
    label_text = indexed_bounds(frame, label_index)
    label_ink_pixels = paint_sample(rows, label_text, label_ink, tolerance=52)
    require(label_ink_pixels >= 4,
            f"{name}: localized label text has too few On Error pixels")
    for stable_id in (HOME, EXPLORE):
        # Use the exact Error token, then confirm no badge-colored area appears
        # in destinations declared Hidden.
        require(paint_sample(rows, owners[stable_id]["bounds"], error) == 0,
                f"{name}: hidden badge paints in {stable_id}")
    return {"navigation": nav, "owners": owners,
            "count": {"face": face_box, "text": count_box, "error": error,
                     "on_error": on_error, "error_pixels": face_pixels,
                     "on_error_pixels": text_pixels},
            "saved_dot": {"bounds": dot_box, "error_pixels": dot_pixels},
            "settings_label": {"bounds": label_box, "error_pixels": label_pixels,
                               "text_bounds": label_text, "on_error": label_ink,
                               "on_error_pixels": label_ink_pixels}}


def clear_count(frame, before, path):
    dom = frame["dom"]["nodes"]
    require(not any(item.get("text") == "999+" for item in dom),
            "keyboard activation left 999+ in the DOM")
    updates = node(frame, UPDATES)
    require(updates["name"] == "最新消息" and updates.get("selected") is True,
            "keyboard activation did not clear count/name and select Updates")
    require(frame["interaction"]["focused"] == UPDATES,
            "keyboard activation did not retain Updates focus")
    box = visible(frame, UPDATES)
    require(box == before["owners"][UPDATES]["bounds"],
            "acknowledgement changed the stable Updates owner bounds")
    _, _, rows = read_png(path.read_bytes())
    # Compare actual pixels over the owner against the initial badge capture;
    # color is taken from the recorded initial palette.
    error = before["count"]["error"]
    pixels = paint_sample(rows, box, error)
    require(pixels == 0, f"acknowledged owner still has {pixels} Error pixels")
    return {"id": UPDATES, "name": updates["name"], "selected": True,
            "focused": UPDATES, "bounds": box, "error_pixels": pixels}


def run_profile(plan, profile):
    name, width, height, dark = profile
    samples = {}
    with launch(plan, name, width, height) as app:
        window(app, samples, "ready")
        frame = activate(app, "section-3")
        require(node(frame, SHOW)["role"] == "Button", f"{name}: badge launcher is missing")
        visible(frame, SHOW)
        frame = activate(app, SHOW)
        focus(frame, BACK)
        if dark:
            for control in (THEME, RTL, LARGE):
                frame = activate(app, control)
        require(node(frame, "adaptive-navigation-demo")["role"] == "Window",
                f"{name}: adaptive demo is not active")
        require(node(frame, HOME).get("selected") is True,
                f"{name}: Home is not the initial current page")
        shot = capture(app, plan, name, frame=frame)
        paint = verify_profile(frame, plan.out / shot["name"], profile)
        window(app, samples, "captured")
        initial_frame = frame
        ack = None
        if name == ACK_PROFILE:
            # The compact bar exposes each enabled destination as a Tab stop.
            tabs = focus_by_tab(app, UPDATES)
            focused_frame = settle(app)
            focus(focused_frame, UPDATES)
            require(node(focused_frame, HOME).get("selected") is True,
                    "moving keyboard focus committed a page")
            frame = key(app, "enter")
            require(node(frame, UPDATES).get("selected") is True,
                    "trusted keyboard Enter did not select Updates")
            ack_shot = capture(app, plan, name + "-acknowledged", frame=frame)
            ack_path = plan.out / ack_shot["name"]
            ack = {"shot": ack_shot, "frame": frame,
                   "proof": clear_count(frame, paint, ack_path),
                   "tab_trace": tabs}
            window(app, samples, "acknowledged")
        pid = app.proc.pid
    require(app.proc.returncode is not None, f"{name}: Gallery process was not reaped")
    return {"profile": name, "pid": pid, "reaped": True, "windows": samples,
            "capture": {"shot": shot, "frame": initial_frame,
                        "paint": paint}, "acknowledged": ack}


def verify(out, seal, config_sha, config):
    files = pinned(out, out / "SHA256SUMS", seal)
    payload = {path.relative_to(out).as_posix() for path in out.rglob("*")
               if path.is_file() and path != out / "SHA256SUMS"}
    require(set(files) == payload, "unexpected native evidence files")
    expected = {"report.json"}
    for repeat in (1, 2):
        for profile in PROFILES:
            base = f"repeat-{repeat}/{profile[0]}"
            expected.add(f"{base}.png")
            if profile[0] == ACK_PROFILE:
                expected.add(f"{base}-acknowledged.png")
    require(payload == expected, "native archive has a missing or extra payload")
    report = json.loads((out / "report.json").read_text())
    require(report["config_sha256"] == config_sha
            and report["source_sha256"] == config["source_sha256"]
            and report["binary_sha256"] == config["binary_sha256"]
            and report["source_commit"] == config["source_commit"]
            and report["source_tree"] == config["source_tree"]
            and report["transports"] == ["Page.screenshot", "CLI", "MCP"],
            "source, binary, or screenshot transport pins differ")
    seen = set()
    captures = 0
    window_samples = 0
    prior = {}
    for repeat in ("1", "2"):
        for profile in PROFILES:
            name = profile[0]
            run = report["runs"][repeat][name]
            phases = {"ready", "captured", "acknowledged"} if name == ACK_PROFILE else {"ready", "captured"}
            require(run["reaped"] is True and run["pid"] not in seen
                    and set(run["windows"]) == phases
                    and all(count == 0 for count in run["windows"].values()),
                    f"{repeat}/{name}: process or visible-window receipt differs")
            seen.add(run["pid"])
            window_samples += len(phases)
            capture = run["capture"]
            shot_path = out / f"repeat-{repeat}" / capture["shot"]["name"]
            require(digest(shot_path) == capture["shot"]["sha256"],
                    f"{repeat}/{name}: screenshot hash differs")
            checked = verify_profile(capture["frame"], shot_path, profile)
            require(checked == capture["paint"], f"{repeat}/{name}: pixel readback differs")
            if repeat == "2":
                first = prior[name]
                require(first[0] == capture["shot"]["sha256"] and first[1] == capture["paint"],
                        f"{name}: repeat PNG or geometry differs")
            else:
                prior[name] = (capture["shot"]["sha256"], capture["paint"])
            captures += 1
            if name == ACK_PROFILE:
                ack = run["acknowledged"]
                ack_path = out / f"repeat-{repeat}" / ack["shot"]["name"]
                require(digest(ack_path) == ack["shot"]["sha256"],
                        f"{repeat}/{name}: acknowledgement screenshot hash differs")
                check = clear_count(ack["frame"], capture["paint"], ack_path)
                require(check == ack["proof"], f"{repeat}/{name}: acknowledgement pixels differ")
                if repeat == "2":
                    first_ack = report["runs"]["1"][name]["acknowledged"]
                    require(first_ack["shot"]["sha256"] == ack["shot"]["sha256"]
                            and first_ack["proof"] == ack["proof"]
                            and first_ack["tab_trace"] == ack["tab_trace"],
                            f"{name}: acknowledged repeat differs")
                captures += 1
    require(report["inputs_before"] == report["inputs_after"],
            "source or binary input pins changed during the run")
    require((captures, len(seen), window_samples) == (12, 10, 22),
            "native capture, process, or window-sample totals differ")
    return {"captures": captures, "reaped_processes": len(seen),
            "zero_window_samples": window_samples,
            "scope": "Headless Surface DOM and PNG proof; no VoiceOver or physical display claim"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
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
    config, binary = inputs(args.config, args.config_sha256, args.check_source)
    if args.check_source or args.check_inputs:
        print("source verified" if binary is None else "source and binaries verified")
        return
    if args.verify:
        print(json.dumps(verify(args.verify, args.evidence_sha256,
                                args.config_sha256, config), indent=2))
        return
    require(args.out is not None, "--run requires --out")
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()) and not out.exists(),
            "use a new output directory under /tmp")
    out.mkdir(mode=0o700)
    before = {"source_commit": config["source_commit"],
              "source_tree": config["source_tree"],
              "source": config["source_sha256"],
              "binary": config["binary_sha256"]}
    runs = {}
    for repeat in (1, 2):
        folder = out / f"repeat-{repeat}"
        folder.mkdir()
        plan = SimpleNamespace(gallery=str(binary / "surface-material-gallery"),
                               cli=str(binary / "surface-devtools"), out=folder)
        runs[str(repeat)] = {profile[0]: run_profile(plan, profile) for profile in PROFILES}
    after_config, _ = inputs(args.config, args.config_sha256)
    after = {"source_commit": after_config["source_commit"],
             "source_tree": after_config["source_tree"],
             "source": after_config["source_sha256"],
             "binary": after_config["binary_sha256"]}
    report = {"config_sha256": args.config_sha256,
              "source_commit": config["source_commit"],
              "source_tree": config["source_tree"],
              "source_sha256": config["source_sha256"],
              "binary_sha256": config["binary_sha256"],
              "inputs_before": before, "inputs_after": after,
              "transports": ["Page.screenshot", "CLI", "MCP"], "runs": runs}
    (out / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    manifest = out / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(out).as_posix()}\n"
                                for path in sorted(out.rglob("*")) if path.is_file()))
    seal = digest(manifest)
    print(json.dumps({"seal": seal, **verify(out, seal, args.config_sha256, config)}, indent=2))


if __name__ == "__main__":
    main()
