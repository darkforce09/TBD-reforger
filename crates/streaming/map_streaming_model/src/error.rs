//! Why a write through the map asset sink is refused.
//!
//! **Role:** the crate's one error type and its `Result` alias: the renderer behind a
//! [`crate::asset_sink::MapAssetSink`] refused a texture layer, a texture write, the glyph atlas
//! or the forest density raster.
//! **Position:** returned by the fallible sink calls; the loaders log it or give up the load.
//! **Signals & state:** none; plain data.
//! **Invariants:** the message names the refused sink call and carries the renderer's own account
//! verbatim.

/// Why a sink write is refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The renderer refused an asset write.
    #[error("the renderer refused {operation}: {reason}")]
    AssetRejected {
        /// The sink call that was refused, such as `tex_layer_begin`.
        operation: &'static str,

        /// The renderer's account of the refusal.
        reason: String,
    },
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
