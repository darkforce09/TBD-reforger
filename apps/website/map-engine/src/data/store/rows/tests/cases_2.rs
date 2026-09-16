//! Role: Domain regression cases.
//! Position: `doc/store/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn two_local_places_are_two_undo_steps() {
    let mut doc = seeded_core();
    assert_eq!(doc.materialize().len(), 8);

    doc.add_slot(
        "p1", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 100.0, 0.0, 0.0,
    );
    doc.add_slot(
        "p2", "sq", "lyr", 1, "Rifleman", None, None, 200.0, 200.0, 0.0, 0.0,
    );
    assert_eq!(doc.materialize().len(), 10);

    assert!(doc.undo());
    assert_eq!(doc.materialize().len(), 9, "undo 1 removes ONLY p2");
    assert!(doc.can_undo());

    assert!(doc.undo());
    assert_eq!(doc.materialize().len(), 8, "undo 2 removes p1");
}

#[test]
fn add_slot_materializes_soa() {
    let doc = MissionDocCore::new();
    doc.add_slot(
        "s1", "sq1", "lyr", 0, "Rifleman", None, None, 100.5, 200.25, 0.0, 0.0,
    );
    doc.add_slot(
        "s2",
        "sq1",
        "lyr",
        1,
        "Squad Leader",
        None,
        None,
        300.0,
        400.0,
        5.0,
        90.0,
    );

    let soa = doc.materialize();
    assert_eq!(soa.len(), 2);
    assert_eq!(ids_sorted(&soa), vec!["s1".to_string(), "s2".to_string()]);

    let r1 = row_of(&soa, "s1");
    assert_eq!(soa.xs[r1], 100.5_f32);
    assert_eq!(soa.ys[r1], 200.25_f32);
    assert_eq!(soa.stance[r1], STANCE_STAND);
    assert_eq!(soa.squads[soa.squad_idx[r1] as usize], "sq1");
    assert_eq!(soa.roles[soa.role_idx[r1] as usize], "Rifleman");
    assert_eq!(soa.tag_idx[r1], NONE_IDX);

    let r2 = row_of(&soa, "s2");
    assert_eq!(soa.rotations[r2], 90.0_f32);
    assert_eq!(soa.roles[soa.role_idx[r2] as usize], "Squad Leader");
}

#[test]
fn apply_update_from_peer_with_bigint_position() {
    let peer = Doc::with_client_id(999);
    let pslots = peer.get_or_insert_map("slots");
    {
        let mut txn = peer.transact_mut();
        let slot = pslots.insert(&mut txn, "p1", MapPrelim::from([("id", "p1")]));
        slot.insert(&mut txn, "role", "Medic");
        slot.insert(&mut txn, "squadId", "sq9");
        slot.insert(&mut txn, "stance", "prone");
        let mut pos: HashMap<String, Any> = HashMap::new();
        pos.insert("x".to_string(), Any::Number(12.5));
        pos.insert("y".to_string(), Any::Number(34.75));
        pos.insert("z".to_string(), Any::BigInt(0));
        pos.insert("rotation".to_string(), Any::BigInt(180));
        slot.insert(&mut txn, "position", Any::Map(Arc::new(pos)));
    }
    let update = peer
        .transact()
        .encode_state_as_update_v1(&StateVector::default());

    let doc = MissionDocCore::new();
    doc.apply_update(&update).expect("apply ok");
    let soa = doc.materialize();
    assert_eq!(soa.len(), 1);

    let r = row_of(&soa, "p1");
    assert_eq!(soa.xs[r], 12.5_f32);
    assert_eq!(soa.ys[r], 34.75_f32);
    assert_eq!(soa.zs[r], 0.0_f32);
    assert_eq!(soa.rotations[r], 180.0_f32);
    assert_eq!(soa.stance[r], STANCE_PRONE);
    assert_eq!(soa.roles[soa.role_idx[r] as usize], "Medic");
}

