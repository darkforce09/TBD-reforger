//! T-090.11.4 — the ray walk over a [`CompoundBuilding`]: every crossing of the shell and of
//! every placed instance in `t` order, then the material-aware verdict — opaque stops the ray,
//! glass conceals 5 % and continues, foliage conceals by the depth crossed
//! (`1 − e^(−k·d)`), a closed leaf blocks, an open door annotates the aperture the ray passes.
//! `blocked` is the yes/no form the wash and the parity replay use.
//!
//! Frames: everything is in the building's local space (y up); an instance's BLAS is hit by
//! mapping both segment endpoints through the inverse placement — the parametric `t` of a hit
//! is invariant under that affine map, so hits from every BLAS merge directly.

use std::collections::HashMap;

use crate::building_blueprint::{BuildingBlueprint, LosHit, LosHitKind, LosResult};
use crate::building_compound::{CompoundBuilding, Instance, InstanceKind};
use crate::bvh::{Hit, SurfaceKind};
use crate::geometry::rigid::Rigid;

/// Concealment one glass pane adds (non-terminal).
pub const GLASS_CONCEALMENT: f64 = 0.05;
/// Foliage attenuation per metre of canopy crossed: `1 − exp(−FOLIAGE_K · d)`
/// (0.5 m → 0.22, 6 m → 0.95).
pub const FOLIAGE_K: f64 = 0.5;
/// Two crossings of the same pane closer than this (its two collider faces) count once.
pub const PANE_MERGE_M: f64 = 0.005;

/// Who owns a crossed triangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owner {
    Shell,
    Instance(usize),
}

/// One crossing along the observer→target segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceEvent {
    /// Parametric position on the full segment (0 = observer, 1 = target).
    pub t: f64,
    pub pos: [f64; 3],
    pub kind: SurfaceKind,
    pub owner: Owner,
    pub tri: u32,
}

pub(crate) fn point_at(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [
        a[0] + t * (b[0] - a[0]),
        a[1] + t * (b[1] - a[1]),
        a[2] + t * (b[2] - a[2]),
    ]
}

pub(crate) fn seg_len(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
}

/// Parametric window `[t_in, t_out]` of the segment `a→b` inside the box (`None` when it
/// misses); the slab test, endpoints inclusive.
#[must_use]
pub fn segment_aabb_window(
    a: [f64; 3],
    b: [f64; 3],
    lo: [f64; 3],
    hi: [f64; 3],
) -> Option<(f64, f64)> {
    let mut t0 = 0.0f64;
    let mut t1 = 1.0f64;
    for k in 0..3 {
        let d = b[k] - a[k];
        if d.abs() < 1e-15 {
            if a[k] < lo[k] || a[k] > hi[k] {
                return None;
            }
            continue;
        }
        let mut ta = (lo[k] - a[k]) / d;
        let mut tb = (hi[k] - a[k]) / d;
        if ta > tb {
            core::mem::swap(&mut ta, &mut tb);
        }
        t0 = t0.max(ta);
        t1 = t1.min(tb);
        if t0 > t1 {
            return None;
        }
    }
    Some((t0, t1))
}

/// T-090.12.3 — every crossing of `instances` along `obs→tgt` within `[t_lo, t_hi]`, appended to
/// `out` unsorted with `Owner::Instance(i)` = the index into `instances` (the world occluder
/// re-keys them per placed row before merging).
pub(crate) fn trace_instances(
    instances: &[Instance],
    obs: [f64; 3],
    tgt: [f64; 3],
    t_lo: f64,
    t_hi: f64,
    out: &mut Vec<TraceEvent>,
) {
    let mut hits: Vec<Hit> = Vec::new();
    for (i, inst) in instances.iter().enumerate() {
        let place = inst.placement();
        let Some((p, q)) = local_segment(inst, &place, obs, tgt) else {
            continue;
        };
        inst.blas.bvh.all_hits(
            &inst.blas.verts,
            &inst.blas.tris,
            p,
            q,
            t_lo,
            t_hi,
            &mut hits,
        );
        out.extend(hits.drain(..).map(|h| TraceEvent {
            t: h.t,
            pos: point_at(obs, tgt, h.t),
            kind: inst.blas.kind(h.tri),
            owner: Owner::Instance(i),
            tri: h.tri,
        }));
    }
}

