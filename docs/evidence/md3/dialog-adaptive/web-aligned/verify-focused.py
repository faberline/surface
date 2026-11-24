#!/usr/bin/env python3
"""Check the finished focused headless report, captures and process cleanup."""
from collections import Counter
from pathlib import Path
from hashlib import sha256
from struct import unpack
import json
import re
import subprocess
import sys

from PIL import Image, ImageChops

attempt = Path(sys.argv[1]).resolve()
assert (attempt / 'exit.txt').read_text() == 'runner=0 tee=0\n'
for kind in ['runtime', 'source']:
    assert (attempt / f'{kind}-before.log').read_bytes() == (attempt / f'{kind}-after.log').read_bytes()
log = (attempt / 'runner.log').read_text()
assert re.findall(r'^  \[(M\d+)\]', log, re.M) == [f'M{n}' for n in range(256, 261)]
result = re.search(r'case material \.\.\. ok \(5 steps, (\d+) checks, (\d+.\d+)s\)', log)
assert result and f'e2e result: 1 passed; 0 failed; {result[1]} checks' in log
report = json.loads((attempt / 'browser-material-dialog.json').read_text())
assert report['errors'] == [] and 'failure' not in report
assert report['adapter'] == {'isFallbackAdapter': False, 'vendor':'apple', 'architecture':'metal-3'}
assert report['basic']['ax']['role'] == 'dialog' and report['basic']['ax']['name'] == '行程通知'
assert report['basic']['escape_return'] and report['basic']['scrim_return']
short = report['short']
assert short['viewport'] == [320, 400] and short['dark'] and short['text_scale'] == 2 and short['pinned']
assert short['last_content_after'][1] + 40 < short['last_content_before'][1]
alert = report['alert']
assert alert['ax']['role'] == 'alertdialog' and alert['ax']['name'] == '連線中斷'
assert alert['acknowledgements'] == 1 and alert['escape_blocked'] and alert['click_through_blocked']
breakpoint = report['breakpoint']
assert breakpoint['viewport_widths'] == [599, 600, 599]
assert breakpoint['same_editor'] and breakpoint['selection']['range'] == [3, 4]
assert breakpoint['selection']['direction'] == 'backward' and breakpoint['selection']['value'] == '秋日散步'
assert breakpoint['tab_cycle'] == ['editor', 'close', 'save', 'editor', 'save']
transaction = report['transaction']
assert transaction['initial'] == '週末行程' and transaction['continued'] == transaction['saved'] == '秋日散步'
assert transaction['discards'] == transaction['saves'] == 1 and transaction['route_focus_restored']
assert transaction['close_continue_focus'] == 'dialog-demo-edit-close'
assert transaction['escape_continue_focus'] == 'dialog-demo-editor'
assert transaction['escape_continue_selection'] == breakpoint['selection']
assert breakpoint['selection']['runtime']['selection_anchor'] == 12
assert breakpoint['selection']['runtime']['selection_cursor'] == 9

expected = {
    'basic-wide': (1920,1528), 'basic-short-before': (640,888), 'basic-short-after': (640,888),
    'alert': (1920,1528), 'edit-compact': (1198,1088), 'edit-wide': (1200,1088), 'dirty-confirm': (640,888),
}
pngs = {}
for name, expected_size in expected.items():
    png = (attempt / f'material-dialog-adaptive-{name}.png').read_bytes()
    assert png[:8] == b'\x89PNG\r\n\x1a\n' and png[12:16] == b'IHDR'
    size = unpack('>II', png[16:24])
    assert size == expected_size, (name, size)
    pngs[name] = {'dimensions': size, 'sha256': sha256(png).hexdigest()}
assert pngs['basic-short-before']['sha256'] != pngs['basic-short-after']['sha256']
before = Image.open(attempt / 'material-dialog-adaptive-basic-short-before.png').convert('RGB')
after = Image.open(attempt / 'material-dialog-adaptive-basic-short-after.png').convert('RGB')
# These screenshots contain the same stock 44 dp Web host bar above the Canvas.
scale = before.width / short['viewport'][0]
canvas_top = before.height / scale - short['viewport'][1]
assert (scale, canvas_top) == (2, 44)


def crop(image, rect):
    x, y, w, h = rect
    return image.crop(tuple(round(scale * coordinate) for coordinate in (x, y+canvas_top, x+w, y+h+canvas_top)))


def color_mask(image, color):
    return [index for index, pixel in enumerate(image.get_flattened_data()) if pixel == color]


pinned_ink = {}
for name, rect in zip(('heading', 'cancel', 'confirm'), [short['heading'], *short['actions']]):
    first, last = crop(before, rect), crop(after, rect)
    colors = Counter(pixel for pixel in first.get_flattened_data() if min(pixel) > 150)
    ink = colors.most_common(1)[0][0]
    mask = color_mask(first, ink)
    assert len(mask) > 500 and mask == color_mask(last, ink), name
    pinned_ink[name] = {'rgb':ink, 'unchanged_ink_pixels':len(mask)}
body_diff = ImageChops.difference(crop(before, short['body']), crop(after, short['body']))
changed_body_pixels = sum(pixel != (0,0,0) for pixel in body_diff.get_flattened_data())
assert changed_body_pixels > 2000

processes = subprocess.check_output(['/bin/ps', '-axww', '-o', 'pid=,command='], text=True).splitlines()
profile = attempt / 'browser-temp'
runner = attempt.parent / 'surface-e2e'
owned_pids = []
for row in processes:
    pid, _, command = row.strip().partition(' ')
    if command.startswith(str(runner)) or (command.startswith('/Applications/Google Chrome.app/') and str(profile) in command):
        owned_pids.append(int(pid))
assert not owned_pids, owned_pids
result = {'browser': report['browser'], 'adapter':report['adapter'], 'steps':5, 'checks':int(result[1]),
          'elapsed_seconds':float(result[2]), 'headless':True, 'page_errors':0, 'remaining_processes':owned_pids,
          'short_canvas':{'pinned_ink':pinned_ink,'changed_body_pixels':changed_body_pixels},'pngs':pngs}
(attempt / 'verified-dialog.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
print(f"Verified {result['steps']} steps, {result['checks']} checks, {len(pngs)} PNGs, zero page errors and processes")