#[test]
fn encode_decode_roundtrip_is_stable() {
    let a = MissionDocCore::new();
    a.add_slot(
        "s1", "sq1", "lyr", 0, "Rifleman", None, None, 1.0, 2.0, 3.0, 4.0,
    );
    a.add_slot(
        "s2", "sq1", "lyr", 1, "Medic", None, None, 5.0, 6.0, 7.0, 8.0,
    );
    let bytes = a.encode_state();

    let b = MissionDocCore::new();
    b.apply_update(&bytes).expect("apply ok");
    let sa = a.materialize();
    let sb = b.materialize();
    assert_eq!(ids_sorted(&sa), ids_sorted(&sb));
    for id in &sa.ids {
        let ra = row_of(&sa, id);
        let rb = row_of(&sb, id);
        assert_eq!(sa.xs[ra], sb.xs[rb]);
        assert_eq!(sa.rotations[ra], sb.rotations[rb]);
    }

    assert_eq!(a.encode_state(), bytes);
}

#[test]
fn two_peers_with_distinct_ids_merge_concurrent_edits() {
    let a = MissionDocCore::with_client_id(0x00A1_A1A1);
    let b = MissionDocCore::with_client_id(0x00B2_B2B2);
    assert_ne!(a.client_id(), b.client_id(), "two peers, two identities");

    a.add_slot(
        "from-a", "sq1", "lyr", 0, "Rifleman", None, None, 10.0, 20.0, 0.0, 90.0,
    );
    b.add_slot(
        "from-b", "sq1", "lyr", 0, "Medic", None, None, 30.0, 40.0, 0.0, 180.0,
    );

    let ua = a.encode_state();
    let ub = b.encode_state();

    a.apply_update(&ub).expect("a integrates b");
    b.apply_update(&ua).expect("b integrates a");

    let sa = a.materialize();
    let sb = b.materialize();
    assert_eq!(
        ids_sorted(&sa),
        vec!["from-a".to_string(), "from-b".to_string()],
        "peer A must hold both concurrent slots"
    );
    assert_eq!(
        ids_sorted(&sb),
        vec!["from-a".to_string(), "from-b".to_string()],
        "peer B must hold both concurrent slots"
    );

    for soa in [&sa, &sb] {
        let ra = row_of(soa, "from-a");
        assert_eq!(soa.xs[ra], 10.0_f32);
        assert_eq!(soa.rotations[ra], 90.0_f32);
        assert_eq!(soa.roles[soa.role_idx[ra] as usize], "Rifleman");
        let rb = row_of(soa, "from-b");
        assert_eq!(soa.xs[rb], 30.0_f32);
        assert_eq!(soa.rotations[rb], 180.0_f32);
        assert_eq!(soa.roles[soa.role_idx[rb] as usize], "Medic");
    }

    a.apply_update(&b.encode_state()).expect("a re-integrates");
    b.apply_update(&a.encode_state()).expect("b re-integrates");
    assert_eq!(ids_sorted(&a.materialize()), ids_sorted(&b.materialize()));
}

#[test]
fn colliding_client_ids_are_rejected_not_merged() {
    let a = MissionDocCore::with_client_id(7);
    let b = MissionDocCore::with_client_id(7);
    a.add_slot(
        "from-a", "sq1", "lyr", 0, "Rifleman", None, None, 1.0, 2.0, 0.0, 0.0,
    );

    for (i, id) in ["from-b1", "from-b2", "from-b3"].iter().enumerate() {
        b.add_slot(
            id,
            "sq1",
            "lyr",
            u32::try_from(i).unwrap(),
            "Medic",
            None,
            None,
            3.0,
            4.0,
            0.0,
            0.0,
        );
    }

    let err = a
        .apply_update(&b.encode_state())
        .expect_err("a collision must not merge");
    assert!(err.contains("client id collision"), "{err}");

    assert_eq!(ids_sorted(&a.materialize()), vec!["from-a".to_string()]);
}

