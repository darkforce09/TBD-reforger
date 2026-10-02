//! Role: Domain regression cases.
//! Position: `mission/compiler/payload/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn save_payload_omits_orbat_and_has_editor_shape() {
    let p = compile_payload(&small_maps(), &slots(), false);
    assert!(p.get("orbat").is_none(), "Save payload must omit orbat");
    assert_eq!(p["schemaVersion"], json!(1));
    assert_eq!(p["map"]["terrain"], json!("everon"));
    assert_eq!(p["map"]["bounds"], json!([0, 0, 12800, 12800]));
    assert_eq!(p["editor"]["slots"].as_array().unwrap().len(), 3);
    assert_eq!(p["editor"]["factions"].as_array().unwrap().len(), 2);
    assert_eq!(p["editor"]["squads"].as_array().unwrap().len(), 2);
    assert_eq!(p["editor"]["editorLayers"], json!([]));
    assert!(p["loadouts"].is_object());
    assert!(p["environment"].is_object());
    assert_eq!(p["objectives"], json!([]));
    assert_eq!(p["vehicles"], json!([]));
    assert_eq!(p["entities"], json!([]));
    assert_eq!(p["markers"], json!([]));
}

#[test]
fn compile_copies_entities_by_id_to_entities_array() {
    let small = json!({
        "meta": Value::Null,
        "factionsById": {},
        "squadsById": {},
        "loadoutsById": {},
        "itemsById": {},
        "objectivesById": {},
        "vehiclesById": {},
        "entitiesById": {
            "e1": {
                "id": "e1",
                "alias": "prop:ammo_crate",
                "resourceName": "{FA}Prefabs/Props/AmmoBox.et",
                "faction": "blufor",
                "position": { "x": 10.0, "y": 20.0, "z": 0.0, "rotation": 45.0 }
            }
        },
        "markersById": {},
        "editorLayersById": {}
    })
    .to_string();
    let p = compile_payload(&small, "{}", false);
    let ents = p["entities"].as_array().expect("entities array");
    assert_eq!(ents.len(), 1);
    assert_eq!(ents[0]["alias"], "prop:ammo_crate");
    assert_eq!(ents[0]["id"], "e1");
    assert_eq!(ents[0]["position"]["x"], 10.0);
    assert_eq!(ents[0]["faction"], "blufor");
}

#[test]
fn export_orbat_is_faction_then_squad_then_index_sorted() {
    let p = compile_payload(&small_maps(), &slots(), true);

    assert_eq!(
        p["orbat"],
        json!([
            {
                "faction": "BLUFOR", "callsign": "Alpha", "squad": "1st",
                "slots": [
                    { "role": "Rifleman", "loadout": "", "tag": "" },
                    { "role": "SL",       "loadout": "", "tag": "CMD" }
                ]
            },
            {
                "faction": "OPFOR", "callsign": "Bravo", "squad": "2nd",
                "slots": [ { "role": "MED", "loadout": "", "tag": "MED" } ]
            }
        ])
    );
}

#[test]
fn compile_export_orbat_loadout() {
    let slots = json!({
        "z1": {
            "id": "z1", "index": 0, "role": "Rifleman", "tag": "",
            "loadout": {
                "primary": "{AAA}Rifle_M16A2.et",
                "summary": "M16A2 \u{00b7} ACOG"
            }
        }
    })
    .to_string();
    let small = json!({
        "meta": Value::Null,
        "factionsById": {
            "fa": { "key": "BLUFOR", "squadIds": ["s1"] }
        },
        "squadsById": {
            "s1": { "id": "s1", "callsign": "Alpha", "name": "1st", "slotIds": ["z1"] }
        },
        "loadoutsById": {},
        "itemsById": {},
        "objectivesById": {},
        "vehiclesById": {},
        "entitiesById": {},
        "markersById": {},
        "editorLayersById": {}
    })
    .to_string();
    let p = compile_payload(&small, &slots, true);
    assert_eq!(
        p["orbat"][0]["slots"][0]["loadout"],
        json!("M16A2 \u{00b7} ACOG")
    );

    let save = compile_payload(&small, &slots, false);
    assert!(save.get("orbat").is_none());
}

