//! Role: sprite frustum culling, CPU side.
//! Position: `draw` in `render_primitives`; the graphics engine's compute cull is checked
//! against it.
//! Signals & state: none; packed 20-byte sprite records and a 4-float rect, in meters.
//! Invariants: `oracle` is the reference the GPU compute pass must agree with; it reads the
//! `cs_icon_cull` kernel from `shaders/shader.wgsl` by its relative path.

/// CPU reference implementation.
pub mod oracle;
