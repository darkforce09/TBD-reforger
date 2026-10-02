//! Role: the palette gestures that arm a place — a character, a vehicle, an object, a saved
//! composition or a briefing marker — and the readings a panel renders its "click the map" hint
//! from.
//! Position: `editor/bridge/host_state/armed_placement` in the frontend editor shell.
//! Signals & state: the armed value on the installed editor context.
//! Invariants: a marker icon outside the closed schema enum cannot even be armed, let alone stored,
//! so a bad vocabulary is refused at the palette rather than at save time.

use super::{arm, Pending};
use crate::v2::apps::editor::arsenal::asset_catalog::PlacePayload;
use crate::v2::apps::editor::bridge::host_state::editor_context::EDITOR_CONTEXT;

/// A palette leaf `pointerdown` arms a character place, consumed by the next canvas release.
pub fn begin_place(payload: PlacePayload) {
    arm(Pending::Character(payload));
}

/// Arm a vehicle place.
pub fn begin_place_vehicle(payload: PlacePayload) {
    arm(Pending::Vehicle(payload));
}

/// Arm a world-object place.
pub fn begin_place_object(payload: PlacePayload) {
    arm(Pending::Object(payload));
}

/// Arm a saved-composition stamp, by library id.
pub fn begin_place_composition(composition_id: String) {
    arm(Pending::Composition(composition_id));
}

/// The armed composition's library id, or `None` — the library row's armed highlight.
#[must_use]
pub fn armed_composition_id() -> Option<String> {
    EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let p = ctx.pending.borrow();
        match &*p {
            Some(Pending::Composition(id)) => Some(id.clone()),
            _ => None,
        }
    })
}

/// A palette icon press arms a marker place. Refuses an alias outside the closed `$defs/marker.icon`
/// enum.
pub fn begin_place_marker(icon: String) {
    if !crate::v2::apps::editor::ui::docks::dock_right::marker_icon_is_authorable(&icon) {
        return;
    }
    arm(Pending::Marker(icon));
}

/// The armed marker icon, or `None`. Backs the panel's "click the map to drop it" hint.
#[must_use]
pub fn armed_marker_icon() -> Option<String> {
    EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let p = ctx.pending.borrow();
        match &*p {
            Some(Pending::Marker(icon)) => Some(icon.clone()),
            _ => None,
        }
    })
}
