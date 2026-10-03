//! The verdict of one sight line through the streamed world.
//!
//! **Role:** [`WorldOccluder::evaluate_los`] traces a segment through every resident chunk, names
//! each crossed object `pid:chunk:row[/inner id]`, reduces the crossings through the interior line
//! of sight's shared evaluation (opaque stops, glass and foliage conceal), and decides the
//! verdict: `Blocked` for an exact opaque blocker, `Provisional` for a proxy blocker or, with no
//! blocker, for a chunk on the segment that is not resident, `Clear` otherwise.
//! **Position:** over the chunk walk of [`crate::raycast`]; the Mission Creator's line-of-sight
//! tool, the debug world line-of-sight bench and the developer tools' world check read the verdict.
//! **Signals & state:** none; reads the occluder.
//! **Invariants:** every crossed instance is named by its prefab, chunk, row and inner instance;
//! a verdict never claims `Clear` over a chunk it could not see.

use std::collections::HashMap;

use building_interiors::blueprint::sight_line::LosHitKind;
use building_interiors::building_ids::BuildingFeatureId;
use interior_line_of_sight::compound_walk::{Owner, TraceEvent, hit_kind_of};
use interior_line_of_sight::sight_line_evaluation::{SightLineScene, evaluate_los};
use spatial_indexes::bounding_volume_hierarchy::surface_kind::SurfaceKind;
use world_chunks::chunk_id::ChunkId;

use crate::query_types::{Fidelity, WorldEvent, WorldLos, WorldVerdict};
use crate::world_occluder::WorldOccluder;

impl WorldOccluder {
    /// Line of sight through the world: the multi-hit walk with the compound's material
    /// semantics, every instance named `pid:chunk:row[/inner id]`.
    #[must_use]
    pub fn evaluate_los(&self, obs: [f64; 3], tgt: [f64; 3]) -> WorldLos {
        let (events, coverage) = self.trace(obs, tgt);
        let evaluation = evaluate_los(&WorldSightLine::index(self, obs, tgt, &events));
        let blocker = if evaluation.is_clear {
            None
        } else {
            events
                .iter()
                .find(|e| e.kind == SurfaceKind::Opaque && (e.t - evaluation.t_end).abs() < 1e-12)
                .or_else(|| events.iter().find(|e| e.kind == SurfaceKind::Opaque))
                .cloned()
        };
        let verdict = if !evaluation.is_clear {
            if blocker
                .as_ref()
                .is_some_and(|b| b.fidelity == Fidelity::Proxy)
            {
                WorldVerdict::Provisional
            } else {
                WorldVerdict::Blocked
            }
        } else if !coverage.chunks_missing.is_empty() {
            WorldVerdict::Provisional
        } else {
            WorldVerdict::Clear
        };
        WorldLos {
            verdict,
            concealment: evaluation.concealment,
            blocker,
            hits: evaluation.hits,
            coverage,
        }
    }
}

/// One crossed object: the chunk, the row in it, the prefab placed there and the descriptor
/// instance crossed (`Shell` for the root record or a proxy).
type CrossedObject = (ChunkId, u32, u16, Owner);

/// One sight line through the world, indexed: every crossed object numbered in first-crossed
/// order (then the foliage around the observer), so the shared evaluation can key instances by
/// index.
struct WorldSightLine<'a> {
    /// The occluder the sight line was traced through.
    occluder: &'a WorldOccluder,

    /// The observer, engine frame.
    obs: [f64; 3],

    /// The target, engine frame.
    tgt: [f64; 3],

    /// Every crossed object, at its owner index.
    objects: Vec<CrossedObject>,

    /// The world crossings with each owner replaced by its object's index.
    crossings: Vec<TraceEvent>,

    /// The foliage volumes the observer stands inside, by object index.
    inside: HashMap<usize, f64>,
}

impl<'a> WorldSightLine<'a> {
    /// Numbers the objects of `events` and the foliage volumes around `obs`.
    fn index(
        occluder: &'a WorldOccluder,
        obs: [f64; 3],
        tgt: [f64; 3],
        events: &[WorldEvent],
    ) -> Self {
        let mut objects: Vec<CrossedObject> = Vec::new();
        let mut index: HashMap<(ChunkId, u32, Owner), usize> = HashMap::new();
        let mut key_of = |chunk: &ChunkId, row: u32, pid: u16, inner: Owner| -> usize {
            let k = (chunk.clone(), row, inner);
            *index.entry(k).or_insert_with(|| {
                objects.push((chunk.clone(), row, pid, inner));
                objects.len() - 1
            })
        };
        let crossings: Vec<TraceEvent> = events
            .iter()
            .map(|e| TraceEvent {
                t: e.t,
                pos: e.pos,
                kind: e.kind,
                owner: Owner::Instance(key_of(&e.chunk, e.row, e.pid, e.inner)),
                tri: e.tri,
            })
            .collect();

        let mut inside: HashMap<usize, f64> = HashMap::new();
        let (present, _) = occluder.chunks_along(obs, obs);
        for id in &present {
            let c = &occluder.chunks[id];
            for cand in occluder.candidates_of(c, obs, obs) {
                let row = &c.rows[cand.index as usize];
                let Some(po) = occluder.expanded.get(&row.pid) else {
                    continue;
                };
                if !po.has_foliage {
                    continue;
                }
                let world = row.rigid();
                for (i, inst) in po.instances.iter().enumerate() {
                    if !inst.blas.kinds.contains(&SurfaceKind::Foliage) {
                        continue;
                    }
                    let (lo, hi) = inst.world_aabb();
                    let (lo, hi) = world.aabb_of(lo, hi);
                    if (0..3).all(|k| obs[k] >= lo[k] && obs[k] <= hi[k]) {
                        let k = key_of(id, cand.index, row.pid, Owner::Instance(i));
                        inside.insert(k, 0.0);
                    }
                }
            }
        }
        Self {
            occluder,
            obs,
            tgt,
            objects,
            crossings,
            inside,
        }
    }
}

impl SightLineScene for WorldSightLine<'_> {
    fn endpoints(&self) -> ([f64; 3], [f64; 3]) {
        (self.obs, self.tgt)
    }

    fn crossings(&self) -> &[TraceEvent] {
        &self.crossings
    }

    fn foliage_around_observer(&self) -> HashMap<usize, f64> {
        self.inside.clone()
    }

    fn name_crossing(&self, crossing: &TraceEvent) -> (LosHitKind, BuildingFeatureId) {
        let Owner::Instance(k) = crossing.owner else {
            return (LosHitKind::Solid, BuildingFeatureId::new(""));
        };
        let (_, _, pid, inner) = &self.objects[k];
        let kind = match inner {
            Owner::Shell => LosHitKind::Solid,
            Owner::Instance(i) => self
                .occluder
                .expanded
                .get(pid)
                .and_then(|po| po.instances.get(*i))
                .map_or(LosHitKind::Solid, hit_kind_of),
        };
        (kind, self.name_instance(k))
    }

    fn name_instance(&self, owner: usize) -> BuildingFeatureId {
        let (chunk, row, pid, inner) = &self.objects[owner];
        match inner {
            Owner::Shell => BuildingFeatureId::new(format!("{pid}:{chunk}:{row}")),
            Owner::Instance(i) => {
                let inner_id = self
                    .occluder
                    .expanded
                    .get(pid)
                    .and_then(|po| po.instances.get(*i))
                    .map(|inst| inst.record.id.to_string())
                    .unwrap_or_default();
                BuildingFeatureId::new(format!("{pid}:{chunk}:{row}/{inner_id}"))
            }
        }
    }
}
