//! Role: Domain regression cases.
//! Position: `mission/compiler/flatten/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn diagnostics_are_never_debug_gated() {
    let doc = flatten_to_mod_document(&meta(), LEDGER_FIXTURE.as_bytes()).expect("compiles");
    assert!(
        !doc.diagnostics.is_empty(),
        "a normal build produced no findings — the path is not live"
    );

    const SRC: &str = concat!(
        include_str!("../compile_graph.rs"),
        include_str!("../roster.rs")
    );
    let production = SRC.split("#[cfg(test)]").next().expect("test marker");

    let emission_needles = [
        "diagnostics.squad_leader_dropped(",
        "diagnostics.slot_identity_dropped(",
        "diagnostics.vehicle_roster_dropped(",
        "diagnostics: diagnostics.findings,",
    ];
    let lines: Vec<&str> = production.lines().collect();
    for needle in emission_needles {
        let at = lines
            .iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("emission site {needle:?} has vanished from the compile"));

        let from = at.saturating_sub(40);
        for line in &lines[from..=at] {
            let t = line.trim_start();
            assert!(
                !(t.starts_with("#[cfg(") || t.starts_with("#![cfg(")),
                "{needle:?} is under a conditional-compilation attribute ({line:?}) — a \
                     diagnostic that only exists in some builds is the un-diagnosable shipped \
                     state fnf_v4.md 14.9 names"
            );
            assert!(
                !t.contains("debug_assertions"),
                "{needle:?} is debug-gated ({line:?})"
            );
        }
    }
}

#[test]
fn a_blank_authored_value_is_not_a_finding() {
    let payload = FIXTURE.replace(
            r#"{"id": "s2", "squadId": "sq1", "index": 1, "role": "TL","#,
            r#"{"id": "s2", "squadId": "sq1", "index": 1, "role": "TL", "rank": "", "tag": "   ", "stance": null,"#,
        );
    assert_ne!(payload, FIXTURE, "the blank seed must change the fixture");
    let doc = flatten_to_mod_document(&meta(), payload.as_bytes()).expect("compiles");
    assert!(
        doc.diagnostics.is_empty(),
        "blank / null authored values must not produce findings; got {:?}",
        doc.diagnostics
    );
}

#[test]
fn an_unreferenced_squad_reports_no_drop() {
    let payload = FIXTURE.replace(
            r#"{"id": "sq2", "factionId": "f2", "name": "Grom", "slotIds": ["s4"]}"#,
            r#"{"id": "sq2", "factionId": "f2", "name": "Grom", "slotIds": ["s4"]},
               {"id": "sqX", "factionId": "f2", "name": "Ghost", "slotIds": [], "leaderSlotId": "sNope"}"#,
        );
    assert_ne!(payload, FIXTURE, "the orphan seed must change the fixture");
    let doc = flatten_to_mod_document(&meta(), payload.as_bytes()).expect("compiles");
    assert!(
        doc.diagnostics.is_empty(),
        "an orphan squad's dropped leader is not an author-actionable finding; got {:?}",
        doc.diagnostics
    );
}

#[test]
fn wrong_typed_identity_keys_still_compile() {
    let payload = FIXTURE.replace(
            r#"{"id": "s2", "squadId": "sq1", "index": 1, "role": "TL","#,
            r#"{"id": "s2", "squadId": "sq1", "index": 1, "role": "TL", "stance": 5, "rank": {"a": 1}, "tag": [1, 2],"#,
        );
    assert_ne!(
        payload, FIXTURE,
        "the wrong-type seed must change the fixture"
    );
    assert!(
        scan_editor_payload_types(payload.as_bytes()).is_empty(),
        "the save-time precheck must still accept a payload that compiles"
    );
    let doc = flatten_to_mod_document(&meta(), payload.as_bytes()).expect("compiles");

    let by_id = |r: &str| {
        doc.diagnostics
            .iter()
            .find(|f| f.rule_id == r)
            .map(|f| f.message.clone())
            .unwrap_or_default()
    };
    assert!(
        by_id(DIAG_DROP_SLOT_STANCE).contains("`5`"),
        "{}",
        by_id(DIAG_DROP_SLOT_STANCE)
    );
    assert!(by_id(DIAG_DROP_SLOT_RANK).contains("an object"));
    assert!(by_id(DIAG_DROP_SLOT_TAG).contains("an array"));
}

