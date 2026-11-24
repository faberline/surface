struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) source: vec4<f32>,
    @location(1) @interpolate(flat) destination: vec4<f32>,
    @location(2) @interpolate(flat) clip: vec4<f32>,
    @location(3) @interpolate(flat) radius_opacity: vec2<f32>,
};

@group(0) @binding(0) var group_image: texture_2d<f32>;

@vertex fn vs_main(
    @location(0) source: vec4<f32>,
    @location(1) destination: vec4<f32>,
    @location(2) clip: vec4<f32>,
    @location(3) radius_opacity: vec2<f32>,
    @builtin(vertex_index) index: u32,
) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0),
    );
    return VertexOutput(
        vec4(positions[index], 0.0, 1.0), source, destination, clip, radius_opacity,
    );
}

fn rounded_distance(pixel: vec2<f32>, bounds: vec4<f32>, radius: f32) -> f32 {
    let half_bounds = bounds.zw * 0.5;
    let relative = abs(pixel - (bounds.xy + half_bounds)) - half_bounds + vec2(radius);
    return length(max(relative, vec2(0.0))) + min(max(relative.x, relative.y), 0.0) - radius;
}

fn load_pixel(tap: vec2<f32>, texture_extent: vec2<f32>) -> vec4<f32> {
    if any(tap < vec2(0.0)) || any(tap >= texture_extent) {
        return vec4(0.0);
    }
    return textureLoad(group_image, vec2<i32>(tap), 0);
}

fn sample_cropped(source: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    let extent = vec2<f32>(textureDimensions(group_image));
    let centered = source.xy + uv * source.zw - vec2(0.5);
    let base = floor(centered);
    let weight = fract(centered);
    // Pixel centers that lie within the crop. Use the containing pixel for
    // any subpixel crop with no center of its own.
    let low = ceil(source.xy - vec2(0.5));
    let high = ceil(source.xy + source.zw - vec2(0.5)) - vec2(1.0);
    let center = floor(source.xy + source.zw * 0.5);
    let first = select(center, low, high >= low);
    let last = select(center, high, high >= low);
    let left_top = clamp(base, first, last);
    let right_bottom = clamp(base + vec2(1.0), first, last);
    let top = mix(
        load_pixel(left_top, extent),
        load_pixel(vec2(right_bottom.x, left_top.y), extent),
        weight.x,
    );
    let bottom = mix(
        load_pixel(vec2(left_top.x, right_bottom.y), extent),
        load_pixel(right_bottom, extent),
        weight.x,
    );
    return mix(top, bottom, weight.y);
}

@fragment fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vertex.position.xy;
    let distance = max(
        rounded_distance(pixel, vertex.destination, 0.0),
        rounded_distance(pixel, vertex.clip, vertex.radius_opacity.x),
    );
    let mask = 1.0 - smoothstep(-0.5, 0.5, distance);
    if mask == 0.0 {
        return vec4(0.0);
    }
    let uv = clamp((pixel - vertex.destination.xy) / vertex.destination.zw, vec2(0.0), vec2(1.0));
    // The intermediate already contains premultiplied colors. Filter samples
    // in that space and apply the final coverage and opacity only once.
    return sample_cropped(vertex.source, uv) * (mask * vertex.radius_opacity.y);
}
