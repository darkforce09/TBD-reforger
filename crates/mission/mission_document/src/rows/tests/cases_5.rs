//! Role: Domain regression cases.
//! Position: a test or policy module of `mission_document`, compiled into the crate.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn authored_triggers_survive_compile_hydrate_compile_whole() {
    let doc = triggers_fixture();

    let compiled =
        mission_payload::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    let first = wire_triggers(&compiled);
    assert_eq!(
        first.len(),
        2,
        "both authored triggers must reach the wire payload: {compiled}"
    );

    let expect_amb = serde_json::json!({
        "id": "t_amb",
        "name": "Ambush",
        "ownerId": "s1",
        "activation": "presence",
        "shape": { "polygon": [
            [1000.25, -4210.75],
            [1600.5,  -4210.75],
            [1600.5,  -3800.125],
            [1000.25, -3800.125]
        ]},
        "rules": { "graceSeconds": 45.5, "contestable": false }
    });
    let expect_timer = serde_json::json!({
        "id": "t_timer",
        "activation": "timer",
        "shape": { "circle": { "x": 1234.5, "z": -3990.25, "r": 175.75 } },
        "rules": { "announceEverySeconds": 12.5 }
    });

    let by_id = |rows: &[serde_json::Value], id: &str| -> serde_json::Value {
        rows.iter()
            .find(|r| r["id"] == id)
            .unwrap_or_else(|| panic!("trigger {id} missing from {rows:?}"))
            .clone()
    };

    assert_eq!(
        by_id(&first, "t_amb"),
        expect_amb,
        "compile #1 dropped part of t_amb (geometry / owner / activation / rules)"
    );
    assert_eq!(
        by_id(&first, "t_timer"),
        expect_timer,
        "compile #1 dropped part of t_timer"
    );

    let reloaded = save_and_reload(&doc);
    assert_eq!(
        reloaded.trigger_count(),
        2,
        "hydrate must restore both triggers"
    );

    let recompiled = mission_payload::compile_payload(
        &reloaded.small_maps_json(),
        &reloaded.slots_json(),
        false,
    );
    let second = wire_triggers(&recompiled);
    assert_eq!(
        by_id(&second, "t_amb"),
        expect_amb,
        "t_amb did not survive save→reload→save WHOLE"
    );
    assert_eq!(
        by_id(&second, "t_timer"),
        expect_timer,
        "t_timer did not survive save→reload→save WHOLE"
    );

    assert!(
        recompiled.get("payloadExtras").is_none(),
        "payloadExtras must not reach the wire: {recompiled}"
    );
}

#[test]
fn trigger_edits_are_undoable_and_hydrate_is_not() {
    let mut doc = MissionDocCore::new();
    doc.add_circle_trigger("t1", "presence", 1.5, 2.5, 3.5);
    assert_eq!(doc.trigger_count(), 1);
    assert!(doc.can_undo(), "a drawn trigger must be undoable");
    assert!(doc.undo());
    assert_eq!(doc.trigger_count(), 0, "undo did not remove the trigger");
    assert!(doc.redo());
    assert_eq!(doc.trigger_count(), 1, "redo did not restore the trigger");

    let fresh = MissionDocCore::new();
    fresh.set_origin_init(true);
    fresh.hydrate(
        &serde_json::json!({
            "triggers": [ { "id": "t", "activation": "radio",
                            "shape": { "circle": { "x": 1.5, "z": 2.5, "r": 3.5 } } } ],
            "editor": { "factions": [], "squads": [], "slots": [], "editorLayers": [] }
        })
        .to_string(),
        "lyr",
    );
    fresh.set_origin_init(false);
    assert_eq!(fresh.trigger_count(), 1);
    assert!(!fresh.can_undo(), "hydrate must not create an undo step");
}

#[test]
fn saved_composition_survives_compile_hydrate_compile_whole() {
    let doc = compositions_fixture();

    let compiled =
        mission_payload::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    let first = wire_compositions(&compiled);
    assert_eq!(
        first.len(),
        1,
        "one composition on the first compile: {compiled}"
    );
    let expected: serde_json::Value =
        serde_json::from_str(&composition_row_json()).expect("expected row");
    assert_eq!(
        canon(&first[0]),
        canon(&expected),
        "the first compile changed the row"
    );

    let reloaded = MissionDocCore::new();
    reloaded.hydrate(&compiled.to_string(), "lyr");
    assert_eq!(
        reloaded.composition_count(),
        1,
        "hydrate lost the composition"
    );

    let recompiled = mission_payload::compile_payload(
        &reloaded.small_maps_json(),
        &reloaded.slots_json(),
        false,
    );
    let second = wire_compositions(&recompiled);
    assert_eq!(
        second.len(),
        1,
        "the composition died on the SECOND compile — echoed, not stored: {recompiled}"
    );
    assert_eq!(
        canon(&second[0]),
        canon(&expected),
        "the row is not identical after the round trip (nested entities/offsets lost?): {}",
        second[0]
    );
}

