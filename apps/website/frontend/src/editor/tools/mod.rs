//! T-934.4 — interactive map tools.

// T-643 — line-of-sight tool.
pub mod los_tool;
// T-645 — shared placement helpers.
pub mod place_helpers;
// T-642 — ruler / measure tool.
pub mod ruler_tool;
// T-159.18 Select / LMB pick foundation — links map-engine-core `camera`+`spatial` and web-sys, so
// wasm32-only, gated like the doc host + persist modules.
#[cfg(target_arch = "wasm32")]
pub mod select_tool;
// T-090.12.5 — the LOS tool's object layer: the pure verdict / wash logic and its wasm adapter
// onto the world occluder (`world_assets::with_occluder`).
pub mod los_world;
#[cfg(target_arch = "wasm32")]
pub mod los_world_wasm;
