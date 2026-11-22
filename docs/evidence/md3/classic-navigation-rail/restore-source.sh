#!/bin/sh
set -eu

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
base=$(CDPATH='' cd -- "${1:?Pass the verified Directional Drawer source checkout}" && pwd)
python3 "$here/check-source-tree.py" "$base" 01ebc338464b01cdaf9e9b860c065edefb1710740be37c8698dc896b555c0efe 662
work=$(mktemp -d /tmp/surface-md3-navigation-rail-source.XXXXXXXX)
mkdir "$work/source"
rsync -a --exclude='/.git' --exclude='/artifacts' --exclude='/target' "$base/" "$work/source/"
reverse() {
    gzip -dc "$here/$1.gz" > "$work/$1"
    test "$(shasum -a 256 "$work/$1" | awk '{print $1}')" = "$2"
    git -C "$work/source" apply --reverse --check "$work/$1"
    git -C "$work/source" apply --reverse "$work/$1"
}
reverse navigation-drawer-reverse.patch 1bc5c867a038c27b9089a6d1c3af81a4ddf5345af589bc5841583b61620692d7
reverse snackbar-reverse.patch 7af04c54b11d63b2bf9331ccd8871670308c14079597b90ceb83fdb39573a3ed
(
    cd "$work/source"
    shasum -a 256 -c "$here/product.sha256"
    shasum -a 256 -c "$here/e2e.sha256"
)
python3 "$here/check-source-tree.py" "$work/source" d1c6adecf9e4397259a34bfd844f4a0b3bf18ecf2fc9470d8dfc1e5b1b974f78 657
printf '%s\n' "$work/source"
