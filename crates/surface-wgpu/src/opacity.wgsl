struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) opacity: f32,
};

@group(0) @binding(0) var group_image: texture_2d<f32>;

@vertex fn vs_main(
    @location(0) opacity: f32,
    @builtin(vertex_index) index: u32,
) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0),
    );
    return VertexOutput(vec4(positions[index], 0.0, 1.0), opacity);
}

@fragment fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    // Rendering onto transparent black already premultiplied the group's
    // colors. Sample the identical device pixel and scale every channel once.
    return textureLoad(group_image, vec2<i32>(vertex.position.xy), 0) * vertex.opacity;
}
