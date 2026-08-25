//! T-934.4 — Mission Creator & Eden 2D editor nest. Grows through the T-934
//! program: library/tools/world_assets first, panels/state/arsenal/canvas in
//! later batches.

pub mod library;
// T-172 B9 — pure SZ payload estimator (missionSize.ts port), native-tested.
pub mod mission_size;
pub mod tools;
// T-159.28 map-asset host (MVP: DEM hillshade) — fetch bytes + call the Rust dem core + engine
// tex_layer. wasm32-only (fetch + engine), gated like the doc host.
#[cfg(target_arch = "wasm32")]
pub mod world_assets;
// T-173 P6 — per-user world-layer visibility prefs + basemap view (localStorage). Pure/native-
// tested; the wasm host applies them to the residency + engine each settle.
pub mod world_layer_prefs;
