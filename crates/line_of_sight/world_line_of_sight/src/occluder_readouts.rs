//! What a host reads off the world occluder: the fetch plan and the read-outs.
//!
//! **Role:** answers what to fetch next for a set of chunks ([`WorldOccluder::wanted`],
//! [`Wanted`]), what the catalogue says of a pid, how many bytes the occluder holds, which chunks
//! are resident with their rows and boxes, and what a prefab's descriptor and expansion are.
//! **Position:** read by the map engine's occluder loader (the fetch plan), the debug world
//! line-of-sight bench and the developer tools' world check (the read-outs).
//! **Signals & state:** none; reads the occluder.
//! **Invariants:** the fetch plan lists the most-placed pids first and never a pid that is
//! expanded or never blocks; resident chunk ids come back sorted.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use building_interiors::compound::instances::{Instance, InstanceKind};
use world_chunks::chunk_id::ChunkId;

use crate::chunk_occluder::{ChunkOccluder, WorldInstance};
use crate::occluder_library::PrefabDescriptor;
use crate::world_occluder::{PrefabOccluder, WorldOccluder};

/// What the host should fetch next for a set of chunks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wanted {
    /// Placed pids with no descriptor yet, most-placed first.
    pub descriptors: Vec<u16>,

    /// BLAS paths known descriptors still wait for, in descriptor order.
    pub blas: Vec<String>,
}

impl WorldOccluder {
    /// What to fetch next for `chunk_ids` (most-placed pids first), at most `limit` items each.
    #[must_use]
    pub fn wanted(&self, chunk_ids: &[ChunkId], limit: usize) -> Wanted {
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

    /// The catalogue kind of a pid.
    #[must_use]
    pub fn kind_of(&self, pid: u16) -> Option<&str> {
        self.info.get(&pid).map(|i| i.kind.as_str())
    }

    /// The catalogue label of a pid.
    #[must_use]
    pub fn label_of(&self, pid: u16) -> Option<&str> {
        self.info.get(&pid).map(|i| i.label.as_str())
    }

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

    /// The number of resident chunks.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// The number of expanded prefabs.
    #[must_use]
    pub fn expanded_count(&self) -> usize {
        self.expanded.len()
    }

    /// The number of BLAS sidecars held.
    #[must_use]
    pub fn blas_count(&self) -> usize {
        self.blas.len()
    }

    /// Resident chunk ids, sorted.
    #[must_use]
    pub fn resident_chunk_ids(&self) -> Vec<ChunkId> {
        let mut v: Vec<ChunkId> = self.chunks.keys().cloned().collect();
        v.sort();
        v
    }

    /// A resident chunk's rows (engine frame).
    #[must_use]
    pub fn chunk_rows(&self, chunk: &ChunkId) -> Option<&[WorldInstance]> {
        self.chunks.get(chunk).map(|c| c.rows.as_slice())
    }

    /// A resident chunk's current world boxes, parallel to its rows (`NO_BOX` = no geometry).
    #[must_use]
    pub fn chunk_boxes(&self, chunk: &ChunkId) -> Option<&[([f64; 3], [f64; 3])]> {
        self.chunks.get(chunk).map(|c| c.boxes.as_slice())
    }

    /// The descriptor of a pid, once fetched.
    #[must_use]
    pub fn descriptor_of(&self, pid: u16) -> Option<&Arc<PrefabDescriptor>> {
        self.descriptors.get(&pid)
    }

    /// Whether a pid's descriptor said it never blocks.
    #[must_use]
    pub fn is_no_block(&self, pid: u16) -> bool {
        self.no_block.contains(&pid)
    }

    /// Rows of a resident chunk placed as proxies (their descriptor is not expanded).
    #[must_use]
    pub fn proxy_rows(&self, chunk: &ChunkId) -> Option<u32> {
        self.chunks.get(chunk).map(|c| c.proxy_rows)
    }

    /// The expanded occluder of a pid.
    #[must_use]
    pub fn expanded_of(&self, pid: u16) -> Option<&Arc<PrefabOccluder>> {
        self.expanded.get(&pid)
    }

    /// The instance kind a pid's root record carries, once expanded.
    #[must_use]
    pub fn root_kind_of(&self, pid: u16) -> Option<InstanceKind> {
        self.expanded
            .get(&pid)
            .and_then(|po| po.instances.first())
            .map(|i| i.record.kind)
    }
}
