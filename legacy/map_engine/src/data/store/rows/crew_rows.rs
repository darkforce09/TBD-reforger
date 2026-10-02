//! Role: crew rows.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::HashMap;
use super::Map;
use super::MapRef;
use super::Out;
use super::ReadTxn;
use super::TransactionMut;

/// Read crew map using the supplied domain data.
pub(super) fn read_crew_map<T: ReadTxn>(txn: &T, vehicle: &MapRef) -> HashMap<String, Any> {
    match vehicle.get(txn, "crew") {
        Some(Out::Any(Any::Map(m))) => (*m).clone(),
        Some(Out::YMap(crew)) => crew
            .iter(txn)
            .filter_map(|(seat, occ)| match occ {
                Out::Any(a) => Some((seat.to_string(), a)),
                _ => None,
            })
            .collect(),
        _ => HashMap::new(),
    }
}

/// Write crew map using the supplied domain data.
pub(super) fn write_crew_map(
    txn: &mut TransactionMut,
    vehicle: &MapRef,
    crew: HashMap<String, Any>,
) {
    if crew.is_empty() {
        vehicle.remove(txn, "crew");
    } else {
        vehicle.insert(txn, "crew", Any::Map(Arc::new(crew)));
    }
}
