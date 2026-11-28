#!/bin/sh
set -eu
lane=/tmp/surface-md3-v0173-replay.CfzLYaxD
build=$lane/outputs
source=/tmp/surface-md3-v0173-resize-candidate.omUSfq1u/source
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
(cd "$lane" && shasum -a 256 -c SHA256SUMS > /dev/null)
"$py" -E -B "$lane/source-check.py"
(cd "$build" && shasum -a 256 -c SHA256SUMS > /dev/null)
native_manifest=$build/native/SHA256SUMS
(cd "$build/native" && shasum -a 256 surface-material-gallery surface-devtools > SHA256SUMS)
native_sha=$(shasum -a 256 "$native_manifest" | cut -d ' ' -f 1)
exec "$py" -E -B "$lane/replay-horizontal-nav.py" --source "$source" \
  --source-manifest "$source/../source.sha256" \
  --source-sha 513d366accbb3ef9cd39120dc0a45bd98d033e0216c56613079011d0bf8e94dd \
  --binary-manifest "$native_manifest" --binary-sha "$native_sha" "$@"
