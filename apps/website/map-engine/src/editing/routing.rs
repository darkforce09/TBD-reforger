//! Role: resolve a subject id to the selection surface that owns it, and narrow that resolution to
//! what a click would actually reach.
//! Position: `editing` in the map engine.
//! Signals & state: none; the document root and the two facts the document cannot carry (is this
//! id a slot, is the zones panel mounted) arrive as arguments.
//! Invariants: one resolution answers both the affordance probe and the click, so a row is
//! clickable if and only if clicking it does something; a widening appends an arm and never
//! changes what an already-resolving id resolves to.

use crate::editing::selection_universe::zone_centre;

/// What [`route_target`] resolved a subject id to — i.e. WHICH selection surface owns it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RouteTarget {
    /// A slot: the caller's SoA predicate matched. The position comes from the SoA row, which the
    /// caller already holds, so no coordinates ride this arm.
    Slot,
    /// A `vehiclesById` row, at its authored `position` in world metres.
    Vehicle {
        /// World easting, in metres.
        x: f64,
        /// World northing, in metres.
        y: f64,
    },
    /// An `entitiesById` row — a placed world object — at its authored `position`.
    ///
    /// Rides the SAME path as [`RouteTarget::Vehicle`] (editor selection plus camera centre),
    /// because a placed object is off the slot SoA in exactly the way a vehicle is: neither is
    /// tinted by the slot lane, and both are real members of the editor selection the attributes
    /// and history mirrors read. This arm is what lets an asset-resolution finding, whose subject
    /// is an `entities[]` row id, resolve at all.
    Entity {
        /// World easting, in metres.
        x: f64,
        /// World northing, in metres.
        y: f64,
    },
    /// A `zonesById` row, at its geometric centre. Selected in the Zones panel, not in the slot
    /// selection — a zone id in the slot selection would read as one selected entity with nothing
    /// highlighted.
    Zone {
        /// World easting, in metres.
        x: f64,
        /// World northing, in metres.
        y: f64,
    },
    /// A `commentsById` row: the editor-only annotation, at its authored position.
    ///
    /// Rides the SAME path as [`RouteTarget::Vehicle`] and [`RouteTarget::Entity`] because it
    /// belongs in the same place they do: the selection is ONE vector, and the composition capture
    /// classifies each selected id as slot / vehicle / object / comment off that one vector. A
    /// comment in a lane of its own would be a selection the composition could never see.
    Comment {
        /// World easting, in metres.
        x: f64,
        /// World northing, in metres.
        y: f64,
    },
}

/// **Where a `subject_id` would go if it were clicked**, over the document's small-maps root plus
/// `is_slot` (slot ids live in the SoA, which is not in that root, so the one fact this function
/// cannot read is supplied by the caller).
///
/// `None` means NOTHING would be selected — a stale id whose entity was deleted, or a kind no
/// selection surface owns. A view MUST NOT paint a click affordance on a row this returns `None`
/// for; that is the lie this function exists to make unnecessary.
///
/// Order is slot, vehicle, entity, zone, comment. Each map is keyed by its own minted id, so the
/// id spaces are disjoint and appending an arm cannot change what an existing id resolves to —
/// appending is the placement that guarantees it without re-arguing disjointness each time.
pub fn route_target(
    root: &serde_json::Value,
    subject_id: &str,
    is_slot: &dyn Fn(&str) -> bool,
) -> Option<RouteTarget> {
    if is_slot(subject_id) {
        return Some(RouteTarget::Slot);
    }
    if let Some(p) = root
        .get("vehiclesById")
        .and_then(|m| m.get(subject_id))
        .and_then(|v| v.get("position"))
    {
        if let (Some(x), Some(y)) = (
            p.get("x").and_then(serde_json::Value::as_f64),
            p.get("y").and_then(serde_json::Value::as_f64),
        ) {
            return Some(RouteTarget::Vehicle { x, y });
        }
    }
    // Placed world objects. An `entitiesById` row carries the SAME `position {x, y, z, rotation}`
    // shape a `vehiclesById` row does, so this is the vehicle lookup over a second map rather than
    // a second kind of resolution.
    if let Some(p) = root
        .get("entitiesById")
        .and_then(|m| m.get(subject_id))
        .and_then(|v| v.get("position"))
    {
        if let (Some(x), Some(y)) = (
            p.get("x").and_then(serde_json::Value::as_f64),
            p.get("y").and_then(serde_json::Value::as_f64),
        ) {
            return Some(RouteTarget::Entity { x, y });
        }
    }
    if let Some(zone) = root.get("zonesById").and_then(|m| m.get(subject_id)) {
        if let Some((x, y)) = zone_centre(zone) {
            return Some(RouteTarget::Zone { x, y });
        }
    }
    // The axes are `{x, z}`, not `{x, y}`: a comment row carries TWO HORIZONTALS and no height.
    // Reading `y` here would find nothing, return `None`, and leave the row inert under an
    // affordance that had already been painted.
    if let Some(p) = root
        .get("commentsById")
        .and_then(|m| m.get(subject_id))
        .and_then(|v| v.get("position"))
    {
        if let (Some(x), Some(y)) = (
            p.get("x").and_then(serde_json::Value::as_f64),
            p.get("z").and_then(serde_json::Value::as_f64),
        ) {
            return Some(RouteTarget::Comment { x, y });
        }
    }
    None
}

/// **AVAILABILITY: what a click on this subject would actually REACH.**
///
/// [`route_target`] answers "which surface owns this id?" over the document alone. That is not the
/// whole of "would this click do something", because one arm needs a SEAM as well as a row: a
/// [`RouteTarget::Zone`] is selected through the Zones panel's own selection hook, which reports
/// `false` when that panel is not mounted. So a zone whose panel is gone RESOLVES but is not
/// AVAILABLE, and a click on it does nothing.
///
/// This is **ONE narrowing that the affordance probe and the click both go through**, so "a row is
/// clickable IFF clicking it does something" is a single decision rather than a condition written
/// twice and kept in step by hope.
///
/// `resolved` is [`route_target`]'s answer with the centre the caller already computed;
/// `zone_panel_live` is the one fact the document cannot carry. Every non-`Zone` arm passes
/// straight through: they select into the editor selection, which exists for as long as the router
/// itself does.
pub fn route_availability(
    resolved: Option<(RouteTarget, f64, f64)>,
    zone_panel_live: &dyn Fn() -> bool,
) -> Option<(RouteTarget, f64, f64)> {
    let (target, x, y) = resolved?;
    if matches!(target, RouteTarget::Zone { .. }) && !zone_panel_live() {
        return None;
    }
    Some((target, x, y))
}
