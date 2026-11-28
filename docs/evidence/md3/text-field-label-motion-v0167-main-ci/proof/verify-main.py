"""Collect or verify the original v0.1.67 postmerge Linux CI evidence."""
import argparse
from datetime import datetime
from hashlib import sha256
import gzip
import json
from pathlib import Path
from runpy import run_path
from shutil import copy2, copytree

HERE = Path(__file__).resolve().parent
READER_SHA = "6d4fc1a8ccd7fba0094aa305529f02f59426028ada260a015b1f01f20c1716a2"
HEAD = "6fb40370ca824b8ed44e6e17099e6156f8d15a9b"
PARENT = "a751900b2f2c4c5f204afc8c4b9a0e49d1f477cf"
PR_HEAD = "7da557d095d16eb43a5725c9e5e580ce198cd8b2"
TREE = "c218faaf7527ec2872fb1ec1097c87667f3bf1f2"
REPO = "faberline/surface"
RUN = 46113171688
JOBS = {"build-and-behavior": 140881014214, "browser-e2e": 140881014024}
PR = 243
RECEIPT = "verified.json"
SOURCE_METADATA = ("main-ref", "merged-pr", "checkout", "run", "jobs")
BUILD_STEPS = (
    "Run cargo xtask provenance",
    "Run cargo fmt --all --check",
    "Run cargo clippy --locked --workspace --all-targets --all-features -- -D warnings",
    "Run cargo test --locked --workspace --all-features",
    "Run cargo check --locked --target wasm32-unknown-unknown -p surface-showcase -p surface-material-gallery --all-features --lib",
    "Run cargo check --locked --target wasm32-unknown-unknown -p ui-runtime --all-features --lib --test consumer_contract",
    "Run cargo check --locked --target wasm32-unknown-unknown -p ui-runtime --no-default-features --lib --test consumer_contract",
)

if sha256((HERE / "prior-reader.py").read_bytes()).hexdigest() != READER_SHA:
    raise SystemExit("Reviewed Label Motion decoder bytes differ.")
reader = run_path(str(HERE / "prior-reader.py"))
require, digest, read, write, gh = (reader[name] for name in ("require", "digest", "read", "write", "gh"))


def pin_source(info):
    ref, pr, checkout = (info[name] for name in ("main-ref", "merged-pr", "checkout"))
    require(ref["ref"] == "refs/heads/main" and ref["object"]["type"] == "commit"
            and ref["object"]["sha"] == HEAD, "GitHub main is not the exact merged source")
    require(pr["number"] == PR and pr["state"] == "closed" and pr["merged"] is True
            and pr["merge_commit_sha"] == HEAD and pr["base"]["ref"] == "main"
            and pr["base"]["sha"] == PARENT and pr["head"]["sha"] == PR_HEAD
            and pr["base"]["repo"]["full_name"] == pr["head"]["repo"]["full_name"] == REPO,
            "PR #243 has different merge ancestry")
    reader["pin_commit"](checkout, HEAD, TREE, [PARENT])


def pin_jobs(run, jobs, terminal=False):
    identity = (run["id"], run["run_attempt"], run["head_sha"], run["event"], run["head_branch"], run["path"])
    require(identity == (RUN, 1, HEAD, "push", "main", ".github/workflows/ci.yml")
            and run["head_repository"]["full_name"] == REPO, "Wrong original postmerge run, branch or attempt")
    by_name = {job["name"]: job for job in jobs["jobs"]}
    require(set(by_name) == set(JOBS) and len(jobs["jobs"]) == len(JOBS), "Wrong original postmerge jobs")
    step_names = {"build-and-behavior": BUILD_STEPS,
                  "browser-e2e": ("Run surface-e2e", "Run actions/upload-artifact@v4")}
    for name, identity in JOBS.items():
        job = by_name[name]
        require((job["id"], job["run_id"], job["run_attempt"], job["head_sha"]) == (identity, RUN, 1, HEAD),
                f"Wrong original {name} identity")
        steps = {step["name"]: step for step in job["steps"]}
        expected = ("Run actions/checkout@v4", *step_names[name])
        require(all(step in steps for step in expected), f"Wrong original {name} step inventory")
        if terminal:
            require(job["status"] == "completed" and job["conclusion"] == "success"
                    and all(steps[step]["status"] == "completed" and steps[step]["conclusion"] == "success"
                            for step in expected), f"Original {name} has no passing result")
    if terminal:
        require(run["status"] == "completed" and run["conclusion"] == "success",
                "The exact original main workflow has no successful terminal result")
    return by_name


