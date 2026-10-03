//! The contract the map host and its loaders write the renderer through.
//!
//! **Role:** the [`MapAssetSink`] trait with its [`MapViewport`] supertrait, the CPU payload
//! types its calls take, and the [`SharedMapAssetSink`] handle over a [`MapAssetSinkSlot`].
//! **Position:** streaming category, so the loaders depend on this contract and never on the
//! renderer; the map renderer implements it.
//! **Signals & state:** the shared handle's `RefCell`; nothing else.
//! **Invariants:** no GPU and no browser type is named here; the browser image is the sink's
//! associated type.

pub mod payloads;
pub mod sink;
pub mod slot;
pub mod viewport;

pub use payloads::{
    ForestDensityRaster, GlyphAtlasImage, TextureLayerSpec, TextureRegion, WorldGlyphLane,
};
pub use sink::MapAssetSink;
pub use slot::{MapAssetSinkSlot, SharedMapAssetSink};
pub use viewport::MapViewport;

#[cfg(test)]
#[path = "tests/shared_sink_tests.rs"]
mod tests;
