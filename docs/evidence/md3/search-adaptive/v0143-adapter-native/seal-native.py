"""Check both rebuilt stock replays and seal the archive without rebuilding."""

import json
from hashlib import sha256
from pathlib import Path

from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parent
SOURCE = '386aec95d1ad1b3fe7dfb08393eb678055eebd895cd8b1a83ae445677987fd9f'
BUNDLE = '9b278d6f0ff1d0e72b6f09f87bb2d3cae50f4431a65a04658044e5c0123267c7'
PROFILES = {
    'docked-light-ltr': [960, 800],
    'compact-dark-rtl-200': [320, 400],
}
TRANSPORTS = ['Page.screenshot', 'CLI', 'MCP']
REFERENCE_MANIFEST = '2355c0cf3ca56ab68e7370e6fa20c23aad3fde7fb8c9c7dbb813b77303b3d6f3'
REFERENCE_STOCK = '8a1e8925fa077d3c1f76a8412b0281209ed87dd8caad16ef1aa9b9167118d6f5'


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text())


def manifest_entries(path):
    items = [line.split(None, 1) for line in path.read_text().splitlines()]
    result = {name: checksum for checksum, name in items}
    assert len(result) == len(items)
    return result


def image_bytes(path, receipt, expected_size):
    assert digest(path) == receipt['sha256']
    with Image.open(path) as image:
        assert image.format == 'PNG' and list(image.size) == expected_size
        assert [receipt['width'], receipt['height']] == expected_size
        image.load()
        return image.convert('RGB')


def verify_selection(before, selected, editor):
    delta = ImageChops.difference(before, selected)
    box = delta.getbbox()
    assert box is not None
    x0, y0, x1, y1 = box
    assert editor['x'] <= x0 < x1 <= editor['x'] + editor['width']
    assert editor['y'] <= y0 < y1 <= editor['y'] + editor['height']
    pixels = delta.crop(box).tobytes()
    changed = sum(1 for i in range(0, len(pixels), 3) if any(pixels[i:i + 3]))
    assert x1 - x0 > 20 and y1 - y0 > 15 and changed > 250
    return {'change_bounds': list(box), 'changed_visible_pixels': changed}


def validate_repeat(repeat, pins, references):
    stock_dir = repeat / 'settled'
    action_dir = repeat / 'interactive'
    sensors = 0
    images = {}
    deltas = {}
    action = read(action_dir / 'native-search-report.json')
    stock = read(stock_dir / 'stock-search-settled-report.json')
    for run in (action, stock):
        assert run['binaries'] == pins and run['test_clock'] == 'unknown_method'
        assert run['transport'] == TRANSPORTS
    for name, profile in zip(PROFILES, action['profiles'], strict=True):
        assert profile['profile'] == name and profile['viewport'] == PROFILES[name]
        assert profile['visible_windows'] == [0] * 5
        sensors += len(profile['visible_windows'])
        assert [record['style'] for record in profile['captures']] == ['contained', 'divided']
        for record in profile['captures']:
            assert record['backward_selection_utf8'] == [6, 0]
            assert record['first_result_after_editor_arrow'] == 'search-doc-design'
            assert record['selected'] == 'search-doc-review'
            assert record['restored_focus'] == 'search-overlay-demo-opener'
            shot = record['shot']
            before = image_bytes(action_dir / shot['name'], shot, PROFILES[name])
            images[shot['name']] = shot['sha256']
            selection = record['selection_shot']
            if selection:
                after = image_bytes(action_dir / selection['name'], selection, PROFILES[name])
                deltas[selection['name']] = verify_selection(before, after, record['geometry']['editor'])
                images[selection['name']] = selection['sha256']
    for name, profile in zip(PROFILES, stock['profiles'], strict=True):
        assert profile['profile'].startswith(name + '-') and profile['visible_window_count'] == 0
        sensors += 1
        shot = profile['shot']
        path = stock_dir / shot['name']
        frame_name = profile['profile'] + '-opening-settled.png'
        image_bytes(path, shot, PROFILES[name])
        assert path.read_bytes() == (ROOT / 'prior-motion-reference' / frame_name).read_bytes()
        assert profile['editor_evidence'] == references[profile['profile']]
        images[shot['name']] = shot['sha256']
    assert len(images) == 8 and len(deltas) == 2 and sensors == 12
    return {'pngs': images, 'backward_selection_pixels': deltas, 'zero_window_observations': sensors}


def main():
    inputs = ROOT / 'inputs'
    assert digest(inputs / 'incoming-source.sha256') == SOURCE
    assert digest(inputs / 'browser-binary-sha256.txt') == BUNDLE
    assert manifest_entries(inputs / 'incoming-source.sha256') == manifest_entries(inputs / 'browser-source.sha256')
    for phase in ('before', 'after'):
        assert len((inputs / f'source-{phase}.log').read_text().splitlines()) == 557
    for group in ('source', 'stage', 'bundle'):
        assert (inputs / f'{group}-before.log').read_bytes() == (inputs / f'{group}-after.log').read_bytes()
    assert (inputs / 'repeat-diff.log').stat().st_size == 0
    binary_manifest = manifest_entries(inputs / 'browser-binary-sha256.txt')
    pins = {kind: binary_manifest[path] for kind, path in {
        'gallery': 'debug/surface-material-gallery', 'cli': 'debug/surface-devtools',
    }.items()}
    prior = ROOT / 'prior-motion-reference'
    prior_receipt = prior / 'repeat.sha256'
    assert digest(prior_receipt) == REFERENCE_MANIFEST
    assert digest(prior / 'exact-stock-comparison.json') == REFERENCE_STOCK
    for path in [*prior.glob('*-opening-settled.png'), prior / 'native-search-motion-report.json']:
        assert digest(path) == manifest_entries(prior_receipt)['motion-1/' + path.name]
    motion_report = read(prior / 'native-search-motion-report.json')
    refs = {item['name']: item['frames']['opening-settled']['editor_evidence']
            for item in motion_report['profiles']}
    first, second = (validate_repeat(ROOT / f'repeat-{n}', pins, refs) for n in (1, 2))
    assert first == second
    receipt = {
        'source_manifest_sha256': SOURCE, 'source_files': 557, 'stock_binary_manifest_sha256': BUNDLE,
        'native_pins': pins, 'stock_clock_reply': 'unknown_method', 'native_api_transports_byte_equal': TRANSPORTS,
        'repeat_pngs_and_reports_byte_equal': True, 'repeats': 2, 'visible_windows': 0,
        'match_prior_motion_settled_endpoints': True, 'prior_motion_manifest_sha256': REFERENCE_MANIFEST,
        'per_repeat': first,
    }
    (ROOT / 'verified-native.json').write_text(json.dumps(receipt, indent=2) + '\n')
    paths = sorted(path for path in ROOT.rglob('*') if path.is_file() and path.name != 'SHA256SUMS')
    (ROOT / 'SHA256SUMS').write_text(''.join(f'{digest(path)}  {path.relative_to(ROOT)}\n' for path in paths))
    print(f'PASS 16 PNGs decoded, 24 zero-window observations, two exact stock endpoints; {len(paths)} files sealed')


if __name__ == '__main__':
    main()
