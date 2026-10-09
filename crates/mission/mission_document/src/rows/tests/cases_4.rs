//! Role: Domain regression cases.
//! Position: a test or policy module of `mission_document`, compiled into the crate.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

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

    let compiled_payload =
        mission_payload::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    let meta = mission_compiler::MissionMeta {
        id: "4c7e1b08-9a35-4d62-b1f7-e30d5a86c941".into(),
        title: "Bridgehead at Levie".into(),
        author: "184472930165846017".into(),
        terrain: "everon".into(),
        custom_terrain_name: String::new(),
        max_players: 12,
        time_of_day: "06:15".into(),
        weather_preset: "overcast".into(),
    };
    let compiled = mission_compiler::flatten_to_mod_document(
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

    let payload =
        mission_payload::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
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

    let recompiled = mission_payload::compile_payload(
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
fn authored_zones_survive_compile_hydrate_compile_whole() {
    let doc = zones_fixture();

    let compiled =
        mission_payload::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
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

    let recompiled = mission_payload::compile_payload(
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

#[test]
fn authored_zones_reach_the_mod_document_through_flatten() {
    use mission_compiler::flatten_to_mod_document;
    use mission_model::compiled::mission::ModZoneShape;

    let doc = zones_fixture();
    let compiled =
        mission_payload::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    let bytes = serde_json::to_vec(&compiled).expect("serialise payload");

    let meta = mission_compiler::MissionMeta {
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
