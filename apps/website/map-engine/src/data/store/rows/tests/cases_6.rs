//! Role: Domain regression cases.
//! Position: `doc/store/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_hidden_slot_is_invisible_to_a_materialize_sourced_id_mint() {
    use std::collections::HashSet;

    fn fixture() -> MissionDocCore {
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_editor_layer("hid", "Hidden layer", None);
        doc.add_editor_layer("vis", "Visible layer", None);
        doc.add_slot(
            "n0",
            "sq",
            "hid",
            0,
            "RECON-LEAD",
            None,
            None,
            10.0,
            20.0,
            0.0,
            0.0,
        );
        doc.add_slot(
            "n1", "sq", "vis", 1, "SNIPER", None, None, 30.0, 40.0, 0.0, 0.0,
        );
        doc.add_slot(
            "n2", "sq", "vis", 2, "Rifleman", None, None, 50.0, 60.0, 0.0, 0.0,
        );
        doc.set_editor_layer_hidden("hid", true);
        doc.set_slot_editor_hidden("n1", true);
        doc.set_origin_init(false);
        doc
    }

    fn mint(existing: &HashSet<String>) -> String {
        let mut next = 0_u32;
        loop {
            let id = format!("n{next}");
            next += 1;
            if !existing.contains(&id) {
                return id;
            }
        }
    }
    let raw_ids = |doc: &MissionDocCore| -> HashSet<String> {
        serde_json::from_str::<serde_json::Value>(&doc.slots_json())
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default()
    };
    let role = |doc: &MissionDocCore, id: &str| -> String {
        let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
        slots[id]["role"].as_str().unwrap_or_default().to_string()
    };

    let doc = fixture();
    let soa: HashSet<String> = doc.materialize().ids.into_iter().collect();
    let raw = raw_ids(&doc);
    assert_eq!(
        soa,
        HashSet::from(["n2".to_string()]),
        "materialize() must drop both hidden slots (layer-hidden n0, editorHidden n1)"
    );
    assert_eq!(
        raw,
        HashSet::from(["n0".to_string(), "n1".to_string(), "n2".to_string()]),
        "slots_json is the EXACT row map — hidden rows are still in the doc"
    );

    let broken = fixture();
    let collided = mint(&soa);
    assert_eq!(
        collided, "n0",
        "an SoA-sourced universe cannot see the hidden slot's id"
    );
    broken.add_slot(
        &collided, "sq", "vis", 3, "Rifleman", None, None, 70.0, 80.0, 0.0, 0.0,
    );
    assert_eq!(
        role(&broken, "n0"),
        "Rifleman",
        "the hidden slot was silently overwritten — this is the destruction the fix prevents"
    );

    let fixed = fixture();
    let minted = mint(&raw);
    assert_eq!(minted, "n3", "the mint must skip every id the doc holds");
    assert!(
        !raw.contains(&minted),
        "the minted id must not collide with any live row, hidden or not"
    );
    fixed.add_slot(
        &minted, "sq", "vis", 3, "Rifleman", None, None, 70.0, 80.0, 0.0, 0.0,
    );
    assert_eq!(
        role(&fixed, "n0"),
        "RECON-LEAD",
        "the layer-hidden slot survives the placement"
    );
    assert_eq!(
        role(&fixed, "n1"),
        "SNIPER",
        "the editorHidden slot survives the placement"
    );
    assert_eq!(
        raw_ids(&fixed).len(),
        4,
        "the placement ADDED a row rather than replacing one"
    );
}

#[test]
fn locked_layer_refuses_move_entities_then_unlock_allows_it() {
    let doc = one_slot_one_layer();

    doc.set_editor_layer_locked("L", true);
    doc.move_entities(vec!["s0".to_string()], 50.0, 60.0, vec![0.0]);
    let soa = doc.materialize();
    let i = row_of(&soa, "s0");
    assert_eq!(
        (soa.xs[i], soa.ys[i]),
        (100.0, 200.0),
        "locked slot did not move"
    );

    doc.set_editor_layer_locked("L", false);
    doc.move_entities(vec!["s0".to_string()], 50.0, 60.0, vec![0.0]);
    let soa = doc.materialize();
    let i = row_of(&soa, "s0");
    assert_eq!(
        (soa.xs[i], soa.ys[i]),
        (150.0, 260.0),
        "unlocked slot moved"
    );
}

