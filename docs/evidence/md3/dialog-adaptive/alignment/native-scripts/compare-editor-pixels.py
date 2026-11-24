"""Compare the editor's fully visible pixel rectangle between two sealed Dialog replays."""

import hashlib
import json
from pathlib import Path
import sys

from gallery_transport import read_png, require
from native_runtime import digest


before, after = map(Path, sys.argv[1:])
report_name = "native-dialog-report.json"
original = {
    item["profile"]: item
    for item in json.loads((before / report_name).read_text())["profiles"]
}
compared = []
for item in json.loads((after / report_name).read_text())["profiles"]:
    old_edit = original[item["profile"]]["edit"]
    rect = item["edit"]["geometry"]["field"]
    require(rect == old_edit["geometry"]["field"], "Editor bounds changed")
    old_rows = read_png((before / old_edit["capture"]["name"]).read_bytes())[2]
    new_rows = read_png((after / item["edit"]["capture"]["name"]).read_bytes())[2]
    x, y, width, height = (int(rect[key]) for key in ("x", "y", "width", "height"))
    old = b"".join(row[4 * x:4 * (x + width)] for row in old_rows[y:y + height])
    new = b"".join(row[4 * x:4 * (x + width)] for row in new_rows[y:y + height])
    require(len(old) == width * height * 4 and old == new, "Editor pixels changed")
    compared.append({"profile": item["profile"], "bounds": rect, "rgba_sha256": hashlib.sha256(new).hexdigest()})
print(json.dumps({
    "schema": "surface.test.dialog-alignment-editor-pixel-parity.v1",
    "before_report_sha256": digest(before / report_name),
    "after_report_sha256": digest(after / report_name),
    "editor_crops": compared,
}, indent=2))
