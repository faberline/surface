#!/bin/sh
set -eu
lane=/tmp/surface-md3-v0169-replay.WfBRFHlH
build=$lane/outputs
source=/tmp/surface-md3-v0169-hint-regression.dAa3WXAs/source
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
test "$#" -ge 1
mode=$1
shift
case "$mode" in
  --check-inputs) test "$#" -eq 0; set -- --verify-inputs-only ;;
  --run) test "$#" -eq 1; set -- --out "$1" ;;
  *) exit 2 ;;
esac
test -f "$build/BUILD-COMPLETE"
"$py" -E -B "$lane/source-check.py"
(cd "$build" && shasum -a 256 -c SHA256SUMS > /dev/null)
native_manifest=$build/native/SHA256SUMS
(cd "$build/native" && shasum -a 256 surface-material-gallery surface-devtools > SHA256SUMS)
native_sha=$(shasum -a 256 "$native_manifest" | cut -d ' ' -f 1)
exec "$py" -E -B "$lane/replay-placeholder.py" --source-root "$source" \
  --source-manifest "$source/../source.sha256" \
  --source-manifest-sha 3eed0f67f0440d4cc37ba48843857e45ad165188f174ebaf2dacf62fd19c426f \
  --binary-manifest "$native_manifest" --binary-manifest-sha "$native_sha" "$@"
