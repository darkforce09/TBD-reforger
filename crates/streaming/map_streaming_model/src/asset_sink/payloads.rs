//! The CPU payloads the map asset sink's texture, atlas, forest and glyph calls take.
//!
//! **Role:** borrowed or copied descriptions of one write: a texture layer to allocate, a region
//! of it to fill, the world glyph atlas, the forest density raster, and which world glyph lane an
//! icon upload targets.
//! **Position:** built by the map host and its loaders, handed to
//! [`crate::asset_sink::MapAssetSink`] methods, read by the renderer's implementation.
//! **Signals & state:** none; plain data.
//! **Invariants:** world coordinates are metres on the map plane (`x` east, `y` north); pixel
//! sizes and offsets are in texels of the named mip level; byte buffers are RGBA8 rows unless the
//! field says otherwise.

/// A texture layer to allocate: which layer, the world rectangle it covers and the texture's
/// size, mip chain and sampling mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextureLayerSpec {
    /// The texture layer: `0` the basemap, `1` the hillshade.
    pub role: u32,

    /// The world corner the texture's first texel covers, in metres.
    pub world_min: [f64; 2],

    /// The opposite world corner, in metres.
    pub world_max: [f64; 2],

    /// Texture width of mip level 0, in texels.
    pub width: u32,

    /// Texture height of mip level 0, in texels.
    pub height: u32,

    /// Mip levels the texture holds, at least one.
    pub mip_count: u32,

    /// The renderer's sampling mode for the layer: `0` the unified mip chain, `2` a single
    /// image, `3` the hillshade.
    pub mode: u32,
}

/// One rectangle of a texture layer's mip level that a write fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureRegion {
    /// The texture layer, as in [`TextureLayerSpec::role`].
    pub role: u32,

    /// The mip level written, relative to the layer's level 0.
    pub mip: u32,

    /// Left edge of the rectangle, in texels.
    pub x: u32,

    /// Top edge of the rectangle, in texels.
    pub y: u32,

    /// Rectangle width, in texels.
    pub width: u32,

    /// Rectangle height, in texels.
    pub height: u32,
}

/// The world glyph atlas: the decoded RGBA image and one UV rectangle per glyph key.
#[derive(Clone, Copy, Debug)]
pub struct GlyphAtlasImage<'a> {
    /// RGBA8 texels, `width * height * 4` bytes.
    pub rgba: &'a [u8],

    /// Atlas width, in texels.
    pub width: u32,

    /// Atlas height, in texels.
    pub height: u32,

    /// Four normalised floats per glyph, `[u0, v0, u1, v1]`, in the residency's glyph key order.
    pub uv: &'a [f32],
}

/// The island's forest density raster with the world rectangle it covers.
#[derive(Clone, Copy, Debug)]
pub struct ForestDensityRaster<'a> {
    /// The world corner of the raster's first texel, in metres.
    pub world_min: [f64; 2],

    /// The opposite world corner, in metres.
    pub world_max: [f64; 2],

    /// Raster width, in texels.
    pub width: u32,

    /// Raster height, in texels.
    pub height: u32,

    /// RGBA8 texels with the density count in red, north as row 0, each row padded to
    /// `bytes_per_row`.
    pub rgba: &'a [u8],

    /// Bytes per raster row, padding included: at least `width * 4` and a multiple of 256.
    pub bytes_per_row: u32,

    /// Density bins that loaded and were stitched into the raster.
    pub bins_loaded: u32,
}

/// The world glyph lane an icon upload replaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldGlyphLane {
    /// Tree glyphs.
    Trees,

    /// Prop glyphs.
    Props,

    /// Building badge glyphs.
    Badges,
}
