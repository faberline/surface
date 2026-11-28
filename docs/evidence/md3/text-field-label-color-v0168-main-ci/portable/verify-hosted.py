"""Collect or replay the original Surface v0.1.67 Linux CI evidence."""
import argparse
from datetime import datetime
from hashlib import file_digest, sha256
import gzip
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
from tempfile import TemporaryDirectory
from zipfile import ZipFile

REPO = "faberline/surface"
PR = 244
BRANCH = "main"
HEAD = "987b5ce8bc4aa260e0cda30c2278ebb3317f879f"
BASE = "6fb40370ca824b8ed44e6e17099e6156f8d15a9b"
MERGE = "0e77157bb75a8547c68ccee11633db9eee145291"
TREE = "9d3ce235f8efde1459fd41862cd918367fc85207"
BASE_TREE = "c218faaf7527ec2872fb1ec1097c87667f3bf1f2"
RUNS = {"push": (46122360299, 140909422239, 140909422067)}
SUITES = ("smoke", "ime", "material", "bridge-native", "bridge-native-compact", "bridge-web",
          "devtools-native", "devtools-headless")
COUNTS = {"smoke": (19, 34), "ime": (20, 77), "bridge-native": (23, 92), "bridge-native-compact": (24, 93),
          "bridge-web": (24, 94), "devtools-native": (17, 192), "devtools-headless": (3, 76)}
LOCAL_EVIDENCE = "d74ad36e0c588a4a43fb3a43f11d1ce5fe7918f4"
LOCAL_FOLDER = "docs/evidence/md3/text-field-label-motion-v0167-candidate"
LOCAL_SEAL = "184b9d871ee954547cbcefef551585c604e0e80b0d4262c305ba3cb46bdfd70b"
LOCAL_SOURCE = "4aa53964b34f033b59b5ab723e58deb612d5db8ce1975bd3cf2115e687479271"
PREVIOUS = "9a635277e383224a5342ccb2ee7d49aad61e1d7d"
PREVIOUS_VERIFIER = "274104a5842e441ec9826fdd082227f1b76005cb676825f03f26aeaac89bbe44"
RESULT = "verified.json"
PREFIX = "material-text-field-label-motion"
MOTION = PREFIX + ".json"
TEXTBOX = "browser-material-multiline-text-field.json"
IME = ("browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json", "browser-ime-line-endings.json")
PNG = ('material-text-field-label-motion-filled-color-clearing.png', 'material-text-field-label-motion-filled-color-error-first.png', 'material-text-field-label-motion-filled-color-error-mid.png', 'material-text-field-label-motion-filled-color-error-settled.png', 'material-text-field-label-motion-filled-color-focus-mid.png', 'material-text-field-label-motion-filled-color-reduced.png', 'material-text-field-label-motion-filled-color-rest.png', 'material-text-field-label-motion-filled-color-reversed.png', 'material-text-field-label-motion-filled-first.png', 'material-text-field-label-motion-filled-middle.png', 'material-text-field-label-motion-filled-preedit.png', 'material-text-field-label-motion-filled-rest.png', 'material-text-field-label-motion-filled-settled.png', 'material-text-field-label-motion-outlined-color-clearing.png', 'material-text-field-label-motion-outlined-color-error-first.png', 'material-text-field-label-motion-outlined-color-error-mid.png', 'material-text-field-label-motion-outlined-color-error-settled.png', 'material-text-field-label-motion-outlined-color-focus-mid.png', 'material-text-field-label-motion-outlined-color-reduced.png', 'material-text-field-label-motion-outlined-color-rest.png', 'material-text-field-label-motion-outlined-color-reversed.png', 'material-text-field-label-motion-outlined-early.png', 'material-text-field-label-motion-outlined-falling.png', 'material-text-field-label-motion-outlined-first.png', 'material-text-field-label-motion-outlined-middle.png', 'material-text-field-label-motion-outlined-reduced.png', 'material-text-field-label-motion-outlined-rest.png', 'material-text-field-label-motion-outlined-retarget.png', 'material-text-field-label-motion-outlined-settled.png', 'material-text-field-label-motion-rtl-above-label-middle.png', 'material-text-field-label-motion-rtl-above-label-rest.png', 'material-text-field-label-motion-rtl-above-label.png', 'material-text-field-label-motion-rtl-middle.png', 'material-text-field-label-motion-rtl-rest.png')
SELECTED = {MOTION, *PNG}
META = ("run.json", "jobs.json", "checkout.json", "artifact.json")
LOCAL_FILES = {"stock-tools/verify_label_color.py": "verify_label_color.py",
               "stock-tools/label-color-oracle.py": "label-color-oracle.py",
               "stock/run/artifacts/material-text-field-label-motion.json": "mac-color.json"}
