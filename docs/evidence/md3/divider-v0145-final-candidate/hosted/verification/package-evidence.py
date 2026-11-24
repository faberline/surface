"""Preserve the exact v0.1.43 Linux stock evidence for each workflow event."""

from hashlib import sha256
import json
from pathlib import Path
import re
from shutil import copy2, copytree
import sys
from tempfile import TemporaryDirectory
from zipfile import ZipFile

from PIL import Image

from importlib import import_module

from verify_dialog_report import DIALOG_STEPS, validate as validate_dialog
from verify_divider_report import STEPS as DIVIDER_STEPS, validate as validate_divider

observe = import_module("observe-hosted")

ROOT = Path(__file__).resolve().parent
WORKFLOW = ROOT / 'ci.yml'
if not WORKFLOW.is_file():
    WORKFLOW = ROOT.parent / 'ci.yml'
EVENTS = ('pull_request', 'push')
MATERIAL = r'\[M\d+\]'
DIVIDER_VALIDATOR_SHA256 = '54091473417fec52c7c9930a01a8acf4d1301a3e55cef36746f04855332b208d'
VALIDATOR_SHA256 = '3a6bcafa050dbfe67fe3ccfe4b8eee205dff81556f5ee0fbe917c03ec20949f4'
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
    assert len([line for line in rows if re.search(MATERIAL, line)]) == 265
    assert any(line.endswith(f"{receipt['total_checks']} checks;") for line in rows)
    target.write_text('\n'.join(rows) + '\n')
    checked, total = observe.check_log(target.read_text())
    assert checked == receipt['cases'] and total == receipt['total_checks']


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


def normalized(value):
    return json.loads(json.dumps(value))


def verify_png(root, filename, item):
    path = root / filename
    assert digest(path) == item['sha256'], filename
    with Image.open(path) as image:
        assert image.format == 'PNG' and list(image.size) == item['dimensions']
        image.load()
        assert sha256(image.convert('RGBA').tobytes()).hexdigest() == item['decoded_rgba_sha256']


def verified_raw(source, target, receipt, artifact_names):
    archive = source / 'surface-e2e-artifacts.zip'
    metadata = receipt['artifact']
    archive_digest = digest(archive)
    assert archive_digest == metadata['digest'].removeprefix('sha256:')
    assert archive.stat().st_size == metadata['original_zip_size']
    with ZipFile(archive) as original:
        members = {}
        for member in original.namelist():
            members.setdefault(Path(member).name, []).append(member)
        for filename in sorted(artifact_names):
            member, = members[filename]
            extracted = artifact_file(source / 'download', filename)
            assert extracted.read_bytes() == original.read(member), filename
            copy2(extracted, target / filename)
    assert metadata['browser_job_attempt'] == receipt['jobs']['browser-e2e']['attempt']
    write(target / 'verified-original-artifact.json', {
        'id':metadata['id'], 'sha256':archive_digest, 'bytes':archive.stat().st_size,
        'browser_workflow_attempt': metadata['browser_job_attempt'], 'created_at': metadata['created_at'],
        'raw_members_matched_byte_for_byte':sorted(artifact_names),
    })


def verify_scoped_files(target, receipt):
    assert digest(ROOT / 'verify_dialog_report.py') == receipt['dialog_validator_sha256'] == VALIDATOR_SHA256
    dialog = receipt['dialog']
    excerpt = (target / 'browser-e2e-excerpt.log').read_text()
    counts, total = observe.check_log(excerpt)
    assert counts == receipt['cases'] and total == receipt['total_checks']
    stock_log = observe.stock_log(excerpt)
    painted = normalized(validate_dialog(target, dialog['browser'], dialog['adapter'], log=stock_log))
    assert painted['steps'] == list(DIALOG_STEPS) and len(painted['pngs']) == 41
    assert painted == {**dialog, 'pngs':{key:{k:v for k,v in value.items() if k != 'decoded_rgba_sha256'}
                                        for key,value in dialog['pngs'].items()}}
    for frame, item in dialog['pngs'].items():
        verify_png(target, f'{observe.DIALOG_PREFIX}{frame}.png', item)
    assert digest(ROOT / 'verify_divider_report.py') == receipt['divider_validator_sha256'] == DIVIDER_VALIDATOR_SHA256
    divider = receipt['divider']
    verified = normalized(validate_divider(target, divider['browser'], divider['adapter'], log=stock_log))
    assert verified['steps'] == list(DIVIDER_STEPS) and len(verified['profiles']) == 4
    assert verified == {**divider, 'profiles':{key:{k:v for k,v in value.items() if k not in ('dimensions','decoded_rgba_sha256')}
                                             for key,value in divider['profiles'].items()}}
    for item in divider['profiles'].values():
        verify_png(target, item['file'], item)
    assert observe.verify_cancellations(target) == receipt['ime_cancellation']
    assert observe.verify_line_endings(target) == receipt['ime_line_endings']
    for group in receipt['page_errors'].values():
        for filename, errors in group.items():
            assert read(target / filename)['errors'] == [] and errors == 0
    smoke = read(target / 'browser-smoke.json')
    side = read(target / 'browser-material-side-sheet.json')
    assert {key: smoke['adapter'][key] for key in observe.ADAPTER} == side['adapter'] == dialog['adapter'] == observe.ADAPTER
    assert smoke['browser'] == side['browser'] == dialog['browser']


