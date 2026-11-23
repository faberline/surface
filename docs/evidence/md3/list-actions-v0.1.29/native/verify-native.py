#!/usr/bin/env python3
"""Replay independent Material row actions in the offscreen Gallery."""

import argparse
from itertools import combinations
import json
from pathlib import Path

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, find, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

DEMO = 'list-actions-demo'
BEFORE = f'{DEMO}-before'
AFTER = f'{DEMO}-after'
GROUP = 'list-actions'
A, B, C, D, E, F = [f'list-action-{letter}' for letter in 'abcdef']
PIN = A + '-pin'
MORE = A + '-more'
B_PIN = B + '-pin'
B_MORE = B + '-more'
C_PIN = C + '-pin'
C_ARCHIVE = C + '-archive'
C_MORE = C + '-more'
F_MORE = F + '-more'


def scene(app, *settings):
    activate(app, 'section-2')
    activate(app, f'{DEMO}-show')
    for setting in settings:
        activate(app, f'{DEMO}-{setting}')
    return settle(app)


def status(frame, label):
    want = '尚未操作' if label is None else f'最近操作：{label}'
    actual = node(frame, f'{DEMO}-status')['text']
    require(actual == want, f'Unexpected command result: {actual}')


def role_and_geometry(frame, owner, action_ids):
    group = node(frame, GROUP)
    require(group['role'] == 'List' and group['name'] == '書籤動作', 'Missing named native list')
    index, row = find(frame, owner + '-item')
    require(row['role'] == 'ListItem' and row.get('checked') is None, 'Expected passive list item')
    boxes = {}
    for action in action_ids:
        control = node(frame, action)
        require(control['role'] == 'Button' and control['name'], f'Missing named button {action}')
        if 'parent' in control:
            require(control['parent'] == index, f'{action} is not a sibling in the list item')
        rect = layout(frame, action)['bounds']
        require(min(rect['width'], rect['height']) >= 47.5, f'Target is too small: {action} {rect}')
        require(same_box(layout(frame, owner + '-item')['bounds'], rect), f'Target escaped row: {action}')
        boxes[action] = rect
    for left, right in combinations(boxes.values(), 2):
        separate = any(a[axis] + a[extent] <= b[axis] + .5
                       for axis, extent in (('x','width'), ('y','height'))
                       for a,b in ((left,right),(right,left)))
        require(separate, f'Buttons overlap: {left} {right}')
    return boxes


def focus_pixels(plan, frame, shot, identifier):
    box = visible(frame, identifier, 47.5)
    ink = node(frame, identifier)['focus_color']
    _, _, pixels = read_png((plan.out / shot['name']).read_bytes())
    x = round(box['x'] + box['width'] / 2)
    y = round(box['y'])
    colors = [list(pixels[top][4*x:4*x+4]) for top in range(y, y + 5)]
    close = [max(abs(a-b) for a,b in zip(color, ink)) <= 3 for color in colors]
    require(close == [True, True, True, False, False], f'Wrong inward focus stroke: {colors}')
    return {'samples': colors, 'expected_focus_color': ink}


def segmented(app, plan):
    frame = scene(app)
    actions = role_and_geometry(frame, A, (A, PIN, MORE))
    require(node(frame, PIN)['checked'] is True, 'Pin does not have its authored state')
    require(node(frame, B)['disabled'] and node(frame, B_MORE)['disabled'] and not node(frame, B_PIN)['disabled'],
            'A disabled button changed a sibling action')
    first = capture(app, plan, 'actions-segmented-light', frame=frame, sample_targets=(A + '-headline',))
    focus_by_tab(app, BEFORE)
    focus(key(app, 'tab'), A)
    frame = key(app, 'right')
    focus(frame, PIN)
    status(frame, None)
    focus_shot = capture(app, plan, 'actions-pin-focus', frame=frame)
    focus_shot['focus'] = focus_pixels(plan, frame, focus_shot, PIN)
    frame = key(app, 'space')
    require(node(frame, PIN)['checked'] is False, 'Space did not toggle the intended pin')
    status(frame, '固定山')
    for direction, target in (('down', MORE), ('right', B_PIN), ('left', MORE), ('up', PIN), ('left', A), ('up', F_MORE)):
        frame = key(app, direction)
        focus(frame, target)
    visible(frame, F_MORE, 47.5)
    status(frame, '固定山')
    frame = key(app, 'enter')
    status(frame, '更多森林的選項')
    reveal = capture(app, plan, 'actions-wrap-focus', frame=frame)
    focus(key(app, 'tab'), AFTER)
    focus(key(app, 'tab', reverse=True), A)
    return {'viewport': [760,640], 'targets': actions, 'wrap': [A,F_MORE],
            'separate_results': [PIN,F_MORE], 'transport_captures': [first, focus_shot, reveal]}


