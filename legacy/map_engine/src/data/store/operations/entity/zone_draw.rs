//! Role: the in-flight zone or trigger draw — the draft, the clicks that build it, and the
//! geometry it yields when it closes.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: the draft is passed in and mutated in place; the host keeps it.
//! Invariants: a draw yields geometry exactly once. A circle's rim click that does not survive the
//! caller's radius rule leaves the draft in flight with its centre kept, rather than committing a
//! shape the schema would refuse, and a ring below three vertices never closes.

use super::DrawTarget;
use super::TRIGGER_ACTIVATIONS;
use super::ZoneShape;

/// A draw in progress. It lives on the host's armed placement rather than in a reading of its own,
/// because "is a draw in flight" is what routes a map release to the draw instead of the select
/// machine — and re-deriving that from a second source is how the two get out of step.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneDraft {
    /// The authored kind: a `zone.type` for a zone, an activation for a trigger. Taken from the
    /// schema by whoever began the draw — never typed here.
    pub kind: String,

    /// Circle or polygon.
    pub shape: ZoneShape,

    /// Circle: the centre, set by the first click. `None` until then.
    pub centre: Option<(f64, f64)>,

    /// Polygon: the ring so far, one vertex per click.
    pub verts: Vec<(f64, f64)>,

    /// The id being reshaped, when the draw edits an existing row rather than minting one.
    pub target: Option<String>,

    /// Which authored collection the draw commits into.
    pub collection: DrawTarget,
}

/// May a draw into `collection` be begun for `kind`? A trigger's kind is one of the closed
/// [`TRIGGER_ACTIVATIONS`]; a zone's is one of the types `authored_zone_types` yields, which the
/// caller reads from the schema because the schema is where that list is authored. The list is
/// asked for only on the zone branch, so a trigger draw never pays to read it.
pub fn zone_draft_kind_is_valid(
    kind: &str,
    collection: DrawTarget,
    authored_zone_types: impl FnOnce() -> Vec<String>,
) -> bool {
    match collection {
        DrawTarget::Zone => authored_zone_types().iter().any(|t| t == kind),
        DrawTarget::Trigger => TRIGGER_ACTIVATIONS.contains(&kind),
    }
}

/// An empty draft. `target` is `Some` when the draw reshapes an existing row and `None` when it
/// mints one; nothing else distinguishes a reshape from a create.
#[must_use]
pub fn begin_zone_draft(
    kind: String,
    shape: ZoneShape,
    collection: DrawTarget,
    target: Option<String>,
) -> ZoneDraft {
    ZoneDraft {
        kind,
        shape,
        centre: None,
        verts: Vec::new(),
        target,
        collection,
    }
}

/// Drop the last polygon vertex. Returns the vertices remaining.
pub fn pop_zone_draft_vertex(draft: &mut ZoneDraft) -> usize {
    draft.verts.pop();
    draft.verts.len()
}

/// What one map release did to a draw.
#[derive(Clone, Debug, PartialEq)]
pub enum ZoneDrawStep {
    /// A vertex was appended, or a circle's centre captured, or a rim click was rejected by the
    /// caller's radius rule. The draw stays in flight.
    Drawing,

    /// The rim click closed the circle. The draft is spent: the host drops it and commits this
    /// geometry into `collection`, onto `target` when reshaping and onto a fresh id otherwise.
    CircleClosed {
        /// The authored kind the draw was begun with.
        kind: String,

        /// Centre in world metres.
        centre: (f64, f64),

        /// Radius in metres.
        radius: f64,

        /// The row being reshaped, or `None` for a create.
        target: Option<String>,

        /// Which collection to commit into.
        collection: DrawTarget,
    },
}

/// One map release at `(x, z)` in world metres.
///
/// A polygon draw appends a vertex. A circle draw captures its centre on the first release and
/// resolves its radius on the second, through `radius_from_rim` — the caller's rule for which
/// centre/rim pairs are a real circle, which is where the "too small to survive a compile" guard
/// lives. A pair that rule rejects keeps the draft in flight with its centre, so the author can
/// click a wider rim without starting over.
pub fn advance_zone_draft(
    draft: &mut ZoneDraft,
    x: f64,
    z: f64,
    radius_from_rim: impl FnOnce(f64, f64, f64, f64) -> Option<(f64, f64, f64)>,
) -> ZoneDrawStep {
    match draft.shape {
        ZoneShape::Polygon => {
            draft.verts.push((x, z));
            ZoneDrawStep::Drawing
        }
        ZoneShape::Circle => {
            let Some((cx, cz)) = draft.centre else {
                draft.centre = Some((x, z));
                return ZoneDrawStep::Drawing;
            };
            match radius_from_rim(cx, cz, x, z) {
                None => ZoneDrawStep::Drawing,
                Some((centre_x, centre_z, radius)) => ZoneDrawStep::CircleClosed {
                    kind: draft.kind.clone(),
                    centre: (centre_x, centre_z),
                    radius,
                    target: draft.target.clone(),
                    collection: draft.collection,
                },
            }
        }
    }
}

/// The ring a closed polygon draw commits.
#[derive(Clone, Debug, PartialEq)]
pub struct ZonePolygonCommit {
    /// The authored kind the draw was begun with.
    pub kind: String,

    /// The closed ring, in world metres.
    pub ring: Vec<(f64, f64)>,

    /// The row being reshaped, or `None` for a create.
    pub target: Option<String>,

    /// Which collection to commit into.
    pub collection: DrawTarget,
}

/// Close the in-flight ring, if it can be closed. `None` — leaving the draft in flight — when the
/// draw is a circle, or when `ring_is_committable` refuses the ring: `$defs/polygon` is
/// `minItems: 3`, and a two-vertex ring is a document the schema rejects.
pub fn close_zone_polygon_draft(
    draft: &ZoneDraft,
    ring_is_committable: impl FnOnce(&[(f64, f64)]) -> bool,
) -> Option<ZonePolygonCommit> {
    if draft.shape != ZoneShape::Polygon || !ring_is_committable(&draft.verts) {
        return None;
    }
    Some(ZonePolygonCommit {
        kind: draft.kind.clone(),
        ring: draft.verts.clone(),
        target: draft.target.clone(),
        collection: draft.collection,
    })
}

#[cfg(test)]
#[path = "tests/zone_draw.rs"]
mod tests;
