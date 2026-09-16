//! Role: Module boundary for streaming/loaders/world_loader.
//! Position: `streaming/loaders/world_loader` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::context::handles::EngineHandle;
use crate::renderers::primitives::compose::LandcoverInput;
use crate::renderers::primitives::compose::PolyMeshGpu;
use crate::renderers::primitives::compose::compose_landcover_mesh;
use crate::streaming::bridge::progress::BootEvent;
use crate::streaming::bridge::progress::BootSeg;
use crate::streaming::bridge::statistics::BridgeHandle;
use crate::streaming::bridge::statistics::publish_engine;
use crate::streaming::loaders::chunk_bin::chunk_bin_path;
use crate::streaming::loaders::fetch::fetch_bytes;
use crate::streaming::loaders::fetch::fetch_text;
use crate::streaming::loaders::manifest::parse_manifest_binary;
use crate::streaming::loaders::occluder_loader::OccluderHost;
use crate::streaming::loaders::store::WorldStore;
use crate::streaming::scheduler::state::WorldResidency;
use crate::world::environment::vegetation::regions::regions_from_bytes;
use crate::world::terrain::roads::mesh::RoadInput;
use crate::world::terrain::roads::mesh::RoadMeshGpu;
use crate::world::terrain::roads::mesh::compose_roads_mesh;
use crate::world::terrain::roads::styling::road_class_signature;
use std::collections::VecDeque;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
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
