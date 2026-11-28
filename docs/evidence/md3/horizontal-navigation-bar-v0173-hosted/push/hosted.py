#!/usr/bin/env python3
"""Read-only collection and sealed offline readback for one reviewed q73 CI attempt."""
import argparse, hashlib, json, re, runpy, shutil, subprocess, sys, tempfile
from pathlib import Path, PurePosixPath
from zipfile import ZipFile
from datetime import datetime

ROOT = Path(__file__).resolve().parent
REPO = 'faberline/surface'
HEAD = 'c2571a3d818ddaf0cb7874998d1c39f0ffc7b1f2'
BASE = '6aa6052b6a830db2b16b481744ac451f45f007df'
TREE = '791d8b2dd3f9b44f3c2a967f1418b671920a2899'
SUITES = ('smoke','ime','material','bridge-native','bridge-native-compact','bridge-web','devtools-native','devtools-headless')
REQUIRED = {
 'build-and-behavior': ('Run actions/checkout@v4','Run cargo xtask provenance','Run cargo fmt --all --check','Run cargo clippy --locked --workspace --all-targets --all-features -- -D warnings','Run cargo test --locked --workspace --all-features','Run cargo check --locked --target wasm32-unknown-unknown -p surface-showcase -p surface-material-gallery --all-features --lib','Run cargo check --locked --target wasm32-unknown-unknown -p ui-runtime --all-features --lib --test consumer_contract','Run cargo check --locked --target wasm32-unknown-unknown -p ui-runtime --no-default-features --lib --test consumer_contract'),
 'browser-e2e': ('Run actions/checkout@v4','Run surface-e2e','Run actions/upload-artifact@v4')}

def need(ok,msg):
 if not ok: raise ValueError(msg)
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def data(p): return json.loads(p.read_text())
def write(p,value): p.write_text(json.dumps(value,indent=2,sort_keys=True,ensure_ascii=False)+'\n')
def gh(path): return json.loads(subprocess.check_output(['/opt/homebrew/bin/gh','api','--method','GET',f'repos/{REPO}/{path}']))
def get(path,out):
 with out.open('xb') as f: subprocess.run(['/opt/homebrew/bin/gh','api','--method','GET',f'repos/{REPO}/{path}'],stdout=f,check=True)
def config(path,expected):
 need(re.fullmatch('[0-9a-f]{64}',expected or '') and sha(path)==expected,'reviewed config SHA differs'); c=data(path)
 need(set(c)=={'pr_number','branch','event','run_id','build_job_id','browser_job_id','checkout'} and c['event'] in ('push','pull_request') and isinstance(c['branch'],str) and c['branch'] and all(type(c[k]) is int and c[k]>0 for k in ('pr_number','run_id','build_job_id','browser_job_id')) and re.fullmatch('[0-9a-f]{40}',c['checkout']),'invalid config')
 return c
def check_metadata(c,pr,commit,run,jobs,artifact):
 need(pr['number']==c['pr_number'] and pr['head']['sha']==HEAD and pr['head']['ref']==c['branch'] and pr['base']['sha']==BASE and pr['base']['ref']=='main' and pr['head']['repo']['full_name']==pr['base']['repo']['full_name']==REPO,'PR source differs')
 need(commit['sha']==c['checkout'] and commit['tree']['sha']==TREE,'checkout tree differs')
 expected_parents=[BASE,HEAD] if c['event']=='pull_request' else [BASE]
 need([p['sha'] for p in commit['parents']]==expected_parents and (c['event']!='push' or c['checkout']==HEAD) and (c['event']!='pull_request' or pr['merge_commit_sha']==c['checkout']),'checkout lineage differs')
 need((run['id'],run['run_attempt'],run['head_sha'],run['event'],run['head_branch'],run['path'],run['status'],run['conclusion'])==(c['run_id'],1,HEAD,c['event'],c['branch'],'.github/workflows/ci.yml','completed','success'),'run differs or not green')
 if c['event']=='pull_request': need([(p['number'],p['head']['sha'],p['base']['sha']) for p in run['pull_requests']]==[(c['pr_number'],HEAD,BASE)],'PR run linkage differs')
 entries={j['name']:j for j in jobs['jobs']};need(len(entries)==len(jobs['jobs'])==2 and set(entries)==set(REQUIRED),'job inventory differs')
 for name,key in [('build-and-behavior','build_job_id'),('browser-e2e','browser_job_id')]:
  j=entries[name];steps={s['name']:s for s in j['steps']};need((j['id'],j['run_id'],j['run_attempt'],j['head_sha'],j['status'],j['conclusion'])==(c[key],c['run_id'],1,HEAD,'completed','success') and all(k in steps and steps[k]['status']=='completed' and steps[k]['conclusion']=='success' for k in REQUIRED[name]),'job or required step differs')
 need(artifact['name']=='surface-e2e-artifacts' and not artifact['expired'] and artifact['workflow_run']['id']==c['run_id'] and artifact['workflow_run']['head_sha']==HEAD and re.fullmatch('sha256:[0-9a-f]{64}',artifact['digest']) and artifact['size_in_bytes']>0,'artifact receipt differs')
 job=entries['browser-e2e'];created=datetime.fromisoformat(artifact['created_at'])
 need(datetime.fromisoformat(job['started_at']) <= created <= datetime.fromisoformat(job['completed_at']),'artifact timestamp differs')