/// T-090.12.3 — does any instance carry a `terminal` crossing on `obs→tgt` within `[t_lo, t_hi]`?
pub(crate) fn blocked_instances_where(
    instances: &[Instance],
    obs: [f64; 3],
    tgt: [f64; 3],
    t_lo: f64,
    t_hi: f64,
    terminal: impl Fn(SurfaceKind) -> bool + Copy,
) -> bool {
    instances.iter().any(|inst| {
        let place = inst.placement();
        let Some((p, q)) = local_segment(inst, &place, obs, tgt) else {
            return false;
        };
        inst.blas
            .bvh
            .any_hit_where(
                &inst.blas.verts,
                &inst.blas.tris,
                &inst.blas.kinds,
                p,
                q,
                t_lo,
                t_hi,
                terminal,
            )
            .is_some()
    })
}

/// Sort by `(t, owner, tri)` and drop the shared-edge duplicates (one crossing, not two).
pub(crate) fn sort_dedup_events(out: &mut Vec<TraceEvent>) {
    out.sort_by(|a, b| {
        a.t.total_cmp(&b.t)
            .then(a.owner.cmp(&b.owner))
            .then(a.tri.cmp(&b.tri))
    });
    out.dedup_by(|b, a| a.owner == b.owner && a.kind == b.kind && (a.t - b.t).abs() < 1e-9);
}

/// What an opaque crossing of `inst` is called.
pub(crate) fn hit_kind_of(inst: &Instance) -> LosHitKind {
    match inst.record.kind {
        InstanceKind::DoorLeaf => LosHitKind::DoorLeaf,
        InstanceKind::DoorFrame => LosHitKind::DoorFrame,
        InstanceKind::WindowFrame | InstanceKind::Glass => LosHitKind::WindowFrame,
        InstanceKind::Furniture => LosHitKind::Furniture,
        InstanceKind::Tree | InstanceKind::TreeCanopy | InstanceKind::Prop => LosHitKind::Prop,
        InstanceKind::Shell => LosHitKind::Solid,
    }
}

/// The material-aware reduction of a sorted event list (T-090.12.3 extraction of the compound's
/// `evaluate_los` walk, shared with the world occluder): opaque terminates and is named by
/// `name`; glass adds [`GLASS_CONCEALMENT`] once per pane (two faces within [`PANE_MERGE_M`]
/// merge); foliage conceals by the depth between its entry and exit crossings, `inside` seeding
/// the instances the observer starts in (their first crossing is an exit); canopies still open
/// when the ray ends close at `t_end`. Glass / foliage keys and `foliage_id` use the
/// `Owner::Instance` index. Hits come back unsorted; see [`finish_concealment`].
pub(crate) struct Reduced {
    pub hits: Vec<LosHit>,
    pub is_clear: bool,
    /// Where the ray stopped (`1` when clear).
    pub t_end: f64,
    pub blocked_by_wall_id: Option<String>,
    pub cover_furniture_id: Option<String>,
    pub window_ids_traversed: Vec<String>,
}

pub(crate) fn reduce_events(
    events: &[TraceEvent],
    obs: [f64; 3],
    tgt: [f64; 3],
    mut inside: HashMap<usize, f64>,
    name: &dyn Fn(&TraceEvent) -> (LosHitKind, String),
    foliage_id: &dyn Fn(usize) -> String,
) -> Reduced {
    let mut r = Reduced {
        hits: Vec::new(),
        is_clear: true,
        t_end: 1.0,
        blocked_by_wall_id: None,
        cover_furniture_id: None,
        window_ids_traversed: Vec::new(),
    };
    let len = seg_len(obs, tgt);
    let mut last_glass: HashMap<usize, f64> = HashMap::new();
    for ev in events {
        match ev.kind {
            SurfaceKind::Opaque => {
                let (kind, id) = name(ev);
                match kind {
                    LosHitKind::Wall => r.blocked_by_wall_id = Some(id.clone()),
                    LosHitKind::Furniture => r.cover_furniture_id = Some(id.clone()),
                    _ => {}
                }
                r.hits.push(LosHit {
                    t: ev.t,
                    pos: ev.pos,
                    kind,
                    id,
                    concealment: 1.0,
                });
                r.is_clear = false;
                r.t_end = ev.t;
                break;
            }
            SurfaceKind::Glass => {
                let Owner::Instance(i) = ev.owner else {
                    continue;
                };
                if last_glass
                    .get(&i)
                    .is_some_and(|t_prev| (ev.t - t_prev) * len <= PANE_MERGE_M)
                {
                    continue;
                }
                last_glass.insert(i, ev.t);
                let id = foliage_id(i);
                r.window_ids_traversed.push(id.clone());
                r.hits.push(LosHit {
                    t: ev.t,
                    pos: ev.pos,
                    kind: LosHitKind::Glass,
                    id,
                    concealment: GLASS_CONCEALMENT,
                });
            }
            SurfaceKind::Foliage => {
                let Owner::Instance(i) = ev.owner else {
                    continue;
                };
                match inside.remove(&i) {
                    Some(t_in) => {
                        r.hits
                            .push(foliage_hit(foliage_id(i), obs, tgt, len, t_in, ev.t))
                    }
                    None => {
                        inside.insert(i, ev.t);
                    }
                }
            }
        }
    }
    let mut open: Vec<(usize, f64)> = inside.into_iter().collect();
    open.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    for (i, t_in) in open {
        if t_in < r.t_end {
            r.hits
                .push(foliage_hit(foliage_id(i), obs, tgt, len, t_in, r.t_end));
        }
    }
    r
}