LOCAL_SEAL = "2718014e69e00127870147b9f5bea66c8b3fef121a88d7681b7c8749ad72a85f"
GH = "/opt/homebrew/bin/gh"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def write(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, sort_keys=True, indent=2) + "\n")


def read(path):
    return json.loads(path.read_bytes())


def manifest(path):
    result = {}
    for line in path.read_text().splitlines():
        want, name = line.split("  ", 1)
        posix = PurePosixPath(name)
        require(re.fullmatch(r"[a-f0-9]{64}", want) and name not in result and not posix.is_absolute()
                and ".." not in posix.parts and str(posix) == name and name != ".", "Invalid manifest member")
        result[name] = want
    return result


def gh(endpoint, out=None):
    command = [GH, "api", "--method", "GET", f"repos/{REPO}/{endpoint}"]
    if out is None:
        result = subprocess.check_output(command)
        return json.loads(result)
    with out.open("xb") as stream:
        subprocess.run(command, stdout=stream, check=True)


def pin_commit(payload, sha, tree, parents=None):
    require(payload["sha"] == sha and payload["tree"]["sha"] == tree, f"GitHub tree differs for {sha}")
    if parents is not None:
        require([parent["sha"] for parent in payload["parents"]] == parents, f" GitHub parents differ for {sha}")


def pin_pr(pr):
    require(pr["number"] == PR and pr["head"]["sha"] == HEAD and pr["head"]["ref"] == BRANCH
            and pr["head"]["repo"]["full_name"] == REPO and pr["base"]["sha"] == BASE
            and pr["base"]["ref"] == "main" and pr["base"]["repo"]["full_name"] == REPO
            and pr["merge_commit_sha"] == MERGE and not pr["draft"] and pr["state"] == "open",
            "Draft source, base or GitHub merge checkout changed")


def pin_jobs(event, run, job_list, success=False):
    run_id, build_id, browser_id = RUNS[event]
    require((run["id"], run["run_attempt"], run["head_sha"], run["event"], run["head_branch"], run["path"])
            == (run_id, 1, HEAD, event, BRANCH, ".github/workflows/ci.yml"), f"Wrong {event} original run")
    if event == "pull_request":
        linked = [(item["number"], item["head"]["sha"], item["base"]["sha"]) for item in run["pull_requests"]]
        require(linked == [(PR, HEAD, BASE)], "Wrong original PR run linkage")
    jobs = {job["name"]: job for job in job_list["jobs"]}
    require(len(jobs) == len(job_list["jobs"]) == 2 and set(jobs) == {"build-and-behavior", "browser-e2e"}, "Wrong jobs")
    required = {
        "build-and-behavior": ("Run actions/checkout@v4", "Run cargo xtask provenance", "Run cargo fmt --all --check",
                               "Run cargo clippy --locked --workspace --all-targets --all-features -- -D warnings",
                               "Run cargo test --locked --workspace --all-features",
                               "Run cargo check --locked --target wasm32-unknown-unknown -p surface-showcase -p surface-material-gallery --all-features --lib",
                               "Run cargo check --locked --target wasm32-unknown-unknown -p ui-runtime --all-features --lib --test consumer_contract",
                               "Run cargo check --locked --target wasm32-unknown-unknown -p ui-runtime --no-default-features --lib --test consumer_contract"),
        "browser-e2e": ("Run actions/checkout@v4", "Run surface-e2e", "Run actions/upload-artifact@v4"),
    }
    for name, job_id in (("build-and-behavior", build_id), ("browser-e2e", browser_id)):
        job = jobs[name]
        require((job["id"], job["run_id"], job["run_attempt"], job["head_sha"]) == (job_id, run_id, 1, HEAD),
                f"Wrong original {event} job {name}")
        if success:
            steps = {step["name"]: step for step in job["steps"]}
            require(job["status"] == "completed" and job["conclusion"] == "success"
                    and all(steps[name]["status"] == "completed" and steps[name]["conclusion"] == "success"
                            for name in required[name]), f"Original {event} job {name} did not pass")
    if success:
        require(run["status"] == "completed" and run["conclusion"] == "success", f"Original {event} did not pass")
    return jobs


