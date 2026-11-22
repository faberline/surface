#!/bin/sh
set -eu

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
base=$(CDPATH='' cd -- "${1:?Pass the verified Classic Navigation Rail source}" && pwd)
python3 "$here/check-source-tree.py" "$base" d1c6adecf9e4397259a34bfd844f4a0b3bf18ecf2fc9470d8dfc1e5b1b974f78 657 >&2
work=$(mktemp -d /tmp/surface-md3-search-view-source.XXXXXXXX)
mkdir "$work/source"
rsync -a --exclude='.git' --exclude='artifacts' --exclude='target' "$base/" "$work/source/"
reverse() {
    gzip -dc "$here/$1.gz" > "$work/$1"
    test "$(shasum -a 256 "$work/$1" | awk '{print $1}')" = "$2"
    git -C "$work/source" apply --reverse --check "$work/$1"
    git -C "$work/source" apply --reverse "$work/$1"
}
reverse navigation-rail.patch e92f6c52a7a409fa748bca3631c6f7c572b24966a881b8488f6f895d49e05cb3
reverse search-feedback.patch 598a8ec94ef3763b000a54879e9a02cdbc3f5a53c56edb77cf0ce4b1078043fc
(
    cd "$work/source"
    shasum -a 256 -c "$here/product.sha256" >&2
    shasum -a 256 -c "$here/e2e.sha256" >&2
)
python3 "$here/check-source-tree.py" "$work/source" 8788ca594976d65e993fc69dd12ae043c398a468c47fe583f80d8906822fa45b 654 >&2
printf '%s\n' "$work/source"
