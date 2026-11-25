"""Read the full q66 focused output with the original q65 paint verifier."""

from hashlib import file_digest
import json
from pathlib import Path
import re
from runpy import run_path
import sys
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parent
PRIOR = Path("/tmp/surface-md3-multiline-stock-v0165.C31fd1AG")
REPORT = "browser-material-multiline-text-field.json"
STEPS = list(map(str, range(341, 347)))
WEB_SHA = "d37470c46d9b81fed27eab062b137633039bcbbf0712afee3d7047dd9b86a797"


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(path):
    with path.open("rb") as file:
        return file_digest(file, "sha256").hexdigest()


def read(path):
    return json.loads(path.read_text())


def verify_inactive(data):
    fixed = "第一行😀\n第二行 á\n第三行 ABC 中文文字\n第四行\n第五行\n第六行\n第七行\n第八行\n"
    expected = {
        "before": ("多行備註", "第一行", True),
        "fixed": ("詳細說明", fixed, True),
        "after": ("多行備註", "第一行\n修訂 á", True),
        "readonly": ("唯讀內容", "已儲存的第一行\n已儲存的第二行\n", True),
        "disabled": ("停用內容", "停用的第一行\n停用的第二行", True),
        "single_line": ("標籤名稱", "中á👍🏽", False),
    }
    require(set(data) == {*expected, "active"}, "missing q66 AX snapshots")
    for name, (label, text, multiline) in expected.items():
        node = data[name]
        props = {part["name"]: part["value"]["value"] for part in node["properties"]}
        require(node["ignored"] is False and node["role"]["value"] == "textbox"
                and node["name"]["value"] == label and node["value"]["value"] == text
                and props["multiline"] is multiline and type(node["backendDOMNodeId"]) is int
                and node["backendDOMNodeId"] > 0, f"{name} has unexpected Chrome AX role, text or line mode")
        if name in {"readonly", "disabled"}:
            require(props[name] is True, f"{name} AX state changed")
    for name, count in (("before", 3), ("after", 8)):
        require(data[name]["description"]["value"]
                == f"井字號 字數，已輸入 {count} 個字，上限 12 個字。 請簡短記錄這次的變更。",
                "inactive AX description order or text differs")
    active = data["active"]
    require(active["label"] == expected["before"][0] and active["multiline"] is True
            and active["description"] == data["before"]["description"]["value"]
            and all(active["backend_dom_node"] != data[key]["backendDOMNodeId"]
                    for key in ("before", "after")), "real textarea handoff differs")
    return {name: {"label": label, "value": text, "multiline": multiline}
            for name, (label, text, multiline) in expected.items()}


def main(run):
    invocation = read(run / "invocation.json")
    require(sha(ROOT / "web.sha256") == invocation["web_manifest_sha256"] == WEB_SHA
            and sha(ROOT / "pins/surface-e2e") == invocation["runner_sha256"]
            == (ROOT / "runner.sha256").read_text().strip()
            and invocation["scope"] == "q66 M341–M346 development", "scoped runner or Web manifest pin differs")
    assets = dict((name, digest) for digest, name in (
        line.split("  ", 1) for line in (ROOT / "web.sha256").read_text().splitlines()))
    require(invocation["web_files"] == assets and all(
        sha(ROOT / "web/material-gallery" / name) == digest for name, digest in assets.items()),
        "retained Web assets differ")
    log = (run / "runner.log").read_text()
    rows = re.findall(r"^case material \.\.\. ok \((\d+) steps, (\d+) checks,", log, re.M)
    summary = re.findall(r"^e2e result: 1 passed; 0 failed; (\d+) checks;", log, re.M)
    require(re.findall(r"^  \[M(\d+)\]", log, re.M) == STEPS and len(rows) == 1
            and rows[0][0] == "6" and summary == [rows[0][1]], "wrong actual focused E2E result")
    window_path = run / "headless-window-observation.json"
    window = read(window_path)
    event, = window["guard_events"]
    ledger, = (run / "guard-ledger").glob("chrome-primary-*.json")
    chrome = window["launch_observations"]["chrome-primary"]
    require(event == read(ledger) and event["allowed"] is True and event["role"] == "chrome-primary"
            and "--headless=new" in event["argv"] and chrome["samples"] > 0 and chrome["pids"] == [event["pid"]]
            and window["stock_exit"] == 0 and window["problems"] == []
            and window["visible_owned_windows"] == window["remaining_observed_processes"] == [],
            "headless Chrome gate failed")
    artifacts = run / "artifacts"
    report_path = artifacts / REPORT
    report = read(report_path)
    require(re.findall(r"^browser: (Chrome/\S+) ", log, re.M) == [report["browser"]]
            and report["adapter"] == {"vendor": "apple", "architecture": "metal-3", "isFallbackAdapter": False},
            "reported Chrome or actual Apple Metal adapter differs")
    new = verify_inactive(report.pop("inactive_ax"))
    old = PRIOR / "verify_multiline.py"
    require(sha(old) == "edbff9925ebfc4a2c3b4561a1cd86557ea1686b64f8cea315228028976b73ee2",
            "the retained verifier differs")
    sys.path.insert(0, str(PRIOR))
    previous = run_path(str(old))
    # Replay q65's original decoder against the exact current PNGs and
    # its portion of the typed report. Original output remains immutable.
    with TemporaryDirectory(prefix="textbox-values-retained-") as current:
        root = Path(current)
        (root / REPORT).write_text(json.dumps(report, ensure_ascii=False))
        for path in artifacts.glob("*.png"):
            (root / path.name).symlink_to(path)
        retained = previous["verify"](root, {"browser": report["browser"], "adapter": report["adapter"]})
        retained.pop("report_sha256")  # temporary q65 projection
    print(json.dumps({"scope": invocation["scope"], "steps": [f"M{step}" for step in STEPS],
        "checks": int(summary[0]), "browser": report["browser"], "adapter": report["adapter"],
        "window_samples": chrome["samples"], "visible_windows": 0, "survivors": 0,
        "report_sha256": sha(report_path), "window_receipt_sha256": sha(window_path),
        "runner_sha256": invocation["runner_sha256"], "web_manifest_sha256": WEB_SHA,
        "inactive_ax": new, "retained_m341_to_m345": retained}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main(Path(sys.argv[1]).resolve(strict=True))
