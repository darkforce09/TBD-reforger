//! **Role:** the render engine's statistics report: `stats()`, the one flat JSON object every
//! performance readout and gate reads, and `set_vector_stat`, which records a vector lane's count.
//! **Position:** the map renderer; the upload belts and `clear_vector_lane` record counts, the
//! asset loaders' `publish_engine` (through the asset sink's `stats_json`) and the Mission
//! Creator's debug HUD read `stats()`.
//! **Signals & state:** none of its own; it reads the engine's batch list, texture records, atlases,
//! compute cull, frame timer, counters, the building and forest layers' counters and
//! `RenderStats`, and records the vector forest counts into the forest layer.
//! **Invariants:** the report's keys, their order and each value's spelling (`{:.1}` for the
//! generation and upload times, `{:.3}` or `null` for the GPU frame time, `{:.4}` for the CPU
//! render times) are fixed: the HUD and the browser gates read keys by name, so a renamed or
//! reordered key changes what they read without a compile error. A sprite lane's count comes from
//! the compute cull when it runs, else from the lane's batch.

#[cfg(target_arch = "wasm32")]
use crate::engine::RenderEngine;
#[cfg(target_arch = "wasm32")]
use gpu_frame::frame::DrawPayload;
#[cfg(target_arch = "wasm32")]
use map_draw_lanes::lane_roles::LaneRole;
#[cfg(target_arch = "wasm32")]
use map_draw_lanes::lane_roles::lane_id;
use renderer_core::stats_json::StatsJson;

/// Every figure of the engine's statistics report, each named as its JSON key.
pub(crate) struct EngineStatisticsReport<'a> {
    /// `"webgpu"` or `"webgl2"`.
    pub(crate) backend: &'a str,
    /// Stress quads seeded.
    pub(crate) instances: u64,
    /// Stress quad batches.
    pub(crate) chunks: u64,
    /// Stress quad bytes plus the camera, calibration and unit-quad buffers.
    pub(crate) gpu_bytes: u64,
    /// The stress staging peak.
    pub(crate) staging_peak_bytes: u64,
    /// Stress generation milliseconds.
    pub(crate) gen_ms: f64,
    /// Stress upload enqueue milliseconds.
    pub(crate) upload_ms: f64,
    /// Uniform bytes written for the last frame.
    pub(crate) uniform_bytes_last_frame: u32,
    /// The last GPU pass time, when the timestamp timer has a sample.
    pub(crate) gpu_frame_ms: Option<f64>,
    /// The basemap texture mode, `"none"` without one.
    pub(crate) basemap_mode: &'a str,
    /// Basemap tiles.
    pub(crate) basemap_tiles: u32,
    /// Basemap and hillshade texture bytes.
    pub(crate) basemap_bytes: u64,
    /// World building fill instances.
    pub(crate) world_building_instances: u32,
    /// World building outline vertices.
    pub(crate) world_building_outline_vertices: u32,
    /// World chunks drawn.
    pub(crate) world_chunks_drawn: u32,
    /// Sea polygons.
    pub(crate) sea_polygons: u32,
    /// Landcover polygons.
    pub(crate) landcover_polygons: u32,
    /// Contour segments.
    pub(crate) contour_segments: u32,
    /// Road segments.
    pub(crate) road_segments: u32,
    /// Forest polygons.
    pub(crate) forest_polygons: u32,
    /// Forest outline segments.
    pub(crate) forest_outline_segments: u32,
    /// Forest density raster width.
    pub(crate) forest_density_w: u32,
    /// Forest density raster height.
    pub(crate) forest_density_h: u32,
    /// Forest density bins that loaded.
    pub(crate) forest_bins_ok: u32,
    /// The forest draw mode.
    pub(crate) forest_mode: &'a str,
    /// Tree sprites.
    pub(crate) tree_glyphs: u32,
    /// Prop sprites.
    pub(crate) prop_glyphs: u32,
    /// Building badge sprites.
    pub(crate) badge_glyphs: u32,
    /// Text labels drawn.
    pub(crate) text_labels_drawn: u32,
    /// Glyph, slot and text atlas bytes.
    pub(crate) atlas_bytes: u64,
    /// Slot sprites.
    pub(crate) slot_instances: u32,
    /// Dragged slot sprites.
    pub(crate) slot_drag_instances: u32,
    /// Cluster sprites.
    pub(crate) cluster_instances: u32,
    /// Mission vehicle sprites.
    pub(crate) mission_vehicles: u32,
    /// Whether the last frame was submitted.
    pub(crate) submitted_last_frame: bool,
    /// Whether the compute cull runs.
    pub(crate) compute_cull: bool,
    /// The compute cull's CPU oracle count.
    pub(crate) compute_cull_cpu_count: u32,
    /// The compute cull's GPU count.
    pub(crate) compute_cull_gpu_count: u32,
    /// Whether a GPU cull count has been read back.
    pub(crate) compute_cull_gpu_sampled: bool,
    /// Icon lane uploads.
    pub(crate) icon_lane_uploads: u64,
    /// Polygon lane uploads.
    pub(crate) polygon_lane_uploads: u64,
    /// Strip lane uploads.
    pub(crate) strip_lane_uploads: u64,
    /// Building uploads.
    pub(crate) building_uploads: u64,
    /// Text label uploads.
    pub(crate) text_label_uploads: u64,
    /// CPU milliseconds of the last submitted frame.
    pub(crate) render_cpu_ms_last: f64,
    /// Moving average of the CPU milliseconds per submitted frame.
    pub(crate) render_cpu_ms_ema: f64,
}