#[test]
fn composition_edits_are_undoable_and_count_as_content() {
    let mut doc = MissionDocCore::new();
    assert!(!doc.has_content(), "fresh doc has no content");
    doc.add_composition("c1", &composition_row_json());
    assert!(
        doc.has_content(),
        "a saved composition is authored work and must count as local content"
    );
    assert_eq!(doc.composition_count(), 1);
    assert!(doc.can_undo(), "a saved composition must be undoable");
    assert!(doc.undo());
    assert_eq!(
        doc.composition_count(),
        0,
        "undo did not remove the composition"
    );
    assert!(doc.redo());
    assert_eq!(
        doc.composition_count(),
        1,
        "redo did not restore the composition"
    );

    doc.set_composition_title("c1", "Renamed");
    assert!(doc.undo(), "the rename must be undoable");
    let after = serde_json::from_str::<serde_json::Value>(&doc.compositions_json())
        .expect("compositions_json");
    assert_eq!(
        after["c1"]["title"], "Fireteam + Technical",
        "undo did not restore the original title"
    );

    let fresh = MissionDocCore::new();
    fresh.set_origin_init(true);
    fresh.hydrate(
            &serde_json::json!({
                "compositions": [ { "id": "c", "title": "T", "author": "a", "category": "x", "entities": [] } ],
                "editor": { "factions": [], "squads": [], "slots": [], "editorLayers": [] }
            })
            .to_string(),
            "lyr",
        );
    fresh.set_origin_init(false);
    assert_eq!(fresh.composition_count(), 1);
    assert!(!fresh.can_undo(), "hydrate must not create an undo step");
}

#[test]
fn placing_a_composition_preserves_offsets_in_one_undo_step() {
    let mut doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_editor_layer("L", "Layer", None);
    doc.set_origin_init(false);

    let row: serde_json::Value = serde_json::from_str(&composition_row_json()).expect("row");
    let entities = row["entities"].to_string();

    let ids = vec![
        "p0".to_string(),
        "p1".to_string(),
        "p2".to_string(),
        "p3".to_string(),
    ];
    let (drop_x, drop_y) = (1000.0, 2000.0);
    let written = doc.place_composition(
        &entities, &ids, "BLUFOR", "L", drop_x, drop_y, 12800.0, 12800.0,
    );
    assert_eq!(written.len(), 4, "all four entities placed");

    let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");

    assert_eq!(
        slots["p0"]["position"]["x"],
        drop_x - 12.75,
        "slot0 x offset"
    );
    assert_eq!(slots["p0"]["position"]["y"], drop_y + 8.5, "slot0 y offset");
    assert_eq!(slots["p0"]["role"], "Squad Leader");
    assert_eq!(
        slots["p0"]["position"]["rotation"], 45.5,
        "slot0 heading kept"
    );

    assert_eq!(slots["p0"]["loadout"]["gear"]["primary"], "M4");

    assert_eq!(slots["p1"]["position"]["x"], drop_x + 12.25);
    assert_eq!(slots["p1"]["position"]["y"], drop_y - 8.5);

    let vehs = vehicles_of(&doc);
    assert_eq!(vehs["p2"]["resourceName"], "Prefab/Technical.et");
    assert_eq!(vehs["p2"]["position"]["x"], drop_x + 0.5);
    assert_eq!(
        vehs["p2"]["position"]["rotation"], 270.75,
        "vehicle heading kept"
    );
    assert_eq!(vehs["p2"]["factionId"], "faction-BLUFOR");
    assert_eq!(
        vehs["p2"]["crew"]["driver"], "s0",
        "crew shape carried verbatim"
    );

    let small = small_maps(&doc);
    assert_eq!(small["entitiesById"]["p3"]["alias"], "sandbag_wall");
    assert_eq!(small["entitiesById"]["p3"]["faction"], "blufor");
    assert_eq!(small["entitiesById"]["p3"]["position"]["x"], drop_x - 30.5);

    assert!(doc.undo(), "the placement must be undoable");
    let slots_after: serde_json::Value =
        serde_json::from_str(&doc.slots_json()).expect("slots_json");
    assert!(
        slots_after.get("p0").is_none() && slots_after.get("p1").is_none(),
        "one undo must remove every placed slot: {slots_after}"
    );
    assert!(
        vehicles_of(&doc).get("p2").is_none(),
        "one undo must remove the placed vehicle too — it was the same step"
    );
    assert!(
        small_maps(&doc)["entitiesById"].get("p3").is_none(),
        "one undo must remove the placed object too"
    );
}

