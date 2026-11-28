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
PR = 243
BRANCH = "codex/md3-text-field-label-motion"
HEAD = "7da557d095d16eb43a5725c9e5e580ce198cd8b2"
BASE = "a751900b2f2c4c5f204afc8c4b9a0e49d1f477cf"
MERGE = "ee1adc586eca7635dfac38874bf1ef543a3fde6a"
TREE = "c218faaf7527ec2872fb1ec1097c87667f3bf1f2"
BASE_TREE = "3de0c143c43d35911593cfc95d3ea5931016c308"
RUNS = {"pull_request": (46109248208, 140868862770, 140868862626),
        "push": (46109213750, 140868755370, 140868755571)}
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
LABEL_NAMES = ("outlined-rest", "outlined-first", "outlined-early", "outlined-middle", "outlined-settled",
               "outlined-falling", "outlined-retarget", "outlined-reduced", "filled-first", "filled-middle",
               "filled-settled", "filled-preedit", "rtl-rest", "rtl-middle", "rtl-above-label")
PNG = tuple(f"{PREFIX}-{name}.png" for name in LABEL_NAMES)
SELECTED = {MOTION, TEXTBOX, *IME, *PNG}
META = ("run.json", "jobs.json", "checkout.json", "artifact.json")
LOCAL_FILES = {"source.sha256": "source.sha256",
               "stock-macos/verify_label_motion.py": "verify_label_motion.py",
               f"stock-macos/run/artifacts/{MOTION}": "mac-motion.json",
               "stock-macos/stock-mac-exact-offline.json": "mac-stock.json"}
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
            and pr["merge_commit_sha"] == MERGE and pr["draft"] and pr["state"] == "open",
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


