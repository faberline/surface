# Native Badge replay

Build the production Surface Gallery with the `devtools` feature. Build
the `surface-devtools` binary from the same source. Copy
`badge_gallery_profile.rs` into `examples/material-gallery/src/bin/` in a
separate checkout of that source. Build `badge_gallery_profile` with
`devtools`. Its only change to the native launch is the logical window
size.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${PWD##*/}"
cap cargo build --locked -p surface-material-gallery --features devtools \
    --bin surface-material-gallery --bin badge_gallery_profile
cap cargo build --locked -p surface-devtools --bin surface-devtools
```

Set the source digest and choose a new output directory under `/tmp`.
Run from a macOS desktop with a Metal GPU and room for a 960 × 800
logical window. The optional AppKit reader requires macOS accessibility
permission for the local caller.

```sh
scratch_dir=$(mktemp -d /tmp/surface-badge-replay.XXXXXXXX)
swiftc -module-cache-path "$scratch_dir/swift-cache" read-badge-ax.swift \
    -o "$scratch_dir/read-badge-ax"
PYTHONDONTWRITEBYTECODE=1 python3 verify-badge-native.py \
    --source "$candidate_sha" \
    --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" \
    --profile-launcher "$CARGO_TARGET_DIR/debug/badge_gallery_profile" \
    --ax-reader "$scratch_dir/read-badge-ax" \
    --out "$scratch_dir/results"
```

The replay checks light and dark 960 × 800 windows and a dark
320 × 480 window with right-to-left layout and 200% text. It observes
standalone and anchored badge geometry, painted fill and ink, original
48 dp anchor size, the 999 to 1000 update, selected state, absence of
paint after dismissal, destination icon visibility, and scroll edge
clearance. It counts one inbox update in the native trace, checks the
next named Tab target, and compares the compact PNG bytes across the
raw API, CLI, and MCP.

AppKit checks match one accessible action per visible subject and look
for the exact uncapped label. The input tests send semantic keys and
activation through Surface DevTools. The replay does not establish
physical keyboard, touch, or live VoiceOver speech. The browser replay
covers pointer hit testing and scrolling. Keep the report, PNGs, matching
source and build evidence. Private session files and unfiltered traces
stay in local temporary storage.
