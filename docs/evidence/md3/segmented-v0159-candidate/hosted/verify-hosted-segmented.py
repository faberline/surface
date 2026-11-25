#!/usr/bin/env python3
"""Verify exact Baseline Segmented Button v0.1.59 GitHub jobs using read-only API calls and original artifacts."""

import argparse
from hashlib import file_digest, sha1, sha256
import json
from pathlib import Path, PurePosixPath
import re
from shutil import copyfile
import stat
import subprocess
import tomllib
import traceback
from time import monotonic, sleep
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
from verify_radio_report import SHOTS as RADIO_SHOTS, STEM as RADIO_STEM, SUITES, validate as verify_radio
from verify_tabs_report import IMAGES as TABS_IMAGES, REPORT as TABS_REPORT, STEM as TABS_STEM, validate as verify_tabs
from verify_chips_report import IMAGES as CHIPS_IMAGES, REPORT as CHIPS_REPORT, STEM as CHIPS_STEM, validate as verify_chips
from verify_icon_buttons_report import IMAGES as ICON_IMAGES, REPORT as ICON_REPORT, STEM as ICON_STEM, validate as verify_icons
from icon_rejection_controls import controls as reject_icons
from verify_sliders_report import IMAGES as SLIDER_IMAGES, REPORT as SLIDER_REPORT, STEM as SLIDER_STEM, validate as verify_sliders
from slider_rejection_controls import controls as reject_sliders
from verify_slider_forms_report import IMAGES as FORMS_IMAGES, REPORT as FORMS_REPORT, STEM as FORMS_STEM, validate as verify_forms
from slider_forms_rejection_controls import controls as reject_forms
from verify_range_pixels import CAPTURES as RANGE_CAPTURES, REPORT as RANGE_REPORT, verify as verify_range
from range_rejection_controls import controls as reject_range, replay_steps as range_steps
from verify_refinement_pixels import (
    CENTERED as CENTERED_REPORT, INSET as INSET_REPORT, CENTERED_IMAGES, INSET_IMAGES,
    verify as verify_refinements,
)
from refinement_rejection_controls import controls as reject_refinements, replay_steps as refinement_steps
from verify_button_pixels import IMAGES as BUTTON_IMAGES, REPORT as BUTTON_REPORT, STEM as BUTTON_STEM, verify as verify_buttons
from button_rejection_controls import controls as reject_buttons
from verify_segmented_pixels import IMAGES as SEGMENTED_IMAGES, REPORT as SEGMENTED_REPORT, STEM as SEGMENTED_STEM, verify as verify_segmented
from rejection_controls import controls as reject_controls
from tabs_rejection_controls import controls as reject_tabs
from chips_rejection_controls import controls as reject_chips


ROOT = Path(__file__).resolve().parent
BRANCH = "codex/md3-segmented-buttons"
REPO = "faberline/surface"
CANDIDATE_PIN = json.loads((ROOT / "candidate-pin.json").read_text())
SOURCE_SHA = CANDIDATE_PIN["source_manifest_sha256"]
SOURCE_COUNT = CANDIDATE_PIN["source_paths"]
BASE_SHA = "b03623198811629a68a2dfe5620d5705c55ea7744157ff2ac732d174833eedbf"
BASE_COUNT = 656
NAV_VALIDATOR_SHA = "53eb0fa5d730a1736be585fc8c78da290a5d17c919f5de6ec0cd11793adc22f2"
DIVIDER_VALIDATOR_SHA = "54091473417fec52c7c9930a01a8acf4d1301a3e55cef36746f04855332b208d"
MENU_ORIGINAL_SHA = "44211cd13e2477ea2fbf6c602c9270142356c102a12b244d3fa1171712082d31"
IME_VALIDATOR_SHA = "46169df81e3b5a36df5863acdf330ef7884a88d54fdad4d9067fff691f3f30c7"
SWITCH_VALIDATOR_SHA = "2023c287d0e8edb9ac1830f8b85e7c3d43e9b923a26050e90953fb061aa25151"
CHECKBOX_VALIDATOR_SHA = "47ce6d5f1d0184b9cb78b6e06b19f1d50e80f0621782ba0475799672762fa75c"
RADIO_VALIDATOR_SHA = "4551cb70c8dd0f2d877de0db100c3a6ad7dcb7d79efdb97372435894e685f1b0"
TABS_VALIDATOR_SHA = "baecc443bda843d6a47e36538aef16b0ec12f9a9e1f747df5d32fd924d4318f2"
TABS_CONTROL_SHA = "889c1e79d7d54abff77b29876f45216c404e3cb03c80afd0f6064fabcd8db37a"
CHIPS_VALIDATOR_SHA = "ed0bb66e35ac8860eab4c123094dc7b859050ddb916de84db0c1da96758ac02e"
CHIPS_CONTROL_SHA = "93e1e58072f3bb101282b937222529c5c5cfd5edd428313f2ab4fe7882170a2a"
ICON_VALIDATOR_SHA = "c22931ba2eefa61db65b33cf83c041ed025891ecefa2dbb8ac12be013d2046eb"
ICON_CONTROL_SHA = "b987fdb29ccc62dd54f048ee92f79cc5714bdb55a3718ad87ef9cd000ca4a547"
SLIDER_VALIDATOR_SHA = "8a6f3a474a310d7ce73cf8b2b005655686970d28c607c7fae822d9494076dea6"
SLIDER_CONTROL_SHA = "2bed74002e2fc60924dd3c4921c0e6ca77571408369595b019a8f7948a458748"
FORMS_ORIGINAL_SHA = "13d2a3b233e36fbbf44410c3b2b4f213a6832e4495670ed54976fce2d6e62b95"
FORMS_VALIDATOR_SHA = "2947aa2c288de87f215b7b6890c704b583d885e5bf5eac9d0140eb1c7b1b2472"
FORMS_CONTROL_SHA = "2098b4cc3991ff7fa170da6c965aff2ef4b92d19947fcceedac42e025f3c2c32"
WORKFLOW_SHA = "de86d826cdacfd7e013ec2bc3083255c345454b6bb479fd2f26ad154f1196244"
PATCH_SHA = "c76e282e1b634e597705e5e9306c7dedd2e43dbb7bc6e4aaebb1f672bb8d7766"
LAST_MATERIAL = 323
MATERIAL_COUNT = 322
EVENTS = ("pull_request", "push")
CASES = SUITES["stock-linux"]
JOBS = ("build-and-behavior", "browser-e2e")
ADAPTER = {"isFallbackAdapter": True, "vendor": "google", "architecture": "swiftshader"}
CANCEL_RECEIPTS = ("browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json")
BASE_REPORTS = ("browser-smoke.json", "browser-ime.json", "browser-material-side-sheet.json")
MATERIAL_REPORTS = (SEGMENTED_REPORT, BUTTON_REPORT, CENTERED_REPORT, INSET_REPORT, RANGE_REPORT, FORMS_REPORT, SLIDER_REPORT, ICON_REPORT, CHIPS_REPORT, TABS_REPORT, f"browser-{RADIO_STEM}.json", f"browser-{CHECKBOX_STEM}.json",
                    f"browser-{SWITCH_STEM}.json", f"browser-{NAV_STEM}.json",
                    f"browser-{MENU_STEM}.json", "browser-material-divider.json")
