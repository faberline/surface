"""Read exact hosted CI evidence for the Surface v0.1.44 Dialog slice."""

import argparse
from hashlib import file_digest, sha256
import json
from pathlib import Path, PurePosixPath
import re
from shutil import copy2
import subprocess
import sys
from time import sleep
from zipfile import ZipFile

from PIL import Image

from verify_dialog_report import DIALOG_STEPS, validate as validate_dialog

REPO = "faberline/surface"
BRANCH = "codex/md3-dialog-adaptive"
GH = "gh"
ROOT = Path(__file__).resolve().parent
ARTIFACT = "surface-e2e-artifacts"
JOBS = ("build-and-behavior", "browser-e2e")
CASES = {
    "smoke", "ime", "material", "bridge-native", "bridge-native-compact",
    "bridge-web", "devtools-native", "devtools-headless",
}
SOURCE_OID = re.compile(r"[0-9a-f]{40}")
REPORT = "browser-material-dialog.json"
DIALOG_PREFIX = "material-dialog-adaptive-"
ADAPTER = {"vendor":"google", "architecture":"swiftshader", "isFallbackAdapter":True}
LINE_ENDINGS = "browser-ime-line-endings.json"
CANCEL_REDRAW = "browser-ime-cancel-redraw.json"
CANCEL_SELECTED = "browser-ime-cancel-selected-redraw.json"


def call(*arguments):
    command = [GH, *map(str, arguments)]
    for delay in (2, 5, 10):
        try:
            return subprocess.run(command, stdout=subprocess.PIPE, check=True).stdout
        except subprocess.CalledProcessError:
            print(f"GitHub read interrupted. Retrying the same read in {delay}s.", flush=True)
            sleep(delay)
    return subprocess.run(command, stdout=subprocess.PIPE, check=True).stdout


def api(path):
    return json.loads(call("api", f"repos/{REPO}/{path}"))


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def find_push(head):
    rows = json.loads(call(
        "run", "list", "--repo", REPO, "--workflow", "ci.yml", "--commit", head,
        "--event", "push", "--json", "databaseId,headSha", "--limit", "8",
    ))
    return next((int(row["databaseId"]) for row in rows if row["headSha"] == head), None)


def exact_run(event, run_id, head, pr, base):
    run = api(f"actions/runs/{run_id}")
    assert (run["head_sha"], run["event"], run["path"]) == (head, event, ".github/workflows/ci.yml"), run_id
    assert run["head_branch"] == BRANCH, run_id
    if event == "pull_request":
        entry, = [entry for entry in run["pull_requests"] if entry["number"] == pr]
        assert (entry["head"]["sha"], entry["head"]["ref"], entry["base"]["ref"]) == (head, BRANCH, "main")
        assert entry["base"]["sha"] == base
    attempt = run["run_attempt"]
    assert attempt == 1, (run_id, attempt)
    jobs = api(f"actions/runs/{run_id}/attempts/{attempt}/jobs")["jobs"]
    by_name = {}
    for job in jobs:
        assert (job["run_id"], job["run_attempt"], job["head_sha"]) == (run_id, attempt, head), job["id"]
        assert job["name"] not in by_name
        by_name[job["name"]] = job
    return run, by_name


def check_log(log):
    assert re.search(r'Run xvfb-run -a -s "-screen 0 1920x1080x24" cargo xtask web e2e', log)
    assert "Operating System" in log and "Ubuntu" in log
    rows = re.findall(r"case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks,", log)
    assert len(rows) == len(CASES) and {row[0] for row in rows} == CASES
    counts = {name: {"steps": int(steps), "checks": int(checks)} for name, steps, checks in rows}
    assert all(value["steps"] > 0 and value["checks"] > 0 for value in counts.values())
    material = [int(value) for value in re.findall(r"\[M(\d+)\]", log)]
    assert material == [n for n in range(1, 264) if n != 23], material
    assert counts["material"]["steps"] == len(material)
    ime = [int(value) for value in re.findall(r"\[I(\d+)\]", log)]
    assert ime == list(range(1, 21)), ime
    assert counts["ime"]["steps"] == len(ime)
    assert counts["ime"]["checks"] == 77, counts["ime"]
    total = sum(row["checks"] for row in counts.values())
    assert f"e2e result: 8 passed; 0 failed; {total} checks;" in log
    assert not re.search(r"case \S+ \.\.\. (?:FAILED|skipped)", log)
    return counts, total


