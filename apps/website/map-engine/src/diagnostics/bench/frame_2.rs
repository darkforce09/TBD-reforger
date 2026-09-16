//! Role: frame 2.
//! Position: `diagnostics/bench` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::state::RenderEngine;
use crate::core::pipeline::draw_order::LaneRole;
use crate::core::pipeline::draw_order::lane_id;
use wasm_bindgen::prelude::*;
use website_graphics_engine::frame::DrawPayload;

#[wasm_bindgen]
impl RenderEngine {
    /// Machine-readable engine stats (every performance claim in the verify log is one of these numbers). `upload_ms` is CPU-side enqueue time for the staging→GPU copies; `gpu_frame_ms` is present only when `TIMESTAMP_QUERY` is available and sampled.
    #[must_use]
    pub fn stats(&self) -> String {
        let stress_lane = lane_id(LaneRole::Stress);
        let stress_count = self
            .batches
            .iter()
            .filter(|b| matches!(b.payload, DrawPayload::Quads(_)) && b.lane == stress_lane)
            .count();
        let stress_bytes: u64 = self
            .batches
            .iter()
            .filter(|b| b.lane == stress_lane)
            .map(|b| match &b.payload {
                DrawPayload::Quads(i) => u64::from(i.count) * 32,
                _ => 0,
            })
            .sum();
        let gpu_bytes = stress_bytes + 64 + 32 + 64;

        // T-0xx Phase 1D: the mode / tile / byte counters left the batch with the texture;
        // they are `RenderEngine::tex_lanes`, keyed by the same lane the batch carries.
        let satellite = self.tex_lane(LaneRole::Satellite);
        let basemap_mode = satellite.map_or("none", |l| l.mode.as_str());
        let basemap_tiles = satellite.map_or(0, |l| l.tiles);
        let basemap_bytes: u64 = [LaneRole::Satellite, LaneRole::Hillshade]
            .into_iter()
            .filter_map(|r| self.tex_lane(r).map(|l| l.bytes))
            .sum();
        let gpu_frame_ms = match &self.timer {
            Some(t) if t.lane.has_sample() => format!("{:.3}", t.last_ms.get()),
            _ => "null".to_owned(),
        };

        let world_building_instances: u32 = self
            .batches
            .iter()
            .filter(|b| b.lane == lane_id(LaneRole::WorldBuildings))
            .map(|b| match &b.payload {
                DrawPayload::OrientedQuads(i) => i.count,
                _ => 0,
            })
            .sum();
        let world_building_outline_vertices: u32 = self
            .batches
            .iter()
            .filter(|b| b.lane == lane_id(LaneRole::WorldBuildingsOutline))
            .map(|b| match &b.payload {
                DrawPayload::Lines(v) => v.vertex_count,
                _ => 0,
            })
            .sum();

        let tree_glyphs: u32 = self.cull_lane_stat(
            LaneRole::WorldTrees,
            self.batches
                .iter()
                .filter(|b| b.lane == lane_id(LaneRole::WorldTrees))
                .map(|b| match &b.payload {
                    DrawPayload::Sprites { instances, .. } => instances.count,
                    _ => 0,
                })
                .sum(),
        );
        let prop_glyphs: u32 = self.cull_lane_stat(
            LaneRole::WorldProps,
            self.batches
                .iter()
                .filter(|b| b.lane == lane_id(LaneRole::WorldProps))
                .map(|b| match &b.payload {
                    DrawPayload::Sprites { instances, .. } => instances.count,
                    _ => 0,
                })
                .sum(),
        );
        let badge_glyphs: u32 = self.cull_lane_stat(
            LaneRole::WorldBadges,
            self.batches
                .iter()
                .filter(|b| b.lane == lane_id(LaneRole::WorldBadges))
                .map(|b| match &b.payload {
                    DrawPayload::Sprites { instances, .. } => instances.count,
                    _ => 0,
                })
                .sum(),
        );
        let atlas_bytes = self.glyph_atlas.as_ref().map_or(0, |a| a.bytes)
            + self.slot_atlas.as_ref().map_or(0, |a| a.bytes)
            + self.text_atlas.as_ref().map_or(0, |a| a.bytes);

        let slot_instances: u32 = self.cull_lane_stat(
            LaneRole::Slots,
            self.batches
                .iter()
                .filter(|b| b.lane == lane_id(LaneRole::Slots))
                .map(|b| match &b.payload {
                    DrawPayload::Sprites { instances, .. } => instances.count,
                    _ => 0,
                })
                .sum(),
        );
        let slot_drag_instances: u32 = self.cull_lane_stat(
            LaneRole::SlotDrag,
            self.batches
                .iter()
                .filter(|b| b.lane == lane_id(LaneRole::SlotDrag))
                .map(|b| match &b.payload {
                    DrawPayload::Sprites { instances, .. } => instances.count,
                    _ => 0,
                })
                .sum(),
        );
        let cluster_instances: u32 = self.cull_lane_stat(
            LaneRole::Clusters,
            self.batches
                .iter()
                .filter(|b| b.lane == lane_id(LaneRole::Clusters))
                .map(|b| match &b.payload {
                    DrawPayload::Sprites { instances, .. } => instances.count,
                    _ => 0,
                })
                .sum(),
        );
        let mission_vehicles: u32 = self.cull_lane_stat(
            LaneRole::MissionVehicles,
            self.batches
                .iter()
                .filter(|b| b.lane == lane_id(LaneRole::MissionVehicles))
                .map(|b| match &b.payload {
                    DrawPayload::Sprites { instances, .. } => instances.count,
                    _ => 0,
                })
                .sum(),
        );
        format!(
            concat!(
                "{{\"backend\":\"{}\",\"instances\":{},\"chunks\":{},\"gpu_bytes\":{},",
                "\"staging_peak_bytes\":{},\"gen_ms\":{:.1},\"upload_ms\":{:.1},",
                "\"uniform_bytes_last_frame\":{},\"gpu_frame_ms\":{},",
                "\"basemap_mode\":\"{}\",\"basemap_tiles\":{},\"basemap_bytes\":{},",
                "\"world_building_instances\":{},\"world_building_outline_vertices\":{},",
                "\"world_chunks_drawn\":{},",
                "\"sea_polygons\":{},\"landcover_polygons\":{},\"contour_segments\":{},",
                "\"road_segments\":{},\"forest_polygons\":{},\"forest_outline_segments\":{},",
                "\"forest_density_w\":{},\"forest_density_h\":{},\"forest_bins_ok\":{},\"forest_mode\":\"{}\",",
                "\"tree_glyphs\":{},\"prop_glyphs\":{},\"badge_glyphs\":{},\"text_labels_drawn\":{},\"atlas_bytes\":{},",
                "\"slot_instances\":{},\"slot_drag_instances\":{},\"cluster_instances\":{},",
                "\"mission_vehicles\":{},",
                "\"submitted_last_frame\":{},",
                "\"compute_cull\":{},\"compute_cull_cpu_count\":{},\"compute_cull_gpu_count\":{},",
                "\"compute_cull_gpu_sampled\":{},",
                "\"icon_lane_uploads\":{},\"polygon_lane_uploads\":{},\"strip_lane_uploads\":{},",
                "\"building_uploads\":{},\"text_label_uploads\":{},",
                "\"render_cpu_ms_last\":{:.4},\"render_cpu_ms_ema\":{:.4}}}"
            ),
            self.backend_kind,
            self.stress_instances,
            stress_count,
            gpu_bytes,
            self.staging_peak_bytes,
            self.gen_ms,
            self.upload_ms,
            self.uniform_bytes_last_frame,
            gpu_frame_ms,
            basemap_mode,
            basemap_tiles,
            basemap_bytes,
            world_building_instances,
            world_building_outline_vertices,
            self.world_chunks_drawn,
            self.sea_polygons,
            self.landcover_polygons,
            self.contour_segments,
            self.road_segments,
            self.forest_polygons,
            self.forest_outline_segments,
            self.forest_density_w,
            self.forest_density_h,
            self.forest_bins_ok,
            self.forest_mode,
            tree_glyphs,
            prop_glyphs,
            badge_glyphs,
            self.text_labels_drawn,
            atlas_bytes,
            slot_instances,
            slot_drag_instances,
            cluster_instances,
            mission_vehicles,
            self.submitted_last_frame,
            self.gpu_cull_enabled(),
            self.icon_cull
                .as_ref()
                .map(|c| c.last_cpu_count())
                .unwrap_or(0),
            self.icon_cull
                .as_ref()
                .map(|c| c.gpu_count_for_stats())
                .unwrap_or(0),
            self.icon_cull.as_ref().is_some_and(|c| c.gpu_sampled()),
            self.icon_lane_uploads,
            self.polygon_lane_uploads,
            self.strip_lane_uploads,
            self.building_uploads,
            self.text_label_uploads,
            self.render_cpu_ms_last,
            self.render_cpu_ms_ema,
        )
    }
}

impl RenderEngine {
    /// Set vector stat.
    pub(crate) fn set_vector_stat(&mut self, role: LaneRole, n: u32) {
        match role {
            LaneRole::Sea => self.sea_polygons = n,
            LaneRole::Landcover => self.landcover_polygons = n,
            LaneRole::Contours => self.contour_segments = n,
            LaneRole::Roads => self.road_segments = n,
            LaneRole::RoadsCasing => {}
            LaneRole::ForestFill => self.forest_polygons = n,
            LaneRole::ForestOutline => self.forest_outline_segments = n,
            _ => {}
        }
    }
}
