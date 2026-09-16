//! Role: Domain regression cases.
//! Position: `doc/store/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[cfg(feature = "scenario")]
#[test]
fn connections_never_reach_the_mod_document() {
    let doc = two_slots_visible_layer();
    assert!(doc.add_connection("conn-1", "triggerOwner", "s1", "s0"));

    let payload = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    let conns = payload["connections"]
        .as_array()
        .expect("connections[] at the editor-payload root");
    assert_eq!(conns.len(), 1, "one drawn edge: {payload}");
    assert_eq!(conns[0]["kind"], "triggerOwner");
    assert_eq!(conns[0]["from"], "s1", "a DIRECTED kind stores verbatim");
    assert_eq!(conns[0]["to"], "s0");

    let reloaded = MissionDocCore::new();
    reloaded.hydrate(&serde_json::to_string(&payload).expect("payload json"), "L");
    assert_eq!(reloaded.connection_count(), 1);
    let rows: serde_json::Value =
        serde_json::from_str(&reloaded.connection_rows_json()).expect("rows");
    assert_eq!(rows[0]["id"], "conn-1", "edge reloaded: {rows}");

    let meta = br#"{"id":"11112222333344445555666677778888","title":"t","author":"a",
            "terrain":"everon","customTerrainName":"","maxPlayers":8,"timeOfDay":"05:30",
            "weatherPreset":"clear"}"#;
    let payload_bytes = serde_json::to_vec(&payload).expect("payload bytes");
    let mod_text = String::from_utf8(
        crate::data::scenario::flatten::flatten_mod_document_json(meta, &payload_bytes)
            .expect("flatten compiles"),
    )
    .expect("utf-8");
    assert!(
        !mod_text.contains("triggerOwner") && !mod_text.contains("connections"),
        "a connection reached the compiled mission: {mod_text}"
    );

    let mut leaked = payload.clone();
    leaked["entities"] = serde_json::json!([
        {
            "id": "leak1",
            "alias": "triggerOwner",
            "position": { "x": 1.0, "z": 2.0 },
            "faction": "",
        },
        {
            "id": "leak2",
            "alias": "connections",
            "position": { "x": 3.0, "z": 4.0 },
            "faction": "",
        },
    ]);
    let leaked_text = String::from_utf8(
        crate::data::scenario::flatten::flatten_mod_document_json(
            meta,
            &serde_json::to_vec(&leaked).expect("leaked bytes"),
        )
        .expect("leaked flatten compiles"),
    )
    .expect("utf-8");
    assert!(
        leaked_text.contains("triggerOwner") && leaked_text.contains("connections"),
        "the leak probe did not reach the mod document, so step 3 proves nothing: {leaked_text}"
    );

    doc.remove_connection("conn-1");
    let after = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    assert!(
        after
            .get("connections")
            .is_none_or(|c| c.as_array().is_some_and(Vec::is_empty)),
        "a deleted connection must not be re-emitted from the parked copy: {after}"
    );
}

