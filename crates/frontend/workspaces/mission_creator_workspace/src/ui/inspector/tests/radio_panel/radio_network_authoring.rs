//! Tests the radio panel subject.

use super::*;
use serde_json::json;

fn cmd() -> Value {
    json!({
        "id": "net:blufor_cmd",
        "label": "Command",
        "freqMHz": 30.0,
        "faction": "blufor",
        "range": "long"
    })
}

#[test]
fn add_appends_a_unique_id_and_frequency() {
    let next = add_net(&[cmd()]).expect("add");
    assert_eq!(next.len(), 2);
    assert_eq!(next[1]["id"], "net:ch_2");
    assert_eq!(next[1]["freqMHz"], 30.5);
    validate(&plan_from_nets(&next).expect("plan"))
        .expect("the panel must not author a refused block");
}

#[test]
fn remove_drops_one_row_and_clearing_the_last_writes_null() {
    let next = remove_net(&[cmd()], 0);
    assert!(next.is_empty());
    let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
    assert_eq!(cleared, json!({"radioPlan": null}));
}

#[test]
fn reset_to_derived_writes_an_explicit_null_patch() {
    let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
    assert_eq!(cleared, json!({"radioPlan": null}));
    let set: Value =
        serde_json::from_str(&env_patch(plan_from_nets(&[cmd()]).as_ref())).expect("json");
    assert_eq!(set["radioPlan"]["nets"][0]["id"], "net:blufor_cmd");
}

#[test]
fn a_full_authoring_pass_produces_a_block_the_compile_accepts() {
    let mut rows = add_net(&[]).expect("first");
    rows = with_field(&rows, 0, "label", "Command").expect("label");
    rows = with_field(&rows, 0, "faction", "blufor").expect("faction");
    rows = with_field(&rows, 0, "range", "long").expect("range");
    rows = add_net(&rows).expect("second");
    rows = with_field(&rows, 1, "label", "Alpha").expect("alpha");
    rows = with_field(&rows, 1, "faction", "blufor").expect("fac2");
    validate(&plan_from_nets(&rows).expect("plan")).expect("valid");
}
