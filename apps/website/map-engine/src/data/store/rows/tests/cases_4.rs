//! Role: Domain regression cases.
//! Position: `doc/store/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[cfg(feature = "scenario")]
#[test]
fn t826_lazy_mint_promotes_pending_markers_and_v1_fires_without_slots() {
    let doc = MissionDocCore::new();
    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 10.0, 20.0, "objective", "OBJ");
    doc.add_faction("faction-BLUFOR", "BLUFOR", "BLUFOR");

    let rows = marker_rows(&doc);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(
        small_maps(&doc)["factionsById"]["faction-BLUFOR"]["briefing"]["markers"][0]["id"],
        serde_json::json!("mk-1"),
        "parked marker must promote onto the faction briefing"
    );

    assert!(
        small_maps(&doc)["meta"]
            .get("pendingBriefingMarkers")
            .is_none(),
        "pending must clear on mint: {:?}",
        small_maps(&doc)["meta"]
    );

    let payload = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    let findings = crate::data::scenario::validate::validate_editor_payload(&payload);
    assert!(
        findings.iter().any(|f| f.rule_id == "V1-PLAYER-SPAWN"),
        "minted faction with no slots must trip V1: {findings:?}"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn a_marker_in_the_root_map_never_reaches_the_compiled_document() {
    let payload = serde_json::json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon" },

        "markers": [{ "id": "root-1", "x": 11.0, "z": 22.0, "icon": "dot", "label": "ROOT" }],
        "editor": {
            "factions": [{ "id": "faction-BLUFOR", "key": "BLUFOR", "name": "US Army",
                           "squadIds": ["sq-a"] }],
            "squads": [{ "id": "sq-a", "factionId": "faction-BLUFOR", "name": "1st",
                         "slotIds": ["z1"] }],
            "slots": [{ "id": "z1", "squadId": "sq-a", "index": 0, "role": "SL",
                        "position": { "x": 1.0, "y": 2.0, "z": 0.0, "rotation": 0.0 } }],
            "editorLayers": []
        }
    })
    .to_string();

    let doc = MissionDocCore::new();
    doc.hydrate(&payload, "lyr");

    assert!(
        small_maps(&doc)["markersById"]
            .as_object()
            .is_some_and(|m| !m.is_empty()),
        "the root map hydrated: {:?}",
        small_maps(&doc)["markersById"]
    );

    assert!(
        marker_rows(&doc).is_empty(),
        "the root map is not an authoring surface, so it is not listed"
    );

    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 33.0, 44.0, "objective", "OBJ");
    assert_eq!(marker_rows(&doc).len(), 1);

    let compiled_payload = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    let meta = crate::data::scenario::flatten::MissionMeta {
        id: "4c7e1b08-9a35-4d62-b1f7-e30d5a86c941".into(),
        title: "Bridgehead at Levie".into(),
        author: "184472930165846017".into(),
        terrain: "everon".into(),
        custom_terrain_name: String::new(),
        max_players: 12,
        time_of_day: "06:15".into(),
        weather_preset: "overcast".into(),
    };
    let compiled = crate::data::scenario::flatten::flatten_to_mod_document(
        &meta,
        &serde_json::to_vec(&compiled_payload).expect("payload serialises"),
    )
    .expect("the fixture has a slot, so the compile must succeed");
    let doc_json = serde_json::to_value(&compiled).expect("mod document serialises");

    let rows = doc_json["briefings"]["blufor"]["markers"]
        .as_array()
        .expect("blufor briefing carries markers");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["label"], serde_json::json!("OBJ"));

    assert!(
        doc_json.get("markers").is_none(),
        "the compiled document has no top-level `markers`: {doc_json:?}"
    );
    assert!(
        !serde_json::to_string(&doc_json)
            .expect("serialises")
            .contains("ROOT"),
        "nothing from the root map may appear in the compiled document"
    );
}

