#!/usr/bin/env python3
"""Read already collected q71 original CI receipts and disabled-contrast PNGs; no network."""
import argparse
from datetime import datetime
from hashlib import sha256
import json
from pathlib import Path
import re
from runpy import run_path
from tempfile import TemporaryDirectory
from zipfile import ZipFile

TREE = 'e1c63aa7f5ec80309c9b6fe50d8c50aed1c25afc'
PRODUCT_HEAD = 'da9403992bd643c86a5bba35071f809857c90e44'
BASE = 'edbcabf5495f24df8941a75572d994bb8d7b2f73'
SUITES = ('smoke','ime','material','bridge-native','bridge-native-compact','bridge-web','devtools-native','devtools-headless')
REQUIRED = {
 'build-and-behavior': ('Run actions/checkout@v4','Run cargo xtask provenance','Run cargo fmt --all --check',
   'Run cargo clippy --locked --workspace --all-targets --all-features -- -D warnings',
   'Run cargo test --locked --workspace --all-features',
   'Run cargo check --locked --target wasm32-unknown-unknown -p surface-showcase -p surface-material-gallery --all-features --lib',
   'Run cargo check --locked --target wasm32-unknown-unknown -p ui-runtime --all-features --lib --test consumer_contract',
   'Run cargo check --locked --target wasm32-unknown-unknown -p ui-runtime --no-default-features --lib --test consumer_contract'),
 'browser-e2e': ('Run actions/checkout@v4','Run surface-e2e','Run actions/upload-artifact@v4')}

def require(ok, message):
    if not ok: raise ValueError(message)

def read(path): return json.loads(path.read_text())
def digest(path): return sha256(path.read_bytes()).hexdigest()
def norm(value): return json.loads(json.dumps(value, ensure_ascii=False))

