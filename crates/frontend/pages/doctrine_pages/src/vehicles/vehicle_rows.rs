//! The page's vehicle list after a write: where a saved row goes, which row leaves, and which row
//! the dossier shows.
//!
//! **Role:** keeps the one fetched list current in place — a saved row replaces its stored copy or
//! joins the list, a deleted row leaves it — and picks the row the dossier shows.
//! **Position:** called by the vehicle index route when a write lands and whenever the detail pane
//! renders.
//! **Signals & state:** none; pure functions over the list.
//! **Invariants:** the rows the backend ordered keep their relative order. A saved row first
//! leaves the list under its id, then takes its place before the first row whose name, then id,
//! sorts after its own, compared as text. A deleted id leaves the list. The row shown is the
//! selected id's row, or the first row when the id names none.

#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::vehicles::Vehicle;

/// Puts `saved` into `rows`: its stored copy leaves, and the saved row takes its place by name,
/// then id.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn place_saved_vehicle(rows: &mut Vec<Vehicle>, saved: Vehicle) {
    rows.retain(|row| row.id != saved.id);
    let key = (saved.name.as_str(), saved.id.as_str());
    let at = rows
        .iter()
        .position(|row| (row.name.as_str(), row.id.as_str()) > key)
        .unwrap_or(rows.len());
    rows.insert(at, saved);
}

/// Takes vehicle `id` out of `rows`.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn remove_vehicle(rows: &mut Vec<Vehicle>, id: &str) {
    rows.retain(|row| row.id != id);
}

/// The row the dossier shows: the one `selected_id` names, else the first.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn shown_vehicle<'a>(rows: &'a [Vehicle], selected_id: &str) -> Option<&'a Vehicle> {
    rows.iter()
        .find(|row| row.id == selected_id)
        .or_else(|| rows.first())
}

/// The id to select after a removal: `selected_id` while it still names a row, else the first
/// row's id, else empty.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn selection_after_removal(rows: &[Vehicle], selected_id: &str) -> String {
    shown_vehicle(rows, selected_id)
        .map(|row| row.id.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "tests/vehicle_rows.rs"]
mod tests;