def checkout_sha(log):
    match = re.search(r"git log -1 --format=%H\r?\n\S+ ([0-9a-f]{40})(?:\r?\n|$)", log)
    assert match, "The checkout log must identify the built commit"
    return match.group(1)


def artifact_metadata(run_id):
    items = [
        item for item in api(f"actions/runs/{run_id}/artifacts")["artifacts"]
        if item["name"] == ARTIFACT and not item["expired"]
    ]
    assert len(items) <= 1, "Run has ambiguous artifacts"
    return items[0] if items else None


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def download(run_id, directory):
    item = artifact_metadata(run_id)
    if item is None:
        return None
    expected = item["digest"].removeprefix("sha256:")
    archive = directory.parent / "surface-e2e-artifacts.zip"
    if not archive.is_file():
        partial = archive.with_suffix(".zip.download")
        with partial.open("xb") as output:
            subprocess.run([GH,"api",f"repos/{REPO}/actions/artifacts/{item['id']}/zip"],
                           stdout=output,check=True)
        assert sha(partial) == expected, (item['id'],expected)
        partial.rename(archive)
    assert sha(archive) == expected, (item['id'],expected)
    assert archive.stat().st_size == item['size_in_bytes']
    with ZipFile(archive) as zipped:
        members = [name for name in zipped.namelist() if not name.endswith("/")]
        assert len(set(members)) == len(members)
        for name in members:
            path = PurePosixPath(name)
            assert not path.is_absolute() and '..' not in path.parts and '\\' not in name
        zipped.extractall(directory)
    return item


def read_log(job_id, path):
    path.write_bytes(call("api", f"repos/{REPO}/actions/jobs/{job_id}/logs"))
    return path.read_text(errors="replace")


def preserve_failures(out, event, run, jobs):
    path = out / f"{event}-failure"
    path.mkdir(exist_ok=True)
    write_json(path / "status.json", {"run": run, "jobs": jobs})
    for name, job in jobs.items():
        if job["status"] == "completed":
            try:
                text = read_log(job["id"], path / f"{name}.log")
            except subprocess.CalledProcessError:
                continue
            hits = [line for line in text.splitlines() if re.search(
                r"\[M(?:25[6-9]|26[0-3])\]|\[I1[0-2]\]|FAILED|e2e result:|preedit underline|Error: Material|cancel restores", line
            )]
            if hits and job["conclusion"] != "success":
                print("\n".join(hits[-20:]), flush=True)
    download(run["id"], path / "download")
    print(f"{event} CI failed. Logs and available artifacts in {path}", flush=True)


def observe_runs(args, out):
    previous = None
    push_id = args.push_run
    results = {}
    while True:
        push_id = push_id or find_push(args.head)
        ids = [("pull_request", args.pull_run)]
        if push_id:
            ids.append(("push", push_id))
        try:
            snapshots = {
                event: exact_run(event, run_id, args.head, args.pr, args.base)
                for event, run_id in ids if event not in results
            }
        except (subprocess.CalledProcessError, json.JSONDecodeError) as error:
            if not args.watch:
                raise
            print(f"GitHub observation temporarily unavailable: {error!r}", flush=True)
            sleep(15)
            continue
        for event, (run, jobs) in snapshots.items():
            expected_job = args.browser_job if event == "pull_request" else args.push_browser_job
            if expected_job and "browser-e2e" in jobs:
                assert jobs["browser-e2e"]["id"] == expected_job, (event, expected_job, jobs)
        status = tuple((event, run["id"], run["status"], tuple(
            (name, jobs[name]["status"]) for name in sorted(jobs)
        )) for event, (run, jobs) in snapshots.items())
        if status != previous:
            print(status, flush=True)
            previous = status
        for event, (run, jobs) in snapshots.items():
            if run["status"] != "completed":
                continue
            if run["conclusion"] != "success":
                preserve_failures(out, event, run, jobs)
                results[event] = None
                continue
            try:
                results[event] = verify_success(out, event, run, jobs, args)
            except (AssertionError, KeyError, ValueError, OSError, subprocess.CalledProcessError) as error:
                import traceback
                print(f"{event} CI passed but receipt validation stopped: {error!r}", flush=True)
                traceback.print_exc()
                results[event] = None
        if len(results) == 2:
            return results
        if not args.watch:
            raise SystemExit("The two exact-head workflows are still pending")
        sleep(15)