RADIO_IMAGES = tuple(f"{RADIO_STEM}-{mode}.png" for mode in RADIO_SHOTS)
CHECKBOX_IMAGES = tuple(f"{CHECKBOX_STEM}-{mode}.png" for mode in CHECKBOX_SHOTS)
SWITCH_IMAGES = tuple(f"{SWITCH_STEM}-{mode}.png" for mode in SWITCH_SHOTS)
NAV_IMAGES = tuple(f"{NAV_STEM}-{mode}.png" for mode in NAV_SHOTS)
MENU_IMAGES = tuple(f"{MENU_STEM}-{mode}.png" for mode in VIEWPORTS)
DIVIDER_IMAGES = tuple(f"material-divider-{name}.png" for name, *_ in PROFILES.values())
RANGE_STEM = "material-range-slider"
RANGE_IMAGES = tuple(name for name, _ in RANGE_CAPTURES.values())
REFINEMENT_IMAGES = tuple(name for name, _ in (*CENTERED_IMAGES.values(), *INSET_IMAGES.values()))
RAW_FILES = (*SEGMENTED_IMAGES, *BUTTON_IMAGES, *REFINEMENT_IMAGES, *BASE_REPORTS, *MATERIAL_REPORTS, *CANCEL_RECEIPTS, *RANGE_IMAGES, *FORMS_IMAGES, *SLIDER_IMAGES, *ICON_IMAGES, *CHIPS_IMAGES, *TABS_IMAGES, *RADIO_IMAGES, *CHECKBOX_IMAGES, *SWITCH_IMAGES,
             *NAV_IMAGES, *MENU_IMAGES, *DIVIDER_IMAGES)
