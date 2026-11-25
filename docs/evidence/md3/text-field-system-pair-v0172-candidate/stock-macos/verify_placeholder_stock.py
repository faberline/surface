"""The original placeholder pixel proof stays unchanged in stock v0.1.70."""

from pathlib import Path
from runpy import run_path

from evidence_common import normalized, require
from stock_config import PLACEHOLDER_PNGS

CHECK = Path(__file__).resolve().parent / "placeholder-proof/verify-placeholder.py"


def verify(artifacts, metadata):
    result = normalized(run_path(str(CHECK))["verify"](artifacts))
    require(result["browser"] == metadata["browser"] and result["adapter"] == metadata["adapter"]
            and len(result["pngs"]) == PLACEHOLDER_PNGS, "Stock placeholder browser, adapter or originals differ")
    return result
