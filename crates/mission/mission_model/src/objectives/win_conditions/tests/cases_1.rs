//! **Role:** Domain regression cases.
//! **Position:** `mission_model::objectives::win_conditions::tests::cases_1` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_vip_block_parses_with_its_param() {
    let got = parse(&vip()).expect("parses");
    assert_eq!(got.mode, "vip");
    assert_eq!(got.end_on, ["faction_eliminated"]);
    assert_eq!(
        got.params.vip_slot_id.as_ref().map(|id| id.as_str()),
        Some("s-12")
    );
    assert!(got.params.extraction_zone_id.is_none());
    assert!(got.params.timeout_minutes.is_none());
}

#[test]
fn a_missing_mode_param_is_refused_and_names_the_mode() {
    for (mode, key) in [
        ("extraction", "extractionZoneId"),
        ("vip", "vipSlotId"),
        ("timeout", "timeoutMinutes"),
    ] {
        let err = parse(&json!({"mode": mode, "endOn": ["time_limit"]}))
            .expect_err("must refuse a missing param");
        assert!(err.to_string().contains(key), "{err}");
        assert!(err.to_string().contains(mode), "{err}");
    }
}

#[test]
fn a_param_belonging_to_another_mode_is_refused() {
    let err = parse(&json!({
        "mode": "vip",
        "endOn": ["time_limit"],
        "vipSlotId": "s1",
        "timeoutMinutes": 30,
    }))
    .expect_err("timeoutMinutes does not belong to vip");
    assert!(err.to_string().contains("timeoutMinutes"), "{err}");
    assert!(err.to_string().contains("\"timeout\""), "{err}");
    assert!(err.to_string().contains("\"vip\""), "{err}");

    let err = parse(&json!({
        "mode": "extraction",
        "endOn": ["time_limit"],
        "extractionZoneId": "z1",
        "vipSlotId": "s1",
    }))
    .expect_err("vipSlotId does not belong to extraction");
    assert!(err.to_string().contains("vipSlotId"), "{err}");
}

#[test]
fn the_timeout_range_is_two_sided_and_inclusive() {
    let at =
        |m: i64| parse(&json!({"mode": "timeout", "endOn": ["time_limit"], "timeoutMinutes": m}));

    assert_eq!(
        at(TIMEOUT_MINUTES_MIN)
            .expect("the minimum is authorable")
            .params
            .timeout_minutes,
        Some(TIMEOUT_MINUTES_MIN)
    );
    assert_eq!(
        at(TIMEOUT_MINUTES_MAX)
            .expect("the maximum is authorable")
            .params
            .timeout_minutes,
        Some(TIMEOUT_MINUTES_MAX)
    );

    assert_eq!(
        at(90)
            .expect("90 minutes is authorable")
            .params
            .timeout_minutes,
        Some(90)
    );

    for bad in [TIMEOUT_MINUTES_MIN - 1, 0, -30] {
        let err = at(bad).expect_err("a round under the floor must be refused");
        assert!(err.to_string().contains("timeoutMinutes"), "{err}");
        assert!(
            err.to_string().contains(&format!(
                "shortest authorable round is {TIMEOUT_MINUTES_MIN}"
            )),
            "the refusal must state the floor: {err}"
        );
    }
    for bad in [TIMEOUT_MINUTES_MAX + 1, 100_000] {
        let err = at(bad).expect_err("a round over the ceiling must be refused");
        assert!(err.to_string().contains("timeoutMinutes"), "{err}");
        assert!(
            err.to_string().contains(&format!(
                "longest authorable round is {TIMEOUT_MINUTES_MAX}"
            )),
            "the refusal must state the ceiling: {err}"
        );
    }
}

#[test]
fn end_on_is_a_closed_vocabulary_and_deduplicates_in_authored_order() {
    let got = parse(&json!({
        "mode": "attrition",
        "endOn": ["faction_eliminated", "time_limit", "faction_eliminated"],
    }))
    .expect("parses");
    assert_eq!(got.end_on, ["faction_eliminated", "time_limit"]);

    let err = parse(&json!({"mode": "attrition", "endOn": ["admin_ended"]}))
        .expect_err("must refuse an unknown trigger");
    assert!(err.to_string().contains("admin_ended"), "{err}");

    let err =
        parse(&json!({"mode": "attrition", "endOn": []})).expect_err("must refuse an empty endOn");
    assert!(err.to_string().contains("endOn"), "{err}");
}

#[test]
fn the_authored_modes_are_the_editor_payload_schema_s_enum() {
    const PAYLOAD_SCHEMA: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../contracts/definitions/mission-editor-payload.schema.json"
    ));
    const MISSION_SCHEMA: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../contracts/definitions/mission.schema.json"
    ));

    let payload: Value = serde_json::from_str(PAYLOAD_SCHEMA).expect("payload schema parses");
    let modes = payload["properties"]["winConditions"]["properties"]["mode"]["enum"]
        .as_array()
        .expect("the editor payload schema declares winConditions.mode as an enum");
    let modes: Vec<&str> = modes.iter().filter_map(Value::as_str).collect();
    assert_eq!(
        modes, AUTHORED_MODES,
        "mission-editor-payload.schema.json's mode enum and AUTHORED_MODES have drifted"
    );

    let mission: Value = serde_json::from_str(MISSION_SCHEMA).expect("mission schema parses");
    let wire = mission["$defs"]["winConditions"]["properties"]["mode"]["enum"]
        .as_array()
        .expect("mission.schema.json declares winConditions.mode as an enum");
    let wire: Vec<&str> = wire.iter().filter_map(Value::as_str).collect();
    for m in AUTHORED_MODES {
        assert!(
            wire.contains(m),
            "the wire enum must admit every authored mode; {m} is missing"
        );
    }

    let triggers = mission["$defs"]["winConditions"]["properties"]["endOn"]["items"]["enum"]
        .as_array()
        .expect("endOn items are an enum");
    let triggers: Vec<&str> = triggers.iter().filter_map(Value::as_str).collect();
    assert_eq!(
        triggers, END_ON_TRIGGERS,
        "END_ON_TRIGGERS and $defs/winConditions.endOn have drifted"
    );
}

#[test]
fn the_timeout_bounds_are_the_schema_s_own() {
    const MISSION_SCHEMA: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../contracts/definitions/mission.schema.json"
    ));
    let mission: Value = serde_json::from_str(MISSION_SCHEMA).expect("mission schema parses");
    let t = &mission["$defs"]["winConditions"]["properties"]["timeoutMinutes"];
    assert_eq!(t["minimum"].as_i64(), Some(TIMEOUT_MINUTES_MIN));
    assert_eq!(t["maximum"].as_i64(), Some(TIMEOUT_MINUTES_MAX));
    assert_eq!(t["type"].as_str(), Some("integer"));
}
