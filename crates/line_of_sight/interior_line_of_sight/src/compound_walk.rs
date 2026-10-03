//! The walk of a sight line through one building compound: every crossing, whether anything
//! opaque stands on it, and its verdict.
//!
//! **Role:** traces a segment in the building's frame through the shell and every placed instance
//! ([`CompoundLineOfSight::trace`]), answers whether anything terminal stands on it
//! ([`CompoundLineOfSight::blocked`]), and evaluates it through the shared sight-line evaluation
//! ([`CompoundLineOfSight::evaluate_los`]), naming shell hits after the blueprint's features and
//! instance hits after their instance ids; the per-instance trace and blocking test
//! ([`trace_instances`], [`blocked_instances_where`]) and the hit naming ([`hit_kind_of`]) serve the
//! world line of sight's expanded prefabs too.
//! **Position:** over `building_interiors`' compound and blueprint and `spatial_indexes`' BVH
//! traversal; the floor wash ([`crate::floor_wash`]) washes through `blocked`, the world line of
//! sight traces its prefabs through the instance functions, and the debug building viewer and the
//! blueprint parity tooling call the trait.
//! **Signals & state:** none; the compound's door states are its owner's.
//! **Invariants:** crossings are sorted by `(t, owner, tri)` with a shared-edge duplicate counted
//! once; glass and foliage never block; a closed door leaf blocks, an open one only where it hangs.

use std::collections::HashMap;

use building_interiors::blueprint::sight_line::{LosHit, LosHitKind, LosResult};
use building_interiors::blueprint::structure::BuildingBlueprint;
use building_interiors::building_ids::BuildingFeatureId;
use building_interiors::compound::assembly::CompoundBuilding;
use building_interiors::compound::instances::{Instance, InstanceKind};
use geometry_primitives::rigid_transform::Rigid;
use geometry_primitives::segment_geometry::point_at;
use spatial_indexes::bounding_volume_hierarchy::segment_box_window::segment_aabb_window;
use spatial_indexes::bounding_volume_hierarchy::surface_kind::SurfaceKind;
use spatial_indexes::bounding_volume_hierarchy::triangle_tree::Hit;

use crate::sight_line_evaluation::{SightLineScene, evaluate_los};

/// Who owns a crossed triangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owner {
    /// The building shell's own mesh.
    Shell,

    /// The placed instance at this index of the compound (or of the scene's own table).
    Instance(usize),
}

/// One crossing along the observer→target segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceEvent {
    /// Parametric position on the full segment (0 = observer, 1 = target).
    pub t: f64,

    /// The crossing point in the segment's frame.
    pub pos: [f64; 3],

    /// The surface kind of the crossed triangle.
    pub kind: SurfaceKind,

    /// Who owns the crossed triangle.
    pub owner: Owner,

    /// The crossed triangle's index in its owner's mesh.
    pub tri: u32,
}