def check_logs(c,build,browser):
 for log in (build,browser): need(re.findall(r'git log -1 --format=%H\r?\n\S+ ([0-9a-f]{40})',log)==[c['checkout']],'checkout log differs')
 tests=re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',build);need('provenance:' in build and tests and all(s=='ok' and f=='0' for s,_,f,_ in tests),'Rust results differ')
 starts=re.findall(r'^.*?Z case ([\w-]+) \.\.\.\s*$',browser,re.M);passed=re.findall(r'^.*?Z case ([\w-]+) \.\.\. ok \((\d+) steps, (\d+) checks,',browser,re.M);need(starts==[x[0] for x in passed]==list(SUITES),'suite order differs')
 cases={n:[int(s),int(k)] for n,s,k in passed};total=sum(v[1] for v in cases.values());need(cases['material'][0]==364 and [int(x) for x in re.findall(r'\[M(\d+)\]',browser)]==[x for x in range(1,366) if x!=23] and [int(x) for x in re.findall(r'\[I(\d+)\]',browser)]==list(range(1,21)),'ordered steps differ')
 need(re.findall(r'e2e result: (\d+) passed; (\d+) failed; (\d+) checks',browser)==[('8','0',str(total))] and 'xvfb-run -a' in browser and '--use-webgpu-adapter=swiftshader' in browser,'browser summary differs')
 versions=set(re.findall(r'\bbrowser: (Chrome/[0-9]+(?:\.[0-9]+){3})\b',browser));need(len(versions)==1,'browser identity differs')
 return {'rust_passed':sum(int(p) for _,p,_,_ in tests),'rust_ignored':sum(int(i) for _,_,_,i in tests),'cases':cases,'checks':total,'browser':next(iter(versions))}
def selected_names():
 return ['browser-material-flexible-navigation-bar.json','material-horizontal-navigation-bar.json']+[f'material-horizontal-navigation-bar-{tag}.png' for tag in ('light','rtl-large','rtl-large-scrolled')]
def pixels(folder, logs):
 sys.path.insert(0,str(ROOT));m=runpy.run_path(str(ROOT/'verify-navigation-pair.py'));compact=data(folder/'browser-material-flexible-navigation-bar.json')
 need(compact['browser']==logs['browser'] and compact['adapter']=={'vendor':'google','architecture':'swiftshader','isFallbackAdapter':True} and compact['errors']==[],'actual Linux adapter or report differs')
 return json.loads(json.dumps(m['medium'](folder,compact)))
