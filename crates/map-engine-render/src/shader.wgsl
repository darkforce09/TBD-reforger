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

// ── Forest density canopy (T-178) ──────────────────────────────────────────────────────────────
// Island TBDD tree counts packed as RGBA8 RG=u16 LE. `tint.a` = fill_alpha; `tint.r` = outline_on
// (0/1). Nearest via textureLoad — Linear would corrupt the packed count.
@fragment
fn fs_forest_density(in: TexVsOut) -> @location(0) vec4<f32> {
    let iso = 2.0;
    let fill_rgb = vec3<f32>(34.0 / 255.0, 120.0 / 255.0, 60.0 / 255.0);
    let outline_rgb = vec3<f32>(24.0 / 255.0, 90.0 / 255.0, 45.0 / 255.0);
    let outline_a = 230.0 / 255.0;
    let fill_alpha = in.tint.a;
    let outline_on = in.tint.r;

    let dims = vec2<i32>(textureDimensions(tex));
    let tc = vec2<i32>(
        clamp(i32(floor(in.uv.x * f32(dims.x))), 0, dims.x - 1),
        clamp(i32(floor(in.uv.y * f32(dims.y))), 0, dims.y - 1)
    );
    let t = textureLoad(tex, tc, 0);
    let count = round(t.r * 255.0) + round(t.g * 255.0) * 256.0;

    let inside = count >= iso;
    var out_rgb = fill_rgb;
    var out_a = 0.0;
    if inside {
        out_a = fill_alpha;
    }
    let edge = fwidth(select(0.0, 1.0, inside));
    if outline_on > 0.5 {
        let rim = smoothstep(0.0, 1.0, edge * 2.0);
        if rim > 0.05 {
            out_rgb = mix(out_rgb, outline_rgb, rim);
            out_a = max(out_a, outline_a * rim);
        }
    }
    if out_a < 0.001 {
        discard;
    }
    return vec4<f32>(out_rgb, out_a);
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

// ── World-building OBB fill (W3) ────────────────────────────────────────────────────────────────
// Instanced rotated quad. Stream 0 is the unit quad; stream 1 is `scene::BuildingInstance`
// {center, half, basis=(cos,sin), color}. The unit quad is scaled by `half`, rotated in the
// `obb.rs` frame (0° = +y north, clockwise-positive: rot(dx,dy) = [dx·c + dy·s, −dx·s + dy·c]),
// and placed at `center` (anchor-relative meters). `basis` is precomputed on the CPU from the same
// `rad = deg·PI/180` as `obb_corners`, so the fill and the outline ring agree. Alpha-blended.
struct BuildingVsIn {
    @location(0) unit: vec2<f32>,
    @location(1) center: vec2<f32>,
    @location(2) half: vec2<f32>,
    @location(3) basis: vec2<f32>,
    @location(4) color: vec4<f32>,
};

struct BuildingVsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_building(in: BuildingVsIn) -> BuildingVsOut {
    // unit {0,1}² → local corner {−half, +half} (matches obb_corners' corner order).
    let local = (in.unit * 2.0 - vec2<f32>(1.0, 1.0)) * in.half;
    let c = in.basis.x;
    let s = in.basis.y;
    let world = in.center + vec2<f32>(local.x * c + local.y * s, -local.x * s + local.y * c);
    var out: BuildingVsOut;
    out.pos = u.mvp * vec4<f32>(world, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_building(in: BuildingVsOut) -> @location(0) vec4<f32> {
    return in.color;
}

// ── Icon instanced (W5 glyph atlas) ─────────────────────────────────────────────────────────────
// Stream 0 = unit quad; stream 1 = IconInstance {pos, size, yaw_snorm16, glyph_u16, tint_u32}.
// group(2): atlas texture + sampler + UV table (N × vec4 = minUV.xy + maxUV.zw) — separate
// from basemap group(1) so the WGSL binding space stays unique.
// Size is already meters (min-px clamped on CPU). Yaw is screen-CCW degrees via snorm.
// W6: UV[N] + drag_delta + px_to_m (world glyphs: px_to_m=1, drag=0; slots: px size).
// N MUST equal Rust `scene::ATLAS_GLYPH_COUNT` (32); the map-engine-render shader-const test asserts
// this literal + the `min(in.glyph, 31u)` clamp below stay in sync with the constant.
struct IconUniforms {
    uv: array<vec4<f32>, 32>,
    drag_delta: vec2<f32>,
    px_to_m: f32,
    _pad: f32,
};

@group(2) @binding(0) var icon_tex: texture_2d<f32>;
@group(2) @binding(1) var icon_samp: sampler;
@group(2) @binding(2) var<uniform> icon_u: IconUniforms;

struct IconVsIn {
    @location(0) unit: vec2<f32>,
    @location(1) pos: vec2<f32>,
    @location(2) size: f32,
    @location(3) yaw: i32,
    @location(4) glyph: u32,
    @location(5) tint: u32,
};

struct IconVsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@vertex
fn vs_icon(in: IconVsIn) -> IconVsOut {
    // snorm16 → degrees: yaw/32767 * 180
    let deg = f32(in.yaw) / 32767.0 * 180.0;
    let rad = deg * 3.14159265358979323846 / 180.0;
    let c = cos(rad);
    let s = sin(rad);
    // size × px_to_m → world meters (glyphs: px_to_m=1; slots: 2^(-zoom))
    let size_m = in.size * icon_u.px_to_m;
    let local = (in.unit - vec2<f32>(0.5, 0.5)) * size_m;
    // CCW rotation + optional drag_delta (SlotDrag lane only)
    let world = in.pos + icon_u.drag_delta + vec2<f32>(local.x * c - local.y * s, local.x * s + local.y * c);
    let gi = min(in.glyph, 31u); // clamp to ATLAS_GLYPH_COUNT-1 (see IconUniforms note)
    let rect = icon_u.uv[gi];
    var out: IconVsOut;
    out.pos = u.mvp * vec4<f32>(world, 0.0, 1.0);
    // T-173 P10 — north-up: unit.y=1 is +y (top of screen), which must sample the TOP of the atlas
    // glyph (v0). Without the `1.0 - unit.y` flip (which the basemap and text lanes already apply)
    // every icon sampled upside-down — only visible on directional building badges; tree/prop/slot
    // art is ~symmetric so it read fine.
    out.uv = mix(rect.xy, rect.zw, vec2<f32>(in.unit.x, 1.0 - in.unit.y));
    let tr = f32(in.tint & 0xffu) / 255.0;
    let tg = f32((in.tint >> 8u) & 0xffu) / 255.0;
    let tb = f32((in.tint >> 16u) & 0xffu) / 255.0;
    let ta = f32((in.tint >> 24u) & 0xffu) / 255.0;
    out.tint = vec4<f32>(tr, tg, tb, ta);
    return out;
}

@fragment
fn fs_icon(in: IconVsOut) -> @location(0) vec4<f32> {
    let s = textureSample(icon_tex, icon_samp, in.uv);
    // Mask path: atlas alpha × tint (tintable glyphs are white-on-alpha).
    return vec4<f32>(s.rgb * in.tint.rgb, s.a * in.tint.a);
}

// ── T-152.7 ASCII text atlas (UV from glyph index; grid dims via uniform — T-152.13) ────────────
// Four f32s = 16 B (matches TEXT_UNIFORM_BYTES). Do NOT use vec3 pad — align-16 makes the struct 32 B.
// grid_cols/grid_rows are written at atlas upload from text_layout::TEXT_ATLAS_{COLS,ROWS} so the
// bake and this shader can never disagree on the cell grid.
struct TextUniforms {
    px_to_m: f32,
    grid_cols: f32,
    grid_rows: f32,
    _pad0: f32,
};

@group(2) @binding(0) var text_tex: texture_2d<f32>;
@group(2) @binding(1) var text_samp: sampler;
@group(2) @binding(2) var<uniform> text_u: TextUniforms;

@vertex
fn vs_text(in: IconVsIn) -> IconVsOut {
    let deg = f32(in.yaw) / 32767.0 * 180.0;
    let rad = deg * 3.14159265358979323846 / 180.0;
    let c = cos(rad);
    let s = sin(rad);
    let size_m = in.size * text_u.px_to_m;
    let local = (in.unit - vec2<f32>(0.5, 0.5)) * size_m;
    let world = in.pos + vec2<f32>(local.x * c - local.y * s, local.x * s + local.y * c);
    let cols = u32(text_u.grid_cols);
    let col = in.glyph % cols;
    let row = in.glyph / cols;
    let u0 = f32(col) / text_u.grid_cols;
    let v0 = f32(row) / text_u.grid_rows;
    let u1 = f32(col + 1u) / text_u.grid_cols;
    let v1 = f32(row + 1u) / text_u.grid_rows;
    var out: IconVsOut;
    out.pos = u.mvp * vec4<f32>(world, 0.0, 1.0);
    // T-152.12: atlas is authored y-down — world-top (unit.y=1) must sample the cell top (v0),
    // same convention as `vs_textured`. Oracle: `text_layout::glyph_cell_uv`.
    out.uv = mix(vec2<f32>(u0, v0), vec2<f32>(u1, v1), vec2<f32>(in.unit.x, 1.0 - in.unit.y));
    let tr = f32(in.tint & 0xffu) / 255.0;
    let tg = f32((in.tint >> 8u) & 0xffu) / 255.0;
    let tb = f32((in.tint >> 16u) & 0xffu) / 255.0;
    let ta = f32((in.tint >> 24u) & 0xffu) / 255.0;
    out.tint = vec4<f32>(tr, tg, tb, ta);
    return out;
}

@fragment
fn fs_text(in: IconVsOut) -> @location(0) vec4<f32> {
    let s = textureSample(text_tex, text_samp, in.uv);
    return vec4<f32>(s.rgb * in.tint.rgb, s.a * in.tint.a);
}

// ── T-151.8.1 WebGPU instance cull (VERTEX|STORAGE compaction) ───────────────────────────────
// 32 B storage record (std430-friendly). AABB rule matches `compute_cull::icon_intersects_frustum`.
struct IconStorage {
    pos: vec2<f32>,
    size: f32,
    yaw: i32,
    glyph: u32,
    tint: u32,
    _pad0: u32,
    _pad1: u32,
};

struct CullParams {
    frustum: vec4<f32>, // min_x, min_y, max_x, max_y (same space as IconStorage.pos)
    count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<storage, read> cull_in: array<IconStorage>;
@group(0) @binding(1) var<storage, read_write> cull_out: array<IconStorage>;
@group(0) @binding(2) var<storage, read_write> cull_counter: array<atomic<u32>>;
@group(0) @binding(3) var<uniform> cull_params: CullParams;

fn icon_in_frustum(pos: vec2<f32>, size: f32, f: vec4<f32>) -> bool {
    let half = max(size * 0.5, 0.0);
    let imin = pos - vec2<f32>(half, half);
    let imax = pos + vec2<f32>(half, half);
    return imax.x >= f.x && imin.x <= f.z && imax.y >= f.y && imin.y <= f.w;
}

@compute @workgroup_size(64)
fn cs_icon_cull(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= cull_params.count) {
        return;
    }
    let inst = cull_in[i];
    if (!icon_in_frustum(inst.pos, inst.size, cull_params.frustum)) {
        return;
    }
    let slot = atomicAdd(&cull_counter[0], 1u);
    cull_out[slot] = inst;
}
