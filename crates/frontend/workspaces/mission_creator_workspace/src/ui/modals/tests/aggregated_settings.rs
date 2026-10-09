use super::{
    DiffState, MISSION_SCHEMA_JSON, MISSION_SETTING_POINTERS, NOT_A_SCHEMA_KEY, SettingDefault,
    SettingRow, aggregate_settings, fmt_setting_default, fmt_setting_value,
    mission_setting_pointer, values_agree,
};
use serde_json::json;

fn schema() -> serde_json::Value {
    serde_json::from_str(MISSION_SCHEMA_JSON).expect(
        "T-688: the embedded mission.schema.json must parse — it is the only source of \
                 every default this view reports",
    )
}

fn zone_rule_props() -> serde_json::Map<String, serde_json::Value> {
    schema()["$defs"]["zoneRules"]["properties"]
        .as_object()
        .expect("T-688: $defs/zoneRules/properties")
        .clone()
}

/// **THE pin.** The view's default for every `$defs/zoneRules` key must be exactly what the
/// schema declares there — read here a second time, independently, straight out of the committed
/// bytes.
///
/// This is the test the ticket asks for: it fails when the schema and the view disagree, which is
/// the only observable symptom a second source of truth ever has. A hand-written table of
/// defaults in Rust passes on the day it is written and goes red the first time the schema moves;
/// a typo goes red immediately. Perturbation this catches: replacing the schema read with a
/// literal for any single key.
#[test]
fn the_view_and_the_schema_agree_key_for_key() {
    let schema = schema();
    let props = zone_rule_props();
    assert!(
        props.len() >= 16,
        "T-688: $defs/zoneRules is the closed 16+ key vocabulary (T-241/T-685); found {}",
        props.len()
    );

    // A document authoring EVERY rule key on one zone. The authored values are deliberately
    // nonsense — what is under test is where the DEFAULT came from, not the value beside it.
    let mut rules = serde_json::Map::new();
    for key in props.keys() {
        rules.insert(key.clone(), json!("__authored_probe__"));
    }
    let doc = json!({
        "zonesById": {
            "zone-a": { "type": "objective_capture", "label": "Hilltop", "rules": rules }
        }
    });

    let rows = aggregate_settings(&doc);
    assert_eq!(
        rows.len(),
        props.len(),
        "T-688: every authored rule key must produce exactly one row — an aggregation that can \
         drop a setting is the defect this view exists to remove"
    );

    let mut with_default = 0usize;
    let mut without_default = 0usize;
    for (key, declared) in &props {
        let row = rows
            .iter()
            .find(|r| r.key == *key)
            .unwrap_or_else(|| panic!("T-688: no row for authored rule key `{key}`"));
        // One-hop `$ref`, exactly as the schema declares it (`targetAlias` → `$defs/alias`).
        let resolved = declared
            .get("$ref")
            .and_then(serde_json::Value::as_str)
            .and_then(|r| schema.pointer(r.trim_start_matches('#')))
            .unwrap_or(declared);
        let want = declared.get("default").or_else(|| resolved.get("default"));
        match (&row.default, want) {
            (SettingDefault::Schema { value, pointer }, Some(expected)) => {
                assert_eq!(
                    value, expected,
                    "T-688: the view and the schema DISAGREE about `{key}`'s default — the view \
                     says {value}, mission.schema.json says {expected}"
                );
                assert!(
                    pointer.ends_with(key.as_str()),
                    "T-688: `{key}`'s default must name the schema location it was read from, \
                     got {pointer:?}"
                );
                with_default += 1;
            }
            (SettingDefault::Declared { .. }, None) => without_default += 1,
            (got, want) => panic!(
                "T-688: `{key}` — the view reports {got:?} but mission.schema.json declares \
                 default={want:?}"
            ),
        }
    }
    assert!(
        with_default >= 11,
        "T-688: $defs/zoneRules declares defaults on at least 11 keys; the view found \
         {with_default}"
    );
    assert!(
        without_default > 0,
        "T-688: some rule keys declare no default (holdSeconds, points, …) and the view must \
         report them as such rather than inventing one"
    );
}

