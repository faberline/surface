"""Check the two deterministic I11 event-order receipts from stock Showcase."""

import json
from pathlib import Path

def validate(root: Path):
    collapsed = json.loads((root / "browser-ime-cancel-redraw.json").read_text())
    selected = json.loads((root / "browser-ime-cancel-selected-redraw.json").read_text())
    assert collapsed["redraw_width"] == 1001
    for label in ("before", "after"):
        sample = collapsed[label]
        assert sample["browser"] == [0, 2]
        assert sample["trustedSelectAll"] is True
        assert sample["value"] == "台文"
        assert sample["runtime"] == ["台文", "台文", "0", "6", "false"]
    mixed = "中👨‍👩‍👧‍👦e\u0301"
    assert collapsed["replacement"] == [mixed, mixed, "31", "31", "false"]

    canceled, moved, unpainted, repainted = (
        selected[label] for label in ("canceled", "moved", "unpainted", "repainted")
    )
    assert canceled["value"] == "台文"
    assert canceled["browser"] == [1, 2] and canceled["direction"] == "backward"
    assert canceled["runtime"] == ["台文", "台文", "6", "3", "false"]
    assert moved["trustedRight"] is True and moved["browser"] == [2, 2]
    assert moved["value"] == "台文"
    assert canceled["frames"] == moved["frames"] == unpainted["frames"]
    for sample in (unpainted, repainted):
        assert sample["value"] == "台文X"
        assert sample["runtime"] == ["台文X", "台文X", "7", "7", "false"]
        assert sample["browser"] == [3, 3]
    return {"select_all_after_cancel_redraw":collapsed, "backward_cancel_before_redraw":selected}
