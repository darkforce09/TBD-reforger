//! The map renderer's diagnostics.
//!
//! **Role:** how the render engine measures itself in the browser: byte-exact offscreen readback
//! checks of each pipeline and of the engine's calibration quads, the one-pixel readback of the
//! live scene, the frame benchmark and the stress pool, each a function over the engine.
//! **Position:** map rendering tier above `map_renderer`; it reads the engine only through
//! `map_renderer::diagnostic_accessors` and draws with `gpu_frame`'s pipelines and encoder; the
//! Mission Creator's viewport bridge publishes the functions on `window.__selfChecks` and
//! `window.__editorBench`, and the editor gate calls them there.
//! **Signals & state:** none of its own; a check builds and drops its own target, pipelines and
//! buffers, and only the stress pool writes the engine (its batch list, texture records, lane pool,
//! staging buffer and stress counters).
//! **Invariants:** a check never writes the engine's frame tables, camera uniform or batch list,
//! so it can run beside the live render loop; expected pixels are exact bytes. Every module that
//! names a GPU or browser type is `wasm32` only; the native build keeps the stress scene generator.

/// The frame benchmark, the stress pool and the stress scene it fills.
pub mod benchmark;
pub mod prelude;
/// The byte-exact offscreen readback checks, the calibration check and the scene readback.
#[cfg(target_arch = "wasm32")]
pub mod readback;
