//! **Role:** the render engine as the map host's asset sink: `map_streaming_model`'s
//! `MapViewport` and `MapAssetSink` implemented over `RenderEngine`'s camera, lane and label
//! methods, the symbology layers and the world layers (buildings, forest, terrain textures).
//! **Position:** the map renderer; the streaming host and the asset loaders hold the engine cell
//! as a `SharedMapAssetSink` and write through these impls.
//! **Signals & state:** none of its own; every call lands in the engine's state.
//! **Invariants:** each method forwards to the engine method, or the symbology or world layer or
//! lane preference function, of the same purpose with the payload unpacked in order; a refused
//! write keeps the refusing layer's message, code first, as the error's reason.

use crate::engine::RenderEngine;
use crate::typed_layers::symbology_layers::SymbologyParts;
use crate::typed_layers::world_layers::WorldParts;
use map_streaming_model::asset_sink::{
    ForestDensityRaster, GlyphAtlasImage, MapAssetSink, MapViewport, TextureLayerSpec,
    TextureRegion, WorldGlyphLane,
};
use map_streaming_model::{Error, Result};
use render_primitives::draw::compose::{HairlineGpu, PolyMeshGpu};
use renderer_core::lane_sink::LaneSink;
use symbology_layers_gpu::lane_preferences;
use symbology_layers_gpu::world_icon_lanes::upload_world_icon_lane;

/// The sink error for a write a typed layer refused with `error`.
fn rejected(operation: &'static str, error: impl Into<crate::Error>) -> Error {
    Error::AssetRejected {
        operation,
        reason: error.into().to_string(),
    }
}

impl MapViewport for RenderEngine {
    fn zoom(&self) -> f64 {
        RenderEngine::zoom(self)
    }

    fn target_x(&self) -> f64 {
        RenderEngine::target_x(self)
    }

    fn target_y(&self) -> f64 {
        RenderEngine::target_y(self)
    }

    fn visible_bounds(&self) -> Option<[f64; 4]> {
        let bounds = RenderEngine::visible_bounds(self);
        bounds.get(..4)?.try_into().ok()
    }

    fn set_view(&mut self, target_x: f64, target_y: f64, zoom: f64) {
        RenderEngine::set_view(self, target_x, target_y, zoom);
    }

    fn on_camera_changed(&mut self) {
        RenderEngine::on_camera_changed(self);
    }
}

impl MapAssetSink for RenderEngine {
    type BrowserImage = web_sys::ImageBitmap;

    fn backend_is_webgl2(&self) -> bool {
        self.backend() == "webgl2"
    }

    fn max_texture_dimension_2d(&self) -> u32 {
        RenderEngine::max_texture_dimension_2d(self)
    }

    fn adapter_max_texture_dimension_2d(&self) -> u32 {
        RenderEngine::adapter_max_texture_dimension_2d(self)
    }

    fn stats_json(&self) -> String {
        self.stats()
    }

    fn set_world_layer_visible(&mut self, layer: &str, visible: bool) {
        lane_preferences::set_world_layer_visible(self, layer, visible);
    }

    fn set_lane_opacity(&mut self, role: u32, opacity: f32, visible: bool) {
        lane_preferences::set_lane_opacity(self, role, opacity, visible);
    }

    fn set_grid(&mut self, width_m: f64, height_m: f64, over_hillshade: bool, visible: bool) {
        lane_preferences::set_grid(self, width_m, height_m, over_hillshade, visible);
    }

    fn clear_vector_lane(&mut self, role: u32) {
        RenderEngine::clear_vector_lane(self, role);
    }

    fn upload_polygon_mesh(&mut self, role: u32, mesh: &PolyMeshGpu, visible: bool) {
        RenderEngine::upload_polygon_mesh(
            self,
            role,
            &mesh.positions,
            &mesh.colors,
            &mesh.indices,
            mesh.polygon_count,
            visible,
        );
    }

    fn upload_strip_tris(&mut self, role: u32, packed: &[f32], item_count: u32, visible: bool) {
        RenderEngine::upload_strip_tris(self, role, packed, item_count, visible);
    }

    fn upload_hairline_segments(&mut self, role: u32, hairlines: &HairlineGpu, visible: bool) {
        RenderEngine::upload_hairline_segments(
            self,
            role,
            &hairlines.verts,
            hairlines.segment_count,
            visible,
        );
    }

    fn upload_world_buildings(&mut self, fill: &[f32], chunk_count: u32, visible: bool) {
        let mut parts = self.world_parts();
        parts
            .buildings
            .upload_buildings(&mut parts.lanes, fill, chunk_count, visible);
    }