#[test]
fn null_meta_defaults_to_everon_and_empty_environment() {
    let p = compile_payload(r#"{"meta":null}"#, "{}", false);
    assert_eq!(p["map"]["terrain"], json!("everon"));
    assert_eq!(p["map"]["bounds"], json!([0, 0, 12800, 12800]));
    assert_eq!(p["environment"], json!({}));
    assert_eq!(p["editor"]["slots"], json!([]));
}

#[test]
fn arland_terrain_yields_4096_bounds() {
    let small = json!({ "meta": { "terrain": "arland" } }).to_string();
    let p = compile_payload(&small, "{}", false);
    assert_eq!(p["map"]["terrain"], json!("arland"));
    assert_eq!(p["map"]["bounds"], json!([0, 0, 4096, 4096]));
}

#[test]
fn version_body_shape() {
    let payload = json!({ "schemaVersion": 1 });
    let body = version_body("0.1.0", "note", &payload);
    assert_eq!(
        body,
        json!({ "semver": "0.1.0", "editor_notes": "note", "payload": { "schemaVersion": 1 } })
    );
}

#[test]
fn both_doors_onto_create_version_serialise_identical_bytes() {
    let payloads = [
        json!({ "schemaVersion": 1, "editor": { "slots": [] } }),
        json!({}),
        Value::Null,
        json!(0),
        json!([]),
        json!({ "zulu": 1, "alpha": 2, "mike": 3 }),
        json!({ "quote\"key": "line\nbreak\ttab", "unicode": "Ärland — Ω 🎖", "solidus": "a/b" }),
        json!({ "a": { "b": { "c": [1, 2, { "d": true, "e": Value::Null }] } } }),
        json!({ "f": 1.5, "neg": -0.000_25, "big": 9_007_199_254_740_993i64 }),
    ];

    let metas = [
        ("0.1.0", "note"),
        ("", ""),
        (
            "1.2.3-rc.1+build",
            "Uploaded from \"my mission\".json\nwith a newline",
        ),
        ("9.9.9", "Ω — em dash and 🎖"),
    ];

    for payload in &payloads {
        for (semver, notes) in metas {
            let via_value = serde_json::to_string(&version_body(semver, notes, payload))
                .expect("version_body's Value must serialise");
            let mut via_writer: Vec<u8> = Vec::new();
            version_body_to_writer(&mut via_writer, semver, notes, payload)
                .expect("version_body_to_writer must not fail writing to a Vec");
            let via_writer = String::from_utf8(via_writer).expect("serde_json emits valid UTF-8");

            assert_eq!(
                via_value, via_writer,
                "the two doors onto create_version must send the same bytes — they have \
                     drifted for semver={semver:?} notes={notes:?} payload={payload}"
            );
        }
    }

    let mut buf: Vec<u8> = Vec::new();
    version_body_to_writer(&mut buf, "0.1.0", "note", &json!({ "schemaVersion": 1 }))
        .expect("write");
    assert_eq!(
        String::from_utf8(buf).expect("utf8"),
        r#"{"semver":"0.1.0","editor_notes":"note","payload":{"schemaVersion":1}}"#
    );
}

#[test]
fn export_envelope_defaults_and_wraps_payload() {
    let payload = compile_payload(r#"{"meta":null}"#, "{}", true);
    let doc = compile_export(
        &payload,
        r#"{"meta":null}"#,
        "smoke",
        "0.1.0",
        "1970-01-01T00:00:00.000Z",
    );
    assert_eq!(doc["exportFormatVersion"], json!(1));
    assert_eq!(doc["missionId"], json!("smoke"));
    assert_eq!(doc["title"], json!("Untitled Mission"));
    assert_eq!(doc["terrain"], json!("everon"));
    assert_eq!(doc["weather"], json!("clear"));
    assert_eq!(doc["timeOfDay"], json!("06:00"));
    assert_eq!(doc["gameMode"], json!(""));
    assert_eq!(doc["maxPlayers"], json!(0));
    assert_eq!(doc["exportedAt"], json!("1970-01-01T00:00:00.000Z"));
    assert_eq!(doc["payload"]["orbat"], json!([]));

    assert_eq!(doc["briefing"], json!(""));
}

#[test]
fn briefing_prose_survives_compile_verbatim_per_faction() {
    let small = json!({
        "meta": Value::Null,
        "factionsById": {
            "fa": {
                "id": "fa", "key": "BLUFOR", "name": "US Army", "squadIds": ["s1"],
                "briefing": authored_briefing()
            },
            "fb": { "id": "fb", "key": "OPFOR", "name": "Soviet VDV", "squadIds": [] }
        },
        "squadsById": {
            "s1": { "id": "s1", "callsign": "Alpha", "name": "1st", "slotIds": [] }
        },
        "loadoutsById": {},
        "itemsById": {},
        "objectivesById": {},
        "vehiclesById": {},
        "entitiesById": {},
        "markersById": {},
        "editorLayersById": {}
    })
    .to_string();

    let p = compile_payload(&small, "{}", false);
    let factions = p["editor"]["factions"].as_array().expect("factions array");
    assert_eq!(factions.len(), 2);

    assert_eq!(factions[0]["briefing"], authored_briefing());

    let situation = factions[0]["briefing"]["situation"]
        .as_str()
        .expect("situation is a string");
    assert_eq!(situation, SITUATION);
    assert!(
        situation.contains("\n\n"),
        "paragraph break must survive verbatim: {situation:?}"
    );
    let execution = factions[0]["briefing"]["execution"]
        .as_str()
        .expect("execution is a string");
    assert_eq!(execution.matches('\n').count(), 3, "{execution:?}");

    assert_eq!(factions[0]["key"], json!("BLUFOR"));

    assert!(factions[1].get("briefing").is_none());

    let ex = compile_payload(&small, "{}", true);
    assert_eq!(
        ex["editor"]["factions"][0]["briefing"]["mission"],
        json!(MISSION)
    );
}

#[cfg(feature = "store")]
#[test]
fn briefing_prose_round_trips_through_the_document_core() {
    use crate::data::store::MissionDocCore;

    let payload = json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon" },
        "editor": {
            "factions": [
                {
                    "id": "fa", "key": "BLUFOR", "name": "US Army", "squadIds": ["s1"],
                    "briefing": authored_briefing()
                },
                { "id": "fb", "key": "OPFOR", "name": "Soviet VDV", "squadIds": [] }
            ],
            "squads": [
                { "id": "s1", "factionId": "fa", "callsign": "Alpha", "name": "1st",
                  "slotIds": ["z1"] }
            ],
            "slots": [
                { "id": "z1", "squadId": "s1", "index": 0, "role": "SL",
                  "position": { "x": 4839.2, "y": 6620.8, "z": 0.0, "rotation": 270.0 } }
            ],
            "editorLayers": []
        }
    })
    .to_string();

    let doc = MissionDocCore::new();
    doc.hydrate(&payload, "layer-1");

    let small = doc.small_maps_json();
    let parsed: Value = serde_json::from_str(&small).expect("small_maps_json is JSON");
    assert_eq!(
        parsed["factionsById"]["fa"]["briefing"],
        authored_briefing()
    );

    let recompiled = compile_payload(&small, &doc.slots_json(), false);
    assert_eq!(
        recompiled["editor"]["factions"][0]["briefing"],
        authored_briefing()
    );

    let situation = recompiled["editor"]["factions"][0]["briefing"]["situation"]
        .as_str()
        .expect("situation is a string");
    assert_eq!(situation, SITUATION);
    assert!(situation.contains("\n\n"), "{situation:?}");

    assert_eq!(recompiled["editor"]["slots"].as_array().unwrap().len(), 1);
    assert_eq!(recompiled["map"]["terrain"], json!("everon"));
}

