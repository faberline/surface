#!/usr/bin/env python3
"""Portable offline readback of a Surface v0.1.71 evidence folder."""
import hashlib, json, re, runpy, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SOURCE_SHA = "7ff2289093941bab73f8f718492d800b8a9c2bd33c3a03aae0f2396b6d03a65e"

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def need(ok, message):
    if not ok: raise SystemExit("READBACK FAILED: " + message)

seal = ROOT / "SHA256SUMS"
rows = {}
for line in seal.read_text().splitlines():
    value, name = line.split("  ", 1)
    need(name not in rows and not Path(name).is_absolute() and ".." not in Path(name).parts,
         "unsafe or duplicate package path")
    rows[name] = value
for name, value in rows.items():
    path = ROOT / name
    need(path.is_file() and sha(path) == value, "package file hash differs: " + name)
actual = {p.relative_to(ROOT).as_posix() for p in ROOT.rglob("*") if p.is_file() and p != seal}
need(actual == set(rows), "package inventory differs from SHA256SUMS")

# Run the exact native verifier with only its fixed helper root rebound to this folder.
native_source = ROOT / "replay/replay-disabled-contrast.py"
code = native_source.read_text()
old = 'BASE = Path("/tmp/surface-md3-v0171-replay.CZtAvGj9/native-helpers")'
new = 'BASE = Path(__file__).resolve().parent / "native-helpers"'
need(code.count(old) == 1, "native verifier path binding changed")
portable = code.replace(old, new, 1)
sys.path.insert(0, str(ROOT / "replay"))
ns = {"__name__": "_portable_native_readback", "__file__": str(native_source)}
exec(compile(portable, str(native_source), "exec"), ns)
report = json.loads((ROOT / "native/native-text-field-disabled-contrast-report.json").read_text())
native = ns["verify_stored"](ROOT / "native", SOURCE_SHA, report["binary_manifest_sha256"],
                              sha(ROOT / "native/SHA256SUMS"))
need(native["original_pngs"] == 24 and native["repeats"] == 2 and native["visible_owned_windows"] == 0
     and native["observed_window_samples"] == 78 and native["reaped_gallery_processes"] == 6,
     "native offline readback counts differ")

# Browser verifier checks both the 33 boundary PNGs and 15 contrast PNGs pixel by pixel.
browser_source = ROOT / "replay/browser.py"
browser_ns = runpy.run_path(str(browser_source))
run = ROOT / "browser"
window = json.loads((run / "headless-window-observation.json").read_text())
computed = browser_ns["verify_run"](run, window)
stored = json.loads((run / "verified-stock-material.json").read_text())
need(computed == stored, "stored Browser receipt differs from blind pixel verification")
need(stored["steps"] == 358 and len(stored["boundary_pngs"]) == 33
     and len(stored["disabled_contrast"]["png_sha256"]) == 15,
     "Browser boundary/contrast image inventory differs")
print(
    f"PORTABLE READBACK PASS: package hashes, {native['original_pngs']} native PNGs, "
    f"{len(stored['boundary_pngs'])} boundary PNGs, "
    f"{len(stored['disabled_contrast']['png_sha256'])} contrast PNGs"
)
