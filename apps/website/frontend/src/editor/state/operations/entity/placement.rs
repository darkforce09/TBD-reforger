//! Role: placement.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;
use website_map_engine::data::store::operations::entity::ArmedPlacement;

/// Commit an armed place at a **world** position, then select it and run the shared post-change tail. Returns `false` when nothing was armed.
#[allow(dead_code)]
pub fn place_at(x: f64, y: f64) -> bool {
    place_at_impl(x, y, false, false)
}

/// Place at alt using the supplied domain data.
pub fn place_at_alt(x: f64, y: f64, alt_empty: bool) -> bool {
    place_at_impl(x, y, alt_empty, false)
}

/// Place at keep using the supplied domain data.
pub fn place_at_keep(x: f64, y: f64, alt_empty: bool) -> bool {
    let snapshot = OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.pending.borrow().clone())
    });
    let placed = place_at_impl(x, y, alt_empty, true);
    if placed {
        if let Some(p) = snapshot {
            OPS_CTX.with(|c| {
                if let Some(ctx) = c.borrow().as_ref() {
                    *ctx.pending.borrow_mut() = Some(p);
                }
            });
        }
    }
    placed
}

/// Rebind vehicle lane after place using the supplied domain data.
pub(in crate::editor::state::operations) fn rebind_vehicle_lane_after_place() {
    let (vxy, valiases, vtints, vheadings) = mission_history::vehicle_lane_fields();
    OPS_CTX.with(|c| {
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

/// The armed value as the engine's placement machine takes it. The host's arm carries the zone
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

/// Place at impl using the supplied domain data.
pub(in crate::editor::state::operations) fn place_at_impl(
    x: f64,
    y: f64,
    alt_empty: bool,
    keep: bool,
) -> bool {
    let _ = keep;

    if zone_draw_armed() {
        return advance_zone_draw(x, y);
    }

    let placed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let pending = ctx.pending.borrow_mut().take()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let side = ctx.active_side.get_untracked();
        let placed =
            website_map_engine::data::store::operations::entity::commit_armed_placement(
                core,
                armed_placement(pending),
                &side,
                x,
                y,
                place_with_crew(),
                alt_empty,
                &ctx.next_id,
                |core| ensure_layer(ctx, core),
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

/// Returns `false` (no-op) when the target has no squad (an unfiled slot, or the target vanished), or already shares the dragged slot's squad — `move_slot_to_squad` is itself a no-op on same-squad, but declining here keeps the caller from firing the dirty tail for nothing.
pub fn regroup_slot_onto(slot_id: &str, target_id: &str) -> bool {
    if slot_id == target_id {
        return false;
    }
    let dest_squad = read_attrs(target_id).map(|a| a.squad).unwrap_or_default();
    let src_squad = read_attrs(slot_id).map(|a| a.squad).unwrap_or_default();
    if dest_squad.is_empty() || dest_squad == src_squad {
        return false;
    }
    refile_slot(slot_id.to_string(), dest_squad)
}
