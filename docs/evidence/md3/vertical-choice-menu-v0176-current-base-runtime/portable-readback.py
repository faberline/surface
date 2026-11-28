#!/usr/bin/env python3
"""Read back sealed q76 source, AppKit, native and Browser evidence."""
import hashlib,json,runpy,subprocess,sys
from pathlib import Path
R=Path(__file__).resolve().parent
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def need(ok,msg):
 if not ok: raise SystemExit("READBACK FAILED: "+msg)
rows={}
for line in (R/"SHA256SUMS").read_text().splitlines():
 h,n=line.split("  ",1); need(n not in rows and not Path(n).is_absolute() and ".." not in Path(n).parts,"unsafe package path"); rows[n]=h
for n,h in rows.items(): need((R/n).is_file() and sha(R/n)==h,"package hash differs: "+n)
actual={p.relative_to(R).as_posix() for p in R.rglob("*") if p.is_file() and p!=R/"SHA256SUMS"}
need(actual==set(rows),"package inventory differs")
meta=json.loads((R/"verified-summary.json").read_text())
need(sha(R/"source/source.sha256")=="84bb4b9da77641b89a990c703bf59789f7f14657cfe97b60592cb3bbd9cfdb02" and sha(R/"source/base-source.sha256")=="8a2037d8b5fb11afcb226f5b27b41656d7c3dd428b09db9c4aaf2d711cacf2d0" and sha(R/"source/forward.patch")=="46494d3c7a2d0c18aeabde438ba186e70bebf88fb1120f0f077d1a2e45b83e0c","source/patch seals differ")
need(meta["source_tree"]=="b61630995f470772785605a944e3515a45b6d553" and meta["source_files"]==703 and meta["native_pngs"]==8 and meta["browser_pngs"]==9 and meta["browser_reports"]==3,"package metadata differs")
# Re-run the exact native geometry/pixel verifier over eight originals and transport receipts.
sys.path.insert(0,str(R/"native-tools")); nt=runpy.run_path(str(R/"native-tools/replay-vertical-choice-menu.py"))
nr=json.loads((R/"native-run/native-vertical-choice-menu-report.json").read_text()); binary=nr["binaries"]["manifest_sha256"]
cmd=[sys.executable,"-E","-B",str(R/"native-tools/replay-vertical-choice-menu.py"),"--readback",str(R/"native-run"),"--source-sha","84bb4b9da77641b89a990c703bf59789f7f14657cfe97b60592cb3bbd9cfdb02","--binary-sha",binary,"--evidence-sha",sha(R/"native-run/SHA256SUMS"),"--source-version","0.1.76"]
n=subprocess.run(cmd,capture_output=True,text=True); need(n.returncode==0,"native readback failed: "+(n.stderr or n.stdout))
np=json.loads(n.stdout); samples=2*sum(len(nt["window_phases"](p)) for p in nt["PROFILES"].values())
need(np["original_pngs"]==8 and np["window_samples"]==samples and np["visible_windows"]==0 and np["reaped"]==6,"native geometry or process/window proof differs")
# Verify the detached AppKit marker, lock and source copy.
need((R/"appkit/APPKIT-PASSED").read_text().strip()=="Detached NSView AppKit selector test exited successfully.","AppKit completion marker differs")
need(sha(R/"appkit/accesskit-macos/Cargo.lock")=="1f4fd82e49595b9727076ed73b3490571c7b470d274e02983803eaadd5bc4d12","AppKit lock hash differs")
# Verify browser before/after pins and run the exact q76 report, pixel and window verifier.
sys.path.insert(0,str(R/"replay")); inv=json.loads((R/"browser-run/invocation.json").read_text()); after=json.loads((R/"browser-run/inputs-after.json").read_text())
need(inv["inputs"]==after and after["source_sha256"]=="84bb4b9da77641b89a990c703bf59789f7f14657cfe97b60592cb3bbd9cfdb02" and after["build_manifest_sha256"]==sha(R/"build/SHA256SUMS"),"Browser input pins differ")
bentries={}
for line in (R/"build/SHA256SUMS").read_text().splitlines():
 h,name=line.split("  ",1); bentries[name.removeprefix("./")]=h
need(after["runner_sha256"]==bentries.get("stock/debug/surface-e2e") and after["observer_sha256"]==sha(R/"replay/observe-focused-original.py"),"Browser runner or observer pin differs")
browser=runpy.run_path(str(R/"replay/browser.py")); window=json.loads((R/"browser-run/headless-window-observation.json").read_text())
proof=browser["verify_run"](R/"browser-run",window); stored=json.loads((R/"browser-run/verified-stock-material.json").read_text())
need(json.dumps(proof,sort_keys=True)==json.dumps(stored,sort_keys=True),"saved Browser report differs after JSON normalization")
need(proof["steps"]==374 and len(proof["menus"])==12 and sum(n.endswith(".png") for n in proof["menus"])==9,"Browser report/image inventory differs")
print("PORTABLE READBACK PASS: q76 source, AppKit marker/lock, native eight images, Browser three reports/nine images and window proof")
