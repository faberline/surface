#!/usr/bin/env python3
"""Reviewed source and native screenshot proof for flexible navigation badges."""
import argparse, json, math, os, re, subprocess
from pathlib import Path
from types import SimpleNamespace
from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import capture, find, layout, node, settle, visible
from window_check import onscreen_windows

IDS = ('navigation-badges-demo-home','navigation-badges-demo-inbox','navigation-badges-demo-updates','navigation-badges-demo-explore','navigation-badges-demo-settings')
NAMES = ('首頁','收件匣，1200 則未讀','最新消息，有新內容','探索路線，有新內容','設定')
PROFILES = [('599-light',599,720,False),('600-light',600,720,False),('320-dark-rtl-200',320,720,True),('839-dark-rtl-200',839,720,True)]
SHA = re.compile(r'[0-9a-f]{64}')

def pinned(root, manifest, expected):
    root = root.resolve(strict=True)
    require(SHA.fullmatch(expected or '') and digest(manifest) == expected, 'manifest hash differs')
    files = {}
    for line in manifest.read_text().splitlines():
        h, name = line.split('  ', 1)
        part = Path(name)
        f = root / part
        require(SHA.fullmatch(h) and not part.is_absolute() and '..' not in part.parts
                and name not in files and f.is_file() and not f.is_symlink()
                and f.resolve().is_relative_to(root) and digest(f) == h, 'invalid manifest entry')
        files[name] = h
    return files

def inputs(path, expected, source_only=False):
    require(SHA.fullmatch(expected or '') and digest(path) == expected, 'config hash differs')
    c = json.loads(path.read_text())
    require(set(c) == {'source','source_manifest','source_sha256','source_commit','source_tree','source_files','binary_manifest','binary_sha256'}, 'wrong config schema')
    source = Path(c['source']).resolve(strict=True)
    files = pinned(source, Path(c['source_manifest']), c['source_sha256'])
    actual = {p.relative_to(source).as_posix() for p in source.rglob('*') if (p.is_file() or p.is_symlink()) and p.relative_to(source).parts[0] != '.git'}
    require(len(files) == c['source_files'] and set(files) == actual, 'source inventory differs')
    env = dict(os.environ, GIT_OPTIONAL_LOCKS='0')
    got = subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD','HEAD^{tree}'], text=True, env=env).splitlines()
    status = subprocess.check_output(['git','-C',str(source),'status','--porcelain'], text=True, env=env)
    require(got == [c['source_commit'],c['source_tree']] and not status, 'source tree differs')
    if source_only:
        return c, None
    manifest = Path(c['binary_manifest']).resolve(strict=True)
    bins = pinned(manifest.parent, manifest, c['binary_sha256'])
    require(set(bins) == {'surface-material-gallery','surface-devtools'}, 'binary inventory differs')
    return c, manifest.parent

def window(app, samples, phase):
    require(app.proc.poll() is None, 'Gallery exited early')
    samples[phase] = onscreen_windows(app.proc.pid)
    require(samples[phase] == 0, 'visible Gallery window')

def descendant(nodes, index, parent):
    while index is not None:
        if index == parent: return True
        index = nodes[index].get('parent')
    return False