def stage_event(event, receipt, directory, target):
    target.mkdir()
    source = directory / event
    per_event = source / 'verified' / 'run-verified.json'
    assert read(per_event) == receipt
    copy2(per_event, target / per_event.name)
    browser_excerpt(source / f'{event}-browser-e2e.log', target / 'browser-e2e-excerpt.log', receipt)
    build = build_excerpt(source / f'{event}-build-and-behavior.log', target / 'build-and-behavior-excerpt.log', receipt)
    status = read(source / 'status.json')
    run = status['run']
    assert run['status'] == 'completed' and run['conclusion'] == 'success'
    assert run['head_sha'] == receipt['head'] and run['id'] == receipt['run'] and run['run_attempt'] == receipt['attempt']
    for name, job in status['jobs'].items():
        assert job['status'] == 'completed' and job['conclusion'] == 'success'
        assert (job['id'], job['run_attempt']) == (receipt['jobs'][name]['id'], receipt['jobs'][name]['attempt'])
        metadata = {key:job[key] for key in ('id','name','run_id','run_attempt','head_sha','html_url',
                                             'status','conclusion','started_at','completed_at')}
        metadata['steps'] = [{key:step[key] for key in ('number','name','status','conclusion')}
                             for step in job['steps']]
        write(target / f'{name}-job.json', metadata)
    build['reported_job_status'] = status['jobs']['build-and-behavior']['conclusion']
    build['workflow_attempt'] = status['jobs']['build-and-behavior']['run_attempt']
    write(target / 'build-and-behavior-receipt.json', build)
    if carried := receipt.get('carried_forward_build'):
        assert carried == run['carried_forward_build']
        assert carried['original_job'] == receipt['jobs']['build-and-behavior']['id']
        assert carried['original_attempt'] == build['workflow_attempt'] == 1
        assert carried['carried_forward_attempt'] == receipt['attempt'] == 2
        assert carried['completed_at'] < status['jobs']['browser-e2e']['started_at']
        assert carried['same_runner_and_all_step_timestamps'] is True
        write(target / 'carried-forward-build.json', carried)
    pngs = {f'{observe.DIALOG_PREFIX}{frame}.png' for frame in receipt['dialog']['pngs']}
    assert len(pngs) == 41
    divider_pngs = {item['file'] for item in receipt['divider']['profiles'].values()}
    assert len(divider_pngs) == 4
    originals = {observe.LINE_ENDINGS, observe.CANCEL_REDRAW, observe.CANCEL_SELECTED} | pngs | divider_pngs
    for reports in receipt['page_errors'].values():
        originals |= set(reports)
    verified_raw(source,target,receipt,originals)
    verify_scoped_files(target,receipt)


