#!/usr/bin/env python3
"""Collect the reviewed q72 squash-main run and replay it fully offline."""
import argparse
import gzip
from hashlib import sha256
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import tempfile
from zipfile import ZipFile
from runpy import run_path

ROOT=Path(__file__).resolve().parent
REPO='faberline/surface'; GH='/opt/homebrew/bin/gh'
PRODUCT='e99b6c060304ad3de74237d9cdc5e152cb8237d8'
BASE='a64f60b796b7bffcd376a99aac903b5235dee2b7'
TREE='8673365398b4c7b51f5e7409abe280d79b79d852'
PREFIX='material-text-field-disabled-contrast'
SCRIPTS=('hosted-main.py','verify-offline.py','verify-field-contrast.py','verify-field-selection.py',
         'verify-system-pair.py','evidence_common.py','BOUND-README.md','binding.json')

def require(ok,msg):
    if not ok: raise ValueError(msg)
def digest(p): return sha256(p.read_bytes()).hexdigest()
def read(p): return json.loads(p.read_text())
def write(p,v): p.write_text(json.dumps(v,ensure_ascii=False,sort_keys=True,indent=2)+'\n')
def norm(v): return json.loads(json.dumps(v,ensure_ascii=False))
def gh(endpoint): return json.loads(subprocess.check_output([GH,'api','--method','GET',f'repos/{REPO}/{endpoint}']))
def download(endpoint,path):
    with path.open('xb') as f: subprocess.run([GH,'api','--method','GET',f'repos/{REPO}/{endpoint}'],stdout=f,check=True)
def binding(root,expected):
    p=root/'binding.json'; require(re.fullmatch('[0-9a-f]{64}',expected or '') and digest(p)==expected,'Wrong reviewed binding SHA')
    b=read(p); require(set(b)=={'head','tree','base','product','run_id','build_job_id','browser_job_id'} and
                       re.fullmatch('[0-9a-f]{40}',b['head']) and (b['tree'],b['base'],b['product'])==(TREE,BASE,PRODUCT) and
                       all(type(b[x]) is int and b[x]>0 for x in ('run_id','build_job_id','browser_job_id')),'Invalid binding')
    return b
def pins(b):
    return {'event':'push','branch':'main','head':b['head'],'checkout':b['head'],'run_id':b['run_id'],
            'build_job_id':b['build_job_id'],'browser_job_id':b['browser_job_id'],'pr_number':None}
def pin_meta(pr,head,listing,b):
    require(pr['number']==248 and pr['merged'] and pr['merge_commit_sha']==b['head'] and
            pr['head']['sha']==PRODUCT and pr['base']['ref']=='main' and
            pr['head']['repo']['full_name']==REPO and pr['base']['repo']['full_name']==REPO,'Wrong merged PR')
    require(head['sha']==b['head'] and head['tree']['sha']==TREE and [p['sha'] for p in head['parents']]==[BASE], 'Wrong squash commit')
    runs=listing['workflow_runs']
    require(listing['total_count']==len(runs)==1 and runs[0]['id']==b['run_id'] and runs[0]['head_sha']==b['head'] and
            runs[0]['head_branch']=='main' and runs[0]['event']=='push' and runs[0]['run_attempt']==1 and
            runs[0]['path']=='.github/workflows/ci.yml','Wrong original main run discovery')
def metadata(b):
    pr=gh('pulls/248'); head=gh(f"git/commits/{b['head']}")
    listing=gh(f"actions/workflows/ci.yml/runs?head_sha={b['head']}&event=push&branch=main&per_page=100")
    pin_meta(pr,head,listing,b)
    path=f"actions/runs/{b['run_id']}/attempts/1"; run,jobs=gh(path),gh(path+'/jobs?per_page=100')
    require((run['id'],run['run_attempt'],run['head_sha'],run['event'],run['head_branch'],run['path'])==
            (b['run_id'],1,b['head'],'push','main','.github/workflows/ci.yml'),'Wrong original run')
    entries={j['name']:j for j in jobs['jobs']}
    require(len(entries)==len(jobs['jobs'])==2 and set(entries)=={'build-and-behavior','browser-e2e'} and
            entries['build-and-behavior']['id']==b['build_job_id'] and entries['browser-e2e']['id']==b['browser_job_id'], 'Wrong original jobs')
    return pr,head,listing,run,jobs
def collect(out,b):
    pr,head,listing,run,jobs=metadata(b)
    require(run['status']=='completed' and run['conclusion']=='success' and
            all(j['status']=='completed' and j['conclusion']=='success' for j in jobs['jobs']),
            'Original main attempt is not green; no artifact collected')
    out.mkdir(mode=0o700)
    for name,value in (('pr',pr),('head',head),('listing',listing)): write(out/f'{name}.json',value)
    folder=out/'push'; folder.mkdir()
    for name,value in (('run',run),('jobs',jobs),('checkout',head),('pins',pins(b))): write(folder/f'{name}.json',value)
    artifacts=gh(f"actions/runs/{b['run_id']}/artifacts?per_page=100")['artifacts']
    selected=[a for a in artifacts if a['name']=='surface-e2e-artifacts']; require(len(selected)==1,'Wrong artifact count')
    write(folder/'artifact.json',selected[0])
    download(f"actions/jobs/{b['build_job_id']}/logs",folder/'build.log')
    download(f"actions/jobs/{b['browser_job_id']}/logs",folder/'browser.log')
    download(f"actions/artifacts/{selected[0]['id']}/zip",folder/'original.zip')
    run_path(str(ROOT/'verify-offline.py'))['verify'](folder,pins(b))
    return {'status':'original main evidence collected and checked','directory':str(out)}
