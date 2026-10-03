//! The GPU frame of the renderer: the frame vocabulary, the draw path, the render pipelines and
//! the animation-frame pump.
//!
//! **Role:** turns a caller's uploaded geometry and sorted draw batches into WebGPU draws: the
//! frame vocabulary a caller describes one frame in ([`frame`]), the vertex and index uploads,
//! the frame encoder and the compute sprite cull ([`draw`]), the render pipeline constructors
//! ([`pipeline`]) and the shared `requestAnimationFrame` pump ([`frame_pump`]).
//! **Position:** graphics tier 1 over `render_primitives` (byte layouts, frame ids, camera
//! uniform, text uniform packing, cull oracle and WGSL source); the map engine's render engine
//! builds its packets, uploads and pipelines with it and is driven by its pump. The device,
//! surface and queue a caller passes in come from `gpu_device`.
//! **Signals & state:** GPU handles the caller owns, the compute cull's per-lane buffers and the
//! pump's target cell, disposal flag and frame counter.
//! **Invariants:** no type, function or document here names a thing in the world being drawn;
//! the caller decides what to draw and in which order, and this crate binds and draws what a
//! packet names. Module declarations are ungated: each file that names a GPU or browser type
//! gates itself on `wasm32`, so the native build keeps the pump's tick and the cull's source
//! check, which the native tests cover.

pub mod draw;
mod error;
pub mod frame;
pub mod frame_pump;
pub mod pipeline;
pub mod prelude;

pub use error::{Error, Result};
