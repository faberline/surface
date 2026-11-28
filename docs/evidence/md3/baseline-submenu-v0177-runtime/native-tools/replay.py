#!/usr/bin/env python3
"""Configurable, source-bound native submenu replay. Requires reviewed pins."""
import argparse
from hashlib import sha256
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import shlex
from tempfile import TemporaryDirectory
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, launch, focus_by_tab
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

ROOT=Path(__file__).resolve().parent
HELPERS={'gallery_transport.py':'9530cfce6f25e32138a0602d8a36b1b747f82f35f5df856c65f709afc719a8d0',
         'native_runtime.py':'10174087c7ba35edc312061d08199a7ef8486c574f9455732c1f304d2e838de2',
         'render_helpers.py':'cc852005670915e2f33e0002143ebd78810fe641fb0373d45d0cf8331a02f21d',
         'window_check.py':'fd0f0b651544fddabaa26e0df5995620fb427b43d001f85c70daa9773a713262'}
SHA=re.compile(r'[0-9a-f]{64}')

def load_pins(path, expected):
    require(SHA.fullmatch(expected or '') and digest(path)==expected, 'Review and pin the config SHA-256')
    c=json.loads(path.read_text())
    require(set(c)=={'source','source_manifest','source_sha256','source_tree','source_files','binary_manifest','binary_sha256','ids','profiles'}, 'Wrong config fields')
    require(SHA.fullmatch(c['source_sha256']) and SHA.fullmatch(c['binary_sha256']) and
            re.fullmatch(r'[0-9a-f]{40}',c['source_tree']) and type(c['source_files']) is int and c['source_files']>0, 'Wrong reviewed source or binary pins')
    require(set(c['ids'])=={'section','show','back','opener','parent_menu','submenu_trigger','child_menu','enabled_child','disabled_child'}, 'Wrong fixture IDs')
    require(all(isinstance(x,str) and x for x in c['ids'].values()) and len(set(c['ids'].values()))==len(c['ids']), 'Fixture IDs must be distinct')
    require(set(c['profiles'])=={'wide-light','narrow-dark-rtl'}, 'Both profiles are required')
    for name,p in c['profiles'].items():
        require(set(p)=={'width','height','settings','open_key','close_key'} and
                all(type(p[k]) is int and 0<p[k]<=1200 for k in ('width','height')) and
                isinstance(p['settings'],list) and all(isinstance(x,dict) and set(x)=={'id','state'} and
                isinstance(x['id'],str) and bool(x['id']) and x['state'] in ('checked','selected') for x in p['settings']) and
                (p['open_key'],p['close_key'])==(('right','left') if name=='wide-light' else ('left','right')),
                'Wrong viewport, settings or logical arrow keys')
    require(c['profiles']['wide-light']['width']>=600 and c['profiles']['wide-light']['settings']==[] and
            c['profiles']['narrow-dark-rtl']['width']<=400 and len(c['profiles']['narrow-dark-rtl']['settings'])>=2,
            'Review wide light and narrow dark RTL profiles')
    return c

def entries(root, manifest, expected):
    require(digest(manifest)==expected, 'Manifest SHA mismatch')
    result={}
    for line in manifest.read_text().splitlines():
        h,name=line.split('  ',1); rel=PurePosixPath(name)
        require(SHA.fullmatch(h) and not rel.is_absolute() and '..' not in rel.parts and str(rel)==name and name not in result, 'Bad manifest entry')
        path=root/name
        require(path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root) and digest(path)==h, 'Input differs: '+name)
        result[name]=h
    return result

def inputs(c):
    for name,h in HELPERS.items(): require(digest(ROOT/name)==h, 'Original helper differs: '+name)
    source=Path(c['source']).resolve(strict=True); manifest=Path(c['source_manifest']).resolve(strict=True)
    files=entries(source,manifest,c['source_sha256'])
    actual={p.relative_to(source).as_posix() for p in source.rglob('*') if (p.is_file() or p.is_symlink()) and p.relative_to(source).parts[0]!='.git'}
    require(set(files)==actual and len(files)==c['source_files'], 'Source inventory differs')
    binaries=Path(c['binary_manifest']).resolve(strict=True); bundle=binaries.parent
    hashes=entries(bundle,binaries,c['binary_sha256'])
    require(set(hashes)=={'surface-material-gallery','surface-devtools'}, 'Wrong binary pair')
    gallery,cli=(bundle/name for name in ('surface-material-gallery','surface-devtools'))
    require(all(os.access(p,os.X_OK) for p in (gallery,cli)), 'Binary is not executable')
    return gallery,cli,{'source_sha256':c['source_sha256'],'reviewed_tree':c['source_tree'],
                        'source_files':len(files),'binary_manifest_sha256':c['binary_sha256'],'binaries':hashes}