def readme(proof):
    head, tree, base = (proof[key] for key in ('head','tree','base'))
    text = [
        '# Surface v0.1.45 hosted Divider verification', '',
        f'[PR #{proof["pr"]}](https://github.com/faberline/surface/pull/{proof["pr"]}) tested head `{head}`',
        f'with source tree `{tree}` and `main` base `{base}`.', '',
        'All 583 files in the source manifest match the GitHub source tree. The candidate follows the',
        'released Dialog tree. The correction changes only the Divider browser test. Its endpoint',
        'probes use the line interior to avoid the two-pixel corner fringe on SwiftShader. The test',
        'still checks exact ink, full thickness, and the immediately adjacent pixels.',
        '[Original corner failures and pixel evidence](https://github.com/faberline/surface/tree/12d72e278ff4b6af1cb2c69941263fee645edf69/docs/evidence/md3/divider-v0145-rebased-candidate/hosted-corner-failure).', '',
        'Both events passed the build and behavior job and the browser job. The build job checked',
        'formatting, lint, workspace tests, WebAssembly and the consumer contract. The browser job',
        'ran all eight stock suites on Linux in the Xvfb virtual display. Chrome reported SwiftShader,',
        'a software WebGPU adapter. Native Linux window tests ran inside the virtual display.', '',
        '| Event | Workflow | Build job and attempt | Browser job and attempt | Checkout | All checks | Material checks | IME checks |',
        '| --- | --- | --- | --- | --- | ---: | ---: | ---: |',
    ]
    for event in EVENTS:
        run = proof['runs'][event]
        build, browser = (run['jobs'][name] for name in ('build-and-behavior','browser-e2e'))
        text.append(f'| `{event}` | [{run["run"]}]({run["url"]}) | [{build["id"]}]({build["url"]}) ({build["attempt"]}) | '
                    f'[{browser["id"]}]({browser["url"]}) ({browser["attempt"]}) | `{run["checkout_commit"]}` | '
                    f'{run["total_checks"]} | {run["cases"]["material"]["checks"]} | {run["cases"]["ime"]["checks"]} |')
    pull = proof['runs']['pull_request']
    if pull['attempt'] > 1:
        text.extend(('', 'The first PR browser job stopped before tests when the Binaryen download returned HTTP 504.',
                     'The browser-only retry used the same PR checkout. The retained build passed on attempt 1.',
                     'See the [first attempt record](pr-bootstrap-failure/README.md).'))
    text.extend((
        '', 'For each event, both jobs checked out the same commit. The PR job used GitHub’s merge',
        'checkout of the named base and head. Both events used the exact candidate tree.', '',
        'The Material suite ran 265 steps through M266 and omits the M23 failure-path check.',
        'Divider M264–M266 checked the full-width, inset, middle and vertical separators, their',
        'one- and eight-dp thickness, RTL placement at 200% text size, and keyboard focus.',
        'Each job produced four Divider PNGs. The portable validator checked the physical line',
        'pixels, including the row or column immediately outside each line. The archive also retains',
        'all 41 Dialog PNGs per job and repeats the prior Dialog validator on the exact current runs.',
        'The original GitHub ZIPs match their posted checksums. Each archived raw report and PNG',
        'matches its original ZIP member byte for byte. The verifier fully decoded every PNG.', '',
        'Both I11 reports confirm that trusted Select All survives a resize after cancellation. They',
        'also confirm that a backward range recovers on cancel and trusted Right plus insertion yields',
        '`台文X` before and after repaint. The Showcase and Material reports had zero page errors.',
        'The accessibility checks use Chrome’s accessibility tree. The input-method steps use browser',
        'composition events.', '',
        '## Reproduce', '',
        'Check out the exact commit in the table and use the dependencies and Chrome flags in `ci.yml`.', '',
        '```sh', 'xvfb-run -a -s "-screen 0 1920x1080x24" cargo xtask web e2e', '```', '',
        'To verify the copied files and independently replay the receipt and pixel checks, use Python',
        '3.12 with Pillow 12.3, then run:', '', '```sh',
        'shasum -a 256 -c SHA256SUMS', 'python3 -B verification/package-evidence.py --verify .', '```', '',
        'Download the original ZIPs and logs while GitHub keeps them:', '', '```sh',
    ))
    for event in EVENTS:
        run = proof['runs'][event]
        text.append(f'gh api repos/faberline/surface/actions/artifacts/{run["artifact"]["id"]}/zip > {event}-artifact.zip')
        for name in ('build-and-behavior','browser-e2e'):
            text.append(f'gh api repos/faberline/surface/actions/jobs/{run["jobs"][name]["id"]}/logs > {event}-{name}.log')
    return '\n'.join([*text,'```',''])


