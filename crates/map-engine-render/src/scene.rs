//! Pure instance-data builders for the render engine — no wgpu/web types, so this module
//! compiles natively and its byte-level tests run under plain `cargo test` (plan §S4b: the
//! exact bytes the GPU upload receives are asserted, closing the "did we upload what we
//! think" gap).
//!
//! Coordinate contract: instance coordinates are **anchor-relative meters** (world minus
//! [`ANCHOR`]), stored f32 once; the per-frame f64 view-projection matrix carries the
//! `target − anchor` translation (`OrthoCamera::wgpu_clip_matrix`). See plan §20M
//! feasibility — anchor rule.

use bytemuck::{Pod, Zeroable};

/// Scene anchor in world meters — the Everon terrain center. Uploaded geometry is stored
/// relative to this point so f32 coordinates stay small (≤ 6400 m ⇒ error ≪ 1 px at all
/// zoom levels; bound derived in `OrthoCamera::wgpu_clip_matrix` docs).
pub const ANCHOR: [f64; 2] = [6400.0, 6400.0];

/// Instance-buffer pool unit: 2^21 instances × 32 B = 64 MiB per GPU buffer — legal by
/// construction under WebGPU's *default* `maxBufferSize` (256 MiB) with 4× headroom, so no
/// device-limit negotiation is ever load-bearing (plan §S4 chunked pool).
pub const CHUNK_CAPACITY: usize = 2_097_152;

/// Unit quad (triangle-strip order) expanded per instance in the vertex shader via
/// `pos = mix(inst.min, inst.max, unit_uv)`. Culling is disabled in the pipeline, so
/// winding is irrelevant.
pub const UNIT_QUAD: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

/// One axis-aligned colored quad instance (anchor-relative meters).
///
/// 32 B — deliberately *heavier* than the pinned ≤ 20 B production icon layout, so every
/// stress measurement is a conservative lower bound on production throughput (plan §S4d).
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

/// One rotated building OBB fill instance (T-151.3 W3). Drawn by `vs_building`: the unit quad is
/// scaled by `half`, rotated by `basis = (cos, sin)` in the `obb.rs` frame (0° = +y north,
/// clockwise-positive), and translated to `center` (anchor-relative meters). 40 B — a NEW struct;
/// [`QuadInstance`] stays 32 B (the stress/calibration spine, unchanged).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct BuildingInstance {
    /// Anchor-relative [x, y] center, meters (world minus [`ANCHOR`]).
    pub center: [f32; 2],
    /// Half-extents [hx, hy], meters (a size — NOT anchor-shifted).
    pub half: [f32; 2],
    /// `(cos(rad), sin(rad))`, `rad = deg·PI/180` — computed once (matching `obb::obb_corners`), so
    /// the fill quad and the outline ring coincide to f32 rounding.
    pub basis: [f32; 2],
    /// RGBA, linear 0..1 (`byte/255`; rendered to a non-sRGB target — no transfer function).
    pub color: [f32; 4],
}

/// Icon-uniform UV-table capacity — the max glyphs the atlas may hold. Headroom above the current
/// `world-glyphs.json` count (29) so the atlas can grow without a coordinated engine/shader change.
/// **Single source of truth:** the shader UV array size (`shader.wgsl` `array<vec4<f32>, 32>`), the
/// icon uniform byte layout (`engine.rs` `ICON_UV_BYTES`/`ICON_UNIFORM_BYTES`/offsets), the TS loader
/// (`atlas_glyph_count()` wasm export), and the CI guards all key off this constant. If you change it,
/// the `map-engine-render` shader-const test and the `map-engine-core` atlas-count test enforce that
/// the shader literal and the atlas stay in sync (they fail loudly otherwise).
pub const ATLAS_GLYPH_COUNT: usize = 32;

/// One icon glyph instance (T-151.5 W5). Production layout ≤ 20 B:
/// pos 2×f32 + size f32 + yaw snorm16 + glyph u16 + tint u32.
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

/// The two calibration instances (plan §S4 calibration scene), anchor-relative:
/// - G: green quad, world [6300,6300]…[6500,6500] → relative [-100,-100]…[100,100]
/// - R: red quad, world [6450,6450]…[6490,6490] → relative [50,50]…[90,90], drawn after G
///
/// At the fixed probe camera (800×600, zoom 0, target = ANCHOR) every edge lands on an
/// integer pixel coordinate (G: x∈[300,500], y∈[200,400]; R: x∈[450,490], y∈[210,250]),
/// which is what makes the readback probes byte-exact with zero rasterization-rule
/// dependence (plan §S4 margin argument).
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

