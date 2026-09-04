//! T-090.12.3 — [`WorldOccluder`]: every placed object on the terrain as a BLAS instance under
//! its chunk row's transform. A segment walks the chunks it crosses (the DDA), asks each chunk's
//! TLAS for the rows it crosses, and traces those rows' geometry — the descriptor's instance
//! list once it is expanded (BLAS fetched), the catalogue proxy box until then — through the
//! same material walk as [`CompoundBuilding`](crate::building_compound::CompoundBuilding):
//! opaque stops, glass conceals 5 %, foliage conceals by depth, doors are closed.
//!
//! Frames: the Enfusion world frame `[x, y_up, z_north]`, like every BLAS. The editor's map
//! frame converts through [`map_to_engine`].
//!
//! Honesty: a verdict decided by a proxy box, or a segment that crosses a chunk that is not
//! resident, is [`WorldVerdict::Provisional`] and its [`Coverage`] says why. Nothing is ever
//! reported clear because geometry was not loaded.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::building_blueprint::LosHit;
use crate::building_compound::{Instance, InstanceKind, instances_from_records};
use crate::building_compound_los::{
    Owner, TraceEvent, blocked_instances_where, point_at, trace_instances,
};
use crate::bvh::{BvhSidecar, SurfaceKind};
use crate::world::chunk::WorldChunk;
use crate::world::chunk_math::{TerrainSizeM, chunk_id};
use crate::world::prefab::PrefabRow;

use super::dda::cells_on_segment;
use super::descriptor::{Bounds3, PrefabDescriptor};
use super::placed::{ChunkOccluder, WorldInstance};
use super::tlas::Candidate;

/// Default BLAS byte budget before least-recently-traced sidecars are dropped.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldVerdict {
    Clear,
    Blocked,
    /// Decided by a proxy box, or the segment crossed a chunk that is not resident.
    Provisional,
}

/// What counts as terminal for [`WorldOccluder::blocked`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockPolicy {
    pub glass_blocks: bool,
    pub foliage_blocks: bool,
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
    pub t: f64,
    pub pos: [f64; 3],
    pub kind: SurfaceKind,
    pub chunk: String,
    pub row: u32,
    pub pid: u16,
    /// The descriptor instance crossed (`Shell` for the root record or a proxy).
    pub inner: Owner,
    pub tri: u32,
    pub fidelity: Fidelity,
}

/// The verdict of [`WorldOccluder::evaluate_los`].
#[derive(Clone, Debug, PartialEq)]
pub struct WorldLos {
    pub verdict: WorldVerdict,
    /// `1 − Π(1 − cᵢ)` over the pass-through events, `1` when blocked.
    pub concealment: f64,
    /// The opaque crossing that stopped the ray.
    pub blocker: Option<WorldEvent>,
    pub hits: Vec<LosHit>,
    pub coverage: Coverage,
}

/// Catalogue facts per prefab id.
#[derive(Clone, Debug)]
struct PrefabInfo {
    kind: String,
    label: String,
    /// The proxy box (object frame) until the descriptor expands.
    proxy: Option<Bounds3>,
}

/// An expanded descriptor: its instance list ready to trace.
#[derive(Debug)]
pub struct PrefabOccluder {
    pub pid: u16,
    pub instances: Vec<Instance>,
    pub local_bounds: Bounds3,
    pub has_foliage: bool,
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

pub struct WorldOccluder {
    chunk_m: f64,
    terrain: TerrainSizeM,
    pub(super) chunks: HashMap<String, ChunkOccluder>,
    info: HashMap<u16, PrefabInfo>,
    descriptors: HashMap<u16, Arc<PrefabDescriptor>>,
    blas: HashMap<String, Arc<BvhSidecar>>,
    blas_bytes: HashMap<String, usize>,
    blas_last_use: HashMap<String, u64>,
    tick: u64,
    pub(super) expanded: HashMap<u16, Arc<PrefabOccluder>>,
    no_block: HashSet<u16>,
    /// Rows per pid across resident chunks.
    placed: HashMap<u16, u32>,
    dirty: HashSet<String>,
    cap_bytes: usize,
}

impl WorldOccluder {
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

    /// Change the BLAS byte budget (applied at the next `insert_blas`).
    pub fn set_blas_cap_bytes(&mut self, bytes: usize) {
        self.cap_bytes = bytes;
    }

