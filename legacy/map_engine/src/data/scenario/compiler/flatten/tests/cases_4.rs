//! Role: Domain regression cases.
//! Position: `mission/compiler/flatten/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn slug_colliding_briefing_keys_merge_rather_than_overwrite() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["editor"]["factions"] = serde_json::json!([
        {"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"],
         "briefing": {"situation": "first",
                      "markers": [{"x": 1.0, "z": 1.0, "icon": "dot", "label": "A"}]}},
        {"id": "f1b", "key": "blufor", "name": "US Army (dup)", "squadIds": [],
         "briefing": {"situation": "second",
                      "markers": [{"x": 2.0, "z": 2.0, "icon": "dot", "label": "B"}]}},
    ]);
    let out = compiled_briefings(&serde_json::to_vec(&p).expect("serialises"));

    let blufor = &out["blufor"];
    assert_eq!(blufor["situation"], "first\n\nsecond");
    assert_eq!(blufor["markers"].as_array().expect("array").len(), 2);
    assert_eq!(blufor["markers"][0]["label"], "A");
    assert_eq!(blufor["markers"][1]["label"], "B");

    assert_eq!(out.as_object().expect("object").len(), 1);
}

#[test]
fn authored_blank_prose_is_distinguishable_from_an_absent_key() {
    let out = compiled_briefings(&payload_with_briefings(serde_json::json!({
        "blufor": {"situation": "", "mission": "", "execution": ""},
        "opfor": {},
    })));

    let blufor = out["blufor"].as_object().expect("object");
    assert_eq!(blufor["situation"], "");
    assert_eq!(blufor["mission"], "");
    assert_eq!(blufor["execution"], "");

    assert!(out.get("opfor").is_some());
    assert_eq!(out["opfor"].as_object().expect("object").len(), 0);
}

#[test]
fn a_library_blurb_on_a_faction_row_is_a_save_time_finding_not_a_compile_500() {
    let payload =
        graph_with_faction_briefing(Some(serde_json::json!("Take the bridge before dawn.")));

    assert!(matches!(
        flatten_to_mod_document(&meta(), &payload),
        Err(CompileError::Parse(_))
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
        Err(CompileError::Parse(_))
    ));
    let found = scan_editor_payload_types(&payload);
    assert_eq!(found.len(), 1, "{found:?}");

    assert!(found[0].starts_with("/editor:"), "{found:?}");
    assert!(found[0].contains("cannot be compiled"), "{found:?}");
}

#[test]
fn not_json_is_left_to_the_schema_pass() {
    assert!(scan_editor_payload_types(b"not json at all").is_empty());
    assert!(scan_editor_payload_types(b"{\"editor\":").is_empty());
}

#[test]
fn marker_field_findings_name_the_row_and_the_field() {
    let payload = graph_with_faction_briefing(Some(serde_json::json!({
        "markers": [
            {"x": 1.0, "z": 2.0, "icon": "objective", "label": "OBJ A"},
            {"x": "5", "z": 10, "icon": 7, "label": "OBJ B"},
        ],
    })));

    let found = scan_editor_payload_types(&payload);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].starts_with("/editor/factions/0/briefing/markers/1/x:"),
        "{found:?}"
    );
    assert!(
        found[1].starts_with("/editor/factions/0/briefing/markers/1/icon:"),
        "{found:?}"
    );

    assert!(
        !found.iter().any(|f| f.contains("/markers/0/")),
        "{found:?}"
    );
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
fn the_flow_keys_share_the_environment_bag_without_colliding() {
    let payload = fixture_with_environment(serde_json::json!({
        "time": "18:45",
        "weather": "overcast",
        "showHillshade": true,
        "showGrid": false,
        "timeLimitSeconds": 7200,
    }));
    let doc = flatten_to_mod_document(&meta(), &payload).expect("compiles");

    assert_eq!(doc.flow.time_limit_seconds, 7200);

    assert_eq!(doc.flow.briefing_seconds, FLOW_DEFAULT_BRIEFING_S);
    assert_eq!(doc.flow.jip, FLOW_DEFAULT_JIP);

    assert_eq!(
        doc.environment
            .as_ref()
            .expect("environment block")
            .date_time,
        format!("{COMPILE_DATE_ANCHOR}T05:30:00Z")
    );
}

#[test]
fn an_authored_zero_duration_is_kept_and_not_read_as_unauthored() {
    let flow = flow_of(serde_json::json!({
        "briefingSeconds": 0,
        "safeStartSeconds": 0,
        "timeLimitSeconds": 0,
    }));

    assert_eq!(flow.briefing_seconds, 0);
    assert_eq!(flow.safe_start_seconds, 0);
    assert_eq!(flow.time_limit_seconds, 0);
}