/// Deterministic 32-bit LCG (the `meters.parity.test.ts` constants: `s*1103515245+12345`),
/// seeded per chunk so any chunk is independently regenerable — the streaming-upload loop
/// fills one 64 MiB staging buffer per chunk without holding N instances in wasm memory.
struct Lcg(u32);

impl Lcg {
    fn new(seed: u64, chunk_idx: u32) -> Self {
        // Fold the u64 seed and de-correlate chunks with the golden-ratio Weyl constant.
        let folded = (seed as u32) ^ ((seed >> 32) as u32);
        Self(folded ^ chunk_idx.wrapping_mul(0x9E37_79B9))
    }

    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.0
    }

    /// Uniform in [0, 1) with 24-bit resolution — exact in f32.
    fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / 16_777_216.0
    }
}

/// Build one stress chunk of `count` deterministic quads: centers uniform over the Everon
/// bounds (anchor-relative [-6400, 6400]²), half-sizes 1–10 m (2–20 m quads), opaque
/// pseudo-random tint. Same `(seed, chunk_idx, count)` ⇒ bit-identical output, asserted by
/// the native byte tests.
#[must_use]
pub fn stress_chunk(chunk_idx: u32, count: usize, seed: u64) -> Vec<QuadInstance> {
    let mut out = Vec::new();
    stress_chunk_into(chunk_idx, count, seed, &mut out);
    out
}

/// [`stress_chunk`] into a caller-owned staging `Vec` — the streaming-upload loop reuses one
/// 64 MiB staging allocation across all chunks, so peak wasm heap is one chunk regardless of
/// total instance count (plan §20M residency).
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

// ── T-790 briefing-marker glyphs + captions ──────────────────────────────────────────────────────
//
// F-03 write-half: a placed marker must draw its ICON SHAPE (not one pale disc for every icon) and
// its CAPTION text on the map. The lane already exists (`MissionMarkers`, fed by T-760); this module
// supplies the three PURE pieces the wasm-only `engine::markers_bind` composes:
//   1. `marker_glyph_for_alias` — the 64-alias → canonical map-glyph table (the T-806 DELIVERABLE).
//   2. `build_marker_slot_atlas` — the shared slot atlas WIDENED with one drawable cell per canonical
//      glyph. Cells 0 (ring) and 1 (disc) are byte-identical to `slots_gpu::build_slot_atlas`, so the
//      slot / vehicle / comment lanes that also bind this atlas are unaffected.
//   3. `pack_marker_caption_bytes` — caption glyph instances for the EXISTING text pipeline (reused,
//      not a second one), placed beside each marker in world meters.
// All three are native + unit-tested here; `--all-features` is not required for this crate's tests.

/// Canonical map-glyph id for a briefing marker — the drawable shape the web map renders, and the
/// index into the widened slot atlas ([`build_marker_slot_atlas`]).
///
/// **This is the T-806 deliverable.** The marker-style PICKER (T-806) and unit symbology (T-808) must
/// map the authored `icon` alias onto exactly this set — call [`marker_glyph_for_alias`] rather than
/// re-deriving the mapping, so the picker's preview and the map's render can never disagree.
///
/// The vocabulary and the folding mirror the mod's authoritative table
/// `TBD_MarkerIcons.EnsureAliases` (`apps/mod/.../Markers/TBD_MarkerIcons.c`), which collapses the 64
/// `Register()` aliases onto ~13 `SCR_EScenarioFrameworkMarkerCustom` families. The web map does not
/// have those in-game imageset quads, so each mod family maps to a distinct procedural SHAPE here; the
/// GROUPING (which aliases share a glyph) is the contract shared with the mod, not the pixels.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerGlyph {
    /// Hollow ring (atlas cell 0 — the slot-ring shape). `circle` / `area` / `zone` / `ao`.
    Ring = 0,
    /// Solid disc (atlas cell 1 — the FALLBACK, matching the mod's `FALLBACK_ICON = DOT`).
    /// `dot` / `dot2` / `point` / `mark` / `marker` and every unrecognised alias.
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
    /// Concentric target (ring + centre dot). Observation family: `observation_post(2)` / `op` /
    /// `observe` / `overwatch` / `recon`.
    Target = 10,
}

