//! The world occluder: the resident state every world line-of-sight query reads.
//!
//! **Role:** declares [`WorldOccluder`], which holds the resident chunks' rows and box trees, the
//! prefab catalogue's kinds, labels and proxy boxes, the descriptors and BLAS sidecars that have
//! arrived, the prefabs expanded into traceable instances ([`PrefabOccluder`]) and the BLAS byte
//! budget ([`DEFAULT_BLAS_CAP_BYTES`]); and decides the best bounds a prefab's rows have now.
//! **Position:** filled by [`crate::residency`] as a host streams chunks, descriptors and BLAS
//! files in; queried by [`crate::raycast`] and [`crate::sight_line`]; described by
//! [`crate::occluder_readouts`].
//! **Signals & state:** owns every map above; a host owns the occluder and mutates it through the
//! residency calls only.
//! **Invariants:** a prefab's rows take the exact union of its expanded meshes, else its
//! descriptor's bounds, else the catalogue proxy; a prefab that never blocks has no bounds and
//! never crosses anything.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use building_interiors::compound::instances::Instance;
use geometry_primitives::axis_aligned_box::Bounds3;
use map_coordinates::chunk_math::TerrainSizeM;
use spatial_indexes::bounding_volume_hierarchy::sidecar::BvhSidecar;
use world_chunks::chunk_id::ChunkId;

use crate::chunk_occluder::ChunkOccluder;
use crate::occluder_library::PrefabDescriptor;

/// The BLAS byte budget a new occluder starts with: 48 MiB.
pub const DEFAULT_BLAS_CAP_BYTES: usize = 48 << 20;

/// What the prefab catalogue says about a pid: its kind, its label and its proxy box.
#[derive(Clone, Debug)]
pub(crate) struct PrefabInfo {
    /// The catalogue kind (`building`, `tree`, `prop`, …).
    pub(crate) kind: String,

    /// The display label.
    pub(crate) label: String,

    /// The proxy box in the object frame, when the catalogue knows the half extents.
    pub(crate) proxy: Option<Bounds3>,
}

/// An expanded descriptor: its instance list ready to trace.
#[derive(Debug)]
pub struct PrefabOccluder {
    /// The catalogue prefab expanded.
    pub pid: u16,

    /// Every placed mesh of the prefab, root first, in the prefab's object frame.
    pub instances: Vec<Instance>,

    /// The union of the instances' placed boxes, object frame.
    pub local_bounds: Bounds3,

    /// Whether any instance carries foliage triangles.
    pub has_foliage: bool,

    /// Every BLAS path the instances use, in first-use order.
    pub blas_paths: Vec<String>,
}

/// The world occluder over the resident chunks of one terrain.
pub struct WorldOccluder {
    /// The chunk size, metres.
    pub(crate) chunk_m: f64,

    /// The terrain size the chunk walk is bounded by.
    pub(crate) terrain: TerrainSizeM,

    /// The resident chunks.
    pub(crate) chunks: HashMap<ChunkId, ChunkOccluder>,

    /// The prefab catalogue, by pid.
    pub(crate) info: HashMap<u16, PrefabInfo>,

    /// The descriptors that arrived, by pid.
    pub(crate) descriptors: HashMap<u16, Arc<PrefabDescriptor>>,

    /// The BLAS sidecars that arrived, by path.
    pub(crate) blas: HashMap<String, Arc<BvhSidecar>>,

    /// Each sidecar's heap bytes, by path.
    pub(crate) blas_bytes: HashMap<String, usize>,

    /// The tick each sidecar was last inserted at, by path.
    pub(crate) blas_last_use: HashMap<String, u64>,

    /// The occluder's clock for `blas_last_use`.
    pub(crate) tick: u64,

    /// The prefabs expanded into traceable instances, by pid.
    pub(crate) expanded: HashMap<u16, Arc<PrefabOccluder>>,

    /// The pids whose descriptor said they never block.
    pub(crate) no_block: HashSet<u16>,

    /// How many resident rows place each pid.
    pub(crate) placed: HashMap<u16, u32>,

    /// The chunks whose boxes and tree are stale until the next refresh.
    pub(crate) dirty: HashSet<ChunkId>,

    /// The BLAS byte budget.
    pub(crate) cap_bytes: usize,
}

impl WorldOccluder {
    /// An empty occluder over a terrain of `terrain` metres cut into `chunk_m`-metre chunks.
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

    /// The object-frame bounds a row of `pid` has now and whether they are a proxy, or `None`
    /// when the prefab never blocks or nothing is known of its size.
    pub(crate) fn bounds_of(&self, pid: u16) -> Option<(Bounds3, bool)> {
        if self.no_block.contains(&pid) {
            return None;
        }
        if let Some(po) = self.expanded.get(&pid) {
            return Some((po.local_bounds, false));
        }
        if let Some(d) = self.descriptors.get(&pid) {
            if !d.blocks {
                return None;
            }
            if let Some(b) = d.local_bounds {
                return Some((b, true));
            }
        }
        self.info.get(&pid).and_then(|i| i.proxy).map(|b| (b, true))
    }
}
