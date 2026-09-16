//! Role: placement.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

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

    let mut recent_stamp: Option<(String, String)> = None;

    let mut placed_vehicle = false;
    let placed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };

        let Some(pending) = ctx.pending.borrow_mut().take() else {
            return false;
        };

        let select = {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            let side = ctx.active_side.get_untracked();
            let id = mint_id(ctx, core);
            match pending {
                Pending::Zone(_) => return false,
                Pending::Vehicle(payload) => {
                    let with_crew = place_with_crew() && !alt_empty;
                    if !place_vehicle_in_core(core, &side, &id, &payload.asset_id, x, y, with_crew)
                    {
                        return false;
                    }
                    placed_vehicle = true;
                    None
                }
                Pending::Object(payload) => {
                    if !place_object_in_core(core, &side, &id, &payload, x, y) {
                        return false;
                    }
                    None
                }

                Pending::Marker(icon) => {
                    let _ = id;

                    let faction_id = side_faction_id(&side);
                    let marker_id = mint_marker_id(&marker_rows_of(core));

                    let (mx, mz) = (x, y);

                    core.set_faction_briefing_marker(&faction_id, &marker_id, mx, mz, &icon, "");

                    None
                }
                Pending::Composition(comp_id) => {
                    let _ = id;
                    let Some((slot_ids, title)) =
                        website_map_engine::data::store::operations::entity::place_saved_composition(
                            core,
                            &comp_id,
                            &side,
                            x,
                            y,
                            &ctx.next_id,
                            |core| ensure_layer(ctx, core),
                        )
                    else {
                        return false;
                    };
                    recent_stamp = Some((comp_id.clone(), title));
                    *ctx.selection.borrow_mut() = slot_ids;
                    None
                }
                Pending::Character(payload) => {
                    let layer_id = ensure_layer(ctx, core);
                    let asset_id = payload.asset_id.clone();
                    if place_character_under_side(
                        core,
                        &side,
                        &id,
                        &layer_id,
                        &payload.role,
                        None,
                        Some(payload.asset_id),
                        x,
                        y,
                        0.0,
                        0.0,
                    )
                    .is_err()
                    {
                        return false;
                    }

                    seed_cargo_in_core(core, &id, &asset_id, None);
                    Some(id)
                }
            }
        };
        if let Some(id) = select {
            *ctx.selection.borrow_mut() = vec![id];
        }
        true
    });
    if placed {
        mission_history::after_local_edit();

        if placed_vehicle {
            rebind_vehicle_lane_after_place();
        }

        if let Some((asset_id, label)) = recent_stamp {
            crate::editor::panels::dock_right::record_placed(asset_id, label);
        }
    }
    placed
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