#[test]
fn authored_prose_round_trips_through_compile_and_hydrate() {
    let doc = briefing_fixture();
    doc.set_faction_briefing(
        "faction-BLUFOR",
        SITUATION,
        "Seize and hold the crossing until relieved.",
        "Alpha leads, Bravo screens the north flank.",
    );

    assert_eq!(
        prose_of(&doc, "faction-BLUFOR", "situation"),
        serde_json::json!(SITUATION),
        "the authored prose must be stored verbatim"
    );
    assert!(
        prose_of(&doc, "faction-BLUFOR", "situation")
            .as_str()
            .expect("situation is a string")
            .contains("\n\n"),
        "the fixture must actually carry a paragraph break, or this test proves nothing"
    );

    assert!(
        small_maps(&doc)["factionsById"]["faction-OPFOR"]
            .get("briefing")
            .is_none(),
        "the unauthored side must not acquire a briefing"
    );

    let payload = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    let reloaded = MissionDocCore::new();
    reloaded.hydrate(&payload.to_string(), "lyr");

    assert_eq!(
        prose_of(&reloaded, "faction-BLUFOR", "situation"),
        serde_json::json!(SITUATION),
        "every newline must survive hydrate verbatim"
    );
    assert_eq!(
        prose_of(&reloaded, "faction-BLUFOR", "mission"),
        serde_json::json!("Seize and hold the crossing until relieved.")
    );
    assert_eq!(
        prose_of(&reloaded, "faction-BLUFOR", "execution"),
        serde_json::json!("Alpha leads, Bravo screens the north flank.")
    );

    let recompiled = crate::data::scenario::compile::compile_payload(
        &reloaded.small_maps_json(),
        &reloaded.slots_json(),
        false,
    );
    assert_eq!(
        recompiled["editor"]["factions"][0]["briefing"],
        payload["editor"]["factions"][0]["briefing"]
    );

    assert_eq!(
        recompiled["editor"]["slots"]
            .as_array()
            .expect("slots")
            .len(),
        1
    );
}

#[test]
fn prose_and_markers_do_not_eat_each_other() {
    let doc = briefing_fixture();
    doc.set_faction_briefing_marker(
        "faction-BLUFOR",
        "mk-1",
        4870.25,
        7760.5,
        "objective",
        "OBJ",
    );
    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-2", 300.5, 400.5, "hazard", "MINES");

    doc.set_faction_briefing("faction-BLUFOR", SITUATION, "Hold.", "Alpha leads.");

    let rows = markers_of(&doc, "faction-BLUFOR");
    let ids: Vec<&str> = rows
        .iter()
        .map(|m| m["id"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        ids,
        vec!["mk-1", "mk-2"],
        "a prose edit must not delete markers: {rows:?}"
    );
    assert_eq!(marker_num(&rows[0], "x"), 4870.25, "marker payload intact");
    assert_eq!(
        prose_of(&doc, "faction-BLUFOR", "situation"),
        serde_json::json!(SITUATION)
    );

    let doc2 = briefing_fixture();
    doc2.set_faction_briefing("faction-BLUFOR", SITUATION, "Hold.", "Alpha leads.");
    doc2.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 111.5, 222.5, "rally", "RP");
    doc2.remove_faction_briefing_marker("faction-BLUFOR", "mk-1");

    assert_eq!(
        prose_of(&doc2, "faction-BLUFOR", "situation"),
        serde_json::json!(SITUATION),
        "a marker add+remove must not touch the prose"
    );
    assert_eq!(
        prose_of(&doc2, "faction-BLUFOR", "mission"),
        serde_json::json!("Hold.")
    );
    assert_eq!(
        prose_of(&doc2, "faction-BLUFOR", "execution"),
        serde_json::json!("Alpha leads.")
    );
}

#[test]
fn setting_all_empty_prose_on_an_unauthored_faction_does_not_mint_a_briefing() {
    let doc = briefing_fixture();
    let before = small_maps(&doc);

    doc.set_faction_briefing("faction-OPFOR", "", "", "");
    doc.set_faction_briefing("faction-does-not-exist", "", "", "");

    doc.set_faction_briefing("faction-does-not-exist", SITUATION, "Hold.", "Go.");

    assert!(
        small_maps(&doc)["factionsById"]["faction-OPFOR"]
            .get("briefing")
            .is_none(),
        "an all-empty set must not author a briefing"
    );

    assert_eq!(
        small_maps(&doc),
        before,
        "a no-op prose write must write nothing"
    );
    assert!(
        !doc.can_undo(),
        "a no-op prose write must not stack an undo step"
    );

    doc.set_faction_briefing("faction-BLUFOR", SITUATION, "Hold.", "Go.");
    assert_eq!(doc.undo_depth(), 1, "the real edit is one step");
    doc.set_faction_briefing("faction-BLUFOR", SITUATION, "Hold.", "Go.");
    assert_eq!(
        doc.undo_depth(),
        1,
        "re-setting identical prose must not stack a second step"
    );
}

