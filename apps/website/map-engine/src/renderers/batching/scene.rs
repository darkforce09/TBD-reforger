//! Role: scene.
//! Position: `renderers/batching` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use bytemuck::{Pod, Zeroable};

/// Scene anchor in world meters — the Everon terrain center. Uploaded geometry is stored relative to this point so f32 coordinates stay small (≤ 6400 m ⇒ error ≪ 1 px at all zoom levels; bound derived in `OrthoCamera::wgpu_clip_matrix` docs).
pub const ANCHOR: [f64; 2] = [6400.0, 6400.0];

/// Instance-buffer pool unit: 2^21 instances × 32 B = 64 MiB per GPU buffer — legal by construction under WebGPU's *default* `maxBufferSize` (256 MiB) with 4× headroom, so no device-limit negotiation is ever load-bearing (plan §S4 chunked pool).
pub const CHUNK_CAPACITY: usize = 2_097_152;

/// Unit quad (triangle-strip order) expanded per instance in the vertex shader via `pos = mix(inst.min, inst.max, unit_uv)`. Culling is disabled in the pipeline, so winding is irrelevant.
pub const UNIT_QUAD: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

/// One axis-aligned colored quad instance (anchor-relative meters).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct QuadInstance {
    /// Anchor-relative [minX, minY] corner, meters.
    pub min: [f32; 2],

    /// Anchor-relative [maxX, maxY] corner, meters.
    pub max: [f32; 2],

    /// RGBA, linear 0..1 (rendered to a non-sRGB target — no transfer function).
    pub color: [f32; 4],
}

/// Building instance.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct BuildingInstance {
    /// Anchor-relative [x, y] center, meters (world minus [`ANCHOR`]).
    pub center: [f32; 2],

    /// Half-extents [hx, hy], meters (a size — NOT anchor-shifted).
    pub half: [f32; 2],

    /// `(cos(rad), sin(rad))`, `rad = deg·PI/180` — computed once (matching `obb::obb_corners`), so the fill quad and the outline ring coincide to f32 rounding.
    pub basis: [f32; 2],

    /// RGBA, linear 0..1 (`byte/255`; rendered to a non-sRGB target — no transfer function).
    pub color: [f32; 4],
}

/// Canonical atlas glyph count value.
pub const ATLAS_GLYPH_COUNT: usize = 32;

/// Icon instance.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct IconInstance {
    /// Anchor-relative [x, y] center, meters.
    pub pos: [f32; 2],

    /// Glyph size in meters (min-px already applied on CPU).
    pub size: f32,

    /// Screen CCW angle as snorm16 (`angle_deg/180 * 32767`).
    pub yaw: i16,

    /// Index into the 28-entry UV uniform table.
    pub glyph: u16,

    /// Packed RGBA8 (r | g<<8 | b<<16 | a<<24).
    pub tint: u32,
}

