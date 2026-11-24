#!/usr/bin/env python3
"""Independently verify stock Dialog JSON and PNGs on the run's actual WebGPU adapter."""

from collections import Counter
from hashlib import sha256
import json
import math
from pathlib import Path
import re
from struct import unpack
import sys

from PIL import Image, ImageChops

DIALOG_STEPS = tuple(f'M{n}' for n in range(256, 264))


def validate(attempt, browser, adapter, *, log=None):
    """Pass the browser and adapter independently observed by the stock gate.

    The caller verifies exact pins, the overall case result and owned processes.
    A focused result also works for local validator testing. Here we require
    the ordered Dialog steps, their behavioral receipt, and all direct pixels.
    """
    attempt = Path(attempt)
    log = (attempt / 'runner.log').read_text() if log is None else log
    material_steps = tuple(re.findall(r'^  \[(M\d+)\]', log, re.M))
    actual_steps = tuple(step for step in material_steps if step in DIALOG_STEPS)
    assert actual_steps == DIALOG_STEPS, actual_steps

    report = json.loads((attempt / 'browser-material-dialog.json').read_text())
    assert report['errors'] == [] and 'failure' not in report
    expected_adapter = {key: adapter[key] for key in ('isFallbackAdapter', 'vendor', 'architecture')}
    assert isinstance(expected_adapter['isFallbackAdapter'], bool)
    assert report['adapter'] == expected_adapter, (report['adapter'], expected_adapter)
    assert report['browser'] == browser, (report['browser'], browser)
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
        'rtl-200-edit': (640,888), 'rtl-200-discard-before': (640,888),
        'rtl-200-discard-after': (640,888), 'rtl-200-discard-unclipped': (640,1128),
        'alert': (1920,1528), 'edit-compact': (1198,1088), 'edit-wide': (1200,1088), 'dirty-confirm': (640,888),
    }
    basic_phases = {
        'rest':'rest', 'enter-zero':'enter_zero', 'entering':'entering', 'open':'opaque',
        'exit-start':'exit_start', 'exiting':'exiting', 'reversed':'reversed', 'closed':'closed',
        'reduced-open':None, 'reduced-closed':None,
    }
    nested_phases = {'edit':'initial','entering':'entering','open':'covered',
                     'exit-start':'exit_start','exiting':'exiting','closed':'closed'}
    expected.update({f'motion-basic-{phase}':(1920,1528) for phase in basic_phases})
    expected.update({f'motion-nested-{phase}':(640,888) for phase in nested_phases})
    transform_phases = ['rest','start','entering','open','exit-start','exiting','reversed','closed','reduced-open','reduced-closed']
    expected.update({f'transform-{phase}':(640,888) for phase in transform_phases})
    expected['transform-adaptive-full-preedit'] = (1198,1088)
    expected['transform-adaptive-basic-preedit'] = (1200,1088)
    for phase in ('before','after'):
        expected[f'transform-clearance-short-{phase}'] = (640,568)
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

    rtl = report['rtl_short']
    assert rtl['viewport'] == [0, 0, 320, 400] and rtl['text_scale'] == 2 and rtl['rtl']
    rtl_title = rtl['title']
    discard = rtl['discard']
    assert discard['description'] == '未儲存的行程變更會遺失。'
    assert discard['focus_during_wheel'] == 'dialog-demo-discard-cancel'
    assert discard['continue_focus'] == 'dialog-demo-editor'


    def rtl_capture(name):
        return Image.open(attempt / f'material-dialog-adaptive-rtl-200-{name}.png').convert('RGB')


    def line_mask(frame, rect, pitch, text_row, rgb, clip):
        x, y, width, _ = rect
        left, top = round(2 * x), round(2 * (y + pitch * text_row + 44))
        clip_x, clip_y, clip_w, clip_h = clip
        x0, x1 = round(2 * clip_x), round(2 * (clip_x + clip_w))
        y0, y1 = round(2 * (clip_y + 44)), round(2 * (clip_y + clip_h + 44))
        visible = (max(left, x0), max(top, y0), min(left + round(width * 2), x1), min(top + round(pitch * 2), y1))
        x0, y0, x1, y1 = visible
        if x0 >= x1 or y0 >= y1:
            return set()
        pixels = frame.crop(visible).get_flattened_data()
        return {(x0-left+index%(x1-x0), y0-top+index//(x1-x0)) for index,pixel in enumerate(pixels)
                if max(abs(a-b) for a,b in zip(pixel,rgb)) <= 24}


    edit_png = rtl_capture('edit')
    edit_lines = []
    for row in (0,1):
        title_mask = line_mask(edit_png, rtl_title['bounds'], rtl_title['pitch'], row, rtl_title['rgb'], rtl_title['bounds'])
        assert len(title_mask) > 100
        edit_lines.append([min(x for x,_ in title_mask), max(x for x,_ in title_mask)])
    assert edit_lines == rtl_title['line_ink_pixels_x'] and edit_lines[0][1] == edit_lines[1][1]
    assert edit_lines[1][0] > edit_lines[0][0] + 24
    before_rtl, after_rtl, tall_rtl = (rtl_capture(f'discard-{stage}') for stage in ('before','after','unclipped'))
    description_masks = []
    for shot,stage,clip in [(before_rtl,'before',discard['body']), (after_rtl,'after',discard['body']), (tall_rtl,'unclipped',discard['unclipped_body'])]:
        description_masks.append(line_mask(shot, discard[f'support_bounds_{stage}'], discard['support_pitch'], 1, discard['support_rgb'], clip))
    assert not description_masks[0]
    assert description_masks[1] and description_masks[1] == description_masks[2]
    assert [len(mask) for mask in description_masks] == [discard[k] for k in ('last_line_ink_before', 'last_line_ink_after', 'unclipped_ink')]
    rtl_pinned = {}
    for name, rect in zip(('heading','cancel','confirm'), [discard['heading'], *discard['actions']]):
        first, last = crop(before_rtl, rect), crop(after_rtl, rect)
        ink = Counter(pixel for pixel in first.get_flattened_data() if min(pixel) > 150).most_common(1)[0][0]
        start = color_mask(first, ink)
        assert len(start) > 500 and start == color_mask(last, ink), name
        rtl_pinned[name] = {'rgb':ink,'unchanged_ink_pixels':len(start)}

    motion = report['motion']
    assert motion['manual_performance_time']
    normal = motion['basic']
    nested = motion['nested']
    reduced = motion['reduced']
    assert normal['exit_ax_absent'] and normal['trusted_pass_through_reopened']
    assert normal['enter_scope'] == 'dialog-demo-basic-confirm' and normal['exit_scope'] == 'dialog-demo-basic-open'
    assert nested['upper_ax_absent_during_exit'] and nested['live_lower_editor'] == 'dialog-demo-editor'
    assert nested['draft_after_trusted_input'] == '秋日散步！'
    assert reduced['clock_ms'] == 0 and reduced['exact_endpoints']
    expected_alpha = 1 - 2.6 * math.exp(-1.6)
    for profile in (normal,nested):
        assert abs(profile['entry_40ms'] - expected_alpha) < .002
        assert abs(profile['exit_40ms'] - (1-expected_alpha)) < .002


    def motion_pixels(phase, points):
        with Image.open(attempt / f'material-dialog-adaptive-motion-{phase}.png') as original:
            image = original.convert('RGB')
            return [list(image.getpixel((math.floor(x*2),math.floor((y+44)*2)))) for x,y in points]


    for phase, target in basic_phases.items():
        pixels = motion_pixels(f'basic-{phase}', normal['sample_points'])
        expected = normal['pixels'][target] if target else reduced[phase.removeprefix('reduced-')]
        assert pixels == expected, (phase,pixels,expected)
    for phase, target in nested_phases.items():
        pixels = motion_pixels(f'nested-{phase}', [nested['sample_point']])[0]
        assert pixels == nested['scrim'][target], (phase,pixels,nested['scrim'][target])
    assert normal['pixels']['enter_zero'] == normal['pixels']['rest'] == normal['pixels']['closed'] == reduced['closed']
    assert normal['pixels']['opaque'] == normal['pixels']['exit_start'] == reduced['open']
    assert normal['pixels']['exiting'] == normal['pixels']['reversed']
    for phase in ('entering','exiting'):
        outside, inside = normal['pixels'][phase]
        assert all(a>b>c for a,b,c in zip(normal['pixels']['rest'][0],outside,normal['pixels']['opaque'][0]))
        assert all(a<b<c for a,b,c in zip(outside,inside,normal['pixels']['opaque'][1]))
        assert all(a>b>c for a,b,c in zip(nested['scrim']['initial'],nested['scrim'][phase],nested['scrim']['covered']))

    full_screen = report['full_screen_transform']
    assert full_screen['manual_performance_time']
    normal = full_screen['normal']
    reduced_transform = full_screen['reduced']
    adaptive_transform = full_screen['adaptive']
    assert normal['anchor'] == [24,320,56,56]
    assert normal['start']['progress'] == 0 and not normal['start']['retiring']
    assert normal['exit']['retiring'] and normal['paint_retired_after_focus_return']
    assert normal['legacy_launchers_absent']
    assert normal['trusted_reverse'] and not normal['reversal']['retiring']
    assert normal['exit']['progress'] == normal['reversal']['progress']
    assert normal['exit']['container'] == normal['reversal']['container']
    for phase in ('entry','exit'):
        pose = normal[phase]
        fraction=pose['progress']
        assert .2 < fraction < .9
        for index,key in enumerate(('x','y','width','height')):
            anchored=normal['anchor'][index]
            goal=[0,0,320,400][index]
            assert abs(pose['container'][key] - (anchored+(goal-anchored)*fraction)) < .015


    def transform_pixels(stage, points):
        with Image.open(attempt / f'material-dialog-adaptive-transform-{stage}.png') as original:
            image=original.convert('RGB')
            return [list(image.getpixel((math.floor(x*2),math.floor((y+44)*2)))) for x,y in points]


    for phase in ('rest','start'):
        assert transform_pixels(phase,[normal['origin_point']])[0] == normal['origin_pixel']
    for sample_key,phase in [('entry_pixels','entering'),('exit_pixels','exiting')]:
        samples=normal[sample_key]
        assert transform_pixels('rest',samples['points']) == samples['rest']
        assert transform_pixels(phase,samples['points']) == samples['frame']
        assert sum(abs(a-b) for a,b in zip(samples['frame'][0],samples['rest'][0])) >= 12
        assert samples['frame'][1:] == samples['rest'][1:]
    assert transform_pixels('reversed',[normal['exit_pixels']['points'][0]])[0] == normal['exit_pixels']['frame'][0]
    for phase in ('open','exit-start','reduced-open'):
        assert transform_pixels(phase,[normal['field_probe']])[0] == normal['field_color']
    assert transform_pixels('closed',[[4,390]]) == transform_pixels('rest',[[4,390]])
    assert isinstance(reduced_transform['resting_anchor_available'], bool)
    assert reduced_transform['clock_ms'] == 0 and reduced_transform['exact_endpoints']
    assert reduced_transform['field_color'] == normal['field_color']
    assert adaptive_transform['widths'] == [599,600,599]
    assert adaptive_transform['draft'] == '秋日散步' and adaptive_transform['saved'] == '週末行程'
    assert adaptive_transform['same_textarea'] and adaptive_transform['preedit']['runtime']['composing']
    assert adaptive_transform['selection']['direction'] == 'backward'
    assert adaptive_transform['selection']['range'] == [3,4]
    short_preview = full_screen['short_clearance']
    assert short_preview['viewport'] == [320,240] and short_preview['text_scale'] == 2
    assert short_preview['clip_gap'] == 8 and short_preview['physical_fab_opens']
    scroll = short_preview['scroller']
    fab = short_preview['fab']
    start_status = short_preview['saved_before']
    visible_status = short_preview['saved_after']
    assert scroll[1] + scroll[3] + 8 == fab[1] and fab[2:] == [56,56]
    assert start_status[1]+start_status[3] > scroll[1]+scroll[3]
    assert scroll[1] <= visible_status[1] and visible_status[1]+visible_status[3] <= scroll[1]+scroll[3]
    assert short_preview['saved_solid_ink'] >= 20
    with Image.open(attempt/'material-dialog-adaptive-transform-clearance-short-after.png') as source:
        after_scroll = source.convert('RGB')
    with Image.open(attempt/'material-dialog-adaptive-transform-clearance-short-before.png') as source:
        before_scroll = source.convert('RGB')
    assert Counter(crop(after_scroll,visible_status).get_flattened_data())[tuple(short_preview['saved_rgb'])] == short_preview['saved_solid_ink']
    assert crop(after_scroll,fab).tobytes() == crop(before_scroll,fab).tobytes()


    return {'browser': browser, 'adapter': expected_adapter, 'steps': list(DIALOG_STEPS), 'page_errors': 0,
              'short_canvas':{'pinned_ink':pinned_ink,'changed_body_pixels':changed_body_pixels},
              'rtl_200':{'title_line_pixels_x':edit_lines,'title_right_dp':rtl_title['bounds'][0]+edit_lines[0][1]/2,
                         'last_line_pixels':[len(mask) for mask in description_masks],
                         'last_line_masks_equal':True,'pinned_ink':rtl_pinned},
              'motion':motion,'full_screen_transform':full_screen,'pngs':pngs}


if __name__ == '__main__':
    stage, adapter_receipt = map(Path, sys.argv[1:3])
    observed = json.loads(adapter_receipt.read_text())
    result = validate(stage, observed['browser'], observed['adapter'])
    print(json.dumps(result, ensure_ascii=False, indent=2))
