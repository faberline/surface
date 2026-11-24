"""Verify the five stock headless suites and the sealed 246-step Material run."""

from hashlib import file_digest
import json
from pathlib import Path
import re
import subprocess
import sys

from verify_choice_report import validate as validate_choice
from verify_contrast_report import validate as validate_contrast
from verify_ime_cancel_report import validate as validate_ime_cancel
from verify_loop_report import validate as validate_loop
from verify_modal_report import validate as validate_modal, verify_compact_pngs
from verify_numeric_report import validate as validate_numeric
from verify_pair_report import validate as validate_pair
from verify_popup_report import validate as validate_popup

CASES = ["smoke", "ime", "material", "bridge-web", "devtools-headless"]
MATERIAL = [f"M{index}" for index in range(1, 248) if index != 23]
ADAPTER = {"vendor": "apple", "architecture": "metal-3", "isFallbackAdapter": False}
REPORTS = {
    "scroll-modal-fields": validate_modal,
    "scroll-popup-fields": validate_popup,
    "scroll-fields": validate_numeric,
    "scroll-choice-fields": validate_choice,
    "scroll-loop-fields": validate_loop,
    "scroll-pair-fields": validate_pair,
}


def read(path):
    return json.loads(path.read_text())


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def owned_processes(stage, run):
    info = subprocess.check_output(["/bin/ps", "-axww", "-o", "pid=,command="], text=True)
    names = ("surface-e2e", "surface-material-gallery", "surface-devtools", "jet-bridge")
    runners = tuple(str(stage / "bundle" / "debug" / name) for name in names)
    runners += tuple(path.removeprefix("/private") for path in runners)
    profile = str(run / "browser-temp")
    profiles = (profile, profile.removeprefix("/private"))
    chrome = "/Applications/Google Chrome.app/Contents/"
    owned = []
    for line in info.splitlines():
        pid, _, command = line.strip().partition(" ")
        if command.startswith(runners):
            owned.append({"pid": int(pid), "process": "Surface E2E"})
        elif command.startswith(chrome) and any(path in command for path in profiles):
            owned.append({"pid": int(pid), "process": "Chrome"})
    return owned


def material_steps(log):
    return re.findall(r"^  \[(M\d+)\]", log, re.M)


def attempt(stage, run, runner_exit, tee_exit):
    log = (run / "runner.log").read_text()
    write(run / "attempt.json", {
        "runner_exit": int(runner_exit),
        "tee_exit": int(tee_exit),
        "headless_flag": "--headless=new",
        "cases_reported": [line for line in log.splitlines() if line.startswith("case ")],
        "material_steps_started": material_steps(log),
        "failure_dumps": sorted(path.name for path in run.glob("*failure*")),
        "remaining_owned_processes": owned_processes(stage, run),
    })