/// The two calibration instances (plan §S4 calibration scene), anchor-relative: - G: green quad, world [6300,6300]…[6500,6500] → relative [-100,-100]…[100,100] - R: red quad, world [6450,6450]…[6490,6490] → relative [50,50]…[90,90], drawn after G.
#[must_use]
pub fn calibration_instances() -> [QuadInstance; 2] {
    [
        QuadInstance {
            min: [-100.0, -100.0],
            max: [100.0, 100.0],
            color: [0.0, 1.0, 0.0, 1.0],
        },
        QuadInstance {
            min: [50.0, 50.0],
            max: [90.0, 90.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
    ]
}

struct Lcg(u32);

impl Lcg {
    fn new(seed: u64, chunk_idx: u32) -> Self {
        let folded = (seed as u32) ^ ((seed >> 32) as u32);
        Self(folded ^ chunk_idx.wrapping_mul(0x9E37_79B9))
    }

    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / 16_777_216.0
    }
}

/// Build one stress chunk of `count` deterministic quads: centers uniform over the Everon bounds (anchor-relative [-6400, 6400]²), half-sizes 1–10 m (2–20 m quads), opaque pseudo-random tint. Same `(seed, chunk_idx, count)` ⇒ bit-identical output, asserted by the native byte tests.
#[must_use]
pub fn stress_chunk(chunk_idx: u32, count: usize, seed: u64) -> Vec<QuadInstance> {
    let mut out = Vec::new();
    stress_chunk_into(chunk_idx, count, seed, &mut out);
    out
}

/// [`stress_chunk`] into a caller-owned staging `Vec` — the streaming-upload loop reuses one 64 MiB staging allocation across all chunks, so peak wasm heap is one chunk regardless of total instance count (plan §20M residency).
pub fn stress_chunk_into(chunk_idx: u32, count: usize, seed: u64, out: &mut Vec<QuadInstance>) {
    let mut rng = Lcg::new(seed, chunk_idx);
    out.clear();
    out.reserve(count);
    for _ in 0..count {
        let cx = rng.unit() * 12_800.0 - 6_400.0;
        let cy = rng.unit() * 12_800.0 - 6_400.0;
        let hs = 1.0 + rng.unit() * 9.0;
        let r = 0.25 + rng.unit() * 0.75;
        let g = 0.25 + rng.unit() * 0.75;
        let b = 0.25 + rng.unit() * 0.75;
        out.push(QuadInstance {
            min: [cx - hs, cy - hs],
            max: [cx + hs, cy + hs],
            color: [r, g, b, 1.0],
        });
    }
}

/// Canonical map-glyph id for a briefing marker — the drawable shape the web map renders, and the index into the widened slot atlas ([`build_marker_slot_atlas`]).
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerGlyph {
    /// Hollow ring (atlas cell 0 — the slot-ring shape). `circle` / `area` / `zone` / `ao`.
    Ring = 0,

    /// Solid disc (atlas cell 1 — the FALLBACK, matching the mod's `FALLBACK_ICON = DOT`). `dot` / `dot2` / `point` / `mark` / `marker` and every unrecognised alias.
    Disc = 1,

    /// Filled square. Objective family: `objective_marker(2)` / `objective` / `obj` / `target` / `task`.
    Square = 2,

    /// Filled diamond. Point-of-interest family: `point_of_interest(2)` / `poi` / `intel` / `contact`.
    Diamond = 3,

    /// Upward triangle. Attack family: `attack` / `assault` / `capture` / `seize` / `advance` / `ambush(2)`.
    TriangleUp = 4,

    /// Downward triangle. Defend family: `defend(2)` / `hold` / `garrison` / `fallback`.
    TriangleDown = 5,

    /// Plus / medical cross. Cross family: `cross(2)` / `medical` / `medic` / `aid` / `casevac` / `medevac`.
    Cross = 6,

    /// Diagonal X. Destroy family: `destroy(2)` / `demolish` / `demo` / `sabotage`.
    Ex = 7,

    /// Pennant flag. Flag family: `flag(2)` / `rally` / `rally_point` / `base` / `hq` / `spawn`.
    Flag = 8,

    /// Chevron. Waypoint family: `waypoint(2)` / `move` / `wp` / `route` / `phase_line`.
    Chevron = 9,

    /// Concentric target (ring + centre dot). Observation family: `observation_post(2)` / `op` / `observe` / `overwatch` / `recon`.
    Target = 10,
}

/// Number of distinct canonical marker glyphs — the atlas cell count [`build_marker_slot_atlas`] emits, and the highest [`MarkerGlyph`] discriminant + 1. Well under [`ATLAS_GLYPH_COUNT`] (32).
pub const MARKER_GLYPH_COUNT: usize = 11;

#[must_use]
fn normalise_alias(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

/// Unknown / empty aliases fall back to [`MarkerGlyph::Disc`] — the same graceful downgrade the mod's `Resolve()` makes (`FALLBACK_ICON = DOT`), so a document the schema enum did not foresee still draws a marker rather than nothing. The full authored vocabulary is the 64-key enum in `mission.schema.json` `$defs/marker.icon`; every one of those keys is covered here.
#[must_use]
pub fn marker_glyph_for_alias(icon: &str) -> MarkerGlyph {
    match normalise_alias(icon).as_str() {
        "circle" | "circle2" | "area" | "zone" | "ao" => MarkerGlyph::Ring,

        "objective_marker" | "objective_marker2" | "objective" | "obj" | "target" | "task" => {
            MarkerGlyph::Square
        }

        "point_of_interest" | "point_of_interest2" | "poi" | "intel" | "contact" => {
            MarkerGlyph::Diamond
        }

        "observation_post" | "observation_post2" | "op" | "observe" | "overwatch" | "recon" => {
            MarkerGlyph::Target
        }

        "attack" | "assault" | "capture" | "seize" | "advance" | "ambush" | "ambush2" => {
            MarkerGlyph::TriangleUp
        }

        "defend" | "defend2" | "hold" | "garrison" | "fallback" => MarkerGlyph::TriangleDown,

        "destroy" | "destroy2" | "demolish" | "demo" | "sabotage" => MarkerGlyph::Ex,

        "waypoint" | "waypoint2" | "move" | "wp" | "route" | "phase_line" => MarkerGlyph::Chevron,

        "flag" | "flag2" | "rally" | "rally_point" | "base" | "hq" | "spawn" => MarkerGlyph::Flag,

        "cross" | "cross2" | "medical" | "medic" | "aid" | "casevac" | "medevac" => {
            MarkerGlyph::Cross
        }

        _ => MarkerGlyph::Disc,
    }
}

#[must_use]
fn edge_cov(d: f64) -> f64 {
    (d + 0.5).clamp(0.0, 1.0)
}

const STROKE_UNIT_PX: f64 = 16.0 / 3.0;

const STROKE_2U_HALF: f64 = STROKE_UNIT_PX;

const STROKE_14U_HALF: f64 = 0.7 * STROKE_UNIT_PX;

const FLAG_SCALE: f64 = 3.2;

const FLAG_DX: f64 = 8.6;

const FLAG_DY: f64 = 3.55;

#[must_use]
fn stroke_cov(dx: f64, dy: f64, ax: f64, ay: f64, bx: f64, by: f64, hw: f64) -> f64 {
    let (vx, vy) = (bx - ax, by - ay);
    let (wx, wy) = (dx - ax, dy - ay);
    let len2 = vx * vx + vy * vy;
    let t = if len2 <= f64::EPSILON {
        0.0
    } else {
        ((wx * vx + wy * vy) / len2).clamp(0.0, 1.0)
    };
    let (cx, cy) = (ax + vx * t, ay + vy * t);
    let (qx, qy) = (dx - cx, dy - cy);
    edge_cov(hw - (qx * qx + qy * qy).sqrt())
}

#[must_use]
fn marker_glyph_coverage(g: u16, px: f64, py: f64) -> f64 {
    let dx = px + 0.5 - 32.0;
    let dy = py + 0.5 - 32.0;
    let d = (dx * dx + dy * dy).sqrt();

    let cov = |dist: f64, r: f64| (r + 0.5 - dist).clamp(0.0, 1.0);
    match g {
        0 => cov(d, 24.0) - cov(d, 10.0),
        1 => cov(d, 26.0),

        2 => edge_cov(22.0 - dx.abs()).min(edge_cov(22.0 - dy.abs())),

        3 => edge_cov(26.0 - (dx.abs() + dy.abs())),

        4 => {
            let apex = edge_cov(dy + 22.0);
            let base = edge_cov(22.0 - dy);
            let sides = edge_cov((dy + 22.0) * (24.0 / 44.0) - dx.abs());
            apex.min(base).min(sides)
        }

        5 => {
            let apex = edge_cov(22.0 - dy);
            let base = edge_cov(dy + 22.0);
            let sides = edge_cov((22.0 - dy) * (24.0 / 44.0) - dx.abs());
            apex.min(base).min(sides)
        }

        6 => {
            let vert = edge_cov(8.0 - dx.abs()).min(edge_cov(24.0 - dy.abs()));
            let horiz = edge_cov(8.0 - dy.abs()).min(edge_cov(24.0 - dx.abs()));
            vert.max(horiz)
        }

        7 => {
            let hw = STROKE_2U_HALF;
            let nw_se = stroke_cov(dx, dy, -18.0, -18.0, 18.0, 18.0, hw);
            let ne_sw = stroke_cov(dx, dy, 18.0, -18.0, -18.0, 18.0, hw);
            nw_se.max(ne_sw)
        }

        8 => {
            let p = |x: f64, y: f64| {
                (
                    (x - 8.0) * FLAG_SCALE + FLAG_DX,
                    (y - 8.0) * FLAG_SCALE + FLAG_DY,
                )
            };
            let (staff_x, staff_top) = p(4.0, 2.0);
            let (_, staff_bot) = p(4.0, 14.0);
            let staff = stroke_cov(
                dx,
                dy,
                staff_x,
                staff_top,
                staff_x,
                staff_bot,
                STROKE_14U_HALF,
            );

            let (_, hoist_top) = p(4.0, 2.5);
            let (tip_x, tip_y) = p(13.0, 4.5);
            let (_, hoist_bot) = p(4.0, 7.5);
            let run = tip_x - staff_x;
            let (drop_top, rise_bot) = (tip_y - hoist_top, tip_y - hoist_bot);
            let upper = ((dy - hoist_top) * run - (dx - staff_x) * drop_top)
                / (run * run + drop_top * drop_top).sqrt();
            let lower = ((hoist_bot - dy) * run + (dx - staff_x) * rise_bot)
                / (run * run + rise_bot * rise_bot).sqrt();
            let pennant = edge_cov(dx - staff_x)
                .min(edge_cov(upper))
                .min(edge_cov(lower));
            staff.max(pennant)
        }

        9 => {
            let hw = STROKE_2U_HALF;
            let left = stroke_cov(dx, dy, -20.0, 10.5, 0.0, -13.5, hw);
            let right = stroke_cov(dx, dy, 0.0, -13.5, 20.0, 10.5, hw);
            left.max(right)
        }

        10 => (cov(d, 24.0) - cov(d, 16.0)).max(cov(d, 7.0)),
        _ => 0.0,
    }
}

/// Widened slot atlas: [`MARKER_GLYPH_COUNT`] cells of 64 px laid out horizontally, white-on-alpha. Returned as `(rgba, width, height, uv)` for [`crate::engine::RenderEngine::ensure_slot_atlas`]; `uv` is the flat `[minU,minV,maxU,maxV]·N` table, cell `i` at `[i/N, 0, (i+1)/N, 1]`.
#[must_use]
pub fn build_marker_slot_atlas() -> (Vec<u8>, u32, u32, Vec<f32>) {
    const CELL: usize = 64;
    let n = MARKER_GLYPH_COUNT;
    let w = CELL * n;
    let h = CELL;
    let mut rgba = vec![0u8; w * h * 4];
    for cell in 0..n {
        let cx0 = cell * CELL;
        for y in 0..h {
            for x in 0..CELL {
                #[allow(clippy::cast_possible_truncation)]
                let a = marker_glyph_coverage(cell as u16, x as f64, y as f64);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let a8 = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
                let i = (y * w + (cx0 + x)) * 4;
                rgba[i..i + 4].copy_from_slice(&[255, 255, 255, a8]);
            }
        }
    }
    let mut uv = Vec::with_capacity(n * 4);
    #[allow(clippy::cast_precision_loss)]
    for cell in 0..n {
        let u0 = cell as f32 / n as f32;
        let u1 = (cell + 1) as f32 / n as f32;
        uv.extend_from_slice(&[u0, 0.0, u1, 1.0]);
    }
    #[allow(clippy::cast_possible_truncation)]
    (rgba, w as u32, h as u32, uv)
}

/// Pack briefing-marker CAPTION text into 20 B text-atlas icon instances (WORLD meters), for the EXISTING text pipeline — the same `text_layout` path place-name labels use, never a second one.
#[must_use]
pub fn pack_marker_caption_bytes(xy: &[f32], captions: &[String], deck_zoom: f64) -> Vec<u8> {
    let n = xy.len() / 2;
    let char_m = crate::renderers::text::metrics::text_char_meters(deck_zoom);
    let advance = char_m * crate::renderers::text::metrics::TEXT_GLYPH_ADVANCE_RATIO;

    let px_m = crate::symbology::instances::symbols::px_to_m_at_zoom(deck_zoom);
    let gap_m = px_m * 14.0;
    let mut glyphs: Vec<crate::renderers::text::metrics::TextGlyphInstance> = Vec::new();
    for i in 0..n {
        let Some(text) = captions.get(i) else {
            continue;
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mx = xy[i * 2];
        let my = xy[i * 2 + 1];
        let x_start = mx + gap_m + advance * 0.5;
        for (col, ch) in trimmed.chars().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let gx = x_start + col as f32 * advance;
            glyphs.push(crate::renderers::text::metrics::TextGlyphInstance {
                x: gx,
                y: my,
                half_m: char_m * 0.5,
                glyph: crate::renderers::text::metrics::glyph_index_for_char(ch),
            });
        }
    }
    crate::renderers::text::packing::pack_text_icon_bytes(&glyphs, deck_zoom)
}

#[cfg(test)]
#[path = "tests/scene_tests.rs"]
mod tests;