#[test]
fn locked_layer_refuses_slot_in_mixed_move_but_vehicle_still_moves() {
    let doc = one_slot_one_layer();
    doc.set_origin_init(true);
    doc.add_vehicle(
        "v0",
        "Prefab/Vehicle.et",
        Some(300.0),
        Some(400.0),
        Some(0.0),
        Some(0.0),
    );
    doc.set_origin_init(false);

    doc.set_editor_layer_locked("L", true);
    doc.move_entities_and_vehicles(
        vec!["s0".to_string()],
        &["v0".to_string()],
        10.0,
        20.0,
        vec![0.0],
    );
    let soa = doc.materialize();
    let i = row_of(&soa, "s0");
    assert_eq!(
        (soa.xs[i], soa.ys[i]),
        (100.0, 200.0),
        "locked slot stayed put"
    );
    let vehs = vehicles_of(&doc);
    assert_eq!(vehs["v0"]["position"]["x"], 310.0, "vehicle still moved");
    assert_eq!(vehs["v0"]["position"]["y"], 420.0, "vehicle still moved");
}

#[test]
fn slot_layer_is_locked_agrees_with_the_update_slot_position_refusal() {
    let doc = one_slot_one_layer();
    doc.add_slot(
        "unfiled", "sq", "", 1, "Rifleman", None, None, 10.0, 20.0, 0.0, 0.0,
    );
    assert!(!doc.slot_layer_is_locked("s0"), "unlocked by default");

    doc.set_editor_layer_locked("L", true);
    assert!(doc.slot_layer_is_locked("s0"), "layer locked ⇒ slot locked");
    assert!(
        !doc.slot_layer_is_locked("unfiled"),
        "a slot in no layer is never locked"
    );
    doc.update_slot_position("s0", Some(777.0), Some(888.0), None, None, 12800.0, 12800.0);
    let soa = doc.materialize();
    let i = row_of(&soa, "s0");
    assert_eq!(
        (soa.xs[i], soa.ys[i]),
        (100.0, 200.0),
        "the query said locked and the mutator refused"
    );

    doc.set_editor_layer_locked("L", false);
    assert!(
        !doc.slot_layer_is_locked("s0"),
        "unlock is visible to the query"
    );
    doc.update_slot_position("s0", Some(777.0), Some(888.0), None, None, 12800.0, 12800.0);
    let soa = doc.materialize();
    let i = row_of(&soa, "s0");
    assert_eq!(
        (soa.xs[i], soa.ys[i]),
        (777.0, 888.0),
        "the query said unlocked and the mutator accepted"
    );
}

#[test]
fn update_slot_object_sets_clears_and_leaves_none_fields_alone() {
    let doc = one_slot_one_layer();
    doc.update_slot("s0", None, Some("MED".into()), None);

    doc.update_slot_object(
        "s0",
        Some("Character_US_Rifleman".into()),
        Some("Point man. Takes the lead on entry.".into()),
    );
    let row = slots_map(&doc);
    assert_eq!(row["s0"]["assetId"], "Character_US_Rifleman");
    assert_eq!(
        row["s0"]["description"],
        "Point man. Takes the lead on entry."
    );

    doc.update_slot_object("s0", None, Some("Now the breacher.".into()));
    let row = slots_map(&doc);
    assert_eq!(
        row["s0"]["assetId"], "Character_US_Rifleman",
        "a None assetId left the type alone"
    );
    assert_eq!(row["s0"]["description"], "Now the breacher.");
    assert_eq!(row["s0"]["role"], "Rifleman", "role untouched");
    assert_eq!(row["s0"]["tag"], "MED", "tag untouched");

    doc.update_slot_object("s0", Some(String::new()), None);
    let row = slots_map(&doc);
    assert!(
        row["s0"].get("assetId").is_none(),
        "empty clears the key rather than storing \"\""
    );
    assert_eq!(row["s0"]["description"], "Now the breacher.");

    doc.update_slot_object("s0", None, None);
    let row = slots_map(&doc);
    assert_eq!(row["s0"]["description"], "Now the breacher.");
    assert_eq!(row["s0"]["role"], "Rifleman");
}

