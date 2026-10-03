//! The errors of the world layers.
//!
//! **Role:** the one error type of the crate: why a world layer refused a raster or texture
//! write, and [`LayerCall`], the layer call that refused it.
//! **Position:** returned by the forest layer's density upload, the terrain texture layer's begin,
//! write and commit, and the terrain line of sight overlay's viewshed upload; the map renderer's
//! asset sink and the Mission Creator log it or map it into their own errors.
//! **Signals & state:** none.
//! **Invariants:** a message starts with the refusing call's stable tag ([`LayerCall::tag`]),
//! followed by the fixed reason text that logs and readouts match on.

use std::fmt;

/// The layer call that refused a write, named by the stable tag its message starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerCall {
    /// The forest layer's density raster upload (`forest_density_upload`).
    ForestDensityUpload,

    /// The terrain line of sight overlay's viewshed raster upload (`viewshed_upload`).
    ViewshedUpload,

    /// The terrain texture layer's begin (`tex_layer`).
    TextureBegin,

    /// The terrain texture layer's browser bitmap write (`tex_layer_write_bitmap`).
    TextureWriteBitmap,

    /// The terrain texture layer's RGBA write (`tex_layer_write_rgba`).
    TextureWriteRgba,

    /// The terrain texture layer's commit (`tex_layer_commit`).
    TextureCommit,
}

impl LayerCall {
    /// The stable tag a refusal's message starts with.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::ForestDensityUpload => "forest_density_upload",
            Self::ViewshedUpload => "viewshed_upload",
            Self::TextureBegin => "tex_layer",
            Self::TextureWriteBitmap => "tex_layer_write_bitmap",
            Self::TextureWriteRgba => "tex_layer_write_rgba",
            Self::TextureCommit => "tex_layer_commit",
        }
    }
}

impl fmt::Display for LayerCall {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.tag())
    }
}

/// Why a world layer refused a raster or texture write.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A raster or texture with a zero width, height or mip count.
    #[error("{call}: zero texture dimensions")]
    ZeroTextureDimensions {
        /// The refusing call.
        call: LayerCall,
    },

    /// A raster row shorter than four bytes per texel, or not a multiple of 256 bytes.
    #[error("{call}: bytes_per_row must be ≥ tex_w*4 and 256-aligned")]
    RasterRowPitch {
        /// The refusing call.
        call: LayerCall,
        /// The row pitch the caller passed, in bytes.
        bytes_per_row: u32,
        /// The raster width in texels.
        width: u32,
    },

    /// A raster whose byte size does not fit in memory.
    #[error("{call}: size overflow")]
    RasterSizeOverflow {
        /// The refusing call.
        call: LayerCall,
    },

    /// A raster whose bytes are not `bytes_per_row × height` long.
    #[error("{call}: rgba length mismatch")]
    RasterByteLength {
        /// The refusing call.
        call: LayerCall,
        /// `bytes_per_row × height`.
        expected: usize,
        /// The byte length the caller passed.
        actual: usize,
    },

    /// A texture role other than 0 (basemap) or 1 (hillshade).
    #[error("tex_layer: role must be 0 (basemap) or 1 (hillshade)")]
    TextureRole {
        /// The role the caller passed.
        role: u32,
    },

    /// An RGBA tile whose bytes are not `w × h × 4` long.
    #[error("tex_layer_write_rgba: byte length != w*h*4")]
    TileByteLength {
        /// `w × h × 4`.
        expected: usize,
        /// The byte length the caller passed.
        actual: usize,
    },

    /// A write or commit for a role with no pending texture.
    #[error("{call}: begin not called")]
    TextureNotBegun {
        /// The refusing call.
        call: LayerCall,
        /// The role the caller passed.
        role: u32,
    },
}

/// The result of a world layer call.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
#[path = "tests/error_tests.rs"]
mod tests;