def extract_contrast(archive,out):
    with ZipFile(archive) as z:
        selected={}
        for entry in z.infolist():
            p=PurePosixPath(entry.filename); name=p.name
            if (name==PREFIX+'.json' or name.startswith(PREFIX+'-') and name.endswith('.png')
                    or name in ('browser-material-web-contrast-selection.json',
                                'browser-material-web-contrast-selection-browser.json',
                                'material-web-contrast-selection-browser-native-arrow.json',
                                'material-web-contrast-selection-browser-delayed-select-arrow.json',
                                'browser-material-text-field-system-pair.json')
                    or name.startswith('material-web-contrast-selection-browser-') and name.endswith('.png')
                    or name.startswith('browser-material-text-field-system-pair-') and name.endswith('.png')):
                require(not p.is_absolute() and '..' not in p.parts and name not in selected and not entry.is_dir()
                        and 0<entry.file_size<20_000_000,'Unsafe or duplicate ZIP member'); selected[name]=entry
        require(len(selected)==46,'Wrong Field/FieldText evidence inventory')
        for name,entry in selected.items(): (out/name).write_bytes(z.read(entry))
def offline(root,b):
    pin_meta(read(root/'pr.json'),read(root/'head.json'),read(root/'listing.json'),b)
    source=root/'push'; verifier=run_path(str(root/'verify-offline.py'))
    with tempfile.TemporaryDirectory(prefix='surface-q72-main-readback-',dir='/tmp') as tmp:
        work=Path(tmp)
        for name in ('run.json','jobs.json','checkout.json','artifact.json','pins.json','original.zip'): shutil.copyfile(source/name,work/name)
        for kind in ('build','browser'): (work/f'{kind}.log').write_bytes(gzip.decompress((source/f'{kind}.log.gz').read_bytes()))
        require(read(work/'pins.json')==pins(b),'Run pins changed')
        result=verifier['verify'](work,pins(b)); selected=work/'selected'; selected.mkdir(); extract_contrast(work/'original.zip',selected)
        require({p.name for p in selected.iterdir()}=={p.name for p in (source/'selected').iterdir()} and
                all(digest(p)==digest(source/'selected'/p.name) for p in selected.iterdir()),'Selected files differ from original ZIP')
        artifact=read(work/'artifact.json')
    return norm({'pr':248,'head':b['head'],'tree':TREE,'base':BASE,'product':PRODUCT,
                 'pr_url':f'https://github.com/{REPO}/pull/248','source_url':f"https://github.com/{REPO}/tree/{b['head']}",
                 'run_url':f"https://github.com/{REPO}/actions/runs/{b['run_id']}/attempts/1",
                 'artifact_url':f"https://github.com/{REPO}/actions/runs/{b['run_id']}/artifacts/{artifact['id']}",'workflow':result})
def export(originals,out,b):
    out.mkdir(mode=0o700)
    for name in ('pr.json','head.json','listing.json'): shutil.copyfile(originals/name,out/name)
    for name in SCRIPTS: shutil.copyfile(ROOT/name,out/name)
    src=originals/'push'; dst=out/'push'; dst.mkdir()
    for name in ('run.json','jobs.json','checkout.json','artifact.json','pins.json','original.zip'): shutil.copyfile(src/name,dst/name)
    for kind in ('build','browser'): (dst/f'{kind}.log.gz').write_bytes(gzip.compress((src/f'{kind}.log').read_bytes(),mtime=0))
    selected=dst/'selected'; selected.mkdir(); extract_contrast(dst/'original.zip',selected)
    write(out/'verified.json',offline(out,b))
    (out/'SHA256SUMS').write_text(''.join(f'{digest(p)}  {p.relative_to(out).as_posix()}\n' for p in sorted(out.rglob('*')) if p.is_file()))
    return {'directory':str(out),'manifest_sha256':digest(out/'SHA256SUMS')}
def verify(root,sha,b):
    sums=root/'SHA256SUMS'; require(re.fullmatch('[0-9a-f]{64}',sha or '') and digest(sums)==sha,'Wrong reviewed package SHA')
    entries={}
    for line in sums.read_text().splitlines():
        h,name=line.split('  ',1); p=PurePosixPath(name)
        require(re.fullmatch('[0-9a-f]{64}',h) and not p.is_absolute() and '..' not in p.parts and str(p)==name and name not in entries,'Invalid manifest member'); entries[name]=h
    actual={p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file()}-{'SHA256SUMS'}
    require(actual==set(entries) and all(not (root/n).is_symlink() and digest(root/n)==h for n,h in entries.items()),'Package seal differs')
    result=offline(root,b); require(result==read(root/'verified.json'),'Offline receipt differs'); return result
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__); mode=p.add_mutually_exclusive_group(required=True)
    mode.add_argument('--preflight',action='store_true'); mode.add_argument('--collect',type=Path)
    mode.add_argument('--export',type=Path); mode.add_argument('--verify-package',type=Path)
    p.add_argument('--originals',type=Path); p.add_argument('--binding-sha',required=True); p.add_argument('--package-sha'); args=p.parse_args()
    source=args.verify_package.resolve() if args.verify_package else ROOT
    b=binding(source,args.binding_sha)
    if args.preflight:
        _,_,_,run,_=metadata(b); result={'status':'preflight only','run':b['run_id'],'run_status':run['status'],'conclusion':run['conclusion']}
    elif args.collect:
        require(args.collect.resolve().is_relative_to(Path('/tmp').resolve()),'Use fresh /tmp'); result=collect(args.collect,b)
    elif args.export:
        require(args.originals and args.export.resolve().is_relative_to(Path('/tmp').resolve()),'Provide originals and fresh /tmp'); result=export(args.originals,args.export,b)
    else: result=verify(args.verify_package,args.package_sha,b)
    print(json.dumps(result,ensure_ascii=False,indent=2,sort_keys=True))
