#!/usr/bin/env python3
"""Verify exact Checkbox v0.1.49 GitHub jobs using read-only API calls and original artifacts."""

import argparse
from hashlib import file_digest, sha1, sha256
import json
from pathlib import Path, PurePosixPath
import re
from shutil import copyfile
import subprocess
import tomllib
from time import sleep
from zipfile import ZipFile

from verify_divider_report import PROFILES, validate as verify_dividers
from verify_ime_cancel_report import validate as verify_cancellations
from verify_menu_report import STEM as MENU_STEM, VIEWPORTS, validate as verify_menu
from verify_navigation_bar import (
    STEM as NAV_STEM, SHOTS as NAV_SHOTS, mac_observation,
    validate as verify_navigation, validate_navigation, validate_log,
)
from verify_switch_report import SHOTS as SWITCH_SHOTS, STEM as SWITCH_STEM, validate as verify_switch
from verify_checkbox_report import SHOTS as CHECKBOX_SHOTS, STEM as CHECKBOX_STEM, validate as verify_checkbox
from rejection_controls import controls as reject_controls


ROOT = Path(__file__).resolve().parent
BRANCH = "codex/md3-checkbox"
REPO = "faberline/surface"
CANDIDATE_PIN = json.loads((ROOT / "candidate-pin.json").read_text())
SOURCE_SHA = CANDIDATE_PIN["source_manifest_sha256"]
SOURCE_COUNT = CANDIDATE_PIN["source_paths"]
BASE_SHA = "a4ec5bc9d9c0da42fe004f6f7d6877f59edc08e5503ae15c3bbefb6c73f4d086"
BASE_COUNT = 598
NAV_VALIDATOR_SHA = "53eb0fa5d730a1736be585fc8c78da290a5d17c919f5de6ec0cd11793adc22f2"
DIVIDER_VALIDATOR_SHA = "54091473417fec52c7c9930a01a8acf4d1301a3e55cef36746f04855332b208d"
MENU_ORIGINAL_SHA = "44211cd13e2477ea2fbf6c602c9270142356c102a12b244d3fa1171712082d31"
IME_VALIDATOR_SHA = "46169df81e3b5a36df5863acdf330ef7884a88d54fdad4d9067fff691f3f30c7"
SWITCH_VALIDATOR_SHA = "2023c287d0e8edb9ac1830f8b85e7c3d43e9b923a26050e90953fb061aa25151"
CHECKBOX_VALIDATOR_SHA = "47ce6d5f1d0184b9cb78b6e06b19f1d50e80f0621782ba0475799672762fa75c"
LAST_MATERIAL = 281
MATERIAL_COUNT = 280
EVENTS = ("pull_request", "push")
CASES = {"smoke", "ime", "material", "bridge-native", "bridge-native-compact",
         "bridge-web", "devtools-native", "devtools-headless"}
JOBS = ("build-and-behavior", "browser-e2e")
ADAPTER = {"isFallbackAdapter": True, "vendor": "google", "architecture": "swiftshader"}
CANCEL_RECEIPTS = ("browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json")
BASE_REPORTS = ("browser-smoke.json", "browser-ime.json", "browser-material-side-sheet.json")
MATERIAL_REPORTS = (f"browser-{CHECKBOX_STEM}.json", f"browser-{SWITCH_STEM}.json", f"browser-{NAV_STEM}.json",
                    f"browser-{MENU_STEM}.json", "browser-material-divider.json")
CHECKBOX_IMAGES = tuple(f"{CHECKBOX_STEM}-{mode}.png" for mode in CHECKBOX_SHOTS)
SWITCH_IMAGES = tuple(f"{SWITCH_STEM}-{mode}.png" for mode in SWITCH_SHOTS)
NAV_IMAGES = tuple(f"{NAV_STEM}-{mode}.png" for mode in NAV_SHOTS)
MENU_IMAGES = tuple(f"{MENU_STEM}-{mode}.png" for mode in VIEWPORTS)
DIVIDER_IMAGES = tuple(f"material-divider-{name}.png" for name, *_ in PROFILES.values())
RAW_FILES = (*BASE_REPORTS, *MATERIAL_REPORTS, *CANCEL_RECEIPTS, *CHECKBOX_IMAGES, *SWITCH_IMAGES,
             *NAV_IMAGES, *MENU_IMAGES, *DIVIDER_IMAGES)
