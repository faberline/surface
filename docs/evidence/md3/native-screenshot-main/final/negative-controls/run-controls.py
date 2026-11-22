"""Run the unchanged DN1–17 journey against each disposable macOS fault."""

import hashlib
import json
import os
from pathlib import Path
import re
import signal
import struct
import subprocess


def run_case(proof, target, mode):
    artifact_dir = proof / "evidence" / mode
    artifact_dir.mkdir(parents=True, exist_ok=False)
    environment = {**os.environ, "CARGO_TARGET_DIR": str(target)}
    option = "SURFACE_SCREENSHOT_NEGATIVE_CONTROL"
    if mode == "baseline":
        environment.pop(option, None)
    else:
        environment[option] = mode
    command = [
        str(target / "debug/surface-e2e"),
        "--native-bin",
        (proof / "fault-app.txt").read_text().strip(),
        "--devtools-bin",
        str(target / "debug/surface-devtools"),
        "--artifacts",
        str(artifact_dir),
        "devtools-native",
    ]
    (artifact_dir / "command.json").write_text(json.dumps(command, indent=2) + "\n")
    log_path = artifact_dir / "runner.log"
    with log_path.open("w") as output:
        running = subprocess.Popen(
            command,
            cwd=proof / "repo",
            env=environment,
            stdout=output,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        try:
            status = running.wait(timeout=120)
        except subprocess.TimeoutExpired:
            os.killpg(running.pid, signal.SIGTERM)
            running.wait(timeout=5)
            raise
    (artifact_dir / "exit.txt").write_text(f"{status}\n")
    stdout = log_path.read_text()
    native_log = (artifact_dir / "devtools-native-stderr.log").read_text()
    return artifact_dir, status, stdout, native_log


def check_baseline(artifact_dir, status, log, native_log):
    assert status == 0, log
    match = re.search(r"^case devtools-native \.\.\. ok \(17 steps, (\d+) checks,", log, re.M)
    assert match and "[negative-control]" not in native_log, log
    receipt = json.loads((artifact_dir / "devtools-native-result.json").read_text())
    assert receipt["screenshot_transports_match"] is True, receipt
    assert receipt["screenshot"]["max_side"] == 400, receipt
    dimensions = {}
    for suffix in ["", "-toggled", "-scaled", "-cli", "-mcp"]:
        png = (artifact_dir / f"devtools-native-screenshot{suffix}.png").read_bytes()
        assert png[:8] == b"\x89PNG\r\n\x1a\n" and png[12:16] == b"IHDR", suffix
        dimensions[suffix or "-raw"] = {
            "width_height": struct.unpack(">2I", png[16:24]),
            "sha256": hashlib.sha256(png).hexdigest(),
        }
    assert dimensions["-raw"]["width_height"] == dimensions["-toggled"]["width_height"]
    assert dimensions["-raw"]["sha256"] != dimensions["-toggled"]["sha256"]
    assert dimensions["-scaled"] == dimensions["-cli"] == dimensions["-mcp"]
    return {
        "exit": status,
        "steps": 17,
        "checks": int(match[1]),
        "render": receipt,
        "pngs": dimensions,
    }


def check_fault(mode, status, log, native_log):
    assert status == 1, log
    assert "  last step: DN15" in log, log
    assert "[DN16]" not in log and "[DN17]" not in log, log
    markers = {
        "swapped-red-blue": "swapped red and blue in the rendered PNG",
        "stale-default-image": "returned the cached first default PNG",
        "ignored-max-side": "ignored max_side Some(400)",
    }
    assert f"[negative-control] {markers[mode]}" in native_log, native_log
    match = re.search(
        r"^case devtools-native \.\.\. FAILED after [\d.]+s: (.+)$",
        log,
        re.M,
    )
    assert match, log
    failure = match[1]
    checks = re.search(r"e2e result: 0 passed; 1 failed; (\d+) checks;", log)
    assert checks, log
    if mode == "ignored-max-side":
        assert re.fullmatch(r"max_side downscales: expected 400, got \d+", failure), failure
        assert int(failure.rsplit(" ", 1)[1]) > 400, failure
    else:
        colors = re.fullmatch(
            r"pixel (\d+),(\d+): expected (\[[\d, ]+\]), got (\[[\d, ]+\])",
            failure,
        )
        assert colors, failure
        expected, actual = json.loads(colors[3]), json.loads(colors[4])
        if mode == "swapped-red-blue":
            assert (colors[1], colors[2]) != ("1", "1"), failure
            assert expected[0] != expected[2], failure
            swapped = [expected[2], expected[1], expected[0], expected[3]]
            assert all(abs(left - right) <= 1 for left, right in zip(actual, swapped)), failure
        else:
            assert (colors[1], colors[2]) == ("1", "1") and actual != expected, failure
    return {"exit": status, "step": "DN15", "checks": int(checks[1]), "assertion": failure}


def main():
    proof = Path(__file__).resolve().parent
    target = Path((proof / "target-dir.txt").read_text().strip())
    subprocess.run(["shasum", "-a", "256", "-c", proof / "binaries.sha256"], check=True)
    receipts = {}
    for mode in ["baseline", "swapped-red-blue", "stale-default-image", "ignored-max-side"]:
        directory, status, log, native_log = run_case(proof, target, mode)
        if mode == "baseline":
            receipts[mode] = check_baseline(directory, status, log, native_log)
        else:
            receipts[mode] = check_fault(mode, status, log, native_log)
        print(f"{mode} exit {status}", flush=True)
    (proof / "evidence" / "receipts.json").write_text(json.dumps(receipts, indent=2) + "\n")


if __name__ == "__main__":
    main()
