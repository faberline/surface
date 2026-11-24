#!/bin/sh
set -eu

batch=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
exact_source=$1
exact_manifest=$2
exact_manifest_hash=$3
test ! -e "$batch/source"
test ! -e "$batch/pins"
test "$(shasum -a 256 "$exact_manifest" | cut -d ' ' -f 1)" = "$exact_manifest_hash"
cp "$exact_manifest" "$batch/source.sha256"
(cd "$exact_source" && shasum -a 256 -c "$batch/source.sha256") > "$batch/incoming-source.log"
cp -cR "$exact_source" "$batch/source"
cd "$batch/source"
shasum -a 256 -c ../source.sha256 > ../build-source-before.log
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export CARGO_HOME=${SURFACE_QA_CARGO_HOME:-/tmp/surface-md3-cargo}
export CARGO_TARGET_DIR=${SURFACE_QA_TARGET_DIR:-/tmp/cargo-target/surface-${batch##*/}}
export CARGO_BUILD_JOBS=2 CARGO_NET_OFFLINE=true CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 RUST_TEST_THREADS=1
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins > ../build.log 2>&1
shasum -a 256 -c ../source.sha256 > ../build-source-after.log
cmp ../build-source-before.log ../build-source-after.log
mkdir "$batch/pins"
cp "$CARGO_TARGET_DIR/debug/surface-material-gallery" "$CARGO_TARGET_DIR/debug/surface-devtools" "$batch/pins/"
cd "$batch/pins"
shasum -a 256 surface-material-gallery surface-devtools > native-binaries.sha256
