use super::{INSTANCE_KINDS, instance_kinds_lockstep_failures, read_json, repo_root};

fn enums() -> serde_json::Value {
    read_json(&developer_tools::repository_layout::definition_path(
        &repo_root().expect("repo root"),
        "map-object-enums.schema.json",
    ))
    .expect("enums schema")
}

/// `INSTANCE_KINDS` — the copy that feeds I1 — must name exactly the kinds the enums schema
/// declares, so a kind added to one and not the other cannot reach a shipped catalogue.
#[test]
fn instance_kinds_match_the_enums_schema_and_the_crate_array() {
    let f = instance_kinds_lockstep_failures(&enums());
    assert!(f.is_empty(), "INSTANCE_KINDS is not in lockstep:\n  {f:#?}");
}

/// Non-vacuity: prove the comparison above actually compares. A schema whose `kind` enum has
/// lost a member this array still carries MUST fail — otherwise the assertion is decorative.
#[test]
fn lockstep_reds_when_the_enum_and_the_array_disagree() {
    let mut doc = enums();
    let kinds = doc["$defs"]["kind"]["enum"]
        .as_array()
        .expect("kind enum")
        .clone();
    let dropped: Vec<serde_json::Value> = kinds
        .into_iter()
        .filter(|v| v.as_str() != Some("vehicle"))
        .collect();
    doc["$defs"]["kind"]["enum"] = serde_json::Value::Array(dropped);
    let f = instance_kinds_lockstep_failures(&doc);
    assert!(
        f.iter()
            .any(|m| m.contains("spurious") && m.contains("vehicle")),
        "removing `vehicle` from the kind enum must red the lockstep check; got {f:#?}"
    );
}

/// Same non-vacuity proof for the missing-`$defs` branch: an unreadable enum set is a FAIL,
/// never a silent pass.
#[test]
fn lockstep_reds_when_the_enums_are_unreadable() {
    let f = instance_kinds_lockstep_failures(&serde_json::json!({}));
    assert!(
        f.iter().any(|m| m.contains("refusing to report lockstep")),
        "an absent kind enum must fail closed; got {f:#?}"
    );
}

/// `road` last, `vehicle` after `water` — this array is the emitted `byKind` key order, so a
/// reorder silently rewrites the committed artifact on the next rebuild.
#[test]
fn instance_kinds_order_is_the_emitted_bykind_order() {
    assert_eq!(INSTANCE_KINDS.last(), Some(&"road"));
    let water = INSTANCE_KINDS.iter().position(|k| *k == "water");
    let vehicle = INSTANCE_KINDS.iter().position(|k| *k == "vehicle");
    assert_eq!(vehicle, water.map(|i| i + 1), "vehicle must follow water");
}
