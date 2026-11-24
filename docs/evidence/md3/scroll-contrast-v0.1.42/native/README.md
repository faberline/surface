# Surface v0.1.42 macOS headless contrast evidence

The tested candidate is `f5b2e24d298c60a06d7b85635baf7a47657e31e2`. Both light and dark Gallery scenes ran at 320 × 400 dp, right-to-left, with 200% text size. Two independent replays each produced eight PNGs and one report. Every PNG matched across `Page.screenshot`, the CLI, and the MCP tool. All nine output files also matched byte for byte across the two runs. CoreGraphics counted zero visible windows in 28 samples across the Gallery child processes.

## Stock and diagnostic paths

The stock Gallery and CLI came from the full production source in [stock-source.sha256](stock-source.sha256). The diagnostic Gallery came from [probe-source.sha256](probe-source.sha256). The diagnostic build differs in exactly two files through [the test overlay](mac-headless-contrast-probe.patch). It reads the actual macOS display preference once through NSWorkspace and exposes a private, authenticated request for driving the Runtime. The stock build rejects that request. Its two screenshots match the corresponding diagnostic Normal screenshots byte for byte.

NSWorkspace returned `false` for Increase Contrast. The Runtime reported Normal on startup. Each diagnostic replay then applied Normal → Increased → Normal to the same focused Quantity editor. The invalid `1203` draft, error description, focus, and accepted `1200` value stayed intact. The popup geometry stayed fixed. The capture checks verified a visible outline and its gap in both palettes. The restored Normal PNG matched the initial image. Invalid requests left the frame intact, and repeating Increased requested no redraw.

The transitions use a test request into the production setter. The stock headless route provides a Normal reference. The desktop observer, live VoiceOver speech, native input-method windows, and OS preference changes are outside this proof. The native Increased palette and Web forced-colors palette have separate verification.

| Capture | Light | Dark |
| --- | --- | --- |
| Stock Normal | [PNG](captures/stock-light-normal.png) | [PNG](captures/stock-dark-normal.png) |
| Diagnostic Normal | [PNG](captures/light-normal.png) | [PNG](captures/dark-normal.png) |
| Diagnostic Increased | [PNG](captures/light-increased.png) | [PNG](captures/dark-increased.png) |
| Diagnostic restored Normal | [PNG](captures/light-restored.png) | [PNG](captures/dark-restored.png) |

See [the native report](captures/native-contrast-report.json) for the startup read, exact focus geometry, sampled outline colors, changed pixel counts, and PNG metadata. [The repeat record](native-repeat.json) stores the matching outputs from both runs. [The input record](native-inputs.json) stores the hashes of both source manifests, both Gallery binaries, and the stock CLI. [Run 1](run-1.log) and [run 2](run-2.log) record the checks. From this folder, run `shasum -a 256 -c SHA256SUMS` to check the archived files.

## Reproduce

Use macOS with a Metal adapter, the Command Line Tools SDK, a working Python 3.12 installation, and the repository's `cap` build wrapper. The tested host ran macOS 27.2.1 on arm64 with Rust 1.96.1. Prepare a local Cargo dependency cache for an offline build. The script uses `CARGO_HOME` when set. It defaults to `/tmp/surface-md3-cargo`.

Run the commands from this evidence folder. Set `contrast_source` to a checkout of the tested commit. Both source copies and all output stay under `/tmp`. The script uses two Cargo jobs with a separate target for each source copy.

```sh
contrast_package="$(pwd -P)"
contrast_source=/path/to/surface-at-f5b2e24
contrast_output="$(mktemp -d /tmp/surface-contrast-proof.XXXXXXXX)"
python3 -B "$contrast_package/native-scripts/native-proof.py" prepare \
  --source "$contrast_source" \
  --source-manifest "$contrast_package/stock-source.sha256" \
  --source-sha256 163ec8673284b3af9c30af57c548b78730f8a5b1912cec164f79b374a5026a57 \
  --out "$contrast_output/sealed"
python3 -B "$contrast_package/native-scripts/native-proof.py" replay \
  --sealed "$contrast_output/sealed" \
  --out "$contrast_output/native-final"
```

The preparation step checks the supplied source manifest and confirms that the diagnostic startup marker occurs only in the diagnostic executable. Replay checks source, binaries, scripts, and overlay before and after each run. It verifies output equality inside the new run pair. The archived binary hashes identify the original tested builds.
