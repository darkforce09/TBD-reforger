//! Role: Module boundary for doc/crdt/id_arrays.
//! Position: `doc/crdt/id_arrays` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use std::collections::HashSet;
#[cfg(test)]
use yrs::Map;
use yrs::{Any, Array, ArrayPrelim, MapRef, Out, ReadTxn, TransactionMut};
mod native_arrays;

/// Expose native arrays :: entity ids at this domain boundary.
pub use native_arrays::ENTITY_IDS;

/// Expose native arrays :: slot ids at this domain boundary.
pub use native_arrays::SLOT_IDS;

/// Expose native arrays :: append id at this domain boundary.
pub use native_arrays::append_id;

/// Expose native arrays :: insert empty native at this domain boundary.
pub use native_arrays::insert_empty_native;

/// Expose native arrays :: is native array at this domain boundary.
pub use native_arrays::is_native_array;

/// Expose native arrays :: migrate legacy id lists at this domain boundary.
pub use native_arrays::migrate_legacy_id_lists;

/// Expose native arrays :: move id at this domain boundary.
pub use native_arrays::move_id;

/// Expose native arrays :: read field at this domain boundary.
pub use native_arrays::read_field;

/// Expose native arrays :: read field ids at this domain boundary.
pub use native_arrays::read_field_ids;

/// Expose native arrays :: read id array at this domain boundary.
pub use native_arrays::read_id_array;

/// Expose native arrays :: read ids at this domain boundary.
pub use native_arrays::read_ids;

/// Expose native arrays :: replace native at this domain boundary.
pub use native_arrays::replace_native;

/// Expose native arrays :: retain ids at this domain boundary.
pub use native_arrays::retain_ids;

/// Expose native arrays :: retain in at this domain boundary.
pub use native_arrays::retain_in;
#[cfg(test)]
mod mission_doc_tests;
#[cfg(test)]
mod tests;
