#!/bin/sh
set -eu
lane=$(CDPATH='' cd -- "$(dirname "$0")" && pwd -P)
source=/tmp/surface-md3-label-color-v0168-corrected.6WxZjn4h/surface
source_sha=086f06d98a7c2f3728b67c9524b15ed8b380fb078d42c48bb80b1f082c24580c
tools=/tmp/surface-md3-label-color-v0168-fresh-replay.AwmrhZ8K/historical/native/harness
py=${SURFACE_REPLAY_PYTHON:-$HOME/.pyenv/versions/3.12.11/bin/python3}
test "$#" -eq 2 || test "$#" -eq 3
mode="$1"
build="$2"
out="${3:-}"
case "$mode" in
    --check-inputs) test "$#" -eq 2 ;;
    --run|--verify) test "$#" -eq 3 ;;
    *) exit 2 ;;
esac
sh "$lane/build-pinned.sh" --verify "$build"
test "$(shasum -a 256 "$tools/harness.sha256" | cut -d' ' -f1)" = c842ea6ac04a2428ac239bf4ec6357d2912603f9cd5b3decd1029a9e0b1b1f35
(cd "$tools" && shasum -a 256 --status -c harness.sha256)
native_manifest="$build/native/SHA256SUMS"
native_sha=$(shasum -a 256 "$native_manifest" | cut -d' ' -f1)
if [ "$mode" != --verify ]; then
    set -- --source-root "$source" --source-manifest "$lane/source.sha256" \
        --source-manifest-sha "$source_sha" --binary-manifest "$native_manifest" --binary-manifest-sha "$native_sha"
    if [ "$mode" = --check-inputs ]; then
        exec "$py" -E -B "$tools/replay-multiline-text-field.py" "$@" --verify-inputs-only
    fi
    "$py" -E -B "$tools/replay-multiline-text-field.py" "$@" --repeats 2 --out "$out"
fi
physical=$(CDPATH='' cd -- "$out" && pwd -P)
evidence_sha=$(shasum -a 256 "$physical/SHA256SUMS" | cut -d' ' -f1)
"$py" -E -B "$tools/verify-stored-multiline.py" "$physical" --manifest-sha "$evidence_sha" \
    --source-manifest-sha "$source_sha" --binary-manifest-sha "$native_sha" --repeats 2
"$py" -E -B "/tmp/surface-md3-label-color-v0168-fresh-replay.AwmrhZ8K/check-native-parity.py" "$physical"
