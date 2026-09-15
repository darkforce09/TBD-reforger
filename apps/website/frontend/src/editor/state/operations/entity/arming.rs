//! Role: arming.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Palette leaf `pointerdown` → arm a place. Consumed by [`place_at`] on a canvas release, or dropped by [`cancel_pending`] on a release over chrome.
pub fn begin_place(payload: PlacePayload) {
    arm(Pending::Character(payload));
}

/// Begin place vehicle using the supplied domain data.
pub fn begin_place_vehicle(payload: PlacePayload) {
    arm(Pending::Vehicle(payload));
}

/// Begin place object using the supplied domain data.
pub fn begin_place_object(payload: PlacePayload) {
    arm(Pending::Object(payload));
}

/// Begin place composition using the supplied domain data.
pub fn begin_place_composition(composition_id: String) {
    arm(Pending::Composition(composition_id));
}

/// Armed composition id using the supplied domain data.
#[must_use]
pub fn armed_composition_id() -> Option<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let p = ctx.pending.borrow();
        match &*p {
            Some(Pending::Composition(id)) => Some(id.clone()),
            _ => None,
        }
    })
}

/// Arm a place. Objects mode only accepts [`Pending::Object`]; side modes reject Object so a leftover Objects arm cannot commit after the chip switches away.
pub(in crate::editor::state::operations) fn arm(pending: Pending) {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let objects = ctx.objects_mode.get_untracked();
            let ok = match &pending {
                Pending::Object(_) => objects,
                Pending::Character(_) | Pending::Vehicle(_) => !objects,

                Pending::Composition(_) => true,

                Pending::Marker(_) => true,

                Pending::Zone(_) => false,
            };
            if !ok {
                *ctx.pending.borrow_mut() = None;
                return;
            }
            *ctx.pending.borrow_mut() = Some(pending);
        }
    });

    bump_doc_tick();
}

/// Selection len using the supplied domain data.
#[must_use]
pub fn selection_len() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .map_or(0, |ctx| ctx.selection.borrow().len())
    })
}

/// Has pending using the supplied domain data.
#[must_use]
pub fn has_pending() -> bool {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .is_some_and(|ctx| ctx.pending.borrow().is_some())
    })
}

/// Drop the armed place (release over chrome, or pointercancel).
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

/// Mint an unused slot id. The counter keeps this O(1) amortized, but uniqueness is **proven** against the live doc rather than assumed: undo frees ids, and an IDB restore can bring back a document that already used `n0`.
pub(in crate::editor::state::operations) fn mint_id(ctx: &OpsCtx, core: &MissionDocCore) -> String {
    website_mission_core::doc::operations::entity::mint_id(core, &ctx.next_id)
}

/// Ensure layer using the supplied domain data.
pub(in crate::editor::state::operations) fn ensure_layer(
    ctx: &OpsCtx,
    core: &MissionDocCore,
) -> String {
    let rows = layer_rows(core);
    if let Some(active) = ctx.active_layer.get_untracked() {
        if rows.iter().any(|l| l.id == active) {
            return active;
        }
        ctx.active_layer.set(None);
    }
    if let Some(first) = rows.first() {
        return first.id.clone();
    }
    core.add_editor_layer(DEFAULT_LAYER_ID, DEFAULT_LAYER_NAME, None);
    DEFAULT_LAYER_ID.to_string()
}

/// Debug seed slots using the supplied domain data.
pub fn debug_seed_slots(n: u32) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };
        let layer_id = ensure_layer(ctx, core);
        for _ in 0..n {
            let id = mint_id(ctx, core);
            let _ = place_character_under_side(
                core, "BLUFOR", &id, &layer_id, "Rifleman", None, None, 0.0, 0.0, 0.0, 0.0,
            );
        }
    });
    mission_history::after_local_edit();
}
