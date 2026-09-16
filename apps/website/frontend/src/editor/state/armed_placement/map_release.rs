//! Role: the canvas release that commits an armed place — resolve the folder, commit the entity
//! under the active side, select it, and run the shared post-change tail.
//! Position: `editor/state/armed_placement` in the frontend editor shell.
//! Signals & state: the armed value and the active side on the installed editor context, plus the
//! render engine handle the vehicle lane is rebound through.
//! Invariants: the armed value is TAKEN before the document opens, so a release commits at most
//! once. The folder a new entity is filed under is resolved by the host and handed to the document
//! already decided, which keeps the folder mint part of the same undoable act as the place it
//! serves. A release while a zone draw is in flight advances the draw instead — a draw owns the map
//! until it closes or is abandoned.

use super::zone_draw::{advance_zone_draw, zone_draw_armed};
use super::Pending;
use crate::editor::panels::outliner;
use crate::editor::state::editor_context::{place_with_crew, EDITOR_CONTEXT};
use crate::editor::state::history as mission_history;
use leptos::prelude::GetUntracked;
use outliner::ensure_active_layer;
use website_map_engine::data::store::operations::entity::ArmedPlacement;

/// Commit an armed place at a WORLD position, then select it and run the shared post-change tail.
/// `false` when nothing was armed.
#[allow(dead_code)]
pub fn place_at(x: f64, y: f64) -> bool {
    place_at_impl(x, y, false, false)
}

/// Commit an armed place, with `alt_empty` asking a vehicle to spawn without its crew.
pub fn place_at_alt(x: f64, y: f64, alt_empty: bool) -> bool {
    place_at_impl(x, y, alt_empty, false)
}

/// Commit an armed place and RE-ARM the same value, so a palette leaf can be stamped repeatedly
/// without going back to the palette between drops.
pub fn place_at_keep(x: f64, y: f64, alt_empty: bool) -> bool {
    let snapshot = EDITOR_CONTEXT.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.pending.borrow().clone())
    });
    let placed = place_at_impl(x, y, alt_empty, true);
    if placed {
        if let Some(p) = snapshot {
            EDITOR_CONTEXT.with(|c| {
                if let Some(ctx) = c.borrow().as_ref() {
                    *ctx.pending.borrow_mut() = Some(p);
                }
            });
        }
    }
    placed
}

/// Rebind the vehicle symbology lane after a place that added one, so the new glyph appears in the
/// same frame the document changed in.
pub(crate) fn rebind_vehicle_lane_after_place() {
    let (vxy, valiases, vtints, vheadings) = mission_history::vehicle_lane_fields();
    EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let mut engine = ctx.engine.borrow_mut();
        let Some(e) = engine.as_mut() else {
            return;
        };
        e.vehicles_bind_symbology(&vxy, valiases, &vtints, &vheadings);
        e.mark_dirty();
    });
}

/// The armed value as the document's placement machine takes it. The host's arm carries the zone
/// DRAFT, which the draw machine owns and the release path never commits.
fn armed_placement(pending: Pending) -> ArmedPlacement {
    match pending {
        Pending::Character(payload) => ArmedPlacement::Character(payload),
        Pending::Vehicle(payload) => ArmedPlacement::Vehicle(payload),
        Pending::Object(payload) => ArmedPlacement::Object(payload),
        Pending::Composition(comp_id) => ArmedPlacement::Composition(comp_id),
        Pending::Marker(icon) => ArmedPlacement::Marker(icon),
        Pending::Zone(_) => ArmedPlacement::ZoneDraw,
    }
}

/// One canvas release: advance an in-flight zone draw, or commit whatever is armed.
fn place_at_impl(x: f64, y: f64, alt_empty: bool, keep: bool) -> bool {
    // The re-arm is the CALLER's, after the commit reports success: a release that placed nothing
    // must not leave a value armed that the operator believes they dropped.
    let _ = keep;

    if zone_draw_armed() {
        return advance_zone_draw(x, y);
    }

    let placed = EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let pending = ctx.pending.borrow_mut().take()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let side = ctx.active_side.get_untracked();
        let placed = website_map_engine::data::store::operations::entity::commit_armed_placement(
            core,
            armed_placement(pending),
            &side,
            x,
            y,
            place_with_crew(),
            alt_empty,
            &ctx.next_id,
            ensure_active_layer,
        )?;
        if let Some(ids) = placed.selection.clone() {
            *ctx.selection.borrow_mut() = ids;
        }
        Some(placed)
    });
    let Some(placed) = placed else {
        return false;
    };

    mission_history::after_local_edit();

    if placed.placed_vehicle {
        rebind_vehicle_lane_after_place();
    }

    if let Some((asset_id, label)) = placed.stamped_composition {
        crate::editor::panels::dock_right::record_placed(asset_id, label);
    }
    true
}