#[test]
fn a_colliding_writer_behind_us_is_undetectable_and_silently_loses_its_edits() {
    let a = MissionDocCore::with_client_id(9);
    let b = MissionDocCore::with_client_id(9);
    for (i, id) in ["a1", "a2", "a3"].iter().enumerate() {
        a.add_slot(
            id,
            "sq1",
            "lyr",
            u32::try_from(i).unwrap(),
            "Rifleman",
            None,
            None,
            1.0,
            2.0,
            0.0,
            0.0,
        );
    }
    b.add_slot(
        "b1", "sq1", "lyr", 0, "Medic", None, None, 3.0, 4.0, 0.0, 0.0,
    );

    a.apply_update(&b.encode_state())
        .expect("undetectable: reads as an echo");

    assert_eq!(
        ids_sorted(&a.materialize()),
        vec!["a1".to_string(), "a2".to_string(), "a3".to_string()],
        "b1 is lost — the silent corruption a shared client id causes"
    );
}

#[test]
fn rehydration_replays_a_blob_into_a_fresh_peer_identity() {
    let session1 = MissionDocCore::with_client_id(0x00C3_C3C3);
    session1.add_slot(
        "persisted",
        "sq1",
        "lyr",
        0,
        "Rifleman",
        None,
        None,
        5.0,
        6.0,
        0.0,
        45.0,
    );
    let blob = session1.encode_state();

    let session2 = MissionDocCore::with_client_id(0x00D4_D4D4);
    assert_ne!(session2.client_id(), session1.client_id());
    session2.apply_update(&blob).expect("restore ok");
    assert_eq!(
        ids_sorted(&session2.materialize()),
        vec!["persisted".to_string()]
    );

    let other_tab = MissionDocCore::with_client_id(0x00E5_E5E5);
    other_tab.apply_update(&blob).expect("restore ok");
    session2.add_slot(
        "tab-two", "sq1", "lyr", 1, "Medic", None, None, 7.0, 8.0, 0.0, 0.0,
    );
    other_tab.add_slot(
        "tab-three",
        "sq1",
        "lyr",
        2,
        "Engineer",
        None,
        None,
        9.0,
        10.0,
        0.0,
        0.0,
    );
    other_tab
        .apply_update(&session2.encode_state())
        .expect("tabs merge");
    assert_eq!(
        ids_sorted(&other_tab.materialize()),
        vec![
            "persisted".to_string(),
            "tab-three".to_string(),
            "tab-two".to_string()
        ]
    );
}

#[test]
fn fresh_peer_replays_a_same_id_blob_without_tripping_the_guard() {
    let a = MissionDocCore::with_client_id(42);
    a.add_slot(
        "s1", "sq1", "lyr", 0, "Rifleman", None, None, 1.0, 2.0, 3.0, 4.0,
    );
    let probe = MissionDocCore::with_client_id(42);
    assert_eq!(probe.client_id(), a.client_id());
    probe
        .apply_update(&a.encode_state())
        .expect("probe replays");
    assert_eq!(ids_sorted(&probe.materialize()), vec!["s1".to_string()]);
}

#[test]
fn new_mints_a_distinct_client_id_per_document() {
    let ids: Vec<u64> = (0..16).map(|_| MissionDocCore::new().client_id()).collect();
    let unique: HashSet<u64> = ids.iter().copied().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "16 fresh docs must have 16 distinct client ids, got {ids:?}"
    );
    for id in ids {
        assert_ne!(id, 1, "the T-222 hardcode must not come back");
        assert_eq!(
            id >> CLIENT_ID_BITS,
            0,
            "{id} is not a 53-bit Yjs client id"
        );
    }
}

