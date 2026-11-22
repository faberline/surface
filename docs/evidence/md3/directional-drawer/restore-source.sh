#!/bin/sh
set -eu

bundle=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
current_source=$(CDPATH='' cd -- "${1:?Pass the verified Navigation Current source checkout}" && pwd)
output=$(mktemp -d /tmp/surface-md3-directional-drawer-source.XXXXXXXX)
destination="$output/source"
gzip -dc "$bundle/navigation-current-reverse.patch.gz" > "$output/navigation-current.patch"
actual=$(shasum -a 256 "$output/navigation-current.patch" | awk '{print $1}')
test "$actual" = 8698f5b588ed83c44fed312baf6baf09fe71d6db9b190cb3b8c0e7d62cb9b02d
mkdir -p "$destination"
rsync -a --exclude='/.git' --exclude='/artifacts' --exclude='/target' "$current_source/" "$destination/"
(
    cd "$destination"
    git apply --reverse --check "$output/navigation-current.patch"
    git apply --reverse "$output/navigation-current.patch"
    shasum -a 256 -c "$bundle/product.sha256"
)
python3 "$bundle/check-source-tree.py" "$destination"
printf '%s\n' "$destination"
