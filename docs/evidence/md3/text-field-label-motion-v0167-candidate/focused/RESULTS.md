# Scoped WebGPU result

The same pinned runner exercised M325–M350 against the same development
Gallery Web assets three times. Each runner completed 26 steps and 1,495
checks. The 15 label-motion PNGs have identical bytes across all three
runs. The typed label-motion JSON also has identical bytes. Chrome
reported 158.0.8127.37 and Apple Metal 3.

R1 and R2 still have a negative outer guard result. Both recorded no
visible windows. R2 identified two owned Google Updater jobs that stayed
past eight seconds. The guard cleaned them up. The original receipts
preserve both results.

R3 adds Chromium's `--disable-updater-scheduler` only to this Chrome
process. The observer saw the process in headless mode 256 times. It
recorded no visible windows and no owned processes after Chrome exited.
The timeout and ownership checks are the same as before. This run uses
a private profile and a process-local option.
[Chromium switch source](https://github.com/chromium/chromium/blob/main/chrome/common/chrome_switches.h)
[Chromium performance test use](https://chromium.googlesource.com/chromium/src/testing/+/29553bbe7bb533d38dfe8b67b9bf8e43e8b4db23)

The readback decodes the original PNGs. In the light Outlined field the
glyph top moves from 165.5 to 163 to 152 to 139 dp. The 14 ms sample
has paper on the upper stroke and primary ink below. At 44 ms it has
paper through the stroke. Reversal keeps the 152 dp pose. Reduce Motion
snaps to 139 dp at the same held clock time. In Filled, CDP composition
shows 441 marked-text pixels and no overlapping placeholder. In dark
RTL at 320 dp and 200 percent text, the short label moves about its
right anchor. The wrapped long label stays above its full closed outline.
The scope includes Chromium accessibility names and focus and simulated
composition through CDP.

## Replay

Run `shasum -a 256 -c EVIDENCE.sha256` in this lane to check the recorded
files. These are scoped development results. A full official stock run
uses its own exact release inputs and guard.

Once the controller authorizes another local run, use the pin command
in README.md. For R3 substitute `run-focused-r3.py` and a new output
path. This wrapper requires the prior v0.1.66 stock completion marker.
For an existing R3 output, run the independent readback with Python 3
and Pillow.

```sh
python3 -B verify-label-motion.py development-r3
```

The R1 and R2 PNGs can be decoded with `--allow-guard-negative`. Their
receipts retain the failed guard status.