#[test]
fn payload_extras_merge_does_not_overwrite_known_keys() {
    let small = json!({
        "meta": { "terrain": "everon" },
        "factionsById": {},
        "squadsById": {},
        "loadoutsById": {},
        "itemsById": {},
        "objectivesById": {},
        "vehiclesById": {},
        "entitiesById": {},
        "markersById": {},
        "editorLayersById": {},
        "payloadExtras": {
            "schemaVersion": 99,
            "serverMigrationToken": "keep-me",
            "map": { "terrain": "arland" }
        }
    })
    .to_string();

    let p = compile_payload(&small, "{}", false);
    assert_eq!(
        p["schemaVersion"],
        json!(1),
        "known key must win over extras"
    );
    assert_eq!(p["map"]["terrain"], json!("everon"));
    assert_eq!(p["serverMigrationToken"], json!("keep-me"));
    assert!(p.get("payloadExtras").is_none());
}

#[test]
fn settings_in_payload_extras_reach_the_wire_payload() {
    let small = json!({
        "meta": { "terrain": "everon" },
        "factionsById": {},
        "squadsById": {},
        "loadoutsById": {},
        "itemsById": {},
        "objectivesById": {},
        "vehiclesById": {},
        "entitiesById": {},
        "markersById": {},
        "editorLayersById": {},
        "payloadExtras": {
            "settings": {
                "respawn": "wave",
                "spectatorPolicy": "free",
                "nightVision": false
            }
        }
    })
    .to_string();

    let p = compile_payload(&small, "{}", false);
    let s = p
        .get("settings")
        .expect("settings must leave payloadExtras onto the wire");
    assert_eq!(s["respawn"], "wave");
    assert_eq!(s["spectatorPolicy"], "free");
    assert_eq!(s["nightVision"], false);
    assert!(p.get("payloadExtras").is_none());
}