def collect_metadata(out=None):
    pr = gh(f"pulls/{PR}")
    pin_pr(pr)
    head = gh(f"git/commits/{HEAD}")
    base = gh(f"git/commits/{BASE}")
    merge = gh(f"git/commits/{MERGE}")
    pin_commit(head, HEAD, TREE, [BASE])
    pin_commit(base, BASE, BASE_TREE)
    pin_commit(merge, MERGE, TREE, [BASE, HEAD])
    if out:
        for name, item in (("pr", pr), ("head", head), ("base", base), ("merge", merge)):
            write(out / f"{name}.json", item)
    runs = {}
    for event, (run_id, _, _) in RUNS.items():
        attempt = f"actions/runs/{run_id}/attempts/1"
        run, jobs = gh(attempt), gh(f"{attempt}/jobs?per_page=100")
        pin_jobs(event, run, jobs)
        if out:
            target = out / event
            target.mkdir()
            write(target / "run.json", run)
            write(target / "jobs.json", jobs)
            write(target / "checkout.json", merge if event == "pull_request" else head)
        runs[event] = (run, jobs)
    return runs


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
    require(all(cases[name] == values for name, values in COUNTS.items()) and cases["material"][0] == last - 1,
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
        require(digest(origin / "SHA256SUMS") == LOCAL_SEAL, "Wrong archived Label Motion inputs")
        shutil.copyfile(origin / "SHA256SUMS", folder / "candidate-SHA256SUMS")
        entries = manifest(origin / "SHA256SUMS")
        for src, name in LOCAL_FILES.items():
            require(digest(origin / src) == entries[src], f"Local expected input changed: {name}")
            shutil.copyfile(origin / src, folder / name)
        shutil.copyfile(Path(__file__).with_name("prior-verify-hosted.py"), folder / "prior-verify-hosted.py")
    require(digest(folder / "candidate-SHA256SUMS") == LOCAL_SEAL
            and digest(folder / "source.sha256") == LOCAL_SOURCE
            and digest(folder / "prior-verify-hosted.py") == PREVIOUS_VERIFIER, "Wrong archived reference pins")
    entries = manifest(folder / "candidate-SHA256SUMS")
    for src, name in LOCAL_FILES.items():
        require(digest(folder / name) == entries[src], f"Archived reference differs: {name}")
    return folder


def motion_decoder(ref, linux):
    path = ref / "verify_label_motion.py"
    text = path.read_text()
    env = {"__file__": str(path), "__name__": "portable_label_motion"}
    if linux:
        old = '''adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal-")
            and adapter["isFallbackAdapter"] is False, "Apple Metal Chrome adapter missing"'''
        require(text.count(old) == 1, "Expected the one macOS-only condition in the reviewed decoder")
        text = text.replace(old, 'swiftshader(adapter), "Chrome SwiftShader software adapter missing"')
        env["swiftshader"] = swiftshader
    exec(compile(text, str(path), "exec"), env)
    return env["verify"]


def image_receipt(artifacts, ref, chrome, linux=True):
    metadata = read(artifacts / MOTION)
    require(metadata["browser"] == chrome, "Typed motion report differs from runner Chrome identity")
    mac = read(ref / "mac-motion.json")
    for name in ("outlined", "filled", "compact"):
        require(metadata[name]["palette"] == mac[name]["palette"], f"Altered exact palette for {name}")
    result = motion_decoder(ref, linux)(artifacts, {key: metadata[key] for key in ("browser", "adapter")})
    require(set(result["pngs"]) == set(PNG), "Wrong expected motion captures")
    return {"adapter": metadata["adapter"], "motion": result}


def predecessor_receipt(artifacts, ref, chrome):
    stock = read(ref / "mac-stock.json")
    report = read(artifacts / TEXTBOX)
    namespace = {"__file__": str(ref / "prior-verify-hosted.py"), "__name__": "previous_verifier"}
    exec(compile((ref / "prior-verify-hosted.py").read_text(), namespace["__file__"], "exec"), namespace)
    namespace["verify_report"](report, stock["textbox_values"])
    require(report["browser"] == chrome and swiftshader(report["adapter"]), "Previous report differs from actual Linux adapter")
    first, selected, endings = (read(artifacts / name) for name in IME)
    require(first["before"]["value"] == first["after"]["value"] == "台文"
            and first["before"]["browser"] == first["after"]["browser"] == [0, 2]
            and first["before"]["runtime"] == first["after"]["runtime"] == ["台文", "台文", "0", "6", "false"]
            and first["after"]["trustedSelectAll"] is True
            and first["redraw_width"] > 0 and selected["canceled"]["runtime"][2:] == ["6", "3", "false"]
            and selected["canceled"]["direction"] == "backward"
            and selected["moved"]["trustedRight"] is True and selected["unpainted"]["value"] == selected["repainted"]["value"] == "台文X"
            and endings["authored"] == "ab\r\n你🦀\rZ" and endings["browser_value"] == "ab\n你🦀\nZ"
            and endings["steps"][1]["direction"] == "backward" and endings["steps"][1]["runtime"] == [4, 2],
            "Wrong actual IME retained-caret/cancel readback")
    return {"inactive_textboxes": len(stock["textbox_values"]["inactive_ax"]),
            "raw_ime_sha256": {name: digest(artifacts / name) for name in IME}}


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
    pin_commit(read(folder / "checkout.json"), checkout, TREE, [BASE] if event == "push" else [BASE, HEAD])
    artifact = read(folder / "artifact.json")
    pin_artifact(artifact, run_id, jobs["browser-e2e"])
    summaries, hashes = logs(folder, compressed, event)
    graphics = image_receipt(folder, ref, summaries["browser"]["browser"])
    preceding = predecessor_receipt(folder, ref, summaries["browser"]["browser"])
    return {"event": event, "attempt": 1, "run": run_id, "checkout": checkout, "tree": TREE,
            "jobs": {"build-and-behavior": build_id, "browser-e2e": browser_id},
            "artifact": {"id": artifact["id"], "sha256": artifact["digest"][7:], "bytes": artifact["size_in_bytes"]},
            "logs": hashes, "linux": summaries, **graphics, "predecessor": preceding,
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
    pin_pr(read(root / "pr.json"))
    pin_commit(read(root / "head.json"), HEAD, TREE, [BASE])
    pin_commit(read(root / "base.json"), BASE, BASE_TREE)
    pin_commit(read(root / "merge.json"), MERGE, TREE, [BASE, HEAD])
    ref = references(root / "reference")
    return {"pr": PR, "head": HEAD, "base": BASE, "tree": TREE,
            "candidate_evidence_commit": LOCAL_EVIDENCE, "candidate_manifest_sha256": LOCAL_SEAL,
            "workflows": {event: event_proof(root / event, event, ref, True) for event in RUNS}}


def export(originals, candidate, out):
    out.mkdir()
    references(out / "reference", candidate)
    for name in ("pr", "head", "base", "merge"):
        shutil.copyfile(originals / f"{name}.json", out / f"{name}.json")
    for event in RUNS:
        source, folder = originals / event, out / event
        folder.mkdir()
        for name in META:
            shutil.copyfile(source / name, folder / name)
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
        with TemporaryDirectory(prefix="surface-q67-hosted-preflight-", dir="/tmp") as tmp:
            ref = references(Path(tmp) / "reference", args.candidate)
            local = args.candidate / "stock-macos/run/artifacts"
            actual = read(local / MOTION)
            candidate = image_receipt(local, ref, actual["browser"], False)
        runs = collect_metadata()
        result = {"status": "read-only preflight", "local_image_captures_decoded": len(candidate["motion"]["pngs"]),
                  "runs": {event: {"run": value[0]["id"], "status": value[0]["status"],
                                   "conclusion": value[0]["conclusion"]} for event, value in runs.items()}}
    elif args.collect:
        require(args.collect.resolve().is_relative_to(Path("/tmp").resolve()), "Collect into fresh /tmp")
        result = collect(args.collect)
    elif args.export:
        require(args.export.resolve().is_relative_to(Path("/tmp").resolve()), "Export into fresh /tmp")
        result = export(args.originals, args.candidate, args.export)
    else:
        result = verify(args.verify_package, args.package_sha)
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
