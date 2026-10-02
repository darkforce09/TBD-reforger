//! Role: the briefing markers a faction pins on the map.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document comes from the host.
//! Invariants: a marker is edited by reading its whole row, changing one field and writing the row
//! back, because the document stores a marker as one object — a partial write would blank the
//! fields it did not name. The icon vocabulary is the HOST's closed schema list and crosses as a
//! closure, so an alias the schema does not declare can never be stored. A caption is stored
//! VERBATIM: the mod caps it at render time and the compiler caps it at emit, and capping here
//! would destroy the authored value in the one place its author can still see and fix it.

use crate::data::store::operations::entity as entity_ops;
use crate::editing::history::after_local_edit;
use crate::editing::host::with_doc;

/// One briefing marker, as the palette's row needs it.
pub use crate::data::store::operations::entity::MarkerRow;

/// Every briefing marker the document carries, across all factions.
#[must_use]
pub fn marker_rows() -> Vec<MarkerRow> {
    with_doc(entity_ops::marker_rows_of).unwrap_or_default()
}

/// How many markers the document carries — the palette header's readout.
#[must_use]
pub fn marker_count() -> usize {
    marker_rows().len()
}

/// Re-icon a marker, keeping its position and caption. `icon_is_authorable` is the host's closed
/// alias list; an alias outside it is refused rather than stored.
#[must_use]
pub fn set_marker_icon(
    faction_id: &str,
    marker_id: &str,
    icon: &str,
    icon_is_authorable: impl Fn(&str) -> bool,
) -> bool {
    if !icon_is_authorable(icon) {
        return false;
    }
    upsert_marker_field(faction_id, marker_id, |row| row.icon = icon.to_string())
}

/// Re-caption a marker.
#[must_use]
pub fn set_marker_label(faction_id: &str, marker_id: &str, label: &str) -> bool {
    upsert_marker_field(faction_id, marker_id, |row| row.label = label.to_string())
}

/// Move a marker to `(x, z)` world metres. Non-finite input is refused rather than stored: the
/// schema types both as `number`, so a NaN would serialise as JSON `null` and fail validation at
/// save time, far from the control that produced it.
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

/// Delete one marker. Its siblings and the briefing prose beside them are untouched.
#[must_use]
pub fn remove_marker(faction_id: &str, marker_id: &str) -> bool {
    let exists = marker_rows()
        .iter()
        .any(|r| r.faction_id == faction_id && r.id == marker_id);
    if !exists {
        return false;
    }
    with_doc(|core| core.remove_faction_briefing_marker(faction_id, marker_id));
    after_local_edit();
    true
}

/// Read one marker's row, apply `edit` to it, and write the whole row back. `false` when no such
/// marker exists, which is what makes every field writer above safe against a row deleted out from
/// under an open editor.
fn upsert_marker_field(
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
    with_doc(|core| {
        core.set_faction_briefing_marker(
            faction_id, marker_id, row.x, row.z, &row.icon, &row.label,
        );
    });
    after_local_edit();
    true
}