def verify(folder, config):
    required = {'event','branch','head','checkout','run_id','build_job_id','browser_job_id','pr_number'}
    require(set(config) == required and config['event'] in ('push','pull_request'), 'Supply reviewed exact run pins')
    require(all(type(config[k]) is int and config[k] > 0 for k in ('run_id','build_job_id','browser_job_id')), 'Invalid IDs')
    require(all(re.fullmatch(r'[a-f0-9]{40}', config[k]) for k in ('head','checkout')), 'Invalid SHA')
    if config['event'] == 'pull_request':
        require(config['head'] == PRODUCT_HEAD and type(config['pr_number']) is int and config['pr_number']>0, 'Unexpected product head or PR')
    else:
        require(config['pr_number'] is None, 'Main push must not name a PR')
    run, jobs, checkout, artifact = (read(folder / f'{name}.json') for name in ('run','jobs','checkout','artifact'))
    require((run['id'],run['run_attempt'],run['head_sha'],run['event'],run['head_branch'],run['path']) ==
            (config['run_id'],1,config['head'],config['event'],config['branch'],'.github/workflows/ci.yml')
            and run['status'] == 'completed' and run['conclusion'] == 'success', 'Wrong or unsuccessful original run')
    require(checkout['sha'] == config['checkout'] and checkout['tree']['sha'] == TREE, 'Wrong checkout commit or tree')
    if config['event'] == 'pull_request':
        require([(p['number'],p['head']['sha'],p['base']['sha']) for p in run['pull_requests']] ==
                [(config['pr_number'],PRODUCT_HEAD,BASE)] and
                [p['sha'] for p in checkout['parents']] == [BASE,PRODUCT_HEAD], 'Wrong PR linkage or merge lineage')
    if config['event'] == 'push':
        require(config['head'] == config['checkout'] == PRODUCT_HEAD and config['branch'] == 'codex/md3-text-field-disabled-contrast-v0171' and
                [p['sha'] for p in checkout['parents']] == [BASE], 'Wrong main lineage')
    entries = {j['name']:j for j in jobs['jobs']}
    require(len(entries)==len(jobs['jobs'])==2 and set(entries)==set(REQUIRED), 'Wrong original jobs')
    for name, key in [('build-and-behavior','build_job_id'),('browser-e2e','browser_job_id')]:
        j=entries[name]; steps={s['name']:s for s in j['steps']}
        require((j['id'],j['run_id'],j['run_attempt'],j['head_sha']) == (config[key],config['run_id'],1,config['head'])
                and j['status']=='completed' and j['conclusion']=='success' and all(
                    n in steps and steps[n]['status']=='completed' and steps[n]['conclusion']=='success' for n in REQUIRED[name]), 'Wrong or failed job')
    require(artifact['name']=='surface-e2e-artifacts' and artifact['expired'] is False and
            artifact['workflow_run']['id']==config['run_id'] and artifact['workflow_run']['head_sha']==config['head'] and
            re.fullmatch(r'sha256:[a-f0-9]{64}', artifact['digest']), 'Wrong artifact receipt')
    job=entries['browser-e2e']; created=datetime.fromisoformat(artifact['created_at'])
    require(datetime.fromisoformat(job['started_at']) <= created <= datetime.fromisoformat(job['completed_at']), 'Artifact outside original job')
    archive=folder/'original.zip'
    require(digest(archive)==artifact['digest'][7:] and archive.stat().st_size==artifact['size_in_bytes'], 'Original ZIP mismatch')
    build=(folder/'build.log').read_text(); browser=(folder/'browser.log').read_text()
    for log in (build,browser):
        require(re.findall(r'git log -1 --format=%H\r?\n\S+ ([0-9a-f]{40})',log)==[config['checkout']], 'Checkout log mismatch')
    tests=re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',build)
    require('provenance:' in build and tests and all(s=='ok' and f=='0' for s,_,f,_ in tests) and sum(int(p) for _,p,_,_ in tests)>1500, 'Build log failed')
    starts=re.findall(r'^.*?Z case ([\w-]+) \.\.\.\s*$',browser,re.M)
    passed=re.findall(r'^.*?Z case ([\w-]+) \.\.\. ok \((\d+) steps, (\d+) checks,',browser,re.M)
    require(starts==[r[0] for r in passed]==list(SUITES), 'Wrong suites')
    cases={name:(int(steps),int(count)) for name,steps,count in passed}
    require(all(n>0 and c>0 for n,c in cases.values()) and cases['material'][0]==358 and
            [int(x) for x in re.findall(r'\[M(\d+)\]',browser)]==[x for x in range(1,360) if x!=23] and
            [int(x) for x in re.findall(r'\[I(\d+)\]',browser)]==list(range(1,21)), 'Wrong ordered steps')
    total=sum(c for _,c in cases.values())
    require(re.findall(r'e2e result: (\d+) passed; (\d+) failed; (\d+) checks',browser)==[('8','0',str(total))]
            and 'xvfb-run -a' in browser and '--use-webgpu-adapter=swiftshader' in browser, 'Wrong Linux suite result')
    browsers=set(re.findall(r'\bbrowser: (Chrome/[0-9]+(?:\.[0-9]+){3})\b',browser)); require(len(browsers)==1, 'Wrong Chrome identity')
    prefix='material-text-field-disabled-contrast'
    with TemporaryDirectory(prefix='surface-q71-ci-readback-',dir='/tmp') as temp:
        with ZipFile(archive) as z:
            members=[e for e in z.infolist() if Path(e.filename).name == prefix+'.json' or
                     (Path(e.filename).name.startswith(prefix+'-') and e.filename.endswith('.png'))]
            require(len(members)==16 and len({Path(e.filename).name for e in members})==16 and all(
                not Path(e.filename).is_absolute() and '..' not in Path(e.filename).parts and
                not e.is_dir() and 0 < e.file_size < 20_000_000 for e in members), 'Wrong disabled-contrast screenshot inventory')
            for e in members: (Path(temp)/Path(e.filename).name).write_bytes(z.read(e))
        contrast=run_path(str(Path(__file__).with_name('verify-field-contrast.py')))['verify_captures'](Path(temp))
    adapter=contrast['adapter']
    require(contrast['browser'] in browsers and adapter=={'vendor':'google','architecture':'swiftshader','isFallbackAdapter':True}, 'Wrong Linux adapter')
    return norm({'config':config,'artifact_sha256':digest(archive),'cases':cases,'checks':total,
                 'rust_passed':sum(int(p) for _,p,_,_ in tests),'disabled_contrast':contrast})

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__); p.add_argument('--originals',type=Path,required=True); p.add_argument('--pins',type=Path,required=True)
    args=p.parse_args(); print(json.dumps(verify(args.originals,read(args.pins)),ensure_ascii=False,indent=2,sort_keys=True))
