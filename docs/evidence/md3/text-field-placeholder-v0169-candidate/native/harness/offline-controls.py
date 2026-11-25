#!/usr/bin/env python3
"""Run synthetic proof and input-pin controls. Never launch the Gallery."""

from copy import deepcopy
import json
from pathlib import Path
from runpy import run_path
import sys
from tempfile import TemporaryDirectory
from types import SimpleNamespace

from PIL import Image, ImageDraw

from gallery_transport import require
from native_runtime import digest

HERE = Path(__file__).resolve().parent
VERIFY = run_path(str(HERE / "verify-stored-placeholder.py"))
SOURCE = VERIFY["SOURCE"]


def reject(message, operation):
    try:
        operation()
    except (ValueError, RuntimeError) as error:
        require(message in str(error), f"Unexpected negative result: {error}")
    else:
        raise RuntimeError(f"Accepted negative control: {message}")


def seal(root):
    files = sorted(path for path in root.rglob("*") if path.is_file() and path.name != "SHA256SUMS")
    target = root / "SHA256SUMS"
    target.write_text("".join(f"{digest(file)}  {file.relative_to(root).as_posix()}\n" for file in files))
    return digest(target)


def capture(root, scope, appearance, stage):
    wide = scope == "wide"
    filled = appearance == "filled"
    dark = [20, 18, 24, 255]
    light = [253, 247, 255, 255]
    background = light if wide else dark
    paper = ([229, 223, 236, 255] if wide else [52, 48, 58, 255]) if filled else background
    face = {"x": 16, "y": 280, "width": 288, "height": 56 if wide else 96}
    line = {"x": 32, "y": 304 if filled and wide else 320 if filled else 296 if wide else 304,
            "width": 256, "height": 24 if wide else 48}
    above = None if wide else {"x": 32, "y": 208, "width": 256, "height": 64}
    value = SOURCE["SEED"] if stage in ("above-seeded", "above-seeded-nohint") else ""
    focus = (SOURCE["EDITOR"] if stage in ("focused", "focused-nohint")
             else SOURCE["SET"] if value else SOURCE["CLEAR"])
    width, height = SOURCE["SCOPES"][scope]
    image = Image.new("RGBA", (width, height), tuple(background))
    ink = ImageDraw.Draw(image)
    ink.rectangle((16, 280, 303, 280 + face["height"] - 1), fill=tuple(paper))
    if above:
        ink.rectangle((220, 224, 265, 241), fill=(210, 207, 216, 255))
    if (wide and stage == "focused") or (not wide and stage in ("above-idle", "above-clear")) or value:
        x = 34 if wide else 252
        y = line["y"] + 2
        ink.rectangle((x, y, x + 30, y + (9 if wide else 18)), fill=(65, 52, 75, 255) if wide else (224, 220, 232, 255))
    name = f"{scope}-{appearance}-{stage}.png"
    path = root / name
    image.save(path)
    return {"name": name, "sha256": digest(path), "width": width, "height": height, "scale": 1, "format": "png",
            "layout": {"face": face, "line": line, "above": above, "text_scale": 1 if wide else 2,
                       "direction": "LTR" if wide else "RTL", "paper": paper, "background": background},
            "semantic": {"editor": SOURCE["EDITOR"], "label": SOURCE["SHORT_LABEL"] if wide else SOURCE["LONG_LABEL"],
                         "role": "TextInput", "value": value, "display_value": value, "focus": focus,
                         "selection_utf8": [0, 0], "disabled": False, "hint_in_semantic_nodes": False}}


def synthetic_replay(root):
    root.mkdir()
    repeats, processes = [], []
    for index in (1, 2):
        directory = root / f"replay-{index}"
        directory.mkdir()
        scopes, samples = {}, {}
        for ordinal, (scope, viewport) in enumerate(SOURCE["SCOPES"].items()):
            appearances = {}
            for appearance in SOURCE["APPEARANCES"]:
                captures = {stage: capture(directory, scope, appearance, stage)
                            for stage in SOURCE["ENDPOINTS"][scope]}
                appearances[appearance] = {
                    "captures": captures,
                    "pixel_checks": SOURCE["verify_one_appearance"](directory, captures, scope, appearance),
                }
            scopes[scope] = {"viewport": list(viewport), "appearances": appearances,
                             "restored_focus": SOURCE["SHOW"]}
            samples[scope] = {"pid": 10 * index + ordinal, "reaped": True,
                              "visible_windows": {name: 0 for name in sorted(SOURCE["phases"](scope))}}
        repeats.append(scopes)
        processes.append(samples)
    src, exe = "ab" * 32, "cd" * 32
    raw = {"schema": SOURCE["SCHEMA"], "scope": SOURCE["SCOPE"], "source": {
        "manifest_sha256": src, "version": "0.1.69", "files": 687}, "binary_manifest_sha256": exe,
        "binaries": {"surface-material-gallery": "cc" * 32, "surface-devtools": "ee" * 32},
        "transports": SOURCE["TRANSPORTS"], "replays": repeats, "processes": processes}
    report = root / SOURCE["REPORT"]
    report.write_text(json.dumps(raw))
    result = VERIFY["verify"](root, seal(root), src, exe)
    require((result["original_pngs"], result["observed_window_samples"]) == (40, 92),
            "Wrong synthetic output counts")
    print("Synthetic transport, image and focus receipt passes both stored replays.")

    changed = deepcopy(raw)
    changed["processes"][1]["compact"]["visible_windows"]["closed"] = 1
    report.write_text(json.dumps(changed))
    reject("zero-window observations", lambda: VERIFY["verify"](root, seal(root), src, exe))
    changed = deepcopy(raw)
    changed["replays"][1]["wide"]["restored_focus"] = "wrong"
    report.write_text(json.dumps(changed))
    reject("replay bytes differ", lambda: VERIFY["verify"](root, seal(root), src, exe))
    report.write_text(json.dumps(raw))
    failure = root / "replay-1/compact-filled-above-idle.png"
    failure.write_bytes((root / "replay-1/compact-filled-above-idle-nohint.png").read_bytes())
    reject("Wrong saved original PNG", lambda: VERIFY["verify"](root, seal(root), src, exe))
    print("Visible window, changed focus and substituted screenshot rejected.")


def input_preflight(source_manifest, scratch):
    # These stand-ins exercise manifest validation only. They cannot launch.
    bundle = scratch / "inert-binary-pin"
    bundle.mkdir()
    for name in ("surface-material-gallery", "surface-devtools"):
        path = bundle / name
        path.write_text("inert fingerprint for --verify-inputs-only\n")
        path.chmod(0o700)
    binary_digest = seal(bundle)
    manifest = source_manifest.resolve(strict=True)
    request = SimpleNamespace(source_root=manifest.parent / "source", source_manifest=manifest,
                              source_manifest_sha=digest(manifest), binary_manifest=bundle / "SHA256SUMS",
                              binary_manifest_sha=binary_digest)
    _, _, receipt = SOURCE["checked_inputs"](request)
    require(receipt["source"]["version"] == "0.1.69", "Wrong actual source")
    request.binary_manifest_sha = "00" * 32
    reject("Reviewed manifest digest differs", lambda: SOURCE["checked_inputs"](request))
    print(f"Actual {receipt['source']['files']}-path source seal verified. Inert binary pin control passed.")


if __name__ == "__main__":
    with TemporaryDirectory(prefix="placeholder-control-", dir=HERE) as temp:
        root = Path(temp)
        input_preflight(Path(sys.argv[1]), root)
        synthetic_replay(root / "proof")
