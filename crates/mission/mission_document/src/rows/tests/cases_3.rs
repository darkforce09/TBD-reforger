//! Role: Domain regression cases.
//! Position: a test or policy module of `mission_document`, compiled into the crate.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn authored_payload_extras_key_is_reserved_not_reemitted() {
    let incoming = serde_json::json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon" },
        "environment": {},
        "payloadExtras": { "nested": true },
        "serverMigrationToken": "keep-me-v2",
        "editor": {
            "factions": [],
            "squads": [],
            "slots": [],
            "editorLayers": []
        }
    });

    let doc = MissionDocCore::new();
    doc.hydrate(&incoming.to_string(), "lyr");

    let small = small_maps(&doc);
    assert!(
        small
            .get("payloadExtras")
            .and_then(|e| e.get("payloadExtras"))
            .is_none(),
        "hydrate must not nest the reserved side-channel name into itself; got {:?}",
        small.get("payloadExtras")
    );
    assert_eq!(
        small["payloadExtras"]["serverMigrationToken"],
        serde_json::json!("keep-me-v2"),
        "unrelated unknown keys must still park"
    );

    let compiled =
        mission_payload::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    assert!(
        compiled.get("payloadExtras").is_none(),
        "compiled wire must not carry the side-channel name; got {:?}",
        compiled.get("payloadExtras")
    );
    assert_eq!(
        compiled["serverMigrationToken"],
        serde_json::json!("keep-me-v2")
    );

    assert!(
        compiled
            .as_object()
            .map(|o| !o
                .values()
                .any(|v| v == &serde_json::json!({ "nested": true })))
            .unwrap_or(false),
        "reserved nested object must not be re-emitted under another wire key; got {compiled:?}"
    );
}

#[test]
fn t220_hydrate_compile_preserves_schema_map_and_slot_order() {
    let incoming = t220_lossy_payload();
    let doc = MissionDocCore::new();
    doc.hydrate(&incoming.to_string(), "lyr");

    let compiled =
        mission_payload::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);

    assert_eq!(
        compiled["schemaVersion"],
        serde_json::json!(2),
        "authored schemaVersion must not downgrade to literal 1"
    );
    assert_eq!(
        compiled["map"]["bounds"],
        serde_json::json!([100, 200, 300, 400]),
        "authored map.bounds must not be recomputed"
    );
    assert_eq!(
        compiled["map"]["center"],
        serde_json::json!([6400.5, 6400.25]),
        "other map.* keys must survive"
    );
    assert_eq!(compiled["map"]["label"], serde_json::json!("ops-sector"));
    assert_eq!(compiled["map"]["terrain"], serde_json::json!("everon"));

    let ids: Vec<&str> = compiled["editor"]["slots"]
        .as_array()
        .expect("slots")
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec!["z-b", "z-a", "z-c"],
        "editor.slots array order must follow hydrate insertion, not id-sort"
    );

    let reloaded = save_and_reload(&doc);
    let again = mission_payload::compile_payload(
        &reloaded.small_maps_json(),
        &reloaded.slots_json(),
        false,
    );
    assert_eq!(again["schemaVersion"], serde_json::json!(2));
    assert_eq!(
        again["map"]["bounds"],
        serde_json::json!([100, 200, 300, 400])
    );
    assert_eq!(again["map"]["center"], serde_json::json!([6400.5, 6400.25]));
}

#[test]
fn t220_paste_preserves_unknown_slot_fields() {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Default", None);
    let extras = serde_json::json!({
        "customFlag": "keep-me",
        "doctrineTag": "assault",
        "position": { "heading": 90.5, "source": "authored" }
    })
    .to_string();
    doc.paste_slots(
        vec!["p-new".into()],
        vec!["sq1".into()],
        vec!["lyr".into()],
        vec![10.0],
        vec![20.0],
        vec![45.0],
        vec![1.25],
        vec!["Rifleman".into()],
        vec![String::new()],
        vec![String::new()],
        vec!["stand".into()],
        vec![String::new()],
        vec![extras],
        Some(100.0),
        Some(200.0),
        12800.0,
        12800.0,
    );
    let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots json");
    assert_eq!(v["p-new"]["customFlag"], serde_json::json!("keep-me"));
    assert_eq!(v["p-new"]["doctrineTag"], serde_json::json!("assault"));

    assert_eq!(v["p-new"]["position"]["x"].as_f64(), Some(100.0));
    assert_eq!(v["p-new"]["position"]["y"].as_f64(), Some(200.0));
    assert_eq!(v["p-new"]["position"]["heading"].as_f64(), Some(90.5));
    assert_eq!(
        v["p-new"]["position"]["source"],
        serde_json::json!("authored")
    );
}

#[test]
fn map_placed_vehicle_position_round_trips_through_compile_and_hydrate() {
    let doc = orbat_fixture();
    doc.add_vehicle(
        "veh-map",
        "{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et",
        Some(4870.25),
        Some(7760.5),
        Some(12.75),
        Some(137.5),
    );
    doc.set_vehicle_faction("veh-map", "faction-BLUFOR");

    let authored = vehicles_of(&doc)["veh-map"].clone();
    assert_eq!(authored["position"]["x"], serde_json::json!(4870.25));
    assert_eq!(authored["position"]["y"], serde_json::json!(7760.5));
    assert_eq!(authored["position"]["z"], serde_json::json!(12.75));
    assert_eq!(authored["position"]["rotation"], serde_json::json!(137.5));
    assert_eq!(authored["factionId"], serde_json::json!("faction-BLUFOR"));

    assert!(
        authored.get("squadId").is_none(),
        "map placement must not attach to a squad: {authored}"
    );
    let vids = small_maps(&doc)["squadsById"]["sq-a"]
        .get("vehicleIds")
        .and_then(|v| v.as_array())
        .map_or(0, Vec::len);
    assert_eq!(vids, 0, "no squad may acquire a map-placed vehicle");

    let survived = vehicles_of(&save_and_reload(&doc))["veh-map"].clone();
    assert_eq!(
        survived, authored,
        "the vehicle row must survive save → reload whole"
    );
}

