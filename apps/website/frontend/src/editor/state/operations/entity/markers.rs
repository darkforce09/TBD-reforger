//! Role: markers.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Marker rows using the supplied domain data.
#[must_use]
pub fn marker_rows() -> Vec<MarkerRow> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            Some(marker_rows_of(d.as_ref()?))
        })
        .unwrap_or_default()
}

/// How many markers the document carries (the palette header readout).
#[must_use]
pub fn marker_count() -> usize {
    marker_rows().len()
}

/// Refuses an alias outside the closed `$defs/marker.icon` enum, so a bad vocabulary cannot even be armed, let alone stored.
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

/// ATTR-FIELD-MRK-TYPE — re-icon an existing marker, keeping its position and caption.
#[must_use]
pub fn set_marker_icon(faction_id: &str, marker_id: &str, icon: &str) -> bool {
    if !crate::editor::panels::dock_right::marker_icon_is_authorable(icon) {
        return false;
    }
    upsert_marker_field(faction_id, marker_id, |row| row.icon = icon.to_string())
}

/// ATTR-FIELD-MRK-TEXT — re-caption an existing marker. The label is stored VERBATIM; the mod caps it at render time and the emitter applies that cap when it compiles, so capping here would destroy the authored value in the one place the author could still see and fix it.
#[must_use]
pub fn set_marker_label(faction_id: &str, marker_id: &str, label: &str) -> bool {
    upsert_marker_field(faction_id, marker_id, |row| row.label = label.to_string())
}

/// ATTR-FIELD-MRK-POSITION — move a marker to `(x, z)` world metres. Non-finite input is refused rather than stored: `$defs/marker` types both as `number`, and a NaN would serialise as JSON `null` and fail the validator at save time, far from the box that produced it.
#[must_use]
pub fn set_marker_position(faction_id: &str, marker_id: &str, x: f64, z: f64) -> bool {
    if !x.is_finite() || !z.is_finite() {
        return false;
    }
    upsert_marker_field(faction_id, marker_id, |row| {
        row.x = x;
        row.z = z;
    })
}

/// Delete one marker. Its siblings and the briefing prose beside them are untouched (`remove_faction_briefing_marker` reads the briefing and writes it back whole).
#[must_use]
pub fn remove_marker(faction_id: &str, marker_id: &str) -> bool {
    let exists = marker_rows()
        .iter()
        .any(|r| r.faction_id == faction_id && r.id == marker_id);
    if !exists {
        return false;
    }
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        if let Some(ctx) = guard.as_ref() {
            let d = ctx.doc.borrow();
            if let Some(core) = d.as_ref() {
                core.remove_faction_briefing_marker(faction_id, marker_id);
            }
        }
    });
    mission_history::after_local_edit();
    true
}

/// Upsert marker field using the supplied domain data.
pub(in crate::editor::state::operations) fn upsert_marker_field(
    faction_id: &str,
    marker_id: &str,
    edit: impl FnOnce(&mut MarkerRow),
) -> bool {
    let Some(mut row) = marker_rows()
        .into_iter()
        .find(|r| r.faction_id == faction_id && r.id == marker_id)
    else {
        return false;
    };
    edit(&mut row);
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        if let Some(ctx) = guard.as_ref() {
            let d = ctx.doc.borrow();
            if let Some(core) = d.as_ref() {
                core.set_faction_briefing_marker(
                    faction_id, marker_id, row.x, row.z, &row.icon, &row.label,
                );
            }
        }
    });
    mission_history::after_local_edit();
    true
}
