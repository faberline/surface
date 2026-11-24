"""Read exact hosted CI evidence for the Surface v0.1.43 Search slice."""

import argparse
from hashlib import sha256
import json
from pathlib import Path
import re
from shutil import copy2
import subprocess
import sys
from time import sleep

from PIL import Image

from stock_contrast_report import validate as validate_contrast
from verify_search_report import PREFIX as SEARCH_PREFIX, validate as validate_search

REPO = "faberline/surface"
BRANCH = "codex/md3-search-adaptive"
GH = "gh"
ROOT = Path(__file__).resolve().parent
ARTIFACT = "surface-e2e-artifacts"
JOBS = ("build-and-behavior", "browser-e2e")
CASES = {
    "smoke", "ime", "material", "bridge-native", "bridge-native-compact",
    "bridge-web", "devtools-native", "devtools-headless",
}
SOURCE_OID = re.compile(r"[0-9a-f]{40}")
REPORT = "browser-material-web-contrast.json"
SEARCH_REPORT = "browser-material-search-overlay.json"
LINE_ENDINGS = "browser-ime-line-endings.json"
CANCEL_REDRAW = "browser-ime-cancel-redraw.json"
CANCEL_SELECTED = "browser-ime-cancel-selected-redraw.json"
ARROW_PHASES = ("native-arrow", "delayed-select-arrow")
KEY_PNGS = [
    "material-web-contrast-popup-normal.png",
    "material-web-contrast-popup-forced.png",
    "material-web-contrast-popup-restored.png",
    "material-web-contrast-preedit-underline-preedit-partial.png",
] + [f"material-web-contrast-selection-browser-{phase}-forced.png" for phase in ARROW_PHASES]


def call(*arguments):
    return subprocess.run([GH, *map(str, arguments)], stdout=subprocess.PIPE, check=True).stdout


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
    assert material == [n for n in range(1, 256) if n != 23], material
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


def download(run_id, directory):
    item = artifact_metadata(run_id)
    if item is None:
        return None
    call("run", "download", run_id, "--repo", REPO, "--name", ARTIFACT, "--dir", directory)
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
            text = read_log(job["id"], path / f"{name}.log")
            hits = [line for line in text.splitlines() if re.search(
                r"\[M(?:24[1-9]|25[0-5])\]|\[I1[0-2]\]|FAILED|e2e result:|preedit underline|Error: Material|cancel restores", line
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
        snapshots = {
            event: exact_run(event, run_id, args.head, args.pr, args.base)
            for event, run_id in ids if event not in results
        }
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
                print(f"{event} CI passed but receipt validation stopped: {error!r}", flush=True)
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


def verify_artifacts(out, run_id, head):
    downloaded = out / "download"
    artifact = download(run_id, downloaded)
    assert artifact and artifact["workflow_run"]["id"] == run_id
    assert artifact["workflow_run"]["head_sha"] == head
    contrast_path = only(downloaded, REPORT)
    side_sheet = json.loads(only(downloaded, "browser-material-side-sheet.json").read_text())
    modal = json.loads(only(downloaded, "browser-material-scroll-modal-fields.json").read_text())
    assert not modal["errors"] and modal["browser"].startswith("Chrome/")
    contrast = validate_contrast(contrast_path.parent, modal["browser"], side_sheet["adapter"])
    search = validate_search(contrast_path.parent, modal["browser"], side_sheet["adapter"])
    for frame, item in search["pngs"].items():
        path = only(downloaded, f"{SEARCH_PREFIX}{frame}.png")
        assert sha256(path.read_bytes()).hexdigest() == item["sha256"]
        with Image.open(path) as image:
            assert image.format == "PNG" and list(image.size) == item["physical_size"]
            image.load()
            rgba = image.convert("RGBA").tobytes()
        assert len(rgba) == item["physical_size"][0] * item["physical_size"][1] * 4
        item["rgba_sha256"] = sha256(rgba).hexdigest()
    assert len(search["pngs"]) == 46
    showcase_errors = {}
    for filename in ("browser-smoke.json", "browser-ime.json"):
        report = json.loads(only(downloaded, filename).read_text())
        assert report["errors"] == []
        showcase_errors[filename] = len(report["errors"])
    material_errors = {}
    for path in sorted(contrast_path.parent.glob("browser-material*.json")):
        errors = json.loads(path.read_text()).get("errors")
        if errors is not None:
            assert errors == [], path
            material_errors[path.name] = len(errors)
    assert all(name in material_errors for name in (SEARCH_REPORT, REPORT, "browser-material-scroll-modal-fields.json"))
    line_endings = verify_line_endings(downloaded)
    cancellations = verify_cancellations(downloaded)
    kept = out / "verified"
    kept.mkdir(exist_ok=True)
    reports = [
        REPORT, SEARCH_REPORT, "browser-smoke.json", "browser-ime.json", "browser-material-web-contrast-selection.json",
        "browser-material-web-contrast-preedit-underline.json", LINE_ENDINGS, CANCEL_REDRAW, CANCEL_SELECTED,
    ]
    reports.extend(f"material-web-contrast-selection-browser-{phase}.json" for phase in ARROW_PHASES)
    for filename in reports:
        copy2(only(downloaded, filename), kept / filename)
    saved_pngs = []
    for filename in KEY_PNGS:
        found = list(downloaded.rglob(filename))
        assert len(found) <= 1, filename
        if found:
            copy2(found[0], kept / filename)
            saved_pngs.append(filename)
    return {
        "artifact": {"id": artifact["id"], "name": artifact["name"], "digest": artifact.get("digest")},
        "contrast": contrast,
        "search": search,
        "page_errors": {"showcase": showcase_errors, "material": material_errors},
        "ime_line_endings": line_endings,
        "ime_cancellation": cancellations,
        "preserved_stock_pngs": saved_pngs,
    }


def verify_success(out, event, run, jobs, args):
    assert set(jobs) == set(JOBS) and all(job["conclusion"] == "success" for job in jobs.values())
    path = out / event
    path.mkdir(exist_ok=True)
    write_json(path / "status.json", {"run": run, "jobs": jobs})
    receipt = verified_run(path, event, run, jobs, args.tree, args.base)
    receipt.update(verify_artifacts(path, run["id"], args.head))
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
        "release": "v0.1.43", "pr": args.pr, "branch": BRANCH, "head": args.head, "tree": args.tree, "base": args.base,
        "host": "GitHub Linux with an Xvfb virtual display and Chrome WebGPU SwiftShader",
        "runs": results,
    }
    kept = out / "verified"
    kept.mkdir(exist_ok=True)
    write_json(kept / "hosted-verified.json", proof)
    print(f"Verified both eight-case suites and exact trees. Receipt: {kept / 'hosted-verified.json'}", flush=True)


if __name__ == "__main__":
    main()