def standard(app, plan):
    frame = scene(app, 'theme', 'standard')
    actions = role_and_geometry(frame, A, (A, PIN, MORE))
    focus_by_tab(app, BEFORE)
    focus(key(app, 'tab'), A)
    frame = key(app, 'enter')
    status(frame, '山')
    require(node(frame, PIN)['checked'] is True, 'Primary unexpectedly toggled the pin')
    shot = capture(app, plan, 'actions-standard-dark', frame=frame, sample_targets=(A + '-headline',))
    return {'viewport': [760,640], 'primary_result': A, 'targets': actions, 'transport_captures': [shot]}


def baseline(app, plan):
    frame = scene(app, 'baseline')
    actions = role_and_geometry(frame, B, (B, B_PIN, B_MORE))
    focus_by_tab(app, BEFORE)
    focus(key(app, 'tab'), A)
    for item in (PIN, MORE, B_PIN):
        focus(key(app, 'right'), item)
    frame = key(app, 'enter')
    status(frame, '固定海')
    require(node(frame, B_PIN)['checked'] is True, 'Enabled action ignored a disabled primary')
    shot = capture(app, plan, 'actions-baseline-light', frame=frame)
    return {'viewport': [760,640], 'independent_result': B_PIN, 'targets': actions, 'transport_captures': [shot]}


def compact(app, plan):
    scene(app, 'theme', 'rtl', 'large-text')
    focus_by_tab(app, BEFORE)
    frame = key(app, 'tab')
    focus(frame, A)
    a_targets = role_and_geometry(frame, A, (A, PIN, MORE))
    require(a_targets[MORE]['x'] < a_targets[PIN]['x'] < a_targets[A]['x'], 'Inline targets do not mirror in RTL')
    for item in (PIN, MORE, B_PIN, C, C_PIN, C_ARCHIVE):
        frame = key(app, 'right')
        focus(frame, item)
    c_targets = role_and_geometry(frame, C, (C,C_PIN,C_ARCHIVE,C_MORE))
    heading = visible(frame, C + '-headline')
    support = visible(frame, C + '-supporting')
    require(heading['height'] > 48 and same_box(c_targets[C], heading) and same_box(c_targets[C], support),
            'Large wrapped text escapes the primary button')
    require(all(c_targets[v]['y'] >= c_targets[C]['y'] + c_targets[C]['height'] - .5 for v in (C_PIN,C_ARCHIVE,C_MORE)),
            'The 320dp row does not wrap its actions under the primary button')
    require(c_targets[C_MORE]['x'] < c_targets[C_ARCHIVE]['x'] < c_targets[C_PIN]['x'], 'RTL action strip did not mirror')
    frame = key(app, 'space')
    status(frame, '封存步道')
    shot = capture(app, plan, 'actions-compact-dark-rtl', frame=frame,
                   sample_targets=(C + '-headline', C + '-supporting'))
    return {'viewport': [320,900], 'inline': a_targets, 'wrapped': c_targets, 'headline': heading,
            'command_result': C_ARCHIVE, 'transport_captures': [shot]}


def short(app, plan):
    scene(app, 'theme', 'rtl', 'large-text')
    focus_by_tab(app, BEFORE)
    focus(key(app, 'tab'), A)
    frame = key(app, 'right')
    focus(frame, PIN)
    box = visible(frame, PIN, 47.5)
    shot = capture(app, plan, 'actions-short-dark-rtl', frame=frame)
    frame = key(app, 'enter')
    status(frame, '固定山')
    require(node(frame, PIN)['checked'] is False, 'Short button did not accept its own press')
    return {'viewport': [320,240], 'visible_target': box, 'command_result': PIN, 'transport_captures': [shot]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--gallery', required=True, type=Path)
    parser.add_argument('--cli', required=True, type=Path)
    parser.add_argument('--source', required=True)
    parser.add_argument('--out', required=True, type=Path)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True)
    result = {'source_manifest_sha256': plan.source, 'gallery_sha256': digest(plan.gallery),
              'transports': ['Page.screenshot','surface-devtools page screenshot','MCP page_screenshot'],
              'profiles': {}}
    for replay, width, height in ((segmented,760,640),(standard,760,640),(baseline,760,640),(compact,320,900),(short,320,240)):
        name = replay.__name__
        with launch(plan, name, width, height) as app:
            before = onscreen_windows(app.proc.pid)
            proof = replay(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(before == after == 0, f'{name} opened a visible Gallery window')
            proof['visible_windows'] = [before,after]
        require(app.proc.poll() is not None, f'{name} did not close the Gallery')
        result['profiles'][name] = proof
        print(f'PASS {name} {width}x{height}. No visible windows.', flush=True)
    (plan.out / 'native-list-actions-report.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')


if __name__ == '__main__':
    main()