#[test]
fn clearing_a_prose_field_returns_it_to_unauthored() {
    let doc = briefing_fixture();
    doc.set_faction_briefing(
        "faction-BLUFOR",
        SITUATION,
        "Hold the crossing.",
        "Alpha leads.",
    );

    doc.set_faction_briefing("faction-BLUFOR", SITUATION, "Hold the crossing.", "");
    let briefing = &small_maps(&doc)["factionsById"]["faction-BLUFOR"]["briefing"];
    assert!(
        briefing.get("execution").is_none(),
        "a cleared field is removed, not blanked to \"\": {briefing:?}"
    );
    assert_eq!(briefing["situation"], serde_json::json!(SITUATION));
    assert_eq!(briefing["mission"], serde_json::json!("Hold the crossing."));

    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 111.5, 222.5, "rally", "RP");
    doc.set_faction_briefing("faction-BLUFOR", "", "", "");

    let briefing = &small_maps(&doc)["factionsById"]["faction-BLUFOR"]["briefing"];
    assert!(
        briefing.get("situation").is_none() && briefing.get("mission").is_none(),
        "clearing every box must land when the briefing really exists: {briefing:?}"
    );
    assert_eq!(
        markers_of(&doc, "faction-BLUFOR").len(),
        1,
        "clearing the prose must not take the marker with it"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn authored_prose_reaches_the_mod_document_keyed_by_faction_slug() {
    let doc = briefing_fixture();
    doc.set_faction_briefing(
        "faction-BLUFOR",
        SITUATION,
        "Seize and hold the crossing until relieved.",
        "Alpha leads, Bravo screens the north flank.",
    );

    let payload = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    let meta = crate::data::scenario::flatten::MissionMeta {
        id: "4c7e1b08-9a35-4d62-b1f7-e30d5a86c941".into(),
        title: "Bridgehead at Levie".into(),
        author: "184472930165846017".into(),
        terrain: "everon".into(),
        custom_terrain_name: String::new(),
        max_players: 12,
        time_of_day: "06:15".into(),
        weather_preset: "overcast".into(),
    };
    let compiled = crate::data::scenario::flatten::flatten_to_mod_document(
        &meta,
        &serde_json::to_vec(&payload).expect("payload serialises"),
    )
    .expect("the fixture has a slot, so the compile must succeed");
    let doc_json = serde_json::to_value(&compiled).expect("mod document serialises");

    let b = doc_json["briefings"]["blufor"]
        .as_object()
        .expect("blufor briefing");
    assert_eq!(
        b["situation"],
        serde_json::json!(SITUATION),
        "the paragraph breaks must reach the compiled document verbatim"
    );
    assert_eq!(
        b["mission"],
        serde_json::json!("Seize and hold the crossing until relieved.")
    );
    assert_eq!(
        b["execution"],
        serde_json::json!("Alpha leads, Bravo screens the north flank.")
    );

    assert_eq!(b.len(), 3, "exactly the three authored fields: {b:?}");

    assert!(
        doc_json["briefings"].get("opfor").is_none(),
        "{:?}",
        doc_json["briefings"]
    );
}

#[cfg(feature = "scenario")]
#[test]
fn authored_zones_survive_compile_hydrate_compile_whole() {
    let doc = zones_fixture();

    let compiled = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    let first = wire_zones(&compiled);
    assert_eq!(
        first.len(),
        2,
        "both authored zones must reach the wire payload: {compiled}"
    );

    let expect_ao = serde_json::json!({
        "id": "z_ao",
        "type": "boundary",
        "label": "Area of Operations",
        "shape": { "polygon": [
            [1000.25, -4210.75],
            [1600.5,  -4210.75],
            [1600.5,  -3800.125],
            [1000.25, -3800.125]
        ]},
        "rules": { "graceSeconds": 45.5, "penalty": "kill", "warnEverySeconds": 7.25 }
    });
    let expect_obj = serde_json::json!({
        "id": "z_obj",
        "type": "objective_capture",
        "faction": "blufor",
        "shape": { "circle": { "x": 1234.5, "z": -3990.25, "r": 175.75 } },
        "rules": { "captureSeconds": 180.5 }
    });

    let by_id = |rows: &[serde_json::Value], id: &str| -> serde_json::Value {
        rows.iter()
            .find(|r| r["id"] == id)
            .unwrap_or_else(|| panic!("zone {id} missing from {rows:?}"))
            .clone()
    };

    assert_eq!(
        by_id(&first, "z_ao"),
        expect_ao,
        "compile #1 dropped part of z_ao"
    );
    assert_eq!(
        by_id(&first, "z_obj"),
        expect_obj,
        "compile #1 dropped part of z_obj"
    );

    let reloaded = save_and_reload(&doc);
    assert_eq!(reloaded.zone_count(), 2, "hydrate must restore both zones");

    let recompiled = crate::data::scenario::compile::compile_payload(
        &reloaded.small_maps_json(),
        &reloaded.slots_json(),
        false,
    );
    let second = wire_zones(&recompiled);
    assert_eq!(
        by_id(&second, "z_ao"),
        expect_ao,
        "z_ao did not survive save→reload→save WHOLE"
    );
    assert_eq!(
        by_id(&second, "z_obj"),
        expect_obj,
        "z_obj did not survive save→reload→save WHOLE"
    );

    assert!(
        recompiled.get("payloadExtras").is_none(),
        "payloadExtras must not reach the wire: {recompiled}"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn authored_zone_rows_use_only_declared_schema_keys() {
    const ZONE_KEYS: &[&str] = &["id", "type", "shape", "label", "faction", "rules"];

    const ZONE_TYPES: &[&str] = &[
        "spawn",
        "objective_capture",
        "objective_destroy",
        "objective_hold_until",
        "boundary",
        "base_protection",
    ];

    const RULE_KEYS: &[&str] = &[
        "graceSeconds",
        "warnEverySeconds",
        "penalty",
        "captureSeconds",
        "neutralizeSeconds",
        "contestable",
        "onEmpty",
        "decayRate",
        "holdSeconds",
        "pauseOnEnemy",
        "resetOnEnemy",
        "requireHolderPresent",
        "targetAlias",
        "targetCount",
        "points",
        "announceEverySeconds",
    ];

    let doc = zones_fixture();
    let compiled = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    let rows = wire_zones(&compiled);
    assert!(!rows.is_empty(), "fixture authored no zones");

    for row in &rows {
        let obj = row.as_object().expect("zone row is an object");
        for k in obj.keys() {
            assert!(
                ZONE_KEYS.contains(&k.as_str()),
                "undeclared zone key `{k}` — `$defs/zone` is additionalProperties:false, so \
                     this row cannot validate: {row}"
            );
        }

        for k in ["id", "type", "shape"] {
            assert!(obj.contains_key(k), "zone missing required `{k}`: {row}");
        }
        assert!(
            ZONE_TYPES.contains(&row["type"].as_str().unwrap_or_default()),
            "zone type outside the declared enum: {row}"
        );

        let shape = row["shape"].as_object().expect("shape object");
        let has_circle = shape.contains_key("circle");
        let has_polygon = shape.contains_key("polygon");
        assert!(
            has_circle ^ has_polygon,
            "`$defs/shape` is oneOf(circle|polygon) — this row satisfies {} branches: {row}",
            usize::from(has_circle) + usize::from(has_polygon)
        );
        assert_eq!(shape.len(), 1, "shape carries an undeclared sibling: {row}");

        if has_circle {
            let c = shape["circle"].as_object().expect("circle object");
            let mut ks: Vec<&str> = c.keys().map(String::as_str).collect();
            ks.sort_unstable();
            assert_eq!(ks, ["r", "x", "z"], "circle is closed to x/z/r: {row}");
            assert!(
                c["r"].as_f64().unwrap_or_default() > 0.0,
                "circle.r is exclusiveMinimum 0: {row}"
            );
        } else {
            let ring = shape["polygon"].as_array().expect("polygon array");
            assert!(ring.len() >= 3, "polygon minItems is 3: {row}");
            for p in ring {
                let pt = p.as_array().expect("polygon vertex is an array");
                assert_eq!(pt.len(), 2, "vertex is min/maxItems 2: {row}");
                assert!(
                    pt.iter().all(serde_json::Value::is_number),
                    "vertex is numeric: {row}"
                );
            }
        }

        if let Some(rules) = row.get("rules") {
            let r = rules.as_object().expect("rules object");
            assert!(
                !r.is_empty(),
                "an empty `rules` must be omitted, not written: {row}"
            );
            for k in r.keys() {
                assert!(
                    RULE_KEYS.contains(&k.as_str()),
                    "`{k}` is outside T-241's closed zoneRules vocabulary — \
                         additionalProperties:false would reject this document: {row}"
                );
            }
        }
    }
}

#[cfg(feature = "scenario")]
#[test]
fn zones_by_id_and_extras_projection_agree_on_order() {
    let incoming = serde_json::json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon" },
        "environment": {},
        "zones": [
            { "id": "z_c", "type": "boundary",  "shape": { "circle": { "x": 3.5, "z": 4.5, "r": 5.5 } } },
            { "id": "z_a", "type": "spawn",     "shape": { "circle": { "x": 1.5, "z": 2.5, "r": 6.5 } } },
            { "id": "z_b", "type": "base_protection", "shape": { "circle": { "x": 7.5, "z": 8.5, "r": 9.5 } } }
        ],
        "editor": { "factions": [], "squads": [], "slots": [], "editorLayers": [] }
    });
    let doc = MissionDocCore::new();
    doc.hydrate(&incoming.to_string(), "lyr");

    let small = small_maps(&doc);
    let projected: Vec<&str> = small["payloadExtras"]["zones"]
        .as_array()
        .expect("projected zones array")
        .iter()
        .map(|z| z["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        projected,
        ["z_c", "z_a", "z_b"],
        "the projection must replay hydrate's authored order, not map order"
    );

    for id in &projected {
        assert_eq!(
            small["zonesById"][id],
            *small["payloadExtras"]["zones"]
                .as_array()
                .expect("arr")
                .iter()
                .find(|z| z["id"] == *id)
                .expect("row"),
            "zonesById and the projection disagree about {id}"
        );
    }
    assert_eq!(
        small["zonesById"].as_object().expect("zonesById").len(),
        3,
        "canonical by-id emit is missing rows"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn deleting_every_zone_clears_them_from_the_wire() {
    let doc = zones_fixture();
    let reloaded = save_and_reload(&doc);
    assert_eq!(reloaded.zone_count(), 2);

    reloaded.remove_zone("z_ao");
    reloaded.remove_zone("z_obj");
    assert_eq!(reloaded.zone_count(), 0);

    let compiled = crate::data::scenario::compile::compile_payload(
        &reloaded.small_maps_json(),
        &reloaded.slots_json(),
        false,
    );
    assert!(
        wire_zones(&compiled).is_empty(),
        "deleted zones must not survive on the wire: {compiled}"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn authored_zones_reach_the_mod_document_through_flatten() {
    use crate::data::scenario::flatten::{ModZoneShape, flatten_to_mod_document};

    let doc = zones_fixture();
    let compiled = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    let bytes = serde_json::to_vec(&compiled).expect("serialise payload");

    let meta = crate::data::scenario::flatten::MissionMeta {
        id: "11112222333344445555666677778888".into(),
        title: "T-211 zones".into(),
        author: "maker".into(),
        terrain: "everon".into(),
        custom_terrain_name: String::new(),
        max_players: 64,
        time_of_day: "05:30".into(),
        weather_preset: "clear".into(),
    };
    let mod_doc = flatten_to_mod_document(&meta, &bytes).expect("mission compiles");

    let ao = mod_doc
        .zones
        .iter()
        .find(|z| z.id == "z_ao")
        .expect("authored boundary zone never reached the mod document");
    assert_eq!(ao.kind, "boundary");
    assert_eq!(ao.label, "Area of Operations");
    match &ao.shape {
        ModZoneShape::Polygon { polygon } => assert_eq!(
            polygon,
            &vec![
                [1000.3, -4210.8],
                [1600.5, -4210.8],
                [1600.5, -3800.1],
                [1000.3, -3800.1],
            ],
            "the polygon reached the mod with different vertices than were drawn \
                 (expected only `round_coord`'s 0.1 m quantisation)"
        ),
        other => panic!("boundary zone lost its polygon on the way to the mod: {other:?}"),
    }
    assert_eq!(
        ao.rules.as_ref().expect("rules reached the mod")["penalty"],
        serde_json::json!("kill"),
        "zoneRules must pass through flatten verbatim"
    );

    let obj = mod_doc
        .zones
        .iter()
        .find(|z| z.id == "z_obj")
        .expect("authored objective zone never reached the mod document");
    assert_eq!(obj.kind, "objective_capture");
    assert_eq!(obj.faction, "blufor");
    match &obj.shape {
        ModZoneShape::Circle { circle } => {
            assert_eq!(circle.x, 1234.5, "x was exact at 0.1 m already");
            assert_eq!(circle.z, -3990.3, "z quantised from -3990.25");
            assert_eq!(circle.r, 175.8, "r quantised from 175.75");
        }
        other => panic!("objective zone lost its circle on the way to the mod: {other:?}"),
    }

    assert!(
        !mod_doc.zones.iter().any(|z| z.id == "z_bounds"),
        "an authored boundary must suppress the synthesised terrain fallback"
    );
}

#[test]
fn reshaping_a_zone_leaves_exactly_one_oneof_branch() {
    let doc = MissionDocCore::new();
    doc.add_polygon_zone("z", "boundary", &[0.5, 0.5, 10.5, 0.5, 10.5, 10.5]);
    doc.set_zone_circle("z", 50.5, 60.5, 25.5);

    let zones: serde_json::Value = serde_json::from_str(&doc.zones_json()).expect("zones_json");
    let shape = zones["z"]["shape"].as_object().expect("shape");
    assert!(
        shape.contains_key("circle"),
        "reshape did not take: {shape:?}"
    );
    assert!(
        !shape.contains_key("polygon"),
        "the polygon branch survived a reshape to circle — row is oneOf-invalid: {shape:?}"
    );

    doc.set_zone_polygon("z", &[1.5, 1.5, 2.5, 1.5, 2.5, 2.5]);
    let zones: serde_json::Value = serde_json::from_str(&doc.zones_json()).expect("zones_json");
    let shape = zones["z"]["shape"].as_object().expect("shape");
    assert!(shape.contains_key("polygon"));
    assert!(
        !shape.contains_key("circle"),
        "the circle branch survived a reshape to polygon: {shape:?}"
    );
}

#[test]
fn clearing_optional_zone_fields_removes_the_keys() {
    let doc = MissionDocCore::new();
    doc.add_circle_zone("z", "objective_hold_until", 5.5, 6.5, 7.5);
    doc.set_zone_label("z", Some("Hill 402"));
    doc.set_zone_faction("z", Some("opfor"));
    doc.set_zone_rules("z", Some(r#"{"holdSeconds":600.5}"#));

    let row = |d: &MissionDocCore| -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(&d.zones_json()).expect("zones_json")["z"].clone()
    };
    let r = row(&doc);
    assert_eq!(r["label"], "Hill 402");
    assert_eq!(r["faction"], "opfor");
    assert_eq!(r["rules"]["holdSeconds"], 600.5);

    doc.set_zone_label("z", None);
    doc.set_zone_faction("z", None);
    doc.set_zone_rules("z", None);
    let r = row(&doc);
    assert!(r.get("label").is_none(), "label not removed: {r}");
    assert!(r.get("faction").is_none(), "faction not removed: {r}");
    assert!(r.get("rules").is_none(), "rules not removed: {r}");

    doc.set_zone_rules("z", Some("{}"));
    assert!(
        row(&doc).get("rules").is_none(),
        "empty rules must not be written"
    );

    doc.set_zone_rules("z", Some(r#"{"holdSeconds":1.5}"#));
    doc.set_zone_rules("z", Some("not json"));
    assert!(
        row(&doc).get("rules").is_none(),
        "malformed rules must clear the key"
    );

    doc.set_zone_label("z", Some(""));
    assert_eq!(
        row(&doc)["label"],
        "",
        "empty label must be a writable state"
    );
}

#[test]
fn zone_edits_are_undoable_and_hydrate_is_not() {
    let mut doc = MissionDocCore::new();
    doc.add_circle_zone("z1", "boundary", 1.5, 2.5, 3.5);
    assert_eq!(doc.zone_count(), 1);
    assert!(doc.can_undo(), "a drawn zone must be undoable");
    assert!(doc.undo());
    assert_eq!(doc.zone_count(), 0, "undo did not remove the zone");
    assert!(doc.redo());
    assert_eq!(doc.zone_count(), 1, "redo did not restore the zone");

    let fresh = MissionDocCore::new();
    fresh.set_origin_init(true);
    fresh.hydrate(
        &serde_json::json!({
            "zones": [ { "id": "z", "type": "spawn",
                         "shape": { "circle": { "x": 1.5, "z": 2.5, "r": 3.5 } } } ],
            "editor": { "factions": [], "squads": [], "slots": [], "editorLayers": [] }
        })
        .to_string(),
        "lyr",
    );
    fresh.set_origin_init(false);
    assert_eq!(fresh.zone_count(), 1);
    assert!(!fresh.can_undo(), "hydrate must not create an undo step");
}

#[test]
fn a_labelled_polygon_zone_create_is_one_undo_step() {
    let mut doc = MissionDocCore::new();
    assert_eq!(doc.undo_depth(), 0, "a fresh doc has nothing to undo");

    doc.add_polygon_zone_labelled(
        "z1",
        "boundary",
        &[0.0, 0.0, 12_800.0, 0.0, 12_800.0, 12_800.0, 0.0, 12_800.0],
        Some("Play Area"),
    );
    let rows: serde_json::Value =
        serde_json::from_str(&doc.zones_json()).expect("zones_json parses");
    assert_eq!(rows["z1"]["type"], "boundary");
    assert_eq!(
        rows["z1"]["label"], "Play Area",
        "the create must land NAMED, not name it afterwards"
    );
    assert_eq!(
        doc.undo_depth(),
        1,
        "one gesture = one LOCAL txn = ONE undo step (capture_timeout_millis = 0)"
    );

    assert!(doc.undo());
    assert_eq!(
        doc.zone_count(),
        0,
        "the author's SINGLE Ctrl+Z must remove the whole zone, not just its name"
    );
    assert_eq!(doc.undo_depth(), 0, "and leave nothing else stacked");
    assert!(doc.redo());
    let back: serde_json::Value =
        serde_json::from_str(&doc.zones_json()).expect("zones_json parses");
    assert_eq!(
        back["z1"]["label"], "Play Area",
        "redo must restore the NAME with the ring — one step, both keys"
    );

    let two_call = MissionDocCore::new();
    two_call.add_polygon_zone("z1", "boundary", &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0]);
    two_call.set_zone_label("z1", Some("Play Area"));
    assert_eq!(
        two_call.undo_depth(),
        2,
        "create-then-name is TWO txns and therefore two Ctrl+Z presses — which is why \
             add_polygon_zone_labelled exists"
    );
}

#[test]
fn an_unlabelled_polygon_zone_carries_no_label_key() {
    let doc = MissionDocCore::new();
    doc.add_polygon_zone("z1", "boundary", &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0]);
    doc.add_polygon_zone_labelled("z2", "boundary", &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0], Some(""));
    let rows: serde_json::Value =
        serde_json::from_str(&doc.zones_json()).expect("zones_json parses");
    assert!(
        rows["z1"].get("label").is_none(),
        "the draw-tool create must still write NO label key: {}",
        rows["z1"]
    );
    assert_eq!(
        rows["z2"]["label"], "",
        "Some(\"\") stays the distinct empty-label state set_zone_label already allows"
    );
}
