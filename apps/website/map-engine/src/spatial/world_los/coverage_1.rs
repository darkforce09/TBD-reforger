//! Role: coverage 1.
//! Position: `spatial/world_los` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::architecture::blueprint::attribution_1::LosHit;
use crate::architecture::compound::instances::Instance;
use crate::architecture::los::walker::Owner;
use crate::spatial::bvh::surface::SurfaceKind;
use crate::spatial::world_los::descriptor::Bounds3;
use crate::spatial::world_los::descriptor::PrefabDescriptor;
use crate::spatial::world_los::placed::ChunkOccluder;
use crate::spatial::world_los::placed::WorldInstance;
use crate::spatial::world_los::state::WorldOccluder;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

/// Canonical default blas cap bytes value.
pub const DEFAULT_BLAS_CAP_BYTES: usize = 48 << 20;

/// Map frame `(x, y_north, elevation)` → engine frame `[x, y_up, z_north]`.
#[must_use]
pub fn map_to_engine(x: f64, y_north: f64, elev: f64) -> [f64; 3] {
    [x, elev, y_north]
}

/// How a crossing was decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fidelity {
    /// Real BLAS geometry.
    Exact,

    /// The catalogue proxy box (descriptor or BLAS not loaded yet).
    Proxy,
}

/// World verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldVerdict {
    /// Clear.
    Clear,

    /// Blocked.
    Blocked,

    /// Decided by a proxy box, or the segment crossed a chunk that is not resident.
    Provisional,
}

/// What counts as terminal for [`WorldOccluder::blocked`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockPolicy {
    /// Glass blocks.
    pub glass_blocks: bool,

    /// Foliage blocks.
    pub foliage_blocks: bool,

    /// Proxy blocks.
    pub proxy_blocks: bool,
}

impl BlockPolicy {
    /// The vision model: opaque only, a proxy box counts (nothing loaded is not nothing there).
    pub const VISION: Self = Self {
        glass_blocks: false,
        foliage_blocks: false,
        proxy_blocks: true,
    };
}

/// What the segment could and could not see.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Coverage {
    /// Chunks crossed.
    pub chunks_crossed: u32,

    /// Chunks on the segment that are not resident.
    pub chunks_missing: Vec<String>,

    /// Prefabs crossed as proxy boxes (distinct, first-crossed first).
    pub proxy_pids: Vec<u16>,

    /// BLAS paths those proxies are waiting for (known descriptors only).
    pub blas_pending: Vec<String>,
}

/// One crossing along `obs→tgt`.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldEvent {
    /// T.
    pub t: f64,

    /// Pos.
    pub pos: [f64; 3],

    /// Kind.
    pub kind: SurfaceKind,

    /// Chunk.
    pub chunk: String,

    /// Row.
    pub row: u32,

    /// Pid.
    pub pid: u16,

    /// The descriptor instance crossed (`Shell` for the root record or a proxy).
    pub inner: Owner,

    /// Tri.
    pub tri: u32,

    /// Fidelity.
    pub fidelity: Fidelity,
}

/// The verdict of [`WorldOccluder::evaluate_los`].
#[derive(Clone, Debug, PartialEq)]
pub struct WorldLos {
    /// Verdict.
    pub verdict: WorldVerdict,

    /// `1 − Π(1 − cᵢ)` over the pass-through events, `1` when blocked.
    pub concealment: f64,

    /// The opaque crossing that stopped the ray.
    pub blocker: Option<WorldEvent>,

    /// Hits.
    pub hits: Vec<LosHit>,

    /// Coverage.
    pub coverage: Coverage,
}

/// Prefab info.
#[derive(Clone, Debug)]
pub(crate) struct PrefabInfo {
    /// Kind.
    pub(crate) kind: String,

    /// Label.
    pub(crate) label: String,

    /// Proxy.
    pub(crate) proxy: Option<Bounds3>,
}

/// An expanded descriptor: its instance list ready to trace.
#[derive(Debug)]
pub struct PrefabOccluder {
    /// Pid.
    pub pid: u16,

    /// Instances.
    pub instances: Vec<Instance>,

    /// Local bounds.
    pub local_bounds: Bounds3,

    /// Has foliage.
    pub has_foliage: bool,

    /// Blas paths.
    pub blas_paths: Vec<String>,
}

/// What the host should fetch next for a set of chunks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wanted {
    /// Placed pids with no descriptor yet, most-placed first.
    pub descriptors: Vec<u16>,

    /// BLAS paths known descriptors still wait for, in descriptor order.
    pub blas: Vec<String>,
}

