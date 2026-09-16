//! Role: markers.
//! Position: `symbology` in the map engine.
//! Signals & state: the marker icon vocabulary, its rasterised atlas, and caption packing.
//! Invariants: the alias table below is `mission.schema.json` `$defs/marker.icon` vocabulary —
//! `casevac`, `rally_point`, `phase_line`. That is exactly why it stays on this side of the
//! wall: the renderer draws cell 6, and never learns that cell 6 means a medical cross.

use crate::renderers::text::metrics::TEXT_GLYPH_ADVANCE_RATIO;
use crate::renderers::text::metrics::TextGlyphInstance;
use crate::renderers::text::metrics::glyph_index_for_char;
use crate::renderers::text::metrics::text_char_meters;
use crate::renderers::text::packing::pack_text_icon_bytes;
use crate::symbology::instances::symbols::px_to_m_at_zoom;

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

/// Number of distinct canonical marker glyphs — the atlas cell count [`build_marker_slot_atlas`] emits, and the highest [`MarkerGlyph`] discriminant + 1. Well under [`website_graphics_engine::draw::instances::ATLAS_GLYPH_COUNT`] (32).
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

/// Widened slot atlas: [`MARKER_GLYPH_COUNT`] cells of 64 px laid out horizontally, white-on-alpha. Returned as `(rgba, width, height, uv)` for [`crate::core::context::state::RenderEngine::ensure_slot_atlas`]; `uv` is the flat `[minU,minV,maxU,maxV]·N` table, cell `i` at `[i/N, 0, (i+1)/N, 1]`.
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
    let char_m = text_char_meters(deck_zoom);
    let advance = char_m * TEXT_GLYPH_ADVANCE_RATIO;

    let px_m = px_to_m_at_zoom(deck_zoom);
    let gap_m = px_m * 14.0;
    let mut glyphs: Vec<TextGlyphInstance> = Vec::new();
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
            glyphs.push(TextGlyphInstance {
                x: gx,
                y: my,
                half_m: char_m * 0.5,
                glyph: glyph_index_for_char(ch),
            });
        }
    }
    pack_text_icon_bytes(&glyphs, deck_zoom)
}

#[cfg(test)]
#[path = "tests/markers_tests.rs"]
mod tests;