def pin_main(pr, head, listing):
    require(pr["merged"] and pr["merge_commit_sha"] == HEAD
            and pr["head"]["sha"] == "26758a7313527c9aab7c2d5e677ec880866638da"
            and pr["base"]["ref"] == "main", "Wrong merged PR")
    pin_commit(head, HEAD, TREE, [BASE])
    runs = listing["workflow_runs"]
    require(listing["total_count"] == len(runs) == 1 and runs[0]["id"] == RUNS["push"][0]
            and runs[0]["head_sha"] == HEAD and runs[0]["event"] == "push"
            and runs[0]["head_branch"] == "main" and runs[0]["run_attempt"] == 1,
            "Wrong original main run discovery")

def collect_metadata(out=None):
    pr = gh("pulls/244")
    head = gh(f"git/commits/{HEAD}")
    listing = gh(f"actions/workflows/ci.yml/runs?head_sha={HEAD}&event=push&branch=main&per_page=100")
    pin_main(pr, head, listing)
    attempt = f"actions/runs/{RUNS['push'][0]}/attempts/1"
    run, jobs = gh(attempt), gh(f"{attempt}/jobs?per_page=100")
    pin_jobs("push", run, jobs)
    if out:
        for name, item in (("pr", pr), ("head", head), ("listing", listing)):
            write(out / f"{name}.json", item)
        target = out / "push"
        target.mkdir()
        for name, item in (("run", run), ("jobs", jobs), ("checkout", head)):
            write(target / f"{name}.json", item)
    return {"push": (run, jobs)}


def collect(out):
    out.mkdir()
    runs = collect_metadata(out)
    require(all(run["status"] == "completed" for run, _ in runs.values()), "Original workflows still running")
    failures = []
    for event, (run_id, build, browser) in RUNS.items():
        run, jobs = runs[event]
        by_name = pin_jobs(event, run, jobs)
        folder = out / event
        gh(f"actions/jobs/{build}/logs", folder / "build.log")
        gh(f"actions/jobs/{browser}/logs", folder / "browser.log")
        items = gh(f"actions/runs/{run_id}/artifacts?per_page=100")
        selected = [item for item in items["artifacts"] if item["name"] == "surface-e2e-artifacts"]
        if run["conclusion"] != "success":
            failures.append(event)
            if not selected:
                continue
        require(len(selected) == 1, f"Wrong artifact count in {event}")
        artifact = selected[0]
        pin_artifact(artifact, run_id, by_name["browser-e2e"])
        write(folder / "artifact.json", artifact)
        gh(f"actions/artifacts/{artifact['id']}/zip", folder / "original.zip")
        require(digest(folder / "original.zip") == artifact["digest"][7:]
                and (folder / "original.zip").stat().st_size == artifact["size_in_bytes"], "Downloaded ZIP differs")
    require(not failures, "Preserved original failure evidence: " + ", ".join(failures))
    return {"status": "original inputs collected", "originals": str(out)}