    /// The catalogue: kinds, labels and the proxy boxes. The proxy is the catalogue's median
    /// world-AABB half extents laid out base-anchored around the origin (`x ±hx`, `y 0..2·hz`,
    /// `z ±hy`) — right for the ground-pivoted majority, replaced by the descriptor's exact
    /// bounds the moment it expands.
    pub fn set_prefabs<'a>(&mut self, rows: impl Iterator<Item = &'a PrefabRow>) {
        for r in rows {
            let Ok(pid) = u16::try_from(r.prefab_id as u64) else {
                continue;
            };
            if r.prefab_id.fract() != 0.0 {
                continue;
            }
            let proxy = match (r.half_x, r.half_y, r.half_z) {
                (Some(hx), Some(hy), Some(hz)) if hx > 0.0 && hy > 0.0 && hz > 0.0 => {
                    Some(Bounds3 {
                        min: [-hx, 0.0, -hy],
                        max: [hx, 2.0 * hz, hy],
                    })
                }
                _ => None,
            };
            let label = r.label.clone().unwrap_or_else(|| {
                r.resource_name
                    .as_deref()
                    .and_then(|s| s.rsplit('/').next())
                    .map(|s| s.trim_end_matches(".et").to_string())
                    .unwrap_or_else(|| format!("pid {pid}"))
            });
            self.info.insert(
                pid,
                PrefabInfo {
                    kind: r.kind.clone(),
                    label,
                    proxy,
                },
            );
        }
        self.dirty.extend(self.chunks.keys().cloned());
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

    /// Object-frame bounds a row of `pid` currently has, and whether they are a proxy.
    fn bounds_of(&self, pid: u16) -> Option<(Bounds3, bool)> {
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

    /// A chunk became resident.
    pub fn insert_chunk(&mut self, id: &str, chunk: &WorldChunk) {
        self.remove_chunk(id);
        let bounds = |pid: u16| self.bounds_of(pid);
        let c = ChunkOccluder::build(id, chunk, &bounds);
        for r in &c.rows {
            *self.placed.entry(r.pid).or_insert(0) += 1;
        }
        self.chunks.insert(id.to_string(), c);
        self.dirty.remove(id);
    }

    /// A chunk was evicted.
    pub fn remove_chunk(&mut self, id: &str) {
        if let Some(c) = self.chunks.remove(id) {
            for r in &c.rows {
                if let Some(n) = self.placed.get_mut(&r.pid) {
                    *n = n.saturating_sub(1);
                }
            }
        }
        self.dirty.remove(id);
    }

    fn mark_dirty_for(&mut self, pid: u16) {
        let ids: Vec<String> = self
            .chunks
            .values()
            .filter(|c| c.rows.iter().any(|r| r.pid == pid))
            .map(|c| c.id.clone())
            .collect();
        self.dirty.extend(ids);
    }

    /// A descriptor arrived. Expands at once when every BLAS it names is already loaded.
    pub fn insert_descriptor(&mut self, d: PrefabDescriptor) {
        let Ok(pid) = u16::try_from(d.prefab_id) else {
            return;
        };
        if !d.blocks {
            self.no_block.insert(pid);
            self.descriptors.remove(&pid);
            self.expanded.remove(&pid);
            self.mark_dirty_for(pid);
            return;
        }
        self.no_block.remove(&pid);
        self.descriptors.insert(pid, Arc::new(d));
        self.mark_dirty_for(pid);
        self.try_expand(pid);
    }

    /// A BLAS arrived. Every descriptor waiting on it expands; the byte budget is enforced.
    pub fn insert_blas(&mut self, path: &str, sc: Arc<BvhSidecar>) {
        let bytes = sc.verts.len() * 24 + sc.tris.len() * 13 + sc.bvh.node_count() * 32;
        self.blas_bytes.insert(path.to_string(), bytes);
        self.blas.insert(path.to_string(), sc);
        self.blas_last_use.insert(path.to_string(), self.tick);
        let waiting: Vec<u16> = self
            .descriptors
            .iter()
            .filter(|(pid, d)| {
                !self.expanded.contains_key(pid) && d.instances.iter().any(|i| i.blas == path)
            })
            .map(|(pid, _)| *pid)
            .collect();
        for pid in waiting {
            self.try_expand(pid);
        }
        self.enforce_cap();
    }

    fn try_expand(&mut self, pid: u16) -> bool {
        let Some(d) = self.descriptors.get(&pid).cloned() else {
            return false;
        };
        if d.instances.iter().any(|i| !self.blas.contains_key(&i.blas)) {
            return false;
        }
        let Ok(instances) = instances_from_records(&d.instances, &self.blas) else {
            return false;
        };
        if instances.is_empty() {
            return false;
        }
        let mut bounds: Option<Bounds3> = None;
        let mut has_foliage = false;
        for inst in &instances {
            let (min, max) = inst.world_aabb();
            let b = Bounds3 { min, max };
            bounds = Some(bounds.map_or(b, |u| u.union(b)));
            has_foliage |= inst.blas.kinds.contains(&SurfaceKind::Foliage);
        }
        let blas_paths: Vec<String> = d.blas_paths().iter().map(ToString::to_string).collect();
        self.expanded.insert(
            pid,
            Arc::new(PrefabOccluder {
                pid,
                instances,
                local_bounds: bounds.unwrap_or(Bounds3 {
                    min: [0.0; 3],
                    max: [0.0; 3],
                }),
                has_foliage,
                blas_paths,
            }),
        );
        self.mark_dirty_for(pid);
        true
    }

    /// Drop least-recently-traced sidecars no placed, expanded prefab needs until the budget
    /// holds. An evicted sidecar un-expands the prefabs that reference it (their rows fall back
    /// to the proxy box until it is fetched again).
    fn enforce_cap(&mut self) {
        let total = |s: &Self| s.blas_bytes.values().sum::<usize>();
        while total(self) > self.cap_bytes {
            let needed: HashSet<&str> = self
                .expanded
                .values()
                .filter(|po| self.placed.get(&po.pid).copied().unwrap_or(0) > 0)
                .flat_map(|po| po.blas_paths.iter().map(String::as_str))
                .collect();
            let victim = self
                .blas
                .keys()
                .filter(|p| !needed.contains(p.as_str()))
                .min_by_key(|p| {
                    (
                        self.blas_last_use.get(*p).copied().unwrap_or(0),
                        (*p).clone(),
                    )
                })
                .cloned();
            let Some(victim) = victim else {
                return;
            };
            self.blas.remove(&victim);
            self.blas_bytes.remove(&victim);
            self.blas_last_use.remove(&victim);
            let unexpand: Vec<u16> = self
                .expanded
                .values()
                .filter(|po| po.blas_paths.contains(&victim))
                .map(|po| po.pid)
                .collect();
            for pid in unexpand {
                self.expanded.remove(&pid);
                self.mark_dirty_for(pid);
            }
        }
    }

    /// Rebuild the boxes + TLAS of every chunk whose geometry changed since the last refresh.
    pub fn refresh(&mut self) {
        let ids: Vec<String> = self.dirty.drain().collect();
        for id in ids {
            let Some(mut c) = self.chunks.remove(&id) else {
                continue;
            };
            let bounds = |pid: u16| self.bounds_of(pid);
            c.rebuild(&bounds);
            self.chunks.insert(id, c);
        }
    }

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

    /// Chunk cells the segment crosses, in order, with the ids of the ones that are not resident.
    pub(super) fn chunks_along(&self, obs: [f64; 3], tgt: [f64; 3]) -> (Vec<String>, Vec<String>) {
        let cols = (self.terrain.width / self.chunk_m).ceil().max(1.0) as i64;
        let rows = (self.terrain.height / self.chunk_m).ceil().max(1.0) as i64;
        let cells = cells_on_segment([obs[0], obs[2]], [tgt[0], tgt[2]], self.chunk_m, cols, rows);
        let mut present = Vec::with_capacity(cells.len());
        let mut missing = Vec::new();
        for (cx, cy) in cells {
            let id = chunk_id(cx, cy);
            if self.chunks.contains_key(&id) {
                present.push(id);
            } else {
                missing.push(id);
            }
        }
        (present, missing)
    }

    pub(super) fn candidates_of(
        &self,
        c: &ChunkOccluder,
        obs: [f64; 3],
        tgt: [f64; 3],
    ) -> Vec<Candidate> {
        let mut out = Vec::new();
        if self.dirty.contains(&c.id) {
            c.tlas.candidates_linear(obs, tgt, &mut out);
        } else {
            c.tlas.candidates(obs, tgt, &mut out);
        }
        out
    }

    /// Every crossing along `obs→tgt`, sorted by `t`, with what the segment could not see.
    #[must_use]
    pub fn trace(&self, obs: [f64; 3], tgt: [f64; 3]) -> (Vec<WorldEvent>, Coverage) {
        let (present, missing) = self.chunks_along(obs, tgt);
        let mut cov = Coverage {
            chunks_crossed: (present.len() + missing.len()) as u32,
            chunks_missing: missing,
            ..Coverage::default()
        };
        let mut out: Vec<WorldEvent> = Vec::new();
        if obs == tgt {
            return (out, cov);
        }
        let mut local: Vec<TraceEvent> = Vec::new();
        for id in &present {
            let c = &self.chunks[id];
            for cand in self.candidates_of(c, obs, tgt) {
                let row = &c.rows[cand.index as usize];
                let pid = row.pid;
                if self.no_block.contains(&pid) {
                    continue;
                }
                match self.expanded.get(&pid) {
                    Some(po) => {
                        let world = row.rigid();
                        let inv = world.inverse();
                        let (p, q) = (inv.point(obs), inv.point(tgt));
                        local.clear();
                        trace_instances(&po.instances, p, q, 0.0, 1.0, &mut local);
                        out.extend(local.drain(..).map(|ev| WorldEvent {
                            t: ev.t,
                            pos: point_at(obs, tgt, ev.t),
                            kind: ev.kind,
                            chunk: id.clone(),
                            row: cand.index,
                            pid,
                            inner: ev.owner,
                            tri: ev.tri,
                            fidelity: Fidelity::Exact,
                        }));
                    }
                    None => {
                        if !cov.proxy_pids.contains(&pid) {
                            cov.proxy_pids.push(pid);
                            if let Some(d) = self.descriptors.get(&pid) {
                                for i in &d.instances {
                                    if !self.blas.contains_key(&i.blas)
                                        && !cov.blas_pending.contains(&i.blas)
                                    {
                                        cov.blas_pending.push(i.blas.clone());
                                    }
                                }
                            }
                        }
                        out.push(WorldEvent {
                            t: cand.t_entry,
                            pos: point_at(obs, tgt, cand.t_entry),
                            kind: SurfaceKind::Opaque,
                            chunk: id.clone(),
                            row: cand.index,
                            pid,
                            inner: Owner::Shell,
                            tri: 0,
                            fidelity: Fidelity::Proxy,
                        });
                    }
                }
            }
        }
        out.sort_by(|a, b| {
            a.t.total_cmp(&b.t)
                .then(a.chunk.cmp(&b.chunk))
                .then(a.row.cmp(&b.row))
                .then(a.inner.cmp(&b.inner))
                .then(a.tri.cmp(&b.tri))
        });
        out.dedup_by(|b, a| {
            a.chunk == b.chunk
                && a.row == b.row
                && a.inner == b.inner
                && a.kind == b.kind
                && (a.t - b.t).abs() < 1e-9
        });
        (out, cov)
    }

    /// Does anything terminal under `policy` stand on `obs→tgt`? A segment through a missing
    /// chunk is judged on what is loaded (see [`Self::evaluate_los`] for the honest form).
    #[must_use]
    pub fn blocked(&self, obs: [f64; 3], tgt: [f64; 3], policy: BlockPolicy) -> bool {
        if obs == tgt {
            return false;
        }
        let terminal = move |k: SurfaceKind| match k {
            SurfaceKind::Opaque => true,
            SurfaceKind::Glass => policy.glass_blocks,
            SurfaceKind::Foliage => policy.foliage_blocks,
        };
        let (present, _) = self.chunks_along(obs, tgt);
        for id in &present {
            let c = &self.chunks[id];
            for cand in self.candidates_of(c, obs, tgt) {
                let row = &c.rows[cand.index as usize];
                if self.no_block.contains(&row.pid) {
                    continue;
                }
                match self.expanded.get(&row.pid) {
                    Some(po) => {
                        let inv = row.rigid().inverse();
                        if blocked_instances_where(
                            &po.instances,
                            inv.point(obs),
                            inv.point(tgt),
                            0.0,
                            1.0,
                            terminal,
                        ) {
                            return true;
                        }
                    }
                    None => {
                        if policy.proxy_blocks {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// [`Self::blocked`] as a closure — the shape `building_viewshed::wash_band` takes.
    pub fn blocked_fn(&self, policy: BlockPolicy) -> impl Fn([f64; 3], [f64; 3]) -> bool + '_ {
        move |a, b| self.blocked(a, b, policy)
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

    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    #[must_use]
    pub fn expanded_count(&self) -> usize {
        self.expanded.len()
    }

    #[must_use]
    pub fn blas_count(&self) -> usize {
        self.blas.len()
    }

    /// Resident chunk ids, sorted.
    #[must_use]
    pub fn resident_chunk_ids(&self) -> Vec<String> {
        let mut v: Vec<String> = self.chunks.keys().cloned().collect();
        v.sort();
        v
    }

    /// A resident chunk's rows (engine frame).
    #[must_use]
    pub fn chunk_rows(&self, id: &str) -> Option<&[WorldInstance]> {
        self.chunks.get(id).map(|c| c.rows.as_slice())
    }

    /// A resident chunk's current world boxes, parallel to its rows (`NO_BOX` = no geometry).
    #[must_use]
    pub fn chunk_boxes(&self, id: &str) -> Option<&[([f64; 3], [f64; 3])]> {
        self.chunks.get(id).map(|c| c.boxes.as_slice())
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
    pub fn proxy_rows(&self, id: &str) -> Option<u32> {
        self.chunks.get(id).map(|c| c.proxy_rows)
    }

    /// The expanded occluder of a pid (tests / the bench).
    #[must_use]
    pub fn expanded_of(&self, pid: u16) -> Option<&Arc<PrefabOccluder>> {
        self.expanded.get(&pid)
    }

    /// The instance kinds a pid's root record carries, once expanded.
    #[must_use]
    pub fn root_kind_of(&self, pid: u16) -> Option<InstanceKind> {
        self.expanded
            .get(&pid)
            .and_then(|po| po.instances.first())
            .map(|i| i.record.kind)
    }
}
