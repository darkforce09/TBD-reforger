// One instanced quad pipeline — the scalability spine (plan §S4). Stream 0 is a unit quad
// (triangle strip); stream 1 is per-instance {min, max, color}; the vertex shader expands
// `world = mix(min, max, unit)`. Positions are anchor-relative meters; `u.mvp` is
// OrthoCamera::wgpu_clip_matrix(anchor) — Z01·VP·T(anchor), f64-composed, f32-cast, WebGPU
// clip conventions on both backends (naga handles the GL depth remap).

struct Uniforms {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsIn {
    @location(0) unit: vec2<f32>,
    @location(1) inst_min: vec2<f32>,
    @location(2) inst_max: vec2<f32>,
    @location(3) inst_color: vec4<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    let world = mix(in.inst_min, in.inst_max, in.unit);
    var out: VsOut;
    out.pos = u.mvp * vec4<f32>(world, 0.0, 1.0);
    out.color = in.inst_color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
