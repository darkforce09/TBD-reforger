//! Role: arming.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;
use crate::editor::panels::outliner::{DEFAULT_LAYER_ID, DEFAULT_LAYER_NAME};
use website_map_engine::data::store::operations::entity::ArmedPlacementKind;

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

/// Arm a place. Objects mode only accepts [`Pending::Object`]; side modes reject Object so a leftover Objects arm cannot commit after the chip switches away.
pub(in crate::editor::state::operations) fn arm(pending: Pending) {
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
    website_map_engine::data::store::operations::entity::mint_id(core, &ctx.next_id)
}

/// Ensure layer using the supplied domain data.
pub(in crate::editor::state::operations) fn ensure_layer(
    ctx: &OpsCtx,
    core: &MissionDocCore,
) -> String {
    let ensured = website_map_engine::data::store::operations::entity::ensure_layer(
        core,
        ctx.active_layer.get_untracked(),
        DEFAULT_LAYER_ID,
        DEFAULT_LAYER_NAME,
    );
    if ensured.active_layer_was_stale {
        ctx.active_layer.set(None);
    }
    ensured.layer_id
}

/// Resolve the folder a new entity is filed under without a context in hand: the active folder
/// when one is set and still live, otherwise the default folder, minted under the LOCAL origin so
/// the mint is part of the same undoable act as the place it serves. This is the form the engine's
/// hosted commands take, which is why the folder id crosses the wall as an answer rather than the
/// engine reaching for the dock's active-folder signal itself.
pub fn ensure_active_layer(core: &MissionDocCore) -> String {
    OPS_CTX
        .with(|c| c.borrow().as_ref().map(|ctx| ensure_layer(ctx, core)))
        .unwrap_or_else(|| DEFAULT_LAYER_ID.to_string())
}

/// Palette icon press → arm a marker place, consumed by the next canvas release. Refuses an alias
/// outside the closed `$defs/marker.icon` enum, so a bad vocabulary cannot even be armed, let alone
/// stored.
pub fn begin_place_marker(icon: String) {
    if !crate::editor::panels::dock_right::marker_icon_is_authorable(&icon) {
        return;
    }
    arm(Pending::Marker(icon));
}

/// The armed marker icon, or `None`. Backs the panel's "click the map to drop it" hint.
#[must_use]
pub fn armed_marker_icon() -> Option<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let p = ctx.pending.borrow();
        match &*p {
            Some(Pending::Marker(icon)) => Some(icon.clone()),
            _ => None,
        }
    })
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
        website_map_engine::data::store::operations::entity::seed_debug_slots(
            core,
            &ctx.next_id,
            &layer_id,
            n,
        );
    });
    mission_history::after_local_edit();
}