def verified_run(out, event, run, jobs, expected_tree, expected_base):
    browser = jobs["browser-e2e"]
    log = read_log(browser["id"], out / f"{event}-browser-e2e.log")
    checkout = checkout_sha(log)
    build = jobs["build-and-behavior"]
    build_log = read_log(build["id"], out / f"{event}-build-and-behavior.log")
    assert checkout_sha(build_log) == checkout, (event, build["id"], checkout)
    commit = api(f"git/commits/{checkout}")
    tree = commit["tree"]["sha"]
    assert tree == expected_tree, (event, checkout, tree, expected_tree)
    if event == "push":
        assert checkout == run["head_sha"]
    else:
        assert [parent["sha"] for parent in commit["parents"]] == [expected_base, run["head_sha"]]
    counts, total = check_log(log)
    return {
        "event": event, "run": run["id"], "attempt": run["run_attempt"], "url": run["html_url"],
        "jobs": {name: {"id": job["id"], "url": job["html_url"]} for name, job in jobs.items()},
        "head": run["head_sha"], "checkout_commit": checkout, "checkout_tree": tree,
        "checkout_parents":[parent["sha"] for parent in commit["parents"]],
        "cases": counts, "total_checks": total,
    }


def only(root, filename):
    path, = list(root.rglob(filename))
    return path


def verify_line_endings(downloaded):
    report = json.loads(only(downloaded, LINE_ENDINGS).read_text())
    assert report["authored"] == "ab\r\n你🦀\rZ" and report["browser_value"] == "ab\n你🦀\nZ"
    expected = [
        ("ArrowLeft", [1, 1], [1, 1]),
        ("Shift+ArrowLeft", [2, 3], [4, 2]),
        ("ArrowRight", [6, 6], [11, 11]),
    ]
    assert [(step["key"], step["browser"], step["runtime"]) for step in report["steps"]] == expected
    assert report["steps"][1]["direction"] == "backward"
    return report


def verify_cancellations(downloaded):
    redraw = json.loads(only(downloaded, CANCEL_REDRAW).read_text())
    assert redraw["redraw_width"] == 1001
    assert redraw["before"]["trustedSelectAll"] is True
    for key in ("before", "after"):
        assert redraw[key]["browser"] == [0, 2] and redraw[key]["value"] == "台文", (key, redraw)
    assert redraw["after"]["runtime"] == ["台文", "台文", "0", "6", "false"], redraw
    mixed = "中👨‍👩‍👧‍👦e\u0301"
    end = str(len(mixed.encode("utf-8")))
    assert redraw["replacement"] == [mixed, mixed, end, end, "false"], redraw

    selected = json.loads(only(downloaded, CANCEL_SELECTED).read_text())
    canceled = selected["canceled"]
    assert canceled["value"] == "台文" and canceled["browser"] == [1, 2], selected
    assert canceled["direction"] == "backward", selected
    assert canceled["runtime"] == ["台文", "台文", "6", "3", "false"], selected
    moved = selected["moved"]
    assert moved["trustedRight"] is True and moved["browser"] == [2, 2], selected
    assert moved["value"] == "台文" and moved["frames"] == canceled["frames"], selected
    assert selected["unpainted"]["frames"] == canceled["frames"], selected
    for key in ("unpainted", "repainted"):
        frame = selected[key]
        assert frame["browser"] == [3, 3] and frame["value"] == "台文X", (key, frame)
        assert frame["runtime"] == ["台文X", "台文X", "7", "7", "false"], (key, frame)
    return {"select_all_survives_redraw": redraw, "backward_range_and_next_edit": selected}


