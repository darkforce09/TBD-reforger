//! **Role:** module boundary of the world object loader: `WorldHost` and the imports its parts
//! share.
//! **Position:** `world_loader` in `map_asset_loading`; the map host owns one `WorldHost`, boots it
//! and runs a viewport pass on each settle.
//! **Signals & state:** none here; the host's state is `state::WorldHost`.
//! **Invariants:** the loader reaches the renderer only through the asset sink handle.

use crate::asset_statistics::BridgeHandle;
use crate::asset_statistics::publish_engine;
use crate::browser_asset_sink::BrowserAssetSinkHandle;
use crate::mesh_composition::LandcoverInput;
use crate::mesh_composition::compose_landcover_mesh;
use crate::occluder_loader::OccluderHost;
use browser_platform::fetch::fetch_bytes;
use browser_platform::fetch::fetch_text;
use chunk_draw_buffers::world_residency::WorldResidency;
use map_streaming_model::boot_progress::BootEvent;
use map_streaming_model::boot_progress::BootSeg;
use render_primitives::draw::compose::PolyMeshGpu;
use road_network::mesh::RoadInput;
use road_network::mesh::RoadMeshGpu;
use road_network::mesh::compose_roads_mesh;
use road_network::styling::road_class_signature;
use std::collections::VecDeque;
use vegetation::regions::regions_from_bytes;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use world_chunks::chunk_container::chunk_bin_path;
use world_chunks::terrain_manifest::parse_manifest_binary;
use world_store::store::WorldStore;
mod viewport;
use viewport::FETCH_CONCURRENCY;
mod atlas;
use atlas::load_glyph_atlas;
mod state;

/// Re-export `state::WorldHost`.
pub use state::WorldHost;
use state::{AtlasUpload, PendingChunk};
mod metrics;
use metrics::CrossingAllocProbe;
mod bootstrap;
mod ingest;
mod terrain;
mod upload;
