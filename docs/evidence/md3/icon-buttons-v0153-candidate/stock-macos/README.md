# Surface v0.1.53 Icon Button headless verification

The runner uses explicit smoke, ime, material, bridge-web, and
devtools-headless cases. The Material journey keeps the preceding
coverage and adds four Icon Button steps through M297.

The guard admits Chrome only with `--headless=new` and the native
Gallery only with `--headless`. It records private Chrome profiles,
owned processes, and physical window inspections. It requires zero
visible windows and zero survivors. The stock verification keeps the
previous Chips PNGs and typed receipts. The `--wide-canvas-height 800`
and `--rtl-canvas-height 1020` flags refer to those Chips screens.

Use a fresh private directory under /tmp. Pin exact source, the native
build and all three Web packs with `prepare-pins.py --help`. Use the
two runner commands below only after the final candidate is sealed.

```sh
python3 -B observe-icon-buttons.py --self-test
python3 -B observe-icon-buttons.py --verify-inputs-only \
  --runner-sha256 "$(cat approved-runner.sha256)"
python3 -B observe-icon-buttons.py --out "$PWD/run" \
  --runner-sha256 "$(cat approved-runner.sha256)"
python3 -B verify-existing.py --run "$PWD/run"
python3 -B archive-proof.py --seal "$PWD/portable-proof"
python3 -B archive-proof.py --verify "$PWD/portable-proof"
```
