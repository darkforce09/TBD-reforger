//! Role: upload.
//! Position: `streaming/loaders/world_loader` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::BridgeHandle;
use super::EngineHandle;
use super::WorldHost;
use super::publish_engine;

impl WorldHost {
    /// Upload airfield apron.
    pub fn upload_airfield_apron(
        &self,
        engine: &EngineHandle,
        grid: &crate::terrain::dem::grid::DemVectorGrid,
        visible: bool,
    ) {
        let Some(bbox) = self.residency.airfield_bbox() else {
            return;
        };
        let mesh = crate::terrain::roads::airfield::build_airfield_apron_mesh(grid, bbox);
        if mesh.polygon_count == 0 {
            return;
        }
        if let Some(e) = engine.borrow_mut().as_mut() {
            e.upload_polygon_mesh(
                crate::core::pipeline::draw_order::role_id::AIRFIELD_APRON,
                &mesh.positions,
                &mesh.colors,
                &mesh.indices,
                mesh.polygon_count,
                visible,
            );
        }
    }
}

impl WorldHost {
    /// Push to engine.
    pub(super) fn push_to_engine(&mut self, engine: &EngineHandle, bridge: &BridgeHandle) -> bool {
        let revision = self.residency.buffers_revision();
        let pin_settled = self.residency.pin_settled();
        let inflight_empty = self.residency.inflight_count() == 0;
        let gate = (revision, pin_settled, inflight_empty);
        if self.last_pushed == Some(gate) {
            let mut b = bridge.borrow_mut();
            b.merge_residency_stats(&self.residency.stats_json());
            b.tree_glyph_packed = self.residency.tree_glyph_count();
            return false;
        }
        self.last_pushed = Some(gate);

        self.crossing_allocs.pass_clones += 6;
        let fill = self.residency.world_building_fill();
        let outline = self.residency.world_building_outline();
        let stats = self.residency.stats_json();

        let skip_buildings =
            fill.is_empty() && (!inflight_empty || !self.pending.is_empty() || !pin_settled);
        let b_vis = self.residency.buildings_visible();
        let chunks_pinned = self.residency.chunks_resident() as u32;
        let trees = self.residency.world_tree_glyphs();
        let props = self.residency.world_prop_glyphs();
        let badges = self.residency.world_badge_glyphs();

        let strips = self.residency.world_fence_strips();
        let strip_vis = self.residency.strips_visible();
        let strip_count = self.residency.fence_strip_segment_count()
            + self.residency.pier_strip_segment_count()
            + self.residency.bridge_rail_strip_count();
        {
            let mut b = bridge.borrow_mut();
            b.tree_glyph_packed = self.residency.tree_glyph_count();
            b.merge_residency_stats(&stats);
        }
        {
            let mut g = engine.borrow_mut();
            let Some(e) = g.as_mut() else {
                return false;
            };
            if !skip_buildings {
                e.upload_world_buildings(&fill, chunks_pinned, b_vis);
                e.upload_world_building_outlines(&outline, b_vis);
            }

            if !trees.is_empty() || pin_settled || self.residency.tree_lane_off() {
                e.upload_icon_lane(0, &trees, true);
            }
            if !props.is_empty() || pin_settled || self.residency.prop_lane_off() {
                e.upload_icon_lane(1, &props, true);
            }
            if !badges.is_empty() || pin_settled || self.residency.badge_lane_off() {
                e.upload_icon_lane(2, &badges, true);
            }
            e.upload_world_fence_strips(&strips, strip_count, strip_vis);
            publish_engine(bridge, e);
        }
        true
    }
}
