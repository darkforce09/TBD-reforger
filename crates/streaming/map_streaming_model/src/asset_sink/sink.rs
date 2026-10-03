//! The asset writes the map host and its loaders make into the renderer.
//!
//! **Role:** [`MapAssetSink`]: every call the terrain boot, the world, forest, label and relief
//! loaders and the satellite loads make into the renderer, as CPU payloads: renderer facts,
//! layer visibility, vector lanes, world objects, labels, the forest density lane and the
//! texture layers.
//! **Position:** declared in the streaming category so the loaders never name the renderer;
//! implemented by the map renderer; reached through a
//! [`crate::asset_sink::SharedMapAssetSink`].
//! **Signals & state:** none here; every write lands in the implementation's GPU state.
//! **Invariants:** payloads are CPU data only (render primitives, byte and float buffers, and
//! the opaque [`MapAssetSink::BrowserImage`]); a lane role is a
//! `map_draw_lanes::lane_roles::role_id` value; a visibility flag applies to the lane the call
//! writes.

use crate::Result;
use crate::asset_sink::payloads::{
    ForestDensityRaster, GlyphAtlasImage, TextureLayerSpec, TextureRegion, WorldGlyphLane,
};
use crate::asset_sink::viewport::MapViewport;
use render_primitives::draw::compose::{HairlineGpu, PolyMeshGpu};

/// The renderer as the map host and its loaders write to it.
pub trait MapAssetSink: MapViewport {
    /// The browser's decoded image handle a texture write can take without a CPU copy (an
    /// `ImageBitmap` in the browser).
    type BrowserImage;

    /// Whether the renderer runs on WebGL2, which takes texture writes as RGBA bytes only.
    fn backend_is_webgl2(&self) -> bool;

    /// The largest 2D texture edge the device was granted, in texels.
    fn max_texture_dimension_2d(&self) -> u32;

    /// The largest 2D texture edge the adapter offered, in texels.
    fn adapter_max_texture_dimension_2d(&self) -> u32;

    /// The renderer's counters as one JSON object (lane uploads, vector counts, forest state).
    fn stats_json(&self) -> String;

    /// Show or hide a world layer by its preference key (`roads`, `forest`, `contours`, `sea`,
    /// `airfield`, `heights`, `townLabels`, `roadNames`).
    fn set_world_layer_visible(&mut self, layer: &str, visible: bool);

    /// Set a texture layer's opacity and visibility (`role` as in [`TextureLayerSpec::role`]).
    fn set_lane_opacity(&mut self, role: u32, opacity: f32, visible: bool);

    /// Lay the map grid over a `width_m` by `height_m` terrain, drawn over the hillshade or not.
    fn set_grid(&mut self, width_m: f64, height_m: f64, over_hillshade: bool, visible: bool);

    /// Empty a vector lane.
    fn clear_vector_lane(&mut self, role: u32);

    /// Replace a vector lane with a triangulated polygon mesh.
    fn upload_polygon_mesh(&mut self, role: u32, mesh: &PolyMeshGpu, visible: bool);

    /// Replace a vector lane with triangle strips, packed `[x, y, r, g, b, a]` per vertex.
    fn upload_strip_tris(&mut self, role: u32, packed: &[f32], item_count: u32, visible: bool);

    /// Replace a vector lane with hairline segments.
    fn upload_hairline_segments(&mut self, role: u32, hairlines: &HairlineGpu, visible: bool);

    /// Replace the world building fills, packed instances over `chunk_count` pinned chunks.
    fn upload_world_buildings(&mut self, fill: &[f32], chunk_count: u32, visible: bool);

    /// Replace the world building outlines, packed line instances.
    fn upload_world_building_outlines(&mut self, lines: &[f32], visible: bool);

    /// Replace the fence, pier and bridge-rail strips, `item_count` packed segments.
    fn upload_world_fence_strips(&mut self, packed: &[f32], item_count: u32, visible: bool);

    /// Replace one world glyph lane with packed icon instances; empty bytes clear the lane.
    fn upload_icon_lane(&mut self, lane: WorldGlyphLane, bytes: &[u8], visible: bool);

    /// Upload the world glyph atlas the icon lanes sample.
    ///
    /// # Errors
    /// [`crate::Error::AssetRejected`] when the renderer cannot create the atlas texture.
    fn upload_glyph_atlas(&mut self, atlas: &GlyphAtlasImage<'_>) -> Result<()>;

    /// Replace the town labels with packed label bytes.
    fn upload_town_labels(&mut self, bytes: &[u8], visible: bool);

    /// Replace the road name labels with packed label bytes.
    fn upload_road_labels(&mut self, bytes: &[u8], visible: bool);

    /// Replace the spot height labels with packed text icon bytes.
    fn upload_text_labels(&mut self, bytes: &[u8], visible: bool);

    /// Upload the island's forest density raster.
    ///
    /// # Errors
    /// [`crate::Error::AssetRejected`] when the raster's size or row padding is invalid.
    fn forest_density_upload(&mut self, raster: &ForestDensityRaster<'_>) -> Result<()>;

    /// Set the forest fill alpha and the fill and outline visibility.
    fn forest_density_set_params(
        &mut self,
        fill_alpha: f32,
        fill_visible: bool,
        outline_visible: bool,
    );

    /// Record how many forest outline segments the outline lane holds.
    fn forest_outline_set_stored(&mut self, segments: u32);

    /// Allocate a texture layer; the writes that follow fill it and
    /// [`MapAssetSink::tex_layer_commit`] shows it.
    ///
    /// # Errors
    /// [`crate::Error::AssetRejected`] for an unknown layer or a zero size.
    fn tex_layer_begin(&mut self, layer: &TextureLayerSpec) -> Result<()>;

    /// Fill a region of the allocated layer from RGBA8 bytes.
    ///
    /// # Errors
    /// [`crate::Error::AssetRejected`] when no layer is allocated or the region falls outside it.
    fn tex_layer_write_rgba(&mut self, region: TextureRegion, rgba: &[u8]) -> Result<()>;

    /// Fill a region of the allocated layer from a decoded browser image.
    ///
    /// # Errors
    /// [`crate::Error::AssetRejected`] when no layer is allocated or the region falls outside it.
    fn tex_layer_write_bitmap(
        &mut self,
        region: TextureRegion,
        image: Self::BrowserImage,
    ) -> Result<()>;

    /// Show the allocated layer at `opacity`, replacing the one it supersedes.
    ///
    /// # Errors
    /// [`crate::Error::AssetRejected`] when no layer of that role was begun.
    fn tex_layer_commit(&mut self, role: u32, opacity: f32, visible: bool) -> Result<()>;
}
