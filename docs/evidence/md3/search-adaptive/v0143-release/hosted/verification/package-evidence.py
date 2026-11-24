"""Preserve the exact v0.1.43 Linux stock evidence for each workflow event."""

from hashlib import sha256
import json
from pathlib import Path
import re
from shutil import copy2
import subprocess
import sys
from tempfile import TemporaryDirectory
from zipfile import ZipFile

from PIL import Image

from stock_contrast_report import png_rows, validate as validate_contrast
from verify_search_report import PREFIX as SEARCH_PREFIX, validate as validate_search

ROOT = Path(__file__).resolve().parent
WORKFLOW = ROOT / 'ci.yml'
if not WORKFLOW.is_file():
    WORKFLOW = ROOT.parent / 'ci.yml'
EVENTS = ('pull_request', 'push')
MATERIAL = r'\[M(?:24[1-9]|25[0-5])\]'
TEST_RECORD = r'test result: ok\. \d+ passed; 0 failed; \d+ ignored; \d+ measured; \d+ filtered out; finished in [\d.]+s'
LOG_LINE = re.compile(
    r'^\S+ (?P<details>##\[group\]Operating System|Ubuntu|'
    r'\[command\]/usr/bin/git log -1 --format=%H|[0-9a-f]{40}|'
    r'##\[group\]Run xvfb-run -a -s "-screen 0 1920x1080x24" cargo xtask web e2e|'
    r'  \[I\d+\] .*|  ' + MATERIAL + r' .*|'
    r'case \S+ \.\.\. ok \(\d+ steps, \d+ checks, [\d.]+s\)|'
    r'e2e result: 8 passed; 0 failed; \d+ checks;)(?: artifacts in .*)?$'
)
BUILD_LINE = re.compile(
    r'^\S+ (?P<details>##\[group\]Operating System|Ubuntu|'
    r'\[command\]/usr/bin/git log -1 --format=%H|[0-9a-f]{40}|'
    r'##\[group\]Run cargo (?:xtask provenance|fmt --all --check|clippy --locked [\w -]+|'
    r'test --locked [\w -]+|check --locked --target wasm32-unknown-unknown [\w -]+)|'
    r'provenance: \d+ of \d+ Core files unchanged, \d+ listed as modified, '
    r'\d+ font files match \(\d+ self-test cases passed\)|' + TEST_RECORD + r')$'
)


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text())


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def log_excerpt(path, pattern, checkout):
    lines = path.read_text().splitlines()
    rows = []
    for number, line in enumerate(lines):
        match = pattern.fullmatch(line)
        if not match:
            continue
        details = match['details']
        if re.fullmatch(r'[0-9a-f]{40}', details):
            if number == 0 or 'git log -1 --format=%H' not in lines[number - 1]:
                continue
        rows.append(line[:match.end('details')])
    assert any(line.endswith(checkout) for line in rows)
    return rows


def browser_excerpt(source, target, receipt):
    rows = log_excerpt(source, LOG_LINE, receipt['checkout_commit'])
    assert len([line for line in rows if ' ... ok (' in line]) == 8
    assert len([line for line in rows if re.search(r'\[I\d+\]', line)]) == 20
    assert len([line for line in rows if re.search(MATERIAL, line)]) == 15
    assert any(line.endswith(f"{receipt['total_checks']} checks;") for line in rows)
    target.write_text('\n'.join(rows) + '\n')


def build_excerpt(source, target, receipt):
    rows = log_excerpt(source, BUILD_LINE, receipt['checkout_commit'])
    steps = [line.split(']Run ', 1)[1] for line in rows if '##[group]Run cargo' in line]
    expected = [line.strip().removeprefix('- run: ') for line in WORKFLOW.read_text().splitlines()
                if line.strip().startswith('- run: cargo ')]
    assert steps == expected, steps
    results = [re.search(TEST_RECORD, line).group(0) for line in rows if re.search(TEST_RECORD, line)]
    assert results
    target.write_text('\n'.join(rows) + '\n')
    passed = sum(int(re.search(r'(\d+) passed', line).group(1)) for line in results)
    ignored = sum(int(re.search(r'(\d+) ignored', line).group(1)) for line in results)
    return {'checkout_commit': receipt['checkout_commit'], 'steps': steps, 'test_summaries': len(results),
            'workspace_test_cases_passed': passed, 'workspace_test_cases_ignored': ignored}


