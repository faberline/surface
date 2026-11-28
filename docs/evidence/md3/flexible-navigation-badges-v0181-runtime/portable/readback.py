"""Verify the portable flexible navigation Badge evidence with an external manifest hash."""
import argparse
import json
from hashlib import file_digest
from pathlib import Path, PurePosixPath
import re
import sys

ROOT = Path(__file__).resolve().parent


def sha(path):
    with path.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def data(path):
    return json.loads(path.read_text())


def verify_inventory(root, manifest, external_sha):
    assert sha(manifest) == external_sha
    expected = {}
    for line in manifest.read_text().splitlines():
        digest, filename = line.split('  ', 1)
        relative = PurePosixPath(filename)
        assert re.fullmatch(r'[0-9a-f]{64}', digest)
        assert str(relative) == filename and not relative.is_absolute() and '..' not in relative.parts
        assert filename not in expected
        target = root / filename
        assert target.is_file() and not target.is_symlink() and target.resolve().is_relative_to(root)
        assert sha(target) == digest
        expected[filename] = digest
    actual = {p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file() and p != manifest}
    assert actual == set(expected)
    return len(actual)


def counts(log):
    rows = re.findall(r'^test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', log, re.M)
    assert rows and 'test result: FAILED' not in log
    return tuple(sum(int(row[i]) for row in rows) for i in range(2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest-sha', required=True)
    args = parser.parse_args()
    files = verify_inventory(ROOT, ROOT / 'SHA256SUMS', args.manifest_sha)
    pins = data(ROOT / 'pins.json')
    assert pins['commit'] == 'b9dc4ad7a5c6cdddd71196a669354f17984af7aa'
    assert pins['tree'] == 'ea61008a90b12a3f753398b9e5c1ff58f5813b3d'
    assert sha(ROOT / 'source/badges-source.bundle') == pins['source_bundle_sha256']
    assert sha(ROOT / 'source/source.sha256') == pins['source_manifest_sha256']
    assert sha(ROOT / 'build/stock.sha256') == pins['browser_build_sha256']
    assert sha(ROOT / 'build/native.sha256') == pins['native_build_sha256']
    assert (ROOT / 'build/BUILD-COMPLETE').read_text().splitlines() == [pins['tree'], pins['source_manifest_sha256']]
    assert sha(ROOT / 'native/config.json') == pins['native_config_sha256']
    assert sha(ROOT / 'browser/config.json') == pins['browser_config_sha256']

    sys.path.insert(0, str(ROOT / 'native/tools'))
    from replay import verify as native_verify
    native = native_verify(ROOT / 'native/originals', pins['native_archive_sha256'],
                           pins['native_config_sha256'], pins['source_manifest_sha256'],
                           pins['native_build_sha256'])
    assert native == {'captures':10, 'window_samples':18, 'reaped_children':8,
                      'scope':'headless screenshots; no physical display or VoiceOver claim'}
    sys.path.insert(0, str(ROOT / 'browser/tools'))
    from browser import verify_log, verify_badges
    browser_dir = ROOT / 'browser'
    browser = data(browser_dir / 'verified-stock-material.json')
    invocation = data(browser_dir / 'invocation.json')
    assert invocation['inputs'] == data(browser_dir / 'inputs-after.json')
    recorded = invocation['inputs']
    assert recorded['verifier_sha256'] == sha(browser_dir / 'tools/browser.py')
    assert recorded['observer_sha256'] == sha(browser_dir / 'tools/observe-focused-original.py')
    assert (recorded['source_commit'], recorded['source_tree'], recorded['source_sha256'],
            recorded['config_sha256'], recorded['build_sha256']) == (
                pins['commit'], pins['tree'], pins['source_manifest_sha256'],
                pins['browser_config_sha256'], pins['browser_build_sha256'])
    assert browser['badge_evidence'] == verify_badges(browser_dir / 'artifacts')
    assert browser['checks'] == verify_log((browser_dir / 'runner.log').read_text())
    hosted = data(browser_dir / 'artifacts/browser-material.json')
    assert not hosted['errors'] and hosted['browser'] == browser['browser']
    window = data(browser_dir / 'headless-window-observation.json')
    assert window['stock_exit'] == 0 and not window['problems'] and not window['visible_owned_windows']
    assert not window['remaining_observed_processes']
    launch, = window['guard_events']
    assert launch['role'] == 'chrome-primary' and launch['allowed'] and '--headless=new' in launch['argv']
    assert browser['chrome_samples'] == window['launch_observations']['chrome-primary']['samples'] > 0

    rust, ignored = counts((ROOT / 'build/check.log').read_text())
    assert (rust, ignored) == (1629, 20)
    metal = ROOT / 'build/metal'
    assert verify_inventory(metal, metal / 'SHA256SUMS', pins['metal_manifest_sha256']) == 23
    offscreen = (metal / 'offscreen.log').read_text()
    selection = (metal / 'text-selection.log').read_text()
    assert 'backend: Metal' in offscreen and 'backend: Metal' in selection
    assert counts(offscreen) == (16, 0) and counts(selection) == (2, 0)
    assert len(list(metal.glob('*.png'))) == 21
    assert browser['material_steps'] == 385 and browser['checks'] > 31830
    assert browser['badge_evidence']['acknowledged']['stable_owner_error_pixels'] == 0
    print(json.dumps({'files':files, 'rust_passed':rust, 'rust_ignored':ignored,
                      'metal_passed':18, 'native':native, 'browser':browser}, indent=2))


if __name__ == '__main__':
    main()
