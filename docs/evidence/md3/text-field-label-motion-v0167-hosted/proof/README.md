# Surface v0.1.67 hosted Label Motion evidence

This verifier accepts only the original attempt-1 runs for PR #243. It checks two GitHub events, two checkout commits, and all four job identities. The PR jobs check out the exact GitHub merge commit. The push jobs check out the exact source head. Both trees must match the reviewed candidate.

Record job and artifact data through authenticated reads. The original ZIP and logs stay in the private originals directory. The portable archive stores their digests, the API receipts, compressed original logs, measured Label Motion and preceding text-box reports, the IME reports, and 15 original Label Motion PNGs per event.

The archived macOS decoder checks timing, label and notch positions, RTL, Reduce Motion, focus and marked text using decoded pixels. The Linux verifier changes only its platform adapter condition. It requires reported Google SwiftShader, with software fallback true. Each renderer is checked against its own original PNGs and measured report. The Mac fixture supplies the expected palette and independent definition of the tested states. The verifier makes no live speech or hardware acceleration claim.

The Linux log must show all eight stock cases, IME I1 through I20, and all Material steps M1 through M350 except M23. It reads the actual check totals from the two original browser logs. Each event must finish both the build and browser jobs.

## Local use

Use Python 3.12 and Pillow. Preflight reads the current run state once.

    py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
    lane=/tmp/surface-md3-label-motion-v0167-hosted.Q6gmi6HS
    candidate=/tmp/surface-md3-label-motion-v0167-recovery.7u1VBXg2/portable
    "$py" -E -B "$lane/verify-hosted.py" --preflight --candidate "$candidate"

Once both original workflows are terminal, use fresh directories. The collector records original failure logs if a workflow failed. The exporter requires original success.

    "$py" -E -B "$lane/verify-hosted.py" --collect "$lane/originals"
    "$py" -E -B "$lane/verify-hosted.py" --export "$lane/portable" --originals "$lane/originals" --candidate "$candidate" > "$lane/exported.json"

For a portable offline replay, use the returned SHA-256 of `SHA256SUMS`.

    "$py" -E -B "$lane/portable/verify-hosted.py" --verify-package "$lane/portable" --package-sha REVIEWED_ARCHIVE_SHA

Nothing in these commands starts a workflow or publishes files.

- [Original pull request workflow](https://github.com/faberline/surface/actions/runs/46109248208/attempts/1)
- [Original push workflow](https://github.com/faberline/surface/actions/runs/46109213750/attempts/1)
- [Reviewed Label Motion reference evidence](https://github.com/faberline/surface/tree/d74ad36e0c588a4a43fb3a43f11d1ce5fe7918f4/docs/evidence/md3/text-field-label-motion-v0167-candidate)
