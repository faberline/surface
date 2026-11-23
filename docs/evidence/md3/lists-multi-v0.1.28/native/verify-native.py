#!/usr/bin/env python3
"""Replay independently checked Material 3 rows in the offscreen Metal Gallery."""

import argparse
import json
from pathlib import Path
import sys

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch, same_box
from render_helpers import activate, capture, focus, key, layout, node, settle, visible
from window_check import onscreen_windows

DEMO = 'lists-demo'
BEFORE = f'{DEMO}-before'
AFTER = f'{DEMO}-after'
GROUP = 'lists-items'
A, B, C, D, E, F = [f'lists-{letter}' for letter in 'abcdef']
ROWS = (A, B, C, D, E, F)


def open_scene(app, *settings):
    activate(app, 'section-2')
    activate(app, f'{DEMO}-show')
    activate(app, f'{DEMO}-multi')
    for setting in settings:
        activate(app, f'{DEMO}-{setting}')
    return settle(app)


def checked(frame, *expected):
    group = node(frame, GROUP)
    require(group['role'] == 'Group' and group['name'] == '書籤', 'Missing named native group')
    for row in ROWS:
        checkbox = node(frame, row)
        require(checkbox['role'] == 'Checkbox' and checkbox['checked'] is (row in expected),
                f'Unexpected native checkbox or state for {row}')
        require(not checkbox.get('mixed', False), 'The list selection is binary')
    require(node(frame, B)['disabled'], 'Disabled row can be selected')
    require(node(frame, C)['described_by_text'] == C + '-supporting', 'Missing checkbox supporting description')
    require(node(frame, f'{DEMO}-status')['text'] == f'已選取 {len(expected)} 項', 'Live count is stale')


def mark(frame, row):
    bounds = visible(frame, row + '-mark')
    require(bounds['width'] == bounds['height'] == 18., f'Checkbox visual is not 18dp: {bounds}')
    require(same_box(visible(frame, row, 48), bounds), f'Checkbox visual detached from its target: {bounds}')
    return bounds


def mark_samples(plan, frame, shot, row):
    bounds = mark(frame, row)
    width, height, pixels = read_png((plan.out / shot['name']).read_bytes())
    x = round(bounds['x'])
    y = round(bounds['y'])
    require(0 <= x < width - 18 and 0 <= y < height - 18, f'Mark outside the captured area: {bounds}')
    offsets = [(9, 0), (9, 1), (9, 2), (9, 9), (3, 3), (14, 4)]
    return [list(pixels[y + dy][4*(x + dx):4*(x + dx + 1)]) for dx, dy in offsets]


def segmented(app, plan):
    frame = open_scene(app)
    checked(frame, C, F)
    before = capture(app, plan, 'multi-segmented-light-start', frame=frame, sample_targets=(A + '-headline', C + '-supporting'))
    unchecked_pixels = mark_samples(plan, frame, before, A)
    require(unchecked_pixels[0] == unchecked_pixels[1] and unchecked_pixels[1] != unchecked_pixels[2],
            f'Unchecked outline must cover the first two pixels: {unchecked_pixels}')
    focus_by_tab(app, BEFORE)
    focus(key(app, 'tab'), C)
    frame = key(app, 'up')
    focus(frame, A)
    checked(frame, C, F)
    frame = key(app, 'space')
    checked(frame, A, C, F)
    focus(frame, A)
    added = capture(app, plan, 'multi-segmented-light-three-checked', frame=frame)
    checked_pixels = mark_samples(plan, frame, added, A)
    require(checked_pixels[2] == checked_pixels[4] and checked_pixels[4] != unchecked_pixels[4],
            f'Checked fill failed to replace the transparent center: {checked_pixels}')
    frame = key(app, 'up')
    focus(frame, F)
    visible(frame, F, 48)
    checked(frame, A, C, F)
    frame = key(app, 'enter')
    checked(frame, A, C)
    out = capture(app, plan, 'multi-segmented-light-last-deselected', frame=frame, sample_targets=(F + '-headline',))
    focus(key(app, 'tab'), AFTER)
    checked(key(app, 'enter'))
    focus(key(app, 'tab', reverse=True), A)
    return {'viewport': [760, 640], 'sets': [[C,F], [A,C,F], [A,C], []], 'wrap': [A,F],
            'unchecked_samples': unchecked_pixels, 'checked_samples': checked_pixels,
            'transport_captures': [before, added, out]}


