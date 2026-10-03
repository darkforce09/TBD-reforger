//! The chunk walk every world query makes: the chunks a segment crosses, the rows their box trees
//! yield, and each row's exact or proxy crossings.
//!
//! **Role:** walks the chunk cells under a segment, records the ones not resident, asks each
//! resident chunk's box tree for the rows the segment crosses, traces an expanded prefab exactly
//! in its row's frame through the interior line of sight's instance trace, and stands a proxy
//! crossing in for any other ([`WorldOccluder::trace`]); answers the yes-or-no form under a
//! [`BlockPolicy`] ([`WorldOccluder::blocked`], [`WorldOccluder::blocked_fn`]).
//! **Position:** over [`crate::chunk_cells`], [`crate::chunk_occluder`] and
//! [`crate::instance_box_tree`]; [`crate::sight_line`] evaluates its crossings; the map engine's
//! object wash and the developer tools' world check call `blocked`.
//! **Signals & state:** none; reads the occluder.
//! **Invariants:** crossings come back sorted by `(t, chunk, row, inner, tri)` with a shared-edge
//! duplicate counted once; a chunk marked dirty is scanned linearly, never through a stale tree;
//! `blocked` judges on what is loaded, while the verdict says what was not.

use crate::chunk_cells::cells_on_segment;
use crate::chunk_occluder::ChunkOccluder;
use crate::instance_box_tree::Candidate;
use crate::query_types::BlockPolicy;
use crate::query_types::Coverage;
use crate::query_types::Fidelity;
use crate::query_types::WorldEvent;
use crate::world_occluder::WorldOccluder;
use geometry_primitives::segment_geometry::point_at;
use interior_line_of_sight::compound_walk::Owner;
use interior_line_of_sight::compound_walk::TraceEvent;
use interior_line_of_sight::compound_walk::blocked_instances_where;
use interior_line_of_sight::compound_walk::trace_instances;
use spatial_indexes::bounding_volume_hierarchy::surface_kind::SurfaceKind;
use world_chunks::chunk_id::ChunkId;

impl WorldOccluder {
    /// Chunk cells the segment crosses, in order, with the ids of the ones that are not resident.
    pub(crate) fn chunks_along(
        &self,
        obs: [f64; 3],
        tgt: [f64; 3],
    ) -> (Vec<ChunkId>, Vec<ChunkId>) {
        let cols = (self.terrain.width / self.chunk_m).ceil().max(1.0) as i64;
        let rows = (self.terrain.height / self.chunk_m).ceil().max(1.0) as i64;
        let cells = cells_on_segment([obs[0], obs[2]], [tgt[0], tgt[2]], self.chunk_m, cols, rows);
        let mut present = Vec::with_capacity(cells.len());
        let mut missing = Vec::new();
        for (cx, cy) in cells {
            let id = ChunkId::of_cell(cx, cy);
            if self.chunks.contains_key(&id) {
                present.push(id);
            } else {
                missing.push(id);
            }
        }
        (present, missing)
    }

    /// The rows of chunk `c` whose boxes `obs→tgt` crosses, by entry `t`: the linear scan while the
    /// chunk is dirty, the box tree otherwise.
    pub(crate) fn candidates_of(
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

    /// Does anything terminal under `policy` stand on `obs→tgt`? A segment through a missing chunk is judged on what is loaded (see [`Self::evaluate_los`] for the honest form).
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

    /// [`Self::blocked`] as a closure, the blocking test the interior line of sight's
    /// `floor_wash::wash_band` takes.
    pub fn blocked_fn(&self, policy: BlockPolicy) -> impl Fn([f64; 3], [f64; 3]) -> bool + '_ {
        move |a, b| self.blocked(a, b, policy)
    }
}
