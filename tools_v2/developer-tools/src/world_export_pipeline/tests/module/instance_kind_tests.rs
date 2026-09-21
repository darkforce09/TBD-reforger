use std::collections::BTreeSet;

use super::INSTANCE_KINDS;
use crate::browser_testing::server::repo_root;
use crate::repository_layout::definition_path;

/// The guard that catches a kind added to the schema and not to this constant.
///
/// Adding `vehicle` to the enums schema and to the classify rules leaves every census
/// kind list stayed at eight. Nothing compared them, so the only signal would have been a
/// panic during an export nobody could run. This compares them.
#[test]
fn instance_kinds_match_enums_schema() {
    let p = definition_path(&repo_root(), "map-object-enums.schema.json");
    let doc: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&p).expect("enums schema")).unwrap();
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
