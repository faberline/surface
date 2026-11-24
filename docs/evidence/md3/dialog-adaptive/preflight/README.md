# Material Dialog headless preflight

This Surface scratch candidate adds Material Basic and Full-screen dialogs to the shared library and a localized Gallery route. The [563-file source manifest](source/dialog-gallery.sha256) has SHA-256 `46b4b82931a1f84977693a38d6e42f5c591795aa9ee27ad63e280d578a6cd802`. It uses the earlier Search preflight source and predates the browser cancellation repair. [Earlier Search candidate](../../search-adaptive/v0143-pre-cancel-fix/) · [Subsequent cancellation fix](https://github.com/faberline/surface/tree/f45cd7d0223719179542a8a4f92c161089359dfc/docs/evidence/md3/scroll-contrast-v0.1.42/web-cancel-fix-preflight)

Basic uses a bounded panel with separate title, scrollable body and actions. Full-screen uses the entire viewport at compact sizes. The application keeps the editor's identity through the 600 dp width breakpoint. It owns save and discard. The alert requires acknowledgement. [Google Dialog specification](https://m3.material.io/components/dialogs/specs) · [Google token table](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.0b1e39a67d5bafdc.json)

- Two stock offscreen Metal journeys produced the same reports and eight identical PNGs. Page.screenshot, CLI and MCP agreed. All 24 checks saw zero visible application windows. [Native reports and screenshots](native/)
- All three Gallery Runtime tests passed. They cover scroll containment, explicit acknowledgement, resize during composition, directed selection and draft ownership. [Gallery test receipt](gallery-tests.log)
- An independent Chrome fixture saw an alert role, heading-derived name, description and modal background suppression in both DOM and the accessibility tree. It also verified fallback naming, the ordinary dialog role and dismissal. The fixture uses the component layer before the Gallery overlay. [Chrome fixture receipt](browser/web-alert-dialog.json)

Right-to-left action placement works. The captures still show start alignment issues for some RTL headings. At 320×400 with 200% text, the discard dialog's initial view clips the second supporting line into its scroll area. Browser Gallery replay, live screen-reader speech, OS input methods and dialog motion remain to be checked.

## Reproduce

Rebuild the 557-file v0.1.43 Search scratch source from the [earlier instructions](../../search-adaptive/integration/README.md#source-and-replay). Keep this evidence folder separate from that source directory. Set `stage` to an absolute scratch directory with the Search source at `"$stage/source"` and `evidence` to the absolute path of this preflight folder. Export a fresh `CARGO_TARGET_DIR` below `/tmp` for this build. The native replay requires macOS with Metal and Python 3.

```sh
(
  cd "$stage/source"
  git apply --whitespace=error-all "$evidence/source/basic-full-ax.patch"
  shasum -a 256 --status -c "$evidence/source/dialog-component.sha256"
  git apply --whitespace=error-all "$evidence/source/dialog-gallery.patch"
  shasum -a 256 --status -c "$evidence/source/dialog-gallery.sha256"
  CARGO_BUILD_JOBS=2 cargo build --locked -p surface-material-gallery -p surface-devtools
)
cp "$evidence/source/dialog-gallery.sha256" "$stage/source.sha256"
mkdir "$stage/pins"
cp "$CARGO_TARGET_DIR/debug/surface-material-gallery" "$CARGO_TARGET_DIR/debug/surface-devtools" "$stage/pins/"
(
  cd "$stage/pins"
  shasum -a 256 surface-material-gallery surface-devtools > native-binaries.sha256
)
(
  cd "$evidence/native"
  sh replay.sh "$stage" "$stage/replay"
)
```

The archived SHA-256 receipts identify the original binaries. A local rebuild records its own binary digests before and after its two replays. Compare its semantic report and images with the archived proof on equivalent hardware. Run `shasum -a 256 -c SHA256SUMS` from this evidence folder to verify the archive.
