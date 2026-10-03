//! **Role:** the engine's lanes as `renderer_core` lane sinks: the engine's own sink, and the two
//! borrows of its lane state a typed layer writes through while it is itself borrowed from the
//! engine.
//! **Position:** the map renderer; `lifecycle.rs` and the upload belts write through the engine's
//! sink, `typed_layers/` builds the borrowed sinks.
//! **Signals & state:** none of its own; each sink writes the engine's batch list, texture records
//! and damage flag.
//! **Invariants:** every sink keeps the same rules: ordered upsert and removal, a texture record
//! that lives exactly as long as its batch, and damage marked on every change that alters what is
//! drawn.

/// `RenderEngine` as `renderer_core`'s `LaneSink`.
pub(crate) mod engine_lane_sink;

/// The engine's lanes borrowed apart for a layer that writes textured lanes.
pub(crate) mod textured_lanes;

/// The engine's lanes borrowed apart for a layer that writes untextured lanes only.
pub(crate) mod untextured_lanes;
