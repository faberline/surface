struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) local: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) color: vec4<f32>,
    // Corner radii ordered top-left, top-right, bottom-right, bottom-left.
    @location(4) radius: vec4<f32>,
    @location(5) stroke: f32,
};
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) radius: vec4<f32>,
    @location(4) stroke: f32,
};
@vertex fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position=vec4(v.position,0.0,1.0);
    out.local=v.local;out.size=v.size;out.color=v.color;out.radius=v.radius;out.stroke=v.stroke;
    return out;
}
// p is the position relative to the centre. Y grows downward.
fn corner_radius(p: vec2<f32>, r: vec4<f32>) -> f32 {
    if p.x < 0.0 {
        return select(r.x, r.w, p.y > 0.0);
    }
    return select(r.y, r.z, p.y > 0.0);
}
@fragment fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    let p=v.local-v.size*0.5;
    let radius=min(corner_radius(p,v.radius),min(v.size.x,v.size.y)*0.5);
    let q=abs(p)-(v.size*0.5-vec2(radius));
    let distance=length(max(q,vec2(0.0)))+min(max(q.x,q.y),0.0)-radius;
    let edge=max(fwidth(distance),0.01);
    var alpha=1.0-smoothstep(-edge*0.5,edge*0.5,distance);
    if v.stroke > 0.0 {
        // The inner edge is the same shape inset by the stroke width, so each
        // corner radius shrinks by the width and stops at a square corner.
        alpha=alpha-(1.0-smoothstep(-edge*0.5,edge*0.5,distance+v.stroke));
    }
    return vec4(v.color.rgb,v.color.a*alpha);
}
