//! The renderer contracts a map renderer and its typed layers meet at.
//!
//! **Role:** the map-agnostic seams between a renderer and the layers that fill it: the lane sink
//! a layer writes its draw batches through (`lane_sink::LaneSink`), the borrowed GPU context a
//! layer builds with (`layer_context::LayerContext`), the per-frame callbacks the renderer runs
//! before it encodes ([`frame_hook`]), the frame and lane counters with the JSON writer the
//! renderer reports them through ([`render_stats`], [`stats_json`]), and the fixed pipeline and
//! bind-group ids a frame packet's tables are indexed by ([`packet_bindings`]).
//! **Position:** graphics tier 2 over `gpu_frame` (the frame vocabulary) and `render_primitives`
//! (the opaque ids); the map renderer implements the lane sink, owns the statistics and the hooks,
//! and its typed layers write through these contracts.
//! **Signals & state:** the statistics' counters and the hook list, owned by whoever holds them;
//! the contracts themselves hold none.
//! **Invariants:** no type, function or document here names a thing in the world being drawn; a
//! lane is an opaque `LaneId`, never a caller's lane role. The lane sink and the layer context
//! name `wgpu` types and are `wasm32` only; the native build keeps the statistics, the JSON
//! writer, the frame hooks and the binding ids, which the native tests cover.

pub mod frame_hook;
#[cfg(target_arch = "wasm32")]
pub mod lane_sink;
#[cfg(target_arch = "wasm32")]
pub mod layer_context;
pub mod packet_bindings;
pub mod prelude;
pub mod render_stats;
pub mod stats_json;
