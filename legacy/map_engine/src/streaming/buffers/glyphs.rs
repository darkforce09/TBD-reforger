//! Role: glyphs.
//! Position: `streaming/buffers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::overlay::lod::INSTANCE_BUDGET;
use crate::overlay::lod::class_visible;
use crate::world::environment::classify::class_code;

use crate::overlay::symbology::labels::glyph_math::BADGE_SIZE_MIN_PX;
use crate::overlay::symbology::labels::glyph_math::DEFAULT_BASE_SIZE_PX;
use crate::overlay::symbology::labels::glyph_math::GLYPH_SIZE_MIN_PX;
use crate::overlay::symbology::labels::glyph_math::badge_size_meters;
use crate::overlay::symbology::labels::glyph_math::deck_angle_for_rotation_deg;
use crate::overlay::symbology::labels::glyph_math::glyph_size_meters;
use crate::overlay::symbology::labels::glyph_math::hex_to_rgba;
use crate::overlay::symbology::labels::glyph_math::landmark_glyph_icon_key;
use crate::overlay::symbology::labels::glyph_math::pack_icon_instance;
use crate::overlay::symbology::labels::glyph_math::pack_rgba_u32;
use crate::overlay::symbology::labels::glyph_math::size_with_min_px;
use crate::streaming::scheduler::state::GlyphPrefabInfo;
use crate::streaming::scheduler::state::WorldResidency;
use crate::world::terrain::roads::airfield::is_airfield_structure_class;
use crate::world::terrain::roads::airfield::point_in_bbox;

impl WorldResidency {
    /// Rebuild glyph lookup from prefabs.
    pub(crate) fn rebuild_glyph_lookup_from_prefabs(&mut self) {
        self.glyph_by_u16.clear();
        if self.icon_key_to_idx.is_empty() {
            return;
        }
        for entry in self.prefab_by_id.values() {
            let pid = entry.row.prefab_id;
            if !(0.0..65536.0).contains(&pid) || pid.fract() != 0.0 {
                continue;
            }
            let Some(icon_key) = entry.row.icon_key.as_deref() else {
                continue;
            };
            let Some(&glyph_idx) = self.icon_key_to_idx.get(icon_key) else {
                continue;
            };
            let code = entry.code;
            let base = entry.row.base_size_px.unwrap_or(DEFAULT_BASE_SIZE_PX);
            let tree_code = class_code("tree");
            let veg_code = class_code("vegetation");
            let prop_code = class_code("prop");
            let rock_code = class_code("rockLarge");
            let building_code = class_code("building");
            let (group, size_m, tint) = if code == tree_code || code == veg_code {
                (
                    0u8,
                    glyph_size_meters(base, entry.row.height_m) as f32,
                    pack_rgba_u32(hex_to_rgba(entry.row.default_color.as_deref())),
                )
            } else if code == prop_code || code == rock_code {
                (
                    1u8,
                    glyph_size_meters(base, entry.row.height_m) as f32,
                    pack_rgba_u32(hex_to_rgba(entry.row.default_color.as_deref())),
                )
            } else if code == building_code {
                (
                    2u8,
                    badge_size_meters() as f32,
                    pack_rgba_u32([255, 255, 255, 255]),
                )
            } else {
                continue;
            };
            self.glyph_by_u16.insert(
                pid as u16,
                GlyphPrefabInfo {
                    glyph_idx,
                    size_m,
                    tint,
                    group,
                },
            );
        }

        let min_glyph_size_m = self
            .glyph_by_u16
            .values()
            .filter(|g| g.group == 0 || g.group == 1)
            .map(|g| f64::from(g.size_m))
            .filter(|s| *s > 0.0)
            .fold(f64::INFINITY, f64::min);
        let z_glyph = if min_glyph_size_m.is_finite() {
            (GLYPH_SIZE_MIN_PX / min_glyph_size_m).log2()
        } else {
            f64::NEG_INFINITY
        };
        let badge_size_m = badge_size_meters();
        let z_badge = if badge_size_m > 0.0 {
            (BADGE_SIZE_MIN_PX / badge_size_m).log2()
        } else {
            f64::NEG_INFINITY
        };
        self.glyph_size_floor_zoom = z_glyph.max(z_badge);
    }
}