def pin_artifact(artifact, run, browser_job):
    require(artifact["name"] == "surface-e2e-artifacts" and artifact["expired"] is False
            and artifact["workflow_run"]["id"] == run and artifact["workflow_run"]["head_sha"] == HEAD
            and re.fullmatch(r"sha256:[a-f0-9]{64}", artifact["digest"]) and artifact["size_in_bytes"] > 0,
            "Wrong original artifact identity")
    generated = datetime.fromisoformat(artifact["created_at"])
    require(datetime.fromisoformat(browser_job["started_at"]) <= generated
            <= datetime.fromisoformat(browser_job["completed_at"]), "Artifact was created outside the original browser job")


def checkout_in_log(log):
    return re.findall(r"git log -1 --format=%H\r?\n\S+ ([0-9a-f]{40})", log)


def browser_log(log, last=350):
    starts = re.findall(r"^.*?Z case ([\w-]+) \.\.\.\s*$", log, re.M)
    passed = re.findall(r"^.*?Z case ([\w-]+) \.\.\. ok \((\d+) steps, (\d+) checks,", log, re.M)
    require(starts == [row[0] for row in passed] == list(SUITES), "Wrong eight stock cases")
    cases = {name: (int(steps), int(count)) for name, steps, count in passed}
    require(all(steps > 0 and checks > 0 for steps, checks in cases.values()) and cases["material"][0] == last - 1,
            "Wrong stock case counts")
    expected_steps = [idx for idx in range(1, last + 1) if idx != 23]
    require([int(step) for step in re.findall(r"\[M(\d+)\]", log)] == expected_steps, "Wrong Material sequence")
    require([int(step) for step in re.findall(r"\[I(\d+)\]", log)] == list(range(1, 21)), "Wrong IME sequence")
    total = sum(count for _, count in cases.values())
    require(re.findall(r"e2e result: (\d+) passed; (\d+) failed; (\d+) checks", log) == [("8", "0", str(total))]
            and "xvfb-run -a" in log and "--use-webgpu-adapter=swiftshader" in log, "Wrong actual Linux runner")
    matches = re.findall(r"\bbrowser: (Chrome/[0-9]+(?:\.[0-9]+){3})\b", log)
    require(len(set(matches)) == 1, "Different or absent Chrome version")
    return {"cases": {name: {"steps": n, "checks": m} for name, (n, m) in cases.items()},
            "checks": total, "material_steps": last - 1, "browser": matches[0], "passed": 8}


def build_log(log):
    tests = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", log)
    require("provenance:" in log and tests and all(state == "ok" and failed == "0" for state, _, failed, _ in tests)
            and sum(int(n) for _, n, _, _ in tests) > 1500, "Wrong Rust run")
    return {"passed": sum(int(n) for _, n, _, _ in tests), "ignored": sum(int(n) for _, _, _, n in tests), "wasm_checks": 3}


def swiftshader(adapter):
    return (set(adapter) == {"vendor", "architecture", "isFallbackAdapter"} and adapter["vendor"].lower() == "google"
            and adapter["architecture"].lower() == "swiftshader" and adapter["isFallbackAdapter"] is True)


def references(folder, origin=None):
    if origin:
        folder.mkdir()
        require(digest(origin / "SHA256SUMS") == LOCAL_SEAL, "Wrong macOS calibration receipt")
        shutil.copyfile(origin / "SHA256SUMS", folder / "candidate-SHA256SUMS")
        entries = manifest(origin / "SHA256SUMS")
        for src, name in LOCAL_FILES.items():
            require(digest(origin / src) == entries[src], "Changed calibration input: " + name)
            shutil.copyfile(origin / src, folder / name)
    require(digest(folder / "candidate-SHA256SUMS") == LOCAL_SEAL, "Wrong calibration seal")
    entries = manifest(folder / "candidate-SHA256SUMS")
    for src, name in LOCAL_FILES.items():
        require(digest(folder / name) == entries[src], "Changed archived reference: " + name)
    return folder


