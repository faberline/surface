struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) local: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) radius: f32,
    @location(5) stroke: f32,
};
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) radius: f32,
    @location(4) stroke: f32,
};
@vertex fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position=vec4(v.position,0.0,1.0);
    out.local=v.local;out.size=v.size;out.color=v.color;out.radius=v.radius;out.stroke=v.stroke;
    return out;
}
@fragment fn fs_main(v: VertexOut) -> @location(0) vec4<f32> {
    let radius=min(v.radius,min(v.size.x,v.size.y)*0.5);
    let q=abs(v.local-v.size*0.5)-(v.size*0.5-vec2(radius));
    let distance=length(max(q,vec2(0.0)))+min(max(q.x,q.y),0.0)-radius;
    let edge=max(fwidth(distance),0.01);
    var alpha=1.0-smoothstep(-edge*0.5,edge*0.5,distance);
    if v.stroke > 0.0 {
        alpha=alpha-(1.0-smoothstep(-edge*0.5,edge*0.5,distance+v.stroke));
    }
    return vec4(v.color.rgb,v.color.a*alpha);
}
