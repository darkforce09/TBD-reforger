//! The map-agnostic building blocks of the renderer: byte layouts, CPU geometry, glyphs and the
//! shader source.
//!
//! **Role:** holds everything the renderer needs that touches no GPU handle: the per-instance
//! vertex layouts and line vertex, colour normalisation, triangulation and fill / hairline
//! composition, the procedural grid, the CPU sprite-cull reference, the frame ids, damage
//! tracking and camera uniform, the bitmap font with its atlas bake, glyph layout and packing,
//! and the WGSL source.
//! **Position:** graphics tier 0, depending on `bytemuck` and `earcutr` only. The graphics engine
//! builds its pipelines, buffers and frame encoding on it; callers pack instance bytes with it.
//! **Signals & state:** none; plain-old-data types, constant tables and pure functions.
//! **Invariants:** no type, function or document here names a thing in the world being drawn;
//! a layout is named for its shape. Every byte layout matches the WGSL vertex inputs and uniform
//! blocks in `shaders/shader.wgsl`, pinned by tests on the native target.

pub mod color_normalization;
pub mod draw;
pub mod frame;
pub mod prelude;
pub mod shaders;
pub mod text;
