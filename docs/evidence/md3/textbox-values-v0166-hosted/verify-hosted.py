"""Check and export the original v0.1.66 Linux browser receipts."""

import argparse
import hashlib
import io
import json
import re
from pathlib import Path
from zipfile import ZipFile

from PIL import Image


HEAD = "c846aebd38367c16c697729d06e6cfb5fa71744e"
BASE = "b40bc212fb32ac97b3f207cdfd002814bf48db66"
TREE = "3de0c143c43d35911593cfc95d3ea5931016c308"
RUNS = {
    "pull_request": (45607226398, 139181131933, 139181132408, 15020853613),
    "push": (45607214912, 139181093372, 139181094044, 15021126743),
}
REPORT = "browser-material-multiline-text-field.json"
CANDIDATE_MANIFEST = "613d62bb6abd3ba245c8a9662ad30a088877e3f3aa046f2210a8df931015764b"


def digest(path):
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def log_text(path):
    return path.read_text(encoding="utf-8").replace("\x1b[0m", "")


def node_state(node):
    properties = {entry["name"]: entry["value"].get("value") for entry in node["properties"]}
    assert node["ignored"] is False and node["role"]["value"] == "textbox"
    assert node["childIds"] == [], node["name"]["value"]
    return properties, {
        "label": node["name"]["value"],
        "multiline": properties["multiline"],
        "value": node["value"]["value"],
    }


def verify_report(report, expected):
    assert not report.get("failure") and report["page_errors"] == []
    nodes = report["inactive_ax"]
    assert set(nodes) == set(expected["inactive_ax"]) | {"active"}
    for name, expected_value in expected["inactive_ax"].items():
        properties, actual = node_state(nodes[name])
        assert actual == expected_value, name
        if name in {"readonly", "disabled"}:
            assert properties[name] is True, name
    handoff = nodes["active"]
    assert handoff["label"] == expected["active_handoff"]["label"]
    assert handoff["description"] == expected["active_handoff"]["description"]
    assert handoff["multiline"] is True
    for name, count in (("before", 3), ("after", 8)):
        description = nodes[name]["description"]["value"]
        assert "井字號" in description and f"已輸入 {count} 個字" in description


def image_bytes(archive, member):
    image = archive.read(member)
    with Image.open(io.BytesIO(image)) as opened:
        assert opened.format == "PNG" and min(opened.size) > 0, member
        opened.verify()
    return image


def original(folder, event, pins, expected, destination):
    run_id, build_id, browser_id, artifact_id = pins
    run = read(folder / "run.json")
    assert (run["id"], run["run_attempt"], run["head_sha"], run["event"]) == (run_id, 1, HEAD, event)
    assert run["status"] == "completed" and run["conclusion"] == "success"
    assert run["path"] == ".github/workflows/ci.yml"
    jobs = {job["name"]: job for job in read(folder / "jobs.json")["jobs"]}
    assert set(jobs) == {"build-and-behavior", "browser-e2e"}
    for label, job_id in (("build-and-behavior", build_id), ("browser-e2e", browser_id)):
        job = jobs[label]
        assert (job["id"], job["run_id"], job["run_attempt"], job["head_sha"]) == (job_id, run_id, 1, HEAD)
        assert job["status"] == "completed" and job["conclusion"] == "success"
    artifact = read(folder / "artifact.json")
    assert artifact["id"] == artifact_id and artifact["name"] == "surface-e2e-artifacts"
    assert artifact["expired"] is False
    assert artifact["workflow_run"]["id"] == run_id and artifact["workflow_run"]["head_sha"] == HEAD
    zipped = folder / "original.zip"
    zip_sha = digest(zipped)
    assert artifact["digest"] == "sha256:" + zip_sha
    assert zipped.stat().st_size == artifact["size_in_bytes"]
    build_log = log_text(folder / "build.log")
    browser_log = log_text(folder / "browser.log")
    checkout_pattern = r"git log -1 --format=%H\r?\n\S+ ([0-9a-f]{40})"
    checkout = read(folder / "checkout.json")
    expected_checkout = HEAD if event == "push" else "c80cd9ab8a53b41a78aaa3db3c1aec7c73e91cb8"
    assert checkout["sha"] == expected_checkout and checkout["tree"]["sha"] == TREE
    assert [parent["sha"] for parent in checkout["parents"]] == ([BASE] if event == "push" else [BASE, HEAD])
    for log in (build_log, browser_log):
        assert re.findall(checkout_pattern, log) == [expected_checkout]
    assert "provenance:" in build_log and "test result: ok." in build_log
    assert "swiftshader" in browser_log.lower() and "Inactive textboxes expose their committed text in Chrome" in browser_log
    steps = [int(step) for step in re.findall(r"\[M(\d+)\]", browser_log)]
    assert steps == [step for step in range(1, 347) if step != 23]
    summaries = re.findall(r"e2e result: (\d+) passed; (\d+) failed; (\d+) checks", browser_log)
    assert summaries == [("8", "0", "25127")], summaries

    copied = {}
    with ZipFile(zipped) as archive:
        names = archive.namelist()
        reports = [name for name in names if Path(name).name == REPORT]
        assert len(reports) == 1, reports
        report = reports[0]
        report_bytes = archive.read(report)
        verify_report(json.loads(report_bytes), expected)
        copied[REPORT] = report_bytes
        photos = [name for name in names if Path(name).name.startswith("material-multiline-text-field-") and name.endswith(".png")]
        assert len(photos) == 12, photos
        for photo in photos:
            filename = Path(photo).name
            assert "failure" not in filename and filename not in copied, filename
            copied[filename] = image_bytes(archive, photo)
    if destination is not None:
        destination.mkdir()
        for name, contents in copied.items():
            (destination / name).write_bytes(contents)
    return {
        "event": event,
        "run": run_id,
        "attempt": 1,
        "checkout": expected_checkout,
        "checkout_tree": TREE,
        "jobs": {"build-and-behavior": build_id, "browser-e2e": browser_id},
        "artifact": {"id": artifact_id, "sha256": zip_sha, "bytes": zipped.stat().st_size},
        "logs": {name: digest(folder / f"{name}.log") for name in ("build", "browser")},
        "passed": 8,
        "checks": 25127,
        "material_steps": len(steps),
        "files": {name: hashlib.sha256(contents).hexdigest() for name, contents in sorted(copied.items())},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("originals", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--export", type=Path)
    args = parser.parse_args()
    if not __debug__:
        parser.error("verification requires assertions")
    assert digest(args.candidate / "SHA256SUMS") == CANDIDATE_MANIFEST
    candidate = read(args.candidate / "verified-textbox-values-v0166.json")
    assert candidate["source"]["tree"] == TREE
    source = args.originals
    receipt = {"head": HEAD, "tree": TREE, "local_manifest_sha256": CANDIDATE_MANIFEST, "workflows": {}}
    for event, pins in RUNS.items():
        output = args.export / event if args.export else None
        receipt["workflows"][event] = original(source / str(pins[0]), event, pins, candidate["focused"], output)
    print(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
