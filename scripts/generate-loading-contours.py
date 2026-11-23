#!/usr/bin/env python3
"""Generate Surface's matched outline samples from pinned AndroidX sources.

Run with Python 3.10 or newer and a JDK. Build artefacts go in --workdir.
"""

import argparse
import base64
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from math import hypot, isfinite
import os
from pathlib import Path
import re
import subprocess
from textwrap import dedent
from urllib.request import urlopen

REVISION = "428942a35fedec39ca7253a473a2ec3be06f614c"
GITILES = "https://android.googlesource.com/platform/frameworks/support/+/"
GRAPHICS = "graphics/graphics-shapes/src/"
MATERIAL = "compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/"
MAVEN = "https://repo.maven.apache.org/maven2/"
GOOGLE_MAVEN = "https://dl.google.com/dl/android/maven2/"

MAX_PROGRESS = 1.10
FLATNESS = .0012
SAFE_RADIUS = .497
UNITS = 32768
MAX_POINTS = 256

SOURCE_PINS = {
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/CornerRounding.kt":
        "c74175544928ba99fb0c9c86d376be5ac72af8fcdbdd4fe1860b413883cfd71c",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/Cubic.kt":
        "dd081ab5b869fc0e20e473792c4eb375e9e1f72b5ec5a15acbd2feacbb09a1e8",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/FeatureDetector.kt":
        "f3525ebede482d4655b1ce806d9f0b41808cebd3f077982e2d2611cdc61d36ac",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/FeatureMapping.kt":
        "bce6cb59d6aba701e78301c1f897902ad7005bfabf08061c1ca9a608c46213da",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/FeatureSerializer.kt":
        "fd254d8c63228a277b61db56dd6bfd99d77255a057e9ed7c783d0eeb0d5879c7",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/Features.kt":
        "a77f8133ef361195da2666035763a42fb42f3e3744943e836a9ee0202518bf26",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/FloatMapping.kt":
        "daf0655958e17cffb291554b62a082abf1b0336f5f71da453abae0d9c8fc698b",
    GRAPHICS + "jvmMain/kotlin/androidx/graphics/shapes/Format.jvm.kt":
        "162536d6546d70309d9cecb2a7656271004cfa20c85bba8689a856a541390dd9",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/Format.kt":
        "84af7e6d833b8dee690520cd076b732c6ada0c331ea157686cab94826f8c1504",
    MATERIAL + "LoadingIndicator.kt":
        "b0ff60de452990100a72a32240cab2c5bf3cffc9f4fec7507322863298c2d0c5",
    MATERIAL + "MaterialShapes.kt":
        "2800909d9dd199badc8ff53081ab7e5553f05db4eb7bb208095209db58603f41",
    "compose/ui/ui-graphics/src/commonMain/kotlin/androidx/compose/ui/graphics/Matrix.kt":
        "36cb7aa36bbf77b154976a7c59075526d4e05126c82657d586e2c0a1fc17540f",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/Morph.kt":
        "9023d8d7c90f9e98bed1f13adf4f0b98e8089bc6cf1f0b39eb4880c64556f2e7",
    "compose/ui/ui-geometry/src/commonMain/kotlin/androidx/compose/ui/geometry/Offset.kt":
        "cc113338a6aecdb7873debf0c8bbc76f48e19878e02e2c9df563d380f0c2b98e",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/Point.kt":
        "8001571066738237348f244b3a79c35c4124ccaea02cc5f4350c76ff67ef878f",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/PolygonMeasure.kt":
        "99bf28fab8ab461145b63c0f5c40d8c01a4747d17d19abf6fcfb2bab24a5c7ec",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/PolygonValidation.kt":
        "7b67751e4684890eb35375a8d969fccbf5a2c1328fd7511417de72a264ba3afc",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/RoundedPolygon.kt":
        "e9b621758b14a0a2b43f5c0332ed784759e444136642c79f0e71493ede5c4b81",
    MATERIAL + "internal/ShapeUtil.kt":
        "894f8cc0121eb13b92e7f28aaa06c04a06b89bfbb7d423f09c8787d68b02c0d5",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/Shapes.kt":
        "b3ac321cec092a8f8e8b2d23809ed1d4e9365dfd919c7a8f2f0e7a06a4910a93",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/SvgPathParser.kt":
        "b3933f3630da3a03f45deabcc3a0ae8eb8ea569d52e530fe8966d94e5b2ebda1",
    GRAPHICS + "commonMain/kotlin/androidx/graphics/shapes/Utils.kt":
        "6622b6920c80f661120c57ce56a190e24a2e41cb5912ec80d1b22eb0729b14ff",
    "LICENSE.txt":
        "809fa1ed21450f59827d1e9aec720bbc4b687434fa22283c6cb5dd82a47ab9c0",
}

