# Surface v0.1.67 postmerge Linux CI evidence

This helper accepts the original attempt 1 of main push 46113171688.
Both job logs must show checkout of 6fb40370ca824b8ed44e6e17099e6156f8d15a9b.
The GitHub checkout tree must be c218faaf7527ec2872fb1ec1097c87667f3bf1f2.
The expected build job is 140881014214. The expected browser job is
140881014024.

The collector checks the original artifact against the size and SHA-256
reported by GitHub. The exporter retains the original log bytes and selected
typed reports. It independently decodes the 15 original Label Motion PNGs
against their measured report. It also checks the six preceding textbox
cases, the two IME cancellation reports, and authored line endings. Its
browser parser requires eight suites, 349 Material steps through M350 with
M23 retired, and 20 IME steps. The receipt carries actual per-suite checks and
the reported SwiftShader software adapter.

## Commands

Use Python 3.12 and Pillow. Run from this tool directory. Preflight makes one
read-only check of the original workflow and jobs.

    py=python3
    lane=$(pwd -P)
    "$py" -E -B "$lane/verify-main.py" --preflight

Once both original jobs and the workflow are successful, collect and export
to fresh directories.

    "$py" -E -B "$lane/verify-main.py" --collect "$lane/originals" > "$lane/collected.json"
    "$py" -E -B "$lane/verify-main.py" --export "$lane/portable" --originals "$lane/originals" > "$lane/exported.json"

The originals directory retains the unmodified ZIP. The portable directory
retains the ZIP digest, selected originals, manifest, and offline readers.
Use the manifest digest printed by the exporter.

    cd "$lane/portable"
    "$py" -E -B verify-main.py --verify . --sha REVIEWED_ARCHIVE_SHA

The old PR and source-branch artifacts stay in their own package. This helper
uses their already verified decoder and the published local candidate
references. Its files and commands leave GitHub unchanged.

- [Original main push](https://github.com/faberline/surface/actions/runs/46113171688/attempts/1)
- [Merged source](https://github.com/faberline/surface/commit/6fb40370ca824b8ed44e6e17099e6156f8d15a9b)
- [Local candidate evidence and reference pixels](https://github.com/faberline/surface/tree/d74ad36e0c588a4a43fb3a43f11d1ce5fe7918f4/docs/evidence/md3/text-field-label-motion-v0167-candidate)
