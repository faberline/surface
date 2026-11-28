#!/usr/bin/env python3
"""Offline q73 package inventory and navigation screenshot pixel check."""
import hashlib,json,runpy,sys
from pathlib import Path
R=Path(__file__).resolve().parent
sys.path.insert(0,str(R/"replay"))
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def need(ok,msg):
 if not ok: raise SystemExit("READBACK FAILED: "+msg)
rows={}
for line in (R/"SHA256SUMS").read_text().splitlines():
 h,n=line.split("  ",1); need(n not in rows and not Path(n).is_absolute() and ".." not in Path(n).parts,"unsafe path"); rows[n]=h
for n,h in rows.items(): need((R/n).is_file() and sha(R/n)==h,"package hash mismatch "+n)
actual={p.relative_to(R).as_posix() for p in R.rglob("*") if p.is_file() and p!=R/"SHA256SUMS"}
need(actual==set(rows),"package inventory differs")
native_decoder=runpy.run_path(str(R/"replay/replay-horizontal-nav.py"))
native=native_decoder["readback"](R/"native",sha(R/"native/SHA256SUMS"),"513d366accbb3ef9cd39120dc0a45bd98d033e0216c56613079011d0bf8e94dd",
                                  sha(R/"build/native/SHA256SUMS"))
need(native["original_pngs"]==4 and native["visible_windows"]==0 and native["reaped"]==4,
     "native screenshot or window receipt differs")
# Run the exact q73 verifier's geometry and pixel decoder without its runtime/window checks.
ns=runpy.run_path(str(R/"replay/verify-navigation-pair.py"))
art=R/"browser/artifacts"; compact=json.loads((art/"browser-material-flexible-navigation-bar.json").read_text())
result=ns["medium"](art,compact)
need(len(result["pngs"])==3 and result["interaction"]["widths"]==[839,600,599,600],"navigation evidence differs")
receipt=json.loads((R/"browser/verified-stock-material.json").read_text())
need(receipt["steps"]==364 and receipt["horizontal"]==json.loads(json.dumps(result)),"full Browser receipt metadata differs")
print("PORTABLE READBACK PASS: package inventory, four native PNGs and three Browser PNGs")
