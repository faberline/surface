# Surface v0.1.75 stock browser replay

The replay runs the unmodified Material suite in macOS Chrome. It checks the
exact v0.1.75 source tree, executable and Gallery files before and after the
run. All 371 ordered steps must pass. This includes the baseline command menu
at M267–M269 and the vertical command menu at M370–M372.

The launcher accepts one Chrome process with `--headless=new` and a private
browser profile. The observer checks the owned process family for visible
windows every 50 ms. Success requires no visible owned window and no surviving
owned process.

The macOS observer and its dependencies came from the [v0.1.74 GitHub evidence
tree](https://github.com/faberline/surface/tree/86c12c6decab921ed16087c31ea8a52f9f49141f/docs/evidence/md3/adaptive-navigation-scaffold-v0174-candidate/stock-macos/tools).
The stock-only entry point was prepared for the [recovered v0.1.75
source](https://github.com/faberline/surface/tree/4b7a41753fc00b996b3a1b70885e272876cb77ca/docs/evidence/md3/vertical-command-menu-v0175-recovered-source).

## Run

Use Python 3.11 or later and the verified build outputs from the recovered
source. The controller supplies an absolute browser runner path and a web
root that contains the built `material-gallery/` directory. Make a SHA-256
manifest of every file in that gallery, with paths relative to the gallery.

```sh
source_bundle=/tmp/surface-md3-menu-v0175-recovered.uSvgC4AK/exact-candidate
replay_dir=/tmp/surface-md3-menu-validation-recovery.afLERMJj/browser
runner_sha=$(shasum -a 256 "$runner" | cut -d ' ' -f 1)
web_sha=$(shasum -a 256 "$web_manifest" | cut -d ' ' -f 1)

python3 -B "$replay_dir/run-stock-material.py" --run \
    --source "$source_bundle/source" --source-manifest "$source_bundle/source.sha256" \
    --runner "$runner" --runner-sha256 "$runner_sha" \
    --web-root "$web_root" --web-manifest "$web_manifest" --web-sha256 "$web_sha" \
    --out "$fresh_output"
```

Use a new directory under an existing private directory in `/tmp` as the
output. If the host stores Chrome outside the default system application
path, pass its absolute executable path with `--chrome`.

Replace `--run` with `--check-inputs` to validate pins offline. Replace it with
`--verify` to recheck a completed run offline. To exercise the built-in
launch guard tests without starting a browser, use
`python3 -B observe-focused-original.py --self-test`.

## Evidence

- `runner.log` contains the exact step list and case summary.
- `invocation.json` and `inputs-after.json` contain the build input hashes.
- `headless-window-observation.json` records launch arguments, samples,
  visible window counts and remaining processes.
- `verified-stock-material.json` records the pass count, both WebGPU adapter
  results and checksums for the two menu reports and six actual screenshots.
- `artifacts/` contains the original screenshots and menu reports.

Offline guard checks passed. Real Chrome evidence begins with a new guarded
run.
