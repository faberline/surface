"""Read back the paired result without starting Chrome."""

from hashlib import file_digest
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
RUNS = ("v0165-negative-r2", "v0166-positive")
MANIFESTS = ("v0165-web.sha256", "v0166-r2-web.sha256")


def require(condition, detail):
    if not condition:
        raise ValueError(detail)


def read(path):
    return json.loads(path.read_text())


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def load(name, manifest):
    root = ROOT / name
    verified = read(root / "verified.json")
    receipt = root / "headless-window-observation.json"
    report = root / "textbox-values.json"
    invocation = read(root / "invocation.json")
    require(verified["report_sha256"] == sha(report)
            and verified["window_receipt_sha256"] == sha(receipt)
            and sha(ROOT / manifest) == verified["asset_manifest_sha256"] == invocation["manifest_sha256"]
            and sha(ROOT / "probe.mjs") == invocation["probe_sha256"], "paired evidence seal differs")
    result = read(report)
    window = read(receipt)
    require(result["pageErrors"] == [] and window["stock_exit"] == 0
            and window["visible_owned_windows"] == window["remaining_observed_processes"] == window["problems"] == []
            and verified["window_samples"] == window["launch_observations"]["chrome-primary"]["samples"] > 0,
            "page, guard or window evidence failed")
    return verified, result


def main():
    (old, baseline), (new, corrected) = [load(name, manifest) for name, manifest in zip(RUNS, MANIFESTS)]
    require(old["expect"] == "missing" and new["expect"] == "present"
            and old["browser"] == new["browser"] and old["adapter"] == new["adapter"]
            and old["value_samples"] == new["value_samples"] == 6, "wrong comparison scope")
    pairs = list(zip(baseline["inactive"], corrected["inactive"], strict=True))
    require(all(before["id"] == after["id"] and before["expected"] == after["expected"]
                and before["value"] is None and after["value"] == after["expected"] == after["hiddenCarrier"]
                for before, after in pairs), "the corrected values differ from their authored strings")
    for phase, before in baseline["phases"].items():
        after = corrected["phases"][phase]
        require(before["dom"]["focus"] == after["dom"]["focus"], "focus path changed")
        for field, first in before["fields"].items():
            second = after["fields"][field]
            require(all(first[key] == second[key] for key in ("name", "description", "multiline", "disabled", "readonly")),
                    f"{phase} {field} changed non-value semantics")
    value = pairs[2][1]["value"]
    require(value.startswith("  ") and "  \n" in value and value.endswith("\n") and "\u0301" in value,
            "the committed fixture lost a whitespace or combining-mark edge")
    print(json.dumps({"scope": new["scope"], "browser": new["browser"], "adapter": new["adapter"],
        "compared_inactive_samples": len(pairs), "old_values_missing": 6, "corrected_values_exact": 6,
        "preserved_names_states_and_descriptions": True, "committed_value": value,
        "zero_visible_windows": True, "observed_survivors": 0, "reports": {RUNS[0]: old, RUNS[1]: new}},
        indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
