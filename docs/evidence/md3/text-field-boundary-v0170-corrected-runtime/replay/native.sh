#!/bin/sh
set -eu
lane=/tmp/surface-md3-v0170-replay.ChhF33kX
build=$lane/outputs
source=/tmp/surface-md3-v0170-boundary-regression.xnkkMNBE/surface
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
test "$#" -ge 1
mode=$1
shift
case "$mode" in
  --check-inputs) test "$#" -eq 0; set -- --verify-inputs-only ;;
  --run) test "$#" -eq 1; set -- --run --out "$1" ;;
  *) exit 2 ;;
esac
test -f "$build/BUILD-COMPLETE"
"$py" -E -B "$lane/source-check.py"
(cd "$build" && shasum -a 256 -c SHA256SUMS > /dev/null)
native_manifest=$build/native/SHA256SUMS
(cd "$build/native" && shasum -a 256 surface-material-gallery surface-devtools > SHA256SUMS)
native_sha=$(shasum -a 256 "$native_manifest" | cut -d ' ' -f 1)
exec "$py" -E -B "$lane/replay-boundaries.py" --source-root "$source" \
  --source-manifest "$source/../source.sha256" \
  --source-manifest-sha 06b2b09c23fb09cc0b28c56f27b1097a5abd3d962d4866b33b0b38764b4db661 \
  --binary-manifest "$native_manifest" --binary-manifest-sha "$native_sha" "$@"