JAR_PINS = {
    MAVEN + "org/jetbrains/kotlin/kotlin-compiler-embeddable/2.1.20/kotlin-compiler-embeddable-2.1.20.jar":
        "c54a00718c8c0e3ee858bf42771c7f401c5e3c1738861e18e9536371042fa1b3",
    MAVEN + "org/jetbrains/kotlin/kotlin-daemon-embeddable/2.1.20/kotlin-daemon-embeddable-2.1.20.jar":
        "d9e83df1d847a201ba3c016a786ced091be9503997d09f6a9cf1796ee489f374",
    MAVEN + "org/jetbrains/kotlin/kotlin-reflect/1.6.10/kotlin-reflect-1.6.10.jar":
        "3277ac102ae17aad10a55abec75ff5696c8d109790396434b496e75087854203",
    MAVEN + "org/jetbrains/kotlin/kotlin-script-runtime/2.1.20/kotlin-script-runtime-2.1.20.jar":
        "ae4397fbb3aa2a1ada09290e753bba99a4114548977c3115526b9958f4cb1a04",
    MAVEN + "org/jetbrains/kotlin/kotlin-stdlib/2.1.20/kotlin-stdlib-2.1.20.jar":
        "1bcc74e8ce84e2c25eaafde10f1248349cce3062b6e36978cbeec610db1e930a",
    MAVEN + "org/jetbrains/intellij/deps/trove4j/1.0.20200330/trove4j-1.0.20200330.jar":
        "c5fd725bffab51846bf3c77db1383c60aaaebfe1b7fe2f00d23fe1b7df0a439d",
    MAVEN + "org/jetbrains/kotlinx/kotlinx-coroutines-core-jvm/1.8.0/kotlinx-coroutines-core-jvm-1.8.0.jar":
        "9860906a1937490bf5f3b06d2f0e10ef451e65b95b269f22daf68a3d1f5065c5",
    MAVEN + "org/jetbrains/annotations/13.0/annotations-13.0.jar":
        "ace2a10dc8e2d5fd34925ecac03e4988b2c0f851650c94b8cef49ba1bd111478",
    GOOGLE_MAVEN + "androidx/collection/collection-jvm/1.5.0/collection-jvm-1.5.0.jar":
        "70b35924e4babcdffa37d0e575ee039c56a2d97123342624c48b603233704341",
    GOOGLE_MAVEN + "androidx/annotation/annotation-jvm/1.9.1/annotation-jvm-1.9.1.jar":
        "1e343917ebf27ba96fe4dc52b1cad7fd32b738fbc6355bb6cd5b3b305d7212d0",
}

