"""Allow reuse only when the sealed inputs differ in the stock runner test."""

from hashlib import file_digest
import json
from pathlib import Path
import sys

OLD = '82d29caaad575703758710ec4e068716dee32a7c099f277e21930827d115ad23'
NEW = '386aec95d1ad1b3fe7dfb08393eb678055eebd895cd8b1a83ae445677987fd9f'
OLD_BUNDLE = 'fb08e7575e9a0b50ad6931cfa814b5917736a4c531fb444f5642bc5054cd22e1'
PATCH = 'ff36ab46cd28f03606aa7776fafb0b45f770f1cef521f6c3c795c6b8c186618f'
TARGET = 'e2e/src/material_search_overlay.rs'
RUNNER = 'debug/surface-e2e'


def digest(path):
    with path.open('rb') as file:
        return file_digest(file, 'sha256').hexdigest()


def manifest(path):
    rows = [line.split(None, 1) for line in path.read_text().splitlines()]
    entries = {name: sha for sha, name in rows}
    assert len(entries) == len(rows)
    return entries


def validate_prior(old, stage, root):
    old_sources = old / 'incoming-source.sha256'
    new_sources = stage / 'incoming-source.sha256'
    patch = root / 'adapter-check.patch'
    old_binaries = old / 'browser-binary-sha256.txt'
    assert digest(old_sources) == OLD and digest(new_sources) == NEW
    assert digest(patch) == PATCH and digest(old_binaries) == OLD_BUNDLE
    past, current = manifest(old_sources), manifest(new_sources)
    assert len(set(past) & set(current)) == len(past) == len(current) == 557
    assert [name for name, sha in past.items() if current[name] != sha] == [TARGET]
    return past, current, manifest(old_binaries)


def verify_payload(old, stage, root, sealing):
    past, current, prior_bins = validate_prior(old, stage, root)
    if not sealing:
        return
    new_binaries = stage / 'browser-binary-sha256.txt'
    new_bins = manifest(new_binaries)
    assert set(prior_bins) == set(new_bins)
    assert [name for name, sha in prior_bins.items() if new_bins[name] != sha] == [RUNNER]
    reused = {name: sha for name, sha in prior_bins.items() if name != RUNNER}
    for name, sha in reused.items():
        assert digest(old / 'bundle' / name) == digest(stage / 'bundle' / name) == sha
    assert digest(stage / 'bundle' / RUNNER) == new_bins[RUNNER]
    path = stage / 'payload-reuse.json'
    path.write_text(json.dumps({
        'candidate_head': 'fc233e9a592d886f56dbc1397234edeaded2331b',
        'candidate_tree': '35f17666034d0f39367dc3b139e06a927fca0455',
        'prior_source_manifest_sha256': OLD, 'exact_source_manifest_sha256': NEW,
        'changed_source': {TARGET: {'before': past[TARGET], 'after': current[TARGET]}},
        'patch_sha256': PATCH,
        'prior_binary_manifest_sha256': OLD_BUNDLE,
        'refreshed_binary_manifest_sha256': digest(new_binaries),
        'rebuilt_runner': {RUNNER: new_bins[RUNNER]},
        'reused_payload_byte_equal': reused,
    }, indent=2) + '\n')


if __name__ == '__main__':
    mode, prior, stage = sys.argv[1:]
    assert mode in ('preflight', 'seal')
    verify_payload(Path(prior), Path(stage), Path(__file__).resolve().parent, mode == 'seal')
