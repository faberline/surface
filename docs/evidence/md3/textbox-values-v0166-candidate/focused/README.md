# v0.1.66 focused Chrome runner

This private copy contains 681 q66 paths. It preserves the exact authored
`e2e/src/material_multiline_text_field.rs`, including its multiline and
single-line M346 assertions. The only diagnostic source change is
`scope-selector.patch`. It limits the Material runner to M341–M346.

The runner was built with `cap cargo build --locked -p surface-e2e`, two
jobs, offline, and with dev and test debug symbols disabled. Build
outputs used the private target stated in `build-focused.sh`. The copied
runner passed its `--help` smoke check. The focused and original source
manifests define the two inputs. The four-file Web pack matches the prior
independent .66 R2 proof bytes.

The wrapper requires the controller's successful v0.1.65 stock marker.
It starts only the six-step scoped run with the pinned guard, a private
Chrome profile, and `--headless=new`. The original outputs stay in the
fresh run directory. No visible windows or process survivors are allowed.

```sh
lane=/tmp/surface-md3-textbox-values-e2e.ohBgx2tj
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
(cd "$lane" && shasum -a 256 -c SHA256SUMS)
cap "$py" -E -B "$lane/run-focused.py" \
  --runner "$lane/pins/surface-e2e" \
  --runner-sha256 3f6ffe6bb0b809c0894b2d3be113c9777fb733a3b510bd9089bf1ed8b3e585cf \
  --web-root "$lane/web" --out "$lane/development-r1"
"$py" -E -B "$lane/verify-focused.py" "$lane/development-r1" > "$lane/verified-focused.json"
```

The offline verifier checks native Chrome accessibility values from M346
and applies the unmodified .65 image verifier to the retained five-step
results. That verifier reads the actual twelve PNGs. The focused scope
does not represent the complete stock suite.
