"""Reproduce both failed endpoint samples from original PNG pixels."""

from hashlib import sha256
import json
from pathlib import Path

from PIL import Image
from verify_divider_report import PROFILES, read_profile

root = Path(__file__).resolve().parent
proof = json.loads((root/'combined-failure.json').read_text())
metal = json.loads((root/'metal-browser-material-divider.json').read_text())['compact_heavy']
image_name = 'material-divider-compact-dark-rtl-200-heavy.png'
expected = (73,69,78)
for event,receipt in proof['events'].items():
    original = root/event/image_name
    assert sha256(original.read_bytes()).hexdigest() == receipt['pixels']['compact_heavy_png_sha256']
    verified = read_profile(root/event,metal,PROFILES['compact_heavy'])
    assert verified == receipt['pixels']['portable_strict_interior_adjacency_check']
    with Image.open(original) as png:
        frame = png.convert('RGB')
    for name,info in receipt['pixels']['shape_pixels'].items():
        x,y,w,h = info['logical_geometry']
        for inset,field in ((1,'original_endpoint_probe_failures'),(1.5,'inset_1_5dp_endpoint_probe_failures')):
            misses=[]
            for depth in (index+.5 for index in range(8)):
                coords = ([(x+depth,row) for row in (y+inset,y+h/2,y+h-inset)] if name=='vertical'
                          else [(col,y+depth) for col in (x+inset,x+w/2,x+w-inset)])
                for col,row in coords:
                    point = [int(col*2),int((row+44)*2)]
                    rgb = frame.getpixel(tuple(point))
                    if rgb != expected:
                        misses.append({'logical':[col,row],'physical':point,'rgb':list(rgb)})
            assert misses == info[field]
    print(f'{event}: original endpoints hit four corner pixels, revised probes and strict interior checks pass')