#[test]
fn two_default_constructed_peers_merge_concurrent_edits() {
    let a = MissionDocCore::new();
    let b = MissionDocCore::new();
    a.add_slot(
        "from-a", "sq1", "lyr", 0, "Rifleman", None, None, 10.0, 20.0, 0.0, 90.0,
    );
    b.add_slot(
        "from-b", "sq1", "lyr", 0, "Medic", None, None, 30.0, 40.0, 0.0, 180.0,
    );
    let (ua, ub) = (a.encode_state(), b.encode_state());
    a.apply_update(&ub).expect("a integrates b");
    b.apply_update(&ua).expect("b integrates a");
    let want = vec!["from-a".to_string(), "from-b".to_string()];
    assert_eq!(ids_sorted(&a.materialize()), want);
    assert_eq!(ids_sorted(&b.materialize()), want);
}

#[test]
fn undo_redo_sequence() {
    let mut doc = MissionDocCore::new();
    assert!(!doc.can_undo());
    doc.add_slot(
        "s1", "sq1", "lyr", 0, "Rifleman", None, None, 0.0, 0.0, 0.0, 0.0,
    );
    doc.add_slot(
        "s2", "sq1", "lyr", 1, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
    );
    doc.add_slot(
        "s3", "sq1", "lyr", 2, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
    );
    assert_eq!(doc.materialize().len(), 3);

    assert!(doc.undo());
    assert_eq!(
        ids_sorted(&doc.materialize()),
        vec!["s1".to_string(), "s2".to_string()]
    );
    assert!(doc.undo());
    assert_eq!(ids_sorted(&doc.materialize()), vec!["s1".to_string()]);
    assert!(doc.redo());
    assert_eq!(
        ids_sorted(&doc.materialize()),
        vec!["s1".to_string(), "s2".to_string()]
    );
}

#[test]
fn init_mode_transactions_are_not_undoable() {
    let mut doc = MissionDocCore::new();
    doc.add_editor_layer("l1", "Alpha", None);
    assert!(doc.can_undo(), "a LOCAL op is undoable");
    assert!(doc.undo());
    assert!(!doc.can_undo());

    doc.set_origin_init(true);
    doc.seed_meta("m1", "Op");
    doc.add_editor_layer("l2", "Bravo", None);
    doc.set_origin_init(false);
    assert!(!doc.can_undo(), "INIT ops must not push undo steps");

    doc.add_editor_layer("l3", "Charlie", None);
    assert!(doc.can_undo());
    assert!(doc.undo());
    assert!(!doc.can_undo());

    assert!(doc.small_maps_json().contains("\"l2\""));
}

#[test]
fn small_maps_json_shape_on_empty_doc() {
    let doc = MissionDocCore::new();
    let json = doc.small_maps_json();
    assert!(json.contains("\"meta\":null"), "{json}");
    for key in [
        "factionsById",
        "squadsById",
        "loadoutsById",
        "itemsById",
        "objectivesById",
        "vehiclesById",
        "entitiesById",
        "markersById",
        "editorLayersById",
    ] {
        assert!(
            json.contains(&format!("\"{key}\":")),
            "missing {key} in {json}"
        );
    }
}

#[test]
fn small_maps_json_includes_applied_entities() {
    let peer = Doc::with_client_id(7);
    let factions = peer.get_or_insert_map("factions");
    let meta = peer.get_or_insert_map("meta");
    {
        let mut txn = peer.transact_mut();
        let f = factions.insert(&mut txn, "f1", MapPrelim::from([("id", "f1")]));
        f.insert(&mut txn, "name", "BLUFOR");
        meta.insert(&mut txn, "title", "Op Test");
    }
    let update = peer
        .transact()
        .encode_state_as_update_v1(&StateVector::default());

    let doc = MissionDocCore::new();
    doc.apply_update(&update).expect("apply ok");
    let json = doc.small_maps_json();
    assert!(json.contains("\"f1\""), "{json}");
    assert!(json.contains("BLUFOR"), "{json}");
    assert!(json.contains("Op Test"), "{json}");
    assert!(
        !json.contains("\"meta\":null"),
        "meta should be populated: {json}"
    );
}

