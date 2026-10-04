//! The crate's error: why a map view could not mount.
//!
//! **Role:** the one failure type of the mount (`mount::mount_map_view`) and of the engine
//! creation it shares with the Mission Creator's boot (`engine_mount::create_engine`).
//! **Position:** built by the browser half from the manifest fetch and the render engine's start;
//! read by the map picker, which words each variant, and by the Mission Creator's boot, which
//! shows the engine's reason.
//! **Signals & state:** none; plain data.
//! **Invariants:** [`Error::EngineFailed`] keeps the render engine's own message, whose `Display`
//! is that message byte for byte (it starts with the engine's stable code, `canvas-zero-size`,
//! `no-adapter`, …).

/// A result whose failure is the crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Why a map view could not mount.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// `/map-assets/<terrain>/manifest.json` was unreachable or had no usable `worldBounds`.
    #[error("the terrain manifest is unavailable")]
    ManifestUnavailable,

    /// The render engine could not start; the engine's reason.
    #[error("{0}")]
    EngineFailed(String),
}