def sample(rows, bounds, color, *, tolerance=8, face=None):
    left = max(0, math.ceil(bounds['x'])); top = max(0, math.ceil(bounds['y']))
    right = min(len(rows[0]) // 4, math.floor(bounds['x'] + bounds['width']))
    bottom = min(len(rows), math.floor(bounds['y'] + bounds['height']))
    require(left < right and top < bottom, 'pixel crop outside screenshot')
    def inside_face(x, y):
        if face is None:
            return True
        radius = min(face['width'], face['height']) / 2
        ax = min(max(x, face['x'] + radius), face['x'] + face['width'] - radius)
        ay = min(max(y, face['y'] + radius), face['y'] + face['height'] - radius)
        return (x - ax) ** 2 + (y - ay) ** 2 <= (radius - 1) ** 2
    return sum(inside_face(x + .5, y + .5)
               and all(abs(rows[y][4*x+i] - color[i]) <= tolerance for i in range(3))
               for y in range(top,bottom) for x in range(left,right))

def analyze(frame, png, width):
    nodes = frame['dom']['nodes']; layouts = frame['layout']['nodes']
    title = 'navigation-badges-demo-title'
    require(node(frame,title).get('text') == '導覽徽章', 'badge gallery title differs')
    visible(frame,title)
    for name, label in zip(IDS, NAMES):
        item = node(frame,name)
        require(item['role'] == 'Button' and item['name'] == label, 'destination semantics differ')
        bounds = visible(frame,name)
        if width >= 600: require(abs(bounds['width'] - 112) <= .5, 'medium cell width differs')
    require(node(frame,IDS[1]).get('selected') is True and node(frame,IDS[4]).get('disabled') is True, 'selection or disabled state differs')
    require(node(frame,'navigation-badges-demo-bar')['role'] == 'Navigation', 'navigation role differs')
    count = next((i for i,n in enumerate(nodes) if n.get('text') == '999+'),None)
    require(count is not None, 'count text missing')
    face = nodes[count].get('parent')
    require(face is not None and descendant(nodes,face,find(frame,IDS[1])[0]), 'count owner differs')
    color = nodes[face]['style']['background']; ink = nodes[count]['style']['foreground']
    require(len(color) == len(ink) == 4 and color[3] == ink[3] == 255 and color != ink, 'badge colors differ')
    _, _, rows = read_png(png.read_bytes()); output = {}
    for i, name in enumerate(IDS):
        bounds = layout(frame,name)['bounds']; pixels = sample(rows,bounds,color)
        require(pixels == 0 if i in (0,4) else pixels >= (8 if i == 2 else 30), 'badge face pixels differ')
        output[name] = {'bounds':bounds,'error_pixels':pixels}
    for owner, label in ((1,'999+'),(3,'新')):
        index = next((i for i,n in enumerate(nodes) if n.get('text') == label and descendant(nodes,i,find(frame,IDS[owner])[0])),None)
        require(index is not None, 'badge text missing')
        parent = nodes[index].get('parent')
        require(parent is not None and nodes[parent]['style']['background'] == color, 'badge face differs')
        bounds = layouts[index]['bounds']; face_bounds = layouts[parent]['bounds']
        tolerance = 56
        require(max(abs(a - b) for a, b in zip(color[:3], ink[:3])) > tolerance,
                'the text tolerance must exclude the badge fill')
        text_pixels = sample(rows, bounds, ink, tolerance=tolerance, face=face_bounds)
        require(text_pixels >= 4, f'{png.name}: badge {label} On Error text has {text_pixels} pixels')
        output[IDS[owner]].update({'text_bounds':bounds,'face_bounds':face_bounds,
                                  'on_error_pixels':text_pixels,'on_error_tolerance':tolerance})
    return {'error':color,'on_error':ink,'owners':output}

def cleared(frame, png, old_paint):
    inbox = node(frame, IDS[1])
    require(inbox['name'] == '收件匣' and inbox.get('selected') is True,
            'acknowledged Inbox must keep selection and lose the unread name')
    require(frame['interaction']['focused'] == IDS[1], 'Inbox lost keyboard focus')
    owner = visible(frame, IDS[1])
    require(owner == old_paint['owners'][IDS[1]]['bounds'], 'Inbox hit geometry changed')
    _, _, rows = read_png(png.read_bytes())
    require(sample(rows, owner, old_paint['error']) == 0, 'acknowledged badge still paints')
    return {'id': IDS[1], 'name': inbox['name'], 'error_pixels': 0, 'owner': owner}

def pose(plan, profile, width, height, dark):
    windows = {}
    acknowledged = None
    with launch(plan, profile, width, height) as app:
        window(app, windows, 'ready')
        app.activate('section-3'); settle(app)
        app.activate('navigation-badges-demo-show'); frame = settle(app)
        if dark:
            for id in ('navigation-badges-demo-theme','navigation-badges-demo-rtl','navigation-badges-demo-large-text'):
                app.activate(id); frame = settle(app)
        for id in ('navigation-badges-demo-theme','navigation-badges-demo-rtl','navigation-badges-demo-large-text'):
            require(node(frame,id).get('selected',False) == dark, 'profile settings differ')
        shot = capture(app, plan, profile, frame=frame)
        paint = analyze(frame, plan.out/shot['name'], width)
        window(app, windows, 'captured'); pid = app.proc.pid
        if profile == '320-dark-rtl-200':
            app.activate(IDS[0]); settle(app)
            focus_by_tab(app, IDS[1])
            app.key('enter'); acknowledged_frame = settle(app)
            acknowledged_shot = capture(app, plan, '320-dark-rtl-200-acknowledged',
                                        frame=acknowledged_frame)
            acknowledged = {'frame': acknowledged_frame, 'shot': acknowledged_shot,
                            'paint': cleared(acknowledged_frame,
                                             plan.out/acknowledged_shot['name'], paint)}
            window(app, windows, 'acknowledged')
    require(app.proc.returncode is not None, 'Gallery child not reaped')
    return {'profile':profile,'pid':pid,'reaped':True,'windows':windows,'frame':frame,'shot':shot,
            'paint':paint,'acknowledged':acknowledged}

def verify(out, evidence, config_sha, source_sha, binary_sha):
    out = out.resolve(strict=True)
    files = pinned(out,out/'SHA256SUMS',evidence)
    actual = {p.relative_to(out).as_posix() for p in out.rglob('*') if p.is_file() and p.name != 'SHA256SUMS'}
    require(actual == set(files), 'evidence inventory differs')
    report = json.loads((out/'report.json').read_text())
    require(report['config_sha256'] == config_sha and report['source_sha256'] == source_sha and report['binary_sha256'] == binary_sha and report['inputs_after'] == report['inputs_before'] and report['transports'] == ['Page.screenshot','CLI','MCP'], 'input or transport receipt differs')
    seen = set()
    for profile, width, _, _ in PROFILES:
        poses = [report['runs'][str(i)][profile] for i in (1,2)]
        require(poses[0]['shot']['sha256'] == poses[1]['shot']['sha256'] and poses[0]['paint'] == poses[1]['paint'], 'repeat pixels differ')
        for i, r in enumerate(poses,1):
            phases = {'ready','captured'}
            if profile == '320-dark-rtl-200': phases.add('acknowledged')
            require(r['reaped'] is True and set(r['windows']) == phases and all(v == 0 for v in r['windows'].values()) and r['pid'] not in seen, 'process or windows differ')
            seen.add(r['pid'])
            path = out/('repeat-'+str(i))/r['shot']['name']
            require(digest(path) == r['shot']['sha256'], 'PNG hash differs')
            require(analyze(r['frame'],path,width) == r['paint'], 'PNG ink readback differs')
            if profile == '320-dark-rtl-200':
                ack = r['acknowledged']; ack_png = path.parent/ack['shot']['name']
                require(digest(ack_png) == ack['shot']['sha256'], 'acknowledged PNG hash differs')
                require(cleared(ack['frame'], ack_png, r['paint']) == ack['paint'],
                        'acknowledged PNG ink readback differs')
            else:
                require(r['acknowledged'] is None, 'unexpected acknowledged capture')
        if profile == '320-dark-rtl-200':
            require(poses[0]['acknowledged']['shot']['sha256'] == poses[1]['acknowledged']['shot']['sha256'],
                    'acknowledged replay PNG differs')
    return {'captures':10,'window_samples':18,'reaped_children':8,'scope':'headless screenshots; no physical display or VoiceOver claim'}

def main():
    p = argparse.ArgumentParser(); mode = p.add_mutually_exclusive_group(required=True)
    mode.add_argument('--check-source',action='store_true'); mode.add_argument('--check-inputs',action='store_true'); mode.add_argument('--run',action='store_true'); mode.add_argument('--verify',type=Path)
    p.add_argument('--config',type=Path,required=True); p.add_argument('--config-sha256',required=True); p.add_argument('--out',type=Path); p.add_argument('--evidence-sha256')
    a = p.parse_args(); c, binary = inputs(a.config,a.config_sha256,a.check_source)
    if a.check_source or a.check_inputs:
        print('source and inputs verified' if binary else 'source verified'); return
    if a.verify:
        print(json.dumps(verify(a.verify,a.evidence_sha256,a.config_sha256,c['source_sha256'],c['binary_sha256']),indent=2)); return
    require(a.out is not None, 'output required'); out = a.out.resolve()
    require(out.is_relative_to(Path('/tmp').resolve()) and not out.exists(), 'use fresh /tmp output'); out.mkdir(mode=0o700)
    before = {'source':c['source_sha256'],'binary':c['binary_sha256']}; runs = {}
    for repeat in (1,2):
        folder = out/('repeat-'+str(repeat)); folder.mkdir()
        plan = SimpleNamespace(gallery=str(binary/'surface-material-gallery'),cli=str(binary/'surface-devtools'),out=folder)
        runs[str(repeat)] = {}
        for name,w,h,dark in PROFILES:
            runs[str(repeat)][name] = pose(plan,name,w,h,dark)
    inputs(a.config,a.config_sha256)
    report = {'config_sha256':a.config_sha256,'source_sha256':c['source_sha256'],'binary_sha256':c['binary_sha256'],'inputs_before':before,'inputs_after':before,'transports':['Page.screenshot','CLI','MCP'],'runs':runs}
    (out/'report.json').write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
    (out/'SHA256SUMS').write_text(''.join(f'{digest(x)}  {x.relative_to(out).as_posix()}\n' for x in sorted(out.rglob('*')) if x.is_file()))
    print(json.dumps(verify(out,digest(out/'SHA256SUMS'),a.config_sha256,c['source_sha256'],c['binary_sha256']),indent=2))
if __name__ == '__main__': main()
