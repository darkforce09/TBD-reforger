//! **Role:** the engine's world typed layers at work: the split borrow that lends the building,
//! forest, terrain texture and terrain line of sight overlay layers the engine's lanes, and the
//! accessor the Mission Creator drives the terrain line of sight overlay through.
//! **Position:** the map renderer; the asset sink forwards the world loaders' building, forest and
//! texture writes to the layers through `RenderEngine::world_parts`, and the Mission Creator and
//! the debug building viewer call `RenderEngine::with_terrain_line_of_sight_overlay`.
//! **Signals & state:** none of its own; `WorldParts` borrows disjoint engine fields.
//! **Invariants:** a world layer reaches the engine only through the parts lent here (the lane sink
//! with its texture records and the strip upload counter every strip lane shares); the engine
//! names the world layers' types, never the reverse.

use crate::engine::RenderEngine;
use crate::lane_sinks::textured_lanes::TexturedLanes;
use crate::lane_sinks::untextured_lanes::UntexturedLanes;
use world_layers_gpu::building_layer::BuildingLayerGpu;
use world_layers_gpu::forest_layer::ForestLayerGpu;
use world_layers_gpu::terrain_line_of_sight_overlay::{
    TerrainLineOfSightOverlay, TerrainLineOfSightOverlayGpu,
};
use world_layers_gpu::terrain_texture_layer::TerrainTextureLayerGpu;

/// The engine split into the world layers and the parts they write through.
pub(crate) struct WorldParts<'a> {
    /// The engine's lanes, textured lanes included.
    pub(crate) lanes: TexturedLanes<'a>,

    /// The building footprints, outlines and fence strips.
    pub(crate) buildings: &'a mut BuildingLayerGpu,

    /// The forest density lane and the forest lane settings.
    pub(crate) forest: &'a mut ForestLayerGpu,

    /// The satellite basemap and hillshade texture lanes.
    pub(crate) terrain_textures: &'a mut TerrainTextureLayerGpu,

    /// The viewshed lane.
    pub(crate) terrain_line_of_sight_overlay: &'a TerrainLineOfSightOverlayGpu,

    /// The strip lane upload counter.
    pub(crate) strip_lane_uploads: &'a mut u64,
}

impl RenderEngine {
    /// The engine split into the world layers and the parts they write through.
    pub(crate) fn world_parts(&mut self) -> WorldParts<'_> {
        let Self {
            gpu,
            batches,
            frame_pipelines,
            frame_bind_groups,
            tex_lanes,
            render_stats,
            damage,
            buildings,
            forest,
            terrain_textures,
            terrain_line_of_sight_overlay,
            strip_lane_uploads,
            ..
        } = self;
        WorldParts {
            lanes: TexturedLanes {
                lanes: UntexturedLanes {
                    batches,
                    tex_lanes,
                    damage,
                    device: gpu.device(),
                    queue: gpu.queue(),
                    surface_format: gpu.surface_format(),
                    frame_pipelines,
                    frame_bind_groups,
                    render_stats,
                },
            },
            buildings,
            forest,
            terrain_textures,
            terrain_line_of_sight_overlay,
            strip_lane_uploads,
        }
    }

    /// Run `work` on the terrain line of sight overlay, lent the engine's lanes: the one door the
    /// Mission Creator's line of sight tools and the building viewer upload and clear the
    /// viewshed wash through.
    pub fn with_terrain_line_of_sight_overlay<R>(
        &mut self,
        work: impl FnOnce(&mut TerrainLineOfSightOverlay<'_>) -> R,
    ) -> R {
        let mut parts = self.world_parts();
        let mut overlay = parts
            .terrain_line_of_sight_overlay
            .at_work(&mut parts.lanes);
        work(&mut overlay)
    }
}