def color_decoder(ref):
    path = ref / "verify_label_color.py"
    text = path.read_text()
    old = 'OBS = run_path(str(ROOT / "observe-focused-original.py"))'
    require(text.count(old) == 1, "Unexpected reference decoder imports")
    text = text.replace(old, 'OBS = {}')
    old = '''adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal-")
            and adapter["isFallbackAdapter"] is False, "Apple Metal Chrome adapter missing"'''
    require(text.count(old) == 1, "Unexpected reference adapter guard")
    text = text.replace(old, 'swiftshader(adapter), "Expected Linux SwiftShader"')
    env = {"__file__": str(path), "__name__": "portable_label_color", "swiftshader": swiftshader}
    exec(compile(text, str(path), "exec"), env)
    return env["verify_artifacts"]


def image_receipt(artifacts, ref, chrome):
    report = read(artifacts / MOTION)
    mac = read(ref / "mac-color.json")
    require(report["browser"] == chrome and swiftshader(report["adapter"]), "Wrong Linux browser or adapter")
    for key in ("outlined", "filled", "compact"):
        require(report[key]["palette"] == mac[key]["palette"], "Wrong palette: " + key)
    result = color_decoder(ref)(artifacts)
    require(set(result["captures"]) == set(PNG), "Wrong PNG inventory")
    sample = report["compact"]["fallback"]["color_frames"][1]
    require(sample["ms"] == 44 and sample["label_rgb"] == [206, 191, 243], "M350 actual 44ms color differs")
    return {"adapter": report["adapter"], "label_color": result}


def logs(folder, compressed, event):
    checkout = HEAD if event == "push" else MERGE
    result, hashes = {}, {}
    for name in ("build", "browser"):
        raw = (folder / f"{name}.log{'.gz' if compressed else ''}").read_bytes()
        raw = gzip.decompress(raw) if compressed else raw
        text = raw.decode().replace("\x1b[0m", "")
        require(checkout_in_log(text) == [checkout], f"Wrong original checkout in {name} log")
        result[name] = (build_log if name == "build" else browser_log)(text)
        hashes[name] = sha256(raw).hexdigest()
    return result, hashes


def event_proof(folder, event, ref, compressed=False):
    run_id, build_id, browser_id = RUNS[event]
    jobs = pin_jobs(event, read(folder / "run.json"), read(folder / "jobs.json"), success=True)
    checkout = HEAD if event == "push" else MERGE
    pin_commit(read(folder / "checkout.json"), checkout, TREE, [BASE])
    artifact = read(folder / "artifact.json")
    pin_artifact(artifact, run_id, jobs["browser-e2e"])
    with TemporaryDirectory(prefix="surface-q68-zip-readback-", dir="/tmp") as temp:
        extracted = Path(temp)
        unpack_original(folder, extracted)
        require(all(digest(extracted / name) == digest(folder / name) for name in SELECTED),
                "Portable selected files do not match the original artifact ZIP")
    summaries, hashes = logs(folder, compressed, event)
    graphics = image_receipt(folder, ref, summaries["browser"]["browser"])
    return {"event": event, "attempt": 1, "run": run_id, "checkout": checkout, "tree": TREE,
            "jobs": {"build-and-behavior": build_id, "browser-e2e": browser_id},
            "artifact": {"id": artifact["id"], "sha256": artifact["digest"][7:], "bytes": artifact["size_in_bytes"]},
            "logs": hashes, "linux": summaries, **graphics,
            "files": {name: digest(folder / name) for name in sorted(SELECTED)}}


def unpack_original(originals, folder):
    zipper = originals / "original.zip"
    artifact = read(originals / "artifact.json")
    require(digest(zipper) == artifact["digest"][7:] and zipper.stat().st_size == artifact["size_in_bytes"],
            "Original Linux ZIP digest differs")
    with ZipFile(zipper) as archive:
        selected = {}
        for item in archive.infolist():
            path = PurePosixPath(item.filename)
            if path.name in SELECTED and not item.is_dir():
                require(not path.is_absolute() and ".." not in path.parts and path.name not in selected
                        and 0 < item.file_size < 20_000_000, "Ambiguous or unsafe selected ZIP entry")
                selected[path.name] = item
        require(set(selected) == SELECTED, "Missing original ZIP evidence: " + str(sorted(SELECTED - set(selected))))
        for name, item in selected.items():
            (folder / name).write_bytes(archive.read(item))


