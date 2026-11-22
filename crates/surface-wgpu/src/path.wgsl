struct PointBlock {
    pairs: array<vec4<f32>, 128>,
};

@group(0) @binding(0) var<uniform> path: PointBlock;

struct VertexInput {
    @location(0) clip_rect: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) point_count: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) color: vec4<f32>,
    @location(1) @interpolate(flat) point_count: u32,
};

@vertex fn vs_main(v: VertexInput, @builtin(vertex_index) index: u32) -> VertexOutput {
    let x = select(v.clip_rect.x, v.clip_rect.z, index == 1u || index >= 4u);
    let y = select(v.clip_rect.y, v.clip_rect.w, index >= 2u && index != 4u);
    return VertexOutput(vec4<f32>(x, y, 0.0, 1.0), v.color, v.point_count);
}

fn point(index: u32) -> vec2<f32> {
    let pair = path.pairs[index / 2u];
    return select(pair.xy, pair.zw, (index & 1u) != 0u);
}

fn fill_coverage(sample_at: vec2<f32>, point_count: u32) -> f32 {
    // A rotated grid within the device pixel gives edge coverage without
    // adding opacity where coincident edges cancel under the even-odd rule.
    let xs = vec4<f32>(sample_at.x) + vec4<f32>(-0.125, 0.375, -0.375, 0.125);
    let ys = vec4<f32>(sample_at.y) + vec4<f32>(-0.375, -0.125, 0.125, 0.375);
    var inside = vec4<bool>(false);
    var a = point(point_count - 1u);
    for (var i = 0u; i < point_count; i += 1u) {
        let b = point(i);
        let spans_y = (vec4<f32>(a.y) > ys) != (vec4<f32>(b.y) > ys);
        if any(spans_y) {
            // Work from the same endpoint even when an edge is reversed.
            let start = select(a, b, b.y < a.y);
            let end = select(b, a, b.y < a.y);
            let fraction = (ys - vec4<f32>(start.y)) / (end.y - start.y);
            let crossing_x = vec4<f32>(start.x) + (end.x - start.x) * fraction;
            inside = select(inside, inside != (xs < crossing_x), spans_y);
        }
        a = b;
    }
    return dot(select(vec4<f32>(0.0), vec4<f32>(1.0), inside), vec4<f32>(0.25));
}

@fragment fn fs_main(v: VertexOutput) -> @location(0) vec4<f32> {
    let coverage = fill_coverage(v.position.xy, v.point_count);
    return vec4<f32>(v.color.rgb, v.color.a * coverage);
}
