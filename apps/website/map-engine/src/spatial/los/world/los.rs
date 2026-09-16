//! Role: los.
//! Position: `spatial/los/world` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::collections::HashMap;

use crate::spatial::bvh::surface::SurfaceKind;
use crate::spatial::los::interior::walker::Owner;
use crate::spatial::los::interior::walker::TraceEvent;
use crate::spatial::los::interior::walker::finish_concealment;
use crate::spatial::los::interior::walker::hit_kind_of;
use crate::spatial::los::interior::walker::reduce_events;
use crate::world::architecture::blueprint::attribution_1::LosHitKind;

use crate::spatial::los::world::coverage_1::Fidelity;
use crate::spatial::los::world::coverage_1::WorldLos;
use crate::spatial::los::world::coverage_1::WorldVerdict;
use crate::spatial::los::world::state::WorldOccluder;

impl WorldOccluder {
    /// Line of sight through the world: the multi-hit walk with the compound's material semantics, every instance named `pid:chunk:row[/inner id]`.
    #[must_use]
    pub fn evaluate_los(&self, obs: [f64; 3], tgt: [f64; 3]) -> WorldLos {
        let (events, coverage) = self.trace(obs, tgt);

        let mut table: Vec<(String, u32, u16, Owner)> = Vec::new();
        let mut index: HashMap<(String, u32, Owner), usize> = HashMap::new();
        let mut key_of = |chunk: &str, row: u32, pid: u16, inner: Owner| -> usize {
            let k = (chunk.to_string(), row, inner);
            *index.entry(k).or_insert_with(|| {
                table.push((chunk.to_string(), row, pid, inner));
                table.len() - 1
            })
        };
        let flat: Vec<TraceEvent> = events
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
        let (present, _) = self.chunks_along(obs, obs);
        for id in &present {
            let c = &self.chunks[id];
            for cand in self.candidates_of(c, obs, obs) {
                let row = &c.rows[cand.index as usize];
                let Some(po) = self.expanded.get(&row.pid) else {
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
        let name_of = |k: usize| -> String {
            let (chunk, row, pid, inner) = &table[k];
            match inner {
                Owner::Shell => format!("{pid}:{chunk}:{row}"),
                Owner::Instance(i) => {
                    let inner_id = self
                        .expanded
                        .get(pid)
                        .and_then(|po| po.instances.get(*i))
                        .map(|inst| inst.record.id.clone())
                        .unwrap_or_default();
                    format!("{pid}:{chunk}:{row}/{inner_id}")
                }
            }
        };
        let name = |ev: &TraceEvent| -> (LosHitKind, String) {
            let Owner::Instance(k) = ev.owner else {
                return (LosHitKind::Solid, String::new());
            };
            let (_, _, pid, inner) = &table[k];
            let kind = match inner {
                Owner::Shell => LosHitKind::Solid,
                Owner::Instance(i) => self
                    .expanded
                    .get(pid)
                    .and_then(|po| po.instances.get(*i))
                    .map_or(LosHitKind::Solid, hit_kind_of),
            };
            (kind, name_of(k))
        };
        let reduced = reduce_events(&flat, obs, tgt, inside, &name, &name_of);
        let mut hits = reduced.hits;
        let concealment = finish_concealment(&mut hits, reduced.is_clear);
        let blocker = if reduced.is_clear {
            None
        } else {
            events
                .iter()
                .find(|e| e.kind == SurfaceKind::Opaque && (e.t - reduced.t_end).abs() < 1e-12)
                .or_else(|| events.iter().find(|e| e.kind == SurfaceKind::Opaque))
                .cloned()
        };
        let verdict = if !reduced.is_clear {
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
            concealment,
            blocker,
            hits,
            coverage,
        }
    }
}
