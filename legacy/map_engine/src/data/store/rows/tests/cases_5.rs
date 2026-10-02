//! Role: Domain regression cases.
//! Position: `doc/store/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_zone_alone_counts_as_local_content() {
    let doc = MissionDocCore::new();
    assert!(!doc.has_content(), "fresh doc has no content");
    doc.add_circle_zone("z", "base_protection", 10.5, 20.5, 30.5);
    assert!(
        doc.has_content(),
        "a drawn play area is authored work and must count as local content"
    );
}

#[test]
fn an_unpaired_trailing_coordinate_is_dropped() {
    let doc = MissionDocCore::new();
    doc.add_polygon_zone("z", "boundary", &[0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5]);
    let zones: serde_json::Value = serde_json::from_str(&doc.zones_json()).expect("zones_json");
    let ring = zones["z"]["shape"]["polygon"]
        .as_array()
        .expect("polygon array");
    assert_eq!(ring.len(), 3, "expected 3 whole vertices: {ring:?}");
    for pt in ring {
        assert_eq!(pt.as_array().expect("vertex").len(), 2, "{pt:?}");
    }
}

#[cfg(feature = "scenario")]
#[test]
fn authored_triggers_survive_compile_hydrate_compile_whole() {
    let doc = triggers_fixture();

    let compiled = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
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

    let recompiled = crate::data::scenario::compile::compile_payload(
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
fn owner_edge_assigns_clears_and_tolerates_dangling() {
    let doc = MissionDocCore::new();
    doc.add_faction("f1", "BLUFOR", "US");
    doc.add_squad("sq1", "f1", "Alpha", Some("Alpha".to_string()));
    doc.add_slot(
        "s1", "sq1", "lyr", 0, "Rifleman", None, None, 10.5, 20.5, 0.0, 0.0,
    );
    doc.add_circle_trigger("t1", "presence", 50.5, 60.5, 25.5);

    let row = |d: &MissionDocCore| -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(&d.triggers_json()).expect("triggers_json")["t1"]
            .clone()
    };

    doc.set_trigger_owner("t1", Some("s1"));
    assert_eq!(row(&doc)["ownerId"], "s1", "owner edge did not record");

    doc.remove_slots(vec!["s1".to_string()]);
    assert_eq!(
        doc.trigger_count(),
        1,
        "deleting the owner must NOT delete the trigger (a dangling edge, not a cascade)"
    );
    assert_eq!(
        row(&doc)["ownerId"],
        "s1",
        "the dangling ownerId stays on the row; readers resolve it to nothing, they do not \
             rewrite it"
    );

    let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
    assert!(
        slots.get("s1").is_none(),
        "the owner slot must actually be removed for this to be a dangling edge: {slots}"
    );

    doc.set_trigger_owner("t1", None);
    assert!(
        row(&doc).get("ownerId").is_none(),
        "clearing the owner must remove the key: {}",
        row(&doc)
    );
}

#[test]
fn reshaping_a_trigger_keeps_metadata_and_one_oneof_branch() {
    let doc = MissionDocCore::new();
    doc.add_polygon_trigger("t", "radio", &[0.5, 0.5, 10.5, 0.5, 10.5, 10.5]);
    doc.set_trigger_name("t", Some("Extract"));
    doc.set_trigger_owner("t", Some("veh1"));
    doc.set_trigger_rules("t", Some(r#"{"holdSeconds":90.5}"#));

    doc.set_trigger_circle("t", 50.5, 60.5, 25.5);
    let row = |d: &MissionDocCore| -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(&d.triggers_json()).expect("triggers_json")["t"]
            .clone()
    };
    let r = row(&doc);
    let shape = r["shape"].as_object().expect("shape");
    assert!(
        shape.contains_key("circle"),
        "reshape did not take: {shape:?}"
    );
    assert!(
        !shape.contains_key("polygon"),
        "the polygon branch survived a reshape to circle — oneOf-invalid: {shape:?}"
    );

    assert_eq!(r["name"], "Extract", "reshape wiped the name");
    assert_eq!(r["ownerId"], "veh1", "reshape wiped the owner edge");
    assert_eq!(r["activation"], "radio", "reshape wiped the activation");
    assert_eq!(r["rules"]["holdSeconds"], 90.5, "reshape wiped the rules");
}

#[test]
fn clearing_optional_trigger_fields_removes_the_keys() {
    let doc = MissionDocCore::new();
    doc.add_circle_trigger("t", "presence", 5.5, 6.5, 7.5);
    doc.set_trigger_name("t", Some("Alarm"));
    doc.set_trigger_owner("t", Some("s9"));
    doc.set_trigger_rules("t", Some(r#"{"points":3.5}"#));

    let row = |d: &MissionDocCore| -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(&d.triggers_json()).expect("triggers_json")["t"]
            .clone()
    };
    let r = row(&doc);
    assert_eq!(r["name"], "Alarm");
    assert_eq!(r["ownerId"], "s9");
    assert_eq!(r["rules"]["points"], 3.5);
    assert_eq!(r["activation"], "presence", "activation is always present");

    doc.set_trigger_name("t", None);
    doc.set_trigger_owner("t", None);
    doc.set_trigger_rules("t", None);
    let r = row(&doc);
    assert!(r.get("name").is_none(), "name not removed: {r}");
    assert!(r.get("ownerId").is_none(), "ownerId not removed: {r}");
    assert!(r.get("rules").is_none(), "rules not removed: {r}");
    assert_eq!(
        r["activation"], "presence",
        "activation must survive clearing the optionals"
    );

    doc.set_trigger_rules("t", Some("{}"));
    assert!(
        row(&doc).get("rules").is_none(),
        "empty rules must not be written"
    );

    doc.set_trigger_activation("t", "timer");
    assert_eq!(
        row(&doc)["activation"],
        "timer",
        "activation retype did not take"
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

#[cfg(feature = "scenario")]
#[test]
fn deleting_every_trigger_clears_them_from_the_wire() {
    let doc = triggers_fixture();
    let reloaded = save_and_reload(&doc);
    assert_eq!(reloaded.trigger_count(), 2);

    reloaded.remove_trigger("t_amb");
    reloaded.remove_trigger("t_timer");
    assert_eq!(reloaded.trigger_count(), 0);

    let compiled = crate::data::scenario::compile::compile_payload(
        &reloaded.small_maps_json(),
        &reloaded.slots_json(),
        false,
    );
    assert!(
        wire_triggers(&compiled).is_empty(),
        "deleted triggers must not survive on the wire: {compiled}"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn saved_composition_survives_compile_hydrate_compile_whole() {
    let doc = compositions_fixture();

    let compiled = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
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

    let recompiled = crate::data::scenario::compile::compile_payload(
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

#[cfg(feature = "scenario")]
#[test]
fn composition_offset_perturbation_is_caught_by_the_round_trip() {
    let good = compositions_fixture();
    let good_wire = wire_compositions(&crate::data::scenario::compile::compile_payload(
        &good.small_maps_json(),
        &good.slots_json(),
        false,
    ));
    let expected: serde_json::Value =
        serde_json::from_str(&composition_row_json()).expect("expected");
    assert_eq!(
        canon(&good_wire[0]),
        canon(&expected),
        "baseline must match before perturbing"
    );

    let mut perturbed_row: serde_json::Value =
        serde_json::from_str(&composition_row_json()).expect("row");
    perturbed_row["entities"][0]["dx"] =
        serde_json::json!(perturbed_row["entities"][0]["dx"].as_f64().unwrap() + 100.0);
    let bad = compositions_fixture();
    bad.add_composition("c1", &perturbed_row.to_string());
    let bad_wire = wire_compositions(&crate::data::scenario::compile::compile_payload(
        &bad.small_maps_json(),
        &bad.slots_json(),
        false,
    ));
    assert_ne!(
        canon(&bad_wire[0]),
        canon(&expected),
        "a shifted offset must be OBSERVABLE in the compiled row — the by-value check is real"
    );

    bad.add_composition("c1", &composition_row_json());
    let restored_wire = wire_compositions(&crate::data::scenario::compile::compile_payload(
        &bad.small_maps_json(),
        &bad.slots_json(),
        false,
    ));
    assert_eq!(
        canon(&restored_wire[0]),
        canon(&expected),
        "restoring the honest offset must bring the row back to identical"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn compositions_by_id_and_extras_projection_agree_on_order() {
    let incoming = serde_json::json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon" },
        "environment": {},
        "compositions": [
            { "id": "c_c", "title": "C", "author": "a", "category": "x", "entities": [] },
            { "id": "c_a", "title": "A", "author": "a", "category": "x", "entities": [] },
            { "id": "c_b", "title": "B", "author": "a", "category": "x", "entities": [] }
        ],
        "editor": { "factions": [], "squads": [], "slots": [], "editorLayers": [] }
    });
    let doc = MissionDocCore::new();
    doc.hydrate(&incoming.to_string(), "lyr");

    let small = small_maps(&doc);
    let projected: Vec<&str> = small["payloadExtras"]["compositions"]
        .as_array()
        .expect("projected compositions array")
        .iter()
        .map(|c| c["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        projected,
        ["c_c", "c_a", "c_b"],
        "the projection must replay hydrate's authored order, not map order"
    );

    for id in &projected {
        assert_eq!(
            small["compositionsById"][id],
            *small["payloadExtras"]["compositions"]
                .as_array()
                .expect("arr")
                .iter()
                .find(|c| c["id"] == *id)
                .expect("row"),
            "compositionsById and the projection disagree about {id}"
        );
    }
    assert_eq!(
        small["compositionsById"]
            .as_object()
            .expect("compositionsById")
            .len(),
        3,
        "canonical by-id emit is missing rows"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn deleting_every_composition_clears_them_from_the_wire() {
    let doc = compositions_fixture();
    let reloaded = save_and_reload(&doc);
    assert_eq!(reloaded.composition_count(), 1);

    reloaded.remove_composition("c1");
    assert_eq!(reloaded.composition_count(), 0);

    let compiled = crate::data::scenario::compile::compile_payload(
        &reloaded.small_maps_json(),
        &reloaded.slots_json(),
        false,
    );
    assert!(
        wire_compositions(&compiled).is_empty(),
        "a deleted composition must not survive on the wire: {compiled}"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn composition_rename_recategorize_after_hydrate_preserves_entities() {
    let doc = save_and_reload(&compositions_fixture());

    let before = serde_json::from_str::<serde_json::Value>(&doc.compositions_json())
        .expect("compositions_json");
    assert_eq!(
        before["c1"]["entities"].as_array().expect("entities").len(),
        4,
        "the reloaded composition must carry all four entities"
    );

    doc.set_composition_title("c1", "Renamed Squad");
    doc.set_composition_category("c1", "Armor");

    let after = serde_json::from_str::<serde_json::Value>(&doc.compositions_json())
        .expect("compositions_json");
    assert_eq!(
        after["c1"]["title"], "Renamed Squad",
        "rename did not apply"
    );
    assert_eq!(
        after["c1"]["category"], "Armor",
        "recategorize did not apply"
    );
    assert_eq!(
        after["c1"]["author"], "Sam",
        "the untouched author metadata must survive the edit"
    );
    assert_eq!(
        after["c1"]["entities"], before["c1"]["entities"],
        "a metadata edit on a HYDRATED row must not wipe the captured entities"
    );
}

#[cfg(feature = "scenario")]
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

#[cfg(feature = "scenario")]
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

#[cfg(feature = "scenario")]
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

    let payload = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
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
        crate::data::scenario::flatten::flatten_mod_document_json(
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
        crate::data::scenario::flatten::flatten_mod_document_json(
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

#[cfg(feature = "scenario")]
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

#[cfg(feature = "scenario")]
#[test]
fn a_malformed_composition_json_writes_no_row() {
    let doc = MissionDocCore::new();
    doc.add_composition("c1", "\"just a string\"");
    assert_eq!(
        doc.composition_count(),
        0,
        "a non-object row must be refused"
    );
    doc.add_composition("c2", "not json at all");
    assert_eq!(doc.composition_count(), 0, "invalid JSON must be refused");
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