def stock_log(log):
    return "\n".join(re.sub(r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z ", "", line)
                     for line in log.splitlines()) + "\n"


def verify_artifacts(out, run_id, head, log):
    downloaded = out / "download"
    artifact = download(run_id, downloaded)
    assert artifact and artifact["workflow_run"]["id"] == run_id
    assert artifact["workflow_run"]["head_sha"] == head
    reports = [only(downloaded, name) for name in ("browser-smoke.json", "browser-ime.json", "browser-material-side-sheet.json")]
    smoke, ime, side = (json.loads(path.read_text()) for path in reports)
    assert smoke["errors"] == ime["errors"] == side["errors"] == []
    assert {key: smoke["adapter"][key] for key in ADAPTER} == side["adapter"] == ADAPTER
    assert smoke["browser"] == side["browser"] and smoke["browser"].startswith("Chrome/")
    assert ime["committed"] == "繁體中文、简体中文"
    dialog_path = only(downloaded, REPORT)
    assert dialog_path.parent == reports[2].parent
    dialog = validate_dialog(dialog_path.parent, smoke["browser"], side["adapter"], log=stock_log(log))
    assert tuple(dialog["steps"]) == DIALOG_STEPS
    actual = {path.stem.removeprefix(DIALOG_PREFIX) for path in downloaded.rglob(f"{DIALOG_PREFIX}*.png")}
    assert len(actual) == 41 and actual == set(dialog["pngs"]), actual ^ set(dialog["pngs"])
    for frame, item in dialog["pngs"].items():
        path = only(downloaded, f"{DIALOG_PREFIX}{frame}.png")
        assert sha(path) == item["sha256"]
        with Image.open(path) as image:
            assert image.format == "PNG" and image.size == item["dimensions"]
            image.load()
            rgba = image.convert("RGBA").tobytes()
        assert len(rgba) == item["dimensions"][0] * item["dimensions"][1] * 4
        item["decoded_rgba_sha256"] = sha256(rgba).hexdigest()
    material_errors = {}
    for path in sorted(dialog_path.parent.glob("browser-material*.json")):
        errors = json.loads(path.read_text()).get("errors")
        if errors is not None:
            assert errors == [], path
            material_errors[path.name] = len(errors)
    assert all(name in material_errors for name in (REPORT, "browser-material-search-overlay.json", "browser-material-web-contrast.json"))
    cancellations = verify_cancellations(downloaded)
    line_endings = verify_line_endings(downloaded)
    kept = out / "verified"
    kept.mkdir(exist_ok=True)
    files = reports + [dialog_path] + [only(downloaded, name) for name in (LINE_ENDINGS, CANCEL_REDRAW, CANCEL_SELECTED)]
    for path in files:
        copy2(path, kept / path.name)
    return {
        "artifact": {"id": artifact["id"], "name": artifact["name"], "digest": artifact["digest"],
                     "original_zip_size": artifact["size_in_bytes"], "original_zip_sha256_verified": True},
        "dialog": dialog,
        "dialog_validator_sha256": sha(ROOT / "verify_dialog_report.py"),
        "page_errors": {"showcase": {reports[0].name:0,reports[1].name:0}, "material": material_errors},
        "ime_line_endings": line_endings,
        "ime_cancellation": cancellations,
    }


def verify_success(out, event, run, jobs, args):
    assert set(jobs) == set(JOBS) and all(job["conclusion"] == "success" for job in jobs.values())
    path = out / event
    path.mkdir(exist_ok=True)
    write_json(path / "status.json", {"run": run, "jobs": jobs})
    receipt = verified_run(path, event, run, jobs, args.tree, args.base)
    receipt.update(verify_artifacts(path, run["id"], args.head,
                                    (path / f"{event}-browser-e2e.log").read_text()))
    proof = path / "verified" / "run-verified.json"
    write_json(proof, receipt)
    print(f"{event} verified: {receipt['total_checks']} checks. Receipt: {proof}", flush=True)
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pr", type=int, required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--tree", required=True)
    parser.add_argument("--base", required=True)
    parser.add_argument("--pull-run", type=int, required=True)
    parser.add_argument("--browser-job", type=int)
    parser.add_argument("--push-run", type=int)
    parser.add_argument("--push-browser-job", type=int)
    parser.add_argument("--watch", action="store_true")
    args = parser.parse_args()
    assert all(SOURCE_OID.fullmatch(oid) for oid in (args.head, args.tree, args.base))
    pr = api(f"pulls/{args.pr}")
    assert pr["number"] == args.pr and pr["base"]["ref"] == "main"
    if pr["head"]["sha"] != args.head or pr["base"]["sha"] != args.base:
        print("Current PR refs have moved. The historical run and checkout must still match the exact pin.", flush=True)
    assert api(f"git/commits/{args.head}")["tree"]["sha"] == args.tree

    out = ROOT / f"run-{args.pull_run}"
    out.mkdir(exist_ok=True)
    results = observe_runs(args, out)
    if not all(results.values()):
        raise SystemExit("At least one exact-head run lacks passing evidence. See the per-event records.")
    proof = {
        "release": "v0.1.44", "pr": args.pr, "branch": BRANCH, "head": args.head, "tree": args.tree, "base": args.base,
        "host": "GitHub Linux with an Xvfb virtual display and Chrome WebGPU SwiftShader",
        "runs": results,
    }
    kept = out / "verified"
    kept.mkdir(exist_ok=True)
    write_json(kept / "hosted-verified.json", proof)
    print(f"Verified both eight-case suites and exact trees. Receipt: {kept / 'hosted-verified.json'}", flush=True)


if __name__ == "__main__":
    main()