# Extract the exact shape factories and reuse the existing normalized shape
# APIs. These narrow JVM adapters mirror only the Compose arithmetic those
# factories call, so the generator does not need an Android or UI process.
HARNESS_PREFIX = """
import androidx.graphics.shapes.*
import kotlin.math.PI
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.sin
import kotlin.math.sqrt

private data class Offset(val x: Float, val y: Float) {
    operator fun plus(other: Offset) = Offset(x + other.x, y + other.y)
    operator fun minus(other: Offset) = Offset(x - other.x, y - other.y)
    operator fun times(value: Float) = Offset(x * value, y * value)
    fun getDistance() = sqrt(x * x + y * y)

    companion object { val Zero = Offset(0f, 0f) }
}

private class Matrix {
    private var a00 = 1f
    private var a01 = 0f
    private var a10 = 0f
    private var a11 = 1f

    fun rotateZ(degrees: Float) {
        val angle = degrees * (PI / 180.0)
        val s = sin(angle).toFloat()
        val c = cos(angle).toFloat()
        val v00 = c * a00 + s * a10
        val v10 = -s * a00 + c * a10
        val v01 = c * a01 + s * a11
        val v11 = -s * a01 + c * a11
        a00 = v00
        a01 = v01
        a10 = v10
        a11 = v11
    }

    fun scale(x: Float, y: Float) {
        a00 *= x
        a01 *= x
        a10 *= y
        a11 *= y
    }

    fun map(x: Float, y: Float) = TransformResult(a00 * x + a10 * y, a01 * x + a11 * y)
}

private fun RoundedPolygon.transformed(matrix: Matrix) = transformed(matrix::map)
private inline fun <T, R> List<T>.fastMap(transform: (T) -> R) = map(transform)
private val rotateNeg45 = Matrix().apply { rotateZ(-45f) }
private val rotateNeg90 = Matrix().apply { rotateZ(-90f) }
private val cornerRound15 = CornerRounding(radius = .15f)
private val cornerRound50 = CornerRounding(radius = .5f)

"""

HARNESS_MAIN = """
fun main() {
    val palette = listOf(/*PALETTE*/)
    fun Cubic.serialize() = listOf(
        anchor0X, anchor0Y, control0X, control0Y,
        control1X, control1Y, anchor1X, anchor1Y
    ).joinToString(" ")
    for (index in palette.indices) {
        val morph = Morph(palette[index], palette[(index + 1) % palette.size])
        val start = morph.asCubics(0f)
        val end = morph.asCubics(1f)
        check(start.size == end.size)
        println("pair ${start.size}")
        for ((a, b) in start.zip(end)) println("${a.serialize()} ${b.serialize()}")
    }
}
"""


def fetch(url, target, digest, *, gitiles=False):
    if target.exists():
        data = target.read_bytes()
    else:
        with urlopen(url, timeout=60) as response:
            payload = response.read()
        data = base64.b64decode(payload) if gitiles else payload
    if hashlib.sha256(data).hexdigest() != digest:
        raise ValueError(f"SHA-256 mismatch for {target}")
    if not target.exists():
        target.write_bytes(data)


def download_inputs(workdir):
    official = workdir / "official"
    compiler = workdir / "compiler"
    official.mkdir(exist_ok=True)
    compiler.mkdir(exist_ok=True)
    futures = []
    with ThreadPoolExecutor(max_workers=4) as pool:
        for source, digest in SOURCE_PINS.items():
            url = f"{GITILES}{REVISION}/{source}?format=TEXT"
            futures.append(pool.submit(fetch, url, official / Path(source).name, digest, gitiles=True))
        for url, digest in JAR_PINS.items():
            futures.append(pool.submit(fetch, url, compiler / Path(url).name, digest))
        for future in futures:
            future.result()
    return official, compiler


def prepare_harness(material, indicator):
    aliases = {
        "SoftBurst": "softBurst",
        "Cookie9Sided": "cookie9",
        "Pentagon": "pentagon",
        "Pill": "pill",
        "Sunny": "sunny",
        "Cookie4Sided": "cookie4",
        "Oval": "oval",
    }
    palette = re.search(
        r"val IndeterminateIndicatorPolygons\s*=\s*listOf\((.*?)\n\s*\)", indicator, re.S,
    )
    names = re.findall(r"MaterialShapes\.(\w+)", palette[1])
    if names != list(aliases):
        raise ValueError(f"Unexpected Compose loading sequence: {names}")
    methods = []
    for name in names:
        function = aliases[name]
        normalized = rf"val {name}\s+get\(\)\s*=\s*_\w+\s*\?: {function}\(\)\.normalized\(\)"
        if not re.search(normalized, material):
            raise ValueError(f"Unexpected Compose normalization for {name}")
        method = re.search(
            rf"        internal fun {function}\(\): RoundedPolygon \{{.*?\n        \}}", material, re.S,
        )
        methods.append(dedent(method[0]).replace("internal fun", "private fun", 1))
    helpers = re.search(
        r"        private data class PointNRound\(.*?(?=\n    \}\n\})", material, re.S,
    )[0]
    factory_order = ", ".join(f"{aliases[name]}().normalized()" for name in names)
    return names, "\n\n".join([HARNESS_PREFIX, *methods, dedent(helpers),
                               HARNESS_MAIN.replace("/*PALETTE*/", factory_order)])