SWIFTSHADER = 'SURFACE_CHROME_ARGS: --enable-unsafe-webgpu --enable-features=Vulkan --use-vulkan=swiftshader --use-webgpu-adapter=swiftshader --use-angle=swiftshader'
INVOCATION = 'Run xvfb-run -a -s "-screen 0 1920x1080x24" cargo xtask web e2e'
TIME_PREFIX = re.compile(r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z ")
ANSI = re.compile(r"\x1b\[[\d;]*m")
CASE = re.compile(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks,", re.M)


def read_json(path):
    return json.loads(path.read_text())


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def digest(path):
    with path.open("rb") as handle:
        return file_digest(handle, "sha256").hexdigest()


def get(path):
    return subprocess.run(["gh", "api", "--method", "GET", f"repos/{REPO}/{path}"],
                          stdout=subprocess.PIPE, check=True).stdout


def api(path):
    return json.loads(get(path))


def read_source(source, *, base=False):
    source = source.resolve(strict=True)
    manifest = ROOT / ("switch-base.sha256" if base else "candidate-source.sha256")
    assert digest(manifest) == (BASE_SHA if base else SOURCE_SHA)
    items = {}
    for line in manifest.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        relative = PurePosixPath(name)
        assert not relative.is_absolute() and ".." not in relative.parts and name not in items, name
        path = source / name
        assert path.is_file() and digest(path) == checksum, name
        data = path.read_bytes()
        items[name] = sha1(b"blob " + str(len(data)).encode() + b"\0" + data,
                           usedforsecurity=False).hexdigest()
    assert len(items) == (BASE_COUNT if base else SOURCE_COUNT)
    actual_files = {
        str(path.relative_to(source))
        for path in source.rglob("*")
        if path.is_file() and path.relative_to(source).parts[0] != ".git"
    }
    assert set(items) == actual_files, sorted(set(items) ^ actual_files)
    workspace = tomllib.loads((source / "Cargo.toml").read_text())
    assert workspace["workspace"]["package"]["version"] == ("0.1.48" if base else "0.1.49")
    assert (source / ".github/workflows/ci.yml").read_bytes() == (ROOT / "ci.yml").read_bytes()
    return items


def validate_tree(source_blobs, remote_tree):
    assert remote_tree["truncated"] is False
    remote_blobs = {item["path"]: item["sha"] for item in remote_tree["tree"] if item["type"] == "blob"}
    assert remote_blobs == source_blobs, {
        "different": sorted(k for k in remote_blobs.keys() & source_blobs.keys()
                            if remote_blobs[k] != source_blobs[k]),
        "paths": sorted(remote_blobs.keys() ^ source_blobs.keys()),
    }


def switch_stock_compatibility(original):
    return original.replace(
        "def runner_record(path, profile):", "def runner_record(path, profile, stock_last_material=277):"
    ).replace(
        "    expected_steps = SWITCH_STEPS if profile == \"focused\" else STOCK_STEPS",
        "    expected_steps = SWITCH_STEPS if profile == \"focused\" else [f\"M{i}\" for i in range(1, stock_last_material + 1) if i != 23]",
    ).replace(
        '"switch_steps": steps[-4:]', '"switch_steps": [step for step in steps if step in SWITCH_STEPS]',
    ).replace(
        "expected_source_sha, source_root=None):", "expected_source_sha, source_root=None, *, stock_last_material=277):",
    ).replace(
        "runner = runner_record(log, profile)", "runner = runner_record(log, profile, stock_last_material)",
    )


def bound_tools():
    assert SOURCE_SHA == "3a9aeda30a8ca3520afd04d9d4db8c26f3a900dc5aae20fad14e4d366d2544fe"
    assert SOURCE_COUNT == 602
    assert digest(ROOT / "candidate-source.sha256") == SOURCE_SHA
    assert digest(ROOT / "switch-base.sha256") == BASE_SHA
    assert digest(ROOT / "verify_checkbox_report.py") == CHECKBOX_VALIDATOR_SHA
    switch = ROOT / "switch-validator-original.py"
    assert digest(switch) == SWITCH_VALIDATOR_SHA
    assert (ROOT / "verify_switch_report.py").read_text() == switch_stock_compatibility(switch.read_text())
    navigation = ROOT / "navigation-validator-original.py"
    assert digest(navigation) == NAV_VALIDATOR_SHA
    compatible_navigation = navigation.read_text().replace(
        "def validate_log(log, platform, stock):",
        "def validate_log(log, platform, stock, stock_last_material=273):").replace(
        "    expected_steps = STOCK_STEPS if stock else NAV_STEPS",
        "    expected_steps = [f\"M{index}\" for index in range(1, stock_last_material + 1) if index != 23] if stock else NAV_STEPS").replace(
        "             expected_metadata=None, expect_swiftshader=False):",
        "             expected_metadata=None, expect_swiftshader=False, stock_last_material=273):").replace(
        "validate_log(log_path.read_text(), platform, stock)",
        "validate_log(log_path.read_text(), platform, stock, stock_last_material=stock_last_material)")
    assert (ROOT / "verify_navigation_bar.py").read_text() == compatible_navigation
    assert digest(ROOT / "verify_divider_report.py") == DIVIDER_VALIDATOR_SHA
    assert digest(ROOT / "verify_ime_cancel_report.py") == IME_VALIDATOR_SHA
    original = ROOT / "menu-validator-original.py"
    assert digest(original) == MENU_ORIGINAL_SHA
    # Use the unchanged Menu proof algorithm against the full Switch stock
    # log. Only the caller-supplied total Material step count is additive.
    compatible = original.read_text().replace('scope="focused", pre_footer=False):',
        'scope="focused", pre_footer=False, stock_material_steps=268):').replace(
        'step_count == (268 if stock else 3)', 'step_count == (stock_material_steps if stock else 3)')
    assert (ROOT / "verify_menu_report.py").read_text() == compatible


def release_base(pin, source):
    tag = api("git/ref/tags/v0.1.48")["object"]
    if tag["type"] == "tag":
        tag = api(f"git/tags/{tag['sha']}")["object"]
    assert tag["type"] == "commit" and tag["sha"] == pin
    release = api("releases/tags/v0.1.48")
    assert release["tag_name"] == "v0.1.48" and not release["draft"] and not release["prerelease"]
    commit = api(f"git/commits/{pin}")
    assert commit["sha"] == pin
    tree_sha = commit["tree"]["sha"]
    tree = api(f"git/trees/{tree_sha}?recursive=1")
    assert tree["sha"] == tree_sha
    validate_tree(read_source(source, base=True), tree)
    return {"release": "v0.1.48", "commit": pin, "tree": tree_sha,
            "source_manifest_sha256": BASE_SHA, "paths": BASE_COUNT, "release_url": release["html_url"]}


def bind_candidate(args):
    bound_tools()
    pr = api(f"pulls/{args.pr}")
    assert pr["number"] == args.pr
    assert (pr["head"]["ref"], pr["base"]["ref"]) == (BRANCH, "main")
    assert pr["head"]["repo"]["full_name"] == pr["base"]["repo"]["full_name"] == REPO
    # The current base can advance after the pinned run. The run's checkout
    # parents and the head's direct parent must still prove the specified base.
    assert pr["head"]["sha"] == args.head
    assert api(f"git/ref/heads/{BRANCH}")["object"]["sha"] == args.head
    published = release_base(args.base, args.switch_source)
    head = api(f"git/commits/{args.head}")
    assert head["tree"]["sha"] == args.tree
    assert [parent["sha"] for parent in head["parents"]] == [args.base]
    tree = api(f"git/trees/{args.tree}?recursive=1")
    assert tree["sha"] == args.tree
    validate_tree(read_source(args.source), tree)
    return {"release": "v0.1.49", "repo": REPO, "branch": BRANCH, "pr": args.pr, "head": args.head,
            "tree": args.tree, "base": args.base, "source_paths": SOURCE_COUNT,
            "source_manifest_sha256": SOURCE_SHA, "pr_url": pr["html_url"], "released_base": published}


def run_snapshot(event, run_id, attempt, args):
    run = api(f"actions/runs/{run_id}")
    assert (run["id"], run["head_sha"], run["event"], run["path"]) == (
        run_id, args.head, event, ".github/workflows/ci.yml")
    assert run["head_branch"] == BRANCH
    if event == "pull_request":
        pr, = [item for item in run["pull_requests"] if item["number"] == args.pr]
        assert (pr["head"]["sha"], pr["head"]["ref"], pr["base"]["sha"], pr["base"]["ref"]) == (
            args.head, BRANCH, args.base, "main")
    assert run["run_attempt"] == attempt, (event, run["run_attempt"], attempt)
    jobs = {}
    for job in api(f"actions/runs/{run_id}/attempts/{attempt}/jobs")["jobs"]:
        assert (job["run_id"], job["head_sha"], job["run_attempt"]) == (run_id, args.head, attempt)
        assert job["name"] in JOBS and job["name"] not in jobs
        jobs[job["name"]] = job
    return run, jobs


def normalize_log(text):
    return "\n".join(ANSI.sub("", TIME_PREFIX.sub("", row)) for row in text.splitlines()) + "\n"


def checkout(log):
    match = re.search(r"git log -1 --format=%H\n([0-9a-f]{40})\n", normalize_log(log))
    assert match, "The runner log did not identify the checked out commit."
    return match.group(1)


def coverage(log, last_material=LAST_MATERIAL):
    assert INVOCATION in log and SWIFTSHADER in log
    assert "Operating System" in log and re.search(r"^Ubuntu$", log, re.M)
    rows = CASE.findall(log)
    assert len(rows) == 8 and {record[0] for record in rows} == CASES, rows
    counts = {name: {"steps": int(steps), "checks": int(checks)} for name, steps, checks in rows}
    assert all(row["steps"] > 0 and row["checks"] > 0 for row in counts.values())
    material = [int(n) for n in re.findall(r"^\s*\[M(\d+)\]", log, re.M)]
    expected = [n for n in range(1, last_material + 1) if n != 23]
    assert material == expected and counts["material"]["steps"] == len(expected), material[-10:]
    ime = [int(n) for n in re.findall(r"^\s*\[I(\d+)\]", log, re.M)]
    assert ime == list(range(1, 21)) and counts["ime"] == {"steps": 20, "checks": 77}
    total = sum(row["checks"] for row in counts.values())
    assert f"e2e result: 8 passed; 0 failed; {total} checks;" in log
    assert not re.search(r"^case \S+ \.\.\. (?:FAILED|skipped)", log, re.M)
    return counts, total


def excerpt(log):
    lines = []
    for row in normalize_log(log).splitlines():
        if row.startswith("browser: "):
            version = re.match(r"browser: Chrome/[\d.]+", row)
            assert version, row
            lines.append(version.group())
        elif INVOCATION in row:
            lines.append(INVOCATION)
        elif "Operating System" in row or row == "Ubuntu" or SWIFTSHADER in row:
            lines.append(row)
        elif re.match(r"^\s*\[[MI]\d+\]|^case \S+ \.\.\.(?: |$)|^e2e result:", row):
            lines.append(row.split(" artifacts in ", 1)[0])
    return "\n".join(lines) + "\n"


def get_archive(run, job, destination, *, required):
    artifacts = api(f"actions/runs/{run['id']}/artifacts")["artifacts"]
    items = [row for row in artifacts if row["name"] == "surface-e2e-artifacts" and not row["expired"]]
    if not items and not required:
        return None
    assert len(items) == 1, "Required artifact missing or ambiguous across attempts."
    item = items[0]
    assert item["workflow_run"]["id"] == run["id"]
    assert item["workflow_run"]["head_sha"] == run["head_sha"]
    assert job["started_at"] <= item["created_at"] <= job["completed_at"], item["id"]
    assert re.fullmatch(r"sha256:[0-9a-f]{64}", item["digest"])
    for index in range(1, 4):
        download = destination.with_suffix(f".download-{index}")
        try:
            with download.open("xb") as output:
                subprocess.run(["gh", "api", "--method", "GET", f"repos/{REPO}/actions/artifacts/{item['id']}/zip"],
                               stdout=output, check=True)
            check_archive(download, item)
        except (AssertionError, subprocess.CalledProcessError):
            if index == 3:
                raise
            sleep(3 * index)
        else:
            assert not destination.exists()
            download.rename(destination)
            break
    return {key: item[key] for key in ("id", "name", "digest", "size_in_bytes", "created_at", "workflow_run")}


def check_archive(path, metadata):
    assert path.stat().st_size == metadata["size_in_bytes"]
    assert digest(path) == metadata["digest"].removeprefix("sha256:")


def extract_evidence(archive, out):
    with ZipFile(archive) as zipped:
        files = [info for info in zipped.infolist() if not info.is_dir()]
        assert len(files) == len({info.filename for info in files})
        by_name = {}
        scanned_errors = {}
        wanted = set(RAW_FILES)
        for info in files:
            relative = PurePosixPath(info.filename)
            assert not relative.is_absolute() and ".." not in relative.parts and "\\" not in info.filename
            name = relative.name
            if name in wanted:
                assert name not in by_name, name
                by_name[name] = info
            if name.startswith("browser-material") and name.endswith(".json"):
                errors = json.loads(zipped.read(info)).get("errors")
                if errors is not None:
                    assert errors == [] and name not in scanned_errors, name
                    scanned_errors[name] = 0
        assert set(by_name) == wanted, wanted - by_name.keys()
        assert len({str(PurePosixPath(info.filename).parent) for info in by_name.values()}) == 1
        records = {}
        for name, info in sorted(by_name.items()):
            data = zipped.read(info)
            if out is not None:
                (out / name).write_bytes(data)
            records[name] = {"path": info.filename, "bytes": info.file_size,
                             "crc32": f"{info.CRC:08x}", "sha256": sha256(data).hexdigest()}
    pngs_in_zip = {PurePosixPath(info.filename).name for info in files}
    for stem, expected in ((CHECKBOX_STEM, CHECKBOX_IMAGES), (SWITCH_STEM, SWITCH_IMAGES),
                           (NAV_STEM, NAV_IMAGES), (MENU_STEM, MENU_IMAGES),
                           ("material-divider", DIVIDER_IMAGES)):
        assert {name for name in pngs_in_zip if name.startswith(stem + "-") and name.endswith(".png")} == set(expected)
    assert set(MATERIAL_REPORTS) <= scanned_errors.keys()
    return {"members": records, "material_page_error_reports": scanned_errors}


def screenshots(raw, log, *, platform="linux", window_receipt=None):
    smoke, ime, side = [read_json(raw / name) for name in BASE_REPORTS]
    assert smoke["errors"] == ime["errors"] == side["errors"] == []
    assert {key: smoke["adapter"][key] for key in ADAPTER} == side["adapter"]
    if platform == "linux":
        assert side["adapter"] == ADAPTER
    assert smoke["browser"] == side["browser"] and smoke["browser"].startswith("Chrome/")
    assert ime["committed"] == "繁體中文、简体中文"
    observed = {"browser": smoke["browser"], "adapter": side["adapter"]}
    cancellation = verify_cancellations(raw)
    profile = {"linux": "stock-linux", "macos": "stock-macos"}[platform]
    adapter = {"linux": "swiftshader", "macos": "metal"}[platform]
    validator_args = (raw, log, profile, adapter, ROOT / "candidate-source.sha256", SOURCE_SHA)
    checkbox = verify_checkbox(*validator_args)
    switch = verify_switch(*validator_args, stock_last_material=LAST_MATERIAL)
    for report in (checkbox, switch):
        assert report["runner"]["browser"] == observed["browser"] and report["observed_adapter"] == observed["adapter"]
        assert report["runner"]["cases"]["material"]["steps"] == MATERIAL_COUNT
        assert len(report["screenshots"]) == 4 and report["source"]["files"] == SOURCE_COUNT
    assert tuple(checkbox["runner"]["checkbox_steps"]) == ("M278", "M279", "M280", "M281")
    assert tuple(switch["runner"]["switch_steps"]) == ("M274", "M275", "M276", "M277")
    if platform == "macos":
        # The newer guarded macOS runner retains its own CoreGraphics receipt
        # rather than the older Navigation-only attempt.json file.
        assert window_receipt is not None
        counts, steps, total, browsers = validate_log(log.read_text(), platform, True, LAST_MATERIAL)
        report = read_json(raw / f"browser-{NAV_STEM}.json")
        assert report["errors"] == [] and report["reduced_motion"] is True
        assert report["browser"] == observed["browser"] and report["adapter"] == observed["adapter"]
        assert browsers == {observed["browser"]}
        navigation = {"case_results": counts, "total_checks": total, "material_steps": len(steps),
                      "window_observation": mac_observation(window_receipt, True),
                      **validate_navigation(report, raw)}
    else:
        assert window_receipt is None
        navigation = verify_navigation(raw, platform=platform, scope="stock", log=log,
                                       expected_metadata=raw / "browser-material-side-sheet.json",
                                       expect_swiftshader=True, stock_last_material=LAST_MATERIAL)
    assert navigation["material_steps"] == MATERIAL_COUNT and len(navigation["pngs"]) == 4
    menu = verify_menu(raw, platform=platform, scope="stock", log=log, expected_metadata=observed,
                       stock_material_steps=MATERIAL_COUNT, window_receipt=window_receipt)
    assert menu["material_steps"] == MATERIAL_COUNT and len(menu["pngs"]) == 3
    divider = verify_dividers(raw, observed["browser"], observed["adapter"], log=log.read_text())
    assert len(divider["profiles"]) == 4
    for profile in divider["profiles"].values():
        assert all(item["physical_thickness_px"] in (2, 16)
                   and item["solid_pixels_checked"] > 0 for item in profile["lines"].values())
    return {**observed, "checkbox": checkbox, "switch": switch, "navigation": navigation, "menu": menu, "divider": divider,
            "ime_cancellation": cancellation, "page_errors": 0}


def job_details(jobs):
    return {name: {key: item[key] for key in ("id", "html_url", "status", "conclusion", "run_attempt",
                                             "started_at", "completed_at")}
            for name, item in sorted(jobs.items())}


def read_job_logs(event_dir, jobs):
    logs = {}
    for name, job in jobs.items():
        if job["status"] == "completed":
            data = get(f"actions/jobs/{job['id']}/logs")
            (event_dir / f"{name}.log").write_bytes(data)
            logs[name] = data.decode("utf-8", errors="replace")
    return logs


def collect(event, run, jobs, pin, workspace):
    event_dir = workspace / event
    event_dir.mkdir()
    write_json(event_dir / "status.json", {"run": run, "jobs": jobs})
    logs = read_job_logs(event_dir, jobs)
    browser = jobs.get("browser-e2e")
    ok = (run["conclusion"] == "success" and set(jobs) == set(JOBS)
          and all(job["status"] == "completed" and job["conclusion"] == "success" for job in jobs.values()))
    archive = event_dir / "surface-e2e-artifacts.zip"
    artifact = get_archive(run, browser, archive, required=ok) if browser and browser["status"] == "completed" else None
    if artifact:
        write_json(event_dir / "original-zip.json", artifact)
    if not ok:
        print(f"{event}: workflow did not pass. Original receipts in {event_dir}", flush=True)
        return None
    assert run["status"] == "completed" and set(logs) == set(JOBS)
    assert len({job["id"] for job in jobs.values()}) == len(JOBS)
    checkout_commit = checkout(logs["browser-e2e"])
    assert checkout(logs["build-and-behavior"]) == checkout_commit
    built = api(f"git/commits/{checkout_commit}")
    assert built["sha"] == checkout_commit and built["tree"]["sha"] == pin["tree"]
    parents = [parent["sha"] for parent in built["parents"]]
    if event == "pull_request":
        assert parents == [pin["base"], pin["head"]]
    else:
        assert checkout_commit == pin["head"] and parents == [pin["base"]]
    write_json(event_dir / "checkout-commit.json", built)
    counts, total = coverage(normalize_log(logs["browser-e2e"]))
    portable = workspace / "portable" / event
    raw = portable / "raw"
    raw.mkdir(parents=True)
    stock = portable / "runner-excerpt.log"
    stock.write_text(excerpt(logs["browser-e2e"]))
    assert coverage(stock.read_text()) == (counts, total)
    evidence = extract_evidence(archive, raw)
    measured = screenshots(raw, stock)
    for component in ("checkbox", "switch"):
        assert measured["navigation"]["case_results"] == measured[component]["runner"]["cases"] == counts
        assert measured["navigation"]["total_checks"] == measured[component]["runner"]["total_checks"] == total
    rejected = reject_controls(raw, stock, platform="linux", stock_last_material=LAST_MATERIAL)
    write_json(portable / "rejected-controls.json", rejected)
    status = {"event": event, "run": run["id"], "run_url": run["html_url"], "attempt": run["run_attempt"],
              "head": pin["head"], "checkout": checkout_commit, "checkout_tree": pin["tree"],
              "checkout_parents": parents, "checkouts_agree": True, "jobs": job_details(jobs),
              "original_log_sha256": {name: digest(event_dir / f"{name}.log") for name in JOBS},
              "original_zip": artifact, "artifact_bytes_verified": True, **evidence,
              "cases": counts, "total_checks": total, "rejection_controls_sha256": digest(portable / "rejected-controls.json"), **measured}
    write_json(portable / "verified.json", status)
    print(f"{event}: {total} checks, four Checkbox PNGs, and Switch, Navigation, Menu, Divider and I11 regressions.", flush=True)
    return status


def distinct_workflows(receipts):
    pull, push = [receipts[event] for event in EVENTS]
    assert pull["event"] == "pull_request" and push["event"] == "push"
    assert pull["head"] == push["head"], "Pull-request and push must attest the same candidate."
    for key in ("run", "checkout"):
        assert pull[key] != push[key], f"Both events reused one {key}."
    assert pull["original_zip"]["id"] != push["original_zip"]["id"], "Both events reused one ZIP."
    assert len({job["id"] for row in (pull, push) for job in row["jobs"].values()}) == 2 * len(JOBS), (
        "The two workflows need four distinct job IDs."
    )


def observe(pin, args):
    workspace = args.out.resolve()
    assert workspace.is_relative_to(Path("/tmp").resolve()) and workspace != Path("/tmp").resolve()
    workspace.mkdir(parents=False)
    write_json(workspace / "candidate.json", pin)
    requests = {"pull_request": (args.pull_run, args.pull_attempt), "push": (args.push_run, args.push_attempt)}
    receipts = {}
    for event, (run_id, attempt) in requests.items():
        run, jobs = run_snapshot(event, run_id, attempt, args)
        if run["status"] != "completed":
            raise SystemExit(f"{event} {run_id} is still pending. Use fresh /tmp output after completion.")
        receipts[event] = collect(event, run, jobs, pin, workspace)
    if not all(receipts.values()):
        raise SystemExit("At least one exact workflow failed or lacks valid receipts. See the original local evidence.")
    distinct_workflows(receipts)
    return workspace, receipts


def package(workspace, pin, receipts):
    portable = workspace / "portable"
    for name in ("candidate-pin.json", "candidate-source.sha256", "switch-base.sha256", "ci.yml",
                 "verify-hosted-checkbox.py", "verify_checkbox_report.py", "verify_switch_report.py",
                 "switch-validator-original.py", "switch-regression.patch", "verify_navigation_bar.py",
                 "navigation-validator-original.py", "navigation-regression.patch",
                 "verify_menu_report.py", "menu-validator-original.py", "menu-regression.patch",
                 "verify_divider_report.py", "verify_ime_cancel_report.py", "rejection_controls.py"):
        copyfile(ROOT / name, portable / name)
    summary = {**pin, "host": "GitHub Ubuntu under Xvfb, headless Chrome WebGPU SwiftShader",
               "workflows": {event: {"run": item["run"], "url": item["run_url"], "checks": item["total_checks"]}
                             for event, item in sorted(receipts.items())}}
    write_json(portable / "hosted-verified.json", summary)
    counts = {event: item["total_checks"] for event, item in receipts.items()}
    readme = ["# Surface v0.1.49 Checkbox hosted receipts", "",
              f"Candidate head `{pin['head']}`, tree `{pin['tree']}`, base `{pin['base']}`.",
              f"The exact {SOURCE_COUNT}-file source seal is `{SOURCE_SHA}`.", "",
              "Both stock workflows ran all eight cases on GitHub Ubuntu. Chrome used the observed",
              "SwiftShader adapter. Each workflow ran 280 ordered Material steps through M281.",
              "The verifier independently checked four Checkbox, four Switch, four Navigation, three Menu and four",
              "Divider PNGs, both I11 cancellation paths and strict Divider stripes.",
              "The Switch, Navigation and Menu validators accept the enclosing stock step count. Their",
              "image and behavior checks match the included original validators.", ""]
    for event in EVENTS:
        value = receipts[event]
        readme.append(f"- [{event}]({value['run_url']}) passed {value['total_checks']:,} checks.")
    readme += ["", "The original GitHub ZIP identities and member SHA-256 values are in each",
               "`verified.json`. The complete original ZIPs and logs remain in the local",
               "capture directory. This package contains selected unchanged artifact members.", "",
               "Run `shasum -a 256 -c SHA256SUMS` and",
               "`python3 -B verify-hosted-checkbox.py --verify-package . --originals ..` to check the package and original receipts.", ""]
    (portable / "README.md").write_text("\n".join(readme))
    pieces = sorted(path for path in portable.rglob("*") if path.is_file())
    manifest = portable / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(portable)}\n" for path in pieces))
    verify_package(portable, originals=workspace)
    print(json.dumps({"portable": str(portable), "manifest_sha256": digest(manifest),
                      "files": len(pieces), "checks_per_workflow": counts}, indent=2), flush=True)


def verify_package(portable, originals):
    checksums = {}
    for line in (portable / "SHA256SUMS").read_text().splitlines():
        checksum, name = line.split("  ", 1)
        path = PurePosixPath(name)
        assert not path.is_absolute() and ".." not in path.parts and name not in checksums
        assert digest(portable / name) == checksum, name
        checksums[name] = checksum
    assert set(checksums) == {str(path.relative_to(portable)) for path in portable.rglob("*")
                              if path.is_file() and path != portable / "SHA256SUMS"}
    bound_tools()
    assert read_json(portable / "candidate-pin.json") == CANDIDATE_PIN
    assert digest(portable / "candidate-source.sha256") == SOURCE_SHA
    assert digest(portable / "switch-base.sha256") == BASE_SHA
    aggregate = read_json(portable / "hosted-verified.json")
    assert aggregate["source_manifest_sha256"] == SOURCE_SHA and aggregate["source_paths"] == SOURCE_COUNT
    released = aggregate["released_base"]
    assert (released["commit"], released["release"], released["source_manifest_sha256"], released["paths"]) == (
        aggregate["base"], "v0.1.48", BASE_SHA, BASE_COUNT)
    assert all(re.fullmatch(r"[0-9a-f]{40}", oid) for oid in (
        aggregate["head"], aggregate["base"], aggregate["tree"], released["tree"]))
    assert set(aggregate["workflows"]) == set(EVENTS)
    receipts = {event: read_json(portable / event / "verified.json") for event in EVENTS}
    distinct_workflows(receipts)
    for event in EVENTS:
        target = portable / event
        row = receipts[event]
        assert row["event"] == event and row["head"] == aggregate["head"]
        assert row["checkout_tree"] == aggregate["tree"] and row["checkouts_agree"] is True
        assert row["original_zip"]["workflow_run"]["id"] == row["run"]
        assert row["original_zip"]["workflow_run"]["head_sha"] == row["head"]
        assert (row["run"], row["run_url"], row["total_checks"]) == tuple(
            aggregate["workflows"][event][key] for key in ("run", "url", "checks"))
        expected_parents = [aggregate["base"]]
        if event == "pull_request":
            expected_parents.append(aggregate["head"])
        else:
            assert row["checkout"] == row["head"]
        assert row["checkout_parents"] == expected_parents
        assert set(row["jobs"]) == set(JOBS)
        assert all(job["run_attempt"] == row["attempt"] and (job["status"], job["conclusion"]) == ("completed", "success")
                   for job in row["jobs"].values())
        assert row["artifact_bytes_verified"] is True
        raw = target / "raw"
        assert set(row["members"]) == {path.name for path in raw.iterdir()} == set(RAW_FILES)
        for name, item in row["members"].items():
            assert (raw / name).stat().st_size == item["bytes"] and digest(raw / name) == item["sha256"], name
        original = originals / event
        status = read_json(original / "status.json")
        run = status["run"]
        assert (run["event"], run["id"], run["head_sha"], run["run_attempt"]) == (
            event, row["run"], row["head"], row["attempt"])
        assert (run["status"], run["conclusion"]) == ("completed", "success")
        assert run["path"] == ".github/workflows/ci.yml" and run["head_branch"] == aggregate["branch"]
        assert job_details(status["jobs"]) == row["jobs"]
        assert all((job["run_id"], job["head_sha"]) == (row["run"], row["head"])
                   for job in status["jobs"].values())
        assert read_json(original / "original-zip.json") == row["original_zip"]
        built = read_json(original / "checkout-commit.json")
        assert built["sha"] == row["checkout"] and built["tree"]["sha"] == row["checkout_tree"]
        assert [parent["sha"] for parent in built["parents"]] == expected_parents
        archive = original / "surface-e2e-artifacts.zip"
        check_archive(archive, row["original_zip"])
        archive_receipt = extract_evidence(archive, None)
        assert all(archive_receipt[key] == row[key] for key in archive_receipt)
        assert set(row["original_log_sha256"]) == set(JOBS)
        for name, checksum in row["original_log_sha256"].items():
            job_log = original / f"{name}.log"
            assert digest(job_log) == checksum and checkout(job_log.read_text()) == row["checkout"]
        log = target / "runner-excerpt.log"
        browser_log = (original / "browser-e2e.log").read_text()
        assert excerpt(browser_log) == log.read_text()
        assert coverage(normalize_log(browser_log)) == (row["cases"], row["total_checks"])
        assert coverage(log.read_text()) == (row["cases"], row["total_checks"])
        measured = json.loads(json.dumps(screenshots(raw, log)))
        assert all(row[key] == value for key, value in measured.items()), event
        controls = target / "rejected-controls.json"
        assert digest(controls) == row["rejection_controls_sha256"]
        assert read_json(controls) == reject_controls(raw, log, platform="linux", stock_last_material=LAST_MATERIAL)
    return aggregate


def main():
    global BRANCH
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, help="Exact sealed Checkbox source directory")
    parser.add_argument("--switch-source", type=Path, help="Released Switch source for the named base")
    parser.add_argument("--branch", default=BRANCH, help="Exact Checkbox branch")
    parser.add_argument("--out", type=Path, help="Fresh directory under /tmp")
    parser.add_argument("--pr", type=int)
    parser.add_argument("--head")
    parser.add_argument("--tree")
    parser.add_argument("--base")
    parser.add_argument("--pull-run", type=int)
    parser.add_argument("--push-run", type=int)
    parser.add_argument("--pull-attempt", type=int, default=1)
    parser.add_argument("--push-attempt", type=int, default=1)
    parser.add_argument("--check-sources", action="store_true", help="verify both local source seals without GitHub")
    parser.add_argument("--verify-package", type=Path,
                        help="Verify a sealed local proof instead of calling GitHub")
    parser.add_argument("--originals", type=Path, help="Required original ZIPs and job logs for --verify-package")
    args = parser.parse_args()
    if not __debug__:
        parser.error("Run without -O so verification assertions stay enabled.")
    if args.check_sources:
        if args.source is None or args.switch_source is None:
            parser.error("--check-sources needs --source and --switch-source")
        bound_tools()
        candidate = read_source(args.source)
        base = read_source(args.switch_source, base=True)
        print(json.dumps({"candidate": {"sha256": SOURCE_SHA, "files": len(candidate)},
                          "switch_base": {"sha256": BASE_SHA, "files": len(base)}}, indent=2))
        return
    if args.verify_package:
        if args.originals is None:
            parser.error("--verify-package requires --originals.")
        print(json.dumps(verify_package(args.verify_package, args.originals), indent=2))
        return
    required = ("source", "switch_source", "out", "pr", "head", "tree", "base", "pull_run", "push_run")
    if missing := [flag.replace("_", "-") for flag in required if getattr(args, flag) is None]:
        parser.error("Required to verify GitHub: " + ", ".join(f"--{flag}" for flag in missing))
    for oid in (args.head, args.tree, args.base):
        if not re.fullmatch(r"[0-9a-f]{40}", oid):
            parser.error("Head, base and tree must be full lowercase Git object IDs.")
    if min(args.pr, args.pull_run, args.push_run, args.pull_attempt, args.push_attempt) < 1:
        parser.error("PR number, run IDs, and attempts must be positive.")
    if args.pull_run == args.push_run:
        parser.error("Supply separate pull-request and push runs.")
    if args.head == args.base:
        parser.error("Supply distinct candidate and released-base commits.")
    BRANCH = args.branch
    pin = bind_candidate(args)
    workspace, receipts = observe(pin, args)
    package(workspace, pin, receipts)


if __name__ == "__main__":
    main()
