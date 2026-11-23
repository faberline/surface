struct Instance {
    @location(0) destination: vec4<f32>,
    @location(1) size: vec2<f32>,
    // Top-left, top-right, bottom-right, bottom-left.
    @location(2) corners: vec4<f32>,
    @location(3) source_uv: vec4<f32>,
};

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) size: vec2<f32>,
    @location(3) @interpolate(flat) corners: vec4<f32>,
};

@group(0) @binding(0) var image: texture_2d<f32>;

@vertex fn vs_main(instance: Instance, @builtin(vertex_index) index: u32) -> Fragment {
    let vertices = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
    );
    let corner = vertices[index];
    let position = mix(instance.destination.xy, instance.destination.zw, corner);
    var out: Fragment;
    out.position = vec4(position, 0.0, 1.0);
    out.local = corner * instance.size;
    out.uv = instance.source_uv.xy + corner * instance.source_uv.zw;
    out.size = instance.size;
    out.corners = instance.corners;
    return out;
}

fn premultiplied_texel(point: vec2<i32>, last: vec2<i32>) -> vec4<f32> {
    let color = textureLoad(image, clamp(point, vec2<i32>(0), last), 0);
    return vec4(color.rgb * color.a, color.a);
}

fn sample_image(uv: vec2<f32>) -> vec4<f32> {
    let size = textureDimensions(image);
    let point = uv * vec2<f32>(size) - vec2(0.5);
    let origin = vec2<i32>(floor(point));
    let weight = fract(point);
    let last = vec2<i32>(size) - vec2<i32>(1);
    // Interpolate premultiplied texels so transparent pixels cannot tint edges.
    let top = mix(
        premultiplied_texel(origin, last),
        premultiplied_texel(origin + vec2(1, 0), last),
        weight.x,
    );
    let bottom = mix(
        premultiplied_texel(origin + vec2(0, 1), last),
        premultiplied_texel(origin + vec2(1, 1), last),
        weight.x,
    );
    return mix(top, bottom, weight.y);
}

@fragment fn fs_main(input: Fragment) -> @location(0) vec4<f32> {
    let p = input.local - input.size * 0.5;
    let left = select(input.corners.x, input.corners.w, p.y > 0.0);
    let right = select(input.corners.y, input.corners.z, p.y > 0.0);
    let radius = min(select(left, right, p.x > 0.0), min(input.size.x, input.size.y) * 0.5);
    let q = abs(p) - (input.size * 0.5 - vec2(radius));
    let distance = length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - radius;
    let edge = max(fwidth(distance), 0.01);
    let coverage = 1.0 - smoothstep(-edge * 0.5, edge * 0.5, distance);
    return sample_image(input.uv) * coverage;
}
