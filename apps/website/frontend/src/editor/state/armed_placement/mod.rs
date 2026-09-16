//! Role: the in-flight placement — what the operator has picked up from a palette and not yet
//! dropped on the map, and the map release that commits it.
//! Position: `editor/state` in the frontend editor shell.
//! Signals & state: the armed value on the installed editor context, and the reactive document
//! tick every arm change nudges so the docks re-read their hints.
//! Invariants: the discriminant lives on the ARMED VALUE rather than on a reading of which palette
//! tab is open — the tab can change, or the surface holding it can unmount, between the pick-up and
//! the commit, and a placement must commit the entity the operator actually picked up. Nothing here
//! is document state: an arm is never undoable, and a release that commits nothing leaves no trace.

use crate::editor::state::operations::context::{bump_doc_tick, Pending, OPS_CTX};
use leptos::prelude::GetUntracked;
use website_map_engine::data::store::operations::entity::ArmedPlacementKind;

mod palette_arming;

/// Expose palette arming :: { armed composition id , armed marker icon , begin place , begin place composition , begin place marker , begin place object , begin place vehicle , } at this domain boundary.
pub use palette_arming::{
    armed_composition_id, armed_marker_icon, begin_place, begin_place_composition,
    begin_place_marker, begin_place_object, begin_place_vehicle,
};

mod map_release;

/// Expose map release :: { place at alt , place at keep } at this domain boundary.
pub use map_release::{place_at_alt, place_at_keep};

mod zone_draw;

/// Expose zone draw :: { begin zone draw , begin zone reshape , cancel zone draw , close zone polygon , zone draft , zone draw armed , zone draw pop vertex , } at this domain boundary.
pub use zone_draw::{
    begin_zone_draw, begin_zone_reshape, cancel_zone_draw, close_zone_polygon, zone_draft,
    zone_draw_pop_vertex,
};

/// Which collection the armed value commits into — the discriminant the arm gate reads, with the
/// payload left behind because the gate never looks at it.
fn armed_placement_kind(pending: &Pending) -> ArmedPlacementKind {
    match pending {
        Pending::Character(_) => ArmedPlacementKind::Character,
        Pending::Vehicle(_) => ArmedPlacementKind::Vehicle,
        Pending::Object(_) => ArmedPlacementKind::Object,
        Pending::Composition(_) => ArmedPlacementKind::Composition,
        Pending::Marker(_) => ArmedPlacementKind::Marker,
        Pending::Zone(_) => ArmedPlacementKind::Zone,
    }
}

/// Arm a place. Objects mode only accepts an object arm; the side modes reject one, so a leftover
/// Objects arm cannot commit after the mode chip switches away.
pub(crate) fn arm(pending: Pending) {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let objects = ctx.objects_mode.get_untracked();
            let ok = website_map_engine::data::store::operations::entity::placement_is_armable(
                armed_placement_kind(&pending),
                objects,
            );
            if !ok {
                *ctx.pending.borrow_mut() = None;
                return;
            }
            *ctx.pending.borrow_mut() = Some(pending);
        }
    });

    bump_doc_tick();
}

/// Is anything armed right now? This is what routes a canvas release to a place instead of the
/// select machine.
#[must_use]
pub fn has_pending() -> bool {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .is_some_and(|ctx| ctx.pending.borrow().is_some())
    })
}

/// Drop the armed place — a release over chrome, or a pointercancel. A zone draw deliberately
/// survives this: it has its own explicit abandon.
pub fn cancel_pending() {
    let cleared = OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let mut p = ctx.pending.borrow_mut();
            if matches!(*p, Some(Pending::Zone(_))) {
                return false;
            }
            let had = p.is_some();
            *p = None;
            return had;
        }
        false
    });
    if cleared {
        bump_doc_tick();
    }
}
