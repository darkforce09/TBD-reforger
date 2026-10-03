use std::collections::BTreeSet;

use super::INSTANCE_KINDS;

/// The enums schema of the map objects, from the checkout this crate lies in.
const ENUMS_SCHEMA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/definitions/map-object-enums.schema.json"
);

/// The guard that catches a kind added to the schema and not to this constant.
///
/// Adding `vehicle` to the enums schema and to the classify rules leaves every census
/// kind list stayed at eight. Nothing compared them, so the only signal would have been a
/// panic during an export nobody could run. This compares them.
#[test]
fn instance_kinds_match_enums_schema() {
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(ENUMS_SCHEMA).expect("enums schema"))
            .unwrap();
    let names = |k: &str| -> BTreeSet<String> {
        doc["$defs"][k]["enum"]
            .as_array()
            .unwrap_or_else(|| panic!("$defs.{k}.enum missing"))
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    };
    let all = names("kind");
    let regions = names("regionKind");
    assert!(!all.is_empty() && !regions.is_empty(), "empty enums");
    let expected: BTreeSet<String> = all.difference(&regions).cloned().collect();
    let actual: BTreeSet<String> = INSTANCE_KINDS.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(
        actual, expected,
        "INSTANCE_KINDS drifted from map-object-enums.schema.json $defs.kind \
         minus $defs.regionKind — a census bucket is missing or spurious, which panics \
         build-world-objects at `by_kind.get_mut(kind).expect(\"kind bucket\")`"
    );
}

/// `road` last and `vehicle` right after `water`: the array is the emitted `byKind` key order,
/// so a reorder silently rewrites the committed census on the next rebuild.
#[test]
fn instance_kinds_keep_the_emitted_by_kind_order() {
    assert_eq!(INSTANCE_KINDS.last(), Some(&"road"));
    let water = INSTANCE_KINDS.iter().position(|k| *k == "water");
    let vehicle = INSTANCE_KINDS.iter().position(|k| *k == "vehicle");
    assert_eq!(vehicle, water.map(|i| i + 1), "vehicle must follow water");
}
