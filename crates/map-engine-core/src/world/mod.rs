//! T-151.2 (W2) — world-object parser ported to Rust, proven byte-exact against the JS
//! `worldObjectsCore` oracle: SoA columns are **Class R** (bit-identical `Float32Array`/
//! `Uint16Array`/`Uint8Array` stores), per-render-class row sets are **Class S**, and the OBB
//! corners + road centerline are **Class T** (≤ 1 ULP). Pure compute — no `wasm-bindgen`, no
//! deck.gl; the JS boundary is the `map-engine-wasm` `WorldStore` handle.

mod chunk;
mod classify;
mod manifest;
mod obb;
mod prefab;
mod regions;
mod roads;
mod store;

pub use chunk::{WorldChunk, parse_chunk};
pub use classify::{
    NO_CLASS, OVERSIZED_HALF_EXTENT_M, RENDER_CLASS_CODES, class_code, narrow_instance_row,
    render_class_for_prefab,
};
pub use manifest::{
    ChunkCell, DEFAULT_CHUNK_SIZE_M, ObjectsManifest, narrow_cells, parse_objects_manifest,
};
pub use obb::{BuildingPrefabInfo, building_prefab_lookup, obb_corners};
pub use prefab::{PrefabEntry, PrefabRow, build_prefab_maps, narrow_prefab_rows};
pub use regions::{LandCoverRegion, parse_regions_payload};
pub use roads::{
    CENTERLINE_DEDUPE_M, RoadSegment, extract_road_centerline, parse_roads_payload,
    road_style_width,
};
pub use store::{WorldError, WorldStore};
