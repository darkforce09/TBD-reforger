//! The one sight-line evaluation: the crossings of a scene reduced to named hits, a blocker and a
//! concealment.
//!
//! **Role:** declares [`SightLineScene`], what a sight line crossed as the evaluation reads it,
//! and [`evaluate_los`], which reduces those crossings: the first opaque crossing stops the ray
//! and names what it hit; each glass pane adds [`GLASS_CONCEALMENT`], its two collider faces
//! within [`PANE_MERGE_M`] counting once; foliage adds `1 − exp(−FOLIAGE_K · depth)` for the
//! metres of canopy crossed; an aperture the scene reports adds a zero-concealment hit; and the
//! concealment folds to `1 − Π(1 − cᵢ)` over the pass-through hits, or 1 when blocked.
//! **Position:** the compound walk ([`crate::compound_walk`]) evaluates one building through it,
//! and the world line of sight evaluates the streamed world's placed objects through it; each
//! implements [`SightLineScene`] over its own crossings.
//! **Signals & state:** none; pure functions over a scene's borrowed crossings.
//! **Invariants:** the crossings arrive sorted by `t`; nothing after the first opaque crossing is
//! reported except a foliage volume or aperture entered before it; glass and foliage conceal but
//! never block; the returned hits are sorted by `(t, concealment)`.

use std::collections::HashMap;

use building_interiors::blueprint::sight_line::{LosHit, LosHitKind, LosResult};
use building_interiors::building_ids::BuildingFeatureId;
use geometry_primitives::segment_geometry::point_at;
use spatial_indexes::bounding_volume_hierarchy::surface_kind::SurfaceKind;

use crate::compound_walk::{Owner, TraceEvent};

/// Concealment one glass pane adds (non-terminal).
pub const GLASS_CONCEALMENT: f64 = 0.05;

/// Foliage attenuation per metre of canopy crossed: `1 − exp(−FOLIAGE_K · d)` (0.5 m → 0.22, 6 m → 0.95).
pub const FOLIAGE_K: f64 = 0.5;

/// Two crossings of the same pane closer than this (its two collider faces) count once.
pub const PANE_MERGE_M: f64 = 0.005;

/// What one sight line crossed, as [`evaluate_los`] reads it: the segment, its crossings with
/// each instance owner an index into the scene's own table, and the names of what was crossed.
pub trait SightLineScene {
    /// The observer (`t = 0`) and the target (`t = 1`) of the sight line.
    fn endpoints(&self) -> ([f64; 3], [f64; 3]);

    /// Every crossing of the sight line, sorted by `(t, owner, tri)`.
    fn crossings(&self) -> &[TraceEvent];

    /// The foliage volumes the observer stands inside, keyed by owner index, each entered at
    /// `t = 0`.
    fn foliage_around_observer(&self) -> HashMap<usize, f64>;

    /// What an opaque crossing is called and the feature it belongs to.
    fn name_crossing(&self, crossing: &TraceEvent) -> (LosHitKind, BuildingFeatureId);

    /// The feature the instance at owner index `owner` is named by (a glass pane or a foliage
    /// volume is named after its instance).
    fn name_instance(&self, owner: usize) -> BuildingFeatureId;

    /// The apertures (open door leaves) the ray passes before it stops at `t_end`, as
    /// zero-concealment hits; a scene without apertures reports none.
    fn apertures_before(&self, _t_end: f64) -> Vec<LosHit> {
        Vec::new()
    }
}

/// The evaluation of one sight line: whether it is clear, where it stopped, every event named,
/// and the concealment folded over them.
#[derive(Clone, Debug, PartialEq)]
pub struct SightLineEvaluation {
    /// No opaque crossing stands on the segment.
    pub is_clear: bool,

    /// Where the ray stopped: the first opaque crossing's `t`, or 1 when clear.
    pub t_end: f64,

    /// Every event along the ray, sorted by `(t, concealment)`.
    pub hits: Vec<LosHit>,

    /// `1 − Π(1 − cᵢ)` over the pass-through hits, or 1 when blocked.
    pub concealment: f64,

