"""Compare Surface Material typography numbers with a pinned AndroidX snapshot."""
import json
import re
from hashlib import sha256
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent
REFERENCE = '268d841a45644cadf438fc335c793869728449ec'
EXPECTED = {
    'TypeScaleTokens.kt': '7ca6378374d78da8a1091d2ee4a4c228f70d0daef65dfd51c18b58f7d9679ba4',
    'TypefaceTokens.kt': 'a1411a760a5d2a1f47c6b698f7e051779566c83b723c13ed698dfae06ef7d36e',
    'FontWeight.kt': '86bc8437557cc879bba5aaaed7f2128f78acf2ab8b40f018b712e6b5ca719560',
}


def token_values(text):
    return dict(re.findall(r'\bval (\w+):[^\n]+\n\s*get\(\) = ([^\n]+)', text))


assert all(sha256((ROOT / name).read_bytes()).hexdigest() == value for name, value in EXPECTED.items())
source = Path(sys.argv[1])
surface_roles = (source / 'crates/surface-material/src/typography.rs').read_text()
surface_weights = (source / 'crates/surface/src/typography.rs').read_text()
official_roles = token_values((ROOT / 'TypeScaleTokens.kt').read_text())
official_weights = token_values((ROOT / 'TypefaceTokens.kt').read_text())
weight_scalars = (ROOT / 'FontWeight.kt').read_text()
aliases = dict(re.findall(r'val (\w+): FontWeight = (W\d+)\b', weight_scalars))
scalars = dict(re.findall(r'val (W\d+): FontWeight = FontWeight\((\d+)\)', weight_scalars))
actual_weights = {name: int(value) for name, value in re.findall(r'const WEIGHT_(\w+): u16 = (\d+);', surface_weights)}
results = []
for role, size, height, tracking, weight in re.findall(r'pub const (\w+): TextStyle = role\(([^,]+), ([^,]+), ([^,]+), (\w+)\);', surface_roles):
    token = ''.join(part.title() for part in role.split('_'))
    actual = (float(size), float(height), float(tracking), actual_weights[weight])
    expected = tuple(float(official_roles[token + key].removesuffix('.sp')) for key in ('Size','LineHeight','Tracking'))
    official_weight = official_roles[token + 'Weight'].removeprefix('TypefaceTokens.')
    expected += (int(scalars[aliases[official_weights[official_weight].removeprefix('FontWeight.')]]),)
    results.append({'role':role, 'surface':actual, 'androidx':expected, 'match':actual == expected})
assert len(results) == len({item['role'] for item in results}) == 30
assert all(item['match'] for item in results)
print(json.dumps({'reference_revision':REFERENCE, 'reference_files':EXPECTED, 'fields':['size','line_height','letter_spacing','weight'], 'roles':results},indent=2))