def artifact_file(source, filename):
    path, = list(source.rglob(filename))
    return path


def png_from_download(source, target, filename, expected):
    path = artifact_file(source, filename)
    assert digest(path) == expected['sha256'], filename
    if 'rgba_sha256' in expected:
        with Image.open(path) as image:
            assert image.format == 'PNG' and list(image.size) == expected['physical_size']
            image.load()
            assert sha256(image.convert('RGBA').tobytes()).hexdigest() == expected['rgba_sha256']
    copy2(path, target / filename)


def verify_archive(event_path, receipt, target, included_names):
    archive = event_path / 'surface-e2e-artifacts.zip'
    remote = receipt['artifact']
    expected = remote['digest'].removeprefix('sha256:')
    if not archive.exists():
        with archive.open('xb') as output:
            subprocess.run(['gh', 'api', f"repos/faberline/surface/actions/artifacts/{remote['id']}/zip"],
                           stdout=output, check=True)
    assert digest(archive) == expected, (event_path, remote['id'])
    with ZipFile(archive) as source:
        by_name = {}
        for entry in source.namelist():
            by_name.setdefault(Path(entry).name, []).append(entry)
        for name in included_names:
            member, = by_name[name]
            assert (target / name).read_bytes() == source.read(member), name
    write(target / 'verified-original-artifact.json', {
        'id': remote['id'], 'sha256': expected, 'bytes': archive.stat().st_size,
        'archived_raw_members_verified_byte_for_byte': sorted(included_names),
    })


def stage_event(event, receipt, directory, target):
    target.mkdir()
    path = directory / event
    original = path / 'download'
    per_event = path / 'verified' / 'run-verified.json'
    assert read(per_event) == receipt
    copy2(per_event, target / per_event.name)
    browser_excerpt(path / f'{event}-browser-e2e.log', target / 'browser-e2e-excerpt.log', receipt)
    build = build_excerpt(path / f'{event}-build-and-behavior.log', target / 'build-and-behavior-excerpt.log', receipt)
    status = read(path / 'status.json')
    run = status['run']
    assert run['status'] == 'completed' and run['conclusion'] == 'success'
    assert run['head_sha'] == receipt['head'] and run['id'] == receipt['run']
    for name, job in status['jobs'].items():
        assert job['status'] == 'completed' and job['conclusion'] == 'success'
        assert job['id'] == receipt['jobs'][name]['id']
        build_status = {key: job[key] for key in ('id', 'name', 'run_id', 'run_attempt', 'head_sha', 'html_url',
                                                  'status', 'conclusion', 'started_at', 'completed_at')}
        build_status['steps'] = [{key: step[key] for key in ('number', 'name', 'status', 'conclusion')}
                                 for step in job['steps']]
        write(target / f'{name}-job.json', build_status)
    build['reported_job_status'] = status['jobs']['build-and-behavior']['conclusion']
    write(target / 'build-and-behavior-receipt.json', build)

    parent = artifact_file(original, 'browser-material-search-overlay.json').parent
    reports = set(receipt['page_errors']['material']) | set(receipt['page_errors']['showcase'])
    reports.update(file.name for file in parent.glob('browser-material-web-contrast*.json'))
    reports.update(file.name for file in parent.glob('material-web-contrast-*.json'))
    reports.update(('browser-ime-line-endings.json', 'browser-ime-cancel-redraw.json', 'browser-ime-cancel-selected-redraw.json'))
    for filename in sorted(reports):
        copy2(artifact_file(original, filename), target / filename)

    select_all = read(target / 'browser-ime-cancel-redraw.json')
    backward = read(target / 'browser-ime-cancel-selected-redraw.json')
    assert select_all['redraw_width'] == 1001
    assert select_all['after']['trustedSelectAll'] and select_all['before']['trustedSelectAll']
    assert select_all['before']['runtime'] == select_all['after']['runtime'] == ['台文', '台文', '0', '6', 'false']
    assert backward['canceled']['frames'] == backward['moved']['frames'] == backward['unpainted']['frames']
    assert backward['repainted']['frames'] > backward['unpainted']['frames']
    assert backward['moved']['runtime'] == ['台文', '台文', '6', '6', 'false']
    contrast = receipt['contrast']
    assert contrast['independent_png_pixel_verification']
    expected_contrast = 28 if contrast['foreground_evidence'] == 'browser' else 36
    assert len(contrast['pngs']) == expected_contrast
    for name, item in contrast['pngs'].items():
        png_from_download(original, target, f'material-web-contrast-{name}.png', item)

    search = receipt['search']
    assert len(search['pngs']) == 46
    for name, item in search['pngs'].items():
        png_from_download(original, target, f'{SEARCH_PREFIX}{name}.png', item)
    raw_members = reports | {f'material-web-contrast-{name}.png' for name in contrast['pngs']}
    raw_members |= {f'{SEARCH_PREFIX}{name}.png' for name in search['pngs']}
    verify_archive(path, receipt, target, raw_members)
    assert validate_contrast(target, contrast['browser'], contrast['adapter']) == contrast
    search_again = validate_search(target, search['browser'], search['adapter'])
    assert search_again == {**search, 'pngs': {name: {key: value for key, value in item.items() if key != 'rgba_sha256'}
                                              for name, item in search['pngs'].items()}}

    pixel = (3, 496)
    arrow_samples = {}
    for suffix, record in (('normal-clear', 'normal_canvas_sample'),
                           ('native-arrow-forced', 'forced_canvas_sample'),
                           ('delayed-select-arrow-forced', 'forced_canvas_sample')):
        png = target / f'material-web-contrast-selection-browser-{suffix}.png'
        rgb = list(png_rows(png, {pixel[1]})[pixel[1]][pixel[0]])
        test = 'delayed_select_redraw' if suffix.startswith('delayed') else 'native_arrow_redraw'
        assert rgb == contrast['browser_arrows'][test][record]
        arrow_samples[png.name] = rgb
    write(target / 'independent-png-checks.json', {
        'search_full_png_decode_count': len(search_again['pngs']),
        'search_caret_forced_palette_and_motion_pixels_rechecked': True,
        'm247_recomputed_from_four_stock_pngs': True,
        'm246_held_arrow_physical_sample_xy': list(pixel),
        'm246_held_arrow_png_rgb': arrow_samples,
    })


