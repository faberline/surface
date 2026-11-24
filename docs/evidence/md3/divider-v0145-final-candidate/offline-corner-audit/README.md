# Divider M266 offline regression

This package rechecks the unmodified compact dark RTL 200% captures from the
failed Surface v0.1.45 GitHub push. Chrome 158.0.8127.88 used the SwiftShader
WebGPU adapter and reported zero page errors. This offline check ran no browser
or CI job. [Original browser job](https://github.com/faberline/surface/actions/runs/45472881234/job/138725260508)

`raw/provenance.json` records the original ZIP checksum and the member SHA-256
and CRC-32 values. The captured failure PNG equals the archived Heavy PNG
byte for byte. The package keeps one copy of that image.

## Reproduce

Use Python 3.11 or newer with Pillow.

```sh
shasum -a 256 -c SHA256SUMS
python3 -B verify-divider-fringe.py > /tmp/divider-offline.json
cmp verified-offline.json /tmp/divider-offline.json
```

If you have the original `surface-e2e-artifacts.zip`, add
`--original-zip /path/to/surface-e2e-artifacts.zip`. The verifier then
checks the ZIP and all four original member bytes, including the duplicate
failure PNG. The resulting JSON stays the same.

## Findings

The old 1 dp endpoint probe intersects a blended corner pixel on all four
Heavy rectangles. The new 1.5 dp probe returns the exact outline variant RGB
`[73,69,78]` at all 96 points. All eight original outside points match the
background. The test still samples each of the eight 1 dp depths.

Three independent transects across each rectangle contain exactly 16 solid
device pixels, which is 8 dp at DPR 2. The 24 adjacent controls and all 3,376
pixels one pixel outside the rectangles match the background `[20,18,24]`.
Each rectangle has exactly three blended corner pixels. The remaining 25,844
Heavy interior pixels use the exact outline color. The verifier derives the
Heavy bounds from the measured Regular layout and checks every enclosed pixel.

The square quad shader computes a signed distance and uses the sum of the
fragment derivatives to blend the edge. A 1 dp derivative width at a distance
of −0.25 dp gives coverage 0.84375. Blending the observed colors at that
coverage yields the observed corner RGB `[65,61,70]`. This calculation
explains the observed SwiftShader pixels. The archive contains no shader
instrumentation.

An in-memory change from `[73,69,78]` to `[72,69,78]` at a solid interior
point fails both the revised fixture and the independent scan.

`endpoint-sampling.patch` is the candidate test-only change. The two
583-file source manifests differ on only `e2e/src/material_divider.rs`.
The evidence establishes the result on the original capture. The corrected
candidate still needs its own browser run.