/// Every schema location this view declares must actually resolve. A pointer that stopped
/// resolving would silently downgrade a real wire key to "editor-local", which reads as "nothing
/// to compare" — a lie by omission rather than by value.
#[test]
fn every_declared_pointer_resolves() {
    let schema = schema();
    for (key, pointer) in MISSION_SETTING_POINTERS {
        assert!(
            schema.pointer(pointer.trim_start_matches('#')).is_some(),
            "T-688: `{key}`'s declared location {pointer} no longer resolves in \
             mission.schema.json"
        );
        assert_eq!(mission_setting_pointer(key), Some(*pointer));
    }
    // The zone-rule pointers are formatted, so one representative proves the shape.
    assert!(
        schema
            .pointer("/$defs/zoneRules/properties/graceSeconds")
            .is_some()
    );
}

/// **The walk is over the DOCUMENT.** A key this file has never heard of still gets a row — as
/// `NotInSchema`, never skipped. That is what stops a future `author_env` key, or a rule the
/// schema later drops, from vanishing out of "every setting in this mission".
///
/// Perturbation this catches: iterating a key table instead of the document.
#[test]
fn the_aggregation_walks_the_document_and_omits_nothing() {
    let doc = json!({
        "meta": { "environment": {
            "showGrid": true,
            "hillshadeOpacity": 0.4,
            "aKeyNobodyHasWrittenYet": 7
        } },
        "zonesById": { "z1": { "type": "boundary", "rules": { "notARuleAnyMore": 3 } } }
    });
    let rows = aggregate_settings(&doc);
    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    for key in [
        "showGrid",
        "hillshadeOpacity",
        "aKeyNobodyHasWrittenYet",
        "notARuleAnyMore",
    ] {
        assert!(
            keys.contains(&key),
            "T-688: `{key}` is authored in the document and must appear; got {keys:?}"
        );
        let row = rows.iter().find(|r| r.key == key).expect("row");
        assert_eq!(
            row.default,
            SettingDefault::NotInSchema,
            "T-688: `{key}` is not declared in mission.schema.json — say so, do not guess"
        );
        assert_eq!(fmt_setting_default(&row.default), NOT_A_SCHEMA_KEY);
    }
}

/// The diff-from-default filter keeps everything it cannot PROVE is unchanged. Hiding a row whose
/// schema declares no default would be the view asserting a fact nobody has — the same defect as
/// an invented default, told by omission.
#[test]
fn the_diff_filter_keeps_what_it_cannot_prove() {
    let doc = json!({
        "meta": { "environment": { "timeLimitSeconds": 900 } },
        "zonesById": { "z1": { "type": "objective_capture", "rules": {
            // `penalty`'s schema default is "warn"; `contestable`'s is true.
            "penalty": "warn",
            "contestable": false
        } } }
    });
    let rows = aggregate_settings(&doc);
    let by = |k: &str| rows.iter().find(|r| r.key == k).expect("row").clone();

    assert_eq!(by("penalty").diff_state(), DiffState::Matches);
    assert_eq!(by("contestable").diff_state(), DiffState::Differs);
    assert_eq!(by("timeLimitSeconds").diff_state(), DiffState::Unknown);

    let kept: Vec<String> = rows
        .iter()
        .filter(|r| r.survives_diff_filter())
        .map(|r| r.key.clone())
        .collect();
    assert!(kept.contains(&"contestable".to_string()));
    assert!(
        kept.contains(&"timeLimitSeconds".to_string()),
        "T-688: a key with no declared default cannot be shown to be at its default, so the \
         filter must keep it"
    );
    assert!(
        !kept.contains(&"penalty".to_string()),
        "T-688: a row provably at its schema default is what the filter hides"
    );
}

