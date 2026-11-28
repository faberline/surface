#!/usr/bin/env python3
"""Offline q75 readback for native and Browser receipts."""
import hashlib,json,runpy,subprocess,sys
from pathlib import Path
R=Path(__file__).resolve().parent
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def need(ok,msg):
 if not ok: raise SystemExit("READBACK FAILED: "+msg)
rows={}
for line in (R/"SHA256SUMS").read_text().splitlines():
 h,n=line.split("  ",1); need(n not in rows and not Path(n).is_absolute() and ".." not in Path(n).parts,"unsafe archive path"); rows[n]=h
for n,h in rows.items(): need((R/n).is_file() and sha(R/n)==h,"package hash differs: "+n)
actual={p.relative_to(R).as_posix() for p in R.rglob("*") if p.is_file() and p!=R/"SHA256SUMS"}
need(actual==set(rows),"package inventory differs")
summary=json.loads((R/"verified-summary.json").read_text())
need(sha(R/"source/source.sha256")=="3946a01298f055f264e759e574af373c202945bfde85a002ef02b948bf25274a" and (R/"source/target.tree").read_text().strip()=="a47ceb4426cd8bc6eb02f1792748fdced9524793","source seal differs")
need(summary["source_sha256"]=="3946a01298f055f264e759e574af373c202945bfde85a002ef02b948bf25274a" and summary["source_files"]==698 and summary["native"]["original_pngs"]==8 and summary["six_new_browser_pngs"]==6,"package metadata differs")
# Run q75's pinned native reader. It verifies all eight original PNGs and semantic geometry.
report=json.loads((R/"native-run/native-vertical-menu-report.json").read_text()); binary=report["binaries"]["manifest_sha256"]
cmd=[sys.executable,"-E","-B",str(R/"native-tools/replay-vertical-menu.py"),"--readback",str(R/"native-run"),"--source-sha","3946a01298f055f264e759e574af373c202945bfde85a002ef02b948bf25274a","--binary-sha",binary,"--evidence-sha",sha(R/"native-run/SHA256SUMS")]
n=subprocess.run(cmd,capture_output=True,text=True); need(n.returncode==0,"native image/geometry readback failed: "+(n.stderr or n.stdout))
np=json.loads(n.stdout); need(np["original_pngs"]==8 and np["visible_windows"]==0 and np["reaped"]==6,"native window or image proof differs")
# Compare before/after Browser input pins, then re-run q75's pixel and process verifier.
sys.path.insert(0,str(R/"browser-tools")); inv=json.loads((R/"browser-run/invocation.json").read_text()); after=json.loads((R/"browser-run/inputs-after.json").read_text())
need(inv["inputs"]==after and after["source_sha256"]=="3946a01298f055f264e759e574af373c202945bfde85a002ef02b948bf25274a","Browser input pins differ")
build_entries={}
for line in (R/"build/SHA256SUMS").read_text().splitlines():
 h,n=line.split("  ",1); build_entries[n.removeprefix("./")]=h
need(after["build_manifest_sha256"]==sha(R/"build/SHA256SUMS") and after["runner_sha256"]==build_entries.get("stock/debug/surface-e2e") and after["observer_sha256"]==sha(R/"browser-tools/observe-focused-original.py"),"Browser build input pins differ")
sys.path.insert(0,str(R/"browser-tools")); mod=runpy.run_path(str(R/"browser-tools/browser.py")); window=json.loads((R/"browser-run/headless-window-observation.json").read_text())
proof=mod["verify_run"](R/"browser-run",window); stored=json.loads((R/"browser-run/verified-stock-material.json").read_text())
need(json.dumps(proof,sort_keys=True)==json.dumps(stored,sort_keys=True),"Browser report differs after JSON normalization")
need(proof["steps"]==371 and sum(name.endswith(".png") for name in proof["pngs"])==6,"Browser did not verify 371 steps and six PNGs")
print("PORTABLE READBACK PASS: source/package seals, eight native PNGs, Browser process proof and six q75 PNGs")
