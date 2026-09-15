//! Role: state.
//! Position: `spatial/world_los` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::bvh::sidecar::BvhSidecar;
use crate::spatial::world_los::coverage_1::DEFAULT_BLAS_CAP_BYTES;
use crate::spatial::world_los::coverage_1::PrefabInfo;
use crate::spatial::world_los::coverage_1::PrefabOccluder;
use crate::spatial::world_los::descriptor::PrefabDescriptor;
use crate::spatial::world_los::placed::ChunkOccluder;
use crate::streaming::scheduler::chunk_math::TerrainSizeM;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

/// World occluder.
pub struct WorldOccluder {
    /// Chunk m.
    pub(crate) chunk_m: f64,

    /// Terrain.
    pub(crate) terrain: TerrainSizeM,

    /// Chunks.
    pub(crate) chunks: HashMap<String, ChunkOccluder>,

    /// Info.
    pub(crate) info: HashMap<u16, PrefabInfo>,

    /// Descriptors.
    pub(crate) descriptors: HashMap<u16, Arc<PrefabDescriptor>>,

    /// Blas.
    pub(crate) blas: HashMap<String, Arc<BvhSidecar>>,

    /// Blas bytes.
    pub(crate) blas_bytes: HashMap<String, usize>,

    /// Blas last use.
    pub(crate) blas_last_use: HashMap<String, u64>,

    /// Tick.
    pub(crate) tick: u64,

    /// Expanded.
    pub(crate) expanded: HashMap<u16, Arc<PrefabOccluder>>,

    /// No block.
    pub(crate) no_block: HashSet<u16>,

    /// Placed.
    pub(crate) placed: HashMap<u16, u32>,

    /// Dirty.
    pub(crate) dirty: HashSet<String>,

    /// Cap bytes.
    pub(crate) cap_bytes: usize,
}

impl WorldOccluder {
    /// New.
    #[must_use]
    pub fn new(chunk_m: f64, terrain: TerrainSizeM) -> Self {
        Self {
            chunk_m,
            terrain,
            chunks: HashMap::new(),
            info: HashMap::new(),
            descriptors: HashMap::new(),
            blas: HashMap::new(),
            blas_bytes: HashMap::new(),
            blas_last_use: HashMap::new(),
            tick: 0,
            expanded: HashMap::new(),
            no_block: HashSet::new(),
            placed: HashMap::new(),
            dirty: HashSet::new(),
            cap_bytes: DEFAULT_BLAS_CAP_BYTES,
        }
    }
}
