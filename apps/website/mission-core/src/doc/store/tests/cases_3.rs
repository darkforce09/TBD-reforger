//! Role: Domain regression cases.
//! Position: `doc/store/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[cfg(feature = "compiler")]
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
        crate::mission::compile::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
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

#[cfg(feature = "compiler")]
#[test]
fn hydrate_without_unknown_keys_clears_payload_extras() {
    let with = serde_json::json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon" },
        "serverMigrationToken": "ghost",
        "editor": {
            "factions": [],
            "squads": [],
            "slots": [],
            "editorLayers": []
        }
    });
    let without = serde_json::json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon" },
        "editor": {
            "factions": [],
            "squads": [],
            "slots": [],
            "editorLayers": []
        }
    });

    let doc = MissionDocCore::new();
    doc.hydrate(&with.to_string(), "lyr");
    assert!(
        small_maps(&doc)["payloadExtras"]
            .get("serverMigrationToken")
            .is_some()
    );

    doc.hydrate(&without.to_string(), "lyr");
    let small = small_maps(&doc);
    assert!(
        small.get("payloadExtras").is_none(),
        "empty extras must be omitted from small_maps_json; got {small:?}"
    );
    let compiled =
        crate::mission::compile::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    assert!(compiled.get("serverMigrationToken").is_none());
}

