//! Role: draw.
//! Position: `legacy/graphics_engine/src` — the GPU half of the draw path; the CPU geometry it
//! uploads comes from `render_primitives::draw`.
//! Signals & state: vertex/index streams and the frame encoder.
//! Invariants: takes geometry, returns geometry. It never asks what a shape represents.

/// Frustum compaction of packed sprite instances: the GPU compute pass, and the CPU oracle it is
/// checked against.
pub mod cull;

/// Encoding a frame packet into a render pass.
#[cfg(target_arch = "wasm32")]
pub mod encode;

/// `LineList` vertex streams.
#[cfg(target_arch = "wasm32")]
pub mod lines;

/// Indexed triangle meshes.
#[cfg(target_arch = "wasm32")]
pub mod polygons;