#[test]
fn slot_identity_and_squad_leader_reach_the_compiled_wire() {
    let doc = flatten_to_mod_document(&meta(), IDENTITY_FIXTURE.as_bytes()).expect("compiles");
    let wire = serde_json::to_value(&doc).expect("serialises");

    assert_eq!(wire["schemaVersion"], "1.3");
    let s0 = &wire["slots"][0];
    assert_eq!(s0["callsign"], "Alpha-One-Actual");

    assert_eq!(s0["rank"], "sergeant");
    assert_eq!(s0["stance"], "prone");
    assert_eq!(s0["unitName"], "Sgt. Reyes");
    assert_eq!(s0["tag"], "MEDIC-TAG");

    assert_eq!(s0["groupCallsign"], "Alpha");

    assert_eq!(wire["orbat"]["blufor"]["groups"][0]["leaderSlotId"], "s2");
    assert!(
        !any_object_has_key(&wire["slots"], "leaderSlotId"),
        "leaderSlotId was denormalised onto the seats — N copies that can disagree, which is \
             what W120 M-4 rejected"
    );

    let s1 = &wire["slots"][1];
    for key in ["callsign", "rank", "stance", "unitName", "tag"] {
        assert!(s1.get(key).is_none(), "seat 2 authored no {key}");
    }

    assert!(
        doc.diagnostics.is_empty(),
        "every authored value reached the wire, so there is nothing to report; got {:?}",
        doc.diagnostics
    );
}