def passing(stage, run):
    execution = read(run / "attempt.json")
    assert execution["runner_exit"] == execution["tee_exit"] == 0
    assert execution["headless_flag"] == "--headless=new"
    assert not execution["remaining_owned_processes"] and not execution["failure_dumps"]
    log = (run / "runner.log").read_text()
    case_lines = re.findall(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks, (\d+(?:\.\d+)?)s\)$", log, re.M)
    cases = [entry[0] for entry in case_lines]
    assert cases == CASES, case_lines
    total_checks = sum(int(entry[2]) for entry in case_lines)
    summary = f"e2e result: 5 passed; 0 failed; {total_checks} checks"
    assert summary in log, summary
    steps = material_steps(log)
    assert steps == execution["material_steps_started"] == MATERIAL, steps
    case_evidence = {
        name: {"steps": int(count), "checks": int(checks), "elapsed_seconds": float(seconds)}
        for name, count, checks, seconds in case_lines
    }
    assert case_evidence["material"]["steps"] == 246
    assert case_evidence["ime"]["steps"] == 20
    assert re.findall(r"^  \[(I\d+)\]", log, re.M) == [f"I{n}" for n in range(1, 21)]
    smoke = read(run / "browser-smoke.json")
    assert smoke["errors"] == []
    assert {key: smoke["adapter"][key] for key in ADAPTER} == ADAPTER
    ime = read(run / "browser-ime.json")
    line_endings = read(run / "browser-ime-line-endings.json")
    ime_cancel = validate_ime_cancel(run)
    assert ime["errors"] == [] and ime["committed"] == "繁體中文、简体中文"
    assert line_endings["authored"] == "ab\r\n你🦀\rZ"
    assert line_endings["browser_value"] == "ab\n你🦀\nZ"
    assert [(part["key"],part["browser"],part["runtime"]) for part in line_endings["steps"]] == [
        ("ArrowLeft",[1,1],[1,1]), ("Shift+ArrowLeft",[2,3],[4,2]), ("ArrowRight",[6,6],[11,11])
    ]
    assert line_endings["steps"][1]["direction"] == "backward"
    results = [f"case {name} ... ok ({count} steps, {checks} checks, {seconds}s)"
               for name, count, checks, seconds in case_lines]
    (run / "headless-results.txt").write_text("\n".join([*results, summary]) + "\n")
    assert read(run / "bridge-web-result.json")["owned_browser_exited"] is True
    assert read(run / "devtools-headless-result.json")["screenshot_transports_match"] is True

    reports = {}
    for name, validate in REPORTS.items():
        reports[name] = report = read(run / f"browser-material-{name}.json")
        validate(report)
    compact_pngs = verify_compact_pngs(run)
    adapter = read(run / "browser-material-side-sheet.json")["adapter"]
    assert adapter == ADAPTER, adapter
    modal = reports["scroll-modal-fields"]
    browser = modal["browser"]
    contrast = validate_contrast(run, browser, adapter)

    checked = {}
    for path in sorted(run.glob("browser-material*.json")):
        errors = read(path).get("errors")
        if errors is not None:
            assert isinstance(errors, list), path
            checked[path.name] = len(errors)
    for name in (*REPORTS, "web-contrast", "web-contrast-selection-browser"):
        assert checked[f"browser-material-{name}.json"] == 0
    assert not any(checked.values()), checked

    source = read(stage / "stock-stage.json")
    receipt = {
        "version": source["expected_workspace_version"],
        "source_git_head": source["source_git_head"],
        "source_identity_kind": source["source_identity_kind"],
        "incoming_source_manifest_sha256": source["incoming_source_manifest_sha256"],
        "exact_source_manifest_sha256": sha(stage / "browser-source.sha256"),
        "exact_binary_manifest_sha256": sha(stage / "browser-binary-sha256.txt"),
        "browser": browser, "adapter": adapter, "material_steps": len(steps),
        "headless_cases": cases, "case_results": case_evidence, "total_checks": total_checks,
        "material_page_errors": checked, "remaining_browser_pids": execution["remaining_owned_processes"],
        "showcase_page_errors": {"smoke": len(smoke["errors"]), "ime": len(ime["errors"])},
        "compact_pngs": compact_pngs, "contrast": contrast, "ime_line_endings": line_endings,
        "ime_cancel": ime_cancel,
        "results": {key: modal[key] for key in ("opening", "save", "invalid", "dismissals", "composition", "compact")},
    }
    write(run / "verified-browser.json", receipt)
    print("\n".join(results))
    print(f"{summary}. Verified both I11 cancellation receipts, {len(contrast['pngs'])} contrast Canvas PNGs and independent M247 pixels.")


if __name__ == "__main__":
    phase, stage_arg, run_arg, *arguments = sys.argv[1:]
    stage, run = Path(stage_arg).resolve(), Path(run_arg).resolve()
    if phase == "attempt":
        attempt(stage, run, *arguments)
    elif phase == "passing":
        passing(stage, run)
    else:
        raise SystemExit(f"Unknown stock receipt phase: {phase}")
