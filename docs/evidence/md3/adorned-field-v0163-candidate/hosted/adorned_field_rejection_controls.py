"""Rejection controls derived from original Text Field Adornments artifacts."""

from copy import deepcopy
import json
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from outlined_field_rejection_controls import rejected
from verify_adorned_field import DIMENSIONS, IDLE, FILLED, PREEDIT, REPORT, verify


def controls(raw, observed):
    report = json.loads((raw / REPORT).read_text())
    with TemporaryDirectory(prefix="surface-hosted-adorned-field-controls.", dir="/tmp") as temporary:
        root = Path(temporary)
        for name in DIMENSIONS:
            copyfile(raw / name, root / name)
        target = root / REPORT
        failures = {}

        def alter(name, edit, expected):
            changed = deepcopy(report)
            edit(changed)
            target.write_text(json.dumps(changed, ensure_ascii=False))
            failures[name] = rejected(lambda: verify(root, observed), expected)

        alter("reordered_description", lambda row: row["presentations"]["outlined"]["description"]["dom"]["names"].reverse(),
              "ordered AX description or separate named action differs")
        alter("untrusted_action", lambda row: row["actions"]["trusted_events"]["pointer"][0].__setitem__(1, False),
              "missing trusted keyboard or pointer sequence")
        alter("early_release", lambda row: row["actions"]["held"].update(actions=3),
              "unexpected caller-owned field state")
        alter("stale_preedit_offsets", lambda row: row["composition"]["preedit"].update(selection_anchor=1),
              "wrong live preedit text or range")
        alter("claimed_ghost_icon", lambda row: row["compact"]["fallback"]["omitted_icon"].update(ink=1),
              "claimed glyph count disagrees")
        copyfile(raw / REPORT, target)
        copyfile(raw / IDLE, root / FILLED)
        failures["idle_duplicated_as_filled"] = rejected(lambda: verify(root, observed), "claimed glyph count disagrees")
        copyfile(raw / FILLED, root / FILLED)
        copyfile(raw / IDLE, root / PREEDIT)
        failures["wrong_preedit_image_size"] = rejected(lambda: verify(root, observed), "invalid Adorned Text Field PNG size")
        return failures