#[test]
fn identity_values_the_wire_cannot_carry_drop_whole() {
    let cases: &[(&str, &str, &str, Option<&str>)] = &[
        (
            "tab in callsign",
            "callsign",
            r#""Alpha\tActual""#,
            Some(DIAG_DROP_SLOT_CALLSIGN),
        ),
        (
            "newline in unitName",
            "unitName",
            r#""Sgt.\nReyes""#,
            Some(DIAG_DROP_SLOT_UNIT_NAME),
        ),
        (
            "DEL in tag",
            "tag",
            r#""ME\u007fDIC""#,
            Some(DIAG_DROP_SLOT_TAG),
        ),
        (
            "rank off the ladder",
            "rank",
            r#""Lance Corporal""#,
            Some(DIAG_DROP_SLOT_RANK),
        ),
        (
            "stance off the enum",
            "stance",
            r#""kneeling""#,
            Some(DIAG_DROP_SLOT_STANCE),
        ),
        ("numeric stance", "stance", "5", Some(DIAG_DROP_SLOT_STANCE)),
        ("array tag", "tag", "[1, 2]", Some(DIAG_DROP_SLOT_TAG)),
        ("empty callsign", "callsign", r#""""#, None),
        ("whitespace tag", "tag", r#""   ""#, None),
        ("null rank", "rank", "null", None),
    ];

    for (label, key, literal, rule) in cases {
        let payload = STRIPPED_IDENTITY_FIXTURE.replace(
            r#""id": "s1", "squadId": "sq1", "index": 0, "role": "RFL","#,
            &format!(
                r#""id": "s1", "squadId": "sq1", "index": 0, "role": "RFL", "{key}": {literal},"#
            ),
        );
        assert_ne!(
            payload, STRIPPED_IDENTITY_FIXTURE,
            "{label}: the seed must change the fixture"
        );

        assert!(
            scan_editor_payload_types(payload.as_bytes()).is_empty(),
            "{label}: the save-time precheck must still accept this payload"
        );
        let doc = flatten_to_mod_document(&meta(), payload.as_bytes())
            .unwrap_or_else(|e| panic!("{label}: must still compile: {e}"));
        let wire = serde_json::to_value(&doc).expect("serialises");

        assert!(
            wire["slots"][0].get(*key).is_none(),
            "{label}: {key} reached the wire as {:?} — an unrepresentable value must drop \
                 WHOLE, not be trimmed, blanked or coerced",
            wire["slots"][0][*key]
        );
        let ids: Vec<&str> = doc.diagnostics.iter().map(|f| f.rule_id).collect();
        match rule {
            Some(expected) => assert_eq!(
                ids,
                vec![*expected],
                "{label}: expected exactly one finding naming the refused value"
            ),
            None => assert!(
                ids.is_empty(),
                "{label}: a value the author never set is not a finding; got {ids:?}"
            ),
        }
    }
}

#[test]
fn a_dangling_squad_leader_never_reaches_the_wire() {
    for (label, leader) in [
        ("names no seat at all", "sNope"),
        ("names a seat in another squad", "s4"),
    ] {
        let payload = IDENTITY_FIXTURE.replace(
            r#""leaderSlotId": "s2""#,
            &format!(r#""leaderSlotId": "{leader}""#),
        );
        assert_ne!(
            payload, IDENTITY_FIXTURE,
            "{label}: the seed must change the fixture"
        );
        let doc = flatten_to_mod_document(&meta(), payload.as_bytes()).expect("compiles");
        let wire = serde_json::to_value(&doc).expect("serialises");
        assert!(
            wire["orbat"]["blufor"]["groups"][0]
                .get("leaderSlotId")
                .is_none(),
            "{label}: a dangling leader reached the wire"
        );
        assert!(
            doc.diagnostics
                .iter()
                .any(|f| f.rule_id == DIAG_DROP_SQUAD_LEADER),
            "{label}: the drop was silent"
        );
    }

    let doc = flatten_to_mod_document(&meta(), IDENTITY_FIXTURE.as_bytes()).expect("compiles");
    let wire = serde_json::to_value(&doc).expect("serialises");
    let leader = wire["orbat"]["blufor"]["groups"][0]["leaderSlotId"]
        .as_str()
        .expect("leaderSlotId emitted");
    assert!(
        doc.slots.iter().any(|s| s.uid == leader),
        "leaderSlotId {leader:?} resolves against no slot uid"
    );
    assert!(
        !doc.slots.iter().any(|s| s.id == leader),
        "leaderSlotId is carrying the DERIVED slots[].id — it must carry uid, or a role rename \
             silently re-points the squad's leader"
    );
}

#[test]
fn the_identity_enums_are_the_schema_s_own() {
    let schema: serde_json::Value =
        serde_json::from_str(MISSION_SCHEMA_RAW).expect("mission.schema.json parses");
    for (key, ours) in [
        ("rank", SLOT_RANKS.as_slice()),
        ("stance", SLOT_STANCES.as_slice()),
    ] {
        let declared: Vec<&str> = schema["$defs"]["slot"]["properties"][key]["enum"]
            .as_array()
            .unwrap_or_else(|| panic!("$defs/slot.{key} declares no enum"))
            .iter()
            .map(|v| v.as_str().expect("enum tokens are strings"))
            .collect();
        assert_eq!(
            ours, declared,
            "the {key} ladder in flatten.rs and the one in mission.schema.json have drifted — \
                 a token on only one side is either a value that can never be authored or a 500 at \
                 /compiled"
        );
    }
}

#[test]
fn the_vehicle_seat_roles_are_the_schema_s_own() {
    let schema: serde_json::Value =
        serde_json::from_str(MISSION_SCHEMA_RAW).expect("mission.schema.json parses");
    let declared: Vec<&str> =
        schema["$defs"]["vehicle"]["properties"]["seats"]["items"]["properties"]["role"]["enum"]
            .as_array()
            .expect("$defs/vehicle.seats[].role declares no enum")
            .iter()
            .map(|v| v.as_str().expect("enum tokens are strings"))
            .collect();
    assert_eq!(
        VEHICLE_SEAT_ROLES.as_slice(),
        declared,
        "the crew-station list in flatten.rs and the one in mission.schema.json have drifted"
    );
}

#[test]
fn the_authored_vehicle_roster_reaches_the_compiled_wire() {
    let doc = roster_wire(|_| {});
    let wire = serde_json::to_value(&doc).expect("serialises");

    assert_eq!(wire["schemaVersion"], "1.3");
    assert_eq!(
        wire["vehicles"],
        serde_json::json!([{
            "alias": "veh:m151_mg",
            "uid": "v1",
            "x": 100.5,
            "z": 200.5,
            "headingDeg": 90.0,
            "faction": "blufor",
            "seats": [
                {"slotId": "s1", "role": "driver"},
                {"slotId": "s2", "role": "cargo", "index": 2},
            ],
        }]),
        "vehicles: {}",
        wire["vehicles"]
    );

    assert_eq!(wire["entities"][0]["headingDeg"], 90.0);

    assert_eq!(wire["vehicles"][0]["seats"][1]["index"], 2);

    assert!(
        doc.diagnostics.is_empty(),
        "a fully representable roster produced findings: {:?}",
        doc.diagnostics
    );
}

#[test]
#[ignore = "manual — writes a document for `cargo xtask schema validate-file`"]
fn dump_roster_document_for_schema_validate() {
    let doc = roster_wire(|_| {});
    let path = std::env::temp_dir().join("t675_roster_document.json");
    let mut s = serde_json::to_string_pretty(&doc).expect("serialize compiled document");
    s.push('\n');
    std::fs::write(&path, s).expect("write document");
    eprintln!("wrote {}", path.display());
}

#[test]
fn a_mission_with_no_roster_carries_no_vehicles_key() {
    for (name, payload) in [
        ("FIXTURE", FIXTURE),
        ("IDENTITY_FIXTURE", IDENTITY_FIXTURE),
        ("COMPILER_SHAPED_PAYLOAD", COMPILER_SHAPED_PAYLOAD),
    ] {
        let doc = flatten_to_mod_document(&meta(), payload.as_bytes()).expect("compiles");
        let wire = serde_json::to_value(&doc).expect("serialises");
        assert!(
            wire.get("vehicles").is_none(),
            "{name} authors no vehicle roster but the compiled document grew a `vehicles` key"
        );
        assert!(
            !any_object_has_key(&wire, "seats"),
            "{name} authors no crew plan but the compiled document grew a `seats` key"
        );
    }
}

#[test]
fn an_unrepresentable_roster_row_drops_whole_and_is_reported() {
    type RosterDropCase = (
        &'static str,
        &'static str,
        &'static str,
        fn(&mut serde_json::Value),
    );
    let cases: &[RosterDropCase] = &[
        (
            "no `veh:` alias (unplaced — a PLACED one refuses the whole compile, T-425)",
            "has no `veh:` alias",
            "v1",
            |p| {
                p["vehicles"][0]["resourceName"] =
                    serde_json::json!("{DEADBEEF00000000}Prefabs/Vehicles/Nope.et");
                p["vehicles"][0]
                    .as_object_mut()
                    .expect("vehicle row")
                    .remove("position");
            },
        ),
        (
            "no map position (schema requires x and z)",
            "has no map position",
            "v1",
            |p| {
                p["vehicles"][0]
                    .as_object_mut()
                    .expect("vehicle row")
                    .remove("position");
            },
        ),
        (
            "wire-unsafe id (`wireSafeString` forbids control bytes)",
            "carries a control character",
            "v\t1",
            |p| p["vehicles"][0]["id"] = serde_json::json!("v\t1"),
        ),
        (
            "crew ref naming no compiled slot",
            "is not on the compiled roster",
            "v1",
            |p| p["vehicles"][0]["crew"] = serde_json::json!({"driver": "sNope"}),
        ),
        (
            "crew ref naming a slot no squad holds (resolves in the payload, dangles on \
                 the wire)",
            "is not on the compiled roster",
            "v1",
            |p| {
                p["editor"]["slots"].as_array_mut().expect("slots").push(
                    serde_json::json!({"id": "sOrphan", "squadId": "", "index": 9,
                                           "role": "RFL",
                                           "position": {"x": 9.0, "y": 9.0, "z": 0,
                                                        "rotation": 0}}),
                );
                p["vehicles"][0]["crew"] = serde_json::json!({"driver": "sOrphan"});
            },
        ),
        (
            "seat id off the schema's station enum",
            "is not a station",
            "v1",
            |p| p["vehicles"][0]["crew"] = serde_json::json!({"turret_left": "s1"}),
        ),
        (
            "seat ordinal the panel cannot write (`cargo0`)",
            "is not a station",
            "v1",
            |p| p["vehicles"][0]["crew"] = serde_json::json!({"cargo0": "s1"}),
        ),
        (
            "two seats naming one station",
            "name the same station",
            "v1",
            |p| p["vehicles"][0]["crew"] = serde_json::json!({"cargo": "s1", "cargo1": "s2"}),
        ),
        (
            "crew occupant that is not a slot id",
            "does not name a slot",
            "v1",
            |p| p["vehicles"][0]["crew"] = serde_json::json!({"driver": 5}),
        ),
        (
            "crew that is not a seat map at all",
            "not the seat-to-slot map",
            "v1",
            |p| p["vehicles"][0]["crew"] = serde_json::json!([["driver", "s1"]]),
        ),
    ];

    for (case, clause, owner_id, mutate) in cases {
        let doc = roster_wire(*mutate);
        let wire = serde_json::to_value(&doc).expect("serialises");
        assert!(
            wire.get("vehicles").is_none(),
            "{case}: the row reached the wire instead of dropping whole: {}",
            wire["vehicles"]
        );
        assert_eq!(
            wire["schemaVersion"], "1.1",
            "{case}: nothing 1.3-shaped is on the wire, so the version must not claim 1.3"
        );
        let findings: Vec<&Finding> = doc
            .diagnostics
            .iter()
            .filter(|f| f.rule_id == DIAG_DROP_VEHICLE_ROSTER)
            .collect();
        assert_eq!(
            findings.len(),
            1,
            "{case}: expected exactly one roster finding, got {findings:?}"
        );
        assert!(
            findings[0].message.contains(clause),
            "{case}: the finding does not say why — {:?}",
            findings[0].message
        );
        assert_eq!(
            findings[0].subject_id.as_deref(),
            Some(*owner_id),
            "{case}: the finding must name the vehicle the author can click"
        );
    }
}

#[test]
fn the_roster_emit_leaves_the_entities_alias_path_alone() {
    let with_crew = roster_wire(|_| {});
    let no_crew = roster_wire(|p| {
        p["vehicles"][0]
            .as_object_mut()
            .expect("vehicle row")
            .remove("crew");
    });
    let dropped = roster_wire(|p| p["vehicles"][0]["crew"] = serde_json::json!({"driver": "x"}));

    let entities =
        |d: &ModMissionDocument| serde_json::to_value(&d.entities).expect("entities serialise");
    assert_eq!(entities(&with_crew), entities(&no_crew));
    assert_eq!(
        entities(&with_crew),
        entities(&dropped),
        "a dropped roster row changed the entities[] projection — the two paths are coupled"
    );
    assert_eq!(with_crew.entities.len(), 1);
    assert_eq!(with_crew.entities[0].alias, "veh:m151_mg");

    let wire = serde_json::to_value(&no_crew).expect("serialises");
    assert_eq!(wire["vehicles"][0]["uid"], "v1");
    assert!(wire["vehicles"][0].get("seats").is_none());
    assert!(
        no_crew.diagnostics.is_empty(),
        "an uncrewed vehicle is not a finding: {:?}",
        no_crew.diagnostics
    );
}

#[test]
fn one_authored_vehicle_emits_two_rows_that_share_a_join_key() {
    let wire = serde_json::to_value(roster_wire(|_| {})).expect("serialises");
    let ent = &wire["entities"][0];
    let veh = &wire["vehicles"][0];

    assert_eq!(ent["alias"], veh["alias"], "same vehicle, both projections");
    assert_eq!(ent["uid"], "v1", "the entity twin carries the authored id");
    assert_eq!(veh["uid"], "v1", "so does the roster row");
    assert_eq!(
        ent["uid"], veh["uid"],
        "the two rows one vehicle emits must share a key a reader can dedupe on"
    );

    let blank = serde_json::to_value(roster_wire(|p| p["vehicles"][0]["id"] = "".into()))
        .expect("serialises");
    assert!(blank["entities"][0].get("uid").is_none());
    assert!(blank["vehicles"][0].get("uid").is_none());
}

#[test]
fn the_roster_row_carries_the_cargo_its_entity_twin_carries() {
    let wire = serde_json::to_value(roster_wire(|p| {
        p["vehicles"][0]["cargo"] = serde_json::json!([
            {"item": "item:mre", "qty": 4},
            {"item": "   ", "qty": 9},
            {"item": "item:ifak", "qty": 0}
        ]);
    }))
    .expect("serialises");

    let want = serde_json::json!([{"item": "item:mre", "qty": 4}]);
    assert_eq!(
        wire["entities"][0]["inventory"], want,
        "the entity twin filters blank items and non-positive quantities"
    );
    assert_eq!(
        wire["vehicles"][0]["inventory"], want,
        "and the roster row must carry the same cargo, not a trimmed view of it"
    );

    let bare = serde_json::to_value(roster_wire(|_| {})).expect("serialises");
    assert!(bare["entities"][0].get("inventory").is_none());
    assert!(bare["vehicles"][0].get("inventory").is_none());
}

#[test]
fn seat_ids_project_onto_the_schema_s_stations() {
    assert_eq!(parse_seat_id("driver"), Some(("driver", None)));
    assert_eq!(parse_seat_id("gunner"), Some(("gunner", None)));
    assert_eq!(parse_seat_id("commander"), Some(("commander", None)));
    assert_eq!(parse_seat_id("cargo1"), Some(("cargo", Some(0))));
    assert_eq!(parse_seat_id("cargo4"), Some(("cargo", Some(3))));

    assert_eq!(parse_seat_id("pilot"), Some(("pilot", None)));
    assert_eq!(parse_seat_id("turret2"), Some(("turret", Some(1))));

    assert_eq!(parse_seat_id("  Driver "), Some(("driver", None)));

    assert_eq!(parse_seat_id("cargo"), Some(("cargo", None)));
    assert_eq!(parse_seat_id("copilot"), Some(("copilot", None)));

    for bad in [
        "",
        "  ",
        "cargo0",
        "cargo1a",
        "cargo-1",
        "cargo+1",
        "1",
        "seat1",
        "cargo99999999999",
        "co-pilot",
        "driver_1",
    ] {
        assert_eq!(parse_seat_id(bad), None, "{bad:?} must not project");
    }
}

#[test]
fn schema_version_bumps_to_1_3_only_when_an_identity_key_emits() {
    let version = |payload: &str| -> String {
        flatten_to_mod_document(&meta(), payload.as_bytes())
            .expect("compiles")
            .schema_version
    };

    assert_eq!(version(FIXTURE), "1.2");

    assert_eq!(version(DROP_FIXTURE), "1.1");

    for (key, literal) in [
        ("callsign", r#""Actual""#),
        ("rank", r#""corporal""#),
        ("stance", r#""crouch""#),
        ("unitName", r#""Cpl. Vega""#),
        ("tag", r#""SL""#),
    ] {
        let payload = STRIPPED_IDENTITY_FIXTURE.replace(
            r#""id": "s2", "squadId": "sq1", "index": 1, "role": "SL","#,
            &format!(
                r#""id": "s2", "squadId": "sq1", "index": 1, "role": "SL", "{key}": {literal},"#
            ),
        );
        assert_ne!(
            payload, STRIPPED_IDENTITY_FIXTURE,
            "{key}: the seed must change the fixture"
        );
        assert_eq!(
            version(&payload),
            "1.3",
            "{key} alone must bump the version"
        );
    }

    assert_eq!(
        version(&STRIPPED_IDENTITY_FIXTURE.replace(
            r#""slotIds": ["s1", "s2"]"#,
            r#""slotIds": ["s1", "s2"], "leaderSlotId": "s2""#,
        )),
        "1.3"
    );

    const ROSTER_SEED: &str = r#""vehicles": [{"id": "v1", "resourceName": "{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et", "position": {"x": 10.0, "y": 20.0, "z": 0, "rotation": 0}}], "editor": {"#;
    let with_roster = STRIPPED_IDENTITY_FIXTURE.replace(r#""editor": {"#, ROSTER_SEED);
    assert_ne!(
        with_roster, STRIPPED_IDENTITY_FIXTURE,
        "the roster seed must change the fixture"
    );
    assert_eq!(
        version(&with_roster),
        "1.3",
        "an emitted vehicles[] alone must bump the version"
    );

    let unplaced = with_roster.replace(
        r#", "position": {"x": 10.0, "y": 20.0, "z": 0, "rotation": 0}"#,
        "",
    );
    assert_ne!(
        unplaced, with_roster,
        "removing the position must change the seed"
    );
    assert_eq!(
        version(&unplaced),
        "1.1",
        "a roster whose every row dropped must not claim 1.3"
    );

    assert_eq!(version(STRIPPED_IDENTITY_FIXTURE), "1.1");
}