    /// The wall that stopped the ray, when a wall did.
    pub blocked_by_wall_id: Option<BuildingFeatureId>,

    /// The furniture that stopped the ray, when furniture did.
    pub cover_furniture_id: Option<BuildingFeatureId>,

    /// The glass panes passed, in order.
    pub window_ids_traversed: Vec<BuildingFeatureId>,

    /// The apertures passed, in the order the scene reported them.
    pub door_ids_traversed: Vec<BuildingFeatureId>,
}

impl From<SightLineEvaluation> for LosResult {
    fn from(evaluation: SightLineEvaluation) -> Self {
        Self {
            is_clear: evaluation.is_clear,
            hits: evaluation.hits,
            window_ids_traversed: evaluation.window_ids_traversed,
            door_ids_traversed: evaluation.door_ids_traversed,
            blocked_by_wall_id: evaluation.blocked_by_wall_id,
            cover_furniture_id: evaluation.cover_furniture_id,
            concealment: evaluation.concealment,
        }
    }
}

/// Reduces the crossings of `scene` to the evaluation of its sight line: the first opaque
/// crossing stops it, glass, foliage and apertures before that point conceal it.
#[must_use]
pub fn evaluate_los(scene: &impl SightLineScene) -> SightLineEvaluation {
    let (obs, tgt) = scene.endpoints();
    let name = |crossing: &TraceEvent| scene.name_crossing(crossing);
    let name_instance = |owner: usize| scene.name_instance(owner);
    let reduced = reduce_events(
        scene.crossings(),
        obs,
        tgt,
        scene.foliage_around_observer(),
        &name,
        &name_instance,
    );
    let mut hits = reduced.hits;
    let apertures = scene.apertures_before(reduced.t_end);
    let door_ids_traversed = apertures.iter().map(|hit| hit.id.clone()).collect();
    hits.extend(apertures);
    let concealment = finish_concealment(&mut hits, reduced.is_clear);
    SightLineEvaluation {
        is_clear: reduced.is_clear,
        t_end: reduced.t_end,
        hits,
        concealment,
        blocked_by_wall_id: reduced.blocked_by_wall_id,
        cover_furniture_id: reduced.cover_furniture_id,
        window_ids_traversed: reduced.window_ids_traversed,
        door_ids_traversed,
    }
}

/// The crossings reduced up to the first opaque one.
struct Reduced {
    /// The events, in crossing order (an open foliage volume last).
    hits: Vec<LosHit>,

    /// No opaque crossing.
    is_clear: bool,

    /// Where the ray stopped (`1` when clear).
    t_end: f64,

    /// The wall that stopped the ray.
    blocked_by_wall_id: Option<BuildingFeatureId>,

    /// The furniture that stopped the ray.
    cover_furniture_id: Option<BuildingFeatureId>,

    /// The glass panes passed.
    window_ids_traversed: Vec<BuildingFeatureId>,
}

/// Walks the sorted crossings: an opaque one stops the walk, a glass pane conceals once per pane,
/// a foliage volume conceals by the depth between its entry and exit (or the stop).
fn reduce_events(
    events: &[TraceEvent],
    obs: [f64; 3],
    tgt: [f64; 3],
    mut inside: HashMap<usize, f64>,
    name: &dyn Fn(&TraceEvent) -> (LosHitKind, BuildingFeatureId),
    foliage_id: &dyn Fn(usize) -> BuildingFeatureId,
) -> Reduced {
    let mut r = Reduced {
        hits: Vec::new(),
        is_clear: true,
        t_end: 1.0,
        blocked_by_wall_id: None,
        cover_furniture_id: None,
        window_ids_traversed: Vec::new(),
    };
    let len = segment_length(obs, tgt);
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

/// Sorts the hits by `(t, concealment)` and folds the pass-through concealments: `1 − Π(1 − cᵢ)`,
/// or `1` when blocked.
fn finish_concealment(hits: &mut [LosHit], is_clear: bool) -> f64 {
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

/// The hit of a foliage volume entered at `t_in` and left at `t_out`.
fn foliage_hit(
    id: BuildingFeatureId,
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

/// The length of the segment `a→b` in metres.
fn segment_length(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
}