#[test]
fn a_composed_comment_is_placed_but_never_reaches_the_mod_document() {
    const TOKEN: &str = "CMT-COMPOSED-ZZQ";
    let mut doc = two_slots_visible_layer();
    let depth0 = doc.undo_depth();

    let entities = serde_json::json!([
        { "kind": "slot", "dx": -12.75, "dz": 8.5, "rotation": 45.5,
          "role": "Squad Leader", "tag": "SL", "assetId": "Prefab/SL.et", "stance": "crouch" },
        { "kind": "comment", "dx": 20.25, "dz": -6.5,
          "title": TOKEN, "tooltip": "the body of CMT-COMPOSED-ZZQ" },
    ])
    .to_string();
    let ids = vec!["p0".to_string(), "cmp0".to_string()];
    let (drop_x, drop_y) = (1_000.0, 2_000.0);
    let written = doc.place_composition(
        &entities, &ids, "BLUFOR", "L", drop_x, drop_y, 12_800.0, 12_800.0,
    );
    assert_eq!(
        written,
        vec!["p0".to_string(), "cmp0".to_string()],
        "the comment must be PLACED, not silently dropped — the whole defect"
    );

    let comments: serde_json::Value =
        serde_json::from_str(&doc.comments_json()).expect("comments_json");
    assert_eq!(comments["cmp0"]["title"], TOKEN, "{comments}");
    assert_eq!(comments["cmp0"]["tooltip"], "the body of CMT-COMPOSED-ZZQ");
    assert_eq!(comments["cmp0"]["position"]["x"], drop_x + 20.25);
    assert_eq!(comments["cmp0"]["position"]["z"], drop_y - 6.5);

    let filed = small_maps(&doc);
    let ents = filed["editorLayersById"]["L"]["entityIds"]
        .as_array()
        .expect("entityIds");
    assert!(
        ents.iter().any(|v| v == "cmp0") && ents.iter().any(|v| v == "p0"),
        "the composed comment files into the layer beside the composed slot: {ents:?}"
    );

    assert_eq!(
        doc.undo_depth(),
        depth0 + 1,
        "a composition place is exactly one undo step, comment included"
    );
    assert!(doc.undo(), "the placement must be undoable");
    assert_eq!(
        doc.comment_count(),
        0,
        "one Ctrl+Z removes the composed note as well as the composed slot"
    );
    assert!(doc.redo(), "the placement must be redoable");
    let after_redo: serde_json::Value =
        serde_json::from_str(&doc.comments_json()).expect("comments_json");
    assert_eq!(after_redo["cmp0"]["title"], TOKEN, "redo restores the note");
    assert_eq!(after_redo["cmp0"]["position"]["x"], drop_x + 20.25);

    let payload =
        mission_payload::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    let wire = payload["comments"]
        .as_array()
        .expect("comments[] at the editor-payload root");
    assert!(
        wire.iter().any(|c| c["title"] == TOKEN),
        "the composed note must ride the editor payload: {payload}"
    );

    let reloaded = MissionDocCore::new();
    reloaded.hydrate(&serde_json::to_string(&payload).expect("payload json"), "L");
    let restored: serde_json::Value =
        serde_json::from_str(&reloaded.comments_json()).expect("comments_json");
    assert_eq!(
        restored["cmp0"]["title"], TOKEN,
        "the composed note survives a restore: {restored}"
    );
    assert_eq!(restored["cmp0"]["position"]["x"], drop_x + 20.25);

    let meta = br#"{"id":"11112222333344445555666677778888","title":"t","author":"a",
            "terrain":"everon","customTerrainName":"","maxPlayers":8,"timeOfDay":"05:30",
            "weatherPreset":"clear"}"#;
    let mod_text = String::from_utf8(
        mission_compiler::flatten_mod_document_json(
            meta,
            &serde_json::to_vec(&payload).expect("payload bytes"),
        )
        .expect("flatten compiles"),
    )
    .expect("utf-8");
    assert!(
        !mod_text.contains(TOKEN),
        "a composed comment reached the compiled mission: {mod_text}"
    );

    let mut leaked = payload.clone();
    leaked["entities"] = serde_json::json!([{
        "id": "cmp0",
        "alias": TOKEN,
        "resourceName": "",
        "position": { "x": drop_x + 20.25, "z": drop_y - 6.5 },
        "faction": "",
    }]);
    let leaked_text = String::from_utf8(
        mission_compiler::flatten_mod_document_json(
            meta,
            &serde_json::to_vec(&leaked).expect("leaked bytes"),
        )
        .expect("leaked flatten compiles"),
    )
    .expect("utf-8");
    assert!(
        leaked_text.contains(TOKEN),
        "the leak probe did not reach the mod document, so step 3 proves nothing: {leaked_text}"
    );
}

