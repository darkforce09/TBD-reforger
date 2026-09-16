//! Role: strips.
//! Position: `streaming/buffers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::environment::buildings::footprint::fill_color;
use crate::environment::classify::class_code;
use crate::streaming::scheduler::state::WorldResidency;
use crate::terrain::roads::cartographic_strip::compose_bridge_rail_strips;
use crate::terrain::roads::cartographic_strip::compose_fence_strip;
use crate::terrain::roads::cartographic_strip::compose_pier_strip;
use crate::terrain::roads::cartographic_strip::pack_cartographic_strips;

impl WorldResidency {
    /// Rebuild strip buffers.
    pub(crate) fn rebuild_strip_buffers(&mut self) {
        self.strip_buf.clear();
        self.fence_strip_count = 0;
        self.pier_strip_count = 0;
        self.bridge_rail_count = 0;
        let prop_code = class_code("prop");
        let building_code = class_code("building");
        let z = self.deck_zoom;
        let mut ids = self.pinned_ids.clone();
        ids.sort();
        let mut pier_v: Vec<crate::terrain::roads::styling::StripVertex> = Vec::new();
        let mut rail_v: Vec<crate::terrain::roads::styling::StripVertex> = Vec::new();
        let mut fence_v: Vec<crate::terrain::roads::styling::StripVertex> = Vec::new();

        let piers_on = self.piers_visible();
        let rails_on = self.buildings_visible();
        if piers_on || rails_on {
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
                    let cls = info.building_class.as_str();
                    let x = f64::from(chunk.positions[2 * r]);
                    let y = f64::from(chunk.positions[2 * r + 1]);
                    let rot = f64::from(chunk.rotations[r]);
                    if piers_on && (cls == "pier" || cls == "dock") {
                        pier_v.extend(compose_pier_strip(
                            x,
                            y,
                            info.half_x,
                            info.half_y,
                            rot,
                            fill_color(cls),
                            z,
                        ));
                        self.pier_strip_count += 1;
                    } else if rails_on && cls == "bridge" {
                        rail_v.extend(compose_bridge_rail_strips(
                            x,
                            y,
                            info.half_x,
                            info.half_y,
                            rot,
                            z,
                        ));
                        self.bridge_rail_count += 2;
                    }
                }
            }
        }

        if self.fences_visible() {
            for id in &ids {
                let Some(chunk) = self.chunks.get(id) else {
                    continue;
                };
                let Some(rows) = chunk.rows_by_class.get(&prop_code) else {
                    continue;
                };
                for &r in rows {
                    let r = r as usize;
                    let Some(finfo) = self.fence_by_u16.get(&chunk.prefab_idx[r]) else {
                        continue;
                    };
                    let x = f64::from(chunk.positions[2 * r]);
                    let y = f64::from(chunk.positions[2 * r + 1]);
                    let rot = f64::from(chunk.rotations[r]);
                    fence_v.extend(compose_fence_strip(
                        x,
                        y,
                        finfo.half_x,
                        finfo.half_y,
                        rot,
                        z,
                    ));
                    self.fence_strip_count += 1;
                }
            }
        }

        let mut acc = pier_v;
        acc.extend(rail_v);
        acc.extend(fence_v);
        self.strip_buf = pack_cartographic_strips(&acc);
    }
}

impl WorldResidency {
    /// Strip compose key.
    pub(crate) fn strip_compose_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let z = self.deck_zoom;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.draw_ids.hash(&mut h);
        self.content_epoch.hash(&mut h);
        self.toggle_fences.hash(&mut h);
        self.toggle_buildings.hash(&mut h);
        z.to_bits().hash(&mut h);
        h.finish()
    }
}
