//! The errors of the GPU frame.
//!
//! **Role:** the one error type of the crate: why a cell atlas could not be built.
//! **Position:** returned by [`crate::frame`]'s atlas constructors; the renderer maps it into its
//! own error at its boundary.
//! **Signals & state:** none.
//! **Invariants:** a message is the atlas's stable tag (`text-atlas-rgba-size`,
//! `glyph-atlas-rgba-size`), which logs and readouts match on.

/// Why a cell atlas could not be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The RGBA8 pixels are not `width × height × 4` bytes long.
    #[error("{atlas}-rgba-size")]
    AtlasPixelLength {
        /// The atlas's label: `text-atlas` or `glyph-atlas`.
        atlas: &'static str,
        /// The atlas width in texels.
        width: u32,
        /// The atlas height in texels.
        height: u32,
        /// The byte length the caller passed.
        actual: usize,
    },
}

/// The result of a GPU frame call.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
#[path = "tests/error_tests.rs"]
mod tests;