#[test]
fn slots_json_roundtrips_a_slot() {
    let doc = MissionDocCore::new();
    doc.add_slot(
        "s1", "sq1", "lyr", 0, "Rifleman", None, None, 100.5, 200.25, 0.0, 90.0,
    );
    let json = doc.slots_json();
    assert!(json.contains("\"s1\""), "{json}");
    assert!(json.contains("Rifleman"), "{json}");
    assert!(json.contains("100.5"), "{json}");
}

#[test]
fn update_slot_loadout_roundtrips_and_clears() {
    let mut doc = MissionDocCore::new();
    doc.add_slot(
        "s1", "sq1", "lyr", 0, "Rifleman", None, None, 1.0, 2.0, 0.0, 0.0,
    );
    let json = r#"{"primary":"{AAA}Rifle_M16A2.et","uniform":null,"vest":null,"helmet":null,"optic":"{BBB}Optic_Acog.et","magazine":null,"summary":"M16A2 · ACOG"}"#;
    assert!(
        doc.update_slot_loadout("s1", Some(json.to_string())),
        "known slot must ack"
    );

    let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    let lo = &v["s1"]["loadout"];
    assert_eq!(lo["primary"], "{AAA}Rifle_M16A2.et");
    assert_eq!(lo["optic"], "{BBB}Optic_Acog.et");
    assert_eq!(lo["summary"], "M16A2 · ACOG");
    assert!(lo["uniform"].is_null(), "explicit null survives: {lo}");

    assert!(doc.undo());
    let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    assert!(v["s1"].get("loadout").is_none(), "undo cleared loadout");
    assert_eq!(v["s1"]["role"], "Rifleman", "slot itself survives");

    assert!(doc.update_slot_loadout("s1", Some(json.to_string())));
    assert!(doc.update_slot_loadout("s1", None));
    let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    assert!(v["s1"].get("loadout").is_none(), "None clears the key");
    assert!(
        !doc.update_slot_loadout("missing", Some(json.to_string())),
        "unknown id must not ack"
    );
}

#[test]
fn paste_slots_copies_loadout() {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Default", None);
    doc.paste_slots(
        vec!["p1".into(), "p2".into()],
        vec!["sq1".into(), "sq1".into()],
        vec!["lyr".into(), "lyr".into()],
        vec![10.0, 20.0],
        vec![10.0, 20.0],
        vec![0.0, 0.0],
        vec![0.0, 0.0],
        vec!["Rifleman".into(), "Medic".into()],
        vec![String::new(), "MED".into()],
        vec![String::new(), String::new()],
        vec!["stand".into(), "prone".into()],
        vec![
            r#"{"primary":"{AAA}Rifle_M16A2.et","optic":null}"#.into(),
            String::new(),
        ],
        vec![String::new(), String::new()],
        Some(100.0),
        Some(100.0),
        12800.0,
        12800.0,
    );
    let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    assert_eq!(v["p1"]["loadout"]["primary"], "{AAA}Rifle_M16A2.et");
    assert!(v["p1"]["loadout"]["optic"].is_null());
    assert!(
        v["p2"].get("loadout").is_none(),
        "empty string = no loadout copied"
    );
}