#[test]
fn payload_extras_key_name_never_promoted_onto_wire() {
    let small = json!({
        "meta": { "terrain": "everon" },
        "factionsById": {},
        "squadsById": {},
        "loadoutsById": {},
        "itemsById": {},
        "objectivesById": {},
        "vehiclesById": {},
        "entitiesById": {},
        "markersById": {},
        "editorLayersById": {},
        "payloadExtras": {
            "payloadExtras": { "nested": true },
            "serverMigrationToken": "keep-me"
        }
    })
    .to_string();

    let p = compile_payload(&small, "{}", false);
    assert!(
        p.get("payloadExtras").is_none(),
        "side-channel name must never become a wire key; got {:?}",
        p.get("payloadExtras")
    );
    assert_eq!(
        p["serverMigrationToken"],
        json!("keep-me"),
        "unrelated parked keys must still re-emit"
    );
}

#[test]
fn authored_schema_version_on_meta_is_emitted() {
    let small = json!({
        "meta": { "terrain": "everon", "schemaVersion": 2 },
        "factionsById": {},
        "squadsById": {},
        "loadoutsById": {},
        "itemsById": {},
        "objectivesById": {},
        "vehiclesById": {},
        "entitiesById": {},
        "markersById": {},
        "editorLayersById": {}
    })
    .to_string();
    let p = compile_payload(&small, "{}", false);
    assert_eq!(p["schemaVersion"], json!(2));
}

#[test]
fn authored_map_keys_and_bounds_survive_compile() {
    let small = json!({
        "meta": {
            "terrain": "everon",
            "map": {
                "terrain": "everon",
                "bounds": [10, 20, 30, 40],
                "center": [6400.5, 6400.25],
                "label": "ops-sector"
            }
        },
        "factionsById": {},
        "squadsById": {},
        "loadoutsById": {},
        "itemsById": {},
        "objectivesById": {},
        "vehiclesById": {},
        "entitiesById": {},
        "markersById": {},
        "editorLayersById": {}
    })
    .to_string();
    let p = compile_payload(&small, "{}", false);
    assert_eq!(p["map"]["bounds"], json!([10, 20, 30, 40]));
    assert_eq!(p["map"]["center"], json!([6400.5, 6400.25]));
    assert_eq!(p["map"]["label"], json!("ops-sector"));
    assert_eq!(p["map"]["terrain"], json!("everon"));
}

