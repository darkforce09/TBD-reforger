//! Role: state.
//! Position: `streaming/loaders/world_loader` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::CrossingAllocProbe;
use super::OccluderHost;
use super::PolyMeshGpu;
use super::RoadMeshGpu;
use super::VecDeque;
use super::WorldResidency;
use super::WorldStore;

/// Pending chunk.
pub(super) struct PendingChunk {
    /// Id.
    pub(super) id: String,

    /// Bytes.
    pub(super) bytes: Option<Vec<u8>>,

    /// Binary.
    pub(super) binary: bool,
}

/// Atlas upload.
pub(super) struct AtlasUpload {
    /// Rgba.
    pub(super) rgba: Vec<u8>,

    /// W.
    pub(super) w: u32,

    /// H.
    pub(super) h: u32,

    /// Uv.
    pub(super) uv: Vec<f32>,

    /// Keys.
    pub(super) keys: Vec<String>,
}

/// World host.
pub struct WorldHost {
    /// World layers.
    pub(super) world_layers: fn() -> crate::streaming::bridge::preferences::WorldLayerPrefs,

    /// Residency.
    pub(super) residency: WorldResidency,

    /// Store.
    pub(super) store: WorldStore,

    /// Asset base.
    pub(super) asset_base: String,

    /// Chunks path.
    pub(super) chunks_path: String,

    /// Chunks bin.
    pub(super) chunks_bin: Option<String>,

    /// Ready.
    pub(super) ready: bool,

    /// Pending.
    pub(super) pending: VecDeque<PendingChunk>,

    /// Atlas.
    pub(super) atlas: Option<AtlasUpload>,

    /// Atlas uploaded.
    pub(super) atlas_uploaded: bool,

    /// Roads loaded.
    pub(super) roads_loaded: bool,

    /// Landcover ready.
    pub(super) landcover_ready: bool,

    /// Road meshes.
    pub(super) road_meshes: std::collections::HashMap<u8, RoadMeshGpu>,

    /// Last road sig.
    pub(super) last_road_sig: Option<u8>,

    /// Landcover mesh.
    pub(super) landcover_mesh: Option<PolyMeshGpu>,

    /// Landcover shown.
    pub(super) landcover_shown: bool,

    /// Last pushed.
    pub(super) last_pushed: Option<(u64, bool, bool)>,

    /// Crossing allocs.
    pub(super) crossing_allocs: CrossingAllocProbe,

    /// Occluder.
    pub(super) occluder: OccluderHost,
}

impl WorldHost {
    /// New.
    pub fn new(
        world_layers: fn() -> crate::streaming::bridge::preferences::WorldLayerPrefs,
    ) -> Self {
        Self {
            world_layers,
            residency: WorldResidency::new(),
            store: WorldStore::new(),
            asset_base: String::new(),
            chunks_path: String::new(),
            chunks_bin: None,
            ready: false,
            pending: VecDeque::new(),
            atlas: None,
            atlas_uploaded: false,
            roads_loaded: false,
            landcover_ready: false,
            road_meshes: std::collections::HashMap::new(),
            last_road_sig: None,
            landcover_mesh: None,
            landcover_shown: false,
            last_pushed: None,
            crossing_allocs: CrossingAllocProbe::new(),
            occluder: OccluderHost::new(),
        }
    }
}

impl Default for WorldHost {
    fn default() -> Self {
        Self::new(crate::streaming::bridge::preferences::WorldLayerPrefs::default)
    }
}
