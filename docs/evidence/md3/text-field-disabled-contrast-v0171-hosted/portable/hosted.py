#!/usr/bin/env python3
"""Collect exact original q71 PR CI evidence and verify a sealed offline package."""
import argparse
import gzip
from hashlib import sha256
import json
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import tempfile
from zipfile import ZipFile
from runpy import run_path

ROOT=Path(__file__).resolve().parent
REPO='faberline/surface'
GH='/opt/homebrew/bin/gh'
HEAD='da9403992bd643c86a5bba35071f809857c90e44'
BASE='edbcabf5495f24df8941a75572d994bb8d7b2f73'
MERGE='8189db7873b199032d94bc8ecaa85e048c5c3f1a'
TREE='e1c63aa7f5ec80309c9b6fe50d8c50aed1c25afc'
BRANCH='codex/md3-text-field-disabled-contrast-v0171'
RUNS={'push':(46140687061,140965701782,140965702104,HEAD),
      'pull_request':(46140694868,140965725380,140965725562,MERGE)}
SCRIPTS=('hosted.py','verify-offline.py','verify-field-contrast.py','README.md')
PREFIX='material-text-field-disabled-contrast'

def require(ok,msg):
    if not ok: raise ValueError(msg)
def digest(p): return sha256(p.read_bytes()).hexdigest()
def read(p): return json.loads(p.read_text())
def write(p,value): p.write_text(json.dumps(value,ensure_ascii=False,sort_keys=True,indent=2)+'\n')
def norm(value): return json.loads(json.dumps(value,ensure_ascii=False))
def gh(endpoint):
    return json.loads(subprocess.check_output([GH,'api','--method','GET',f'repos/{REPO}/{endpoint}']))
def download(endpoint,path):
    with path.open('xb') as f: subprocess.run([GH,'api','--method','GET',f'repos/{REPO}/{endpoint}'],stdout=f,check=True)
def pins(event):
    run,build,browser,checkout=RUNS[event]
    return {'event':event,'branch':BRANCH,'head':HEAD,'checkout':checkout,'run_id':run,
            'build_job_id':build,'browser_job_id':browser,'pr_number':247 if event=='pull_request' else None}
def pin_meta(pr,head,merge):
    require(pr['number']==247 and pr['state']=='open' and not pr['draft'] and
            pr['head']['sha']==HEAD and pr['head']['ref']==BRANCH and pr['head']['repo']['full_name']==REPO and
            pr['base']['sha']==BASE and pr['base']['ref']=='main' and pr['base']['repo']['full_name']==REPO and
            pr['merge_commit_sha']==MERGE, 'PR head, base, or merge checkout changed')
    require(head['sha']==HEAD and head['tree']['sha']==TREE and [p['sha'] for p in head['parents']]==[BASE], 'Head mismatch')
    require(merge['sha']==MERGE and merge['tree']['sha']==TREE and [p['sha'] for p in merge['parents']]==[BASE,HEAD], 'Merge mismatch')
def metadata():
    pr,head,merge=gh('pulls/247'),gh(f'git/commits/{HEAD}'),gh(f'git/commits/{MERGE}')
    pin_meta(pr,head,merge); values={}
    for event,(run_id,build,browser,_) in RUNS.items():
        url=f'actions/runs/{run_id}/attempts/1'
        run,jobs=gh(url),gh(f'{url}/jobs?per_page=100')
        require((run['id'],run['run_attempt'],run['head_sha'],run['event'],run['head_branch'],run['path'])==
                (run_id,1,HEAD,event,BRANCH,'.github/workflows/ci.yml'),'Wrong run')
        entries={j['name']:j for j in jobs['jobs']}
        require(len(entries)==len(jobs['jobs'])==2 and set(entries)=={'build-and-behavior','browser-e2e'} and
                entries['build-and-behavior']['id']==build and entries['browser-e2e']['id']==browser,'Wrong original jobs')
        values[event]=(run,jobs)
    return pr,head,merge,values
def collect(out):
    pr,head,merge,values=metadata()
    for event,(run,jobs) in values.items():
        require(run['status']=='completed' and run['conclusion']=='success' and
                all(j['status']=='completed' and j['conclusion']=='success' for j in jobs['jobs']),
                f'{event} original attempt is not green; no artifacts collected')
    out.mkdir(mode=0o700)
    for name,value in (('pr',pr),('head',head),('merge',merge)): write(out/f'{name}.json',value)
    for event,(run,jobs) in values.items():
        folder=out/event; folder.mkdir(); run_id,build,browser,checkout=RUNS[event]
        for name,value in (('run',run),('jobs',jobs),('checkout',head if checkout==HEAD else merge)): write(folder/f'{name}.json',value)
        write(folder/'pins.json',pins(event))
        artifacts=gh(f'actions/runs/{run_id}/artifacts?per_page=100')['artifacts']
        chosen=[a for a in artifacts if a['name']=='surface-e2e-artifacts']
        require(len(chosen)==1,'Wrong artifact count'); write(folder/'artifact.json',chosen[0])
        download(f'actions/jobs/{build}/logs',folder/'build.log')
        download(f'actions/jobs/{browser}/logs',folder/'browser.log')
        download(f"actions/artifacts/{chosen[0]['id']}/zip",folder/'original.zip')
        verifier=run_path(str(ROOT/'verify-offline.py'))
        verifier['verify'](folder,pins(event))
    return {'status':'original evidence collected and checked','directory':str(out)}