impl WorldResidency {
    /// Rebuild glyph buffers.
    pub(crate) fn rebuild_glyph_buffers(&mut self) {
        self.tree_glyph_buf.clear();
        self.prop_glyph_buf.clear();
        self.badge_glyph_buf.clear();

        let z = self.deck_zoom;
        let tree_want =
            self.toggle_trees && (class_visible("tree", z) || class_visible("vegetation", z));
        let prop_want =
            self.toggle_props && (class_visible("prop", z) || class_visible("rockLarge", z));

        let badge_gate = class_visible("buildingBadge", z);
        let badge_want = self.toggle_buildings
            && (badge_gate || self.min_importance_zoom.is_some_and(|m| z >= m));

        self.tree_want = tree_want;
        self.prop_want = prop_want;
        self.badge_want = badge_want;

        if !tree_want && !prop_want && !badge_want {
            return;
        }

        let tree_code = class_code("tree");
        let veg_code = class_code("vegetation");
        let prop_code = class_code("prop");
        let rock_code = class_code("rockLarge");
        let building_code = class_code("building");

        let ids = self.draw_ids.clone();

        let pack_trees = tree_want && !self.heatmap_trees;
        let mut prop_total = 0usize;
        let mut badge_total = 0usize;

        for id in &ids {
            let Some(chunk) = self.chunks.get(id) else {
                continue;
            };

            if pack_trees || prop_want {
                for &code in &[tree_code, veg_code, prop_code, rock_code] {
                    let class_ok = match code {
                        c if c == tree_code => class_visible("tree", z),
                        c if c == veg_code => class_visible("vegetation", z),
                        c if c == prop_code => class_visible("prop", z),
                        c if c == rock_code => class_visible("rockLarge", z),
                        _ => false,
                    };
                    if !class_ok {
                        continue;
                    }
                    let Some(rows) = chunk.rows_by_class.get(&code) else {
                        continue;
                    };
                    for &r in rows {
                        let r = r as usize;
                        let Some(info) = self.glyph_by_u16.get(&chunk.prefab_idx[r]).cloned()
                        else {
                            continue;
                        };
                        let is_tree_group = info.group == 0;
                        if is_tree_group {
                            if !pack_trees {
                                continue;
                            }
                        } else {
                            if !prop_want {
                                continue;
                            }

                            if prop_total + badge_total >= INSTANCE_BUDGET {
                                continue;
                            }
                        }
                        let size =
                            size_with_min_px(f64::from(info.size_m), GLYPH_SIZE_MIN_PX, z) as f32;
                        let yaw = deck_angle_for_rotation_deg(f64::from(chunk.rotations[r]));
                        let px = chunk.positions[2 * r];
                        let py = chunk.positions[2 * r + 1];
                        if is_tree_group {
                            pack_icon_instance(
                                &mut self.tree_glyph_buf,
                                px,
                                py,
                                size,
                                yaw,
                                info.glyph_idx,
                                info.tint,
                            );
                        } else {
                            pack_icon_instance(
                                &mut self.prop_glyph_buf,
                                px,
                                py,
                                size,
                                yaw,
                                info.glyph_idx,
                                info.tint,
                            );
                            prop_total += 1;
                        }
                    }
                }
            }

            if badge_want {
                let Some(rows) = chunk.rows_by_class.get(&building_code) else {
                    continue;
                };
                for &r in rows {
                    if prop_total + badge_total >= INSTANCE_BUDGET {
                        break;
                    }
                    let r = r as usize;
                    let Some(binfo) = self.building_by_u16.get(&chunk.prefab_idx[r]) else {
                        continue;
                    };

                    if !badge_gate && !binfo.importance_zoom.is_some_and(|iz| z >= iz) {
                        continue;
                    }
                    let cls = binfo.building_class.as_str();
                    if is_airfield_structure_class(cls) {
                        if !self.airfield_visible() {
                            continue;
                        }
                        let px = f64::from(chunk.positions[2 * r]);
                        let py = f64::from(chunk.positions[2 * r + 1]);
                        let Some(bbox) = self.airfield_bbox else {
                            continue;
                        };
                        if !point_in_bbox(px, py, bbox) {
                            continue;
                        }
                    }
                    let Some(key) = landmark_glyph_icon_key(cls) else {
                        continue;
                    };
                    let Some(&glyph_idx) = self.icon_key_to_idx.get(key) else {
                        continue;
                    };
                    let size = size_with_min_px(badge_size_meters(), BADGE_SIZE_MIN_PX, z) as f32;
                    pack_icon_instance(
                        &mut self.badge_glyph_buf,
                        chunk.positions[2 * r],
                        chunk.positions[2 * r + 1],
                        size,
                        0.0,
                        glyph_idx,
                        pack_rgba_u32([255, 255, 255, 255]),
                    );
                    badge_total += 1;
                }
            }
        }
    }
}