def guard_text(gallery,w,h):
    checks=' && '.join(f'[ "${i}" = {shlex.quote(a)} ]' for i,a in enumerate(('--headless','--width',str(w),'--height',str(h)),1))
    return '#!/bin/sh\n[ "$#" -eq 5 ] && '+checks+' || exit 71\nexec '+shlex.quote(str(gallery))+' "$@"\n'

def capture_pixels(path, rect):
    width,height,rows=read_png(path.read_bytes())
    x,y=math.ceil(rect['x']),math.ceil(rect['y']); r,b=math.floor(rect['x']+rect['width']),math.floor(rect['y']+rect['height'])
    require(0<=x<r<=width and 0<=y<b<=height and r-x>=48 and b-y>=32, 'Child is not fully visible')
    colors={tuple(rows[yy][4*xx:4*xx+4]) for yy in range(y,b) for xx in range(x,r)}
    require(len(colors)>1, 'Child screenshot has no painted detail')
    return {'child_bounds':rect,'distinct_rgba':len(colors)}

def play(directory,name,profile,ids,gallery,cli):
    windows={}; stage='launch'
    def sample(app,label):
        require(app.proc.poll() is None and label not in windows,'Invalid process sample')
        windows[label]=onscreen_windows(app.proc.pid); require(windows[label]==0,'Visible Gallery window')
    with TemporaryDirectory(prefix='.argv-',dir=directory) as tmp:
        guard=Path(tmp)/'headless-gallery'; guard.write_text(guard_text(gallery,profile['width'],profile['height'])); guard.chmod(0o700)
        plan=SimpleNamespace(out=directory,gallery=guard,cli=cli)
        with launch(plan,name,profile['width'],profile['height']) as app:
            try:
                sample(app,'ready'); stage='route'; activate(app,ids['section']); focus_by_tab(app,ids['show'])
                focus(key(app,'enter'),ids['back'])
                for setting in profile['settings']:
                    frame=activate(app,setting['id']); require(node(frame,setting['id']).get(setting['state']) is True,'Profile setting did not stay selected')
                stage='open-parent'; activate(app,ids['opener']); frame=settle(app); focus(frame,ids['submenu_trigger'])
                require(node(frame,ids['submenu_trigger']).get('expanded') is False,'Parent row not collapsed')
                stage='open-child'; frame=key(app,profile['open_key']); focus(frame,ids['enabled_child'])
                require(node(frame,ids['submenu_trigger']).get('expanded') is True and node(frame,ids['child_menu'])['role']=='Menu','Child menu did not open')
                parent=layout(frame,ids['parent_menu'])['bounds']; child=visible(frame,ids['child_menu'])
                if name=='wide-light':
                    require(child['x']+child['width']>parent['x']+parent['width'] or child['x']<parent['x'], 'Wide child does not extend beyond parent')
                sample(app,'before-capture'); shot=capture(app,plan,name+'-child'); pixels=capture_pixels(directory/shot['name'],child); sample(app,'after-capture')
                stage='disabled'; frame=key(app,'down'); focus(frame,ids['disabled_child']); require(node(frame,ids['disabled_child'])['disabled'] is True,'Expected disabled item')
                for press in ('enter','space'):
                    result=app.wire('Action.key',{'key':press}); require(result.get('changed') is False,'Disabled activation changed state'); focus(settle(app),ids['disabled_child'])
                app.wire('Action.activate',{'id':ids['disabled_child']},expected_error='disabled')
                frame=key(app,profile['close_key']); focus(frame,ids['submenu_trigger']); require(find(frame,ids['child_menu'])[1] is None,'Child did not close')
                frame=key(app,profile['open_key']); focus(frame,ids['enabled_child']); frame=key(app,'escape'); focus(frame,ids['submenu_trigger']); require(find(frame,ids['child_menu'])[1] is None,'Escape did not close child')
                frame=key(app,'escape'); focus(frame,ids['opener']); require(find(frame,ids['parent_menu'])[1] is None,'Escape did not close parent')
                sample(app,'closed'); frame=activate(app,ids['back']); focus(frame,ids['show']); sample(app,'returned')
            except Exception as error:
                record={'stage':stage,'error':str(error),'window_samples':windows}
                try: record['snapshot']=app.wire('DOM.snapshot',{})
                except Exception as unavailable: record['snapshot_error']=str(unavailable)
                (directory/(name+'-failure.json')).write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n')
                raise
        require(app.proc.returncode is not None, 'Gallery process not reaped')
    return {'screenshot':shot,'pixels':pixels,'parent_bounds':parent,'window_samples':windows,'pid':app.proc.pid,'reaped':True,
            'focus_trace':[ids['submenu_trigger'],ids['enabled_child'],ids['disabled_child'],ids['submenu_trigger'],
                           ids['enabled_child'],ids['submenu_trigger'],ids['opener'],ids['show']],
            'disabled_enter_space_changed':False,'disabled_direct':'disabled','parent_and_child_closed':True}