#[test]
fn t505_hydrate_and_prefer_payload_title_over_stale_row() {
    let payload = serde_json::json!({
        "schemaVersion": 1,
        "title": "  Authored Bridgehead  ",
        "map": { "terrain": "everon" },
        "environment": {},
        "editor": {
            "factions": [],
            "squads": [],
            "slots": [],
            "editorLayers": []
        }
    });
    let doc = MissionDocCore::new();
    doc.set_title("Stale Library Title");
    doc.hydrate(&payload.to_string(), "lyr");
    assert_eq!(
        small_maps(&doc)["meta"]["title"],
        "Authored Bridgehead",
        "hydrate must load trimmed payload title into meta"
    );

    let preferred = payload
        .get("title")
        .and_then(|t| t.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Stale Library Title");
    doc.apply_row_meta(preferred, "everon", None, None, None);
    assert_eq!(
        small_maps(&doc)["meta"]["title"],
        "Authored Bridgehead",
        "prefer-payload title must survive apply_row_meta; got {:?}",
        small_maps(&doc)["meta"]["title"]
    );
    assert!(
        small_maps(&doc).get("payloadExtras").is_none()
            || small_maps(&doc)["payloadExtras"].get("title").is_none(),
        "title is a known hydrate key — must not park in payloadExtras"
    );
}

#[test]
fn t505_hydrate_whitespace_title_ignored_row_can_fill() {
    let payload = serde_json::json!({
        "schemaVersion": 1,
        "title": "   ",
        "map": { "terrain": "everon" },
        "environment": {},
        "editor": {
            "factions": [],
            "squads": [],
            "slots": [],
            "editorLayers": []
        }
    });
    let doc = MissionDocCore::new();
    doc.set_title("Preexisting");
    doc.hydrate(&payload.to_string(), "lyr");
    assert!(
        small_maps(&doc)["meta"].get("title").is_none(),
        "whitespace-only payload title must clear sticky meta.title, not keep Preexisting"
    );
    doc.apply_row_meta("  Row Title  ", "everon", None, None, None);
    assert_eq!(
        small_maps(&doc)["meta"]["title"],
        "Row Title",
        "apply_row_meta must trim and accept a real row title when payload title is blank"
    );
}

#[test]
fn t418_apply_row_meta_threads_briefing_into_meta() {
    let doc = MissionDocCore::new();
    doc.apply_row_meta(
        "Op",
        "everon",
        None,
        None,
        Some("  Hold the bridge.\nWait for extract.  ".into()),
    );
    assert_eq!(
        small_maps(&doc)["meta"]["briefing"],
        "Hold the bridge.\nWait for extract.",
        "non-blank row briefing must trim and land in meta.briefing"
    );

    let empty = MissionDocCore::new();
    empty.apply_row_meta("Op", "everon", None, None, Some("   \n\t  ".into()));
    assert!(
        small_maps(&empty)["meta"].get("briefing").is_none(),
        "whitespace-only briefing must not invent meta.briefing"
    );
}

#[test]
fn t766_clear_meta_briefing_drops_key_blank_apply_does_not() {
    let doc = MissionDocCore::new();
    doc.apply_row_meta(
        "Op",
        "everon",
        None,
        None,
        Some("Hold the bridge.\nWait for extract.".into()),
    );
    assert_eq!(
        small_maps(&doc)["meta"]["briefing"],
        "Hold the bridge.\nWait for extract.",
        "precondition: row briefing must land"
    );

    doc.apply_row_meta("Op", "everon", None, None, Some("".into()));
    assert_eq!(
        small_maps(&doc)["meta"]["briefing"],
        "Hold the bridge.\nWait for extract.",
        "blank apply_row_meta must stay 'not supplied', not a clear"
    );
    doc.apply_row_meta("Op", "everon", None, None, Some("   \n\t  ".into()));
    assert_eq!(
        small_maps(&doc)["meta"]["briefing"],
        "Hold the bridge.\nWait for extract.",
        "whitespace-only apply_row_meta must stay 'not supplied'"
    );

    doc.clear_meta_briefing();
    assert!(
        small_maps(&doc)["meta"].get("briefing").is_none(),
        "clear_meta_briefing must remove meta.briefing so compile_export emits \"\""
    );

    doc.clear_meta_briefing();
    assert!(
        small_maps(&doc)["meta"].get("briefing").is_none(),
        "second clear must stay absent"
    );
}

#[cfg(feature = "compiler")]
#[test]
fn t220_hydrate_compile_preserves_schema_map_and_slot_order() {
    let incoming = t220_lossy_payload();
    let doc = MissionDocCore::new();
    doc.hydrate(&incoming.to_string(), "lyr");

    let compiled =
        crate::mission::compile::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);

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
    let again = crate::mission::compile::compile_payload(
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

#[cfg(feature = "compiler")]
#[test]
fn t220_position_subkeys_survive_first_edit() {
    let doc = MissionDocCore::new();
    doc.hydrate(&t220_lossy_payload().to_string(), "lyr");

    let before: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots json");
    assert_eq!(
        before["z-b"]["position"]["heading"],
        serde_json::json!(90.5)
    );
    assert_eq!(
        before["z-b"]["position"]["source"],
        serde_json::json!("authored")
    );

    doc.set_slot_position("z-b", 11.0, 21.0, 1.25, 45.0);
    let after: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots json");
    assert_eq!(after["z-b"]["position"]["x"].as_f64(), Some(11.0));
    assert_eq!(after["z-b"]["position"]["y"].as_f64(), Some(21.0));
    assert_eq!(
        after["z-b"]["position"]["heading"].as_f64(),
        Some(90.5),
        "unknown position sub-keys must survive set_slot_position"
    );
    assert_eq!(
        after["z-b"]["position"]["source"],
        serde_json::json!("authored")
    );

    doc.update_slot_position("z-b", Some(12.0), None, None, None, 12800.0, 12800.0);
    let after2: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots json");
    assert_eq!(after2["z-b"]["position"]["x"].as_f64(), Some(12.0));
    assert_eq!(
        after2["z-b"]["position"]["heading"].as_f64(),
        Some(90.5),
        "unknown position sub-keys must survive update_slot_position"
    );
}

#[cfg(feature = "compiler")]
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

#[cfg(feature = "compiler")]
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

#[cfg(feature = "compiler")]
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
fn set_vehicle_cargo_drops_unrepresentable_rows_and_clears_on_empty() {
    let doc = orbat_fixture();
    doc.add_vehicle(
        "v",
        "{AAAA}P.et",
        Some(1.0),
        Some(1.0),
        Some(0.0),
        Some(0.0),
    );

    doc.set_vehicle_cargo(
        "v",
        &[
            ("   ".to_string(), 3),
            ("{BBBB}P.et".to_string(), 0),
            ("{CCCC}P.et".to_string(), -1),
            ("{DDDD}P.et".to_string(), 1),
        ],
    );
    assert_eq!(
        vehicles_of(&doc)["v"]["cargo"],
        serde_json::json!([{ "item": "{DDDD}P.et", "qty": 1 }]),
    );

    doc.set_vehicle_cargo("v", &[]);
    assert!(
        vehicles_of(&doc)["v"].get("cargo").is_none(),
        "an empty result must REMOVE the key, not write []"
    );
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
        crate::mission::compile::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    let reloaded = MissionDocCore::new();
    reloaded.hydrate(&payload.to_string(), "lyr");

    let survived = markers_of(&reloaded, "faction-BLUFOR");
    assert_eq!(survived, authored, "marker rows must survive hydrate whole");

    let recompiled = crate::mission::compile::compile_payload(
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

#[cfg(feature = "compiler")]
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
        crate::mission::compile::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    let meta = crate::mission::flatten::MissionMeta {
        id: "4c7e1b08-9a35-4d62-b1f7-e30d5a86c941".into(),
        title: "Bridgehead at Levie".into(),
        author: "184472930165846017".into(),
        terrain: "everon".into(),
        custom_terrain_name: String::new(),
        max_players: 12,
        time_of_day: "06:15".into(),
        weather_preset: "overcast".into(),
    };
    let compiled = crate::mission::flatten::flatten_to_mod_document(
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

#[test]
fn setting_the_same_marker_id_moves_it_in_place() {
    let doc = briefing_fixture();
    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 100.0, 200.0, "objective", "OBJ");
    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-2", 300.0, 400.0, "hazard", "MINES");

    doc.set_faction_briefing_marker(
        "faction-BLUFOR",
        "mk-1",
        111.5,
        222.5,
        "rally",
        "Rally point",
    );

    let rows = markers_of(&doc, "faction-BLUFOR");
    assert_eq!(rows.len(), 2, "an upsert must not duplicate: {rows:?}");

    assert_eq!(rows[0]["id"], serde_json::json!("mk-1"));
    assert_eq!(marker_num(&rows[0], "x"), 111.5);
    assert_eq!(marker_num(&rows[0], "z"), 222.5);
    assert_eq!(rows[0]["icon"], serde_json::json!("rally"));
    assert_eq!(rows[0]["label"], serde_json::json!("Rally point"));
    assert_eq!(rows[1]["id"], serde_json::json!("mk-2"));
    assert_eq!(marker_num(&rows[1], "x"), 300.0);
}

#[test]
fn removing_a_marker_by_id_leaves_its_siblings_alone() {
    let doc = briefing_fixture();
    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 100.0, 200.0, "objective", "OBJ");
    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-2", 300.0, 400.0, "hazard", "MINES");
    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-3", 500.0, 600.0, "rally", "RP");

    doc.remove_faction_briefing_marker("faction-BLUFOR", "mk-2");

    let rows = markers_of(&doc, "faction-BLUFOR");
    let ids: Vec<&str> = rows
        .iter()
        .map(|m| m["id"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(ids, vec!["mk-1", "mk-3"], "{rows:?}");
    assert_eq!(marker_num(&rows[1], "x"), 500.0, "survivor intact");

    doc.remove_faction_briefing_marker("faction-BLUFOR", "mk-1");
    doc.remove_faction_briefing_marker("faction-BLUFOR", "mk-3");
    assert!(markers_of(&doc, "faction-BLUFOR").is_empty());
    assert!(
        small_maps(&doc)["factionsById"]["faction-BLUFOR"]["briefing"]["markers"].is_array(),
        "an emptied list stays an array, not a dropped key"
    );
}

#[test]
fn removing_from_an_unauthored_faction_does_not_mint_a_briefing() {
    let doc = briefing_fixture();
    let before = small_maps(&doc);
    doc.remove_faction_briefing_marker("faction-OPFOR", "mk-1");
    doc.remove_faction_briefing_marker("faction-does-not-exist", "mk-1");

    assert!(
        small_maps(&doc)["factionsById"]["faction-OPFOR"]
            .get("briefing")
            .is_none(),
        "a no-op delete must not author a briefing"
    );

    assert_eq!(
        small_maps(&doc),
        before,
        "a no-op delete must write nothing"
    );
    assert!(
        !doc.can_undo(),
        "a no-op delete must not stack an undo step"
    );
}

#[test]
fn a_marker_edit_preserves_authored_prose_on_the_same_briefing() {
    let payload = serde_json::json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon" },
        "editor": {
            "factions": [{
                "id": "faction-BLUFOR", "key": "BLUFOR", "name": "US Army",
                "squadIds": ["sq-a"],
                "briefing": {
                    "situation": "Enemy armour holds the east bank.\n\nBridge is intact.",
                    "mission": "Seize and hold the crossing.",
                    "execution": "Alpha leads.",
                }
            }],
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
    doc.set_faction_briefing_marker(
        "faction-BLUFOR",
        "mk-1",
        4870.25,
        7760.5,
        "objective",
        "OBJ",
    );

    let briefing = &small_maps(&doc)["factionsById"]["faction-BLUFOR"]["briefing"];
    assert_eq!(
        briefing["situation"],
        serde_json::json!("Enemy armour holds the east bank.\n\nBridge is intact."),
        "the paragraph break must survive a marker edit verbatim"
    );
    assert_eq!(
        briefing["mission"],
        serde_json::json!("Seize and hold the crossing.")
    );
    assert_eq!(briefing["execution"], serde_json::json!("Alpha leads."));
    assert_eq!(briefing["markers"].as_array().expect("markers").len(), 1);

    doc.remove_faction_briefing_marker("faction-BLUFOR", "mk-1");
    let after = &small_maps(&doc)["factionsById"]["faction-BLUFOR"]["briefing"];
    assert_eq!(
        after["mission"],
        serde_json::json!("Seize and hold the crossing.")
    );
    assert!(after["markers"].as_array().expect("markers").is_empty());
}

#[test]
fn every_listed_marker_is_addressable_by_the_pair_the_mutators_take() {
    let doc = briefing_fixture();
    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 100.5, 200.5, "objective", "OBJ");
    doc.set_faction_briefing_marker("faction-OPFOR", "mk-2", 300.5, 400.5, "ambush", "AMB");

    let rows = marker_rows(&doc);
    assert_eq!(rows.len(), 2, "both sides list: {rows:?}");

    assert_eq!(rows[0]["factionId"], serde_json::json!("faction-BLUFOR"));
    assert_eq!(rows[1]["factionId"], serde_json::json!("faction-OPFOR"));
    assert_eq!(marker_num(&rows[0], "x"), 100.5);
    assert_eq!(marker_num(&rows[0], "z"), 200.5);
    assert_eq!(rows[0]["icon"], serde_json::json!("objective"));
    assert_eq!(rows[0]["label"], serde_json::json!("OBJ"));

    for r in &rows {
        let f = r["factionId"].as_str().expect("factionId");
        let id = r["id"].as_str().expect("id");
        doc.remove_faction_briefing_marker(f, id);
    }
    assert!(
        marker_rows(&doc).is_empty(),
        "every listed row deleted through its own (factionId, id)"
    );
}

#[test]
fn marker_rows_are_stable_across_calls_and_keep_array_order() {
    let doc = briefing_fixture();

    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-z", 1.0, 1.0, "rally", "Z");
    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-a", 2.0, 2.0, "rally", "A");
    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-m", 3.0, 3.0, "rally", "M");

    let ids: Vec<String> = marker_rows(&doc)
        .iter()
        .map(|r| r["id"].as_str().expect("id").to_string())
        .collect();
    assert_eq!(
        ids,
        vec!["mk-z".to_string(), "mk-a".to_string(), "mk-m".to_string()],
        "array order, not id order"
    );

    let first = doc.briefing_marker_rows_json();
    for _ in 0..8 {
        assert_eq!(
            doc.briefing_marker_rows_json(),
            first,
            "the reader must not shuffle an unchanged document"
        );
    }

    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-z", 9.0, 9.0, "rally", "Z");
    let after: Vec<String> = marker_rows(&doc)
        .iter()
        .map(|r| r["id"].as_str().expect("id").to_string())
        .collect();
    assert_eq!(after, ids, "a move must not reorder the list");
}

#[cfg(feature = "compiler")]
#[test]
fn t826_marker_without_faction_parks_and_does_not_declare_players() {
    let doc = MissionDocCore::new();
    doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 10.0, 20.0, "objective", "OBJ");

    assert!(
        small_maps(&doc)["factionsById"]
            .as_object()
            .is_none_or(|m| m.is_empty()),
        "marker place must not mint a faction: {:?}",
        small_maps(&doc)["factionsById"]
    );
    let rows = marker_rows(&doc);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["factionId"], serde_json::json!("faction-BLUFOR"));
    assert_eq!(rows[0]["id"], serde_json::json!("mk-1"));

    let payload =
        crate::mission::compile::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    let findings = crate::mission::validate::validate_editor_payload(&payload);
    assert!(
        findings.iter().all(|f| f.rule_id != "V1-PLAYER-SPAWN"),
        "marker-only must not trip V1: {findings:?}"
    );
}
