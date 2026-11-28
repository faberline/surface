#!/bin/sh
set -eu
lane=/tmp/surface-md3-v0172-replay.OgmvVGia
build=$lane/outputs
source=/tmp/surface-md3-v0172-full-source.OQDetDtP/source
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
test "$#" -ge 1
mode=$1
shift
case "$mode" in
  --check-inputs) test "$#" -eq 0 ;;
  --run) test "$#" -eq 1; case "$1" in /tmp/*) ;; *) exit 2 ;; esac; set -- --run "$1" ;;
  *) exit 2 ;;
esac
test -f "$build/BUILD-COMPLETE"
"$py" -E -B "$lane/source-check.py"
(cd "$build" && shasum -a 256 -c SHA256SUMS > /dev/null)
native_manifest=$build/native/SHA256SUMS
(cd "$build/native" && shasum -a 256 surface-material-gallery surface-devtools > SHA256SUMS)
native_sha=$(shasum -a 256 "$native_manifest" | cut -d ' ' -f 1)
exec "$py" -E -B "$lane/replay-field-pair.py" --source "$source" \
  --source-manifest "$source/../source.sha256" \
  --source-sha a59fd437cbd2f3cea304708ff2e95752fd70e8f1ec1fe2004a1289903d6a6f49 \
  --binary-manifest "$native_manifest" --binary-sha "$native_sha" "$@"
