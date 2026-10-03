//! The per-bucket counts of a terrain's object census.
//!
//! **Role:** [`count_prefab`], which adds one prefab type and its placed instances to a census
//! bucket (`byKind`, `byBuildingClass`, `bySpeciesClass`).
//! **Position:** the catalogue builder (`chunk_partitioner`) and the reclassification
//! (`reclassify`) call it while they assemble a type inventory.
//! **Signals & state:** none; mutates the bucket it is given.
//! **Invariants:** a bucket keeps its key order (the emitted order), and a bucket's counts only grow.

use serde_json::{Map, Value, json};

/// Adds one prefab type and its `instances` placed instances to a census `bucket`, keeping the
/// bucket's key order; a count the bucket does not hold yet starts at zero.
pub(crate) fn count_prefab(bucket: &mut Map<String, Value>, instances: u64) {
    let prefab_types = bucket
        .get("prefabTypes")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let placed = bucket.get("instances").and_then(Value::as_u64).unwrap_or(0);
    bucket.insert("prefabTypes".to_string(), json!(prefab_types + 1));
    bucket.insert("instances".to_string(), json!(placed + instances));
}