def run_morph(workdir):
    official, compiler = download_inputs(workdir)
    names, harness = prepare_harness(
        (official / "MaterialShapes.kt").read_text(),
        (official / "LoadingIndicator.kt").read_text(),
    )
    harness_file = workdir / "Harness.kt"
    harness_file.write_text(harness)
    jars = sorted(compiler / Path(url).name for url in JAR_PINS)
    classpath = os.pathsep.join(map(str, jars))
    common = sorted(official / Path(path).name for path in SOURCE_PINS
                    if path.startswith(GRAPHICS + "commonMain/"))
    compiled = workdir / "compiled"
    subprocess.run([
        "java", "-Xmx768m", "-cp", classpath, "org.jetbrains.kotlin.cli.jvm.K2JVMCompiler",
        "-Xmulti-platform", "-Xcommon-sources=" + ",".join(map(str, common)), "-no-stdlib",
        "-no-reflect", "-classpath", classpath, "-d", compiled, *common, official / "Format.jvm.kt",
        harness_file,
    ], check=True)
    data = subprocess.check_output([
        "java", "-cp", os.pathsep.join([str(compiled), classpath]), "HarnessKt",
    ], text=True)
    (workdir / "cubics.txt").write_text(data)
    return names, list(load_paired_cubics(data))


def interpolate(start, end, amount):
    return tuple(a + (b - a) * amount for a, b in zip(start, end))


def split(curve):
    a, b, c, d = curve
    ab = interpolate(a, b, .5)
    bc = interpolate(b, c, .5)
    cd = interpolate(c, d, .5)
    left = interpolate(ab, bc, .5)
    right = interpolate(bc, cd, .5)
    center = interpolate(left, right, .5)
    return (a, ab, left, center), (center, right, cd, d)


def chord_error(curve):
    a, b, c, d = curve
    # In Bézier form the chord has control points at 1/3 and 2/3.
    # The largest control-point difference bounds the curve's displacement.
    return max(hypot(*(x - y for x, y in zip(control, interpolate(a, d, amount))))
               for control, amount in ((b, 1 / 3), (c, 2 / 3)))


def flatten(start, end):
    extrapolated = tuple(interpolate(a, b, MAX_PROGRESS) for a, b in zip(start, end))
    # The difference from the chord is affine in morph progress. Its maximum
    # bound over the supported spring interval is attained at an endpoint.
    if max(chord_error(start), chord_error(extrapolated)) <= FLATNESS:
        yield start[0], end[0]
        return
    for first, second in zip(split(start), split(end)):
        yield from flatten(first, second)


def envelope_switches(lines):
    """Find the progress values where an upper envelope switches points."""
    stack = []
    for slope, start in sorted(lines):
        if stack and slope == stack[-1][0]:
            stack.pop()
        while stack:
            previous_slope, previous_start, previous_switch = stack[-1]
            switch = (previous_start - start) / (slope - previous_slope)
            if switch > previous_switch:
                break
            stack.pop()
        else:
            switch = float("-inf")
        stack.append((slope, start, switch))
    return {switch for _, _, switch in stack if 0. < switch < MAX_PROGRESS}


def centered(points):
    x_values, y_values = zip(*points)
    center_x = (min(x_values) + max(x_values)) * .5
    center_y = (min(y_values) + max(y_values)) * .5
    return [(x - center_x, y - center_y) for x, y in points]


def envelope_radius(pairs):
    progress_limits = {0., MAX_PROGRESS}
    for axis in (0, 1):
        for sign in (1, -1):
            progress_limits.update(envelope_switches(
                (sign * (b[axis] - a[axis]), sign * a[axis]) for a, b in pairs
            ))
    # Between switches the center and every vertex are affine in progress.
    # Distance to the center is convex, so test only the interval endpoints.
    return max(hypot(*point) for progress in progress_limits
               for point in centered([interpolate(a, b, progress) for a, b in pairs]))


