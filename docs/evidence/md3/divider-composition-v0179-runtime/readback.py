#!/usr/bin/env python3
import argparse,hashlib,json,re,runpy,sys
from pathlib import Path,PurePosixPath
P=Path(__file__).resolve().parent
SOURCE='4ff1e8e17f1eb66d230ad43a8f3226bb126c31eea79791bda765f62c90b2b979';TREE='c2470303ad4a11e356f93f6f66d361d8d78e13d5';PATCH='6cb52152e3cdb66edc94e0213a828d6fb17b55d8c7b486a7442111b8512bd836'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def need(x,m):
 if not x:raise ValueError(m)
def data(p):return json.loads(p.read_text())
def rows(p):
 d={}
 for line in p.read_text().splitlines():
  h,n=line.split('  ',1);q=PurePosixPath(n);need(re.fullmatch('[0-9a-f]{64}',h) and not q.is_absolute() and '..' not in q.parts and str(q)==n and n not in d,'invalid manifest');d[n]=h
 return d
def main():
 p=argparse.ArgumentParser();p.add_argument('--manifest-sha');p.add_argument('--unsealed',action='store_true');a=p.parse_args()
 if not a.unsealed:
  need(a.manifest_sha and sha(P/'SHA256SUMS')==a.manifest_sha,'package seal mismatch');r=rows(P/'SHA256SUMS');actual={x.relative_to(P).as_posix() for x in P.rglob('*') if x.is_file() and x!=P/'SHA256SUMS'};need(actual==set(r) and all(not (P/n).is_symlink() and sha(P/n)==h for n,h in r.items()),'package inventory mismatch')
 pins=data(P/'pins.json');need(pins['commit']=='2c5003c521f518ed80c59035f2b74e86cf97ea73' and pins['tree']==TREE and pins['source_sha256']==SOURCE and pins['patch_sha256']==PATCH,'wrong R2 pins')
 src=P/'source';manifest=src/'source.sha256';need(sha(manifest)==SOURCE and len(rows(manifest))==708 and sha(src/'forward-from-v0178-r2.patch')==PATCH,'source inventory or patch mismatch');expected={f'{n}: OK' for n in rows(manifest)}
 for name in ('workspace-r2-manifest-before.log','workspace-r2-manifest-after.log','reconstructed-source.log'):
  lines=(src/name).read_text().splitlines();need(len(lines)==708 and set(lines)==expected,'source log mismatch '+name)
 need((src/'workspace-r2-passed').read_text().strip()==TREE,'workspace tree receipt mismatch');text=(src/'workspace-r2.log').read_text();tests=re.findall(r'^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;',text,re.M);need(tests and all(a=='ok' and f=='0' for a,_,f,_ in tests),'workspace failed');wasm=len(re.findall(r'^\s*Finished `dev` profile',text.rsplit('test result: ',1)[1],re.M));need(wasm>=3,'missing wasm checks')
 gpu=rows(src/'gpu-r2.SHA256SUMS');need(gpu and all(sha(src/'gpu-r2'/n)==h for n,h in gpu.items()),'GPU manifest mismatch');gpu_tests=[]
 for n in gpu:
  if n.endswith('.log'):gpu_tests+=re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;', (src/'gpu-r2'/n).read_text())
 need(gpu_tests and all(s=='ok' and int(p)>0 and f=='0' and i=='0' for s,p,f,i in gpu_tests),'GPU tests failed or skipped')
 need(sha(P/'build/SHA256SUMS')==pins['build_sha256'] and sha(P/'build/native/SHA256SUMS')==pins['native_sha256'] and (P/'build/BUILD-COMPLETE').read_text().splitlines()==[TREE,SOURCE],'build pins mismatch')
 native_bins=rows(P/'build/native/SHA256SUMS');build_bins=rows(P/'build/SHA256SUMS');need(all(build_bins.get('native/'+n)==h for n,h in native_bins.items()),'build binary mismatch');np=P/'native/pins.json';need(sha(np)==pins['native_pins_sha256'],'native config mismatch');nc=data(np);need(nc['source_sha256']==SOURCE and nc['source_tree']==TREE and nc['binary_sha256']==pins['native_sha256'],'native config pins mismatch')
 tools=P/'native/tools';tr=rows(tools/'SHA256SUMS');need(all(sha(tools/n)==h for n,h in tr.items()),'native helpers mismatch');sys.path.insert(0,str(tools));native=runpy.run_path(str(tools/'replay-divider.py'));nr=native['verify_output'](P/'native/run',pins['native_evidence_sha256'],pins['native_pins_sha256'],SOURCE,pins['native_sha256']);need(nr['captures']==4,'native screenshot count mismatch')
 cfg=P/'browser/config.json';need(sha(cfg)==pins['browser_config_sha256'],'browser config mismatch');c=data(cfg);need(c['source_sha256']==SOURCE and c['source_tree']==TREE and c['build_sha256']==pins['build_sha256'],'browser input mismatch');br=P/'browser/run';inv=data(br/'invocation.json');after=data(br/'inputs-after.json');need(inv['inputs']==after and after['config_sha256']==pins['browser_config_sha256'],'browser input receipts differ');sys.path.insert(0,str(P/'browser'));browser=runpy.run_path(str(P/'browser/browser.py'));result=browser['verify_run'](br,data(br/'headless-window-observation.json'));need(json.loads(json.dumps(result))==data(br/'verified-stock-material.json') and result['steps']==381,'stock browser result mismatch')
 histories={}
 for name in ('r1-dark-heading-missing','r2-focused-pilot'):
  h=P/'history'/name;summary=data(h/'diagnostic-summary.json');w=data(h/'headless-window-observation.json');invoke=data(h/'invocation.json');after=data(h/'inputs-after.json');need(invoke['inputs']==after and summary['inputs']==after and summary['scope']=='Focused Divider proof' and w['stock_exit']==0 and not w['problems'] and not w['visible_owned_windows'] and not w['remaining_observed_processes'] and sha(h/'pilot-runner.diff')==after['pilot_patch_sha256'],'diagnostic receipt mismatch');histories[name]={'source_commit':after['source_commit'],'steps':summary['steps'],'scope':'diagnostic only'}
 pilot=P/'history/r2-focused-pilot';need(data(pilot/'inputs-after.json')['source_tree']==TREE,'focused pilot source mismatch');pilot_result=browser['verify_divider_compositions'](pilot/'artifacts');need(len(pilot_result['images'])==8,'focused pilot image mismatch')
 print(json.dumps({'source_reconstructed_during_readback':False,'rust_passed':sum(int(p) for _,p,_,_ in tests),'rust_ignored':sum(int(i) for _,_,_,i in tests),'wasm_finished':wasm,'metal_passed':sum(int(p) for _,p,_,_ in gpu_tests),'native':nr,'browser':result,'history':histories},indent=2))
if __name__=='__main__':main()
