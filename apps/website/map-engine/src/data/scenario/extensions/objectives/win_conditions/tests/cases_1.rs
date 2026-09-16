//! Role: Domain regression cases.
//! Position: `mission/extensions/objectives/win_conditions/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_vip_block_parses_with_its_param() {
    let got = parse(&vip()).expect("parses");
    assert_eq!(got.mode, "vip");
    assert_eq!(got.end_on, ["faction_eliminated"]);
    assert_eq!(got.params.vip_slot_id.as_deref(), Some("s-12"));
    assert!(got.params.extraction_zone_id.is_none());
    assert!(got.params.timeout_minutes.is_none());
}

#[test]
fn attrition_and_objective_take_no_param() {
    for mode in ["attrition", "objective"] {
        let got = parse(&json!({"mode": mode, "endOn": ["time_limit"]})).expect("parses");
        assert!(got.params.is_empty(), "{mode} must carry no param");
    }
}

#[test]
fn a_mode_outside_the_editor_vocabulary_is_refused() {
    for mode in [
        "points_then_attrition",
        "defender_holds_or_attacker_destroys",
        "vip_hunt",
    ] {
        let err =
            parse(&json!({"mode": mode, "endOn": ["time_limit"]})).expect_err("must refuse {mode}");
        assert!(err.contains("winConditions.mode"), "{err}");
        assert!(err.contains(mode), "the refusal must name the value: {err}");
    }
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
        assert!(err.contains(key), "{err}");
        assert!(err.contains(mode), "{err}");
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
    assert!(err.contains("timeoutMinutes"), "{err}");
    assert!(err.contains("\"timeout\""), "{err}");
    assert!(err.contains("\"vip\""), "{err}");

    let err = parse(&json!({
        "mode": "extraction",
        "endOn": ["time_limit"],
        "extractionZoneId": "z1",
        "vipSlotId": "s1",
    }))
    .expect_err("vipSlotId does not belong to extraction");
    assert!(err.contains("vipSlotId"), "{err}");
}

#[test]
fn vip_may_also_carry_an_optional_extraction_zone() {
    let with_zone = parse(&json!({
        "mode": "vip",
        "endOn": ["time_limit"],
        "vipSlotId": "s-12",
        "extractionZoneId": "z-lz",
    }))
    .expect("vip accepts an extraction zone");
    assert_eq!(with_zone.params.vip_slot_id.as_deref(), Some("s-12"));
    assert_eq!(with_zone.params.extraction_zone_id.as_deref(), Some("z-lz"));

    let without = parse(&json!({
        "mode": "vip", "endOn": ["time_limit"], "vipSlotId": "s-12"
    }))
    .expect("the zone is optional");
    assert!(without.params.extraction_zone_id.is_none());

    let err = parse(&json!({
        "mode": "vip", "endOn": ["time_limit"], "vipSlotId": "s", "extractionZoneId": "  "
    }))
    .expect_err("blank is not an id");
    assert!(err.contains("blank"), "{err}");
}