SWIFTSHADER = 'SURFACE_CHROME_ARGS: --enable-unsafe-webgpu --enable-features=Vulkan --use-vulkan=swiftshader --use-webgpu-adapter=swiftshader --use-angle=swiftshader'
INVOCATION = 'Run xvfb-run -a -s "-screen 0 1920x1080x24" cargo xtask web e2e'
TIME_PREFIX = re.compile(r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z ")
ANSI = re.compile(r"\x1b\[[\d;]*m")
CASE = re.compile(r"^case (\S+) \.\.\. ok \((\d+) steps, (\d+) checks,", re.M)
CASE_START = re.compile(r"^case (\S+) \.\.\.$", re.M)
EXCERPT_LINE = re.compile(r"^\s*\[[MI]\d+\]|^case \S+ \.\.\.(?= |$)|^e2e result:")


def read_json(path):
    return json.loads(path.read_text())


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def digest(path):
    with path.open("rb") as handle:
        return file_digest(handle, "sha256").hexdigest()


def get(path):
    return subprocess.run(["gh", "api", "--method", "GET", f"repos/{REPO}/{path}"],
                          stdout=subprocess.PIPE, timeout=60, check=True).stdout


def api(path):
    return json.loads(get(path))


def read_source(source, *, base=False):
    source = source.resolve(strict=True)
    manifest = ROOT / ("buttons-base.sha256" if base else "candidate-source.sha256")
    assert digest(manifest) == (BASE_SHA if base else SOURCE_SHA)
    items = {}
    tree = {}
    for line in manifest.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        relative = PurePosixPath(name)
        assert not relative.is_absolute() and ".." not in relative.parts and name not in items, name
        path = source / name
        assert path.is_file() and not path.is_symlink() and digest(path) == checksum, name
        data = path.read_bytes()
        items[name] = sha1(b"blob " + str(len(data)).encode() + b"\0" + data,
                           usedforsecurity=False).hexdigest()
        directory = tree
        for part in relative.parts[:-1]:
            directory = directory.setdefault(part, {})
        mode = b"100755" if path.stat().st_mode & stat.S_IXUSR else b"100644"
        directory[relative.name] = (mode, bytes.fromhex(items[name]))
    assert len(items) == (BASE_COUNT if base else SOURCE_COUNT)
    actual_files = {
        str(path.relative_to(source))
        for path in source.rglob("*")
        if path.is_file() and path.relative_to(source).parts[0] != ".git"
    }
    assert set(items) == actual_files, sorted(set(items) ^ actual_files)
    workspace = tomllib.loads((source / "Cargo.toml").read_text())
    assert workspace["workspace"]["package"]["version"] == ("0.1.58" if base else "0.1.59")
    assert (source / ".github/workflows/ci.yml").read_bytes() == (ROOT / "ci.yml").read_bytes()
    assert git_tree(tree).hex() == CANDIDATE_PIN["base_tree" if base else "candidate_tree"], "local Git tree differs"
    return items


def git_tree(directory):
    ordered = sorted(directory, key=lambda name: name.encode() + (b"/" if isinstance(directory[name], dict) else b""))
    entries = []
    for name in ordered:
        item = directory[name]
        mode, digest_bytes = (b"40000", git_tree(item)) if isinstance(item, dict) else item
        entries.append(mode + b" " + name.encode() + b"\0" + digest_bytes)
    data = b"".join(entries)
    return sha1(b"tree " + str(len(data)).encode() + b"\0" + data, usedforsecurity=False).digest()


def validate_tree(source_blobs, remote_tree):
    assert remote_tree["truncated"] is False
    blobs = [item for item in remote_tree["tree"] if item["type"] == "blob"]
    remote_blobs = {item["path"]: item["sha"] for item in blobs}
    assert len(blobs) == len(remote_blobs), "duplicate remote tree blob"
    assert remote_blobs == source_blobs, {
        "different": sorted(k for k in remote_blobs.keys() & source_blobs.keys()
                            if remote_blobs[k] != source_blobs[k]),
        "paths": sorted(remote_blobs.keys() ^ source_blobs.keys()),
    }


def prior_stock_compatibility(original, component, last):
    prefix = component.upper()
    compatible = original.replace(
        "def runner_record(path, profile):", f"def runner_record(path, profile, stock_last_material={last}):"
    ).replace(
        f'    expected_steps = {prefix}_STEPS if profile == "focused" else STOCK_STEPS',
        f'    expected_steps = {prefix}_STEPS if profile == "focused" else '
        '[f"M{i}" for i in range(1, stock_last_material + 1) if i != 23]',
    ).replace(
        f'"{component}_steps": steps[-4:]', f'"{component}_steps": [step for step in steps if step in {prefix}_STEPS]',
    ).replace(
        "expected_source_sha, source_root=None):", f"expected_source_sha, source_root=None, *, stock_last_material={last}):",
    ).replace(
        "runner = runner_record(log, profile)", "runner = runner_record(log, profile, stock_last_material)",
    )
    if component == "radio":
        compatible = compatible.replace(
            "def runner_record(log, profile):", f"def runner_record(log, profile, stock_last_material={last}):",
        ).replace(
            "expected_sha, source_root=None):", f"expected_sha, source_root=None, *, stock_last_material={last}):",
        )
    if component == "checkbox":
        compatible = compatible.replace(
            "def manifest_record(path, expected, source_root):",
            "def manifest_record(path, expected, source_root, source_file_count=602):",
        ).replace(
            "    need(len(entries) == 602,", "    need(len(entries) == source_file_count,",
        ).replace(
            "*, stock_last_material=281):", "*, stock_last_material=281, source_file_count=602):",
        ).replace(
            "manifest_record(manifest, expected_source_sha, source_root)",
            "manifest_record(manifest, expected_source_sha, source_root, source_file_count)",
        )
    return compatible


def bound_tools():
    assert SOURCE_SHA == "9051c16e996c9b310062682765a7d0bc54cf42dfbadbda4c5f63bd457b5d53b8"
    assert SOURCE_COUNT == 660
    assert digest(ROOT / "candidate-source.sha256") == SOURCE_SHA
    assert digest(ROOT / "buttons-base.sha256") == BASE_SHA
    assert digest(ROOT / "segmented-v0159.patch") == PATCH_SHA
    assert CANDIDATE_PIN["version"] == "0.1.59"
    assert CANDIDATE_PIN["candidate_tree"] == "757179e260a331b4488c6be1b49410263097d675"
    assert CANDIDATE_PIN["base_tree"] == "8ba487496a2816f3bff3363328ba0dc40a059c30"
    assert digest(ROOT / "verify_range_pixels.py") == "eb0b46d89b9c73dbe933b9d592077ccde9cbc75bdb379fef97f2aedb1c2cd40d"
    assert digest(ROOT / "range_rejection_controls.py") == "12522485816a72010d4dd0ea1593415b9e146fba30fea3d353ba8b2eafea9105"
    assert digest(ROOT / "verify_segmented_pixels.py") == "5ae7f8c42274f34283cf484be877f6de1ed4eeb6e8c43abe0d87e99311e9f796"
    assert digest(ROOT / "verify_button_pixels.py") == "7a5fcd3de0ae07b25878be91a40d1aea4bfaced55ea14eb67bb1988d063b987c"
    assert digest(ROOT / "button_rejection_controls.py") == "6dcaa7060ca2d4d30ab65d3baf3488a77b66472b07a92812f7e5902bf7be1ad4"
    assert digest(ROOT / "verify_refinement_pixels.py") == "2bc808755ceb6d0affa6cbb7102abb77b9bc245e08e4d5cf549f266e55fdb3a2"
    assert digest(ROOT / "refinement_rejection_controls.py") == "92f3aa62506fd4159cf2e807208905b989c6cb7b01ffff98db4f2f0b7b44da75"
    assert digest(ROOT / "verify_forms_pixels.py") == FORMS_ORIGINAL_SHA
    assert digest(ROOT / "verify_slider_forms_report.py") == FORMS_VALIDATOR_SHA
    assert digest(ROOT / "slider_forms_rejection_controls.py") == FORMS_CONTROL_SHA
    assert digest(ROOT / "ci.yml") == WORKFLOW_SHA
    assert digest(ROOT / "verify_tabs_report.py") == TABS_VALIDATOR_SHA
    assert digest(ROOT / "tabs_rejection_controls.py") == TABS_CONTROL_SHA
    assert digest(ROOT / "verify_icon_buttons_report.py") == ICON_VALIDATOR_SHA
    assert digest(ROOT / "icon_rejection_controls.py") == ICON_CONTROL_SHA
    assert digest(ROOT / "verify_sliders_report.py") == SLIDER_VALIDATOR_SHA
    assert digest(ROOT / "slider_rejection_controls.py") == SLIDER_CONTROL_SHA
    assert digest(ROOT / "verify_chips_report.py") == CHIPS_VALIDATOR_SHA
    assert digest(ROOT / "chips_rejection_controls.py") == CHIPS_CONTROL_SHA
    for component, last, checksum in (("radio", 285, RADIO_VALIDATOR_SHA), ("checkbox", 281, CHECKBOX_VALIDATOR_SHA), ("switch", 277, SWITCH_VALIDATOR_SHA)):
        original = ROOT / f"{component}-validator-original.py"
        assert digest(original) == checksum
        assert (ROOT / f"verify_{component}_report.py").read_text() == prior_stock_compatibility(
            original.read_text(), component, last)
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
    tag = api("git/ref/tags/v0.1.58")["object"]
    if tag["type"] == "tag":
        tag = api(f"git/tags/{tag['sha']}")["object"]
    assert tag["type"] == "commit" and tag["sha"] == pin
    release = api("releases/tags/v0.1.58")
    assert release["tag_name"] == "v0.1.58" and not release["draft"] and not release["prerelease"]
    assert release["published_at"] is not None
    commit = api(f"git/commits/{pin}")
    assert commit["sha"] == pin
    tree_sha = commit["tree"]["sha"]
    tree = api(f"git/trees/{tree_sha}?recursive=1")
    assert tree["sha"] == tree_sha == CANDIDATE_PIN["base_tree"]
    validate_tree(read_source(source, base=True), tree)
    return {"release": "v0.1.58", "commit": pin, "tree": tree_sha,
            "source_manifest_sha256": BASE_SHA, "paths": BASE_COUNT, "release_url": release["html_url"]}


def bind_candidate(args):
    bound_tools()
    assert (args.pr, args.head, args.base) == tuple(
        CANDIDATE_PIN[key] for key in ("pr", "head", "base")), "wrong exact draft, head or published base"
    assert (args.pull_run, args.pull_attempt, args.push_run, args.push_attempt) == tuple(
        CANDIDATE_PIN["runs"][event][key] for event in EVENTS for key in ("id", "attempt")), "wrong exact attempt IDs"
    assert args.tree == CANDIDATE_PIN["candidate_tree"], "wrong sealed Baseline Segmented Button tree"
    pr = api(f"pulls/{args.pr}")
    assert pr["number"] == args.pr and pr["state"] == "open" and pr["draft"] is True
    assert (pr["head"]["ref"], pr["base"]["ref"]) == (BRANCH, "main")
    assert pr["head"]["repo"]["full_name"] == pr["base"]["repo"]["full_name"] == REPO
    assert pr["head"]["sha"] == args.head and pr["base"]["sha"] == args.base
    assert api("git/ref/heads/main")["object"]["sha"] == args.base
    assert api(f"git/ref/heads/{BRANCH}")["object"]["sha"] == args.head
    published = release_base(args.base, args.buttons_source)
    head = api(f"git/commits/{args.head}")
    assert head["tree"]["sha"] == args.tree
    assert [parent["sha"] for parent in head["parents"]] == [args.base]
    tree = api(f"git/trees/{args.tree}?recursive=1")
    assert tree["sha"] == args.tree
    validate_tree(read_source(args.source), tree)
    return {"release": "v0.1.59", "repo": REPO, "branch": BRANCH, "pr": args.pr, "head": args.head,
            "tree": args.tree, "base": args.base, "source_paths": SOURCE_COUNT,
            "source_manifest_sha256": SOURCE_SHA, "pr_url": pr["html_url"], "released_base": published}


def run_snapshot(event, run_id, attempt, args):
    run = api(f"actions/runs/{run_id}/attempts/{attempt}")
    assert (run["id"], run["head_sha"], run["event"], run["path"]) == (
        run_id, args.head, event, ".github/workflows/ci.yml")
    assert run["head_branch"] == BRANCH
    assert run["repository"]["full_name"] == run["head_repository"]["full_name"] == REPO
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
    pinned = CANDIDATE_PIN["runs"][event]
    assert (run_id, attempt) == (pinned["id"], pinned["attempt"])
    assert all(job["id"] == pinned["jobs"][name] for name, job in jobs.items()), "wrong exact job IDs"
    if run["status"] == "completed":
        assert set(jobs) == set(pinned["jobs"]), "incomplete exact attempt job inventory"
    return run, jobs


def normalize_log(text):
    return "\n".join(ANSI.sub("", TIME_PREFIX.sub("", row)) for row in text.splitlines()) + "\n"


def checkout(log):
    commits = re.findall(r"git log -1 --format=%H\n([0-9a-f]{40})\n", normalize_log(log))
    assert len(commits) == 1, "The original log must identify one checkout."
    return commits[0]


def coverage(log, last_material=LAST_MATERIAL):
    assert INVOCATION in log and SWIFTSHADER in log
    assert "Operating System" in log and re.search(r"^Ubuntu$", log, re.M)
    rows = CASE.findall(log)
    assert CASE_START.findall(log) == [record[0] for record in rows] == CASES, rows
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
        elif EXCERPT_LINE.match(row):
            lines.append(row.split(" artifacts in ", 1)[0])
    return "\n".join(lines) + "\n"


def check_artifact_identity(item, run, job, expected=None):
    origin = item["workflow_run"]
    assert item["name"] == "surface-e2e-artifacts"
    assert expected is None or item["id"] == expected
    assert (origin["id"], origin["head_sha"], origin["head_branch"]) == (
        run["id"], run["head_sha"], run["head_branch"])
    assert (origin["repository_id"], origin["head_repository_id"]) == (
        run["repository"]["id"], run["head_repository"]["id"])
    assert job["started_at"] <= item["created_at"] <= job["completed_at"], item["id"]
    assert re.fullmatch(r"sha256:[0-9a-f]{64}", item["digest"])


def get_archive(run, job, destination, *, required, expected=None):
    artifacts = api(f"actions/runs/{run['id']}/artifacts")["artifacts"]
    items = [row for row in artifacts if row["name"] == "surface-e2e-artifacts" and not row["expired"]
             and job["started_at"] <= row["created_at"] <= job["completed_at"]
             and (expected is None or row["id"] == expected)]
    if not items and not required:
        return None
    assert len(items) == 1, "Required artifact missing or ambiguous across attempts."
    item = items[0]
    check_artifact_identity(item, run, job, expected)
    for index in range(1, 4):
        download = destination.with_suffix(f".download-{index}")
        try:
            with download.open("xb") as output:
                subprocess.run(["gh", "api", "--method", "GET", f"repos/{REPO}/actions/artifacts/{item['id']}/zip"],
                               stdout=output, timeout=180, check=True)
            check_archive(download, item)
        except (AssertionError, subprocess.SubprocessError):
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
        page_error_reports = (SEGMENTED_REPORT, BUTTON_REPORT, CENTERED_REPORT, INSET_REPORT, RANGE_REPORT, FORMS_REPORT, SLIDER_REPORT, ICON_REPORT, TABS_REPORT, CHIPS_REPORT)
        for info in files:
            relative = PurePosixPath(info.filename)
            assert not relative.is_absolute() and ".." not in relative.parts and "\\" not in info.filename
            name = relative.name
            if name in wanted:
                assert name not in by_name, name
                by_name[name] = info
            if name in page_error_reports or name.startswith("browser-material") and name.endswith(".json"):
                report = json.loads(zipped.read(info))
                errors = report["page_errors"] if name in page_error_reports else report.get("errors")
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
    for stem, expected in ((SEGMENTED_STEM, SEGMENTED_IMAGES), (BUTTON_STEM, BUTTON_IMAGES),
                           ("material-vertical-centered", [name for name, _ in CENTERED_IMAGES.values()]),
                           ("material-slider-inset", [name for name, _ in INSET_IMAGES.values()]),
                           (RANGE_STEM, RANGE_IMAGES), (FORMS_STEM, FORMS_IMAGES), (SLIDER_STEM, SLIDER_IMAGES), (ICON_STEM, ICON_IMAGES), (CHIPS_STEM, CHIPS_IMAGES), (TABS_STEM, TABS_IMAGES), (RADIO_STEM, RADIO_IMAGES), (CHECKBOX_STEM, CHECKBOX_IMAGES), (SWITCH_STEM, SWITCH_IMAGES),
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
    segmented = verify_segmented(raw, observed, log=log)
    assert len(segmented["captures"]) == 5
    buttons = verify_buttons(raw, observed, log=log)
    assert len(buttons["captures"]) == 7
    refinement_steps(log)
    refinements = verify_refinements(raw, observed)
    range_steps(log)
    ranges = verify_range(raw, observed)
    forms = verify_forms(raw, log, observed)
    sliders = verify_sliders(raw, log, observed)
    icons = verify_icons(raw, log, observed)
    chips = verify_chips(raw, log, observed)
    tabs = verify_tabs(raw, log, observed)
    radio = verify_radio(*validator_args, stock_last_material=LAST_MATERIAL)
    checkbox = verify_checkbox(*validator_args, stock_last_material=LAST_MATERIAL, source_file_count=SOURCE_COUNT)
    switch = verify_switch(*validator_args, stock_last_material=LAST_MATERIAL)
    for report in (checkbox, switch):
        assert report["runner"]["browser"] == observed["browser"] and report["observed_adapter"] == observed["adapter"]
        assert report["runner"]["cases"]["material"]["steps"] == MATERIAL_COUNT
        assert len(report["screenshots"]) == 4 and report["source"]["files"] == SOURCE_COUNT
    assert tuple(checkbox["runner"]["checkbox_steps"]) == ("M278", "M279", "M280", "M281")
    assert tuple(switch["runner"]["switch_steps"]) == ("M274", "M275", "M276", "M277")
    assert tuple(radio["runner"]["radio_steps"]) == ("M282", "M283", "M284", "M285")
    assert radio["runner"]["browser"] == observed["browser"] and radio["adapter"] == observed["adapter"]
    assert radio["runner"]["material_steps"] == MATERIAL_COUNT
    assert len(radio["pngs"]) == 4 and radio["source"]["files"] == SOURCE_COUNT
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
    return {**observed, "segmented": segmented, "common_buttons": buttons, "slider_refinements": refinements, "range_slider": ranges, "slider_forms": forms, "sliders": sliders, "icon_buttons": icons, "chips": chips, "tabs": tabs, "radio": radio, "checkbox": checkbox, "switch": switch,
            "navigation": navigation, "menu": menu, "divider": divider, "ime_cancellation": cancellation, "page_errors": 0}


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


def collect(event, run, jobs, pin, workspace, expected_artifact):
    event_dir = workspace / event
    event_dir.mkdir()
    write_json(event_dir / "status.json", {"run": run, "jobs": jobs})
    logs = read_job_logs(event_dir, jobs)
    browser = jobs.get("browser-e2e")
    ok = (run["conclusion"] == "success" and set(jobs) == set(JOBS)
          and all(job["status"] == "completed" and job["conclusion"] == "success" for job in jobs.values()))
    archive = event_dir / "surface-e2e-artifacts.zip"
    artifact = (get_archive(run, browser, archive, required=ok, expected=expected_artifact)
                if browser and browser["status"] == "completed" else None)
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
    for component in ("radio", "checkbox", "switch"):
        assert measured["navigation"]["case_results"] == measured[component]["runner"]["cases"] == counts
        assert measured["navigation"]["total_checks"] == measured[component]["runner"]["total_checks"] == total
    rejected = {"common_buttons": reject_buttons(raw, stock, measured), "slider_refinements": reject_refinements(raw, stock, measured), "range_slider": reject_range(raw, stock, measured), "slider_forms": reject_forms(raw, stock, measured), "sliders": reject_sliders(raw, stock, measured), "icon_buttons": reject_icons(raw, stock, measured), "chips": reject_chips(raw, stock, measured), "tabs": reject_tabs(raw, stock, measured),
                "navigation": reject_controls(raw, stock, platform="linux", stock_last_material=LAST_MATERIAL)}
    write_json(portable / "rejected-controls.json", rejected)
    status = {"event": event, "run": run["id"], "run_url": run["html_url"], "attempt": run["run_attempt"],
              "head": pin["head"], "checkout": checkout_commit, "checkout_tree": pin["tree"],
              "checkout_parents": parents, "checkouts_agree": True, "jobs": job_details(jobs),
              "original_log_sha256": {name: digest(event_dir / f"{name}.log") for name in JOBS},
              "original_zip": artifact, "artifact_bytes_verified": True, **evidence,
              "cases": counts, "total_checks": total, "rejection_controls_sha256": digest(portable / "rejected-controls.json"), **measured}
    write_json(portable / "verified.json", status)
    print(f"{event}: {total} checks, five Segmented PNGs, plus seven Common Buttons, Slider Refinements, Range Slider, Slider Forms, Standard Slider, Icon Buttons, Chips, Tabs, Radio, Checkbox, Switch, Navigation, Menu, Divider and I11 regressions.", flush=True)
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
    requests = {"pull_request": (args.pull_run, args.pull_attempt, args.pull_artifact),
                "push": (args.push_run, args.push_attempt, args.push_artifact)}
    receipts = {}
    deadline = monotonic() + args.timeout
    pending = None
    while set(receipts) != set(EVENTS):
        waiting = []
        for event, (run_id, attempt, artifact) in requests.items():
            if event in receipts:
                continue
            run, jobs = run_snapshot(event, run_id, attempt, args)
            if run["status"] != "completed":
                waiting.append((event, run_id, run["status"]))
                continue
            try:
                receipts[event] = collect(event, run, jobs, pin, workspace, artifact)
            except (AssertionError, KeyError, ValueError, RuntimeError, OSError, subprocess.SubprocessError) as error:
                receipts[event] = None
                evidence = workspace / event / "verification-error.txt"
                evidence.write_text(traceback.format_exc())
                print(f"{event}: original evidence verification failed ({type(error).__name__}). See {evidence}.", flush=True)
        if not waiting:
            break
        if waiting != pending:
            print(f"Exact Baseline Segmented Button workflows pending: {waiting}", flush=True)
            pending = waiting
        remaining = deadline - monotonic()
        if not args.watch or remaining <= 0:
            raise SystemExit("Exact workflows are still pending. Use a fresh scratch directory for a later observation.")
        sleep(min(15, remaining))
    if not all(receipts.values()):
        raise SystemExit("At least one exact workflow failed or lacks valid receipts. See the original local evidence.")
    distinct_workflows(receipts)
    return workspace, receipts


def package(workspace, pin, receipts):
    portable = workspace / "portable"
    for name in ("candidate-pin.json", "candidate-source.sha256", "buttons-base.sha256", "segmented-v0159.patch", "ci.yml",
                 "verify_range_pixels.py", "range_rejection_controls.py",
                 "verify_refinement_pixels.py", "refinement_rejection_controls.py", "verify_button_pixels.py", "button_rejection_controls.py",
                 "verify_segmented_pixels.py",
                 "verify_forms_pixels.py", "verify_slider_forms_report.py", "slider_forms_rejection_controls.py",
                 "verify_sliders_report.py", "slider_rejection_controls.py",
                 "verify_icon_buttons_report.py", "icon_rejection_controls.py", "verify_chips_report.py", "chips_rejection_controls.py",
                 "verify-hosted-segmented.py", "verify_tabs_report.py", "tabs_rejection_controls.py",
                 "verify_radio_report.py", "radio-validator-original.py", "radio-regression.patch",
                 "verify_checkbox_report.py", "checkbox-validator-original.py", "checkbox-regression.patch",
                 "verify_switch_report.py",
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
    readme = ["# Surface v0.1.59 Baseline Segmented Button hosted receipts", "",
              f"Candidate head `{pin['head']}`, tree `{pin['tree']}`, base `{pin['base']}`.",
              f"The exact {SOURCE_COUNT}-file source seal is `{SOURCE_SHA}`.", "",
              "Both stock workflows ran all eight cases on GitHub Ubuntu. Chrome used the observed",
              "SwiftShader adapter. Each workflow ran 322 ordered Material steps through M323.",
              "The verifier independently checked five Baseline Segmented Button, seven Common Buttons, seven Slider Refinements, four Range Slider, four Slider Forms, four Standard Slider, four Icon Buttons, four Chips, four Tabs, four Radio, four Checkbox, four Switch, four Navigation, three Menu and four",
              "Divider PNGs, both I11 cancellation paths and strict Divider stripes.",
              "The Radio, Checkbox, Switch, Navigation and Menu validators accept the enclosing stock step count. Their",
              "image and behavior checks match the included original validators.", ""]
    for event in EVENTS:
        value = receipts[event]
        readme.append(f"- [{event}]({value['run_url']}) passed {value['total_checks']:,} checks.")
    readme += ["", "The original GitHub ZIP identities and member SHA-256 values are in each",
               "`verified.json`. The complete original ZIPs and logs remain in the local",
               "capture directory. This package contains selected unchanged artifact members.", "",
               "Run `shasum -a 256 -c SHA256SUMS` and",
               "`python3 -B verify-hosted-segmented.py --verify-package . --originals ..` to check the package and original receipts.", ""]
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
    assert digest(portable / "buttons-base.sha256") == BASE_SHA
    assert digest(portable / "segmented-v0159.patch") == PATCH_SHA
    aggregate = read_json(portable / "hosted-verified.json")
    assert (aggregate["repo"], aggregate["release"], aggregate["branch"]) == (REPO, "v0.1.59", BRANCH)
    assert aggregate["tree"] == CANDIDATE_PIN["candidate_tree"]
    assert all(aggregate[key] == CANDIDATE_PIN[key] for key in ("pr", "head", "base"))
    assert aggregate["source_manifest_sha256"] == SOURCE_SHA and aggregate["source_paths"] == SOURCE_COUNT
    released = aggregate["released_base"]
    assert (released["commit"], released["release"], released["source_manifest_sha256"], released["paths"]) == (
        aggregate["base"], "v0.1.58", BASE_SHA, BASE_COUNT)
    assert released["tree"] == CANDIDATE_PIN["base_tree"]
    assert all(re.fullmatch(r"[0-9a-f]{40}", oid) for oid in (
        aggregate["head"], aggregate["base"], aggregate["tree"], released["tree"]))
    assert set(aggregate["workflows"]) == set(EVENTS)
    receipts = {event: read_json(portable / event / "verified.json") for event in EVENTS}
    distinct_workflows(receipts)
    for event in EVENTS:
        target = portable / event
        row = receipts[event]
        assert row["event"] == event and row["head"] == aggregate["head"]
        pinned = CANDIDATE_PIN["runs"][event]
        assert (row["run"], row["attempt"]) == (pinned["id"], pinned["attempt"])
        assert {name: job["id"] for name, job in row["jobs"].items()} == pinned["jobs"]
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
        assert run["repository"]["full_name"] == run["head_repository"]["full_name"] == aggregate["repo"]
        assert run["path"] == ".github/workflows/ci.yml" and run["head_branch"] == aggregate["branch"]
        if event == "pull_request":
            pr, = [item for item in run["pull_requests"] if item["number"] == aggregate["pr"]]
            assert (pr["head"]["sha"], pr["head"]["ref"], pr["base"]["sha"], pr["base"]["ref"]) == (
                aggregate["head"], aggregate["branch"], aggregate["base"], "main")
        assert job_details(status["jobs"]) == row["jobs"]
        assert all((job["run_id"], job["head_sha"]) == (row["run"], row["head"])
                   for job in status["jobs"].values())
        assert read_json(original / "original-zip.json") == row["original_zip"]
        check_artifact_identity(row["original_zip"], run, status["jobs"]["browser-e2e"])
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
        assert read_json(controls) == {"common_buttons": reject_buttons(raw, log, measured), "slider_refinements": reject_refinements(raw, log, measured), "range_slider": reject_range(raw, log, measured), "slider_forms": reject_forms(raw, log, measured), "sliders": reject_sliders(raw, log, measured), "icon_buttons": reject_icons(raw, log, measured), "chips": reject_chips(raw, log, measured), "tabs": reject_tabs(raw, log, measured),
                                       "navigation": reject_controls(raw, log, platform="linux", stock_last_material=LAST_MATERIAL)}
    return aggregate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, help="Exact sealed Baseline Segmented Button source directory")
    parser.add_argument("--buttons-source", type=Path, help="Released Common Buttons source for the named base")
    parser.add_argument("--branch", default=BRANCH, help="Exact Baseline Segmented Button branch")
    parser.add_argument("--out", type=Path, help="Fresh directory under /tmp")
    parser.add_argument("--pr", type=int)
    parser.add_argument("--head")
    parser.add_argument("--tree")
    parser.add_argument("--base")
    parser.add_argument("--pull-run", type=int)
    parser.add_argument("--push-run", type=int)
    parser.add_argument("--pull-attempt", type=int, default=1)
    parser.add_argument("--push-attempt", type=int, default=1)
    parser.add_argument("--pull-artifact", type=int, help="Pin the exact original PR ZIP artifact ID if known")
    parser.add_argument("--push-artifact", type=int, help="Pin the exact original push ZIP artifact ID if known")
    parser.add_argument("--watch", action="store_true", help="Poll only the two pinned attempts")
    parser.add_argument("--timeout", type=int, default=3600, help="Maximum wait in seconds with --watch")
    parser.add_argument("--check-sources", action="store_true", help="verify both local source seals without GitHub")
    parser.add_argument("--verify-package", type=Path,
                        help="Verify a sealed local proof instead of calling GitHub")
    parser.add_argument("--originals", type=Path, help="Required original ZIPs and job logs for --verify-package")
    args = parser.parse_args()
    if not __debug__:
        parser.error("Run without -O so verification assertions stay enabled.")
    if args.check_sources:
        if args.source is None or args.buttons_source is None:
            parser.error("--check-sources needs --source and --buttons-source")
        bound_tools()
        candidate = read_source(args.source)
        base = read_source(args.buttons_source, base=True)
        print(json.dumps({"candidate": {"sha256": SOURCE_SHA, "files": len(candidate)},
                          "common_buttons_base": {"sha256": BASE_SHA, "files": len(base)}}, indent=2))
        return
    if args.verify_package:
        if args.originals is None:
            parser.error("--verify-package requires --originals.")
        print(json.dumps(verify_package(args.verify_package, args.originals), indent=2))
        return
    required = ("source", "buttons_source", "out", "pr", "head", "tree", "base", "pull_run", "push_run")
    if missing := [flag.replace("_", "-") for flag in required if getattr(args, flag) is None]:
        parser.error("Required to verify GitHub: " + ", ".join(f"--{flag}" for flag in missing))
    for oid in (args.head, args.tree, args.base):
        if not re.fullmatch(r"[0-9a-f]{40}", oid):
            parser.error("Head, base and tree must be full lowercase Git object IDs.")
    numerics = (args.pr, args.pull_run, args.push_run, args.pull_attempt, args.push_attempt,
                args.pull_artifact, args.push_artifact, args.timeout)
    if any(number is not None and number < 1 for number in numerics):
        parser.error("PR number, run IDs, and attempts must be positive.")
    if args.pull_run == args.push_run:
        parser.error("Supply separate pull-request and push runs.")
    if args.pull_artifact is not None and args.pull_artifact == args.push_artifact:
        parser.error("Supply separate original artifact IDs.")
    if args.head == args.base:
        parser.error("Supply distinct candidate and released-base commits.")
    if args.branch != BRANCH:
        parser.error("The branch must match this sealed Baseline Segmented Button candidate.")
    pin = bind_candidate(args)
    workspace, receipts = observe(pin, args)
    package(workspace, pin, receipts)


if __name__ == "__main__":
    main()