def readme(proof):
    head, tree, base = (proof[key] for key in ('head', 'tree', 'base'))
    lines = [
        '# Surface v0.1.43 hosted Search verification', '',
        f'[PR #{proof["pr"]}](https://github.com/faberline/surface/pull/{proof["pr"]}) tested head `{head}`',
        f'with tree `{tree}` and `main` base `{base}`.', '',
        'The first candidate stopped at a test assertion that required a non-fallback WebGPU adapter.',
        'The updated Search test requires Chrome to report whether its adapter is a fallback.',
        'The checkout trees differ in this one E2E test file. Search and rendering code are identical.', '',
        'Both events passed the build and behavior job and the browser job. The build job checked',
        'formatting, lint, workspace tests, WebAssembly, and the consumer contract. Each browser job',
        'ran all eight stock cases on Linux under the Xvfb virtual display. Chrome used SwiftShader',
        'for software WebGPU. Native Linux window tests ran inside the virtual display.',
        'Each original GitHub artifact ZIP matched GitHub’s digest. These raw reports and PNGs match',
        'their ZIP members byte for byte.', '',
        '| Event | Workflow | Build job | Browser job | Checked-out commit | Checks | Material | IME |',
        '| --- | --- | --- | --- | --- | ---: | ---: | ---: |',
    ]
    for event in EVENTS:
        run = proof['runs'][event]
        build, browser = (run['jobs'][name] for name in ('build-and-behavior', 'browser-e2e'))
        lines.append(f'| `{event}` | [{run["run"]}]({run["url"]}) | [{build["id"]}]({build["url"]}) | '
                     f'[{browser["id"]}]({browser["url"]}) | `{run["checkout_commit"]}` | '
                     f'{run["total_checks"]} | {run["cases"]["material"]["checks"]} | {run["cases"]["ime"]["checks"]} |')
    lines.extend((
        '', 'For each event, both jobs checked out the same commit. The PR workflow tested GitHub’s merge',
        'of the base and head listed above. Both events used the candidate source tree.', '',
        'The Material suite ran 254 steps through M255. It excludes the M23 failure-path step.',
        'M248–M255 covered both Search styles, the editor across the adaptive breakpoint, accessibility',
        'and keyboard controls, input at 200% in dark right-to-left layout, the browser forced-color',
        'palette, motion, and Reduce Motion. Each event includes all 46 original Search PNGs. The',
        'verifier decoded each image and checked the caret, forced-color and motion pixels directly.', '',
        'Both I11 reports show that Select All survives a redraw after cancellation. They also show that',
        'cancellation restores a backward range and that trusted Right plus insertion yields `台文X`',
        'before repaint and stays correct after repaint. The stock fixture waits for the rendered 1001 px',
        'viewport in the Select All case. Its auxiliary frame counter counts wrapped callbacks. The',
        'fixture restores the original requestAnimationFrame at release. I20 checks UTF-16 and UTF-8',
        'selection ranges.', '',
        'M246 preserves trusted held-arrow selection through a palette change, including a delayed',
        'selection event. M247 verifies the selected and unselected parts of an input method preedit',
        'underline with Chrome’s resolved forced-color pair. The Showcase and Material reports had',
        'zero page errors. The browser input method and preference tests use browser emulation.', '',
        '## Reproduce', '',
        'Check out the exact commit in the table. Use the included `ci.yml` for dependencies and Chrome',
        'flags. Run the same command in Linux:', '',
        '```sh', 'xvfb-run -a -s "-screen 0 1920x1080x24" cargo xtask web e2e', '```', '',
        'Download the original reports and logs while GitHub keeps them:', '', '```sh',
    ))
    for event in EVENTS:
        run = proof['runs'][event]
        lines.append(f'gh run download {run["run"]} --repo faberline/surface --name surface-e2e-artifacts --dir raw-{event}')
        for name in ('build-and-behavior', 'browser-e2e'):
            lines.append(f'gh api repos/faberline/surface/actions/jobs/{run["jobs"][name]["id"]}/logs > {event}-{name}.log')
    pull = proof['runs']['pull_request']
    push = proof['runs']['push']
    lines.extend((
        '```', '', 'Run `shasum -a 256 -c SHA256SUMS` from this folder to verify the archive.', '',
        'To repeat the receipt and pixel checks, use Python 3 with Pillow and run:', '', '```sh',
        f'python3 -B verification/observe-hosted.py --pr {proof["pr"]} --head {head} --tree {tree} --base {base} '
        f'--pull-run {pull["run"]} --browser-job {pull["jobs"]["browser-e2e"]["id"]} '
        f'--push-run {push["run"]} --push-browser-job {push["jobs"]["browser-e2e"]["id"]}',
        f'python3 -B verification/package-evidence.py {pull["run"]}', '```', '',
    ))
    return '\n'.join(lines)