def extract_contrast(archive,out):
    with ZipFile(archive) as z:
        selected={}
        for entry in z.infolist():
            path=PurePosixPath(entry.filename); name=path.name
            if name==PREFIX+'.json' or (name.startswith(PREFIX+'-') and name.endswith('.png')):
                require(not path.is_absolute() and '..' not in path.parts and name not in selected and
                        not entry.is_dir() and 0<entry.file_size<20_000_000,'Unsafe or duplicate ZIP member')
                selected[name]=entry
        require(len(selected)==16,'Wrong disabled-contrast evidence inventory')
        for name,entry in selected.items(): (out/name).write_bytes(z.read(entry))
def offline(root):
    pin_meta(read(root/'pr.json'),read(root/'head.json'),read(root/'merge.json'))
    verifier=run_path(str(root/'verify-offline.py')); results={}
    with tempfile.TemporaryDirectory(prefix='surface-q71-hosted-readback-',dir='/tmp') as tmp:
        for event in RUNS:
            source=root/event; work=Path(tmp)/event; work.mkdir()
            for name in ('run.json','jobs.json','checkout.json','artifact.json','pins.json','original.zip'):
                shutil.copyfile(source/name,work/name)
            for kind in ('build','browser'): (work/f'{kind}.log').write_bytes(gzip.decompress((source/f'{kind}.log.gz').read_bytes()))
            require(read(work/'pins.json')==pins(event),'Pin file changed')
            result=verifier['verify'](work,pins(event)); selected=work/'selected'; selected.mkdir(); extract_contrast(work/'original.zip',selected)
            require({p.name for p in selected.iterdir()}=={p.name for p in (source/'selected').iterdir()} and
                    all(digest(p)==digest(source/'selected'/p.name) for p in selected.iterdir()),'Selected originals differ from ZIP')
            artifact=read(work/'artifact.json')
            results[event]={**result,'run_url':f'https://github.com/{REPO}/actions/runs/{RUNS[event][0]}/attempts/1',
                            'artifact_url':f'https://github.com/{REPO}/actions/runs/{RUNS[event][0]}/artifacts/{artifact["id"]}'}
    return norm({'pr':247,'pr_url':f'https://github.com/{REPO}/pull/247',
                 'source_url':f'https://github.com/{REPO}/tree/{HEAD}',
                 'head':HEAD,'base':BASE,'merge':MERGE,'tree':TREE,'events':results})
def export(originals,out):
    out.mkdir(mode=0o700)
    for name in ('pr.json','head.json','merge.json'): shutil.copyfile(originals/name,out/name)
    for name in SCRIPTS: shutil.copyfile(ROOT/name,out/name)
    for event in RUNS:
        src=originals/event; dst=out/event; dst.mkdir()
        for name in ('run.json','jobs.json','checkout.json','artifact.json','pins.json','original.zip'): shutil.copyfile(src/name,dst/name)
        for kind in ('build','browser'): (dst/f'{kind}.log.gz').write_bytes(gzip.compress((src/f'{kind}.log').read_bytes(),mtime=0))
        selected=dst/'selected'; selected.mkdir(); extract_contrast(dst/'original.zip',selected)
    result=offline(out); write(out/'verified.json',result)
    (out/'SHA256SUMS').write_text(''.join(f'{digest(p)}  {p.relative_to(out).as_posix()}\n' for p in sorted(out.rglob('*')) if p.is_file()))
    return {'directory':str(out),'manifest_sha256':digest(out/'SHA256SUMS'),'events':list(RUNS)}
def verify(root,sha):
    sums=root/'SHA256SUMS'; require(digest(sums)==sha,'Manifest hash differs')
    entries={}
    for line in sums.read_text().splitlines():
        h,name=line.split('  ',1); p=PurePosixPath(name)
        require(len(h)==64 and not p.is_absolute() and '..' not in p.parts and str(p)==name and name not in entries,'Invalid manifest member')
        entries[name]=h
    actual={p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file()}-{'SHA256SUMS'}
    require(actual==set(entries) and all(not (root/n).is_symlink() and digest(root/n)==h for n,h in entries.items()),'Package seal differs')
    result=offline(root); require(result==read(root/'verified.json'),'Offline receipt changed'); return result
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__); mode=p.add_mutually_exclusive_group(required=True)
    mode.add_argument('--preflight',action='store_true'); mode.add_argument('--collect',type=Path)
    mode.add_argument('--export',type=Path); mode.add_argument('--verify-package',type=Path)
    p.add_argument('--originals',type=Path); p.add_argument('--package-sha'); args=p.parse_args()
    if args.preflight:
        _,_,_,values=metadata(); result={'status':'preflight only','runs':{k:{'status':v[0]['status'],'conclusion':v[0]['conclusion']} for k,v in values.items()}}
    elif args.collect:
        require(args.collect.resolve().is_relative_to(Path('/tmp').resolve()),'Use fresh /tmp'); result=collect(args.collect)
    elif args.export:
        require(args.originals and args.export.resolve().is_relative_to(Path('/tmp').resolve()),'Provide originals and fresh /tmp'); result=export(args.originals,args.export)
    else:
        require(args.package_sha is not None,'Provide reviewed seal'); result=verify(args.verify_package,args.package_sha)
    print(json.dumps(result,ensure_ascii=False,indent=2,sort_keys=True))