def archived(root):
    pin_main(read(root / "pr.json"), read(root / "head.json"), read(root / "listing.json"))
    ref = references(root / "reference")
    proof = event_proof(root / "push", "push", ref, True)
    return {"provisional": True, "head": HEAD, "tree": TREE,
            "source_url": f"https://github.com/{REPO}/tree/{HEAD}",
            "run_url": f"https://github.com/{REPO}/actions/runs/{RUNS['push'][0]}/attempts/1",
            "artifact_url": f"https://github.com/{REPO}/actions/runs/{RUNS['push'][0]}/artifacts/" + str(proof["artifact"]["id"]),
            "candidate_manifest_sha256": LOCAL_SEAL, "workflow": proof}


def export(originals, candidate, out):
    out.mkdir()
    references(out / "reference", candidate)
    for name in ("pr", "head", "listing"):
        shutil.copyfile(originals / f"{name}.json", out / f"{name}.json")
    for event in RUNS:
        source, folder = originals / event, out / event
        folder.mkdir()
        for name in META:
            shutil.copyfile(source / name, folder / name)
        shutil.copyfile(source / "original.zip", folder / "original.zip")
        for kind in ("build", "browser"):
            raw = (source / f"{kind}.log").read_bytes()
            (folder / f"{kind}.log.gz").write_bytes(gzip.compress(raw, mtime=0))
        unpack_original(source, folder)
    for name in ("verify-hosted.py", "README.md"):
        shutil.copyfile(Path(__file__).with_name(name), out / name)
    result = archived(out)
    write(out / RESULT, result)
    sums = out / "SHA256SUMS"
    sums.write_text("".join(f"{digest(path)}  {path.relative_to(out).as_posix()}\n"
                            for path in sorted(out.rglob("*")) if path.is_file()))
    return {"path": str(out), "manifest_sha256": digest(sums), **result}


def verify(root, pin):
    require(digest(root / "SHA256SUMS") == pin, "Wrong portable manifest digest")
    entries = manifest(root / "SHA256SUMS")
    actual = {path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file()} - {"SHA256SUMS"}
    require(actual == set(entries) and all(not (root / path).is_symlink() and digest(root / path) == sha
                                           for path, sha in entries.items()), "Portable archive differs")
    verdict = archived(root)
    require(verdict == read(root / RESULT), "Portable receipt differs")
    return verdict


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--preflight", action="store_true")
    mode.add_argument("--collect", type=Path)
    mode.add_argument("--export", type=Path)
    mode.add_argument("--verify-package", type=Path)
    parser.add_argument("--originals", type=Path)
    parser.add_argument("--candidate", type=Path)
    parser.add_argument("--package-sha")
    args = parser.parse_args()
    require(__debug__, "Predecessor checks require Python assertions")
    if args.preflight:
        with TemporaryDirectory(prefix="surface-q68-reference-", dir="/tmp") as tmp:
            references(Path(tmp) / "reference", args.candidate)
        runs = collect_metadata()
        result = {"status": "metadata and reference preflight only; no acceptance",
                  "runs": {event: {"run": value[0]["id"], "status": value[0]["status"],
                                   "conclusion": value[0]["conclusion"]} for event, value in runs.items()}}
    elif args.collect:
        require(args.collect.resolve().is_relative_to(Path("/tmp").resolve()), "Collect into fresh /tmp")
        result = collect(args.collect)
    elif args.export:
        require(args.export.resolve().is_relative_to(Path("/tmp").resolve()), "Export into fresh /tmp")
        result = export(args.originals, args.candidate, args.export)
    else:
        require(args.package_sha is not None, "Expected reviewed manifest SHA")
        result = verify(args.verify_package, args.package_sha)
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