/// Number of distinct canonical marker glyphs — the atlas cell count [`build_marker_slot_atlas`]
/// emits, and the highest [`MarkerGlyph`] discriminant + 1. Well under [`ATLAS_GLYPH_COUNT`] (32).
pub const MARKER_GLYPH_COUNT: usize = 11;

/// Normalise an authored alias exactly as the mod's `TBD_MarkerIcons.Normalise` does: trim,
/// lowercase, and fold `-`/space to `_`. This is what COLLAPSES the case-duplicates the T-806 trap
/// calls out (`Waypoint` and `waypoint` both normalise to `waypoint`), so the table below is keyed on
/// the normalised form and a mixed-case or spaced alias can never miss.
#[must_use]
fn normalise_alias(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

/// Map an authored marker `icon` alias to its canonical [`MarkerGlyph`] (the T-806 mapping).
///
/// Unknown / empty aliases fall back to [`MarkerGlyph::Disc`] — the same graceful downgrade the mod's
/// `Resolve()` makes (`FALLBACK_ICON = DOT`), so a document the schema enum did not foresee still
/// draws a marker rather than nothing. The full authored vocabulary is the 64-key enum in
/// `mission.schema.json` `$defs/marker.icon`; every one of those keys is covered here.
#[must_use]
pub fn marker_glyph_for_alias(icon: &str) -> MarkerGlyph {
    match normalise_alias(icon).as_str() {
        // circle / area — hollow ring
        "circle" | "circle2" | "area" | "zone" | "ao" => MarkerGlyph::Ring,
        // objective — square
        "objective_marker" | "objective_marker2" | "objective" | "obj" | "target" | "task" => {
            MarkerGlyph::Square
        }
        // point of interest — diamond
        "point_of_interest" | "point_of_interest2" | "poi" | "intel" | "contact" => {
            MarkerGlyph::Diamond
        }
        // observation post — concentric target
        "observation_post" | "observation_post2" | "op" | "observe" | "overwatch" | "recon" => {
            MarkerGlyph::Target
        }
        // attack / ambush — up triangle
        "attack" | "assault" | "capture" | "seize" | "advance" | "ambush" | "ambush2" => {
            MarkerGlyph::TriangleUp
        }
        // defend — down triangle
        "defend" | "defend2" | "hold" | "garrison" | "fallback" => MarkerGlyph::TriangleDown,
        // destroy — X
        "destroy" | "destroy2" | "demolish" | "demo" | "sabotage" => MarkerGlyph::Ex,
        // waypoint — chevron
        "waypoint" | "waypoint2" | "move" | "wp" | "route" | "phase_line" => MarkerGlyph::Chevron,
        // flag / rally — pennant
        "flag" | "flag2" | "rally" | "rally_point" | "base" | "hq" | "spawn" => MarkerGlyph::Flag,
        // medical — plus cross
        "cross" | "cross2" | "medical" | "medic" | "aid" | "casevac" | "medevac" => {
            MarkerGlyph::Cross
        }
        // dot family + every unknown — solid disc (mod FALLBACK_ICON = DOT)
        _ => MarkerGlyph::Disc,
    }
}

/// Anti-aliased coverage of a half-plane / disc edge: 1 inside, 0 outside, one-pixel linear ramp
/// across the boundary at signed distance `d` (positive = inside).
#[must_use]
fn edge_cov(d: f64) -> f64 {
    (d + 0.5).clamp(0.0, 1.0)
}

/// Straight-alpha coverage for canonical glyph `g` at cell-local pixel centre `(px, py)` in a 64 px
/// cell whose centre is (32, 32). White-on-alpha (the tint multiplies), matching the slot atlas.
///
/// Cells 0 (ring) and 1 (disc) reproduce `slots_gpu::build_slot_atlas` EXACTLY (ring outer r24 inner
/// r10; disc r26) — `marker_atlas_cells_0_and_1_match_slot_atlas` pins the byte match so the shared
/// slot / vehicle / comment lanes are never perturbed by the widening.
#[must_use]
fn marker_glyph_coverage(g: u16, px: f64, py: f64) -> f64 {
    let dx = px + 0.5 - 32.0;
    let dy = py + 0.5 - 32.0;
    let d = (dx * dx + dy * dy).sqrt();
    // `cov(dist, r)` — inside a disc of radius r (the slot-atlas convention: `(r + 0.5 - d)`).
    let cov = |dist: f64, r: f64| (r + 0.5 - dist).clamp(0.0, 1.0);
    match g {
        // 0 RING / 1 DISC — byte-identical to slots_gpu::build_slot_atlas.
        0 => cov(d, 24.0) - cov(d, 10.0),
        1 => cov(d, 26.0),
        // 2 SQUARE — filled box, half-extent 22.
        2 => edge_cov(22.0 - dx.abs()).min(edge_cov(22.0 - dy.abs())),
        // 3 DIAMOND — |dx|+|dy| ≤ 26.
        3 => edge_cov(26.0 - (dx.abs() + dy.abs())),
        // 4 TRIANGLE UP — apex at top (dy≈-24), base at bottom; three half-plane edges.
        4 => {
            let base = edge_cov(dy + 22.0); // above the base line dy = -22
            // left edge: dx >= slope*(dy) ... use the two slanted sides meeting at the top apex.
            let left = edge_cov((dy + 22.0) + 1.7 * dx); // dx negative allowed
            let right = edge_cov((dy + 22.0) - 1.7 * dx);
            base.min(left).min(right)
        }
        // 5 TRIANGLE DOWN — mirror of 4.
        5 => {
            let base = edge_cov(22.0 - dy);
            let left = edge_cov((22.0 - dy) + 1.7 * dx);
            let right = edge_cov((22.0 - dy) - 1.7 * dx);
            base.min(left).min(right)
        }
        // 6 CROSS (plus) — union of a vertical and a horizontal bar, half-thickness 8, half-len 24.
        6 => {
            let vert = edge_cov(8.0 - dx.abs()).min(edge_cov(24.0 - dy.abs()));
            let horiz = edge_cov(8.0 - dy.abs()).min(edge_cov(24.0 - dx.abs()));
            vert.max(horiz)
        }
        // 7 X — union of the two diagonals (rotate the plus 45°), half-thickness 8.
        7 => {
            let u = (dx + dy) * std::f64::consts::FRAC_1_SQRT_2;
            let v = (dx - dy) * std::f64::consts::FRAC_1_SQRT_2;
            let a = edge_cov(8.0 - u.abs()).min(edge_cov(24.0 - v.abs()));
            let b = edge_cov(8.0 - v.abs()).min(edge_cov(24.0 - u.abs()));
            a.max(b)
        }
        // 8 FLAG — a pole (left) + a filled pennant triangle to its right.
        8 => {
            let pole = edge_cov(3.0 - (dx + 16.0).abs()).min(edge_cov(24.0 - dy.abs()));
            // pennant: x in [-13, 20], upper triangle tapering rightward, centred vertically high.
            let fx = dx + 13.0; // 0 at pole side
            let in_x = edge_cov(fx).min(edge_cov(30.0 - fx));
            let taper = 16.0 - fx * 0.5; // half-height shrinks with x
            let in_y = edge_cov(taper - (dy + 8.0).abs());
            pole.max(in_x.min(in_y))
        }
        // 9 CHEVRON — two thick strokes forming a ">"-rotated up arrow (apex at top).
        9 => {
            let arm_a = edge_cov(7.0 - ((dy + 22.0) + 1.7 * dx).abs());
            let arm_b = edge_cov(7.0 - ((dy + 22.0) - 1.7 * dx).abs());
            let span = edge_cov(dy + 26.0).min(edge_cov(6.0 - dy));
            arm_a.max(arm_b).min(span)
        }
        // 10 TARGET — outer ring (r24 inner r16) plus a solid centre dot (r7).
        10 => (cov(d, 24.0) - cov(d, 16.0)).max(cov(d, 7.0)),
        _ => 0.0,
    }
}

/// Widened slot atlas: [`MARKER_GLYPH_COUNT`] cells of 64 px laid out horizontally, white-on-alpha.
/// Returned as `(rgba, width, height, uv)` for [`crate::engine::RenderEngine::ensure_slot_atlas`];
/// `uv` is the flat `[minU,minV,maxU,maxV]·N` table, cell `i` at `[i/N, 0, (i+1)/N, 1]`.
///
/// Cells 0/1 (ring/disc) are byte-identical to `slots_gpu::build_slot_atlas`, so replacing that
/// two-cell atlas with this one leaves every slot / vehicle / comment glyph pixel-for-pixel unchanged
/// while adding the marker shapes the `MissionMarkers` lane selects per marker.
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

/// Pack briefing-marker CAPTION text into 20 B text-atlas icon instances (WORLD meters), for the
/// EXISTING text pipeline — the same `text_layout` path place-name labels use, never a second one.
///
/// `xy` is the interleaved `[x0,z0,…]` marker anchors (as fed to `markers_bind`); `captions[i]` is
/// marker `i`'s label (empty = no caption). Each non-empty caption is laid out as a single line whose
/// FIRST glyph sits a short gap to the RIGHT of the marker glyph and is vertically centred on it, so
/// the caption reads beside its marker and its nearest ink stays well within the 40 px acceptance
/// window at standard zooms (the gap is `~14 px` in world meters via `px_to_m_at_zoom`). Returns
/// `text_layout::pack_text_icon_bytes` output (world coords; the engine applies the anchor shift).
#[must_use]
pub fn pack_marker_caption_bytes(xy: &[f32], captions: &[String], deck_zoom: f64) -> Vec<u8> {
    let n = xy.len() / 2;
    let char_m = crate::text_layout::text_char_meters(deck_zoom);
    let advance = char_m * crate::text_layout::TEXT_GLYPH_ADVANCE_RATIO;
    // Marker glyph is ~SLOT_RING_PX across; start the caption just past its right edge.
    let px_m = map_engine_core::slots_gpu::px_to_m_at_zoom(deck_zoom);
    let gap_m = px_m * 14.0;
    let mut glyphs: Vec<crate::text_layout::TextGlyphInstance> = Vec::new();
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
            glyphs.push(crate::text_layout::TextGlyphInstance {
                x: gx,
                y: my,
                half_m: char_m * 0.5,
                glyph: crate::text_layout::glyph_index_for_char(ch),
            });
        }
    }
    crate::text_layout::pack_text_icon_bytes(&glyphs, deck_zoom)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Class R: the calibration instances' upload bytes, memcmp'd against literals built
    /// from the exact f32 constants of the plan's calibration scene.
    #[test]
    fn calibration_instance_bytes_exact() {
        let instances = calibration_instances();
        let got: &[u8] = bytemuck::cast_slice(&instances);

        let mut expect = Vec::with_capacity(64);
        for v in [
            -100.0_f32, -100.0, 100.0, 100.0, 0.0, 1.0, 0.0, 1.0, // G
            50.0, 50.0, 90.0, 90.0, 1.0, 0.0, 0.0, 1.0, // R
        ] {
            expect.extend_from_slice(&v.to_le_bytes());
        }
        assert_eq!(core::mem::size_of::<QuadInstance>(), 32);
        assert_eq!(got, expect.as_slice());
    }

    /// Class R: `IconInstance` is exactly 20 B (pos2 + size + yaw_i16 + glyph_u16 + tint_u32).
    #[test]
    fn icon_instance_layout_is_20_bytes() {
        assert_eq!(core::mem::size_of::<IconInstance>(), 20);
        assert_eq!(core::mem::align_of::<IconInstance>(), 4);
        let inst = IconInstance {
            pos: [1.5, -2.5],
            size: 3.0,
            yaw: -16384, // ~-90° snorm
            glyph: 7,
            tint: 0xFF27_5A2D, // note: little-endian store of pack order
        };
        let got: &[u8] = bytemuck::bytes_of(&inst);
        assert_eq!(got.len(), 20);
        assert_eq!(f32::from_le_bytes(got[0..4].try_into().unwrap()), 1.5);
        assert_eq!(f32::from_le_bytes(got[4..8].try_into().unwrap()), -2.5);
        assert_eq!(f32::from_le_bytes(got[8..12].try_into().unwrap()), 3.0);
        assert_eq!(i16::from_le_bytes(got[12..14].try_into().unwrap()), -16384);
        assert_eq!(u16::from_le_bytes(got[14..16].try_into().unwrap()), 7);
    }

    /// Class R: the `BuildingInstance` GPU layout is exactly 40 B (`center,half,basis,color` =
    /// 10 f32, no padding), and its upload bytes are the concatenated little-endian f32s in field
    /// order — the byte contract `upload_world_buildings` casts through.
    #[test]
    fn building_instance_layout_and_bytes_exact() {
        assert_eq!(core::mem::size_of::<BuildingInstance>(), 40);
        let inst = BuildingInstance {
            center: [1.5, -2.5],
            half: [40.0, 20.0],
            basis: [0.25, 0.75],
            color: [38.0 / 255.0, 38.0 / 255.0, 44.0 / 255.0, 1.0],
        };
        let got: &[u8] = bytemuck::cast_slice(core::slice::from_ref(&inst));
        let mut expect = Vec::with_capacity(40);
        for v in [
            1.5_f32,
            -2.5,
            40.0,
            20.0,
            0.25,
            0.75,
            38.0 / 255.0,
            38.0 / 255.0,
            44.0 / 255.0,
            1.0,
        ] {
            expect.extend_from_slice(&v.to_le_bytes());
        }
        assert_eq!(got, expect.as_slice());
    }

    const SEED: u64 = 0x1234_5678;

    /// Determinism (Class R): same inputs ⇒ bit-identical bytes; distinct chunks differ.
    #[test]
    fn stress_chunk_is_deterministic_and_chunk_independent() {
        let a = stress_chunk(0, 1_000, SEED);
        let b = stress_chunk(0, 1_000, SEED);
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(&a),
            bytemuck::cast_slice::<_, u8>(&b)
        );
        let c = stress_chunk(1, 1_000, SEED);
        assert_ne!(
            bytemuck::cast_slice::<_, u8>(&a),
            bytemuck::cast_slice::<_, u8>(&c)
        );
    }

    /// Domain properties: centers within the anchor-relative Everon bounds, half-sizes in
    /// [1, 10] m, alpha exactly 1.
    #[test]
    fn stress_chunk_domain_bounds() {
        for inst in stress_chunk(3, 10_000, SEED) {
            let cx = (inst.min[0] + inst.max[0]) / 2.0;
            let cy = (inst.min[1] + inst.max[1]) / 2.0;
            let hs = (inst.max[0] - inst.min[0]) / 2.0;
            assert!((-6_400.0..6_400.0).contains(&cx));
            assert!((-6_400.0..6_400.0).contains(&cy));
            assert!((1.0..=10.0).contains(&hs));
            assert_eq!(inst.color[3], 1.0);
        }
    }

    /// Class R cross-oracle pin: the first instance of chunks 0 and 1 at the house seed,
    /// as f32 bit patterns derived from an INDEPENDENT JavaScript implementation of the
    /// generator (Math.imul LCG + Math.fround per f32 op) — two implementations agreeing
    /// bit-for-bit, not a self-snapshot. Any change to the LCG, fold, or arithmetic order
    /// fails this loudly.
    #[test]
    fn stress_chunk_first_instances_pinned() {
        let c0 = stress_chunk(0, 4, SEED)[0];
        let c1 = stress_chunk(1, 4, SEED)[0];
        let expect_c0 = QuadInstance {
            min: [f32::from_bits(0xC5B6_3386), f32::from_bits(0xC451_A70A)],
            max: [f32::from_bits(0xC5B6_0996), f32::from_bits(0xC450_5786)],
            color: [
                f32::from_bits(0x3F33_2F4A),
                f32::from_bits(0x3F3C_71B5),
                f32::from_bits(0x3F19_A77F),
                1.0,
            ],
        };
        let expect_c1 = QuadInstance {
            min: [f32::from_bits(0x4396_6908), f32::from_bits(0x44EC_A312)],
            max: [f32::from_bits(0x439E_A338), f32::from_bits(0x44EE_B19E)],
            color: [
                f32::from_bits(0x3EE5_BB09),
                f32::from_bits(0x3F22_6D2F),
                f32::from_bits(0x3EB4_F6B9),
                1.0,
            ],
        };
        assert_eq!(c0, expect_c0);
        assert_eq!(c1, expect_c1);
    }

    /// Guard C (glyph-atlas fix) — the WGSL shader's icon UV-table size and glyph clamp are literals
    /// that MUST equal `ATLAS_GLYPH_COUNT` (WGSL cannot import the Rust const). If the constant is
    /// bumped without updating `shader.wgsl` (or vice-versa), the Rust icon-uniform byte size
    /// (`ICON_UNIFORM_BYTES`, itself derived from this constant) and the shader's `IconUniforms`
    /// struct size disagree, and `create_render_pipeline` fails at runtime. Pinning the coupling here
    /// turns that into a loud CI failure instead of the silent 28-vs-29 regression that dark-glyphed
    /// the whole icon lane. (The Rust-side offsets are compile-time derived from the constant, so
    /// they can't drift; only this cross-language shader literal needs a runtime guard.)
    #[test]
    fn shader_uv_table_tracks_atlas_glyph_count() {
        let src = include_str!("shader.wgsl");
        let arr = format!("array<vec4<f32>, {ATLAS_GLYPH_COUNT}>");
        assert!(
            src.contains(&arr),
            "shader.wgsl must declare the icon UV table as `{arr}`"
        );
        let clamp = format!("min(in.glyph, {}u)", ATLAS_GLYPH_COUNT - 1);
        assert!(
            src.contains(&clamp),
            "shader.wgsl must clamp the glyph index with `{clamp}`"
        );
    }

    // ── T-790 marker glyph + caption tests ────────────────────────────────────────────────────

    /// The three DISTINCT-icon aliases the acceptance uses must resolve to three DIFFERENT canonical
    /// glyphs (else "3 distinct glyph pixel signatures" is unreachable), and the fallback holds.
    #[test]
    fn marker_glyph_mapping_is_distinct_and_folds_case() {
        assert_eq!(marker_glyph_for_alias("attack"), MarkerGlyph::TriangleUp);
        assert_eq!(marker_glyph_for_alias("defend"), MarkerGlyph::TriangleDown);
        assert_eq!(marker_glyph_for_alias("flag"), MarkerGlyph::Flag);
        // three distinct signatures
        let three = [
            marker_glyph_for_alias("attack") as u16,
            marker_glyph_for_alias("defend") as u16,
            marker_glyph_for_alias("flag") as u16,
        ];
        assert_eq!(
            three.iter().collect::<std::collections::HashSet<_>>().len(),
            3,
            "the three acceptance icons must map to three distinct glyphs"
        );
        // Case + separator folding collapses the duplicates the T-806 trap names.
        assert_eq!(marker_glyph_for_alias("Waypoint"), MarkerGlyph::Chevron);
        assert_eq!(marker_glyph_for_alias("waypoint"), MarkerGlyph::Chevron);
        assert_eq!(marker_glyph_for_alias("PHASE-LINE"), MarkerGlyph::Chevron);
        assert_eq!(marker_glyph_for_alias("rally point"), MarkerGlyph::Flag);
        // Unknown / empty degrade to DISC (the mod's FALLBACK_ICON = DOT).
        assert_eq!(marker_glyph_for_alias(""), MarkerGlyph::Disc);
        assert_eq!(marker_glyph_for_alias("not-a-real-icon"), MarkerGlyph::Disc);
        assert_eq!(marker_glyph_for_alias("dot"), MarkerGlyph::Disc);
    }

    /// Every one of the 64 authored `mission.schema.json` aliases must map to SOME canonical glyph
    /// (never panic), and DOT-family aliases must land on the fallback disc — the mod's grouping.
    #[test]
    fn every_schema_alias_maps() {
        // The DOT family from TBD_MarkerIcons + the schema `$defs/marker.icon` enum.
        for a in ["dot", "dot2", "point", "mark", "marker"] {
            assert_eq!(marker_glyph_for_alias(a), MarkerGlyph::Disc, "{a}");
        }
        for a in ["objective_marker", "obj", "target", "task"] {
            assert_eq!(marker_glyph_for_alias(a), MarkerGlyph::Square, "{a}");
        }
        for a in ["observation_post", "op", "overwatch", "recon"] {
            assert_eq!(marker_glyph_for_alias(a), MarkerGlyph::Target, "{a}");
        }
        // Full enum: none may exceed the atlas cell count.
        for a in [
            "dot",
            "dot2",
            "objective_marker",
            "objective_marker2",
            "point_of_interest",
            "point_of_interest2",
            "observation_post",
            "observation_post2",
            "destroy",
            "destroy2",
            "attack",
            "defend",
            "defend2",
            "waypoint",
            "waypoint2",
            "ambush",
            "ambush2",
            "flag",
            "flag2",
            "cross",
            "cross2",
            "circle",
            "circle2",
            "objective",
            "obj",
            "target",
            "task",
            "assault",
            "capture",
            "seize",
            "advance",
            "hold",
            "garrison",
            "fallback",
            "demolish",
            "demo",
            "sabotage",
            "move",
            "wp",
            "route",
            "phase_line",
            "poi",
            "intel",
            "contact",
            "op",
            "observe",
            "overwatch",
            "recon",
            "rally",
            "rally_point",
            "base",
            "hq",
            "spawn",
            "medical",
            "medic",
            "aid",
            "casevac",
            "medevac",
            "area",
            "zone",
            "ao",
            "point",
            "mark",
            "marker",
        ] {
            assert!(
                (marker_glyph_for_alias(a) as usize) < MARKER_GLYPH_COUNT,
                "{a}"
            );
        }
    }

    /// The widened atlas keeps cells 0 (ring) and 1 (disc) BYTE-IDENTICAL to
    /// `slots_gpu::build_slot_atlas`, so the slot / vehicle / comment lanes that share this atlas are
    /// pixel-unchanged. Deleting the `0`/`1` arms of `marker_glyph_coverage` fails this loudly.
    #[test]
    fn marker_atlas_cells_0_and_1_match_slot_atlas() {
        let (rgba, w, h, uv) = build_marker_slot_atlas();
        assert_eq!(h, 64);
        assert_eq!(w, 64 * MARKER_GLYPH_COUNT as u32);
        assert_eq!(uv.len(), MARKER_GLYPH_COUNT * 4);
        // cell 0 UV starts at 0, cell 1 at 1/N, both full height.
        assert_eq!(&uv[0..4], &[0.0, 0.0, 1.0 / MARKER_GLYPH_COUNT as f32, 1.0]);

        let slot = map_engine_core::slots_gpu::build_slot_atlas();
        // slot atlas is 128×64: ring in x[0,64), disc in x[64,128). Compare row by row.
        let sw = slot.width as usize; // 128
        let mw = w as usize;
        for y in 0..64usize {
            for x in 0..64usize {
                // ring: slot cell 0 (x) vs marker cell 0 (x)
                let s = (y * sw + x) * 4 + 3;
                let m = (y * mw + x) * 4 + 3;
                assert_eq!(rgba[m], slot.rgba[s], "ring alpha @({x},{y})");
                // disc: slot cell 1 (64+x) vs marker cell 1 (64+x)
                let s2 = (y * sw + 64 + x) * 4 + 3;
                let m2 = (y * mw + 64 + x) * 4 + 3;
                assert_eq!(rgba[m2], slot.rgba[s2], "disc alpha @({x},{y})");
            }
        }
    }

    /// Each canonical glyph cell must actually carry ink (a blank cell is an invisible icon → no
    /// distinct signature), and distinct shapes must produce distinct pixel footprints.
    #[test]
    fn every_marker_glyph_cell_has_distinct_ink() {
        let (rgba, w, _h, _uv) = build_marker_slot_atlas();
        let mw = w as usize;
        let cell_alpha = |cell: usize| -> Vec<u8> {
            let mut out = Vec::with_capacity(64 * 64);
            for y in 0..64usize {
                for x in 0..64usize {
                    out.push(rgba[(y * mw + cell * 64 + x) * 4 + 3]);
                }
            }
            out
        };
        let mut prints = Vec::new();
        for cell in 0..MARKER_GLYPH_COUNT {
            let a = cell_alpha(cell);
            let ink: u32 = a.iter().map(|&v| u32::from(v)).sum();
            assert!(ink > 0, "glyph cell {cell} is blank");
            prints.push(a);
        }
        // Attack (4), Defend (5), Flag (8) — the acceptance trio — must be pairwise distinct.
        assert_ne!(
            prints[4], prints[5],
            "attack vs defend footprints identical"
        );
        assert_ne!(prints[4], prints[8], "attack vs flag footprints identical");
        assert_ne!(prints[5], prints[8], "defend vs flag footprints identical");
    }

    /// Caption bytes: one 20 B text instance per glyph, only for non-empty labels, placed to the
    /// RIGHT of the marker and vertically on it, with the first glyph inside the 40 px window.
    #[test]
    fn marker_captions_pack_beside_their_marker() {
        // two markers at world (100, 200) and (300, 400); only the first captioned.
        let xy = [100.0_f32, 200.0, 300.0, 400.0];
        let caps = vec!["AB".to_string(), String::new()];
        let zoom = 0.0; // px_to_m = 1
        let bytes = pack_marker_caption_bytes(&xy, &caps, zoom);
        // "AB" = 2 glyphs × 20 B; the empty caption contributes nothing.
        assert_eq!(bytes.len(), 2 * 20);
        // first glyph position: x > marker x (to the right), y == marker y (centred).
        let gx0 = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let gy0 = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert!(gx0 > 100.0, "caption starts right of the marker");
        assert!(
            (gy0 - 200.0).abs() < 1.0,
            "caption centred on the marker row"
        );
        // At zoom 0 (1 m/px) the first glyph is within 40 px of the marker.
        assert!((gx0 - 100.0) < 40.0, "first glyph within the 40 px window");
        // blank / all-empty captions yield no bytes.
        assert!(pack_marker_caption_bytes(&xy, &[String::new(), String::new()], zoom).is_empty());
        assert!(pack_marker_caption_bytes(&[], &[], zoom).is_empty());
    }
}
