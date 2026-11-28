#!/bin/sh
set -eu
lane=/tmp/surface-md3-v0176-current-replay.qSPAab9k
build=$lane/outputs
source=/tmp/surface-md3-v0176-forward.pjxmpm8l/strict-replay
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
test "$#" -ge 1
mode=$1
shift
case "$mode" in
  --check-inputs) test "$#" -eq 0; set -- --check-inputs ;;
  --run) test "$#" -eq 1; case "$1" in /tmp/*) ;; *) exit 2 ;; esac; set -- --run --out "$1" ;;
  *) exit 2 ;;
esac
test -f "$build/BUILD-COMPLETE"
test "$(shasum -a 256 "$lane/native-tools/SHA256SUMS" | cut -d ' ' -f 1)" = cb781df3bef883568898a838a62d14bd1aa847ba8d80f5de2e9e95c1117d8266
(cd "$lane/native-tools" && shasum -a 256 -c SHA256SUMS > /dev/null)
"$py" -E -B "$lane/source-check.py"
(cd "$build" && shasum -a 256 -c SHA256SUMS > /dev/null)
native_manifest=$build/native/SHA256SUMS
(cd "$build/native" && shasum -a 256 surface-material-gallery surface-devtools > SHA256SUMS)
native_sha=$(shasum -a 256 "$native_manifest" | cut -d ' ' -f 1)
exec "$py" -E -B "$lane/native-tools/replay-vertical-choice-menu.py" --source "$source" \
  --source-manifest "$source/../source.sha256" \
  --source-sha 84bb4b9da77641b89a990c703bf59789f7f14657cfe97b60592cb3bbd9cfdb02 \
  --binary-manifest "$native_manifest" --binary-sha "$native_sha" "$@"