/// Appends every crossing of `instances` along `obs→tgt` restricted to `t ∈ [t_lo, t_hi]`, each
/// instance traced in its own frame and owned by its index, unsorted.
pub fn trace_instances(
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

/// Whether any of `instances` has a triangle whose kind `terminal` accepts on `obs→tgt` within
/// `t ∈ [t_lo, t_hi]`.
pub fn blocked_instances_where(
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
fn sort_dedup_events(out: &mut Vec<TraceEvent>) {
    out.sort_by(|a, b| {
        a.t.total_cmp(&b.t)
            .then(a.owner.cmp(&b.owner))
            .then(a.tri.cmp(&b.tri))
    });
    out.dedup_by(|b, a| a.owner == b.owner && a.kind == b.kind && (a.t - b.t).abs() < 1e-9);
}

/// What an opaque crossing of `inst` is called.
#[must_use]
pub fn hit_kind_of(inst: &Instance) -> LosHitKind {
    match inst.record.kind {
        InstanceKind::DoorLeaf => LosHitKind::DoorLeaf,
        InstanceKind::DoorFrame => LosHitKind::DoorFrame,
        InstanceKind::WindowFrame | InstanceKind::Glass => LosHitKind::WindowFrame,
        InstanceKind::Furniture => LosHitKind::Furniture,
        InstanceKind::Tree | InstanceKind::TreeCanopy | InstanceKind::Prop => LosHitKind::Prop,
        InstanceKind::Shell => LosHitKind::Solid,
    }
}

/// The sight-line feature an instance's crossing is named by: its compound instance id.
fn feature_of(inst: &Instance) -> BuildingFeatureId {
    BuildingFeatureId::new(inst.record.id.as_str())
}

/// `obs→tgt` in the frame of `inst` placed by `place`, or `None` when the segment misses the
/// instance's placed box.
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

/// The interior line of sight over a building compound: every crossing of a sight line, whether
/// anything opaque stands on it, and its verdict with every event named.
pub trait CompoundLineOfSight {
    /// Every crossing of the shell and of every instance along `obs→tgt`, sorted by
    /// `(t, owner, tri)`. Both endpoints inclusive (`t ∈ [0, 1]`).
    #[must_use]
    fn trace(&self, obs: [f64; 3], tgt: [f64; 3]) -> Vec<TraceEvent>;

    /// [`Self::trace`] restricted to `t ∈ [t_lo, t_hi]`.
    #[must_use]
    fn trace_range(&self, obs: [f64; 3], tgt: [f64; 3], t_lo: f64, t_hi: f64) -> Vec<TraceEvent>;

    /// Does anything opaque stand on `obs→tgt`? Glass and foliage never block; a closed leaf
    /// does; an open leaf blocks only where it now hangs.
    #[must_use]
    fn blocked(&self, obs: [f64; 3], tgt: [f64; 3]) -> bool;

    /// [`Self::blocked`] restricted to `t ∈ [t_lo, t_hi]` (the parity lane's endpoint policy).
    #[must_use]
    fn blocked_range(&self, obs: [f64; 3], tgt: [f64; 3], t_lo: f64, t_hi: f64) -> bool;

    /// The verdict of `obs→tgt`: the first terminal crossing, the glass, foliage and open doors
    /// passed, each named after its instance or, for the shell, after the blueprint feature
    /// `bp` attributes it to.
    #[must_use]
    fn evaluate_los(
        &self,
        bp: Option<&BuildingBlueprint>,
        obs: [f64; 3],
        tgt: [f64; 3],
    ) -> LosResult;
}

impl CompoundLineOfSight for CompoundBuilding {
    fn trace(&self, obs: [f64; 3], tgt: [f64; 3]) -> Vec<TraceEvent> {
        self.trace_range(obs, tgt, 0.0, 1.0)
    }

    fn trace_range(&self, obs: [f64; 3], tgt: [f64; 3], t_lo: f64, t_hi: f64) -> Vec<TraceEvent> {
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

    fn blocked(&self, obs: [f64; 3], tgt: [f64; 3]) -> bool {
        self.blocked_range(obs, tgt, 0.0, 1.0)
    }

    fn blocked_range(&self, obs: [f64; 3], tgt: [f64; 3], t_lo: f64, t_hi: f64) -> bool {
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

    fn evaluate_los(
        &self,
        bp: Option<&BuildingBlueprint>,
        obs: [f64; 3],
        tgt: [f64; 3],
    ) -> LosResult {
        evaluate_los(&CompoundSightLine::trace(self, bp, obs, tgt)).into()
    }
}

/// One sight line through a compound, traced once: the scene the shared evaluation reduces.
struct CompoundSightLine<'a> {
    /// The compound the sight line crosses.
    compound: &'a CompoundBuilding,

    /// The blueprint shell hits are attributed to, if any.
    blueprint: Option<&'a BuildingBlueprint>,

    /// The observer, in the building's frame.
    obs: [f64; 3],

    /// The target, in the building's frame.
    tgt: [f64; 3],

    /// Every crossing of `obs→tgt`, sorted.
    crossings: Vec<TraceEvent>,
}

impl<'a> CompoundSightLine<'a> {
    /// Traces `obs→tgt` through `compound`.
    fn trace(
        compound: &'a CompoundBuilding,
        blueprint: Option<&'a BuildingBlueprint>,
        obs: [f64; 3],
        tgt: [f64; 3],
    ) -> Self {
        Self {
            compound,
            blueprint,
            obs,
            tgt,
            crossings: compound.trace(obs, tgt),
        }
    }
}

impl SightLineScene for CompoundSightLine<'_> {
    fn endpoints(&self) -> ([f64; 3], [f64; 3]) {
        (self.obs, self.tgt)
    }

    fn crossings(&self) -> &[TraceEvent] {
        &self.crossings
    }

    fn foliage_around_observer(&self) -> HashMap<usize, f64> {
        let obs = self.obs;
        let mut inside: HashMap<usize, f64> = HashMap::new();
        for (i, inst) in self.compound.instances.iter().enumerate() {
            if !inst.blas.kinds.contains(&SurfaceKind::Foliage) {
                continue;
            }
            let (lo, hi) = inst.world_aabb();
            if (0..3).all(|k| obs[k] >= lo[k] && obs[k] <= hi[k]) {
                inside.insert(i, 0.0);
            }
        }
        inside
    }

    fn name_crossing(&self, crossing: &TraceEvent) -> (LosHitKind, BuildingFeatureId) {
        match crossing.owner {
            Owner::Shell => self.blueprint.map_or_else(
                || (LosHitKind::Solid, BuildingFeatureId::new("")),
                |bp| bp.attribute_structural_hit(crossing.pos),
            ),
            Owner::Instance(i) => {
                let inst = &self.compound.instances[i];
                (hit_kind_of(inst), feature_of(inst))
            }
        }
    }

    fn name_instance(&self, owner: usize) -> BuildingFeatureId {
        feature_of(&self.compound.instances[owner])
    }

    fn apertures_before(&self, t_end: f64) -> Vec<LosHit> {
        let (obs, tgt) = (self.obs, self.tgt);
        let mut apertures = Vec::new();
        for inst in self
            .compound
            .instances
            .iter()
            .filter(|i| i.is_door() && i.state.is_open())
        {
            let inv = inst.local.inverse();
            let (p, q) = (inv.point(obs), inv.point(tgt));
            if let Some((t_in, _)) =
                segment_aabb_window(p, q, inst.bounds.0, inst.bounds.1).filter(|w| w.0 <= t_end)
            {
                apertures.push(LosHit {
                    t: t_in,
                    pos: point_at(obs, tgt, t_in),
                    kind: LosHitKind::DoorAperture,
                    id: feature_of(inst),
                    concealment: 0.0,
                });
            }
        }
        apertures
    }
}

#[cfg(test)]
#[path = "tests/compound_walk_tests.rs"]
mod tests;