def package(directory):
    proof_path = directory / 'verified' / 'hosted-verified.json'
    proof = read(proof_path)
    assert set(proof['runs']) == set(EVENTS)
    source = WORKFLOW.with_name('candidate-source.json')
    delta = WORKFLOW.with_name('candidate-delta.json')
    tree_delta = read(delta)
    assert tree_delta['corrected_tree'] == proof['tree']
    assert read(source) == {'sha': proof['head'], 'tree': proof['tree'], 'parents': [tree_delta['prior_head']]}
    merge_checkout = WORKFLOW.with_name('pull-request-checkout.json')
    assert read(merge_checkout) == {
        'sha': proof['runs']['pull_request']['checkout_commit'], 'tree': proof['tree'],
        'parents': [proof['base'], proof['head']],
    }
    with TemporaryDirectory(dir=directory) as work:
        target = Path(work) / 'hosted'
        target.mkdir()
        for event in EVENTS:
            stage_event(event, proof['runs'][event], directory, target / event)
        copy2(proof_path, target / proof_path.name)
        copy2(WORKFLOW, target / 'ci.yml')
        copy2(source, target / source.name)
        copy2(merge_checkout, target / merge_checkout.name)
        copy2(delta, target / delta.name)
        tools = target / 'verification'
        tools.mkdir()
        for name in ('observe-hosted.py', 'package-evidence.py', 'stock_contrast_report.py',
                     'verify_search_report.py', 'verify_contrast_report.py'):
            copy2(ROOT / name, tools / name)
        (target / 'README.md').write_text(readme(proof))
        files = sorted(path for path in target.rglob('*') if path.is_file())
        (target / 'SHA256SUMS').write_text(''.join(f'{digest(path)}  {path.relative_to(target)}\n' for path in files))
        destination = directory / 'hosted'
        assert not destination.exists(), destination
        target.rename(destination)
    print(f'Packaged {len(files)} checked files and 92 fully decoded Search PNGs at {destination}')


if __name__ == '__main__':
    package(ROOT / f'run-{int(sys.argv[1])}')