#[test]
fn map_placed_vehicle_cargo_round_trips_as_entity_inventory_rows() {
    let doc = orbat_fixture();
    doc.add_vehicle(
        "veh-map",
        "{AAAA}Prefabs/Vehicles/T.et",
        Some(1.5),
        Some(2.5),
        Some(0.0),
        Some(0.0),
    );
    doc.set_vehicle_cargo(
        "veh-map",
        &[
            ("{BBBB}Prefabs/Weapons/M16.et".to_string(), 4),
            ("{CCCC}Prefabs/Items/Bandage.et".to_string(), 12),
        ],
    );

    let authored = vehicles_of(&doc)["veh-map"]["cargo"].clone();
    assert_eq!(
        authored,
        serde_json::json!([
            { "item": "{BBBB}Prefabs/Weapons/M16.et", "qty": 4 },
            { "item": "{CCCC}Prefabs/Items/Bandage.et", "qty": 12 },
        ]),
        "cargo rows must be $defs/entityInventory verbatim"
    );

    let survived = vehicles_of(&save_and_reload(&doc))["veh-map"]["cargo"].clone();
    assert_eq!(survived, authored, "cargo must survive save → reload whole");
}

#[test]
fn slot_indices_dense_after_move() {
    let doc = orbat_fixture();
    doc.add_slot(
        "a0", "sq-a", "lyr", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
    );
    doc.add_slot(
        "a1", "sq-a", "lyr", 1, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
    );
    doc.add_slot(
        "a2", "sq-a", "lyr", 2, "Rifleman", None, None, 3.0, 3.0, 0.0, 0.0,
    );
    doc.set_leader("sq-a", "a0");
    doc.add_slot(
        "b0", "sq-b", "lyr", 0, "Rifleman", None, None, 4.0, 4.0, 0.0, 0.0,
    );
    doc.set_leader("sq-b", "b0");

    doc.move_slot_to_squad("a1", "sq-b");
    let slots = slots_map(&doc);
    let root = small_maps(&doc);
    for (sq_id, key) in [("sq-a", "sq-a"), ("sq-b", "sq-b")] {
        let ids = root["squadsById"][key]["slotIds"]
            .as_array()
            .unwrap_or_else(|| panic!("{sq_id} slotIds"));
        for (i, id_val) in ids.iter().enumerate() {
            let sid = id_val.as_str().expect("id str");
            let idx = slots[sid]["index"].as_i64().expect("index");
            assert_eq!(idx, i as i64, "{sq_id}/{sid} index");
        }
    }
}

#[test]
fn authored_marker_round_trips_through_compile_and_hydrate() {
    let doc = briefing_fixture();
    doc.set_faction_briefing_marker(
        "faction-BLUFOR",
        "mk-1",
        4870.25,
        7760.5,
        "objective",
        "Seize the bridge",
    );

    let authored = markers_of(&doc, "faction-BLUFOR");
    assert_eq!(authored.len(), 1, "one authored marker: {authored:?}");
    assert_eq!(authored[0]["x"], serde_json::json!(4870.25));
    assert_eq!(authored[0]["z"], serde_json::json!(7760.5));
    assert_eq!(authored[0]["icon"], serde_json::json!("objective"));
    assert_eq!(authored[0]["label"], serde_json::json!("Seize the bridge"));
    assert_eq!(authored[0]["id"], serde_json::json!("mk-1"));

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

    let survived = markers_of(&reloaded, "faction-BLUFOR");
    assert_eq!(survived, authored, "marker rows must survive hydrate whole");

    let recompiled = mission_payload::compile_payload(
        &reloaded.small_maps_json(),
        &reloaded.slots_json(),
        false,
    );
    assert_eq!(
        recompiled["editor"]["factions"][0]["briefing"]["markers"],
        payload["editor"]["factions"][0]["briefing"]["markers"]
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
fn authored_marker_reaches_the_mod_document_without_its_doc_id() {
    let doc = briefing_fixture();
    doc.set_faction_briefing_marker(
        "faction-BLUFOR",
        "mk-1",
        4870.25,
        7760.5,
        "objective",
        "Seize the bridge",
    );

    let payload =
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
        &serde_json::to_vec(&payload).expect("payload serialises"),
    )
    .expect("the fixture has a slot, so the compile must succeed");
    let doc_json = serde_json::to_value(&compiled).expect("mod document serialises");

    let rows = doc_json["briefings"]["blufor"]["markers"]
        .as_array()
        .expect("blufor briefing carries markers");
    assert_eq!(rows.len(), 1, "{rows:?}");

    let m = rows[0].as_object().expect("marker is an object");
    assert_eq!(m["x"], serde_json::json!(4870.25));
    assert_eq!(m["z"], serde_json::json!(7760.5));
    assert_eq!(m["icon"], serde_json::json!("objective"));
    assert_eq!(m["label"], serde_json::json!("Seize the bridge"));

    assert!(
        m.get("id").is_none(),
        "the doc-internal marker id must not reach the compiled document: {m:?}"
    );
    assert_eq!(
        m.len(),
        4,
        "exactly the four `$defs/marker` fields, no more: {m:?}"
    );

    assert!(
        doc_json["briefings"].get("opfor").is_none(),
        "{:?}",
        doc_json["briefings"]
    );
}
