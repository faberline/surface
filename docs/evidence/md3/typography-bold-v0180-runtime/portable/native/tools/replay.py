#!/usr/bin/env python3
"""Reviewed source and native screenshot proof for the q80 weight fixture."""
import argparse, json, os, re, subprocess
from pathlib import Path
from types import SimpleNamespace
from gallery_transport import read_png, require
from native_runtime import digest, launch
from render_helpers import capture, ink_sample, node, settle, visible
from window_check import onscreen_windows

IDS = ('font-weight-demo-regular', 'font-weight-demo-bold')
TEXT = 'Surface 繁體中文'
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

def pose(plan, profile, width, height, dark):
    windows = {}
    with launch(plan, profile, width, height) as app:
        window(app, windows, 'ready')
        app.activate('section-3'); settle(app)
        app.activate('font-weight-demo-show'); frame = settle(app)
        if dark:
            app.activate('font-weight-demo-theme'); frame = settle(app)
        require(bool(node(frame,'font-weight-demo-theme')['checked']) == dark, 'theme differs')
        for name in IDS:
            require(node(frame,name)['name'] == TEXT, 'specimen text differs')
            visible(frame,name)
        styles = [node(frame,n)['style'] for n in IDS]
        require(all(style['font_size'] == 16 and style['line_height'] == 24 for style in styles)
                and styles[0].get('font_weight', 400) == 400 and styles[1]['font_weight'] == 700
                and styles[0]['foreground'] == styles[1]['foreground'], 'specimen styles differ')
        shot = capture(app, plan, profile, frame=frame, sample_targets=IDS)
        normal = shot['ink'][IDS[0]]['ink_pixels']; bold = shot['ink'][IDS[1]]['ink_pixels']
        require(bold * 10 > normal * 11, 'bold ink is not at least 10 percent greater')
        window(app, windows, 'captured'); pid = app.proc.pid
    require(app.proc.returncode is not None, 'Gallery child not reaped')
    return {'profile':profile,'pid':pid,'reaped':True,'windows':windows,'frame':frame,'shot':shot}

def verify(out, evidence, config_sha, source_sha, binary_sha):
    out = out.resolve(strict=True)
    files = pinned(out,out/'SHA256SUMS',evidence)
    actual = {p.relative_to(out).as_posix() for p in out.rglob('*') if p.is_file() and p.name != 'SHA256SUMS'}
    require(actual == set(files), 'evidence inventory differs')
    report = json.loads((out/'report.json').read_text())
    require(report['config_sha256'] == config_sha and report['source_sha256'] == source_sha and report['binary_sha256'] == binary_sha and report['inputs_after'] == report['inputs_before'] and report['transports'] == ['Page.screenshot','CLI','MCP'], 'input or transport receipt differs')
    seen = set()
    for profile in ('wide-light','compact-dark'):
        poses = [report['runs'][str(i)][profile] for i in (1,2)]
        require(poses[0]['shot']['sha256'] == poses[1]['shot']['sha256'] and poses[0]['shot']['ink'] == poses[1]['shot']['ink'], 'repeat pixels differ')
        for i, r in enumerate(poses,1):
            require(r['reaped'] is True and set(r['windows']) == {'ready','captured'} and all(v == 0 for v in r['windows'].values()) and r['pid'] not in seen, 'process or windows differ')
            seen.add(r['pid'])
            path = out/('repeat-'+str(i))/r['shot']['name']
            require(digest(path) == r['shot']['sha256'], 'PNG hash differs')
            _, _, rows = read_png(path.read_bytes())
            samples = {name:ink_sample(r['frame'],rows,name) for name in IDS}
            require(samples == r['shot']['ink'] and samples[IDS[1]]['ink_pixels'] * 10 > samples[IDS[0]]['ink_pixels'] * 11, 'PNG ink readback differs')
    return {'captures':4,'window_samples':8,'reaped_children':4,'scope':'headless screenshots; no physical display or VoiceOver claim'}

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
        for name,w,h,dark in [('wide-light',960,800,False),('compact-dark',320,720,True)]:
            runs[str(repeat)][name] = pose(plan,name,w,h,dark)
    inputs(a.config,a.config_sha256)
    report = {'config_sha256':a.config_sha256,'source_sha256':c['source_sha256'],'binary_sha256':c['binary_sha256'],'inputs_before':before,'inputs_after':before,'transports':['Page.screenshot','CLI','MCP'],'runs':runs}
    (out/'report.json').write_text(json.dumps(report,indent=2,sort_keys=True)+'\n')
    (out/'SHA256SUMS').write_text(''.join(f'{digest(x)}  {x.relative_to(out).as_posix()}\n' for x in sorted(out.rglob('*')) if x.is_file()))
    print(json.dumps(verify(out,digest(out/'SHA256SUMS'),a.config_sha256,c['source_sha256'],c['binary_sha256']),indent=2))
if __name__ == '__main__': main()