#[test]
fn t743_paste_without_anchor_lands_on_source_coordinates() {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Default", None);
    doc.paste_slots(
        vec!["p1".into(), "p2".into()],
        vec!["sq1".into(), "sq1".into()],
        vec!["lyr".into(), "lyr".into()],
        vec![1234.5, 4321.25],
        vec![6789.75, 987.5],
        vec![0.0, 0.0],
        vec![0.0, 0.0],
        vec!["Rifleman".into(), "Medic".into()],
        vec![String::new(), String::new()],
        vec![String::new(), String::new()],
        vec!["stand".into(), "stand".into()],
        vec![String::new(), String::new()],
        vec![String::new(), String::new()],
        None,
        None,
        12800.0,
        12800.0,
    );
    let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    assert_eq!(
        v["p1"]["position"]["x"].as_f64(),
        Some(1234.5),
        "paste-at-original must not move x"
    );
    assert_eq!(
        v["p1"]["position"]["y"].as_f64(),
        Some(6789.75),
        "paste-at-original must not move y"
    );

    assert_eq!(v["p2"]["position"]["x"].as_f64(), Some(4321.25));
    assert_eq!(v["p2"]["position"]["y"].as_f64(), Some(987.5));
}

#[test]
fn t743_anchored_paste_still_moves_the_centroid_onto_the_anchor() {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Default", None);
    doc.paste_slots(
        vec!["a1".into(), "a2".into()],
        vec!["sq1".into(), "sq1".into()],
        vec!["lyr".into(), "lyr".into()],
        vec![100.0, 200.0],
        vec![300.0, 500.0],
        vec![0.0, 0.0],
        vec![0.0, 0.0],
        vec!["Rifleman".into(), "Medic".into()],
        vec![String::new(), String::new()],
        vec![String::new(), String::new()],
        vec!["stand".into(), "stand".into()],
        vec![String::new(), String::new()],
        vec![String::new(), String::new()],
        Some(1000.0),
        Some(2000.0),
        12800.0,
        12800.0,
    );
    let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");

    assert_eq!(v["a1"]["position"]["x"].as_f64(), Some(950.0));
    assert_eq!(v["a1"]["position"]["y"].as_f64(), Some(1900.0));
    assert_eq!(v["a2"]["position"]["x"].as_f64(), Some(1050.0));
    assert_eq!(v["a2"]["position"]["y"].as_f64(), Some(2100.0));
}

#[test]
fn remove_editor_layer_reseeds_when_subtree_is_all_layers() {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("root", "Root", None);
    doc.add_editor_layer("child", "Child", Some("root".to_string()));
    doc.add_slot(
        "s1", "sq1", "child", 0, "Rifleman", None, None, 1.0, 2.0, 0.0, 0.0,
    );

    doc.remove_editor_layer("root", "reseed-1");

    assert_eq!(doc.slot_count(), 0, "the filed slot cascaded away");
    let json = doc.small_maps_json();
    assert!(json.contains("reseed-1"), "reseeded default id: {json}");
    assert!(json.contains("Default Layer"), "{json}");
    assert!(!json.contains("\"root\""), "root deleted: {json}");
    assert!(!json.contains("\"child\""), "child deleted: {json}");
}

#[test]
fn set_leader_exclusive() {
    let doc = orbat_fixture();
    doc.add_slot("a", "sq-a", "lyr", 0, "SL", None, None, 1.0, 1.0, 0.0, 0.0);
    doc.add_slot(
        "b", "sq-a", "lyr", 1, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
    );
    doc.set_leader("sq-a", "a");
    doc.set_leader("sq-a", "b");
    let root = small_maps(&doc);
    assert_eq!(root["squadsById"]["sq-a"]["leaderSlotId"], "b");
    assert_ne!(root["squadsById"]["sq-a"]["leaderSlotId"], "a");
}

#[test]
fn empty_squad_garbage_collected() {
    let doc = orbat_fixture();
    doc.add_slot(
        "solo", "sq-a", "lyr", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
    );
    doc.set_leader("sq-a", "solo");
    doc.add_vehicle("v1", "Prefab/Vehicle.et", None, None, None, None);
    doc.attach_vehicle("sq-a", "v1");
    doc.move_slot_to_squad("solo", "sq-b");
    let root = small_maps(&doc);
    assert!(
        root["squadsById"].get("sq-a").is_none(),
        "empty source must be GC'd: {}",
        root["squadsById"]
    );
    assert!(
        root["vehiclesById"].get("v1").is_none(),
        "vehicles attached only to GC'd squad must be deleted"
    );
    assert_eq!(slots_map(&doc)["solo"]["squadId"], "sq-b");
}

