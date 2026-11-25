"""Require actual counter snapshots to reject altered semantics and image data."""

from copy import deepcopy
import json
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from outlined_field_rejection_controls import rejected
from verify_counter import DIMENSIONS, NORMAL, OVER_LIMIT, PREEDIT, REPORT, verify


def controls(raw, observed):
    report = json.loads((raw / REPORT).read_text())
    with TemporaryDirectory(prefix="surface-hosted-counter-controls.", dir="/tmp") as directory:
        root = Path(directory)
        target = root / REPORT
        for name in DIMENSIONS:
            copyfile(raw / name, root / name)
        failures = {}

        def alter(name, edit, expected):
            changed = deepcopy(report)
            edit(changed)
            target.write_text(json.dumps(changed, ensure_ascii=False))
            failures[name] = rejected(lambda: verify(root, observed), expected)

        alter("wrong_committed_count", lambda row: row["authored"]["description"].update(ratio="4/5"),
              "ordered passive AX count")
        alter("ordinary_live_announcement", lambda row: row["authored"].update(ordinary_live_messages=["counter"]),
              "ordinary count changes wrote to a live region")
        alter("truncated_overflow", lambda row: row["limits"]["submitted"].update(value="中"),
              "unexpected caller-owned counter model")
        alter("untrusted_preedit_enter", lambda row: row["composition"]["trusted_keys"][1].__setitem__(1, False),
              "trusted separate IME Escape/Enter events missing")
        alter("incorrect_stacked_count", lambda row: row["compact"]["fallback_filled"]["count"].__setitem__(0, 10000),
              "logical rectangle leaves the captured canvas")
        copyfile(raw / REPORT, target)
        copyfile(raw / NORMAL, root / PREEDIT)
        failures["wrong_preedit_origins"] = rejected(lambda: verify(root, observed), "invalid Character Counter PNG size")
        copyfile(raw / PREEDIT, root / PREEDIT)
        copyfile(raw / NORMAL, root / OVER_LIMIT)
        failures["normal_reused_as_overflow"] = rejected(lambda: verify(root, observed), "claimed glyph count disagrees")
        return failures
