"""Recheck original frame pixels and the stale anchor in the retained reports."""

from hashlib import sha256
import json
from pathlib import Path

from PIL import Image

root = Path(__file__).resolve().parent


def read(name):
    return json.loads((root / name).read_text())


def bounds(rect):
    return [rect[key] for key in ('x','y','width','height')]


comparison = read('geometry-comparison.json')
paint = comparison['paint_comparison']
for filename in paint['captures']:
    path = root / filename
    assert sha256(path.read_bytes()).hexdigest() == paint['encoded_png_sha256']
    with Image.open(path) as frame:
        assert list(frame.size) == paint['physical_dimensions']
        crop = frame.convert('RGBA').crop(paint['half_open_crop_left_top_right_bottom'])
        assert sha256(crop.tobytes()).hexdigest() == paint['cropped_rgba_sha256']
red = read('push-failure/browser-material-dialog.json')
green = read('pull_request/browser-material-dialog.json')
state = red['state_at_failure']['dialog_demo']['transform']
actual = red['dom_at_failure']
position, = [i for i,node in enumerate(actual['dom']['nodes']) if node.get('stable_id') == comparison['enabled_launcher_id']]
assert not actual['dom']['nodes'][position]['disabled']
assert bounds(actual['layout']['nodes'][position]['bounds']) == comparison['push_launcher_dom_bounds']
assert bounds(actual['layout']['viewport'])[2:] == comparison['current_logical_viewport']
assert bounds(state['anchor']) == comparison['push_cached_anchor']
assert state['progress'] == comparison['transform_progress'] == 0
assert green['full_screen_transform']['normal']['anchor'] == comparison['pr_valid_anchor']
assert red['browser'] == green['browser'] == comparison['same_browser']
assert red['adapter'] == green['adapter'] == comparison['same_adapter']
print('The rendered FAB pixels match. The push report retains the stale anchor.')
