//! **Role:** the symbol atlas's GPU upload.
//! **Position:** `overlay/symbology/atlas` in the map engine; the atlas cells it uploads are
//! `unit_symbology::symbol_atlas`.
//! **Signals & state:** the atlas texture, owned by `gpu`.
//! **Invariants:** preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// The atlas texture upload.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod gpu;
