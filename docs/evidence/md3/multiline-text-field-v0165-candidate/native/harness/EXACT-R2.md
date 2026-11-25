# Exact v0.1.65 R2 native replay

Both actual Apple Metal replays passed. Each replay returned seven
original PNGs. The raw Page.screenshot, stock CLI and MCP transports
returned identical bytes for each PNG. The window observer returned
zero visible windows in 24 checks. The harness reaped all four Gallery
children.

The separate offline check decoded all 14 originals with Pillow.
Their PNG bytes, decoded RGBA pixels and interaction results matched
the previous development-5 run. The exact candidate includes four
further source changes. The change to macOS accessibility source
updates only test assertions. The other changes are one coverage note
and two browser test files.

| Input or output | SHA-256 |
| --- | --- |
| Exact source manifest, 681 files | 6a420b56fc090457f9841fc8fda7c1bf0602e27138f586ca3f6b251652b580f2 |
| Native Gallery and CLI manifest | 85afe8106d75c9c71213b9b098e86e54f8095a34449e089f35bad13072e5cdce |
| Exact native capture manifest | d5fe9fccdb4214455fd0281c2cfc576dc80eb8b07cfd1d28fb54638819064fd6 |
| Verified replay report | b189efb1abf6cf5f85ae2b64df3fb98c008efff02033b0fc1ef819080002df61 |

## Offline check

~~~sh
lane=/tmp/surface-md3-multiline-native.cZ38fMkb
stage=/tmp/surface-md3-text-field-multiline-v0165.ZRjH7vPS
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
source_sha=6a420b56fc090457f9841fc8fda7c1bf0602e27138f586ca3f6b251652b580f2
binary_sha=85afe8106d75c9c71213b9b098e86e54f8095a34449e089f35bad13072e5cdce
replay_sha=d5fe9fccdb4214455fd0281c2cfc576dc80eb8b07cfd1d28fb54638819064fd6

(cd "$lane" && shasum -a 256 -c exact-proof.sha256)
"$py" -E -B "$lane/verify-stored-multiline.py" "$lane/exact-1" \
  --manifest-sha "$replay_sha" --source-manifest-sha "$source_sha" \
  --binary-manifest-sha "$binary_sha"
"$py" -E -B "$lane/compare-exact-pngs.py" "$lane/development-5" "$lane/exact-1" \
  --development-source-manifest "$stage/native-development-r2/source.sha256" \
  --exact-source-manifest "$stage/exact-candidate/source.sha256"
~~~

## Repeat the headless native run

Use an approved offscreen Metal adapter. Supply a fresh output path.

~~~sh
cap "$py" -E -B "$lane/replay-multiline-text-field.py" --repeats 2 --out "$new_output" \
  --source-root "$stage/exact-candidate/source" \
  --source-manifest "$stage/exact-candidate/source.sha256" --source-manifest-sha "$source_sha" \
  --binary-manifest "$stage/exact-candidate/binaries/SHA256SUMS" --binary-manifest-sha "$binary_sha"
~~~

The native input checks use semantic DevTools keys. This receipt
covers native drawing and keyboard behavior. It does not include
native touch, live speech or an operating-system input method.