#[test]
fn export_envelope_briefing_is_the_row_blurb_not_the_faction_block() {
    let small = json!({
        "meta": {
            "title": "Bridgehead at Levie",
            "briefing": "A short library blurb for the mission card."
        },
        "factionsById": {
            "fa": {
                "id": "fa", "key": "BLUFOR", "squadIds": [],
                "briefing": authored_briefing()
            }
        },
        "squadsById": {},
        "loadoutsById": {},
        "itemsById": {},
        "objectivesById": {},
        "vehiclesById": {},
        "entitiesById": {},
        "markersById": {},
        "editorLayersById": {}
    })
    .to_string();

    let payload = compile_payload(&small, "{}", true);
    let doc = compile_export(
        &payload,
        &small,
        "smoke",
        "0.2.0",
        "1970-01-01T00:00:00.000Z",
    );

    assert!(doc["briefing"].is_string());
    assert_eq!(
        doc["briefing"],
        json!("A short library blurb for the mission card.")
    );

    assert_eq!(
        doc["payload"]["editor"]["factions"][0]["briefing"]["situation"],
        json!(SITUATION)
    );

    assert!(doc.get("briefings").is_none());
    assert!(!doc["briefing"].is_object());
}

#[test]
fn compile_payload_includes_title_when_doc_has_one() {
    let small = json!({
        "meta": { "title": "Bridgehead at Levie", "terrain": "everon" },
        "factionsById": {},
        "squadsById": {},
        "loadoutsById": {},
        "itemsById": {},
        "objectivesById": {},
        "vehiclesById": {},
        "entitiesById": {},
        "markersById": {},
        "editorLayersById": {}
    })
    .to_string();

    let save = compile_payload(&small, "{}", false);
    assert_eq!(save["title"], json!("Bridgehead at Levie"));
    assert!(save.get("orbat").is_none());

    let export_payload = compile_payload(&small, "{}", true);
    assert_eq!(export_payload["title"], json!("Bridgehead at Levie"));

    let doc = compile_export(
        &export_payload,
        &small,
        "smoke",
        "0.1.0",
        "1970-01-01T00:00:00.000Z",
    );
    assert_eq!(doc["title"], json!("Bridgehead at Levie"));
}

#[test]
fn compile_payload_omits_blank_or_whitespace_title() {
    for raw in ["", "   ", "\t\n"] {
        let small = json!({ "meta": { "title": raw, "terrain": "everon" } }).to_string();
        let p = compile_payload(&small, "{}", false);
        assert!(
            p.get("title").is_none(),
            "whitespace-only title {raw:?} must not appear on the Save payload; got {:?}",
            p.get("title")
        );
        let doc = compile_export(&p, &small, "smoke", "0.1.0", "1970-01-01T00:00:00.000Z");
        assert_eq!(doc["title"], json!("Untitled Mission"));
    }

    let padded = json!({ "meta": { "title": "  Op Red Dawn  " } }).to_string();
    let p = compile_payload(&padded, "{}", false);
    assert_eq!(p["title"], json!("Op Red Dawn"));
}

#[test]
fn editor_only_and_transitional_keys_stay_absent_from_compile_known_list() {
    for key in [
        "zones",
        "compositions",
        "triggers",
        "comments",
        "connections",
    ] {
        assert!(
            !is_known_editor_payload_top_level(key),
            "T-751 — `{key}` must stay OUT of KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS; see                  store.rs is_known_editor_payload_top_level notes (T-211 / T-651 / T-672)"
        );
        assert!(
            !KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS.contains(&key),
            "T-751 — const list must omit `{key}` (helper alone is not enough)"
        );
    }
}

