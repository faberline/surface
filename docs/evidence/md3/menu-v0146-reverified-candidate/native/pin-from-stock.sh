#!/bin/sh
set -eu

batch=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
stock=$1
source_tree=$2
expected=$3
expected_files=590
test ! -e "$batch/pins"
test -s "$stock/sealed-inputs.sha256"
test "$(shasum -a 256 "$batch/source.sha256" | cut -d ' ' -f 1)" = "$expected"
test "$(wc -l < "$batch/source.sha256")" -eq "$expected_files"
(cd "$stock" && shasum -a 256 -c sealed-inputs.sha256) > "$batch/stock-seal-check.log"
(cd "$stock/.." && shasum -a 256 -c "$stock/harness.sha256") > "$batch/stock-harness-check.log"
for manifest in incoming-source.sha256 browser-source.sha256 built-input-sha256.txt; do
    cmp "$batch/source.sha256" "$stock/$manifest"
done
cp -p "$stock/browser-binary-sha256.txt" "$batch/stock-binaries.sha256"
cp -p "$stock/sealed-inputs.sha256" "$batch/stock-sealed-inputs.sha256"
(cd "$source_tree" && shasum -a 256 -c "$batch/source.sha256") > "$batch/pin-source-before.log"
(cd "$stock/bundle" && shasum -a 256 -c "$batch/stock-binaries.sha256") > "$batch/stock-binary-check.log"
mkdir "$batch/pins"
cp -cp "$stock/bundle/debug/surface-material-gallery" "$stock/bundle/debug/surface-devtools" "$batch/pins/"
awk '$2 == "debug/surface-material-gallery" || $2 == "debug/surface-devtools" {
    print $1 "  " substr($2, 7)
}' "$batch/stock-binaries.sha256" > "$batch/pins/native-binaries.sha256"
test "$(wc -l < "$batch/pins/native-binaries.sha256")" -eq 2
(cd "$batch/pins" && shasum -a 256 -c native-binaries.sha256) > "$batch/pin-native.log"
(cd "$source_tree" && shasum -a 256 -c "$batch/source.sha256") > "$batch/pin-source-after.log"
cmp "$batch/pin-source-before.log" "$batch/pin-source-after.log"
cmp "$batch/stock-binaries.sha256" "$stock/browser-binary-sha256.txt"
cmp "$batch/stock-sealed-inputs.sha256" "$stock/sealed-inputs.sha256"
