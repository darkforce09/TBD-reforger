//! Role: footprint.
//! Position: `world/environment/buildings` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::streaming::buffers::revision::BUILDING_MIN_ZOOM;
use crate::streaming::buffers::revision::norm;
use crate::streaming::scheduler::state::WorldResidency;
use crate::world::environment::buildings::obb::obb_corners;
use crate::world::environment::classify::class_code;

/// Canonical outline color value.
pub(crate) const OUTLINE_COLOR: [u8; 4] = [30, 30, 34, 255];

/// Canonical fill default value.
pub(crate) const FILL_DEFAULT: [u8; 4] = [38, 38, 44, 184];

/// Canonical bridge deck rgba value.
pub(crate) const BRIDGE_DECK_RGBA: [u8; 4] = [120, 116, 110, 215];

/// Canonical bridge casing rgba value.
pub(crate) const BRIDGE_CASING_RGBA: [u8; 4] = [40, 40, 46, 220];

/// Canonical bridge casing margin m value.
pub(crate) const BRIDGE_CASING_MARGIN_M: f64 = 0.8;

/// Fill color.
#[must_use]
pub(crate) fn fill_color(class: &str) -> [u8; 4] {
    match class {
        "military" => [0x7a, 0x5c, 0x3d, 184],

        "pier" | "dock" => [110, 95, 75, 190],
        "ruin" => [58, 56, 60, 110],
        "castle" => [70, 58, 48, 190],
        "lighthouse" => [235, 235, 235, 220],
        "container" => [60, 70, 90, 184],
        "tent" => [92, 82, 50, 184],
        "shed" | "garage" => [50, 50, 56, 184],
        _ => FILL_DEFAULT,
    }
}

/// Building visible.
#[inline]
pub(crate) fn building_visible(deck_zoom: f64) -> bool {
    deck_zoom >= BUILDING_MIN_ZOOM
}

impl WorldResidency {
    /// Rebuild buffers.
    pub(crate) fn rebuild_buffers(&mut self) {
        self.fill_recomposes += 1;

        if !self.toggle_buildings || !building_visible(self.deck_zoom) {
            self.fill_buf.clear();
            self.outline_buf.clear();
            self.rebuild_strip_buffers();
            self.refresh_draw_set_and_glyphs();
            return;
        }
        let building_code = class_code("building");
        let outline_norm = norm(OUTLINE_COLOR);
        let mut fill: Vec<f32> = Vec::new();
        let mut outline: Vec<f32> = Vec::new();
        let mut ids = self.pinned_ids.clone();
        ids.sort();
        for id in &ids {
            let Some(chunk) = self.chunks.get(id) else {
                continue;
            };
            let Some(rows) = chunk.rows_by_class.get(&building_code) else {
                continue;
            };
            for &r in rows {
                let r = r as usize;
                let Some(info) = self.building_by_u16.get(&chunk.prefab_idx[r]) else {
                    continue;
                };
                let x = f64::from(chunk.positions[2 * r]);
                let y = f64::from(chunk.positions[2 * r + 1]);
                let rot = f64::from(chunk.rotations[r]);
                let cls = info.building_class.as_str();

                if cls == "pier" || cls == "dock" {
                    continue;
                }

                let rad = (rot * std::f64::consts::PI) / 180.0;
                let cos = rad.cos() as f32;
                let sin = rad.sin() as f32;
                if cls == "bridge" {
                    let (chx, chy) = if info.half_x >= info.half_y {
                        (info.half_x + BRIDGE_CASING_MARGIN_M, info.half_y)
                    } else {
                        (info.half_x, info.half_y + BRIDGE_CASING_MARGIN_M)
                    };
                    let casing = norm(BRIDGE_CASING_RGBA);
                    fill.extend_from_slice(&[
                        x as f32, y as f32, chx as f32, chy as f32, cos, sin, casing[0], casing[1],
                        casing[2], casing[3],
                    ]);
                    let deck = norm(BRIDGE_DECK_RGBA);
                    fill.extend_from_slice(&[
                        x as f32,
                        y as f32,
                        info.half_x as f32,
                        info.half_y as f32,
                        cos,
                        sin,
                        deck[0],
                        deck[1],
                        deck[2],
                        deck[3],
                    ]);
                } else {
                    let fill_rgba = if self.early_landmark_glyph_active(cls, info.importance_zoom) {
                        FILL_DEFAULT
                    } else {
                        fill_color(cls)
                    };
                    let c = norm(fill_rgba);
                    fill.extend_from_slice(&[
                        x as f32,
                        y as f32,
                        info.half_x as f32,
                        info.half_y as f32,
                        cos,
                        sin,
                        c[0],
                        c[1],
                        c[2],
                        c[3],
                    ]);
                }

                let ring = obb_corners(x, y, info.half_x, info.half_y, rot);
                for e in 0..4 {
                    let a = ring[e];
                    let b = ring[(e + 1) % 4];
                    outline.extend_from_slice(&[
                        a[0] as f32,
                        a[1] as f32,
                        outline_norm[0],
                        outline_norm[1],
                        outline_norm[2],
                        outline_norm[3],
                    ]);
                    outline.extend_from_slice(&[
                        b[0] as f32,
                        b[1] as f32,
                        outline_norm[0],
                        outline_norm[1],
                        outline_norm[2],
                        outline_norm[3],
                    ]);
                }
            }
        }
        self.fill_buf = fill;
        self.outline_buf = outline;
        self.rebuild_strip_buffers();
        self.refresh_draw_set_and_glyphs();
    }
}
