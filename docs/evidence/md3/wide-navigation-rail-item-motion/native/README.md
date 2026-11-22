# Native wide rail item motion

The Surface Material Gallery ran on macOS 27.2.1 and Apple M1 Max Metal. All captures use production manifest `item-motion-production.sha256` with SHA-256 `97bb30b5aa0e1153a00ca7210c54bbdd791db0345a8058885b5862d70967fe38`. The sizing launcher uses the public platform options for the unmodified Gallery. All input uses public DevTools keys and actions. The Swift helper reads the spawned Gallery's accessibility data.

## Replay

Use the restored product source and an active macOS session with Metal. The replay needs Cap, Python 3 and the Xcode command-line tools.

```sh
shasum -a 256 -c evidence.sha256
bash build-and-run.sh /path/to/restored-surface /tmp/new-item-motion-captures
python3 -B audit-item-frames.py captures/inline
```

The last command audits specified archived frames. A new run can sample different widths because the native host uses its real clock. The inline runner keeps sample timings and observed widths. The optional compact retained run is in `verify-wide-rail-motion.py --profile dark-rtl-compact-retained`, which takes the same Gallery, CLI, AX reader and artifact arguments.

## Inline profiles

The 704 × 512 dp windows exercised light LTR and dark RTL at 200% text. Both retain exactly one public AppKit Inbox button with current-page semantics at both endpoints. The direction keys skip disabled Archive. Enter makes Inbox current. The disclosure keeps focus on activation, and selection survives reflow. Each of the four settled endpoint PNGs matches byte for byte across the raw public protocol, CLI and MCP.

The compact destination label wraps below the moving icon and pill. Light closing frames cover 106, 118, 128 and 130 dp. Dark RTL closing frames cover 118, 122 and 130 dp. The archived raster audit finds two light text bands and five dark RTL text bands. The corresponding line pitches are 8 and 16 pixels at 0.5× output. The selected pill and icon move between the two measured widths in both directions. The native opening samples show intermediate widths above 200 dp. The 105–130 dp evidence comes from closing.

## Optional retained modal profile

The 320 × 240 dp dark RTL retained-modal attempt confirmed keyboard focus, AppKit modal semantics, full two-line Inbox after arrow navigation and immediate Escape return to the opener. The retiring Inbox row moved as the items reflowed. A public snapshot found navigation scroll offset remained 136 dp. The closing visual overlay was captured at 160 dp. The first available opening PNG already filled the width, so the strict live opening-middle comparison has incomplete evidence. The retained files include the diagnostic sample sequence and the settled focused PNG.

The evidence contains application captures and sanitized receipts. Session descriptors, local binaries and system-wide input are outside this replay.
