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

// ── Textured quad (W1 basemap + hillshade) ────────────────────────────────────────────────────
// Reuses the unit-quad + `QuadInstance{min,max,color}` streams; `color` is the tint [1,1,1,opacity]
// (opacity = satOpacity / hillshadeOpacity). group(1) carries the sampled texture + sampler.
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct TexVsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@vertex
fn vs_textured(in: VsIn) -> TexVsOut {
    let world = mix(in.inst_min, in.inst_max, in.unit);
    var out: TexVsOut;
    out.pos = u.mvp * vec4<f32>(world, 0.0, 1.0);
    // North-up: unit.y=1 (world maxY = north) → v=0 (texture top). Mirrors `lanes::corner_uv`.
    out.uv = vec2<f32>(in.unit.x, 1.0 - in.unit.y);
    out.tint = in.inst_color;
    return out;
}

@fragment
fn fs_textured(in: TexVsOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv) * in.tint;
}

// ── Polyline (W1 grid) ────────────────────────────────────────────────────────────────────────
// Per-vertex anchor-relative position + normalized RGBA (`lanes::LineVertex`); drawn as a
// `LineList`, alpha-blended (the grid colors carry alpha).
struct LineVsIn {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct LineVsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_line(in: LineVsIn) -> LineVsOut {
    var out: LineVsOut;
    out.pos = u.mvp * vec4<f32>(in.pos, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_line(in: LineVsOut) -> @location(0) vec4<f32> {
    return in.color;
}