#[test]
fn placing_a_composition_keeps_each_entrys_authored_elevation() {
    let doc = two_slots_visible_layer();

    let entities = serde_json::json!([
        { "kind": "slot",    "dx": -12.75, "dz": 8.5,    "rotation": 45.5,
          "role": "Squad Leader", "tag": "SL", "assetId": "Prefab/SL.et", "stance": "crouch",
          "elevation": 37.3 },
        { "kind": "vehicle", "dx": 0.5,    "dz": 30.125, "rotation": 270.75,
          "resourceName": "Prefab/Technical.et", "elevation": 121.5 },
        { "kind": "object",  "dx": -30.5,  "dz": 0.25,   "rotation": 90.0,
          "alias": "sandbag_wall", "resourceName": "Prefab/Sandbag.et", "faction": "blufor",
          "elevation": -4.25 },
        { "kind": "comment", "dx": 5.5,    "dz": -5.5,
          "title": "note", "tooltip": "a note has no height" },
        { "kind": "slot",    "dx": 12.25,  "dz": -8.5,   "rotation": 0.0,
          "role": "Rifleman", "tag": "", "assetId": "Prefab/Rifleman.et", "stance": "stand" },
    ])
    .to_string();
    let ids: Vec<String> = ["p0", "p1", "p2", "cmp0", "p3"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let written = doc.place_composition(
        &entities, &ids, "BLUFOR", "L", 1_000.0, 2_000.0, 12_800.0, 12_800.0,
    );
    assert_eq!(written.len(), 5, "every entry placed: {written:?}");

    fn z_of(row: &serde_json::Value) -> f64 {
        row["position"]["z"]
            .as_f64()
            .unwrap_or_else(|| panic!("numeric position.z on {row}"))
    }

    let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
    assert_eq!(z_of(&slots["p0"]), 37.3, "entry 0's own elevation");
    let vehs = vehicles_of(&doc);
    assert_eq!(z_of(&vehs["p1"]), 121.5, "entry 1's own elevation");
    let small = small_maps(&doc);
    assert_eq!(
        z_of(&small["entitiesById"]["p2"]),
        -4.25,
        "entry 2's own elevation"
    );

    assert_eq!(
        z_of(&slots["p3"]),
        0.0,
        "an entry carrying no elevation still places at ground"
    );

    assert_eq!(slots["p0"]["position"]["x"], 1_000.0 - 12.75);
    assert_eq!(slots["p0"]["position"]["y"], 2_000.0 + 8.5);

    let comments: serde_json::Value =
        serde_json::from_str(&doc.comments_json()).expect("comments_json");
    let pos = comments["cmp0"]["position"]
        .as_object()
        .expect("comment position object");
    let mut keys: Vec<&str> = pos.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["x", "z"],
        "a composed note keeps the two-horizontal marker shape: {pos:?}"
    );
    assert_eq!(comments["cmp0"]["position"]["x"], 1_000.0 + 5.5);
    assert_eq!(comments["cmp0"]["position"]["z"], 2_000.0 - 5.5);
}

#[test]
fn hidden_layer_slot_is_filtered_from_materialize_but_kept_in_the_doc() {
    let doc = one_slot_one_layer();
    assert_eq!(doc.materialize().len(), 1, "visible by default");

    doc.set_editor_layer_hidden("L", true);
    assert_eq!(
        doc.materialize().len(),
        0,
        "hidden layer's slot dropped from the render SoA"
    );

    let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
    assert_eq!(
        slots["s0"]["role"], "Rifleman",
        "slot survives hide: {slots}"
    );
    assert_eq!(slots["s0"]["position"]["x"], 100.0, "position untouched");

    doc.set_editor_layer_hidden("L", false);
    assert_eq!(doc.materialize().len(), 1, "un-hide restores the slot");
}