#[test]
fn formation_offsets_are_distinct_per_schema_token_and_fall_back_to_column() {
    const TOKENS: [&str; 9] = [
        "column",
        "stagger_column",
        "wedge",
        "echelon_left",
        "echelon_right",
        "vee",
        "line",
        "file",
        "diamond",
    ];
    let shapes: Vec<Vec<(f64, f64)>> = TOKENS.iter().map(|t| formation_offsets(t, 6)).collect();
    for (i, a) in shapes.iter().enumerate() {
        assert_eq!(a.len(), 6, "{}: one offset per member", TOKENS[i]);
        for (j, b) in shapes.iter().enumerate().skip(i + 1) {
            assert_ne!(
                a, b,
                "`{}` and `{}` are the same shape",
                TOKENS[i], TOKENS[j]
            );
        }
    }

    for (i, shape) in shapes.iter().enumerate() {
        assert!(
            !shape.contains(&(0.0, 0.0)),
            "`{}` puts a member on the leader",
            TOKENS[i]
        );
    }
    assert_eq!(
        formation_offsets("no_such_formation", 3),
        formation_offsets("column", 3),
        "an unknown token falls back to column"
    );
    assert_eq!(
        formation_offsets("column", 0),
        Vec::new(),
        "no members, no offsets"
    );

    assert_eq!(
        formation_offsets("column", 3),
        vec![
            (0.0, -FORMATION_SPACING_M),
            (0.0, -2.0 * FORMATION_SPACING_M),
            (0.0, -3.0 * FORMATION_SPACING_M),
        ]
    );

    let schema = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/tbd-schema/schema/mission.schema.json"
    ));
    let force = format!("{}{}", "force_to_", "formation");
    let offsets = format!("{}{}", "formation_", "offsets");
    assert!(
        schema.contains(&force),
        "mission.schema.json formation description must mention force_to_formation"
    );
    assert!(
        schema.contains(&offsets),
        "mission.schema.json formation description must mention formation_offsets"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn force_to_formation_anchors_the_leader_rotates_by_heading_and_is_one_undo_step() {
    let mut doc = two_slots_visible_layer();
    doc.set_origin_init(true);
    doc.add_slot(
        "s2", "sq", "L", 2, "Rifleman", None, None, 999.0, 999.0, 0.0, 0.0,
    );

    doc.set_slot_position("s0", 1_000.0, 2_000.0, 0.0, 90.0);
    doc.set_origin_init(false);

    let depth = doc.undo_depth();
    assert_eq!(
        doc.force_to_formation("s0", "column"),
        2,
        "both members moved"
    );
    assert_eq!(
        doc.undo_depth(),
        depth + 1,
        "a re-form is ONE transaction, not one per member"
    );

    let at = |d: &MissionDocCore, id: &str| {
        let slots: serde_json::Value = serde_json::from_str(&d.slots_json()).expect("slots_json");
        (
            slots[id]["position"]["x"].as_f64().expect("x"),
            slots[id]["position"]["y"].as_f64().expect("y"),
        )
    };
    assert_eq!(
        at(&doc, "s0"),
        (1_000.0, 2_000.0),
        "the LEADER is the anchor and does not move"
    );

    let (x1, y1) = at(&doc, "s1");
    assert!(
        (x1 - (1_000.0 - FORMATION_SPACING_M)).abs() < 1e-9 && (y1 - 2_000.0).abs() < 1e-9,
        "member 1 must trail along the leader's heading, got ({x1}, {y1})"
    );
    let (x2, y2) = at(&doc, "s2");
    assert!(
        (x2 - (1_000.0 - 2.0 * FORMATION_SPACING_M)).abs() < 1e-9 && (y2 - 2_000.0).abs() < 1e-9,
        "member 2 must trail twice as far, got ({x2}, {y2})"
    );

    assert!(doc.undo(), "the re-form is on the undo stack");
    assert_eq!(
        at(&doc, "s2"),
        (999.0, 999.0),
        "one Ctrl+Z restores every re-formed unit"
    );

    assert_eq!(doc.force_to_formation("s1", "wedge"), 0);
    assert_eq!(doc.force_to_formation("", "wedge"), 0);
    assert_eq!(doc.force_to_formation("not-a-slot", "wedge"), 0);
}

#[test]
fn materialize_drops_hidden_slots_the_document_still_holds() {
    let doc = two_sided_core(4);
    doc.add_editor_layer("layer-hidden", "Stashed", None);
    doc.add_slot(
        "on-hidden-layer",
        "sq-blu",
        "layer-hidden",
        9,
        "Rifleman",
        None,
        None,
        5.0,
        5.0,
        0.0,
        0.0,
    );
    doc.set_editor_layer_hidden("layer-hidden", true);
    doc.set_slot_editor_hidden("n0", true);

    let soa = doc.materialize();
    for gone in ["on-hidden-layer", "n0"] {
        assert!(
            !soa.ids.iter().any(|s| s == gone),
            "T-937.3: `{gone}` must be absent from the materialized view"
        );
        assert!(
            doc.slots_json().contains(gone),
            "T-937.3: … while the document still holds `{gone}` verbatim — that gap is the \
                 whole defect: an existence check sourced from the SoA answers NO for work the \
                 mission will happily save"
        );
    }
}

#[test]
fn materialize_resolves_each_distinct_side_once_over_500_slots() {
    let doc = two_sided_core(500);
    let before = doc.side_key_resolution_count();
    let soa = doc.materialize();
    let spent = doc.side_key_resolution_count() - before;

    assert_eq!(
        soa.ids.len(),
        500,
        "the fixture must materialize all 500 rows"
    );
    assert!(
        spent >= 1,
        "T-937.3: a cold document resolved {spent} side keys — a zero means this call read a \
             memo it must have invalidated, and the bound below would then pass vacuously"
    );
    assert!(
        spent <= 2,
        "T-937.3: 500 slots over 2 distinct sides took {spent} side-key resolutions; \
             materialize must resolve each distinct side once per call"
    );
    assert_eq!(
        soa.side_keys.iter().filter(|k| *k == "BLUFOR").count(),
        250,
        "T-937.3: the memo must not change WHICH side each row gets"
    );
    assert_eq!(
        soa.side_keys.iter().filter(|k| *k == "OPFOR").count(),
        250,
        "T-937.3: the memo must not change WHICH side each row gets"
    );
}

#[test]
fn slot_attrs_exists_reads_the_raw_map_not_the_materialized_view() {
    let ops = strip_rust_lexical_noise(include_str!("../../operations/entity/identity.rs"));
    let body = only_fn_body(&ops, "fn slot_attrs_exists(");
    assert!(
        body.len() > 2,
        "T-937.3: the scrubbed body of slot_attrs_exists is empty — the probe is reading \
             nothing and would pass over anything"
    );
    assert!(
        body.contains("slot_exists("),
        "T-937.3: slot_attrs_exists must answer existence off the raw slot map \
             (`MissionDocCore::slot_exists`), so a hidden slot counts as existing; body:{body}"
    );
    assert!(
        !body.contains("materialize("),
        "T-937.3: slot_attrs_exists must NOT materialize — the SoA drops hidden slots \
             (T-665/T-701), so a materialize-sourced existence check answers NO for a slot the \
             document holds, and it pays an O(all slots) walk to do it; body:{body}"
    );
}

#[test]
fn slot_exists_answers_true_for_slots_materialize_drops() {
    let doc = two_sided_core(4);
    doc.add_editor_layer("layer-hidden", "Stashed", None);
    doc.add_slot(
        "on-hidden-layer",
        "sq-blu",
        "layer-hidden",
        9,
        "Rifleman",
        None,
        None,
        5.0,
        5.0,
        0.0,
        0.0,
    );
    doc.set_editor_layer_hidden("layer-hidden", true);
    doc.set_slot_editor_hidden("n0", true);

    let dropped = doc.materialize();
    let quiet = doc.side_key_resolution_count();

    for hidden in ["on-hidden-layer", "n0"] {
        assert!(
            !dropped.ids.iter().any(|s| s == hidden),
            "the fixture is only meaningful while `{hidden}` is dropped from the SoA"
        );
        assert!(
            doc.slot_exists(hidden),
            "T-937.3: `{hidden}` is in the document, so existence must answer TRUE — a NO here \
                 is the silent-overwrite path the id minters warn about"
        );
    }
    for live in ["n1", "n2", "n3"] {
        assert!(doc.slot_exists(live), "T-937.3: `{live}` is a visible slot");
    }
    assert!(
        !doc.slot_exists("never-minted"),
        "T-937.3: existence must still be able to say NO"
    );
    assert!(!doc.slot_exists(""), "T-937.3: the empty id names no slot");
    assert_eq!(
        doc.side_key_resolution_count(),
        quiet,
        "T-937.3: six existence checks moved the side-key counter — slot_exists materialized \
             the document instead of reading the raw map"
    );
}

#[test]
fn a_warm_memo_materializes_exactly_what_a_cold_one_does() {
    let warm = two_sided_core(64);
    warm.set_slot_editor_hidden("n7", true);
    let first = warm.materialize();
    let _ = warm.materialize();
    let third = warm.materialize();

    let cold = MissionDocCore::new();
    cold.apply_update(&warm.encode_state())
        .expect("the twin takes the same document's bytes");
    let fresh = cold.materialize();

    assert_eq!(
        soa_rows(&first),
        soa_rows(&third),
        "T-937.3: repeated materialize calls over an unchanged document must agree"
    );
    assert_eq!(
        soa_rows(&third),
        soa_rows(&fresh),
        "T-937.3: a warm memo must materialize exactly what a cold core does"
    );
    assert_eq!(
        fresh.ids.len(),
        63,
        "non-vacuity: 64 slots minus the one hidden row must have been compared"
    );
    assert!(
        fresh.side_keys.iter().any(|k| k == "OPFOR")
            && fresh.side_keys.iter().any(|k| k == "BLUFOR"),
        "non-vacuity: the comparison must span both sides, not one repeated key"
    );
}

#[test]
fn side_key_memo_sees_a_faction_minted_after_the_first_materialize() {
    let doc = one_slot_awaiting_its_faction();
    assert_eq!(
        doc.materialize().side_keys,
        vec!["BLUFOR".to_string()],
        "a missing faction hop resolves to BLUFOR (T-180.3)"
    );

    doc.add_faction("faction-OPFOR", "OPFOR", "Soviet Army");
    assert_eq!(
        doc.materialize().side_keys,
        vec!["OPFOR".to_string()],
        "T-937.3: the side-key memo served a stale side after the faction was minted — the \
             memo is not being invalidated by the document's own change signal"
    );

    doc.add_faction("faction-OPFOR", "INDFOR", "Independent");
    assert_eq!(
        doc.materialize().side_keys,
        vec!["INDFOR".to_string()],
        "T-937.3: the memo served a stale side after the faction's `key` was rewritten"
    );
}

#[test]
fn side_key_memo_sees_an_undo() {
    let mut doc = one_slot_awaiting_its_faction();
    doc.add_faction("faction-OPFOR", "OPFOR", "Soviet Army");
    assert_eq!(doc.materialize().side_keys, vec!["OPFOR".to_string()]);

    assert!(doc.undo(), "the faction mint is one LOCAL undo step");
    assert_eq!(
        doc.materialize().side_keys,
        vec!["BLUFOR".to_string()],
        "T-937.3: Ctrl+Z removed the faction and the memo kept serving its side key"
    );

    assert!(doc.redo(), "and redo puts it back");
    assert_eq!(
        doc.materialize().side_keys,
        vec!["OPFOR".to_string()],
        "T-937.3: the memo served the undone side after a redo"
    );
}

#[test]
fn side_key_memo_sees_a_remote_update() {
    let doc = one_slot_awaiting_its_faction();
    assert_eq!(
        doc.materialize().side_keys,
        vec!["BLUFOR".to_string()],
        "the fallback is cached before the peer's bytes arrive — that is the setup"
    );

    let peer = MissionDocCore::with_client_id(7);
    peer.add_faction("faction-OPFOR", "OPFOR", "Soviet Army");
    doc.apply_update(&peer.encode_state())
        .expect("a peer's faction mint applies");
    assert_eq!(
        doc.materialize().side_keys,
        vec!["OPFOR".to_string()],
        "T-937.3: a remote update moved the side and the memo did not notice"
    );
}

#[test]
fn side_key_memo_survives_a_slot_moving_sides() {
    let side_of = |doc: &MissionDocCore, id: &str| {
        let soa = doc.materialize();
        soa.side_keys[row_of(&soa, id)].clone()
    };

    let doc = two_sided_core(2);
    assert_eq!(side_of(&doc, "n0"), "BLUFOR", "n0 starts under sq-blu");
    assert_eq!(side_of(&doc, "n1"), "OPFOR", "n1 starts under sq-opf");

    doc.move_slot_to_squad("n0", "sq-opf");
    assert_eq!(
        side_of(&doc, "n0"),
        "OPFOR",
        "T-937.3: the moved slot kept its old side — the memo answered under a stale key"
    );
    assert_eq!(
        side_of(&doc, "n1"),
        "OPFOR",
        "T-937.3: the slot that did not move must be unaffected"
    );
}

#[test]
fn t257_loadouts_undo_scoped() {
    let mut doc = MissionDocCore::new();
    let map = doc.doc.get_or_insert_map("loadouts");
    {
        let mut txn = doc.begin();
        map.insert(&mut txn, "l1", "v");
    }
    let ok = doc.undo();
    let l = {
        let txn2 = doc.begin();
        map.len(&txn2)
    };
    assert!(ok, "undo returned false");
    assert_eq!(l, 0, "loadouts not undo-scoped");
    assert!(doc.redo());
    let l2 = {
        let txn3 = doc.begin();
        map.len(&txn3)
    };
    assert_eq!(l2, 1);
}

#[test]
fn t257_items_undo_scoped() {
    let mut doc = MissionDocCore::new();
    let map = doc.doc.get_or_insert_map("items");
    {
        let mut txn = doc.begin();
        map.insert(&mut txn, "l1", "v");
    }
    let ok = doc.undo();
    let l = {
        let txn2 = doc.begin();
        map.len(&txn2)
    };
    assert!(ok, "undo returned false");
    assert_eq!(l, 0, "items not undo-scoped");
    assert!(doc.redo());
    let l2 = {
        let txn3 = doc.begin();
        map.len(&txn3)
    };
    assert_eq!(l2, 1);
}

#[test]
fn t257_objectives_undo_scoped() {
    let mut doc = MissionDocCore::new();
    let map = doc.doc.get_or_insert_map("objectives");
    {
        let mut txn = doc.begin();
        map.insert(&mut txn, "l1", "v");
    }
    let ok = doc.undo();
    let l = {
        let txn2 = doc.begin();
        map.len(&txn2)
    };
    assert!(ok, "undo returned false");
    assert_eq!(l, 0, "objectives not undo-scoped");
    assert!(doc.redo());
    let l2 = {
        let txn3 = doc.begin();
        map.len(&txn3)
    };
    assert_eq!(l2, 1);
}

#[test]
fn t257_markers_undo_scoped() {
    let mut doc = MissionDocCore::new();
    let map = doc.doc.get_or_insert_map("markers");
    {
        let mut txn = doc.begin();
        map.insert(&mut txn, "l1", "v");
    }
    let ok = doc.undo();
    let l = {
        let txn2 = doc.begin();
        map.len(&txn2)
    };
    assert!(ok, "undo returned false");
    assert_eq!(l, 0, "markers not undo-scoped");
    assert!(doc.redo());
    let l2 = {
        let txn3 = doc.begin();
        map.len(&txn3)
    };
    assert_eq!(l2, 1);
}

#[test]
fn move_slot_to_squad_keep_source_keeps_the_emptied_squad_its_vehicles_and_its_position() {
    let doc = keep_source_fixture();
    doc.add_slot(
        "solo", "sq-mid", "lyr", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
    );
    doc.set_leader("sq-mid", "solo");
    doc.add_vehicle("v1", "Prefab/Truck.et", None, None, None, None);
    doc.attach_vehicle("sq-mid", "v1");
    let before = squad_ids_of(&doc, "faction-BLUFOR");

    doc.move_slot_to_squad_keep_source("solo", "sq-a");

    let root = small_maps(&doc);
    let mut lost: Vec<&str> = Vec::new();
    if root["squadsById"].get("sq-mid").is_none() {
        lost.push("the squad row");
    }
    if root["vehiclesById"].get("v1").is_none() {
        lost.push("the attached vehicle v1");
    }
    if root["squadsById"]["sq-mid"]["vehicleIds"]
        .as_array()
        .is_none_or(|a| a.len() != 1)
    {
        lost.push("the squad->vehicle attachment");
    }
    if squad_ids_of(&doc, "faction-BLUFOR") != before {
        lost.push("its place in faction.squadIds");
    }
    assert!(
        lost.is_empty(),
        "T-939.2: the keep-source move lost {}; squads were {}, vehicles were {}",
        lost.join(", "),
        root["squadsById"],
        root["vehiclesById"]
    );
    assert_eq!(
        root["squadsById"]["sq-mid"]["slotIds"]
            .as_array()
            .expect("slotIds")
            .len(),
        0,
        "the source really is empty — the survival above is not a failed move"
    );

    assert_eq!(slots_map(&doc)["solo"]["squadId"], "sq-a");
    assert!(
        root["squadsById"]["sq-a"]["slotIds"]
            .as_array()
            .expect("slotIds")
            .iter()
            .any(|v| v == "solo"),
        "the destination must list the moved slot"
    );
    assert!(
        root["squadsById"]["sq-mid"]["leaderSlotId"].is_null(),
        "T-939.2: the kept squad must not point at a leader it no longer has; got {}",
        root["squadsById"]["sq-mid"]
    );
}

#[test]
fn the_default_move_slot_to_squad_still_garbage_collects_an_emptied_source() {
    let doc = keep_source_fixture();
    doc.add_slot(
        "solo", "sq-mid", "lyr", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
    );
    doc.add_vehicle("v1", "Prefab/Truck.et", None, None, None, None);
    doc.attach_vehicle("sq-mid", "v1");

    doc.move_slot_to_squad("solo", "sq-a");

    let root = small_maps(&doc);
    assert!(
        root["squadsById"].get("sq-mid").is_none(),
        "the default path must still GC the emptied source (T-180.2 B2)"
    );
    assert!(
        root["vehiclesById"].get("v1").is_none(),
        "the default path must still cascade the attached vehicles"
    );
    assert!(
        !squad_ids_of(&doc, "faction-BLUFOR")
            .iter()
            .any(|s| s == "sq-mid"),
        "the default path must still prune the squad out of faction.squadIds"
    );
}

#[test]
fn keep_source_move_carries_the_derived_side_key_across_factions() {
    let side_of = |doc: &MissionDocCore, id: &str| {
        let soa = doc.materialize();
        soa.side_keys[row_of(&soa, id)].clone()
    };
    let doc = keep_source_fixture();
    doc.add_slot(
        "solo", "sq-mid", "lyr", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
    );
    doc.add_slot(
        "stay", "sq-a", "lyr", 0, "Medic", None, None, 2.0, 2.0, 0.0, 0.0,
    );

    assert_eq!(side_of(&doc, "solo"), "BLUFOR");
    assert_eq!(side_of(&doc, "stay"), "BLUFOR");

    doc.move_slot_to_squad_keep_source("solo", "sq-opf");

    assert_eq!(
        side_of(&doc, "solo"),
        "OPFOR",
        "T-939.2: the derived side key must follow the slot into the new faction's squad"
    );
    assert_eq!(
        side_of(&doc, "stay"),
        "BLUFOR",
        "T-939.2: a slot that did not move must keep its side"
    );

    assert_eq!(
        small_maps(&doc)["squadsById"]["sq-mid"]["factionId"],
        "faction-BLUFOR"
    );
}

#[test]
fn keep_source_move_promotes_the_next_leader_and_keeps_indices_dense() {
    let doc = keep_source_fixture();
    for (i, id) in ["lead", "next", "tail"].iter().enumerate() {
        doc.add_slot(
            id,
            "sq-mid",
            "lyr",
            u32::try_from(i).expect("fixture index fits u32"),
            "Rifleman",
            None,
            None,
            f64::from(u32::try_from(i).expect("fixture index fits u32")),
            1.0,
            0.0,
            0.0,
        );
    }
    doc.set_leader("sq-mid", "lead");

    doc.move_slot_to_squad_keep_source("lead", "sq-a");

    let root = small_maps(&doc);
    assert_eq!(root["squadsById"]["sq-mid"]["leaderSlotId"], "next");
    let slots = slots_map(&doc);
    assert_eq!(slots["next"]["index"].as_i64(), Some(0));
    assert_eq!(slots["tail"]["index"].as_i64(), Some(1));
    assert_eq!(
        slots["lead"]["index"].as_i64(),
        Some(0),
        "dest is dense too"
    );
}

#[test]
fn keep_source_revert_restores_five_mixed_memberships_and_authored_data_in_one_group() {
    let mut doc = keep_source_fixture();
    let snapshot = [
        ("b0", "sq-mid"),
        ("c0", "sq-c"),
        ("o0", "sq-opf"),
        ("b1", "sq-mid"),
        ("o1", "sq-opf"),
    ];
    for (id, squad) in snapshot {
        let index = read_id_array(&doc.doc.transact(), &doc.squads, squad, "slotIds").len();
        doc.add_slot(
            id,
            squad,
            "lyr",
            u32::try_from(index).unwrap(),
            "Medic",
            None,
            None,
            1.0,
            2.0,
            3.0,
            45.0,
        );
        if index == 0 {
            doc.set_leader(squad, id);
        }
    }
    for squad in ["sq-a", "sq-mid", "sq-c", "sq-opf"] {
        let vehicle = format!("vehicle-{squad}");
        doc.add_vehicle(&vehicle, "Prefab/Truck.et", None, None, None, None);
        doc.attach_vehicle(squad, &vehicle);
    }
    doc.update_slot_loadout("b1", Some(r#"{"authored":"medic-kit"}"#.into()));
    let original_maps = small_maps(&doc);
    let original_slots = slots_map(&doc);
    let depth = doc.undo_depth();

    doc.begin_group();
    for (id, _) in snapshot {
        doc.move_slot_to_squad_keep_source(id, "sq-a");
    }
    doc.end_group();
    assert_eq!(
        doc.undo_depth(),
        depth + 1,
        "all five move as one undo step"
    );
    let reassigned_maps = small_maps(&doc);
    let reassigned_slots = slots_map(&doc);
    for (id, _) in snapshot {
        assert_eq!(reassigned_slots[id]["squadId"], "sq-a");
    }
    assert_eq!(
        reassigned_maps["vehiclesById"],
        original_maps["vehiclesById"]
    );
    assert!(
        doc.undo(),
        "one undo restores the entire five-slot reassign"
    );
    assert_eq!(slots_map(&doc), original_slots);
    assert_eq!(small_maps(&doc), original_maps);
    assert!(doc.redo());

    doc.begin_group();
    for (id, original_squad) in snapshot {
        doc.move_slot_to_squad_keep_source(id, original_squad);
    }
    doc.end_group();
    assert_eq!(
        doc.undo_depth(),
        depth + 2,
        "membership Revert is one group"
    );
    assert_eq!(
        slots_map(&doc),
        original_slots,
        "every slot returns home with its authored data"
    );
    assert_eq!(
        small_maps(&doc),
        original_maps,
        "original squads and all attached vehicles survive"
    );
    let restored = doc.materialize();
    for (id, squad) in snapshot {
        let expected_side = if squad == "sq-opf" { "OPFOR" } else { "BLUFOR" };
        assert_eq!(restored.side_keys[row_of(&restored, id)], expected_side);
    }
    assert!(
        doc.undo(),
        "one undo reverses the heterogeneous membership Revert"
    );
    assert_eq!(slots_map(&doc), reassigned_slots);
    assert_eq!(small_maps(&doc), reassigned_maps);
    assert!(doc.redo());
    assert_eq!(slots_map(&doc), original_slots);
    assert_eq!(small_maps(&doc), original_maps);
}
