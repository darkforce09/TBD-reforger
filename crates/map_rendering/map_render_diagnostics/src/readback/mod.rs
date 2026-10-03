//! **Role:** the byte-exact offscreen readback checks: the engine's calibration quads, each
//! pipeline's own probe scene, the live scene's one-pixel readback, and the readback helpers they
//! share.
//! **Position:** `readback` of the map render diagnostics; the Mission Creator's viewport bridge
//! publishes each check on `window.__selfChecks`; every check reads the engine only through
//! `map_renderer::diagnostic_accessors`.
//! **Signals & state:** none; each check builds and drops its own target, pipelines and buffers.
//! **Invariants:** a check draws into its own `Rgba8Unorm` target with its own camera, never into
//! the engine's tables, and resolves a promise to a JSON verdict.

/// The calibration check: seven pixel probes over the engine's two calibration quads.
pub mod calibration;

/// The shared readback helpers and the live scene's one-pixel readback.
pub mod scene;

/// The textured pipeline's check.
pub mod texture;

/// The oriented quad (building) pipeline's check.
pub mod world_building;

/// The polygon pipeline's sea band check.
pub mod sea_band;

/// The polygon pipeline's road centreline check.
pub mod road_centerline;

/// The icon pipeline's tree glyph check.
pub mod tree_glyph;

/// The text pipeline's check.
pub mod text;

/// The marquee's translucent fill and outline check.
pub mod marquee;

/// The compute cull's agreement with the CPU oracle.
pub mod compute_cull;
