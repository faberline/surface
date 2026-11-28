#!/bin/sh
set -eu
lane=/tmp/surface-md3-v0171-replay.CZtAvGj9
build=$lane/outputs
source=/tmp/surface-md3-v0171-corrected.BWoiL6mk/source
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
test "$#" -ge 1
mode=$1
shift
runner=$build/stock/debug/surface-e2e
manifest=$build/SHA256SUMS
runner_sha=$(shasum -a 256 "$runner" | cut -d ' ' -f 1)
build_sha=$(shasum -a 256 "$manifest" | cut -d ' ' -f 1)
case "$mode" in
  --check-inputs) test "$#" -eq 0 ;;
  --run|--verify|--recover-receipt) test "$#" -eq 1; set -- --out "$1" ;;
  *) exit 2 ;;
esac
exec "$py" -E -B "$lane/browser.py" --source "$source" --source-manifest "$source/../source.sha256" \
  --runner "$runner" --runner-sha256 "$runner_sha" --web-root "$build/stock/web/release" \
  --web-manifest "$manifest" --web-sha256 "$build_sha" "$mode" "$@"