def collect(c,out,config_path):
 need(out.resolve().is_relative_to(Path('/tmp').resolve()) and not out.exists(),'use fresh /tmp output')
 pr=gh(f"pulls/{c['pr_number']}");commit=gh(f"git/commits/{c['checkout']}");run=gh(f"actions/runs/{c['run_id']}/attempts/1");jobs=gh(f"actions/runs/{c['run_id']}/attempts/1/jobs?per_page=100");listing=gh(f"actions/runs/{c['run_id']}/artifacts?per_page=100");matches=[a for a in listing['artifacts'] if a['name']=='surface-e2e-artifacts'];need(len(matches)==1,'wrong artifact inventory');artifact=matches[0];check_metadata(c,pr,commit,run,jobs,artifact)
 out.mkdir(mode=0o700)
 shutil.copyfile(config_path,out/'config.json')
 for name,value in [('pr',pr),('commit',commit),('run',run),('jobs',jobs),('artifact',artifact)]:write(out/(name+'.json'),value)
 for name,key in [('build','build_job_id'),('browser','browser_job_id')]:get(f"actions/jobs/{c[key]}/logs",out/(name+'.log'))
 logs=check_logs(c,(out/'build.log').read_text(),(out/'browser.log').read_text());(out/'selected').mkdir()
 with tempfile.TemporaryDirectory(prefix='q73-original-zip-',dir='/tmp') as tmp:
  archive=Path(tmp)/'original.zip';get(f"actions/artifacts/{artifact['id']}/zip",archive);need(sha(archive)==artifact['digest'][7:] and archive.stat().st_size==artifact['size_in_bytes'],'original ZIP digest or size differs')
  with ZipFile(archive) as z:
   selected={}
   for e in z.infolist():
    p=PurePosixPath(e.filename);name=p.name
    if name in selected_names():
     need(not p.is_absolute() and '..' not in p.parts and name not in selected and not e.is_dir() and 0<e.file_size<20_000_000,'unsafe ZIP member');selected[name]=e
   need(set(selected)==set(selected_names()),'selected ZIP inventory differs')
   for n,e in selected.items():(out/'selected'/n).write_bytes(z.read(e))
 result=pixels(out/'selected',logs);write(out/'verified.json',{'scope':'original ZIP verified at collection; ZIP omitted from portable subset','zip_sha256':artifact['digest'],'logs':logs,'navigation':result})
 for n in ('hosted.py','verify-navigation-pair.py','verify-compact-predecessor.py','prepare-focused.py','evidence_common.py','README.md'):shutil.copy2(ROOT/n,out/n)
 (out/'SHA256SUMS').write_text(''.join(f'{sha(f)}  {f.relative_to(out).as_posix()}\n' for f in sorted(out.rglob('*')) if f.is_file()))
 return {'package':str(out),'manifest_sha256':sha(out/'SHA256SUMS'),'zip_sha256':artifact['digest']}
def verify(root,manifest,config_sha):
 need(sha(root/'SHA256SUMS')==manifest,'package seal differs');entries={}
 for line in (root/'SHA256SUMS').read_text().splitlines():
  h,n=line.split('  ',1);p=PurePosixPath(n);need(re.fullmatch('[0-9a-f]{64}',h) and not p.is_absolute() and '..' not in p.parts and n not in entries,'invalid manifest');entries[n]=h
 actual={p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file() and p!=root/'SHA256SUMS'};need(actual==set(entries) and all(not (root/n).is_symlink() and sha(root/n)==h for n,h in entries.items()),'package inventory differs')
 c=config(root/'config.json',config_sha);check_metadata(c,*(data(root/(n+'.json')) for n in ('pr','commit','run','jobs','artifact')));logs=check_logs(c,(root/'build.log').read_text(),(root/'browser.log').read_text());need(set(p.name for p in (root/'selected').iterdir())==set(selected_names()),'selected inventory differs');result=pixels(root/'selected',logs);receipt=data(root/'verified.json');need(receipt=={'scope':'original ZIP verified at collection; ZIP omitted from portable subset','zip_sha256':data(root/'artifact.json')['digest'],'logs':logs,'navigation':result},'receipt differs');return receipt
if __name__=='__main__':
 p=argparse.ArgumentParser();g=p.add_mutually_exclusive_group(required=True);g.add_argument('--collect',type=Path);g.add_argument('--verify',type=Path);p.add_argument('--config',type=Path);p.add_argument('--config-sha',required=True);p.add_argument('--manifest-sha');a=p.parse_args()
 if a.collect:need(a.config is not None,'config required');result=collect(config(a.config,a.config_sha),a.collect,a.config)
 else:need(a.manifest_sha is not None,'manifest SHA required');result=verify(a.verify.resolve(),a.manifest_sha,a.config_sha)
 print(json.dumps(result,indent=2,ensure_ascii=False))