impl WorldOccluder {
    /// The catalogue kind of a pid.
    #[must_use]
    pub fn kind_of(&self, pid: u16) -> Option<&str> {
        self.info.get(&pid).map(|i| i.kind.as_str())
    }
}

impl WorldOccluder {
    /// The catalogue label of a pid.
    #[must_use]
    pub fn label_of(&self, pid: u16) -> Option<&str> {
        self.info.get(&pid).map(|i| i.label.as_str())
    }
}

impl WorldOccluder {
    /// Bounds of.
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

impl WorldOccluder {
    /// What to fetch next for `chunk_ids` (most-placed pids first), at most `limit` items each.
    #[must_use]
    pub fn wanted(&self, chunk_ids: &[String], limit: usize) -> Wanted {
        let mut count: HashMap<u16, u32> = HashMap::new();
        for id in chunk_ids {
            if let Some(c) = self.chunks.get(id) {
                for r in &c.rows {
                    *count.entry(r.pid).or_insert(0) += 1;
                }
            }
        }
        let mut pids: Vec<(u32, u16)> = count.into_iter().map(|(p, n)| (n, p)).collect();
        pids.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut out = Wanted::default();
        let mut seen_blas: HashSet<&str> = HashSet::new();
        for (_, pid) in pids {
            if self.no_block.contains(&pid) || self.expanded.contains_key(&pid) {
                continue;
            }
            match self.descriptors.get(&pid) {
                None => {
                    if out.descriptors.len() < limit {
                        out.descriptors.push(pid);
                    }
                }
                Some(d) => {
                    for i in &d.instances {
                        if !self.blas.contains_key(&i.blas)
                            && seen_blas.insert(i.blas.as_str())
                            && out.blas.len() < limit
                        {
                            out.blas.push(i.blas.clone());
                        }
                    }
                }
            }
        }
        out
    }
}

impl WorldOccluder {
    /// Heap bytes: rows + boxes + trees + sidecars + expanded instance lists.
    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.chunks
            .values()
            .map(ChunkOccluder::bytes)
            .sum::<usize>()
            + self.blas_bytes.values().sum::<usize>()
            + self
                .expanded
                .values()
                .map(|po| po.instances.len() * core::mem::size_of::<Instance>())
                .sum::<usize>()
    }
}

impl WorldOccluder {
    /// Chunk count.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}

impl WorldOccluder {
    /// Expanded count.
    #[must_use]
    pub fn expanded_count(&self) -> usize {
        self.expanded.len()
    }
}

impl WorldOccluder {
    /// Blas count.
    #[must_use]
    pub fn blas_count(&self) -> usize {
        self.blas.len()
    }
}

impl WorldOccluder {
    /// Resident chunk ids, sorted.
    #[must_use]
    pub fn resident_chunk_ids(&self) -> Vec<String> {
        let mut v: Vec<String> = self.chunks.keys().cloned().collect();
        v.sort();
        v
    }
}

impl WorldOccluder {
    /// A resident chunk's rows (engine frame).
    #[must_use]
    pub fn chunk_rows(&self, id: &str) -> Option<&[WorldInstance]> {
        self.chunks.get(id).map(|c| c.rows.as_slice())
    }
}

impl WorldOccluder {
    /// A resident chunk's current world boxes, parallel to its rows (`NO_BOX` = no geometry).
    #[must_use]
    pub fn chunk_boxes(&self, id: &str) -> Option<&[([f64; 3], [f64; 3])]> {
        self.chunks.get(id).map(|c| c.boxes.as_slice())
    }
}

impl WorldOccluder {
    /// The descriptor of a pid, once fetched.
    #[must_use]
    pub fn descriptor_of(&self, pid: u16) -> Option<&Arc<PrefabDescriptor>> {
        self.descriptors.get(&pid)
    }
}

impl WorldOccluder {
    /// Whether a pid's descriptor said it never blocks.
    #[must_use]
    pub fn is_no_block(&self, pid: u16) -> bool {
        self.no_block.contains(&pid)
    }
}

impl WorldOccluder {
    /// Rows of a resident chunk placed as proxies (their descriptor is not expanded).
    #[must_use]
    pub fn proxy_rows(&self, id: &str) -> Option<u32> {
        self.chunks.get(id).map(|c| c.proxy_rows)
    }
}

impl WorldOccluder {
    /// The expanded occluder of a pid (tests / the bench).
    #[must_use]
    pub fn expanded_of(&self, pid: u16) -> Option<&Arc<PrefabOccluder>> {
        self.expanded.get(&pid)
    }
}
