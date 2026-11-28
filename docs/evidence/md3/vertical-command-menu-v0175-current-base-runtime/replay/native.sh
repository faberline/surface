#!/bin/sh
set -eu
lane=/tmp/surface-md3-v0175-replay.71RwhtQh
build=$lane/outputs
source=/tmp/surface-md3-v0175-current-base.rWr9qUiy/source
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
test "$(shasum -a 256 "$lane/native-tools/SHA256SUMS" | cut -d ' ' -f 1)" = f9dab8e24b20b9a07dc94dd6955171f697217658fdaabe18485838827069bc5d
(cd "$lane/native-tools" && shasum -a 256 -c SHA256SUMS > /dev/null)
"$py" -E -B "$lane/source-check.py"
(cd "$build" && shasum -a 256 -c SHA256SUMS > /dev/null)
native_manifest=$build/native/SHA256SUMS
(cd "$build/native" && shasum -a 256 surface-material-gallery surface-devtools > SHA256SUMS)
native_sha=$(shasum -a 256 "$native_manifest" | cut -d ' ' -f 1)
exec "$py" -E -B "$lane/native-tools/replay-vertical-menu.py" --source "$source" \
  --source-manifest "$source/../source.sha256" \
  --source-sha 3946a01298f055f264e759e574af373c202945bfde85a002ef02b948bf25274a \
  --binary-manifest "$native_manifest" --binary-sha "$native_sha" "$@"
