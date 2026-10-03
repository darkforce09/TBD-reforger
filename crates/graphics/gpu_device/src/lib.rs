//! The GPU device of a browser canvas: one bootstrap, the pooled lane buffers, the readback guard
//! and the frame timer.
//!
//! **Role:** stands up and owns the GPU of one canvas (`context::gpu_context::GpuContext`:
//! instance, surface, adapter facts, device, queue and surface configuration, with its resize
//! and swapchain acquire), supplies the web display handle every instance is built with
//! (`context::web_display`), sizes and reuses per-lane vertex buffers ([`buffers::pool`]),
//! guards a readback buffer's one-at-a-time mapping ([`buffers::readback`]) and times a render
//! pass with timestamp queries (`timing::gpu_timer`).
//! **Position:** graphics tier 0 with no workspace dependency; the bootstrap is the one a map
//! renderer and the Arsenal paper doll renderer create their GPU with, `gpu_frame` draws with the
//! device and queue it hands out, and the map engine reaches the buffers and the frame timer
//! through re-exports.
//! **Signals & state:** the GPU handles of one canvas inside a `GpuContext`, the pool's buffers
//! and host copies, and the readback and timer cells; all single-threaded.
//! **Invariants:** no type, function or document here names a thing in the world being drawn.
//! Every item that names a GPU or browser type is `wasm32` only; the native build keeps the pool
//! arithmetic, the readback guard and the surface policy, which the native tests cover.

pub mod buffers;
pub mod context;
mod error;
pub mod prelude;
pub mod timing;

#[cfg(target_arch = "wasm32")]
pub use context::frame_acquire::{Acquired, acquire_frame};
#[cfg(target_arch = "wasm32")]
pub use context::gpu_context::GpuContext;
pub use context::surface_policy::BackendKind;
#[cfg(target_arch = "wasm32")]
pub use context::web_display::{WebDisplay, instance_descriptor};
pub use error::{Error, Result};
#[cfg(target_arch = "wasm32")]
pub use timing::gpu_timer::GpuTimer;
