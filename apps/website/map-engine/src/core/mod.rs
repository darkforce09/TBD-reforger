//! Role: Module boundary for core.
//! Position: `core` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Buffers.
// T-0xx Phase 1C: moved to `website-graphics-engine`. Re-exported at its former path so
// every call site in this crate keeps its spelling — the move is a relocation, not a rename.
//
// T-0xx Phase 2B: this line is one of the two `device`/`pipeline` sites gate rule 3b still
// allows, and it is allowed here rather than at its three call sites because collapsing it
// would turn one named seam into three. `LanePool` and `ReadbackLane` are GPU buffer
// bookkeeping that belongs to graphics-engine and lives there; this crate has to *name* them
// only because `RenderEngine` holds them, and `RenderEngine` did not cross in Phase 1. The
// re-export travels to `frame/mod.rs` with the rest of this hub, which is where rule 3a wants
// every graphics name. See `xtask/src/gate_engine_layers.rs` for the pinned list.
pub use website_graphics_engine::device::buffers;

/// Context.
pub mod context;

/// Culling.
pub mod culling;

/// Pipeline.
pub mod pipeline;
