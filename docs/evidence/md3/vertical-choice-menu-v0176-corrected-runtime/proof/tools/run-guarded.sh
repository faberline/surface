#!/bin/sh
set -eu
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
source=/tmp/surface-md3-choice-menu-v0176-corrected.2R544r7F/source
native="$lane/native-tools"
source_sha=a9aa38ee7ab64746471491ccfd6cd7df0132e8f30254d536a6fdd3fc95e10eb8
py=${SURFACE_REPLAY_PYTHON:-$HOME/.pyenv/versions/3.12.11/bin/python3}
test "$#" -eq 3 || test "$#" -eq 4
case "$2" in
    --check-inputs) test "$#" -eq 3 ;;
    --run|--verify) test "$#" -eq 4 ;;
    *) exit 2 ;;
esac
sh "$lane/build-pinned.sh" --verify "$3"
build=$(CDPATH='' cd -- "$3" && pwd -P)
sha() { shasum -a 256 "$1" | cut -d ' ' -f 1; }

case "$1" in
    native)
        (cd "$native" && shasum -a 256 --status -c SHA256SUMS)
        binary_sha=$(sha "$build/native/SHA256SUMS")
        if [ "$2" = --verify ]; then
            exec "$py" -E -B "$native/replay-vertical-choice-menu.py" --readback "$4" \
                --evidence-sha "$(sha "$4/SHA256SUMS")" --source-sha "$source_sha" --binary-sha "$binary_sha"
        fi
        set -- "$2" --source "$source" --source-manifest "$lane/source.sha256" \
            --source-sha "$source_sha" --binary-manifest "$build/native/SHA256SUMS" \
            --binary-sha "$binary_sha" ${4:+--out "$4"}
        exec "$py" -E -B "$native/replay-vertical-choice-menu.py" "$@"
        ;;
    browser)
        set -- "$2" --source "$source" --source-manifest "$lane/source.sha256" \
            --runner "$build/runners/surface-e2e" --runner-sha256 "$(sha "$build/runners/surface-e2e")" \
            --web-root "$build/web" --web-manifest "$build/web/SHA256SUMS" \
            --web-sha256 "$(sha "$build/web/SHA256SUMS")" ${4:+--out "$4"}
        exec "$py" -E -B "$lane/run-stock-material.py" "$@"
        ;;
    *) exit 2 ;;
esac
