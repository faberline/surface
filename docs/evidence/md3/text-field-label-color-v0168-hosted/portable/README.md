# Surface v0.1.68 Linux evidence reader

This reader is prepared for the original attempt 1 of PR 244. It does not start a job or change GitHub. Preflight is not acceptance. Collection needs both workflows to finish. Export needs both original runs and all required jobs to pass. It retains the original authenticated ZIP and compressed log bytes and verifies their digests. The portable readback compares each selected original file with the retained ZIP.

The color decoder comes from the sealed macOS v0.1.68 receipt. It changes the backend guard to Google SwiftShader and checks the Linux pixels directly. The macOS receipt supplies calibration, not Linux acceptance. The existing v0.1.67 Linux archive supplies shell dimensions and platform calibration. The decoder rejects a different image shape. M350 must show all four actual 44 ms probes at RGB 206,191,243. Counts come from each original run. If real Linux evidence differs, stop and investigate; do not relax the reader without evidence.

Use Python 3.12 and Pillow. The candidate is the sealed macOS calibration receipt.

```
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
lane=/tmp/surface-md3-v0168-linux.dGmJQKKd/offline
candidate=/tmp/surface-md3-label-color-v0168-runtime-proof.XqBuKmCa/portable
"$py" -E -B "$lane/verify-hosted.py" --preflight --candidate "$candidate"
"$py" -E -B "$lane/verify-hosted.py" --collect "$lane/originals"
"$py" -E -B "$lane/verify-hosted.py" --export "$lane/portable" --originals "$lane/originals" --candidate "$candidate"
"$py" -E -B "$lane/portable/verify-hosted.py" --verify-package "$lane/portable" --package-sha REVIEWED_MANIFEST_SHA
```

Collect and export paths must be fresh. Collection preserves a failure log when possible and then fails. Never use a failed or unfinished run as acceptance. `controls.py` exercises calibration and rejection controls. No real v0.1.68 Linux artifact was available when this reader was prepared.
