#!/usr/bin/env python3
"""Verify q74 package inventory, native geometry, browser proof and pixels offline."""
import hashlib,json,runpy,subprocess,sys
from pathlib import Path
R=Path(__file__).resolve().parent
def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def need(ok,msg):
 if not ok: raise SystemExit("READBACK FAILED: "+msg)
rows={}
for line in (R/"SHA256SUMS").read_text().splitlines():
 h,n=line.split("  ",1); need(n not in rows and not Path(n).is_absolute() and ".." not in Path(n).parts,"unsafe package path"); rows[n]=h
for n,h in rows.items(): need((R/n).is_file() and digest(R/n)==h,"package file hash differs: "+n)
actual={p.relative_to(R).as_posix() for p in R.rglob("*") if p.is_file() and p!=R/"SHA256SUMS"}
need(actual==set(rows),"package file inventory differs")
# Native readback runs the exact pinned q74 verifier and decodes all six original images.
native_report=json.loads((R/"native-run/native-adaptive-nav-report.json").read_text())
binary_sha=native_report["binaries"]["manifest_sha256"]
cmd=[sys.executable,"-E","-B",str(R/"native-tools/replay-adaptive-nav.py"),"--readback",str(R/"native-run"),"--source-sha","53fb1fd46ab0b360186238bc3a946862c16047646e4578fc25dff9d308811f8f","--binary-sha",binary_sha,"--evidence-sha",digest(R/"native-run/SHA256SUMS")]
n=subprocess.run(cmd,capture_output=True,text=True); need(n.returncode==0,"native geometry/pixel readback failed: "+(n.stderr or n.stdout))
np=json.loads(n.stdout); need(np["original_pngs"]==6 and np["window_samples"]==30 and np["visible_windows"]==0 and np["reaped"]==6,"native receipt counts or window proof differ")
# Check that the saved Browser run used matching before/after input pins and decode the four q74 PNGs.
sys.path.insert(0,str(R/"browser-tools"))
inv=json.loads((R/"browser-run/invocation.json").read_text()); after=json.loads((R/"browser-run/inputs-after.json").read_text())
need(inv["inputs"]==after and after["source_sha256"]=="53fb1fd46ab0b360186238bc3a946862c16047646e4578fc25dff9d308811f8f","Browser input pins differ")
build_sha=digest(R/"build/SHA256SUMS")
need(after["build_manifest_sha256"]==build_sha and after["observer_sha256"]==digest(R/"browser-tools/observe-focused-original.py"),"Browser build or observer pin differs")
records={}
for line in (R/"build/SHA256SUMS").read_text().splitlines():
 h,n=line.split("  ",1); records[n.removeprefix("./")]=h
need(after["runner_sha256"]==records.get("stock/debug/surface-e2e"),"Browser runner hash differs from saved build manifest")
browser=runpy.run_path(str(R/"browser-tools/browser.py"))
window=json.loads((R/"browser-run/headless-window-observation.json").read_text())
proof=browser["verify_run"](R/"browser-run",window)
stored=json.loads((R/"browser-run/verified-stock-material.json").read_text())
need(json.dumps(proof,sort_keys=True)==json.dumps(stored,sort_keys=True),"stored Browser proof differs")
need(proof["steps"]==368 and len(proof["adaptive_navigation"]["pngs"])==4,"q74 Browser step/image count differs")
print("PORTABLE READBACK PASS: package hashes, six native originals and geometry, Browser window proof and four q74 PNGs")
