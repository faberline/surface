#!/usr/bin/env python3
import argparse,hashlib,json,re,runpy,sys
from pathlib import Path, PurePosixPath
P=Path(__file__).resolve().parent
SOURCE='e58979727fbe6a9adc0b8ba64f9141e11bb7a32e570063f328d2449a8d034a4e'
TREE='0de7289a17faab38a240c387a8534471fb9d8906'
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def need(x,m):
 if not x: raise ValueError(m)
def data(p): return json.loads(p.read_text())
def rows(p):
 result={}
 for line in p.read_text().splitlines():
  h,n=line.split('  ',1); q=PurePosixPath(n); need(len(h)==64 and not q.is_absolute() and '..' not in q.parts and str(q)==n and n not in result,'bad manifest row'); result[n]=h
 return result
def verify():
 p=argparse.ArgumentParser(); p.add_argument('--manifest-sha'); p.add_argument('--unsealed',action='store_true'); a=p.parse_args()
 if not a.unsealed:
  need(a.manifest_sha and sha(P/'SHA256SUMS')==a.manifest_sha,'package seal mismatch'); files=rows(P/'SHA256SUMS'); actual={x.relative_to(P).as_posix() for x in P.rglob('*') if x.is_file() and x!=P/'SHA256SUMS'}; need(actual==set(files) and all(not (P/n).is_symlink() and sha(P/n)==h for n,h in files.items()),'package inventory or content mismatch')
 pins=data(P/'pins.json'); need(pins['source_sha256']==SOURCE and pins['tree']==TREE and pins['commit']=='4afa793c9332340de373c7164e200995280572be','source pins mismatch')
 manifest=P/'source/source.sha256'; need(sha(manifest)==SOURCE and len(rows(manifest))==707,'source manifest mismatch'); need(sha(P/'source/forward-from-v0177.patch')=='7381765d31afc25cb5e71d7aa5812ff5ac8c5d038774173bf5f9c6917879ff75','forward patch mismatch')
 src=P/'source'
 expected={f'{n}: OK' for n in rows(manifest)}
 for log in ['workspace-r2-manifest-before.log','workspace-r2-manifest-after.log','remote-source-readback.log']:
  values=(src/log).read_text().splitlines(); need(len(values)==707 and set(values)==expected,'source verification log mismatch: '+log)
 need((src/'workspace-r2-passed').read_text().strip()==TREE,'workspace receipt does not pin source')
 workspace=(src/'workspace-r2.log').read_text()
 suites=re.findall(r'^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;',workspace,re.M)
 need(all(status=='ok' and failed=='0' for status,_,failed,_ in suites) and sum(int(n) for _,n,_,_ in suites)==1626 and sum(int(n) for _,_,_,n in suites)==20,'workspace test result mismatch')
 wasm=workspace.rsplit('test result: ',1)[1]
 need(len(re.findall(r'^\s*Finished `dev` profile',wasm,re.M))==3,'WebAssembly checks differ')
 gpu=rows(src/'gpu-r2.SHA256SUMS')
 need(len(gpu)==23 and sum(name.endswith('.png') for name in gpu)==21 and all(sha(src/'gpu-r2'/n)==h for n,h in gpu.items()),'Metal artifact manifest mismatch')
 for name,passed in [('offscreen.log',16),('text-selection.log',2)]:
  need(f'test result: ok. {passed} passed; 0 failed; 0 ignored;' in (src/'gpu-r2'/name).read_text(),'Metal test result mismatch')
 need(sha(P/'build/SHA256SUMS')==pins['build_sha256'] and sha(P/'build/native/SHA256SUMS')==pins['native_sha256'],'build manifest mismatch')
 need((P/'build/BUILD-COMPLETE').read_text().splitlines()==[TREE,SOURCE],'build source mismatch')
 native_bins=rows(P/'build/native/SHA256SUMS'); build_bins=rows(P/'build/SHA256SUMS'); need(set(native_bins)=={'surface-material-gallery','surface-devtools'} and all(build_bins.get('native/'+n)==h for n,h in native_bins.items()),'native and build manifests disagree')
 need(data(P/'native/run/native-vertical-choice-menu-report.json')['binaries']['files']==native_bins,'native report binary hashes differ')
 tools=P/'native/tools'; need(sha(tools/'SHA256SUMS')=='75e20e6b556e458c2e9d07ae9e94393eb54129fe3ee11bf183b4fd620fa085b7','native tool seal mismatch'); toolrows=rows(tools/'SHA256SUMS'); need(all(sha(tools/n)==h for n,h in toolrows.items()),'native tool bytes mismatch')
 sys.path.insert(0,str(P/'native/tools')); native=runpy.run_path(str(P/'native/tools/replay-vertical-choice-menu.py')); nr=native['readback'](P/'native/run',pins['native_evidence_sha256'],SOURCE,pins['native_sha256'],'0.1.78'); need(nr['original_pngs']==8,'native capture count mismatch')
 cfg=P/'browser/config.json'; need(sha(cfg)==pins['browser_config_sha256'],'browser config mismatch'); c=data(cfg); need(c['source_sha256']==SOURCE and c['source_tree']==TREE and c['build_sha256']==pins['build_sha256'],'browser source/build mismatch')
 br=P/'browser/run'; inv=data(br/'invocation.json'); after=data(br/'inputs-after.json'); need(inv['inputs']==after and after['config_sha256']==pins['browser_config_sha256'],'browser input receipts differ')
 sys.path.insert(0,str(P/'browser')); browser=runpy.run_path(str(P/'browser/browser.py')); result=browser['verify_run'](br,data(br/'headless-window-observation.json')); need(json.loads(json.dumps(result))==data(br/'verified-stock-material.json') and result['steps']==379,'browser result mismatch')
 # Historical R1 runs are diagnostic only; they cannot stand in for the R2 stock run.
 failed=P/'history/r1-stock-failure'; failure=(failed/'runner.log').read_text(); fw=data(failed/'headless-window-observation.json')
 need('last step: M380' in failure and '31154 checks' in failure and 'menu-demo-choice-show' in failure and fw['stock_exit']==1 and not fw['problems'] and not fw['visible_owned_windows'] and not fw['remaining_observed_processes'],'R1 negative history differs')
 pilot=P/'history/r1-focused-pilot'; log=(pilot/'runner.log').read_text(); pw=data(pilot/'headless-window-observation.json'); pinv=data(pilot/'invocation.json'); pa=data(pilot/'inputs-after.json')
 need('1 steps, 127 checks' in log and pw['stock_exit']==0 and not pw['problems'] and not pw['visible_owned_windows'] and not pw['remaining_observed_processes'] and pinv['inputs']==pa and pa['scope']=='focused diagnostic only','pilot history differs')
 need(sha(pilot/'route-only.diff')==pa['route_patch_sha256'] and sha(pilot/'pilot-runner.diff')==pa['pilot_patch_sha256'],'pilot patch hashes differ')
 pilot_pixels=browser['verify_forced_menu'](pilot/'artifacts')
 print(json.dumps({'r1_history':'failure and focused pilot only','focused_pilot_images':len(pilot_pixels['images']),'source_manifest_rows' :707,'source_reconstructed':False,'rust_passed':1626,'rust_ignored':20,'wasm_checks':3,'metal_passed':18,'metal_original_pngs':21,'native':nr,'browser':result},indent=2))
if __name__=='__main__': verify()