impl EngineStatisticsReport<'_> {
    /// The report as one flat JSON object, keys in their fixed order.
    pub(crate) fn to_json(&self) -> String {
        let mut json = StatsJson::new();
        json.text("backend", self.backend)
            .count("instances", self.instances)
            .count("chunks", self.chunks)
            .count("gpu_bytes", self.gpu_bytes)
            .count("staging_peak_bytes", self.staging_peak_bytes)
            .decimal("gen_ms", self.gen_ms, 1)
            .decimal("upload_ms", self.upload_ms, 1)
            .count("uniform_bytes_last_frame", self.uniform_bytes_last_frame)
            .optional_decimal("gpu_frame_ms", self.gpu_frame_ms, 3)
            .text("basemap_mode", self.basemap_mode)
            .count("basemap_tiles", self.basemap_tiles)
            .count("basemap_bytes", self.basemap_bytes)
            .count("world_building_instances", self.world_building_instances)
            .count(
                "world_building_outline_vertices",
                self.world_building_outline_vertices,
            )
            .count("world_chunks_drawn", self.world_chunks_drawn)
            .count("sea_polygons", self.sea_polygons)
            .count("landcover_polygons", self.landcover_polygons)
            .count("contour_segments", self.contour_segments)
            .count("road_segments", self.road_segments)
            .count("forest_polygons", self.forest_polygons)
            .count("forest_outline_segments", self.forest_outline_segments)
            .count("forest_density_w", self.forest_density_w)
            .count("forest_density_h", self.forest_density_h)
            .count("forest_bins_ok", self.forest_bins_ok)
            .text("forest_mode", self.forest_mode)
            .count("tree_glyphs", self.tree_glyphs)
            .count("prop_glyphs", self.prop_glyphs)
            .count("badge_glyphs", self.badge_glyphs)
            .count("text_labels_drawn", self.text_labels_drawn)
            .count("atlas_bytes", self.atlas_bytes)
            .count("slot_instances", self.slot_instances)
            .count("slot_drag_instances", self.slot_drag_instances)
            .count("cluster_instances", self.cluster_instances)
            .count("mission_vehicles", self.mission_vehicles)
            .flag("submitted_last_frame", self.submitted_last_frame)
            .flag("compute_cull", self.compute_cull)
            .count("compute_cull_cpu_count", self.compute_cull_cpu_count)
            .count("compute_cull_gpu_count", self.compute_cull_gpu_count)
            .flag("compute_cull_gpu_sampled", self.compute_cull_gpu_sampled)
            .count("icon_lane_uploads", self.icon_lane_uploads)
            .count("polygon_lane_uploads", self.polygon_lane_uploads)
            .count("strip_lane_uploads", self.strip_lane_uploads)
            .count("building_uploads", self.building_uploads)
            .count("text_label_uploads", self.text_label_uploads)
            .decimal("render_cpu_ms_last", self.render_cpu_ms_last, 4)
            .decimal("render_cpu_ms_ema", self.render_cpu_ms_ema, 4);
        json.finish()
    }
}

#[cfg(target_arch = "wasm32")]
impl RenderEngine {
    /// Machine-readable engine stats (every performance claim in the verify log is one of these
    /// numbers). `upload_ms` is CPU-side enqueue time for the staging→GPU copies; `gpu_frame_ms`
    /// is present only when `TIMESTAMP_QUERY` is available and sampled.
    #[must_use]
    pub fn stats(&self) -> String {
        self.statistics_report().to_json()
    }

    /// Debug JSON for `window.__wgpuSlotStats`: the engine stats with the slot symbology's
    /// flags appended.
    #[must_use]
    pub fn slot_stats_json(&self) -> String {
        self.slot_symbology.append_slot_stats(&self.stats())
    }

    /// Set vector stat: record `role`'s polygon or segment count; the casing and every
    /// non-vector lane record nothing.
    pub(crate) fn set_vector_stat(&mut self, role: LaneRole, n: u32) {
        match role {
            LaneRole::Sea | LaneRole::Landcover | LaneRole::Contours | LaneRole::Roads => {
                self.render_stats.set_lane_count(lane_id(role), n);
            }
            LaneRole::RoadsCasing => {}
            LaneRole::ForestFill => self.forest.record_fill_polygons(n),
            LaneRole::ForestOutline => self.forest.record_outline_segments(n),
            _ => {}
        }
    }