def package(directory):
    proof_path = directory / 'verified' / 'hosted-verified.json'
    proof = read(proof_path)
    assert set(proof['runs']) == set(EVENTS)
    candidate = WORKFLOW.with_name('candidate-source.json')
    merge = WORKFLOW.with_name('pull-request-checkout.json')
    prior = WORKFLOW.with_name('base-source.json')
    prior_candidate = WORKFLOW.with_name('prior-candidate-source.json')
    source_manifest = WORKFLOW.with_name('candidate-source.sha256')
    source_content = WORKFLOW.with_name('source-content-verified.json')
    source_delta = WORKFLOW.with_name('source-delta-verified.json')
    first = read(prior)
    assert first['sha'] == proof['base'] and first['tree'] == '50c3a5646a033b04280f7050748ca92c42154ad5'
    original = read(prior_candidate)
    assert original['parents'] == [first['sha']]
    assert read(source_delta) == {'original_tree':original['tree'],'candidate_tree':proof['tree'],
                                  'changed_paths':['e2e/src/material_divider.rs']}
    assert read(candidate) == {'sha':proof['head'], 'tree':proof['tree'], 'parents':[original['sha']]}
    assert digest(source_manifest) == 'a2750ba3be68e0b6bc5b6f0183d4d21acf7e90fe9c4a65e623ecacbd192dd0aa'
    assert len(source_manifest.read_text().splitlines()) == 583
    assert read(source_content) == {'source_git_head':proof['head'],'candidate_tree':proof['tree'],
                                    'source_manifest_sha256':digest(source_manifest),'tracked_blobs_checked':583,
                                    'all_source_sha256_and_git_blob_ids_match':True}
    assert read(merge) == {'sha':proof['runs']['pull_request']['checkout_commit'], 'tree':proof['tree'],
                           'parents':[proof['base'],proof['head']]}
    with TemporaryDirectory(dir=directory) as work:
        target = Path(work) / 'hosted'
        target.mkdir()
        for event in EVENTS:
            stage_event(event, proof['runs'][event], directory, target / event)
        for source in (proof_path,WORKFLOW,candidate,prior,prior_candidate,merge,source_manifest,source_content,source_delta):
            copy2(source,target / source.name)
        if proof['runs']['pull_request']['attempt'] > 1:
            failed = ROOT / 'pr-bootstrap-failure'
            assert read(failed / 'bootstrap-failure.json')['head'] == proof['head']
            copytree(failed,target / 'pr-bootstrap-failure')
        tools = target / 'verification'
        tools.mkdir()
        for name in ('observe-hosted.py','package-evidence.py','verify_dialog_report.py','verify_divider_report.py'):
            copy2(ROOT / name, tools / name)
        (target / 'README.md').write_text(readme(proof))
        files = sorted(path for path in target.rglob('*') if path.is_file())
        (target / 'SHA256SUMS').write_text(''.join(f'{digest(path)}  {path.relative_to(target)}\n' for path in files))
        destination = directory / 'hosted'
        assert not destination.exists(), destination
        target.rename(destination)
    print(f'Packaged {len(files)} files, two original ZIP receipts, eight decoded Divider PNGs and 82 decoded Dialog PNGs at {destination}')


def verify_only(directory):
    proof = read(directory / 'hosted-verified.json')
    for event in EVENTS:
        receipt = read(directory / event / 'run-verified.json')
        assert receipt == proof['runs'][event]
        verify_scoped_files(directory / event, receipt)
        if carried := receipt.get('carried_forward_build'):
            assert read(directory / event / 'carried-forward-build.json') == carried
            builder = read(directory / event / 'build-and-behavior-job.json')
            browser = read(directory / event / 'browser-e2e-job.json')
            assert builder['id'] == carried['original_job'] and builder['run_attempt'] == carried['original_attempt']
            assert browser['run_attempt'] == carried['carried_forward_attempt'] == receipt['attempt']
            assert builder['completed_at'] == carried['completed_at'] < browser['started_at']
    print('Verified both Divider and Dialog receipts and pixel checks from the sealed files.')


if __name__ == '__main__':
    if sys.argv[1] == '--verify':
        verify_only(Path(sys.argv[2]).resolve())
    else:
        package(ROOT / f'run-{int(sys.argv[1])}')