#[test]
fn move_slot_bidirectional() {
    let doc = orbat_fixture();
    doc.add_slot(
        "m", "sq-a", "lyr", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
    );
    doc.add_slot(
        "keep", "sq-a", "lyr", 1, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
    );
    doc.set_leader("sq-a", "keep");
    doc.move_slot_to_squad("m", "sq-b");
    let root = small_maps(&doc);
    let src_ids = root["squadsById"]["sq-a"]["slotIds"]
        .as_array()
        .expect("slotIds");
    let dst_ids = root["squadsById"]["sq-b"]["slotIds"]
        .as_array()
        .expect("slotIds");
    assert!(!src_ids.iter().any(|v| v == "m"));
    assert!(dst_ids.iter().any(|v| v == "m"));
    assert_eq!(slots_map(&doc)["m"]["squadId"], "sq-b");
}

#[test]
fn leader_invariant_holds() {
    let doc = orbat_fixture();
    doc.add_slot("a1", "sq-a", "lyr", 0, "SL", None, None, 1.0, 1.0, 0.0, 0.0);
    doc.add_slot(
        "a2", "sq-a", "lyr", 1, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
    );
    doc.set_leader("sq-a", "a1");
    doc.add_slot(
        "b1", "sq-b", "lyr", 0, "Rifleman", None, None, 3.0, 3.0, 0.0, 0.0,
    );
    doc.move_slot_to_squad("a1", "sq-b");
    doc.set_leader("sq-b", "a1");
    let root = small_maps(&doc);
    let squads = root["squadsById"].as_object().expect("squadsById");
    for (sid, sq) in squads {
        let slot_ids = sq["slotIds"].as_array().expect("slotIds");
        assert!(
            !slot_ids.is_empty(),
            "empty squad {sid} should have been GC'd"
        );
        let leader = sq["leaderSlotId"].as_str().expect("leaderSlotId");
        assert!(
            slot_ids.iter().any(|v| v.as_str() == Some(leader)),
            "squad {sid}: leader {leader} not in {slot_ids:?}"
        );
    }
}

#[test]
fn move_leader_promotes_next() {
    let doc = orbat_fixture();
    doc.add_slot(
        "lead", "sq-a", "lyr", 0, "SL", None, None, 1.0, 1.0, 0.0, 0.0,
    );
    doc.add_slot(
        "next", "sq-a", "lyr", 1, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
    );
    doc.add_slot(
        "tail", "sq-a", "lyr", 2, "Medic", None, None, 3.0, 3.0, 0.0, 0.0,
    );
    doc.set_leader("sq-a", "lead");
    doc.move_slot_to_squad("lead", "sq-b");
    let root = small_maps(&doc);
    assert_eq!(root["squadsById"]["sq-a"]["leaderSlotId"], "next");
    let ids = root["squadsById"]["sq-a"]["slotIds"]
        .as_array()
        .expect("slotIds");
    assert_eq!(ids[0], "next");
}