#[test]
fn locked_layer_refuses_update_slot_position_then_unlock_allows_it() {
    let doc = one_slot_one_layer();

    doc.set_editor_layer_locked("L", true);
    doc.update_slot_position("s0", Some(777.0), Some(888.0), None, None, 12800.0, 12800.0);
    let soa = doc.materialize();
    let i = row_of(&soa, "s0");
    assert_eq!(
        (soa.xs[i], soa.ys[i]),
        (100.0, 200.0),
        "locked slot refused the Attributes edit"
    );

    doc.set_editor_layer_locked("L", false);
    doc.update_slot_position("s0", Some(777.0), Some(888.0), None, None, 12800.0, 12800.0);
    let soa = doc.materialize();
    let i = row_of(&soa, "s0");
    assert_eq!(
        (soa.xs[i], soa.ys[i]),
        (777.0, 888.0),
        "unlocked slot took the edit"
    );
}

#[test]
fn child_layer_inherits_parent_hidden_and_locked() {
    let doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_editor_layer("parent", "Parent", None);
    doc.add_editor_layer("child", "Child", Some("parent".to_string()));
    doc.add_slot(
        "s0", "sq", "child", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    doc.set_origin_init(false);
    assert_eq!(doc.materialize().len(), 1, "visible before any flag");

    doc.set_editor_layer_hidden("parent", true);
    assert_eq!(doc.materialize().len(), 0, "child inherits parent's hidden");
    let layers: serde_json::Value =
        serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json");
    assert!(
        layers["editorLayersById"]["child"].get("hidden").is_none(),
        "flag not copied down onto the child row: {}",
        layers["editorLayersById"]["child"]
    );
    doc.set_editor_layer_hidden("parent", false);
    assert_eq!(doc.materialize().len(), 1, "un-hiding parent reveals child");

    doc.set_editor_layer_locked("parent", true);
    doc.move_entities(vec!["s0".to_string()], 5.0, 5.0, vec![0.0]);
    let soa = doc.materialize();
    let i = row_of(&soa, "s0");
    assert_eq!(
        (soa.xs[i], soa.ys[i]),
        (100.0, 200.0),
        "child inherits parent's lock"
    );
}

#[test]
fn hidden_and_locked_flag_flips_are_one_undo_step_each() {
    let mut doc = one_slot_one_layer();
    assert_eq!(doc.undo_depth(), 0, "INIT setup is not on the stack");

    doc.set_editor_layer_hidden("L", true);
    assert_eq!(doc.undo_depth(), 1, "hide is one LOCAL step");
    assert_eq!(doc.materialize().len(), 0, "hidden now");
    assert!(doc.undo());
    assert_eq!(doc.materialize().len(), 1, "undo un-hid the layer");
    assert_eq!(doc.undo_depth(), 0);

    doc.set_editor_layer_locked("L", true);
    assert_eq!(doc.undo_depth(), 1, "lock is one LOCAL step");
    assert!(doc.undo());
    assert_eq!(doc.undo_depth(), 0);

    doc.move_entities(vec!["s0".to_string()], 3.0, 4.0, vec![0.0]);
    let soa = doc.materialize();
    let i = row_of(&soa, "s0");
    assert_eq!(
        (soa.xs[i], soa.ys[i]),
        (103.0, 204.0),
        "undo removed the lock"
    );
}

#[test]
fn clearing_a_layer_flag_removes_the_key() {
    let doc = one_slot_one_layer();
    doc.set_editor_layer_hidden("L", true);
    doc.set_editor_layer_locked("L", true);
    doc.set_editor_layer_hidden("L", false);
    doc.set_editor_layer_locked("L", false);
    let layers: serde_json::Value =
        serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json");
    let row = &layers["editorLayersById"]["L"];
    assert!(row.get("hidden").is_none(), "hidden key removed: {row}");
    assert!(row.get("locked").is_none(), "locked key removed: {row}");
}

#[cfg(feature = "scenario")]
#[test]
fn editor_hidden_rides_the_row_only_when_true_and_filters_materialize() {
    let doc = two_slots_visible_layer();
    assert_eq!(doc.materialize().len(), 2, "both visible by default");

    let before: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
    assert!(
        before["s1"].get("editorHidden").is_none(),
        "flag omitted until set: {}",
        before["s1"]
    );

    doc.set_slot_editor_hidden("s1", true);
    let soa = doc.materialize();
    assert_eq!(
        soa.ids.len(),
        1,
        "hidden entity dropped from the render SoA"
    );
    assert_eq!(soa.ids[0], "s0", "the un-hidden entity survives");

    let after: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
    assert_eq!(
        after["s1"]["editorHidden"], true,
        "flag on the row: {}",
        after["s1"]
    );
    assert_eq!(after["s1"]["role"], "Rifleman", "row survives hide");
    assert_eq!(after["s1"]["position"]["x"], 110.0, "position untouched");

    doc.set_slot_editor_hidden("s1", false);
    assert_eq!(doc.materialize().len(), 2, "un-hide restores the entity");
    let cleared: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
    assert!(
        cleared["s1"].get("editorHidden").is_none(),
        "false removes the key: {}",
        cleared["s1"]
    );
}

#[cfg(feature = "scenario")]
#[test]
fn effective_hidden_is_layer_or_entity() {
    let doc = two_slots_visible_layer();

    assert_eq!(
        ids_sorted(&doc.materialize()),
        vec!["s0", "s1"],
        "both visible"
    );

    doc.set_slot_editor_hidden("s1", true);
    assert_eq!(
        ids_sorted(&doc.materialize()),
        vec!["s0"],
        "entity flag hides s1"
    );

    doc.set_editor_layer_hidden("L", true);
    assert!(
        doc.materialize().ids.is_empty(),
        "layer OR entity hides both"
    );

    doc.set_slot_editor_hidden("s1", false);
    assert!(
        doc.materialize().ids.is_empty(),
        "layer alone still hides both even with entity flags clear"
    );

    doc.set_editor_layer_hidden("L", false);
    assert_eq!(
        ids_sorted(&doc.materialize()),
        vec!["s0", "s1"],
        "revealing the layer restores the un-flagged entities"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn editor_hidden_survives_hydrate_round_trip() {
    let doc = two_slots_visible_layer();
    doc.set_slot_editor_hidden("s1", true);
    let payload = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );

    let reloaded = MissionDocCore::new();
    reloaded.hydrate(&serde_json::to_string(&payload).expect("payload json"), "L");

    let slots: serde_json::Value =
        serde_json::from_str(&reloaded.slots_json()).expect("slots_json");
    assert_eq!(
        slots["s1"]["editorHidden"], true,
        "flag reloaded: {}",
        slots["s1"]
    );
    assert!(
        slots["s0"].get("editorHidden").is_none(),
        "un-hidden slot has no key after reload: {}",
        slots["s0"]
    );

    assert_eq!(
        ids_sorted(&reloaded.materialize()),
        vec!["s0"],
        "hidden entity stays hidden after reload"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn editor_hidden_never_reaches_mod_wire() {
    let doc = two_slots_visible_layer();
    doc.set_slot_editor_hidden("s1", true);

    let payload = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    let s1 = payload["editor"]["slots"]
        .as_array()
        .expect("editor.slots")
        .iter()
        .find(|s| s["id"] == "s1")
        .expect("s1 in editor.slots");
    assert_eq!(
        s1["editorHidden"], true,
        "editor block keeps the flag: {s1}"
    );

    let meta = br#"{"id":"11112222333344445555666677778888","title":"t","author":"a",
            "terrain":"everon","customTerrainName":"","maxPlayers":8,"timeOfDay":"05:30",
            "weatherPreset":"clear"}"#;
    let payload_bytes = serde_json::to_vec(&payload).expect("payload bytes");
    let mod_bytes = crate::data::scenario::flatten::flatten_mod_document_json(meta, &payload_bytes)
        .expect("flatten compiles");
    let mod_text = String::from_utf8(mod_bytes).expect("utf-8");
    assert!(
        !mod_text.contains("editorHidden"),
        "editorHidden leaked onto the MOD wire: {mod_text}"
    );

    let mut perturbed = payload.clone();
    for slot in perturbed["editor"]["slots"]
        .as_array_mut()
        .expect("editor.slots")
    {
        slot["editorHidden"] = serde_json::Value::Bool(true);
    }
    let perturbed_bytes = serde_json::to_vec(&perturbed).expect("perturbed bytes");
    let perturbed_mod =
        crate::data::scenario::flatten::flatten_mod_document_json(meta, &perturbed_bytes)
            .expect("perturbed flatten compiles");
    let perturbed_text = String::from_utf8(perturbed_mod).expect("utf-8");
    assert!(
        !perturbed_text.contains("editorHidden"),
        "perturbed editorHidden must STILL be stripped by SlotIn/ModSlot: {perturbed_text}"
    );

    let clean = two_slots_visible_layer();
    let clean_payload = crate::data::scenario::compile::compile_payload(
        &clean.small_maps_json(),
        &clean.slots_json(),
        false,
    );
    let clean_bytes = serde_json::to_vec(&clean_payload).expect("clean bytes");
    let clean_mod = crate::data::scenario::flatten::flatten_mod_document_json(meta, &clean_bytes)
        .expect("clean flatten compiles");
    assert!(
        !String::from_utf8(clean_mod)
            .expect("utf-8")
            .contains("editorHidden"),
        "control: a doc with no hidden entity has no editorHidden token on the wire"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn editor_hidden_flip_is_one_undo_step() {
    let mut doc = two_slots_visible_layer();
    assert_eq!(doc.undo_depth(), 0, "INIT setup is not on the stack");

    doc.set_slot_editor_hidden("s1", true);
    assert_eq!(doc.undo_depth(), 1, "hide is one LOCAL step");
    assert_eq!(doc.materialize().len(), 1, "hidden now");

    assert!(doc.undo());
    assert_eq!(doc.materialize().len(), 2, "undo un-hid the entity");
    assert_eq!(doc.undo_depth(), 0, "one flip = one step");
}

#[cfg(feature = "scenario")]
#[test]
fn show_all_clears_every_flag_in_one_txn() {
    let mut doc = two_slots_visible_layer();
    doc.set_slot_editor_hidden("s0", true);
    doc.set_slot_editor_hidden("s1", true);
    assert_eq!(doc.undo_depth(), 2, "two individual hides = two steps");
    assert!(doc.materialize().ids.is_empty(), "both hidden");

    let cleared = doc.clear_all_editor_hidden();
    assert_eq!(cleared, 2, "both entities un-hidden");
    assert_eq!(doc.undo_depth(), 3, "show-all is exactly ONE more step");
    assert_eq!(doc.materialize().len(), 2, "everything visible again");
    let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
    assert!(slots["s0"].get("editorHidden").is_none(), "s0 key removed");
    assert!(slots["s1"].get("editorHidden").is_none(), "s1 key removed");

    assert!(doc.undo());
    assert!(
        doc.materialize().ids.is_empty(),
        "one undo restored the whole reveal-all: both hidden again"
    );
    assert_eq!(doc.undo_depth(), 2, "back to the two individual hides");
}

#[cfg(feature = "scenario")]
#[test]
fn batch_hide_selection_is_one_undo_step() {
    let mut doc = two_slots_visible_layer();
    doc.set_slots_editor_hidden(&["s0".to_string(), "s1".to_string()], true);
    assert_eq!(
        doc.undo_depth(),
        1,
        "hiding the whole selection is ONE step"
    );
    assert!(doc.materialize().ids.is_empty(), "both hidden by the batch");

    assert!(doc.undo());
    assert_eq!(
        doc.materialize().len(),
        2,
        "one undo un-hid the whole batch"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn editor_hidden_survives_an_unrelated_slot_edit() {
    let doc = two_slots_visible_layer();
    doc.set_slot_editor_hidden("s1", true);

    doc.update_slot(
        "s1",
        Some("MED".to_string()),
        None,
        Some("prone".to_string()),
    );
    assert_eq!(
        doc.materialize().len(),
        1,
        "s1 still hidden after an unrelated edit"
    );
    let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
    assert_eq!(
        slots["s1"]["editorHidden"], true,
        "flag preserved: {}",
        slots["s1"]
    );
    assert_eq!(
        slots["s1"]["role"], "MED",
        "the unrelated edit still landed"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn merge_into_empty_doc_lands_everything() {
    let payload = template_payload_blufor_alpha();
    let doc = MissionDocCore::new();
    let report = doc.merge_mission_payload(&payload, MergeOpts::default());

    assert_eq!(report.slots_added, 2, "both slots landed");
    assert_eq!(report.factions_created, 1);
    assert_eq!(report.squads_created, 1);
    assert_eq!(report.vehicles_added, 1);
    assert!(
        report.skipped.is_empty(),
        "clean payload: {:?}",
        report.skipped
    );

    let root = small_maps(&doc);
    let squads = root["squadsById"].as_object().expect("squads");
    assert_eq!(squads.len(), 1);
    let (_sq_id, squad) = squads.iter().next().unwrap();
    let slot_ids = squad["slotIds"].as_array().expect("slotIds");
    assert_eq!(slot_ids.len(), 2, "squad owns both merged slots");

    let slots = slots_map(&doc);
    for sid in slot_ids {
        let sid = sid.as_str().unwrap();
        assert!(
            slots.get(sid).is_some(),
            "slotIds entry {sid} is a real slot"
        );
    }

    let leader = squad["leaderSlotId"].as_str().expect("leaderSlotId");
    assert!(
        slot_ids.iter().any(|s| s.as_str() == Some(leader)),
        "leaderSlotId points at a member slot"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn merge_dedups_squad_by_name_and_side() {
    let doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_editor_layer("lyr", "Layer", None);
    doc.add_faction("faction-BLUFOR", "BLUFOR", "1st Battalion");
    doc.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
    doc.add_slot(
        "res0", "sq-a", "lyr", 0, "SL", None, None, 1.0, 2.0, 0.0, 0.0,
    );
    doc.set_leader("sq-a", "res0");
    doc.set_origin_init(false);

    let src = MissionDocCore::new();
    src.set_origin_init(true);
    src.add_editor_layer("lyr", "Layer", None);
    src.add_faction("faction-BLUFOR", "BLUFOR", "1st Battalion");
    src.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
    src.add_squad("sq-b", "faction-BLUFOR", "Bravo", None);
    src.add_slot(
        "s0", "sq-a", "lyr", 0, "Rifleman", None, None, 5.0, 6.0, 0.0, 0.0,
    );
    src.add_slot(
        "s1", "sq-b", "lyr", 0, "Rifleman", None, None, 7.0, 8.0, 0.0, 0.0,
    );
    src.set_leader("sq-a", "s0");
    src.set_leader("sq-b", "s1");
    src.set_origin_init(false);
    let payload = crate::data::scenario::compile::compile_payload(
        &src.small_maps_json(),
        &src.slots_json(),
        false,
    );

    let report = doc.merge_mission_payload(&payload, MergeOpts::default());
    assert_eq!(report.squads_merged, 1, "Alpha deduped onto resident");
    assert_eq!(report.squads_created, 1, "Bravo created");
    assert_eq!(report.factions_merged, 1, "BLUFOR deduped onto resident");
    assert_eq!(report.factions_created, 0);
    assert_eq!(report.slots_added, 2);

    let root = small_maps(&doc);
    let squads = root["squadsById"].as_object().expect("squads");
    assert_eq!(
        squads.len(),
        2,
        "one resident Alpha + one new Bravo, not two Alphas"
    );

    let alpha = &root["squadsById"]["sq-a"];
    assert_eq!(
        alpha["slotIds"].as_array().unwrap().len(),
        2,
        "incoming Alpha slot merged into resident Alpha: {alpha}"
    );

    assert_eq!(root["factionsById"].as_object().unwrap().len(), 1);
}

#[cfg(feature = "scenario")]
#[test]
fn merge_remints_references_consistently() {
    let doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_editor_layer("lyr", "Layer", None);
    doc.add_faction("faction-OPFOR", "OPFOR", "Resident");
    doc.add_squad("sq", "faction-OPFOR", "Resident Squad", None);
    doc.add_slot("s0", "sq", "lyr", 0, "SL", None, None, 9.0, 9.0, 0.0, 0.0);
    doc.set_leader("sq", "s0");
    doc.add_vehicle("v0", "Prefab/Resident.et", Some(1.0), Some(1.0), None, None);
    doc.set_origin_init(false);

    let src = MissionDocCore::new();
    src.set_origin_init(true);
    src.add_editor_layer("lyr", "Layer", None);
    src.add_faction("faction-BLUFOR", "BLUFOR", "Incoming");
    src.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
    src.add_slot(
        "s0", "sq-a", "lyr", 0, "SL", None, None, 100.0, 100.0, 0.0, 0.0,
    );
    src.set_leader("sq-a", "s0");
    src.add_vehicle("v0", "Prefab/Truck.et", Some(2.0), Some(2.0), None, None);
    src.assign_crew_seat("v0", "driver", "s0");
    src.add_circle_trigger("t0", "presence", 3.0, 3.0, 5.0);
    src.set_trigger_owner("t0", Some("s0"));
    src.set_origin_init(false);
    let payload = crate::data::scenario::compile::compile_payload(
        &src.small_maps_json(),
        &src.slots_json(),
        false,
    );

    let report = doc.merge_mission_payload(&payload, MergeOpts::default());
    assert_eq!(report.slots_added, 1);
    assert_eq!(report.vehicles_added, 1);
    assert_eq!(report.triggers_added, 1);

    let root = small_maps(&doc);

    let merged_slot_id = root["squadsById"]
        .as_object()
        .unwrap()
        .values()
        .find(|sq| sq["name"] == "Alpha")
        .and_then(|sq| sq["slotIds"][0].as_str())
        .expect("merged Alpha slot")
        .to_string();
    assert_ne!(
        merged_slot_id, "s0",
        "merged slot id must be re-minted, not the resident s0"
    );

    let merged_vehicle = root["vehiclesById"]
        .as_object()
        .unwrap()
        .values()
        .find(|v| v["resourceName"] == "Prefab/Truck.et")
        .expect("merged truck");
    assert_eq!(
        merged_vehicle["crew"]["driver"].as_str(),
        Some(merged_slot_id.as_str()),
        "crew seat re-minted to the merged slot: {merged_vehicle}"
    );

    let trig = root["triggersById"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(
        trig["ownerId"].as_str(),
        Some(merged_slot_id.as_str()),
        "trigger ownerId re-minted to the merged slot: {trig}"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn merge_is_one_undo_step_and_undo_restores_exactly() {
    let mut doc = seeded_core();

    let before_small = small_maps(&doc);
    let before_digest = slots_digest(&doc.materialize());

    let payload = template_payload_blufor_alpha();
    let report = doc.merge_mission_payload(&payload, MergeOpts::default());
    assert!(report.slots_added > 0, "merge changed the doc");
    assert_eq!(
        doc.undo_depth(),
        1,
        "the whole merge is exactly one undo step"
    );
    assert_ne!(
        slots_digest(&doc.materialize()),
        before_digest,
        "merge added slots"
    );

    assert!(doc.undo(), "undo the merge");
    assert_eq!(
        small_maps(&doc),
        before_small,
        "one undo restores the pre-merge document exactly"
    );
    assert_eq!(
        slots_digest(&doc.materialize()),
        before_digest,
        "undo restores the exact pre-merge slot bits"
    );
    assert!(!doc.can_undo(), "the merge was the only stack item");
}
