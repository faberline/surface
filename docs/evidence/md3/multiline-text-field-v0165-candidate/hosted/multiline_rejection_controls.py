"""Require exact multiline evidence to reject changed operations and images."""

from copy import deepcopy
import json
from pathlib import Path
from shutil import copyfile
from tempfile import TemporaryDirectory

from outlined_field_rejection_controls import rejected
from verify_multiline import REPORT, TAGS, filename, verify


def controls(raw, metadata):
    original = json.loads((raw / REPORT).read_text())
    with TemporaryDirectory(prefix="surface-hosted-multiline-controls.", dir="/tmp") as directory:
        root = Path(directory)
        target = root / REPORT
        for tag in TAGS:
            copyfile(raw / filename(tag), root / filename(tag))
        failures = {}

        def alter(name, edit, message):
            changed = deepcopy(original)
            edit(changed)
            target.write_text(json.dumps(changed, ensure_ascii=False))
            failures[name] = rejected(lambda: verify(root, metadata), message)

        alter("enter_submitted", lambda row: row["outlined"]["enter_model"].update(submits=1),
              "unexpected multiline caller state or separate submit count")
        alter("lost_preedit_selection", lambda row: row["filled_ime"]["after_cancel"]["web"].__setitem__(0, 0),
              "composition cancellation did not restore the backward committed selection")
        alter("changed_wheel_delta", lambda row: row["wheel"]["trusted"][0].update(dy=49),
              "trusted wheel deltas or inner-first scroll differ")
        alter("lost_page_key_selection", lambda row: row["page_keys"]["backward"]["web"].__setitem__(2, "forward"),
              "textarea selection offsets or direction differ")
        alter("untrusted_touch", lambda row: row["touch"]["trace"][0].update(trusted=False),
              "untrusted or foreign touch event")
        alter("disabled_in_tab_order", lambda row: row["locked_ax"].update(skipped_disabled=False),
              "locked editor focus or committed newlines differ")
        copyfile(raw / REPORT, target)
        copyfile(raw / filename("wide-outlined"), root / filename("compact-filled"))
        failures["wrong_compact_png"] = rejected(lambda: verify(root, metadata), "wrong Multiline PNG format or dimensions")
        copyfile(raw / filename("compact-filled"), root / filename("compact-filled"))
        copyfile(raw / filename("wheel-before"), root / filename("wheel-inner"))
        failures["missing_wheel_translation"] = rejected(lambda: verify(root, metadata),
                                                         "wheel PNGs do not reproduce the translated inner rows")
        return failures
