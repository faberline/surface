# Material 3 Search View checkpoint

The shared search field retains its query and focus across docked and expanded layouts. The caller supplies stable results and handles selection. The checkpoint includes fixes for long single-line queries and for horizontal pan when replacing them.

- Source tree SHA-256 `8788ca594976d65e993fc69dd12ae043c398a468c47fe583f80d8906822fa45b` for 654 files.
- Original 25-path Search View forward delta SHA-256 `73e765427726f3ea393eee18be3231b430156717c5f266d05721e8d42bf9a795`. This packet recovers Search View by reversing Classic Navigation Rail, then Search Feedback.

## Retained results

- Rust workspace verification passed 1,370 tests, with 25 ignored. The Metal offscreen suite passed 12 tests.
- Chrome 157 on Apple Metal 3 WebGPU passed 188 focused checks and 23,904 canonical checks. A CDP composition event followed by trusted Enter did not select a result. Replacing a long query restored both the original short-query ink and caret geometry. [Browser accessibility receipt](captures/browser-search-view-accessibility.json) · [Replaced query](captures/web-replaced-query.png)
- Native Apple M1 Max Metal ran four profiles with 74 CLI PNGs and 14 direct AppKit accessibility reads. The 320 dp dark RTL 200% CLI/MCP images matched byte for byte. The reopened short query retained its ink and had zero stale pan. Native composition used injected Winit events. Live VoiceOver speech and physical Zhuyin input were outside this verification. [Native audit](native-audit.json) · [320 dp capture](captures/native-320-dark-rtl-200.png)

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