/// Sort the hits by `(t, concealment)` and fold the pass-through concealments:
/// `1 − Π(1 − cᵢ)`, or `1` when blocked.
pub(crate) fn finish_concealment(hits: &mut [LosHit], is_clear: bool) -> f64 {
    hits.sort_by(|a, b| {
        a.t.total_cmp(&b.t)
            .then(a.concealment.total_cmp(&b.concealment))
    });
    let mut pass = 1.0f64;
    for h in hits.iter() {
        if h.concealment < 1.0 {
            pass *= 1.0 - h.concealment;
        }
    }
    if is_clear { 1.0 - pass } else { 1.0 }
}

/// A foliage crossing of depth `(t_out − t_in) · len` at the entry point.
fn foliage_hit(
    id: String,
    obs: [f64; 3],
    tgt: [f64; 3],
    len: f64,
    t_in: f64,
    t_out: f64,
) -> LosHit {
    let depth = ((t_out - t_in) * len).max(0.0);
    LosHit {
        t: t_in,
        pos: point_at(obs, tgt, t_in),
        kind: LosHitKind::Foliage,
        id,
        concealment: 1.0 - (-FOLIAGE_K * depth).exp(),
    }
}

/// The instance's BLAS-space segment, or `None` when the world AABB cull rejects it.
fn local_segment(
    inst: &Instance,
    place: &Rigid,
    obs: [f64; 3],
    tgt: [f64; 3],
) -> Option<([f64; 3], [f64; 3])> {
    let (lo, hi) = place.aabb_of(inst.bounds.0, inst.bounds.1);
    segment_aabb_window(obs, tgt, lo, hi)?;
    let inv = place.inverse();
    Some((inv.point(obs), inv.point(tgt)))
}

impl CompoundBuilding {
    /// Every crossing of the shell and of every instance along `obs→tgt`, sorted by
    /// `(t, owner, tri)`. Both endpoints inclusive (`t ∈ [0, 1]`).
    #[must_use]
    pub fn trace(&self, obs: [f64; 3], tgt: [f64; 3]) -> Vec<TraceEvent> {
        self.trace_range(obs, tgt, 0.0, 1.0)
    }

    /// [`Self::trace`] restricted to `t ∈ [t_lo, t_hi]`.
    #[must_use]
    pub fn trace_range(
        &self,
        obs: [f64; 3],
        tgt: [f64; 3],
        t_lo: f64,
        t_hi: f64,
    ) -> Vec<TraceEvent> {
        let mut out: Vec<TraceEvent> = Vec::new();
        if obs == tgt {
            return out;
        }
        let mut hits: Vec<Hit> = Vec::new();
        let shell = &self.shell;
        shell
            .bvh
            .all_hits(&shell.verts, &shell.tris, obs, tgt, t_lo, t_hi, &mut hits);
        out.extend(hits.drain(..).map(|h| TraceEvent {
            t: h.t,
            pos: point_at(obs, tgt, h.t),
            kind: shell.kind(h.tri),
            owner: Owner::Shell,
            tri: h.tri,
        }));
        trace_instances(&self.instances, obs, tgt, t_lo, t_hi, &mut out);
        sort_dedup_events(&mut out);
        out
    }

