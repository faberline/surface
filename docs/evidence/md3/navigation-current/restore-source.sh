#!/bin/sh
set -eu

bundle=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
drawer_source=$(CDPATH='' cd -- "${1:?Pass the verified Drawer Swipe source checkout}" && pwd)
output=$(mktemp -d /tmp/surface-md3-navigation-current-source.XXXXXXXX)
destination="$output/source"
gzip -dc "$bundle/drawer-swipe-reverse.patch.gz" > "$output/drawer-swipe.patch"
actual=$(shasum -a 256 "$output/drawer-swipe.patch" | awk '{print $1}')
test "$actual" = bcd087692a296a119f57a2ab8ea38cf84020ae1ccc4ee81566443ef4cef947da
mkdir -p "$destination"
rsync -a --exclude='/.git' --exclude='/artifacts' --exclude='/target' "$drawer_source/" "$destination/"
(
    cd "$destination"
    git apply --reverse --check "$output/drawer-swipe.patch"
    git apply --reverse "$output/drawer-swipe.patch"
    shasum -a 256 -c "$bundle/product.sha256"
)
python3 "$bundle/check-source-tree.py" "$destination"
printf '%s\n' "$destination"
