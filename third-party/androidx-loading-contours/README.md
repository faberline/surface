# AndroidX Loading Indicator contours

The generated contour data comes from AndroidX Material 3 and Graphics Shapes
at commit `428942a35fedec39ca7253a473a2ec3be06f614c`. Copyright 2024 The Android
Open Source Project. The derived data uses the [Apache License 2.0](LICENSE-APACHE).

- `LoadingIndicator.kt` supplies the order of its seven indeterminate shapes.
  [Pinned Compose Loading Indicator](https://android.googlesource.com/platform/frameworks/support/+/428942a35fedec39ca7253a473a2ec3be06f614c/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/LoadingIndicator.kt)
- `MaterialShapes.kt` supplies their normalized rounded polygons. The generator
  extracts the seven shape factories and their helpers. The small JVM adapters
  use the Offset and Matrix arithmetic required by those factories.
  [Pinned Material Shapes](https://android.googlesource.com/platform/frameworks/support/+/428942a35fedec39ca7253a473a2ec3be06f614c/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/MaterialShapes.kt)
- The pinned Graphics Shapes JVM source computes the matched cubics using
  `Morph.asCubics`. The generator compiles those source files unchanged with
  Collection 1.5.0 and Annotation 1.9.1, as declared in its build file.
  [Pinned Graphics Shapes source](https://android.googlesource.com/platform/frameworks/support/+/428942a35fedec39ca7253a473a2ec3be06f614c/graphics/graphics-shapes/src/)

The [generator](../../scripts/generate-loading-contours.py) lists the SHA-256
for each source and build-only jar. It verifies every input, including cached
files. Python and the Kotlin compiler run only when producing the data.
Surface renders its shipped point samples with its existing contour painter.

## Reproduce

Run from the Surface root with Python 3.10 or newer and a JDK. The first
run downloads the pinned inputs into the scratch directory.

```sh
scratch=$(mktemp -d /tmp/surface-loading-contours.XXXXXXXX)
python3 -B scripts/generate-loading-contours.py \
  --workdir "$scratch" --output "$scratch/contour_data.rs"
cmp "$scratch/contour_data.rs" crates/surface-material/src/loading_indicator/contour_data.rs
```

The generator subdivides both cubics of each match at the same parameters.
It checks the error from each polyline segment through progress 1.10, which
includes the actual spring overshoot. One scale fits the whole palette.
The radius proof uses the exact affine bounds switches of the centered
samples. It gives a maximum radius of 0.496998 in the active box. The seven
pairs contain 89 to 191 vertices. A further check samples 777 poses for
positive winding and crossing edges. The renderer shares one set of
points between neighboring pairs at exact endpoint junctions.
