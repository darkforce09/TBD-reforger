//! **Role:** Parse refusals, type scans, entities and the compile findings of dropped identity.
//! **Position:** `mission_compiler::game_document::tests::cases_4` in the `mission_compiler` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_library_blurb_on_a_faction_row_is_a_save_time_finding_not_a_compile_500() {
    let payload =
        graph_with_faction_briefing(Some(serde_json::json!("Take the bridge before dawn.")));

    assert!(matches!(
        flatten_to_mod_document(&meta(), &payload),
        Err(Error::Parse(_))
    ));

    let found = scan_editor_payload_types(&payload);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("/editor/factions/0/briefing:"),
        "the finding must name the offending node, not a byte column: {found:?}"
    );
    assert!(found[0].contains("got a string"), "{found:?}");
    assert!(
        found[0].contains("library blurb"),
        "the finding must name the briefing-vs-briefings distinction that caused it: {found:?}"
    );
}

#[test]
fn the_precheck_accepts_exactly_what_the_compiler_can_parse() {
    let cases: [(&str, Option<serde_json::Value>, bool); 11] = [
        ("no briefing key", None, true),
        ("explicit null", Some(serde_json::Value::Null), true),
        (
            "the library blurb",
            Some(serde_json::json!("some string")),
            false,
        ),
        ("a number", Some(serde_json::json!(3)), false),
        ("a boolean", Some(serde_json::json!(true)), false),
        (
            "situation as a number",
            Some(serde_json::json!({"situation": 5})),
            false,
        ),
        (
            "markers as a string",
            Some(serde_json::json!({"markers": "OBJ"})),
            false,
        ),
        (
            "marker.x as a string",
            Some(serde_json::json!({"markers": [{"x": "5", "z": 10, "icon": "o", "label": "L"}]})),
            false,
        ),
        ("an array", Some(serde_json::json!([])), true),
        (
            "situation null",
            Some(serde_json::json!({"situation": null})),
            true,
        ),
        (
            "an unknown subkey",
            Some(serde_json::json!({"nope": 1})),
            true,
        ),
    ];

    for (name, value, compiles) in cases {
        let payload = graph_with_faction_briefing(value);
        let found = scan_editor_payload_types(&payload);
        let compiled = flatten_to_mod_document(&meta(), &payload);

        assert_eq!(
            compiled.is_ok(),
            compiles,
            "{name}: compile outcome moved away from the measured baseline"
        );
        assert_eq!(
            found.is_empty(),
            compiles,
            "{name}: the save boundary and the compiler disagree — {found:?}"
        );
    }
}

