# Provisional Surface v0.1.69 Linux evidence

This reader accepts only the two original attempt-1 runs for PR 245. It uses authenticated GitHub reads. It starts no workload and writes nothing to GitHub. The current preflight passed while both runs were active. Linux acceptance is pending.

The reader pins the reviewed source, its sole parent, GitHub's synthetic merge and tree, both run IDs and all four job IDs. It requires successful original jobs, all eight browser suites, their actual counts, Material M1 through M353 except retired M23, all 20 IME steps, and three successful Wasm job steps. It retains the original artifact ZIP, original logs and their hashes. The offline readback compares selected files with the original ZIP.

The sealed macOS verifier supplies the placeholder pixel rules. The only decoder change requires Google SwiftShader instead of Apple Metal. It directly reads the 58 original Linux PNGs and the typed M351–M353 report. Its geometry rules are strict. If the actual Linux data differs, stop and investigate. The macOS calibration decoded 58 images; it is not Linux acceptance.

Use Python 3.12 and Pillow. Preflight performs reads only. Wait for both original runs to finish before collection. Use fresh paths.

```sh
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
lane=/tmp/surface-md3-v0169-linux.Nv19CU9p
candidate=/tmp/surface-md3-v0169-runtime-publish.Prvup84G/portable
"$py" -E -B "$lane/verify-hosted.py" --preflight --candidate "$candidate"
"$py" -E -B "$lane/verify-hosted.py" --collect "$lane/originals"
"$py" -E -B "$lane/verify-hosted.py" --export "$lane/portable" --originals "$lane/originals" --candidate "$candidate"
"$py" -E -B "$lane/portable/verify-hosted.py" --verify-package "$lane/portable" --package-sha REPLACE_WITH_REVIEWED_MANIFEST_SHA
```

The generated portable package and its real Linux checks have not yet been tested. Collection preserves failure logs when available and then fails. Do not interpret an incomplete or failed run as passing evidence.