def read_metadata():
    attempt = f"actions/runs/{RUN}/attempts/1"
    metadata = {"main-ref": gh("git/ref/heads/main"), "merged-pr": gh(f"pulls/{PR}"),
                "checkout": gh(f"git/commits/{HEAD}"), "run": gh(attempt),
                "jobs": gh(f"{attempt}/jobs?per_page=100")}
    pin_source(metadata)
    pin_jobs(metadata["run"], metadata["jobs"])
    return metadata


def pin_artifact(artifact, browser):
    source = artifact["workflow_run"]
    require(artifact["name"] == "surface-e2e-artifacts" and artifact["expired"] is False
            and source["id"] == RUN and source["head_sha"] == HEAD and source["head_branch"] == "main"
            and isinstance(artifact["digest"], str) and artifact["digest"].startswith("sha256:")
            and len(artifact["digest"]) == 71 and all(c in "0123456789abcdef" for c in artifact["digest"][7:])
            and artifact["size_in_bytes"] > 0, "Artifact does not belong to the exact main run")
    created = datetime.fromisoformat(artifact["created_at"])
    require(datetime.fromisoformat(browser["started_at"]) <= created <= datetime.fromisoformat(browser["completed_at"]),
            "Artifact creation is outside the original browser job")


def output(path):
    path = path.resolve()
    require(path.is_relative_to(Path("/tmp").resolve()) and path.parent.is_dir() and not path.exists(),
            "Use a fresh output directory under /tmp")
    return path


def collect(folder):
    metadata = read_metadata()
    jobs = pin_jobs(metadata["run"], metadata["jobs"], terminal=True)
    artifact_list = gh(f"actions/runs/{RUN}/artifacts?per_page=100")
    artifacts = [item for item in artifact_list["artifacts"] if item["name"] == "surface-e2e-artifacts"]
    require(len(artifacts) == 1, "Expected one official main workflow ZIP")
    artifact = artifacts[0]
    pin_artifact(artifact, jobs["browser-e2e"])
    folder.mkdir()
    for name, value in metadata.items():
        write(folder / f"{name}.json", value)
    write(folder / "artifact.json", artifact)
    gh(f"actions/jobs/{JOBS['build-and-behavior']}/logs", folder / "build.log")
    gh(f"actions/jobs/{JOBS['browser-e2e']}/logs", folder / "browser.log")
    gh(f"actions/artifacts/{artifact['id']}/zip", folder / "original.zip")
    zipper = folder / "original.zip"
    require(zipper.stat().st_size == artifact["size_in_bytes"] and digest(zipper) == artifact["digest"][7:],
            "Downloaded original main ZIP differs from GitHub's digest or size")
    return {"originals": str(folder), "artifact": artifact["id"], "sha256": digest(zipper),
            "bytes": zipper.stat().st_size, "run": RUN, "attempt": 1}


def log_proof(folder, compressed):
    findings, hashes = {}, {}
    for name, parse in (("build", reader["build_log"]), ("browser", reader["browser_log"])):
        path = folder / (f"{name}.log.gz" if compressed else f"{name}.log")
        raw = path.read_bytes()
        raw = gzip.decompress(raw) if compressed else raw
        log = raw.decode().replace("\x1b[0m", "")
        require(reader["checkout_in_log"](log) == [HEAD], f"The original {name} log checked out a different commit")
        findings[name] = parse(log)
        hashes[name] = sha256(raw).hexdigest()
    return findings, hashes