#[test]
fn every_optional_param_key_has_a_parse_branch() {
    for mode in AUTHORED_MODES {
        for key in optional_param_keys_for_mode(mode) {
            assert!(PARAM_KEYS.contains(key), "{mode}: {key} is not a param key");
            assert_ne!(
                param_key_for_mode(mode),
                Some(*key),
                "{mode}: {key} cannot be both required and optional"
            );
            let mut block = json!({"mode": mode, "endOn": ["time_limit"]});
            if let Some(required) = param_key_for_mode(mode) {
                block[required] = json!("x");
            }
            block[*key] = json!("x");
            parse(&block).unwrap_or_else(|e| {
                panic!("{mode} declares {key} optional but parse refuses it: {e}")
            });
        }
    }
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
        assert!(err.contains("timeoutMinutes"), "{err}");
        assert!(
            err.contains(&format!(
                "shortest authorable round is {TIMEOUT_MINUTES_MIN}"
            )),
            "the refusal must state the floor: {err}"
        );
    }
    for bad in [TIMEOUT_MINUTES_MAX + 1, 100_000] {
        let err = at(bad).expect_err("a round over the ceiling must be refused");
        assert!(err.contains("timeoutMinutes"), "{err}");
        assert!(
            err.contains(&format!(
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
    assert!(err.contains("admin_ended"), "{err}");

    let err =
        parse(&json!({"mode": "attrition", "endOn": []})).expect_err("must refuse an empty endOn");
    assert!(err.contains("endOn"), "{err}");
}

#[test]
fn a_blank_or_wrong_typed_param_is_refused() {
    let err = parse(&json!({"mode": "vip", "endOn": ["time_limit"], "vipSlotId": "   "}))
        .expect_err("blank is not an id");
    assert!(err.contains("blank"), "{err}");

    let err = parse(&json!({"mode": "vip", "endOn": ["time_limit"], "vipSlotId": 12}))
        .expect_err("a number is not an id");
    assert!(err.contains("must be a string"), "{err}");

    let err = parse(&json!({
        "mode": "timeout", "endOn": ["time_limit"], "timeoutMinutes": 12.5
    }))
    .expect_err("a fraction is not a whole minute");
    assert!(err.contains("whole number"), "{err}");
}

#[test]
fn a_param_is_trimmed_before_it_reaches_the_wire() {
    let got = parse(&json!({
        "mode": "extraction", "endOn": ["time_limit"], "extractionZoneId": "  z-lz  "
    }))
    .expect("parses");
    assert_eq!(got.params.extraction_zone_id.as_deref(), Some("z-lz"));
}

#[test]
fn a_non_object_block_is_refused_rather_than_defaulted() {
    for bad in [json!(null), json!("attrition"), json!([]), json!(7)] {
        assert!(parse(&bad).is_err(), "{bad} must not parse");
    }
}

#[test]
fn empty_params_serialise_to_no_keys_at_all() {
    let empty = WinConditionParams::default();
    assert!(empty.is_empty());
    assert_eq!(serde_json::to_string(&empty).expect("serialises"), "{}");

    let one = WinConditionParams {
        vip_slot_id: Some("s-12".to_string()),
        ..WinConditionParams::default()
    };
    assert!(!one.is_empty());
    assert_eq!(
        serde_json::to_string(&one).expect("serialises"),
        r#"{"vipSlotId":"s-12"}"#
    );
}

#[test]
fn the_authored_modes_are_the_editor_payload_schema_s_enum() {
    const PAYLOAD_SCHEMA: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/tbd-schema/schema/mission-editor-payload.schema.json"
    ));
    const MISSION_SCHEMA: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/tbd-schema/schema/mission.schema.json"
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
        "/../../../packages/tbd-schema/schema/mission.schema.json"
    ));
    let mission: Value = serde_json::from_str(MISSION_SCHEMA).expect("mission schema parses");
    let t = &mission["$defs"]["winConditions"]["properties"]["timeoutMinutes"];
    assert_eq!(t["minimum"].as_i64(), Some(TIMEOUT_MINUTES_MIN));
    assert_eq!(t["maximum"].as_i64(), Some(TIMEOUT_MINUTES_MAX));
    assert_eq!(t["type"].as_str(), Some("integer"));
}

#[test]
fn every_mode_that_takes_a_param_names_it_both_ways() {
    for mode in AUTHORED_MODES {
        if let Some(key) = param_key_for_mode(mode) {
            assert_eq!(mode_for_param_key(key), *mode, "{mode} round-trips");
            assert!(PARAM_KEYS.contains(&key), "{key} must be in PARAM_KEYS");
        }
    }

    for key in PARAM_KEYS {
        assert!(
            AUTHORED_MODES.contains(&mode_for_param_key(key)),
            "{key} names a mode that is not authorable"
        );
    }
}
