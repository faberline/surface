# Material Search keyboard highlight feedback

The caller supplies a localized status through `Theme::search_view_with_status`. Gallery reports the highlighted destination’s name and position while the query keeps focus.

- Source tree SHA-256 `4cb2ac5f7879b4fc4b56efec5a8efed2a73cc09e3e8b248b18b96ca38f27cbc4` for 655 files.
- Original 8-path Search Feedback forward delta SHA-256 `598a8ec94ef3763b000a54879e9a02cdbc3f5a53c56edb77cf0ce4b1078043fc`. Its predecessor is Search View.

## Retained results

- Rust workspace verification passed 1,370 tests, with 25 ignored.
- Chrome 157 on Apple Metal 3 WebGPU passed 209 focused checks and 23,925 canonical checks. Two moves caused two text mutations in the same status region. Resizing preserved the dialog, status element, and editor focus. Closing retired the old action and restored the opener. [Browser accessibility receipt](captures/browser-search-view-accessibility.json)
- The native Apple M1 Max Metal run recorded six CLI PNGs and five exact public AppKit focus checks. A collector in the private Gallery process observed two polite announcement requests. Repeating the query, resizing from 960 to 320 dp, and closing caused no extra requests. The resized frame matches the Search View highlight frame byte for byte. Live VoiceOver speech was outside this verification. [AppKit requests](captures/native-appkit-requests.tsv) · [Native audit](native-audit.json) · [320 dp capture](captures/native-320-dark-rtl-200.png)

## Restore the product source

Use the [Classic Navigation Rail recovery procedure](https://github.com/faberline/surface/tree/b457ea7e2f4d65f75ac2b7e66e247481b9f13989/docs/evidence/md3/classic-navigation-rail) first. This packet verifies that source and reverses the saved deltas. It checks the complete file list, file hashes, symlink targets and executable bits of this checkpoint.

```sh
shasum -a 256 -c evidence.sha256
packet_dir="$PWD"
source_dir=$(sh restore-source.sh /path/to/verified-classic-navigation-rail-source)
cd "$source_dir"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-$(basename "$(dirname "$source_dir")")"
sh scripts/check.sh
cap cargo xtask web e2e --artifacts "$(mktemp -d /tmp/surface-search-browser.XXXXXXXX)" material web-accessibility
```

The browser command rebuilds the canonical suites. After it builds both Gallery bundles, the same shell can run the focused M174 and A63 fixtures.

```sh
focused_dir="$(dirname "$source_dir")/focused-source"
focused_target="$CARGO_TARGET_DIR-focused"
python3 "$packet_dir/prepare-search-view-browser.py" "$source_dir" "$focused_dir"
(
    cd "$focused_dir"
    shasum -a 256 -c "$packet_dir/focused-e2e.sha256"
    CARGO_TARGET_DIR="$focused_target" cap cargo build --locked -p surface-e2e --bin surface-e2e
    "$focused_target/debug/surface-e2e" \
        --web-root "$CARGO_TARGET_DIR/web/release" \
        --bridge-web "$CARGO_TARGET_DIR/web/devtools/material-gallery" \
        --artifacts "$(mktemp -d /tmp/surface-search-focused.XXXXXXXX)" \
        material web-accessibility
)
```

The native historical runs used a private Gallery overlay with test input and screenshot hooks. The packet preserves source identity, raw run hashes, and selected evidence from the original runs. It excludes the private overlay, binaries, session files and broad generated output. [Test receipts](test-receipts.json)
