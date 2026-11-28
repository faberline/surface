"""Validate and record the corrected tree's full workspace completion."""

from pathlib import Path
import sys

from evidence_common import json_file, require, tests
from pins import SOURCE_SHA, SOURCE_TREE

out = Path(sys.argv[1])
log, counts = tests(out / "check.log")
wasm = log.rsplit("\ntest result:", 1)[-1].count("Finished `dev` profile")
require(counts["passed"] == 1607 and counts["ignored"] == 20 and "provenance:" in log
        and wasm == 3, "Expected the corrected workspace and three Wasm checks")
json_file(out / "verified-workspace.json", {"source_tree": SOURCE_TREE, "source_sha256": SOURCE_SHA,
                                           "command": ["sh", "scripts/check.sh"], "wasm_checks": wasm, **counts})