#[test]
fn a_type_error_anywhere_in_the_graph_is_caught_not_just_in_a_briefing() {
    let payload = serde_json::to_vec(&serde_json::json!({
        "schemaVersion": 1,
        "editor": {
            "factions": [{"key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
            "squads": [{"id": "sq1", "callsign": "Ranger", "slotIds": ["n0"]}],
            "slots": [{"id": "n0", "index": 0, "role": "SL",
                       "position": {"x": "4837.6", "y": 7710.8, "z": 0, "rotation": 45}}],
        },
    }))
    .expect("serialises");

    assert!(matches!(
        flatten_to_mod_document(&meta(), &payload),
        Err(Error::Parse(_))
    ));
    let found = scan_editor_payload_types(&payload);
    assert_eq!(found.len(), 1, "{found:?}");

    assert!(found[0].starts_with("/editor:"), "{found:?}");
    assert!(found[0].contains("cannot be compiled"), "{found:?}");
}

#[test]
fn the_precheck_is_silent_on_every_payload_this_module_already_compiles() {
    for (name, bytes) in [
        ("FIXTURE", FIXTURE.as_bytes()),
        (
            "COMPILER_SHAPED_PAYLOAD",
            COMPILER_SHAPED_PAYLOAD.as_bytes(),
        ),
    ] {
        assert!(
            scan_editor_payload_types(bytes).is_empty(),
            "{name} compiles but the save boundary would have rejected it"
        );
    }
}

#[test]
fn authored_flow_values_reach_the_compiled_document() {
    let flow = flow_of(serde_json::json!({
        "briefingSeconds": 480,
        "safeStartSeconds": 180,
        "timeLimitSeconds": 3000,
        "jip": "disabled",
    }));

    assert_eq!(flow.briefing_seconds, 480);
    assert_eq!(flow.safe_start_seconds, 180);
    assert_eq!(flow.time_limit_seconds, 3000);
    assert_eq!(flow.jip, "disabled");
}

#[test]
fn out_of_range_and_wrong_typed_values_fall_back_to_the_default() {
    for bad in [
        serde_json::json!(-1),
        serde_json::json!(-5400),
        serde_json::json!("5400"),
        serde_json::json!(5400.5),
        serde_json::json!(true),
        serde_json::json!(null),
        serde_json::json!([5400]),
    ] {
        let flow = flow_of(serde_json::json!({ "timeLimitSeconds": bad.clone() }));
        assert_eq!(
            flow.time_limit_seconds, FLOW_DEFAULT_TIMELIMIT_S,
            "timeLimitSeconds {bad} must read as unauthored"
        );
    }

    for bad in [
        serde_json::json!("Disabled"),
        serde_json::json!("until_safestart"),
        serde_json::json!(""),
        serde_json::json!(0),
    ] {
        let flow = flow_of(serde_json::json!({ "jip": bad.clone() }));
        assert_eq!(
            flow.jip, FLOW_DEFAULT_JIP,
            "jip {bad} must read as unauthored"
        );
    }
}

#[test]
fn t682_environment_axes_serialise_when_authored() {
    let doc = flatten_to_mod_document(
        &meta(),
        &fixture_with_environment(serde_json::json!({
            "time": "05:30",
            "weather": "clear",
            "windDirDeg": 45,
            "fog": 0.2,
            "wind": 3.5,
            "viewDistance": 2500,
        })),
    )
    .expect("compiles");
    let env = serde_json::to_value(doc.environment.as_ref().expect("environment block"))
        .expect("serialises");
    assert_eq!(env["windDirDeg"].as_f64(), Some(45.0));
    assert_eq!(env["fog"].as_f64(), Some(0.2));
    assert_eq!(env["wind"].as_f64(), Some(3.5));
    assert_eq!(env["viewDistance"].as_f64(), Some(2500.0));
    assert_eq!(doc.schema_version, "1.3", "those four keys are 1.3-shaped");
}

#[test]
fn t682_malformed_or_out_of_range_axes_are_dropped() {
    for (name, bag) in [
        ("string fog", serde_json::json!({"fog": "thick"})),
        ("fog > 1", serde_json::json!({"fog": 1.1})),
        ("fog < 0", serde_json::json!({"fog": -0.1})),
        ("windDirDeg 361", serde_json::json!({"windDirDeg": 361})),
        ("negative wind", serde_json::json!({"wind": -1})),
        ("viewDistance 0", serde_json::json!({"viewDistance": 0})),
        (
            "viewDistance negative",
            serde_json::json!({"viewDistance": -50}),
        ),
    ] {
        let doc = flatten_to_mod_document(&meta(), &fixture_with_environment(bag))
            .unwrap_or_else(|e| panic!("{name} must still compile: {e}"));
        let env = serde_json::to_value(doc.environment.as_ref().expect("environment block"))
            .expect("serialises");
        for key in ["windDirDeg", "fog", "wind", "viewDistance"] {
            assert!(
                env.get(key).is_none(),
                "{name}: {key} must not reach the wire"
            );
        }
        assert_eq!(doc.schema_version, "1.2", "{name}: must not claim 1.3");
    }
}

#[test]
fn a_clean_mission_compiles_with_no_diagnostics() {
    let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
    assert!(
        doc.diagnostics.is_empty(),
        "a clean mission must produce NO findings (no severity on correct input); got: {:?}",
        doc.diagnostics
    );

    let seeded = FIXTURE.replace(
        r#""id": "s2", "squadId": "sq1", "index": 1, "role": "TL""#,
        r#""id": "s2", "squadId": "sq1", "index": 1, "role": "TL", "rank": "Lance Corporal""#,
    );
    assert_ne!(seeded, FIXTURE, "the seed must actually change the fixture");
    let dirty = flatten_to_mod_document(&meta(), seeded.as_bytes()).expect("compiles");
    assert_eq!(
        dirty
            .diagnostics
            .iter()
            .map(|f| f.rule_id.as_str())
            .collect::<Vec<_>>(),
        vec![DIAG_DROP_SLOT_RANK],
        "seeding one dropped value must produce exactly one finding; got: {:?}",
        dirty.diagnostics
    );
}

#[test]
fn every_drop_rule_fires_over_a_fixture_built_to_trip_it() {
    let doc = flatten_to_mod_document(&meta(), DROP_FIXTURE.as_bytes()).expect("compiles");
    let ids: Vec<&str> = doc.diagnostics.iter().map(|f| f.rule_id.as_str()).collect();
    for expected in COMPILE_DIAGNOSTIC_RULE_IDS {
        assert!(
            ids.contains(&expected),
            "{expected} never fired over a fixture that authors an unrepresentable value for \
                 every rule by construction; got: {ids:?}"
        );
    }

    let wire = serde_json::to_value(&doc).expect("serialises");
    assert_eq!(wire["schemaVersion"], "1.1");
    for key in ["callsign", "rank", "stance", "unitName", "tag"] {
        assert!(
            wire["slots"][0].get(key).is_none(),
            "an unrepresentable {key} reached the wire and was reported dropped"
        );
    }
    assert!(
        wire["orbat"]["blufor"]["groups"][0]
            .get("leaderSlotId")
            .is_none(),
        "a dangling leaderSlotId reached the wire"
    );

    assert!(
        wire.get("vehicles").is_none(),
        "a vehicle whose crew ref dangles reached the wire: {}",
        wire["vehicles"]
    );

    assert_eq!(wire["entities"][0]["alias"], "veh:m151_mg");

    let owner = |rule: &str| -> Option<String> {
        doc.diagnostics
            .iter()
            .find(|f| f.rule_id == rule)
            .and_then(|f| f.subject_id.clone())
            .map(String::from)
    };
    assert_eq!(owner(DIAG_DROP_SQUAD_LEADER).as_deref(), Some("sq1"));
    assert_eq!(owner(DIAG_DROP_SLOT_TAG).as_deref(), Some("s1"));
    assert_eq!(owner(DIAG_DROP_SLOT_CALLSIGN).as_deref(), Some("s1"));
    assert_eq!(owner(DIAG_DROP_SLOT_RANK).as_deref(), Some("s1"));
    assert_eq!(owner(DIAG_DROP_SLOT_STANCE).as_deref(), Some("s1"));
    assert_eq!(owner(DIAG_DROP_SLOT_UNIT_NAME).as_deref(), Some("s1"));
    assert_eq!(owner(DIAG_DROP_VEHICLE_ROSTER).as_deref(), Some("v1"));

    for f in &doc.diagnostics {
        assert!(
            COMPILE_DIAGNOSTIC_RULE_IDS.contains(&f.rule_id.as_str()),
            "unregistered rule id {:?} — COMPILE_DIAGNOSTIC_RULE_IDS is the one enumeration",
            f.rule_id
        );
        assert!(
            !f.message.trim().is_empty(),
            "{:?} has no message",
            f.rule_id
        );
        assert!(
            f.subject.starts_with('/'),
            "{:?} subject {:?} is not a payload pointer",
            f.rule_id,
            f.subject
        );
        assert!(
            f.subject_id
                .as_ref()
                .map(SubjectId::as_str)
                .is_some_and(|s| !s.is_empty()),
            "{:?} names no owning entity — the panel could not select the offender",
            f.rule_id
        );
        assert!(
            matches!(f.severity, Severity::Warning | Severity::Info),
            "{:?} is an Error, but a drop is not a refusal",
            f.rule_id
        );
    }
}

#[test]
fn a_diagnostic_is_not_a_refusal_and_does_not_move_the_bytes() {
    let (bytes, findings) =
        flatten_mod_document_json_with_diagnostics(DIAG_META_JSON, LEDGER_FIXTURE.as_bytes())
            .expect("compiles despite findings");
    assert!(
        !findings.is_empty(),
        "the ledger fixture must have findings"
    );
    assert!(!bytes.is_empty(), "the compile must still produce bytes");

    let plain =
        flatten_mod_document_json(DIAG_META_JSON, LEDGER_FIXTURE.as_bytes()).expect("compiles");
    assert_eq!(
        bytes, plain,
        "the diagnostics entry point must return byte-identical bytes — one compile, three \
             projections, or the panel and the download disagree about what was compiled"
    );

    let wire: serde_json::Value = serde_json::from_slice(&bytes).expect("bytes are JSON");
    assert!(
        !any_object_has_key(&wire, "diagnostics"),
        "the diagnostics must ride ALONGSIDE the bytes, never inside them"
    );
}
