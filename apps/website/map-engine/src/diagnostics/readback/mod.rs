//! Role: Module boundary for diagnostics/readback.
//! Position: `diagnostics/readback` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Scene.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod scene;

/// Texture.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod texture;

/// World building.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod world_building;

/// Sea band.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod sea_band;

/// Road centerline.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod road_centerline;

/// Tree glyph.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod tree_glyph;

/// Text.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod text;

/// Marquee.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod marquee;

/// Compute cull.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod compute_cull;

/// Doll.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod doll;