#[test]
fn title_is_known_editor_payload_top_level_key() {
    assert!(
        is_known_editor_payload_top_level("title"),
        "T-524 — `title` missing from KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS; lockstep with store.rs broken"
    );
    assert!(
        KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS.contains(&"title"),
        "T-524 — const list must include `title` (helper alone is not enough)"
    );
}

#[test]
fn title_in_payload_extras_is_not_re_emitted() {
    let small = json!({
        "meta": { "terrain": "everon" },
        "payloadExtras": {
            "title": "Should Not Leak From Extras",
            "serverMigrationToken": "keep-me"
        }
    })
    .to_string();
    let p = compile_payload(&small, "{}", false);
    assert!(
        p.get("title").is_none(),
        "known-key title in extras must not reach the wire; got {:?}",
        p.get("title")
    );
    assert_eq!(p["serverMigrationToken"], json!("keep-me"));

    let with_meta = json!({
        "meta": { "title": "Authored", "terrain": "everon" },
        "payloadExtras": { "title": "Extras Must Lose" }
    })
    .to_string();
    let p2 = compile_payload(&with_meta, "{}", false);
    assert_eq!(p2["title"], json!("Authored"));
}

#[test]
fn an_authored_block_rides_the_environment_bag_and_lands_at_the_payload_root() {
    let block = json!({"mode": "vip", "endOn": ["faction_eliminated"], "vipSlotId": "s-12"});
    let small = json!({
        "meta": {
            "terrain": "everon",
            "environment": { "weather": "clear", "winConditions": block.clone() }
        }
    })
    .to_string();
    let p = compile_payload(&small, "{}", false);
    assert_eq!(p["winConditions"], block, "carried verbatim to the root");
    assert_eq!(
        p["environment"]["winConditions"], block,
        "the bag reaches the wire unchanged — that is the reload path"
    );
    assert_eq!(p["environment"]["weather"], json!("clear"));

    let ex = compile_payload(&small, "{}", true);
    assert_eq!(ex["winConditions"], block);
}

#[test]
fn an_unlisted_environment_key_is_not_promoted_to_the_payload_root() {
    let small = json!({
        "meta": {
            "terrain": "everon",
            "environment": { "weather": "clear", "notAnAuthoredBlock": [] }
        }
    })
    .to_string();
    let p = compile_payload(&small, "{}", false);
    assert!(
        p.get("notAnAuthoredBlock").is_none(),
        "`notAnAuthoredBlock` has no AUTHORED_BLOCKS row and must not be promoted: {p}"
    );

    assert_eq!(
        p["environment"],
        json!({"weather": "clear", "notAnAuthoredBlock": []})
    );
}

#[test]
fn a_live_authored_block_wins_over_a_stale_parked_one() {
    let live = json!({"mode": "vip", "endOn": ["time_limit"], "vipSlotId": "live"});
    let stale = json!({"mode": "attrition", "endOn": ["time_limit"]});
    let small = json!({
        "meta": { "terrain": "everon", "environment": { "winConditions": live.clone() } },
        "payloadExtras": { "winConditions": stale }
    })
    .to_string();
    let p = compile_payload(&small, "{}", false);
    assert_eq!(p["winConditions"], live);
}

#[test]
fn a_cleared_authored_block_stays_cleared_against_a_stale_parked_copy() {
    let stale = json!({"mode": "vip", "endOn": ["time_limit"], "vipSlotId": "ghost"});

    for bag in [
        json!({ "weather": "clear" }),
        json!({ "weather": "clear", "winConditions": serde_json::Value::Null }),
    ] {
        let small = json!({
            "meta": { "terrain": "everon", "environment": bag },
            "payloadExtras": { "winConditions": stale.clone() }
        })
        .to_string();
        let p = compile_payload(&small, "{}", false);
        assert!(
            p.get("winConditions").is_none(),
            "a deleted win rule came back from payloadExtras: {p}"
        );
    }
}
