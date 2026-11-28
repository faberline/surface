"""Read the actual embedded font assets with FontTools 4.66.1."""
import json
from hashlib import file_digest
from pathlib import Path
import sys

import fontTools
from fontTools.ttLib import TTFont

assert fontTools.__version__ == '4.66.1'
assets = Path(sys.argv[1]) / 'crates/surface-wgpu/assets'
fonts = {
    'native_regular': 'NotoSansCJKtc-Regular.otf',
    'native_bold': 'NotoSansCJKtc-Bold.otf',
    'web_regular': 'NotoSansCJKtc-Subset.otf',
    'web_bold': 'NotoSansCJKtc-Bold-Subset.otf',
}
results, cmaps = {}, {}
for name, filename in fonts.items():
    path = assets / filename
    with TTFont(path, lazy=True) as font, path.open('rb') as stream:
        cmap = set(font.getBestCmap())
        cmaps[name] = cmap
        weight = 700 if name.endswith('bold') else 400
        assert font['OS/2'].usWeightClass == weight
        assert font['name'].getDebugName(1) == 'Noto Sans CJK TC'
        assert set(map(ord, 'Surface 繁體中文ㄅㄆㄇㄩ')).issubset(cmap)
        assert (ord('龘') in cmap) == name.startswith('native')
        results[name] = {'file':filename, 'weight':weight, 'bytes':path.stat().st_size,
                         'sha256':file_digest(stream, 'sha256').hexdigest(), 'cmap_count':len(cmap)}
assert cmaps['native_regular'] == cmaps['native_bold']
assert cmaps['web_regular'] == cmaps['web_bold'] < cmaps['native_bold']
assert (len(cmaps['native_bold']), len(cmaps['web_bold'])) == (44810, 5997)
print(json.dumps({'fonttools':fontTools.__version__, 'fonts':results, 'matching_weight_coverage':True,
                  'native_has_u9f98':True, 'web_has_u9f98':False}, indent=2))
