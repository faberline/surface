"""Verify that each isolated fault reaches its intended, unchanged DN25 check."""
from hashlib import sha256
import json
from pathlib import Path
import re
import struct
import sys
import zlib

root = Path(__file__).resolve().parent
proof = Path(sys.argv[1]).resolve()
pixel_failure = r'pixel (\d+),(\d+): expected (\[.+?\]), got (\[.+?\])'


def png_reference(path):
    data = path.read_bytes()
    assert data.startswith(b'\x89PNG\r\n\x1a\n'), path
    cursor = 8
    pieces = []
    while cursor < len(data):
        length, kind = struct.unpack_from('>I4s', data, cursor)
        start = cursor + 8
        if kind == b'IHDR':
            width, height, depth, color_type, _, _, interlace = struct.unpack_from('>IIBBBBB', data, start)
            assert (depth, color_type, interlace) == (8, 6, 0), path
        if kind == b'IDAT':
            pieces.append(data[start:start + length])
        cursor = start + length + 4
    # Decode the first two rows to compare the unchanged runner's (1, 1) probe.
    raw = zlib.decompress(b''.join(pieces))
    stride = width * 4
    assert len(raw) == (stride + 1) * height
    previous = [0] * stride
    for y in range(2):
        row = list(raw[y * (stride + 1):(y + 1) * (stride + 1)])
        filter_type = row.pop(0)
        for x, encoded in enumerate(row):
            left = row[x - 4] if x >= 4 else 0
            up = previous[x]
            corner = previous[x - 4] if x >= 4 else 0
            integrated = left + up - corner
            paeth = min((left, up, corner), key=lambda value: abs(integrated - value))
            row[x] = (encoded + (0, left, up, (left + up) // 2, paeth)[filter_type]) % 256
        previous = row
    return {'size': [width, height], 'pixel_1_1': row[4:8], 'sha256': sha256(data).hexdigest()}


def read_scenario(name):
    return ((proof / f'{name}.log').read_text(),
            int((proof / f'{name}-exit.txt').read_text()))


def failed_dn25(name, marker):
    log, exit_status = read_scenario(name)
    diagnostic = (proof / name / 'devtools-native-stderr.log').read_text()
    assert exit_status == 1, (name, exit_status)
    assert 'last step: DN25' in log and '[DN25]' in log and '[DN26]' not in log, log
    assert re.search('case devtools-native .* FAILED', log), log
    assert marker in diagnostic, diagnostic
    return log


healthy, exit_status = read_scenario('baseline')
assert exit_status == 0
checks = re.search(r'case devtools-native .* ok \(7 steps, (\d+) checks,', healthy)
assert checks and '[DN25]' in healthy and '[DN26]' in healthy and '[DN27]' in healthy, healthy
light = png_reference(proof / 'baseline' / 'devtools-native-screenshot.png')
dark = png_reference(proof / 'baseline' / 'devtools-native-screenshot-toggled.png')
assert light['pixel_1_1'] != dark['pixel_1_1']

swap = failed_dn25('swapped-red-blue', 'swapped red and blue in the rendered PNG')
swap_x, swap_y, swap_expected, swap_actual = re.search(pixel_failure, swap).groups()
swap_expected, swap_actual = json.loads(swap_expected), json.loads(swap_actual)
r, g, b, alpha = swap_expected
assert r != b and all(abs(a - b) <= 1 for a, b in zip(swap_actual, [b, g, r, alpha]))

stale = failed_dn25('stale-default-image', 'returned the cached first default PNG')
stale_x, stale_y, stale_expected, stale_actual = re.search(pixel_failure, stale).groups()
stale_expected, stale_actual = json.loads(stale_expected), json.loads(stale_actual)
assert (stale_x, stale_y) == ('1', '1')
assert stale_actual == light['pixel_1_1'] and stale_expected == dark['pixel_1_1']

ignored = failed_dn25('ignored-max-side', 'ignored max_side Some(400)')
dimension = int(re.search(r'max_side downscales: expected 400, got (\d+)', ignored).group(1))
uncapped = png_reference(proof / 'ignored-max-side' / 'devtools-native-screenshot.png')
assert dimension > 400 and dimension == max(uncapped['size'])

report = {
    'frozen_product_manifest_sha256': sha256((root / 'frozen-source.sha256').read_bytes()).hexdigest(),
    'screenshot_assertions_unchanged': True,
    'baseline': {'exit_status': 0, 'steps': 7, 'checks': int(checks.group(1)), 'light': light, 'dark': dark},
    'swapped-red-blue': {'exit_status': 1, 'step': 'DN25', 'pixel': [int(swap_x), int(swap_y)], 'expected': swap_expected, 'actual': swap_actual},
    'stale-default-image': {'exit_status': 1, 'step': 'DN25', 'pixel': [1, 1], 'expected': stale_expected, 'actual': stale_actual},
    'ignored-max-side': {'exit_status': 1, 'step': 'DN25', 'requested': 400, 'actual': dimension},
}
(proof / 'negative-controls.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
print(json.dumps(report, ensure_ascii=False, indent=2))
