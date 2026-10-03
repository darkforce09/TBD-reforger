//! The prefab a JSON number names.
//!
//! **Role:** turns the numbers the served JSON carries into [`PrefabId`]s: a catalogue row's
//! `prefabId` ([`catalogue_prefab_id`]) and a chunk instance's `pid`
//! ([`prefab_id_from_f64`]), the join key of every chunk row against the prefab map.
//! **Position:** under `prefab_catalog`; read by [`crate::prefab_rows`] and
//! [`crate::footprint_lookups`] for the catalogue, and by `world_chunks`' chunk decoder and the
//! developer tools' binary chunk emitter for the chunk rows.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a number names a prefab only when it is a whole number in `0..=u32::MAX`; a
//! chunk `pid` must moreover be the exact `f64` of that `u32` (sign bit clear), so a fractional,
//! negative (`-0.0` included), non-finite or out-of-range `pid` joins no prefab, never a cast.

use world_file_formats::ids::PrefabId;

/// A catalogue's numeric `prefabId` → its [`PrefabId`], or `None` when the number is fractional,
/// negative, non-finite or above `u32::MAX` (`-0` reads as `0`).
pub(crate) fn catalogue_prefab_id(value: f64) -> Option<PrefabId> {
    if value.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&value) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(PrefabId::new(value as u32))
    } else {
        None
    }
}

/// The prefab a chunk instance's numeric `pid` names: `Some` exactly when `pid` is the `f64` of a
/// `u32` (finite, whole, in `0..=u32::MAX`, its sign bit clear), so a fractional, negative
/// (`-0.0` included) or out-of-range `pid` names no prefab and its row stays unclassified.
#[must_use]
pub fn prefab_id_from_f64(pid: f64) -> Option<PrefabId> {
    if pid.is_sign_positive() {
        catalogue_prefab_id(pid)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "tests/numeric_prefab_ids_tests.rs"]
mod tests;
