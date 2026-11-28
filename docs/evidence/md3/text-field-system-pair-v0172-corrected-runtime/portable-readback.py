#!/usr/bin/env python3
"""Blind, offline q72 receipt readback. Needs Pillow for PNG decoding."""
import hashlib, json, runpy, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parent
SOURCE_SHA = "a59fd437cbd2f3cea304708ff2e95752fd70e8f1ec1fe2004a1289903d6a6f49"
BINARY_SHA = "98c18044551a1baada3b8d75076707e548c7c25491cb5fc2a26dce7f77410d02"

def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def need(ok, message):
    if not ok: raise SystemExit("READBACK FAILED: " + message)

# Verify the complete archive inventory without trusting a machine-specific path.
rows = {}
for line in (ROOT / "SHA256SUMS").read_text().splitlines():
    value, name = line.split("  ", 1)
    need(name not in rows and not Path(name).is_absolute() and ".." not in Path(name).parts,
         "unsafe or duplicate path")
    rows[name] = value
for name, value in rows.items():
    p = ROOT / name
    need(p.is_file() and sha(p) == value, "package hash differs: " + name)
actual = {p.relative_to(ROOT).as_posix() for p in ROOT.rglob("*") if p.is_file() and p != ROOT / "SHA256SUMS"}
need(actual == set(rows), "package inventory differs")

# Rebind the native verifier's helper directory in memory; preserve the original tool bytes.
native_path = ROOT / "replay/replay-field-pair.py"
text = native_path.read_text()
old = 'BASE = Path("/tmp/surface-md3-v0172-replay.OgmvVGia/native-helpers")'
new = 'BASE = Path(__file__).resolve().parent / "native-helpers"'
need(text.count(old) == 1, "native helper path in the pinned verifier changed")
sys.path.insert(0, str(ROOT / "replay"))
ns = {"__name__": "_portable_q72_native", "__file__": str(native_path)}
exec(compile(text.replace(old, new, 1), str(native_path), "exec"), ns)
native = ns["readback"](ROOT / "native", sha(ROOT / "native/SHA256SUMS"))
need(native["source"] == SOURCE_SHA and native["binaries"] == BINARY_SHA
     and native["original_pngs"] == 16 and native["window_samples"] == 44
     and native["visible_windows"] == 0 and native["reaped"] == 4,
     "native readback counts differ")

# Verify only q72's 14 images. The full receipt keeps q70/q71 image hashes without duplicating those files.
verify_q72 = runpy.run_path(str(ROOT / "replay/verify-system-pair.py"))["verify_captures"]
actual_q72 = verify_q72(ROOT / "browser/artifacts")
stored = json.loads((ROOT / "browser/verified-stock-material.json").read_text())
need(actual_q72 == stored["system_pair"] and len(actual_q72["png_sha256"]) == 14,
     "q72 typed report or original PNG pixel proof differs")
need(stored["steps"] == 361 and len(stored["boundary_pngs"]) == 33
     and len(stored["disabled_contrast"]["png_sha256"]) == 15,
     "full Browser receipt metadata differs")
print("PORTABLE READBACK PASS: package hashes, 16 native PNGs, and 14 q72 Browser PNGs")