def standard(app, plan):
    frame = open_scene(app, 'standard', 'theme')
    checked(frame, C, F)
    require(node(frame, A)['style']['corners'] == dict.fromkeys(('top_left','top_right','bottom_left','bottom_right'), 4),
            'Wrong Expressive Standard resting corners')
    focus_by_tab(app, BEFORE)
    focus(key(app, 'tab'), C)
    frame = key(app, 'enter')
    checked(frame, F)
    focus(frame, C)
    mark(frame, C)
    shot = capture(app, plan, 'multi-standard-dark', frame=frame, sample_targets=(C + '-headline',))
    return {'viewport': [760, 640], 'selected': [F], 'transport_captures': [shot]}


def baseline(app, plan):
    frame = open_scene(app, 'baseline')
    checked(frame, C, F)
    require(set(node(frame, A)['style']['corners'].values()) == {0}, 'Baseline resting row must be square')
    require(layout(frame, A + '-leading')['bounds']['width'] == 24., 'Baseline leading icon is not 24dp')
    mark(frame, C)
    focus_by_tab(app, BEFORE)
    focus(key(app, 'tab'), C)
    frame = key(app, 'down')
    focus(frame, D)
    checked(frame, C, F)
    shot = capture(app, plan, 'multi-baseline-light', frame=frame, sample_targets=(C + '-headline',))
    return {'viewport': [760, 640], 'selected': [C,F], 'transport_captures': [shot]}


def compact(app, plan):
    open_scene(app, 'theme', 'rtl', 'large-text')
    focus_by_tab(app, BEFORE)
    focus(key(app, 'tab'), C)
    frame = key(app, 'up')
    focus(frame, A)
    leading = visible(frame, A + '-leading')
    checkbox = mark(frame, A)
    require(checkbox['x'] + checkbox['width'] < leading['x'], 'RTL does not mirror the two slots')
    focus(key(app, 'down'), C)
    focus(key(app, 'down'), D)
    frame = key(app, 'enter')
    checked(frame, C, D, F)
    row = visible(frame, D, 48)
    headline = visible(frame, D + '-headline')
    support = visible(frame, D + '-supporting')
    require(headline['height'] > 48. and same_box(row, headline) and same_box(row, support),
            'Long RTL headline or support is clipped')
    mark(frame, D)
    shot = capture(app, plan, 'multi-compact-dark-rtl', frame=frame,
                   sample_targets=(D + '-headline', D + '-supporting', f'{DEMO}-status'))
    return {'viewport': [320, 900], 'selected': [C,D,F], 'row': row, 'headline': headline,
            'rtl_slots': [leading, checkbox], 'transport_captures': [shot]}


def short(app, plan):
    open_scene(app, 'theme', 'rtl', 'large-text')
    focus_by_tab(app, BEFORE)
    frame = key(app, 'tab')
    focus(frame, C)
    row = visible(frame, C, 48)
    mark(frame, C)
    checked(frame, C, F)
    shot = capture(app, plan, 'multi-short-dark-rtl', frame=frame, sample_targets=(C + '-headline',))
    frame = key(app, 'enter')
    checked(frame, F)
    focus(frame, C)
    return {'viewport': [320, 240], 'selected_after_enter': [F], 'visible_row': row, 'transport_captures': [shot]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--gallery', required=True, type=Path)
    parser.add_argument('--cli', required=True, type=Path)
    parser.add_argument('--source', required=True)
    parser.add_argument('--out', required=True, type=Path)
    plan = parser.parse_args()
    plan.out.mkdir(parents=True)
    result = {'source_manifest_sha256': plan.source, 'gallery_sha256': digest(plan.gallery),
              'transports': ['Page.screenshot', 'surface-devtools page screenshot', 'MCP page_screenshot'],
              'profiles': {}}
    setups = ((segmented, 760, 640), (standard, 760, 640), (baseline, 760, 640),
              (compact, 320, 900), (short, 320, 240))
    for replay, width, height in setups:
        name = replay.__name__
        with launch(plan, name, width, height) as app:
            before = onscreen_windows(app.proc.pid)
            proof = replay(app, plan)
            after = onscreen_windows(app.proc.pid)
            require(before == after == 0, f'{name} opened a visible Gallery window')
            proof['visible_windows'] = [before, after]
        require(app.proc.poll() is not None, f'{name} did not close its Gallery process')
        result['profiles'][name] = proof
        print(f'PASS {name} {width}x{height}. No visible windows.', flush=True)
    (plan.out / 'native-list-multi-report.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')


if __name__ == '__main__':
    main()
