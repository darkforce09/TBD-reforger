//! The pier, bridge-rail and fence strips over the pinned chunks, and their memo key.
//!
//! **Role:** composes the cartographic strip buffer: piers and docks, two rails a bridge, and
//! fences, each lane behind its own visibility predicate, and counts each lane's strips.
//! **Position:** `chunk_draw_buffers`; methods of
//! `DrawBuffers` (`draw_buffers.rs`) reading a
//! [`chunk_scheduler::state::ChunkResidency`]; called by the draw set refresh, the
//! footprint rebuild and the fences toggle.
//! **Signals & state:** writes the strip buffer and the three strip counts.
//! **Invariants:** chunks compose in chunk-id order; the fences toggle never changes the pier or
//! rail lanes and the buildings toggle never changes the fence lane.

use crate::draw_buffers::DrawBuffers;
use crate::footprint::fill_color;
use chunk_scheduler::state::ChunkResidency;
use prefab_catalog::render_classes::class_code;
use road_network::cartographic_strip::compose_bridge_rail_strips;
use road_network::cartographic_strip::compose_fence_strip;
use road_network::cartographic_strip::compose_pier_strip;
use road_network::cartographic_strip::pack_cartographic_strips;

impl DrawBuffers {
    /// Rebuild strip buffers.
    pub(super) fn rebuild_strip_buffers(&mut self, residency: &ChunkResidency) {
        self.strip_buf.clear();
        self.fence_strip_count = 0;
        self.pier_strip_count = 0;
        self.bridge_rail_count = 0;
        let prop_code = class_code("prop");
        let building_code = class_code("building");
        let z = residency.deck_zoom();
        let mut ids = residency.pinned_ids().to_vec();
        ids.sort();
        let mut pier_v: Vec<road_network::styling::StripVertex> = Vec::new();
        let mut rail_v: Vec<road_network::styling::StripVertex> = Vec::new();
        let mut fence_v: Vec<road_network::styling::StripVertex> = Vec::new();

        let piers_on = self.piers_visible(z);
        let rails_on = self.buildings_visible(z);
        if piers_on || rails_on {
            for id in &ids {
                let Some(chunk) = residency.resident_chunks().get(id) else {
                    continue;
                };
                let Some(rows) = chunk.rows_by_class.get(&building_code) else {
                    continue;
                };
                for &r in rows {
                    let r = r as usize;
                    let Some(info) = residency.building_prefab(chunk.prefab_idx[r]) else {
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

        if self.fences_visible(z) {
            for id in &ids {
                let Some(chunk) = residency.resident_chunks().get(id) else {
                    continue;
                };
                let Some(rows) = chunk.rows_by_class.get(&prop_code) else {
                    continue;
                };
                for &r in rows {
                    let r = r as usize;
                    let Some(finfo) = residency.fence_prefab(chunk.prefab_idx[r]) else {
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

impl DrawBuffers {
    /// Strip compose key.
    pub(super) fn strip_compose_key(&self, residency: &ChunkResidency) -> u64 {
        use std::hash::{Hash, Hasher};
        let z = residency.deck_zoom();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.draw_ids.hash(&mut h);
        residency.content_epoch().hash(&mut h);
        self.toggle_fences.hash(&mut h);
        self.toggle_buildings.hash(&mut h);
        z.to_bits().hash(&mut h);
        h.finish()
    }
}
