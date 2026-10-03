//! The errors of the symbology layers.
//!
//! **Role:** the one error type of the crate: why an atlas upload was refused.
//! **Position:** returned by the glyph atlas layer's upload and the slot symbology's slot atlas
//! upload; the renderer and the Mission Creator log it or map it into their own errors.
//! **Signals & state:** none.
//! **Invariants:** a message starts with the atlas's stable tag (`glyph-atlas-uv-count`,
//! `slot-atlas-rgba-size`, or the GPU frame's `glyph-atlas-rgba-size`), which logs and readouts
//! match on.

/// Why an atlas upload was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The glyph atlas's UV table holds more `f32` values than the icon uniform block.
    #[error("glyph-atlas-uv-count: capacity {capacity}, got {actual}")]
    GlyphAtlasUvCount {
        /// The `f32` values the uniform block's UV table holds.
        capacity: usize,
        /// The `f32` values the caller passed.
        actual: usize,
    },

    /// The slot atlas's RGBA8 pixels are not `width × height × 4` bytes long.
    #[error("slot-atlas-rgba-size")]
    SlotAtlasPixelLength {
        /// The atlas width in texels.
        width: u32,
        /// The atlas height in texels.
        height: u32,
        /// The byte length the caller passed.
        actual: usize,
    },

    /// The GPU frame refused to build the glyph atlas.
    #[error(transparent)]
    GlyphAtlasBuild(#[from] gpu_frame::Error),
}

/// The result of a symbology layer call.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
#[path = "tests/error_tests.rs"]
mod tests;