#[test]
fn flow_default_literals_are_the_contract() {
    assert_eq!(
        FLOW_DEFAULT_BRIEFING_S, 600,
        "briefing length is announced by TBD_FrameworkManager.OnEnterBriefing"
    );
    assert_eq!(
        FLOW_DEFAULT_SAFESTART_S, 300,
        "must equal TBD_SafestartManager.DEFAULT_COUNTDOWN_SECONDS"
    );
    assert_eq!(
        FLOW_DEFAULT_TIMELIMIT_S, 5400,
        "the round clock TBD_FrameworkManager.ArmRoundClock arms"
    );
    assert_eq!(
        FLOW_DEFAULT_JIP, "until_safestart_end",
        "the JIP door TBD_SpawnManager holds open"
    );
    assert!(
        JIP_VALUES.contains(&FLOW_DEFAULT_JIP),
        "a default outside the schema enum resolves to ALWAYS in PolicyFromString"
    );
}

#[test]
fn an_unauthored_mission_emits_the_defaults_it_runs_with() {
    for (label, env) in [
        ("absent", None),
        ("empty object", Some(serde_json::json!({}))),
        ("null", Some(serde_json::Value::Null)),
        ("non-object", Some(serde_json::json!("clear"))),
    ] {
        let doc = match env {
            None => flatten_to_mod_document(&meta(), FIXTURE.as_bytes()),
            Some(e) => flatten_to_mod_document(&meta(), &fixture_with_environment(e)),
        }
        .unwrap_or_else(|e| panic!("{label} environment must still compile: {e}"));

        assert_eq!(
            doc.flow.briefing_seconds, FLOW_DEFAULT_BRIEFING_S,
            "{label}"
        );
        assert_eq!(
            doc.flow.safe_start_seconds, FLOW_DEFAULT_SAFESTART_S,
            "{label}"
        );
        assert_eq!(
            doc.flow.time_limit_seconds, FLOW_DEFAULT_TIMELIMIT_S,
            "{label}"
        );
        assert_eq!(doc.flow.jip, FLOW_DEFAULT_JIP, "{label}");
    }
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
fn every_schema_legal_jip_reaches_the_document() {
    for want in JIP_VALUES {
        let flow = flow_of(serde_json::json!({ "jip": want }));
        assert_eq!(flow.jip, want);
    }
}

#[test]
fn the_flow_defaults_are_the_values_the_committed_golden_carries() {
    let golden: serde_json::Value =
        serde_json::from_str(COMPILER_SHAPED_GOLDEN).expect("the golden is valid JSON");
    let flow = &golden["flow"];

    assert_eq!(flow["briefingSeconds"], FLOW_DEFAULT_BRIEFING_S);
    assert_eq!(flow["safeStartSeconds"], FLOW_DEFAULT_SAFESTART_S);
    assert_eq!(flow["timeLimitSeconds"], FLOW_DEFAULT_TIMELIMIT_S);
    assert_eq!(flow["jip"], FLOW_DEFAULT_JIP);
}

#[test]
fn a_malformed_environment_neither_fails_the_precheck_nor_the_compile() {
    for env in [
        serde_json::json!({"timeLimitSeconds": "90 minutes"}),
        serde_json::json!({"timeLimitSeconds": 5400.0}),
        serde_json::json!({"jip": 3}),
        serde_json::json!({"briefingSeconds": {"minutes": 10}}),
        serde_json::json!("not an object"),
        serde_json::json!(42),
        serde_json::Value::Null,
    ] {
        let payload = fixture_with_environment(env.clone());
        assert!(
            scan_editor_payload_types(&payload).is_empty(),
            "the save boundary must not reject environment {env}"
        );
        assert!(
            flatten_to_mod_document(&meta(), &payload).is_ok(),
            "environment {env} must still compile"
        );
    }
}

#[test]
fn clock_accepts_the_shapes_the_document_can_hold() {
    assert_eq!(clock_hhmm("21:45").as_deref(), Some("21:45"));
    assert_eq!(clock_hhmm("21:45:00").as_deref(), Some("21:45"));
    assert_eq!(clock_hhmm("06:00:59").as_deref(), Some("06:00"));
    assert_eq!(clock_hhmm("6:5").as_deref(), Some("06:05"));
    assert_eq!(clock_hhmm("00:00").as_deref(), Some("00:00"));
    assert_eq!(clock_hhmm("23:59").as_deref(), Some("23:59"));

    for bad in [
        "",
        "24:00",
        "12:60",
        "12:00:60",
        "12",
        "12:00:00:00",
        "-1:00",
        " 12:00",
        "noon",
        "12:0a",
    ] {
        assert_eq!(clock_hhmm(bad), None, "{bad:?} must not reach dateTime");
    }
}

#[test]
fn only_a_usable_authored_value_displaces_the_row() {
    let row = || MissionMeta {
        time_of_day: "05:30".into(),
        weather_preset: "clear".into(),
        ..MissionMeta::default()
    };
    let env_payload = |env: serde_json::Value| {
        serde_json::to_vec(&serde_json::json!({ "environment": env })).unwrap()
    };

    for (name, env) in [
        ("absent", serde_json::json!({})),
        (
            "wrong types",
            serde_json::json!({"time": 2145, "weather": false}),
        ),
        (
            "blank strings",
            serde_json::json!({"time": "", "weather": ""}),
        ),
        (
            "off-enum + junk",
            serde_json::json!({"time": "half past four", "weather": "blizzard"}),
        ),
        (
            "out-of-range clock",
            serde_json::json!({"time": "24:00", "weather": "Clear"}),
        ),
        ("non-object bag", serde_json::json!("not an object")),
        ("null bag", serde_json::Value::Null),
    ] {
        let mut meta = row();
        apply_authored_environment(&mut meta, &env_payload(env));
        assert_eq!(meta.time_of_day, "05:30", "{name}: expected the row's time");
        assert_eq!(meta.weather_preset, "clear", "{name}: expected the row");
    }

    let mut meta = row();
    apply_authored_environment(
        &mut meta,
        &env_payload(serde_json::json!({"weather": "overcast"})),
    );
    assert_eq!(meta.weather_preset, "overcast");
    assert_eq!(meta.time_of_day, "05:30");

    let mut meta = row();
    apply_authored_environment(
        &mut meta,
        &env_payload(serde_json::json!({"time": "19:05"})),
    );
    assert_eq!(meta.time_of_day, "19:05");
    assert_eq!(meta.weather_preset, "clear");

    let payload = env_payload(serde_json::json!({"time": "21:45", "weather": "dense_fog"}));
    let mut meta = row();
    apply_authored_environment(&mut meta, &payload);
    let once = (meta.time_of_day.clone(), meta.weather_preset.clone());
    apply_authored_environment(&mut meta, &payload);
    assert_eq!((meta.time_of_day, meta.weather_preset), once);
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
fn t682_authored_zero_fog_and_wind_are_kept() {
    let doc = flatten_to_mod_document(
        &meta(),
        &fixture_with_environment(serde_json::json!({
            "fog": 0,
            "wind": 0,
            "windDirDeg": 0,
        })),
    )
    .expect("compiles");
    let env = serde_json::to_value(doc.environment.as_ref().expect("environment block"))
        .expect("serialises");
    assert_eq!(env["fog"].as_f64(), Some(0.0));
    assert_eq!(env["wind"].as_f64(), Some(0.0));
    assert_eq!(env["windDirDeg"].as_f64(), Some(0.0));
    assert!(
        env.get("viewDistance").is_none(),
        "unauthored viewDistance must not appear"
    );
}

#[test]
fn t682_absent_axes_omit_the_keys_and_do_not_bump_version() {
    let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
    let env = serde_json::to_value(doc.environment.as_ref().expect("environment block"))
        .expect("serialises");
    for key in ["windDirDeg", "fog", "wind", "viewDistance"] {
        assert!(
            env.get(key).is_none(),
            "{key} must be omitted when the payload never authored it"
        );
    }
    assert_eq!(doc.schema_version, "1.2");
    assert_eq!(env["weatherPreset"], "clear");
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
            .map(|f| f.rule_id)
            .collect::<Vec<_>>(),
        vec![DIAG_DROP_SLOT_RANK],
        "seeding one dropped value must produce exactly one finding; got: {:?}",
        dirty.diagnostics
    );
}

#[test]
fn every_drop_rule_fires_over_a_fixture_built_to_trip_it() {
    let doc = flatten_to_mod_document(&meta(), DROP_FIXTURE.as_bytes()).expect("compiles");
    let ids: Vec<&str> = doc.diagnostics.iter().map(|f| f.rule_id).collect();
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
            COMPILE_DIAGNOSTIC_RULE_IDS.contains(&f.rule_id),
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
            f.subject_id.as_deref().is_some_and(|s| !s.is_empty()),
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
