#!/bin/sh
set -eu
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
r5=/tmp/surface-md3-choice-menu-v0176-browser-corner.3ixT0mHe
web=/tmp/surface-md3-choice-menu-v0176-corrected-replay.Q1k95t8V/outputs/web
py=${SURFACE_REPLAY_PYTHON:-$HOME/.pyenv/versions/3.12.11/bin/python3}
case "${1:-}" in
    --check-inputs) test "$#" -eq 1 ;;
    --run|--verify) test "$#" -eq 2 ;;
    *) exit 2 ;;
esac
mode="$1"
out="${2:-}"
runner="$r5/runner/surface-e2e"
runner_sha=$(shasum -a 256 "$runner" | cut -d ' ' -f 1)
web_sha=3c471440987193713a55394e334f0b8a5078da7bad8dc98016c9dca1985d9ec4
set -- "$mode" --source "$r5/source" --source-manifest "$r5/source.sha256" \
    --runner "$runner" --runner-sha256 "$runner_sha" \
    --web-root "$web" --web-manifest "$web/SHA256SUMS" --web-sha256 "$web_sha"
if [ -n "$out" ]; then set -- "$@" --out "$out"; fi
exec "$py" -E -B "$lane/run-stock-material.py" "$@"
