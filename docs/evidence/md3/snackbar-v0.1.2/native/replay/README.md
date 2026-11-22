# Replay the native Snackbar test

Use an exact candidate checkout and an isolated copy for the compact
launcher. Set `evidence_dir` to the top of this evidence package,
`qa_checkout` to the isolated copy, and `native_output` to a new
temporary directory. Open a normal macOS GUI session with room for a
960 × 800 logical window. Grant the local caller Accessibility access
when running the optional AppKit checks.

Build the stock Gallery and client from the exact candidate.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
stock_target="/tmp/cargo-target/surface-native-replay"
export CARGO_TARGET_DIR="$stock_target"
cap cargo build --locked -p surface-material-gallery --features devtools --bin surface-material-gallery
cap cargo build --locked -p surface-devtools --bin surface-devtools
```

The compact profile needs a 320 × 480 logical window. The launcher uses
the exact Gallery and host through the public runtime API.

```sh
qa_target="/tmp/cargo-target/surface-native-replay-compact"
mkdir -p "$qa_checkout/examples/material-gallery/src/bin"
cp "$evidence_dir/native/replay/snackbar_gallery_profile.rs" \
   "$qa_checkout/examples/material-gallery/src/bin/"
(
  cd "$qa_checkout"
  export CARGO_TARGET_DIR="$qa_target"
  cap cargo build --locked -p surface-material-gallery --features devtools --bin snackbar_gallery_profile
)
```

Build the optional AppKit reader and run all three native profiles.

```sh
swiftc -module-cache-path /tmp/surface-snackbar-swift-cache \
  "$evidence_dir/native/replay/read-snackbar-ax.swift" -o /tmp/read-snackbar-ax

PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/replay/verify-snackbar-native.py" \
  --source d693740b8635b0db4cf4a5f3cb96b421116defc6fc3ae603857a39ac91c6e507 \
  --gallery "$stock_target/debug/surface-material-gallery" \
  --cli "$stock_target/debug/surface-devtools" \
  --profile-launcher "$qa_target/debug/snackbar_gallery_profile" \
  --ax-reader /tmp/read-snackbar-ax \
  --out "$native_output"
```

A successful report sets `completed` to `true`. Check
`native_accessibility.verified` and `announcements.verified` separately
for each profile. An unsupported observer or untrusted AppKit reader
does not verify the corresponding gate. The captured announcement
requests establish publication to AppKit. Live VoiceOver speech is
outside this run.

The runner uses Surface's semantic input interface and its public
screenshot API. The host session descriptor, token and unfiltered trace
remain in private temporary storage.
