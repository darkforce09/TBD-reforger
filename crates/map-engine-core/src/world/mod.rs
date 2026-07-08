//! T-151.2 (W2) — world-object parser ported to Rust, proven byte-exact against the JS
//! `worldObjectsCore` oracle: SoA columns are **Class R** (bit-identical `Float32Array`/
//! `Uint16Array`/`Uint8Array` stores), per-render-class row sets are **Class S**, and the OBB
//! corners + road centerline are **Class T** (≤ 1 ULP). Pure compute — no `wasm-bindgen`, no
//! deck.gl; the JS boundary is the `map-engine-wasm` `WorldStore` handle.

mod chunk;
mod chunk_math;
mod classify;
mod index;
mod manifest;
mod obb;
mod prefab;
mod regions;
mod residency;
mod roads;
mod store;

pub use chunk::{WorldChunk, parse_chunk};
pub use chunk_math::{
    Bbox, ChunkRect, TerrainSizeM, chunk_id, chunk_ids_for_rect, chunk_ids_for_viewport,
    chunk_rect_for_bbox, expand_bbox, expand_chunk_rect, preload_margin_m,
};
pub use classify::{
    NO_CLASS, OVERSIZED_HALF_EXTENT_M, RENDER_CLASS_CODES, class_code, narrow_instance_row,
    render_class_for_prefab,
};
pub use index::WorldSpatialIndex;
pub use manifest::{
    ChunkCell, DEFAULT_CHUNK_SIZE_M, ObjectsManifest, narrow_cells, parse_objects_manifest,
};
pub use obb::{BuildingPrefabInfo, building_prefab_lookup, obb_corners};
pub use prefab::{PrefabEntry, PrefabRow, build_prefab_maps, narrow_prefab_rows};
pub use regions::{LandCoverRegion, parse_regions_payload};
pub use residency::{APPLY_BUDGET_MS, BUILDING_MIN_ZOOM, LRU_MIN_CHUNKS, WorldResidency};
pub use roads::{
    CENTERLINE_DEDUPE_M, RoadSegment, extract_road_centerline, parse_roads_payload,
    road_style_width,
};
pub use store::{WorldError, WorldStore};