    fn upload_world_building_outlines(&mut self, lines: &[f32], visible: bool) {
        let mut parts = self.world_parts();
        parts
            .buildings
            .upload_outlines(&mut parts.lanes, lines, visible);
    }

    fn upload_world_fence_strips(&mut self, packed: &[f32], item_count: u32, visible: bool) {
        let WorldParts {
            mut lanes,
            buildings,
            strip_lane_uploads,
            ..
        } = self.world_parts();
        buildings.upload_fence_strips(&mut lanes, strip_lane_uploads, packed, item_count, visible);
    }

    fn upload_icon_lane(&mut self, lane: WorldGlyphLane, bytes: &[u8], visible: bool) {
        let kind = match lane {
            WorldGlyphLane::Trees => 0,
            WorldGlyphLane::Props => 1,
            WorldGlyphLane::Badges => 2,
        };
        let SymbologyParts {
            mut lanes,
            icon_cull,
            icon_lane_uploads,
            ..
        } = self.symbology_parts();
        upload_world_icon_lane(
            &mut lanes,
            icon_cull,
            icon_lane_uploads,
            kind,
            bytes,
            visible,
        );
    }

    fn upload_glyph_atlas(&mut self, atlas: &GlyphAtlasImage<'_>) -> Result<()> {
        let mut parts = self.symbology_parts();
        let context = parts.lanes.layer_context();
        parts
            .glyph_atlas
            .upload(&context, atlas.rgba, atlas.width, atlas.height, atlas.uv)
            .map_err(|error| rejected("upload_glyph_atlas", error))
    }

    fn upload_town_labels(&mut self, bytes: &[u8], visible: bool) {
        RenderEngine::upload_town_labels(self, bytes, visible);
    }

    fn upload_road_labels(&mut self, bytes: &[u8], visible: bool) {
        RenderEngine::upload_road_labels(self, bytes, visible);
    }

    fn upload_text_labels(&mut self, bytes: &[u8], visible: bool) {
        RenderEngine::upload_text_labels(self, bytes, visible);
    }

    fn forest_density_upload(&mut self, raster: &ForestDensityRaster<'_>) -> Result<()> {
        let mut parts = self.world_parts();
        parts
            .forest
            .upload_density(
                &mut parts.lanes,
                raster.world_min[0],
                raster.world_min[1],
                raster.world_max[0],
                raster.world_max[1],
                raster.width,
                raster.height,
                raster.rgba,
                raster.bytes_per_row,
                raster.bins_loaded,
            )
            .map_err(|error| rejected("forest_density_upload", error))
    }

    fn forest_density_set_params(
        &mut self,
        fill_alpha: f32,
        fill_visible: bool,
        outline_visible: bool,
    ) {
        let mut parts = self.world_parts();
        parts
            .forest
            .set_params(&mut parts.lanes, fill_alpha, fill_visible, outline_visible);
    }

    fn forest_outline_set_stored(&mut self, segments: u32) {
        self.forest.set_outline_stored(segments);
    }

    fn tex_layer_begin(&mut self, layer: &TextureLayerSpec) -> Result<()> {
        let mut parts = self.world_parts();
        parts
            .terrain_textures
            .begin(
                &mut parts.lanes,
                layer.role,
                layer.world_min[0],
                layer.world_min[1],
                layer.world_max[0],
                layer.world_max[1],
                layer.width,
                layer.height,
                layer.mip_count,
                layer.mode,
            )
            .map_err(|error| rejected("tex_layer_begin", error))
    }

    fn tex_layer_write_rgba(&mut self, region: TextureRegion, rgba: &[u8]) -> Result<()> {
        let mut parts = self.world_parts();
        parts
            .terrain_textures
            .write_rgba(
                &mut parts.lanes,
                region.role,
                region.mip,
                region.x,
                region.y,
                region.width,
                region.height,
                rgba,
            )
            .map_err(|error| rejected("tex_layer_write_rgba", error))
    }

    fn tex_layer_write_bitmap(
        &mut self,
        region: TextureRegion,
        image: web_sys::ImageBitmap,
    ) -> Result<()> {
        let mut parts = self.world_parts();
        parts
            .terrain_textures
            .write_bitmap(
                &mut parts.lanes,
                region.role,
                region.mip,
                region.x,
                region.y,
                region.width,
                region.height,
                image,
            )
            .map_err(|error| rejected("tex_layer_write_bitmap", error))
    }

    fn tex_layer_commit(&mut self, role: u32, opacity: f32, visible: bool) -> Result<()> {
        let mut parts = self.world_parts();
        parts
            .terrain_textures
            .commit(&mut parts.lanes, role, opacity, visible)
            .map_err(|error| rejected("tex_layer_commit", error))
    }
}