    /// Every figure of the report, read from the engine's current state.
    fn statistics_report(&self) -> EngineStatisticsReport<'_> {
        let stress_lane = lane_id(LaneRole::Stress);
        let chunks = self
            .batches
            .iter()
            .filter(|b| matches!(b.payload, DrawPayload::Quads(_)) && b.lane == stress_lane)
            .count() as u64;
        let stress_bytes: u64 = self
            .batches
            .iter()
            .filter(|b| b.lane == stress_lane)
            .map(|b| match &b.payload {
                DrawPayload::Quads(i) => u64::from(i.count) * 32,
                _ => 0,
            })
            .sum();

        // The basemap mode, tile and byte counters are the textured lanes' records, keyed by the
        // same lane the batch carries.
        let satellite = self.tex_lane(LaneRole::Satellite);
        let basemap_bytes: u64 = [LaneRole::Satellite, LaneRole::Hillshade]
            .into_iter()
            .filter_map(|r| self.tex_lane(r).map(|l| l.bytes()))
            .sum();

        let lane_batches = |role: LaneRole| {
            let lane = lane_id(role);
            self.batches.iter().filter(move |b| b.lane == lane)
        };
        let sprites = |role: LaneRole| {
            self.cull_lane_stat(
                role,
                lane_batches(role)
                    .map(|b| match &b.payload {
                        DrawPayload::Sprites { instances, .. } => instances.count,
                        _ => 0,
                    })
                    .sum(),
            )
        };

        EngineStatisticsReport {
            backend: self.gpu.backend_kind().as_str(),
            instances: self.stress_instances,
            chunks,
            gpu_bytes: stress_bytes + 64 + 32 + 64,
            staging_peak_bytes: self.staging_peak_bytes,
            gen_ms: self.gen_ms,
            upload_ms: self.upload_ms,
            uniform_bytes_last_frame: self.uniform_bytes_last_frame,
            gpu_frame_ms: self.timer.as_ref().and_then(|t| t.last_sample_ms()),
            basemap_mode: satellite.map_or("none", |l| l.mode().as_str()),
            basemap_tiles: satellite.map_or(0, |l| l.tiles()),
            basemap_bytes,
            world_building_instances: lane_batches(LaneRole::WorldBuildings)
                .map(|b| match &b.payload {
                    DrawPayload::OrientedQuads(i) => i.count,
                    _ => 0,
                })
                .sum(),
            world_building_outline_vertices: lane_batches(LaneRole::WorldBuildingsOutline)
                .map(|b| match &b.payload {
                    DrawPayload::Lines(v) => v.vertex_count,
                    _ => 0,
                })
                .sum(),
            world_chunks_drawn: self.buildings.chunks_drawn(),
            sea_polygons: self.render_stats.lane_count(lane_id(LaneRole::Sea)),
            landcover_polygons: self.render_stats.lane_count(lane_id(LaneRole::Landcover)),
            contour_segments: self.render_stats.lane_count(lane_id(LaneRole::Contours)),
            road_segments: self.render_stats.lane_count(lane_id(LaneRole::Roads)),
            forest_polygons: self.forest.polygons(),
            forest_outline_segments: self.forest.outline_segments(),
            forest_density_w: self.forest.density_width(),
            forest_density_h: self.forest.density_height(),
            forest_bins_ok: self.forest.bins_loaded(),
            forest_mode: self.forest.mode(),
            tree_glyphs: sprites(LaneRole::WorldTrees),
            prop_glyphs: sprites(LaneRole::WorldProps),
            badge_glyphs: sprites(LaneRole::WorldBadges),
            text_labels_drawn: self.text_labels_drawn,
            atlas_bytes: self.glyph_atlas.bytes()
                + self.slot_symbology.atlas_bytes()
                + self.text_atlas.as_ref().map_or(0, |a| a.bytes),
            slot_instances: sprites(LaneRole::Slots),
            slot_drag_instances: sprites(LaneRole::SlotDrag),
            cluster_instances: sprites(LaneRole::Clusters),
            mission_vehicles: sprites(LaneRole::MissionVehicles),
            submitted_last_frame: self.render_stats.submitted_last_frame(),
            compute_cull: self.icon_cull.enabled(),
            compute_cull_cpu_count: self.icon_cull.last_cpu_count(),
            compute_cull_gpu_count: self.icon_cull.gpu_count(),
            compute_cull_gpu_sampled: self.icon_cull.gpu_sampled(),
            icon_lane_uploads: self.icon_lane_uploads,
            polygon_lane_uploads: self.polygon_lane_uploads,
            strip_lane_uploads: self.strip_lane_uploads,
            building_uploads: self.buildings.uploads(),
            text_label_uploads: self.text_label_uploads,
            render_cpu_ms_last: self.render_stats.render_cpu_ms_last(),
            render_cpu_ms_ema: self.render_stats.render_cpu_ms_ema(),
        }
    }
}
