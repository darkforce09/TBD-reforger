//! The map renderer.
//!
//! **Role:** `RenderEngine`, the renderer of the Mission Creator's map canvas: it creates the
//! canvas's GPU through `gpu_device`'s context, builds the shader, layouts and pipelines, owns the
//! camera and the persistent batch list, holds the symbology and world typed layers as fields and
//! lends them its lanes, writes the vector, marquee and label lanes, encodes one frame packet per
//! damaged frame, reports its statistics and serves the streaming host as its asset sink.
//! **Position:** map rendering tier 5, over the renderer contracts (`renderer_core`), the GPU
//! device and frame crates, the typed layers (`symbology_layers_gpu`, `world_layers_gpu`) and the
//! streaming model's asset sink contract; the Mission Creator and the debug benches create it and
//! drive it through `EngineHandle`, the frame pump renders it and the render diagnostics read it
//! through `diagnostic_accessors`.
//! **Signals & state:** one `RenderEngine` per canvas, single-threaded, shared through
//! `EngineHandle`.
//! **Invariants:** the renderer is damage-driven (a frame is submitted only when something marked
//! it damaged) and refills its batch list and packet tables in place, never per frame. Every
//! module that names a GPU or browser type is `wasm32` only; the native build keeps [`Error`], and
//! the tests add the surface size policy, the statistics report and the calibration scene.

#[cfg(target_arch = "wasm32")]
mod asset_sink;
#[cfg(target_arch = "wasm32")]
mod bindings;
#[cfg(target_arch = "wasm32")]
mod boot;
#[cfg(any(target_arch = "wasm32", test))]
mod calibration_scene;
#[cfg(target_arch = "wasm32")]
mod cull;
#[cfg(target_arch = "wasm32")]
pub mod diagnostic_accessors;
#[cfg(target_arch = "wasm32")]
pub mod encode;
#[cfg(target_arch = "wasm32")]
pub mod engine;
#[cfg(any(target_arch = "wasm32", test))]
mod engine_statistics;
mod error;
#[cfg(target_arch = "wasm32")]
mod lane_sinks;
#[cfg(target_arch = "wasm32")]
mod lifecycle;
pub mod prelude;
#[cfg(target_arch = "wasm32")]
mod pump;
#[cfg(any(target_arch = "wasm32", test))]
mod surface_size;
#[cfg(target_arch = "wasm32")]
mod typed_layers;
#[cfg(target_arch = "wasm32")]
mod upload;
#[cfg(target_arch = "wasm32")]
mod viewport;

#[cfg(target_arch = "wasm32")]
pub use engine::{CLEAR_COLOR, EngineHandle, RenderEngine};
pub use error::{Error, Result};

// The source pins of the damage-driven frame path and of the mission lane bind functions read
// source text, never a GPU, so they run natively.
#[cfg(test)]
#[path = "tests/damage_discipline.rs"]
mod damage_discipline;

#[cfg(test)]
#[path = "tests/lane_bind_source_pins/mod.rs"]
mod lane_bind_source_pins;