    /// Does anything opaque stand on `obs→tgt`? Glass and foliage never block; a closed leaf
    /// does; an open leaf blocks only where it now hangs.
    #[must_use]
    pub fn blocked(&self, obs: [f64; 3], tgt: [f64; 3]) -> bool {
        self.blocked_range(obs, tgt, 0.0, 1.0)
    }

    /// [`Self::blocked`] restricted to `t ∈ [t_lo, t_hi]` (the parity lane's endpoint policy).
    #[must_use]
    pub fn blocked_range(&self, obs: [f64; 3], tgt: [f64; 3], t_lo: f64, t_hi: f64) -> bool {
        if obs == tgt {
            return false;
        }
        let shell = &self.shell;
        if shell
            .bvh
            .any_hit_where(
                &shell.verts,
                &shell.tris,
                &shell.kinds,
                obs,
                tgt,
                t_lo,
                t_hi,
                SurfaceKind::is_terminal,
            )
            .is_some()
        {
            return true;
        }
        blocked_instances_where(
            &self.instances,
            obs,
            tgt,
            t_lo,
            t_hi,
            SurfaceKind::is_terminal,
        )
    }

    /// Line of sight through the compound: the multi-hit walk of [`Self::trace`] with the
    /// material semantics of the module doc. `bp` (when given) names shell hits — wall id,
    /// window frame, roof, stairs — exactly as [`BuildingBlueprint::evaluate_los`] does;
    /// without it a shell hit is [`LosHitKind::Solid`]. Instance hits are named by their kind
    /// and id. Open doors add a [`LosHitKind::DoorAperture`] where the segment crosses the
    /// leaf's CLOSED footprint. `concealment` is `1 − Π(1 − cᵢ)` over the pass-through events,
    /// `1` when blocked.
    #[must_use]
    pub fn evaluate_los(
        &self,
        bp: Option<&BuildingBlueprint>,
        obs: [f64; 3],
        tgt: [f64; 3],
    ) -> LosResult {
        let events = self.trace(obs, tgt);
        // Observer starting inside a foliage instance's box: its first crossing is an exit.
        let mut inside: HashMap<usize, f64> = HashMap::new();
        for (i, inst) in self.instances.iter().enumerate() {
            if !inst.blas.kinds.contains(&SurfaceKind::Foliage) {
                continue;
            }
            let (lo, hi) = inst.world_aabb();
            if (0..3).all(|k| obs[k] >= lo[k] && obs[k] <= hi[k]) {
                inside.insert(i, 0.0);
            }
        }
        let name = |ev: &TraceEvent| -> (LosHitKind, String) {
            match ev.owner {
                Owner::Shell => bp.map_or_else(
                    || (LosHitKind::Solid, String::new()),
                    |bp| bp.attribute_structural_hit(ev.pos),
                ),
                Owner::Instance(i) => {
                    let inst = &self.instances[i];
                    (hit_kind_of(inst), inst.record.id.clone())
                }
            }
        };
        let id_of = |i: usize| self.instances[i].record.id.clone();
        let reduced = reduce_events(&events, obs, tgt, inside, &name, &id_of);
        let mut result = LosResult {
            is_clear: reduced.is_clear,
            blocked_by_wall_id: reduced.blocked_by_wall_id,
            cover_furniture_id: reduced.cover_furniture_id,
            window_ids_traversed: reduced.window_ids_traversed,
            ..LosResult::default()
        };
        let mut hits = reduced.hits;
        let t_end = reduced.t_end;
        // Open doors: the aperture is where the CLOSED leaf would have been.
        for inst in self
            .instances
            .iter()
            .filter(|i| i.is_door() && i.state.is_open())
        {
            let inv = inst.local.inverse();
            let (p, q) = (inv.point(obs), inv.point(tgt));
            if let Some((t_in, _)) =
                segment_aabb_window(p, q, inst.bounds.0, inst.bounds.1).filter(|w| w.0 <= t_end)
            {
                result.door_ids_traversed.push(inst.record.id.clone());
                hits.push(LosHit {
                    t: t_in,
                    pos: point_at(obs, tgt, t_in),
                    kind: LosHitKind::DoorAperture,
                    id: inst.record.id.clone(),
                    concealment: 0.0,
                });
            }
        }
        result.concealment = finish_concealment(&mut hits, result.is_clear);
        result.hits = hits;
        result
    }
}

#[cfg(test)]
#[path = "building_compound_tests.rs"]
mod tests;
