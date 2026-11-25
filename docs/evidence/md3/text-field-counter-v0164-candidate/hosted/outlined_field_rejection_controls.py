"""Rejection controls derived from original Outlined Text Field artifacts."""

from copy import deepcopy
import json
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from verify_outlined_field import DIMENSIONS, FOCUSED, IDLE, PREEDIT, REPORT, verify


def rejected(operation, expected):
    try:
        operation()
    except (ValueError, RuntimeError) as error:
        if expected not in str(error):
            raise AssertionError(f"Unexpected rejection: {error}") from error
        return str(error)
    raise AssertionError(f"Altered Outlined Text Field evidence passed: {expected}")


def controls(raw, observed):
    report = json.loads((raw / REPORT).read_text())
    with TemporaryDirectory(prefix="surface-hosted-outlined-field-controls.", dir="/tmp") as temporary:
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

        alter("duplicate_validation", lambda row: row["states"]["assertive_messages"].append(row["states"]["assertive_messages"][0]),
              "assertive validation deliveries")
        alter("first_enter_submits", lambda row: row["composition"]["first_enter"].update(submits=1),
              "unexpected caller-owned editor state")
        alter("selection_lost", lambda row: row["composition"].update(after_selection=["中文", 0, 0]),
              "synthetic preedit or its offsets")
        alter("missing_error_link", lambda row: row["states"]["description"]["dom"].update(linked=False),
              "editor description or invalid state lost")
        alter("missing_focus_restoration", lambda row: row["compact"].update(back_focus="foreign-editor"),
              "compact Back focus was not restored")
        copyfile(raw / REPORT, target)
        copyfile(raw / IDLE, root / FOCUSED)
        failures["idle_duplicated_as_focus"] = rejected(lambda: verify(root, observed), "unverified pixel trace")
        copyfile(raw / FOCUSED, root / FOCUSED)
        copyfile(raw / IDLE, root / PREEDIT)
        failures["wrong_preedit_image_size"] = rejected(lambda: verify(root, observed), "invalid PNG size")
        return failures
