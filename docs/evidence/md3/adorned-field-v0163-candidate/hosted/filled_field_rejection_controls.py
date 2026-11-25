"""Rejection controls derived from original Filled Text Field artifacts."""

from copy import deepcopy
import json
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from outlined_field_rejection_controls import rejected
from verify_filled_field import DIMENSIONS, HOVER, IDLE, PREEDIT, REPORT, verify


def controls(raw, observed):
    report = json.loads((raw / REPORT).read_text())
    with TemporaryDirectory(prefix="surface-hosted-filled-field-controls.", dir="/tmp") as temporary:
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

        alter("composing_enter_untrusted",
              lambda row: row["composition"]["enter_keys"][0].update(trusted=False),
              "missing separate trusted composing and noncomposing Enter events")
        alter("selection_lost", lambda row: row["composition"].update(after_selection=["中文", 0, 0]),
              "composition or selection was lost on actual reflow")
        alter("missing_error_link", lambda row: row["states"]["description"]["dom"].update(linked=False),
              "editor description or invalid state lost")
        alter("wrong_blend", lambda row: row["paint"]["hover"]["fill"].update(encoded_mix=[0, 0, 0]),
              "incorrect reported blend computation")
        alter("wrong_indicator_width", lambda row: row["paint"]["focused"]["indicator"].update(width_dp=1),
              "incorrect Filled Text Field bottom indicator width")
        copyfile(raw / REPORT, target)
        copyfile(raw / IDLE, root / HOVER)
        failures["idle_duplicated_as_hover"] = rejected(lambda: verify(root, observed), "unverified pixel trace")
        copyfile(raw / HOVER, root / HOVER)
        copyfile(raw / IDLE, root / PREEDIT)
        failures["wrong_preedit_image_size"] = rejected(lambda: verify(root, observed), "invalid Filled Text Field PNG size")
        return failures