/// A zone rule's owner is the ZONE, named and addressable. The `subject_id` is the document id a
/// click routes on; the label is what the row shows (name, else type, else id — never faceless).
#[test]
fn zone_rules_are_owned_by_their_zone() {
    let doc = json!({ "zonesById": {
        "z-named": { "type": "objective_capture", "label": "Hilltop",
                     "rules": { "captureSeconds": 90 } },
        "z-plain": { "type": "boundary", "rules": { "graceSeconds": 45 } }
    } });
    let rows = aggregate_settings(&doc);
    let named = rows
        .iter()
        .find(|r| r.key == "captureSeconds")
        .expect("row");
    assert_eq!(named.owner.subject_id(), Some("z-named"));
    assert!(named.owner.label().contains("Hilltop"));
    assert!(named.owner.label().contains("Zone"));

    let plain = rows.iter().find(|r| r.key == "graceSeconds").expect("row");
    assert_eq!(plain.owner.subject_id(), Some("z-plain"));
    assert!(
        plain.owner.label().contains("boundary"),
        "T-688: an unlabelled zone falls back to its type, got {:?}",
        plain.owner.label()
    );
    // A mission-level row names no entity, so it is not a click-through target.
    let doc = json!({ "meta": { "terrain": "everon" } });
    assert_eq!(aggregate_settings(&doc)[0].owner.subject_id(), None);
}

/// `120` (schema, integer) and `120.0` (authored through a number control) are the same setting.
/// Derived `Value` equality would call them different and paint an untouched row "changed" —
/// which is exactly the false diff this whole view must not produce.
#[test]
fn numeric_defaults_compare_across_int_and_float() {
    assert!(values_agree(&json!(120), &json!(120.0)));
    assert!(values_agree(&json!(0), &json!(-0.0)));
    assert!(!values_agree(&json!(120), &json!(121)));
    assert!(values_agree(&json!("warn"), &json!("warn")));
    assert!(!values_agree(&json!("warn"), &json!("kill")));
    assert!(!values_agree(&json!(true), &json!(1)));

    let doc = json!({ "zonesById": { "z1": { "type": "objective_capture",
        "rules": { "captureSeconds": 120.0 } } } });
    assert_eq!(
        aggregate_settings(&doc)[0].diff_state(),
        DiffState::Matches,
        "T-688: an authored 120.0 against a schema default of 120 is not a change"
    );
}

/// Row order is deterministic — the pointer table's order, then unlisted mission keys sorted,
/// then zones by id and rules by key. An author who scrolls to a row must find it in the same
/// place next time, and a test can only pin what does not shuffle.
#[test]
fn row_order_is_stable() {
    let doc = json!({
        "meta": { "terrain": "everon", "environment": {
            "jip": "always", "time": "06:00", "showGrid": true, "showHillshade": false
        } },
        "zonesById": {
            "z-b": { "type": "boundary", "rules": { "penalty": "kill", "graceSeconds": 10 } },
            "z-a": { "type": "spawn", "rules": { "warnEverySeconds": 2 } }
        }
    });
    let rows: Vec<(String, Option<String>)> = aggregate_settings(&doc)
        .into_iter()
        .map(|r| (r.key, r.owner.subject_id().map(ToString::to_string)))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("terrain".into(), None),
            ("time".into(), None),
            ("jip".into(), None),
            ("showGrid".into(), None),
            ("showHillshade".into(), None),
            ("warnEverySeconds".into(), Some("z-a".into())),
            ("graceSeconds".into(), Some("z-b".into())),
            ("penalty".into(), Some("z-b".into())),
        ]
    );
}

/// A `SettingRow` carries all four columns the ticket names — key, owning entity, authored value,
/// schema default — so no column can be quietly dropped from the type the view renders.
#[test]
fn a_row_carries_all_four_columns() {
    let doc = json!({ "zonesById": { "z1": { "type": "objective_capture", "label": "Hill",
        "rules": { "captureSeconds": 240 } } } });
    let row: SettingRow = aggregate_settings(&doc).remove(0);
    assert_eq!(row.key, "captureSeconds");
    assert_eq!(row.owner.label(), "Zone — Hill");
    assert_eq!(fmt_setting_value(&row.value), "240");
    assert_eq!(fmt_setting_default(&row.default), "120");
    assert_eq!(row.diff_state(), DiffState::Differs);
}
