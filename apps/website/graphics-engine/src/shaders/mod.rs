//! Role: shaders.
//! Position: `apps/website/graphics-engine/src` — WGSL sources.
//! Signals & state: shader text, compiled by the device builder.
//! Invariants: the `.wgsl` files live beside this module so `include_str!` stays relative —
//! a relative include is a compile-time proof that the source is inside this crate, which is
//! the wall doing its job. Reaching one with `../../..` would mean the file is in the wrong place.

/// The map shader: every vertex/fragment entry point plus the icon-cull compute kernel.
pub const SHADER_WGSL: &str = include_str!("shader.wgsl");

#[cfg(test)]
#[path = "tests/contract_tests.rs"]
mod tests;
