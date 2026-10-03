//! The target-neutral model of map streaming.
//!
//! **Role:** what the map host, its loaders and an embedding frontend share without naming a
//! browser or a GPU: the world-layer switches ([`world_layer_preferences`]), the boot scope and
//! preference readers ([`host_preferences`]), the boot progress events and the Range planning
//! ([`boot_progress`]), the memory budget ledger and its satellite floor walk
//! ([`memory_budget`]), and the [`asset_sink::MapAssetSink`] contract the loaders write their
//! CPU payloads through.
//! **Position:** streaming category, tier 1, over `render_primitives`; the map engine's streaming
//! host and loaders and the frontend's map view and Mission Creator import it, and the renderer
//! implements its sink.
//! **Signals & state:** none at the root; [`asset_sink`] hands out shared sink handles,
//! everything else is plain data and pure functions.
//! **Invariants:** no browser crate, no GPU crate; every type compiles and tests natively.

pub mod asset_sink;
pub mod boot_progress;
mod error;
pub mod host_preferences;
pub mod memory_budget;
pub mod prelude;
pub mod world_layer_preferences;

pub use error::{Error, Result};
