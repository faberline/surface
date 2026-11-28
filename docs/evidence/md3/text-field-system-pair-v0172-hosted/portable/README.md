# Surface v0.1.72 hosted PR proof

This kit is pinned to PR #248, head `e99b6c060304ad3de74237d9cdc5e152cb8237d8`, base `a64f60b796b7bffcd376a99aac903b5235dee2b7`, and tree `8673365398b4c7b51f5e7409abe280d79b79d852`.

The exact original push run is `46149096116`, with build job `140991495559` and browser job `140991495838`. The exact original pull request run is `46149103037`, with build job `140991514326` and browser job `140991514040`. The pull request merge checkout is `c946335980ccf66d11ac3cb56a9087f9062df3fa`.

The runs were in progress when this kit was prepared. Collection stops until both original runs and all four original jobs pass. It does not retry or start another run.

Use Python 3.12 or later with Pillow 12.3.0 installed. `hosted.py` checks the original run and job IDs, commit lineage, build log, browser suite order and counts, artifact digest, system Field report, FieldText selection screenshots, and disabled label contrast screenshots. The PNG reader checks the selected text against the actual Field and selected-text captures.

```sh
kit=/tmp/surface-md3-v0172-hosted-pr.WkagIyvx
python3 -E -B "$kit/hosted.py" --preflight
python3 -E -B "$kit/hosted.py" --collect "$kit/originals"
python3 -E -B "$kit/hosted.py" --export "$kit/portable" --originals "$kit/originals"
```

Review the archive SHA printed by export. Then run the portable checker offline:

```sh
python3 -E -B "$kit/portable/hosted.py" --verify-package "$kit/portable" --package-sha REVIEWED_SHA256
```

The portable package contains the original GitHub receipts, logs, artifact ZIP, 14 system Field/FieldText PNGs, 11 selection PNGs, 15 disabled label contrast PNGs, their typed reports, and offline verifiers. It checks 361 ordered Material steps and reads final counts from the original logs.

`main/bind-main.py` and `main/hosted-main.py` support a separate main run after merge. The binder requires PR #248 to be merged, main to equal the exact squash commit, the expected tree and sole parent, and exactly one original main push workflow. It does not guess a future main commit or workflow ID.

```sh
cd "$kit/main"
python3 -E -B bind-main.py --check-source
python3 -E -B bind-main.py --bind /tmp/surface-md3-v0172-main-binding
```
