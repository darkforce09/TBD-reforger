//! The Arsenal paper doll's renderer: the soldier drawn on its own canvas, and its self-check.
//!
//! **Role:** `renderer::PaperDollRenderer` (wasm32) creates the GPU of a canvas through
//! `gpu_device::GpuContext`, draws the soldier of `paper_doll_scene` as instanced cubes and one
//! cylinder with depth testing whenever something changed, takes the Arsenal's region states,
//! hover, turns and resizes, answers picks and callout anchors, and runs a byte-exact offscreen
//! readback self-check; [`instance_packing`] packs the instance stream it draws from.
//! **Position:** paper doll category, tier 3, over `gpu_device`, `paper_doll_scene` and
//! `camera_math`; the Arsenal host in the frontend
//! (`apps/frontend/src/workspaces/editor/arsenal/doll.rs`) creates and drives it and puts its
//! self-check on `window.__arsenalDoll`.
//! **Signals & state:** the GPU handles, buffers, yaw, CSS size, region states, hover and dirty
//! flag inside one renderer, owned by its host; single-threaded.
//! **Invariants:** everything that names a GPU or browser type is `wasm32` only, so the native
//! build keeps the instance packing and [`Error`] and the native tests cover them; the renderer
//! never acquires a frame when nothing changed.

mod error;
pub mod instance_packing;
pub mod prelude;

#[cfg(target_arch = "wasm32")]
mod doll_draw;
#[cfg(target_arch = "wasm32")]
mod frame_render;
#[cfg(target_arch = "wasm32")]
mod pipeline;
#[cfg(target_arch = "wasm32")]
pub mod renderer;
#[cfg(target_arch = "wasm32")]
mod self_check;

pub use error::{Error, Result};
#[cfg(target_arch = "wasm32")]
pub use renderer::PaperDollRenderer;
