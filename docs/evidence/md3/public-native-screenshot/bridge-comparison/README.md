# Native bridge comparison

Before the companion fix, the native and compact Jet bridge checks fail
at step B9 on both sources.
The CLI fails when it asks for a snapshot after the Gallery opens the
action section and runs its filled button. The Web bridge follows the
same action flow and passes.

The comparison used freshly built native Gallery binaries from the
[verified Wide Rail checkpoint](https://github.com/faberline/surface/tree/2c22421a180f27cf5d4aeaa55ffa8dc3cde7e1dc/docs/evidence/md3/wide-navigation-rail)
and from the public screenshot source. It used the same schema-aware
Surface runner and Jet client for both. The Jet client reported
`incomplete or oversized application response`. A second, read-only
snapshot request measured 327,212 bytes without the final newline.
Both window sizes and both source versions produced the same records.
[Baseline JSON](baseline.jsonl) · [Screenshot source JSON](with-screenshot.jsonl)

The server had a complete reply of 376,299 bytes in the normal window and
372,478 bytes in the compact window. The write stopped with `WouldBlock`
in under 150 microseconds. A 500 ms write timeout was already set.
On this Mac an accepted socket inherits the nonblocking mode of the
listener. The companion fix sets each accepted socket to blocking mode,
leaving the existing read and write deadlines in force. With only that
fix added, both native journeys passed. The same replies completed in
under 360 microseconds in the instrumented run.
[Socket flags](accept-flags.log) · [Server measurements](server-write.log)
· [Fixed client measurements](fixed.jsonl) · [Companion fix](native-bridge-fix.patch.gz)

The wrapper logs byte counts, the presence of the frame delimiter and
known error strings. The authenticated request stays on the tested
loopback endpoint. The records omit the snapshot body and session token.

Build each version of the Gallery in a separate temporary checkout.
Use the final screenshot source to build the shared runner and Jet
client. Use the same macOS desktop with Metal for both runs. Repeat
the command with each Gallery build and an empty artifact directory.

```sh
cap cargo build --locked -p surface-material-gallery --features devtools --bin surface-material-gallery --example compact
chmod 755 /absolute/path/to/bridge-comparison/diagnose.py
export SURFACE_BRIDGE_DIAGNOSTIC_BIN=/absolute/path/to/shared/jet-bridge
export SURFACE_BRIDGE_DIAGNOSTIC_LOG=/tmp/native-bridge-diagnostic.jsonl
/absolute/path/to/shared/surface-e2e \
  --jet-bridge /absolute/path/to/bridge-comparison/diagnose.py \
  --native-bin /absolute/path/to/gallery/debug/surface-material-gallery \
  --compact-bin /absolute/path/to/gallery/debug/examples/compact \
  --artifacts /absolute/path/to/artifacts bridge-native bridge-native-compact
```