def verify(out,c,expected_pin,evidence_sha,proof):
    out=out.resolve(strict=True); table=entries(out,out/'SHA256SUMS',evidence_sha)
    actual={p.relative_to(out).as_posix() for p in out.rglob('*') if p.is_file()}-{'SHA256SUMS'}
    require(set(table)==actual,'Unexpected replay files')
    report=json.loads((out/'report.json').read_text())
    require(report['schema']=='surface.test.submenu.native.v1' and report['pins_sha256']==expected_pin and
            report['inputs']==proof and report['transports']==['Page.screenshot','CLI','MCP'] and
            report['window_scope']=='sampled; not continuous' and len(report['captures'])==2,'Wrong replay receipt')
    pids=set()
    for number,record in enumerate(report['captures'],1):
        require(set(record)==set(c['profiles']),'Wrong profile inventory')
        for name,item in record.items():
            shot=item['screenshot']; path=out/f'repeat-{number}'/shot['name']; ids=c['ids']
            child=item['pixels']['child_bounds']; parent=item['parent_bounds']
            require((name!='wide-light' or child['x']+child['width']>parent['x']+parent['width'] or child['x']<parent['x']) and
                    path.name==name+'-child.png' and digest(path)==shot['sha256'] and
                    item['pixels']==capture_pixels(path,item['pixels']['child_bounds']) and item['reaped'] is True and
                    type(item['pid']) is int and item['pid'] not in pids and set(item['window_samples'])==
                    {'ready','before-capture','after-capture','closed','returned'} and
                    all(v==0 for v in item['window_samples'].values()) and
                    item['focus_trace']==[ids['submenu_trigger'],ids['enabled_child'],ids['disabled_child'],ids['submenu_trigger'],
                                          ids['enabled_child'],ids['submenu_trigger'],ids['opener'],ids['show']] and
                    item['disabled_enter_space_changed'] is False and item['disabled_direct']=='disabled' and
                    item['parent_and_child_closed'] is True,'Native receipt or original PNG differs')
            pids.add(item['pid'])
    require(len(pids)==4,'Expected four independent Gallery processes')
    return {'captures':4,'gallery_processes_reaped':4,'zero_window_samples':20,'evidence_sha256':evidence_sha}

def main():
    p=argparse.ArgumentParser(description=__doc__); p.add_argument('--pins',type=Path,required=True); p.add_argument('--pins-sha256',required=True)
    mode=p.add_mutually_exclusive_group(required=True); mode.add_argument('--check-inputs',action='store_true'); mode.add_argument('--run',type=Path); mode.add_argument('--verify',type=Path)
    p.add_argument('--evidence-sha256')
    args=p.parse_args(); c=load_pins(args.pins,args.pins_sha256); gallery,cli,proof=inputs(c)
    if args.check_inputs: print(json.dumps(proof,indent=2)); return
    if args.verify:
        require(SHA.fullmatch(args.evidence_sha256 or ''),'Supply reviewed evidence manifest SHA')
        print(json.dumps(verify(args.verify,c,args.pins_sha256,args.evidence_sha256,proof),indent=2)); return
    out=args.run.resolve(); require(out!=Path('/tmp').resolve() and out.is_relative_to(Path('/tmp').resolve()) and not out.exists(),'Use fresh private /tmp output')
    old=os.umask(0o077)
    try:
        out.mkdir(mode=0o700); records=[]
        for repeat in (1,2):
            d=out/f'repeat-{repeat}'; d.mkdir(mode=0o700)
            records.append({name:play(d,name,profile,c['ids'],gallery,cli) for name,profile in c['profiles'].items()})
        require(inputs(c)==(gallery,cli,proof) and load_pins(args.pins,args.pins_sha256)==c,'Inputs changed during replay')
        for name in c['profiles']:
            a,b=(r[name] for r in records); require(a['screenshot']['sha256']==b['screenshot']['sha256'] and a['pixels']==b['pixels'],'Independent captures differ')
        result={'schema':'surface.test.submenu.native.v1','pins_sha256':args.pins_sha256,'inputs':proof,'captures':records,
                'transports':['Page.screenshot','CLI','MCP'],'window_scope':'sampled; not continuous'}
        (out/'report.json').write_text(json.dumps(result,ensure_ascii=False,indent=2,sort_keys=True)+'\n')
        (out/'SHA256SUMS').write_text(''.join(f'{digest(f)}  {f.relative_to(out).as_posix()}\n' for f in sorted(out.rglob('*')) if f.is_file()))
    finally: os.umask(old)
    evidence_sha=digest(out/'SHA256SUMS')
    print(json.dumps(verify(out,c,args.pins_sha256,evidence_sha,proof),indent=2))

if __name__=='__main__': main()
