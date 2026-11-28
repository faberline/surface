# Surface q73 original main CI evidence

This collector verifies one exact, original attempt-1 push to `main` for the
merged q73 pull request #249. It does not select the latest run. It requires a
reviewed config with the exact main merge commit, workflow run ID and both job
IDs.

The source candidate is `c2571a3d818ddaf0cb7874998d1c39f0ffc7b1f2`. Its exact
tree is `791d8b2dd3f9b44f3c2a967f1418b671920a2899`. The only allowed parent is
`6aa6052b6a830db2b16b481744ac451f45f007df`. The collector checks that PR #249
is merged to `main` at the configured commit, that the commit has the exact
tree and sole parent, and that the original main push and both jobs passed.

Before extracting the original `surface-e2e-artifacts` ZIP, the collector
checks its GitHub SHA-256 and byte size. It reads the original build and
browser logs. It requires all eight ordered suites, 364 ordered Material steps,
successful Rust and WebAssembly steps, and the SwiftShader browser adapter.
The portable result keeps the original GitHub receipts and logs, the two
navigation reports, three q73 PNGs, the independent PNG reader and a SHA-256
seal. It omits the full ZIP after checking its digest and size. The seal and
config SHA must be saved outside the package.

Run this only after a human has reviewed and filled the exact values in
`main-config.example.json`. The config must contain the main merge commit and
the matching original run and job IDs. Do not replace these values by querying
for the latest run. The helper requires a 64-character SHA for the reviewed
config.

Use Python 3.12 with Pillow installed. From a fresh `/tmp` directory, run:

```sh
py=/path/to/python3.12
config=/path/to/reviewed-main-config.json
config_sha=REVIEWED_CONFIG_SHA256
package=/tmp/surface-md3-v0173-main-original
"$py" -E -B hosted-main.py --collect "$package" --config "$config" --config-sha "$config_sha"
"$py" -E -B "$package/hosted-main.py" --verify "$package" --config-sha "$config_sha" --manifest-sha REVIEWED_PACKAGE_SHA256
```

The review config must be supplied from the controller's saved metadata. The
collector does not make GitHub changes, rerun jobs, build the app, or launch a
browser. It uses read-only GitHub API calls to retrieve the exact receipts,
logs and original ZIP.
