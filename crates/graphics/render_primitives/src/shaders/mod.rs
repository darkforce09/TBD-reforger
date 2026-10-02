//! Role: the WGSL source of every render entry point and the sprite-cull compute kernel.
//! Position: `shaders` in `render_primitives`; the graphics engine compiles it into its
//! pipelines, and the cull oracle and the layout tests read it as text.
//! Signals & state: none; shader text embedded at compile time.
//! Invariants: the `.wgsl` file lives beside this module so `include_str!` stays relative, a
//! compile-time proof that the source is inside this crate.

/// The shader source: every vertex and fragment entry point plus the icon-cull compute kernel.
pub const SHADER_WGSL: &str = include_str!("shader.wgsl");

#[cfg(test)]
#[path = "tests/contract_tests.rs"]
mod tests;
