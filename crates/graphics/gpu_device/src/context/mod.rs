//! The GPU context of one browser canvas.
//!
//! **Role:** the bootstrap both renderers share (`gpu_context::GpuContext`), the swapchain
//! acquire (`frame_acquire`), the web display handle every instance is built with
//! (`web_display`) and the GPU-free policy they apply ([`surface_policy`]).
//! **Position:** under the crate root; the map engine's render engine and the paper doll renderer
//! call it, and `gpu_frame` encodes into the images it acquires.
//! **Signals & state:** the GPU handles inside a `GpuContext`; the policy is pure.
//! **Invariants:** the shaders, layouts and pipelines a renderer builds are the renderer's, never
//! this module's; `surface_policy` names no GPU type, so the native tests cover it.

/// The swapchain acquire: the next image, a skip, or a typed failure.
#[cfg(target_arch = "wasm32")]
pub mod frame_acquire;

/// `GpuContext`: instance, surface, adapter facts, device, queue and surface configuration.
#[cfg(target_arch = "wasm32")]
pub mod gpu_context;

/// The GPU-free decisions of the bootstrap: sizes, surface format, backend kind, timestamps.
pub mod surface_policy;

/// The web display handle and the instance descriptor built on it.
#[cfg(target_arch = "wasm32")]
pub mod web_display;
