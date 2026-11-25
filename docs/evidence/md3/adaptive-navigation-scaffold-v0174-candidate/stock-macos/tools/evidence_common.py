"""File and receipt helpers reused from the v0.1.66 evidence packer."""

from contextlib import contextmanager
import gzip
from hashlib import file_digest, sha1, sha256
import json
import os
from pathlib import Path, PurePosixPath
import re
from runpy import run_path
import shutil
import subprocess
import sys
import tomllib


HEX = re.compile(r"[a-f0-9]{64}")

def require(value, message):
    if not value:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def data(path):
    return json.loads(path.read_text())


def json_file(path, content):
    path.write_text(json.dumps(content, indent=2, ensure_ascii=False, sort_keys=True) + "\n")


def normalized(value):
    return json.loads(json.dumps(value))


def path_entries(path, *, allow_dot_prefix=False):
    entries = {}
    for line in path.read_text().splitlines():
        value, name = line.split("  ", 1)
        if allow_dot_prefix:
            name = name.removeprefix("./")
        relative = PurePosixPath(name)
        require(HEX.fullmatch(value) and relative.as_posix() == name and not relative.is_absolute()
                and ".." not in relative.parts and name not in entries and name != ".",
                f"Invalid manifest path: {name}")
        entries[name] = value
    require(entries, f"Empty manifest: {path}")
    return entries


def verify_entries(root, entries):
    for name, expected in entries.items():
        path = root / name
        require(path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(root.resolve())
                and digest(path) == expected, f"Different or missing pinned file: {path}")


def seal(root, name, expected, *, exact=False):
    require(HEX.fullmatch(expected or "") and digest(root / name) == expected, f"Wrong seal for {root / name}")
    entries = path_entries(root / name)
    verify_entries(root, entries)
    if exact:
        files = {str(file.relative_to(root)) for file in root.rglob("*") if file.is_file() and file != root / name}
        require(files == entries.keys(), "Portable proof has unsealed files")
    return entries


def object_id(kind, content):
    return sha1(f"{kind} {len(content)}\0".encode() + content).digest()


def source_tree(root, entries):
    folders = {}
    for name, checksum in entries.items():
        *parents, filename = PurePosixPath(name).parts
        folder = folders
        for parent in parents:
            folder = folder.setdefault(parent, {})
        path = root / name
        payload = path.read_bytes()
        require(sha256(payload).hexdigest() == checksum, f"Source differs during tree verification: {name}")
        folder[filename] = (b"100755" if path.stat().st_mode & 0o100 else b"100644", object_id("blob", payload))

    def tree(folder):
        items = []
        for name, child in sorted(folder.items(), key=lambda item: (item[0]+("/" if isinstance(item[1], dict) else "")).encode()):
            mode, oid = (b"40000", tree(child)) if isinstance(child, dict) else child
            items.append(mode + b" " + name.encode() + b"\0" + oid)
        return object_id("tree", b"".join(items))

    return tree(folders).hex()


@contextmanager
def import_from(root):
    sys.path.insert(0, str(root))
    try:
        yield
    finally:
        sys.path.pop(0)


def run_json(argv):
    result = subprocess.run([sys.executable, "-E", "-B", *map(str, argv)], capture_output=True, text=True, check=False)
    require(result.returncode == 0, f"Offline verifier failed: {result.stderr.strip()}")
    return json.loads(result.stdout)


def raw_log(path):
    return gzip.decompress(path.read_bytes()) if path.suffix == ".gz" else path.read_bytes()


def tests(log):
    payload = raw_log(log)
    text = payload.decode()
    rows = re.findall(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", text, re.M)
    require(rows and all(state == "ok" and failed == "0" for state, _, failed, _ in rows),
            f"Failed or missing exact tests in {log.name}")
    return text, {"passed": sum(int(passed) for _, passed, _, _ in rows),
                  "ignored": sum(int(ignored) for _, _, _, ignored in rows),
                  "log_sha256": sha256(payload).hexdigest()}


def clean_windows(watch):
    require(watch["stock_exit"] == 0 and watch["visible_owned_windows"] == []
            and watch["remaining_observed_processes"] == [] and watch["problems"] == [],
            "Guard recorded an error, visible window or surviving process")


def verify_ledgers(run, watch, roles):
    ledger = [data(file) for file in (run / "guard-ledger").glob("*.json")]
    events = watch["guard_events"]
    require(len(events) == len(roles) and {event["role"] for event in events} == roles
            and len({event["pid"] for event in events}) == len(roles)
            and sorted(ledger, key=lambda event: event["pid"]) == sorted(events, key=lambda event: event["pid"]),
            "Wrong original guard ledger or launch scope")
    for event in events:
        group = watch["launch_observations"][event["role"]]
        require(event["allowed"] is True and group["pids"] == [event["pid"]] and group["samples"] > 0,
                "Incomplete guarded process samples")
        if event["role"] == "native":
            require(event["argv"] == ["--headless", "--width", "720", "--height", "640"] and event["profile"] is None,
                    "Native launch lost exact offscreen flags")
        else:
            headless = [flag for flag in event["argv"] if flag == "--headless" or flag.startswith("--headless=")]
            profiles = [flag for flag in event["argv"] if flag.startswith("--user-data-dir=")]
            require(headless == ["--headless=new"] and profiles == [f'--user-data-dir={event["profile"]}']
                    and "/tmp/" in event["profile"], "Chrome launch lost exact headless flags or private profile")


def copy(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)


def compressed(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(gzip.compress(source.read_bytes(), mtime=0))


def copy_members(source, target, names):
    for name in sorted(set(names)):
        copy(source / name, target / name)