#[test]
fn attach_vehicle_roundtrip() {
    let doc = orbat_fixture();
    doc.add_slot(
        "s", "sq-a", "lyr", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
    );
    doc.set_leader("sq-a", "s");
    doc.add_vehicle(
        "veh-1",
        "Prefabs/Vehicles/Wheeled/M113/M113.et",
        Some(10.0),
        Some(20.0),
        Some(0.0),
        Some(90.0),
    );
    doc.attach_vehicle("sq-a", "veh-1");
    let root = small_maps(&doc);
    let vids = root["squadsById"]["sq-a"]["vehicleIds"]
        .as_array()
        .expect("vehicleIds");
    assert!(vids.iter().any(|v| v == "veh-1"));
    assert_eq!(root["vehiclesById"]["veh-1"]["squadId"], "sq-a");
    assert_eq!(
        root["vehiclesById"]["veh-1"]["resourceName"],
        "Prefabs/Vehicles/Wheeled/M113/M113.et"
    );
    doc.detach_vehicle("sq-a", "veh-1");
    let root = small_maps(&doc);
    let vids = root["squadsById"]["sq-a"]["vehicleIds"]
        .as_array()
        .expect("vehicleIds");
    assert!(!vids.iter().any(|v| v == "veh-1"));
    assert!(
        root["vehiclesById"].get("veh-1").is_some(),
        "detach must keep the vehicle row"
    );
    assert!(root["vehiclesById"]["veh-1"].get("squadId").is_none());
}

#[test]
fn add_entity_materializes_entities_by_id_and_is_undoable() {
    let mut doc = MissionDocCore::new();
    doc.add_entity(
        "e1",
        "prop:ammo_crate",
        "{FA}Prefabs/Props/Military/AmmoBox.et",
        100.0,
        200.0,
        0.0,
        90.0,
    );
    doc.set_entity_faction("e1", "blufor");
    let root = small_maps(&doc);
    let row = &root["entitiesById"]["e1"];
    assert_eq!(row["alias"], "prop:ammo_crate");
    assert_eq!(row["resourceName"], "{FA}Prefabs/Props/Military/AmmoBox.et");
    assert_eq!(row["faction"], "blufor");
    assert_eq!(row["position"]["x"], 100.0);
    assert_eq!(row["position"]["y"], 200.0);
    assert_eq!(row["position"]["rotation"], 90.0);

    assert!(doc.undo());
    assert!(doc.undo());
    let root = small_maps(&doc);
    assert!(
        root["entitiesById"].get("e1").is_none(),
        "two undos must remove the placed entity"
    );
}

#[cfg(feature = "scenario")]
#[test]
fn unknown_top_level_keys_survive_compile_hydrate_compile() {
    let incoming = serde_json::json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon" },
        "environment": {},
        "serverMigrationToken": "keep-me-v2",
        "featureFlags": { "alpha": true, "n": 42.5 },
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
    assert_eq!(
        small["payloadExtras"]["serverMigrationToken"],
        serde_json::json!("keep-me-v2"),
        "hydrate must park unknown keys in payloadExtras"
    );
    assert_eq!(
        small["payloadExtras"]["featureFlags"]["n"],
        serde_json::json!(42.5)
    );

    let compiled = crate::data::scenario::compile::compile_payload(
        &doc.small_maps_json(),
        &doc.slots_json(),
        false,
    );
    assert_eq!(
        compiled["serverMigrationToken"],
        serde_json::json!("keep-me-v2"),
        "compile must re-emit parked unknown keys onto the wire payload"
    );
    assert_eq!(
        compiled["featureFlags"],
        serde_json::json!({ "alpha": true, "n": 42.5 })
    );
    assert!(
        compiled.get("payloadExtras").is_none(),
        "payloadExtras is a small_maps side-channel, never a wire key"
    );

    let reloaded = save_and_reload(&doc);
    let recompiled = crate::data::scenario::compile::compile_payload(
        &reloaded.small_maps_json(),
        &reloaded.slots_json(),
        false,
    );
    assert_eq!(
        recompiled["serverMigrationToken"],
        serde_json::json!("keep-me-v2"),
        "unknown keys must survive a full Save→reload→Save cycle"
    );
    assert_eq!(
        recompiled["featureFlags"],
        serde_json::json!({ "alpha": true, "n": 42.5 })
    );
    assert_eq!(recompiled["map"]["terrain"], serde_json::json!("everon"));
    assert_eq!(recompiled["schemaVersion"], serde_json::json!(1));
}