def archive_proof(folder):
    info = {name: read(folder / f"{name}.json") for name in SOURCE_METADATA}
    pin_source(info)
    jobs = pin_jobs(info["run"], info["jobs"], terminal=True)
    artifact = read(folder / "artifact.json")
    pin_artifact(artifact, jobs["browser-e2e"])
    ref = reader["references"](folder / "reference")
    linux, hashes = log_proof(folder, compressed=True)
    require(linux["build"] == {"passed": 1545, "ignored": 20, "wasm_checks": 3}
            and linux["browser"]["passed"] == 8 and linux["browser"]["material_steps"] == 349,
            "Hosted results differ from the reviewed exact source scope")
    chrome = linux["browser"]["browser"]
    return {"scope": "original postmerge push, attempt 1", "pr": PR, "head": HEAD, "base": PARENT, "tree": TREE,
            "run": RUN, "jobs": JOBS, "artifact": {"id": artifact["id"], "bytes": artifact["size_in_bytes"],
                                                    "sha256": artifact["digest"][7:]},
            "logs": hashes, "linux": linux, **reader["image_receipt"](folder, ref, chrome),
            "predecessor": reader["predecessor_receipt"](folder, ref, chrome),
            "selected_artifacts": {name: digest(folder / name) for name in sorted(reader["SELECTED"])}}


def export(originals, folder):
    info = {name: read(originals / f"{name}.json") for name in SOURCE_METADATA}
    pin_source(info)
    jobs = pin_jobs(info["run"], info["jobs"], terminal=True)
    pin_artifact(read(originals / "artifact.json"), jobs["browser-e2e"])
    log_proof(originals, compressed=False)
    folder.mkdir()
    for name in (*SOURCE_METADATA, "artifact"):
        copy2(originals / f"{name}.json", folder / f"{name}.json")
    for name in ("build", "browser"):
        raw = (originals / f"{name}.log").read_bytes()
        (folder / f"{name}.log.gz").write_bytes(gzip.compress(raw, mtime=0))
    reader["unpack_original"](originals, folder)
    for name in ("README.md", "verify-main.py", "prior-reader.py"):
        copy2(HERE / name, folder / name)
    copytree(HERE / "reference", folder / "reference")
    result = archive_proof(folder)
    write(folder / RECEIPT, result)
    manifest = folder / "SHA256SUMS"
    manifest.write_text("".join(f"{digest(file)}  {file.relative_to(folder).as_posix()}\n"
                                for file in sorted(folder.rglob("*")) if file.is_file()))
    return {"portable": str(folder), "manifest_sha256": digest(manifest), "files": len(reader["manifest"](manifest)),
            "checks": result["linux"]["browser"]["checks"], "build_tests": result["linux"]["build"]["passed"]}


def verify(folder, pin):
    require(digest(folder / "SHA256SUMS") == pin, "Wrong portable evidence manifest")
    inventory = reader["manifest"](folder / "SHA256SUMS")
    names = {p.relative_to(folder).as_posix() for p in folder.rglob("*") if p.is_file()} - {"SHA256SUMS"}
    require(names == set(inventory) and all(not (folder / name).is_symlink()
                                           and digest(folder / name) == expected for name, expected in inventory.items()),
            "Portable evidence member differs")
    proof = archive_proof(folder)
    require(proof == read(folder / RECEIPT), "Portable postmerge proof differs from independent readback")
    return proof


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--preflight", action="store_true")
    modes.add_argument("--collect", type=Path)
    modes.add_argument("--export", type=Path)
    modes.add_argument("--verify", type=Path)
    parser.add_argument("--originals", type=Path)
    parser.add_argument("--sha")
    args = parser.parse_args()
    require(__debug__, "Run Python without -O")
    reader["references"](HERE / "reference")
    if args.preflight:
        data = read_metadata()
        run = data["run"]
        result = {"status": "read-only preflight", "run": RUN, "attempt": 1, "head": HEAD, "base": PARENT,
                  "tree": TREE, "workflow_status": run["status"], "workflow_conclusion": run["conclusion"],
                  "jobs": {name: {"id": job["id"], "status": job["status"], "conclusion": job["conclusion"]}
                           for name, job in pin_jobs(run, data["jobs"]).items()}}
    elif args.collect:
        result = collect(output(args.collect))
    elif args.export:
        result = export(args.originals.resolve(strict=True), output(args.export))
    else:
        result = verify(args.verify.resolve(strict=True), args.sha)
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
