#!/bin/sh
set -eu
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools

prior=/tmp/surface-md3-text-field-multiline-v0165.ZRjH7vPS
current=/tmp/surface-md3-textbox-values-v0166.l4950l9e/native-development-r1
test -f /tmp/surface-md3-multiline-stock-v0165.C31fd1AG/stock-mac-v0165-passed
[ "$(shasum -a 256 "$current/source.sha256" | awk '{print $1}')" = 0675a92cea6b221f96ade99a9e7e5569094a313b64c1367879b75f905ae784cb ]
[ "$(shasum -a 256 "$current/binaries/SHA256SUMS" | awk '{print $1}')" = 89fc80bebbc8af785eca14e568f06429755ea1acd15520379490dd721cc0a2d7 ]
(cd "$current/source" && shasum -a 256 --status -c "$current/source.sha256")
(cd "$current/binaries" && shasum -a 256 --status -c SHA256SUMS)
cap /Users/chrischeng/.pyenv/versions/3.12.11/bin/python3 -E -B "$prior/preview-multiline-native.py" "$current" "$current/preview"
for name in wide-outlined-r1 wide-filled-r1 compact-outlined-r1 compact-filled-r1; do
    cmp "$prior/native-development-r1/preview/$name.png" "$current/preview/$name.png"
done
(cd "$current/preview" && shasum -a 256 *.png preview.json) > "$current/preview.sha256"
printf '%s\n' 'All four original native PNGs match the captured v0.1.65 bytes.'