def load_paired_cubics(data):
    records = iter(data.splitlines())
    for header in records:
        command, count = header.split()
        if command != "pair":
            raise ValueError(f"Invalid Morph record: {header}")
        curves = []
        for _ in range(int(count)):
            values = list(map(float, next(records).split()))
            if len(values) != 16 or not all(map(isfinite, values)):
                raise ValueError("Invalid paired cubic")
            points = list(zip(values[::2], values[1::2]))
            curves.append((points[:4], points[4:]))
        yield curves


def has_crossing(points):
    def side(a, b, point):
        return (b[0] - a[0]) * (point[1] - a[1]) - (b[1] - a[1]) * (point[0] - a[0])

    edges = []
    for index, (a, b) in enumerate(zip(points, points[1:] + points[:1])):
        if a != b:
            edges.append((min(a[0], b[0]), max(a[0], b[0]), min(a[1], b[1]), max(a[1], b[1]),
                          index, a, b))
    edges.sort()
    for edge, (_, right, top, bottom, index, a, b) in enumerate(edges):
        for other in range(edge + 1, len(edges)):
            left, _, min_y, max_y, affix, c, d = edges[other]
            if left > right:
                break
            if min_y > bottom or max_y < top or abs(index - affix) in (1, len(points) - 1):
                continue
            if side(a, b, c) * side(a, b, d) < 0 and side(c, d, a) * side(c, d, b) < 0:
                return True
    return False


def validate_samples(pairs):
    radius = max(envelope_radius(pair) for pair in pairs) / UNITS
    if radius >= .5:
        raise ValueError(f"Contour exceeds the rotation envelope: {radius}")
    for pair in pairs:
        if not 3 <= len(pair) <= MAX_POINTS:
            raise ValueError(f"Contour has {len(pair)} samples")
        for step in range(111):
            points = [interpolate(a, b, step / 100.) for a, b in pair]
            successors = points[1:] + points[:1]
            area = sum(a[0] * b[1] - a[1] * b[0] for a, b in zip(points, successors))
            if area <= 0 or has_crossing(points):
                raise ValueError(f"Contour has invalid winding or a crossing at {step / 100.}")
    return radius


def generate(names, curves, output):
    samples = [[point for a, b in group for point in flatten(a, b)] for group in curves]
    scale = SAFE_RADIUS / max(envelope_radius(pair) for pair in samples)

    def quantize(point):
        return tuple(round((value - .5) * scale * UNITS) for value in point)

    points = [[(quantize(a), quantize(b)) for a, b in pair] for pair in samples]
    radius = validate_samples(points)
    lines = [
        "// Generated by scripts/generate-loading-contours.py.",
        "// Shape data derived from AndroidX, Copyright 2024 The Android Open Source Project.",
        "// SPDX-License-Identifier: Apache-2.0",
        "// See third-party/androidx-loading-contours/README.md for the source pins.",
        "",
        "pub(super) const PAIRS: [&[[i16; 4]]; 7] = [",
    ]
    for index, pair in enumerate(points):
        lines.extend([f"    // {names[index]} to {names[(index + 1) % len(names)]}.", "    &["])
        for start, end in pair:
            values = ", ".join(map(str, (*start, *end)))
            lines.append(f"        [{values}],")
        lines.append("    ],")
    lines.append("];")
    output.write_text("\n".join(lines) + "\n")
    return {
        "androidx_revision": REVISION,
        "pairs": names,
        "matched_cubics": [len(pair) for pair in curves],
        "matched_samples": [len(pair) for pair in points],
        "shape_scale": scale,
        "rotation_radius_upper_bound": radius,
        "maximum_progress": MAX_PROGRESS,
        "sampled_validation_poses": 111 * len(points),
        "generated_sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workdir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    args.workdir.mkdir(parents=True, exist_ok=True)
    names, shapes = run_morph(args.workdir.resolve())
    print(json.dumps(generate(names, shapes, args.output), indent=2))


if __name__ == "__main__":
    main()
