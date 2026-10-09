//! Role: Domain regression cases.
//! Position: a test or policy module of `mission_document`, compiled into the crate.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn merge_records_skipped_malformed_rows() {
    let payload = serde_json::json!({
        "vehicles": [
            { "id": "v-ok", "resourceName": "Prefab/Ok.et", "position": {"x": 1.0, "y": 2.0} },
            { "resourceName": "Prefab/NoId.et" },
            "not-an-object"
        ],
        "editor": {
            "factions": [],
            "squads": [],
            "slots": [
                { "id": "s-ok", "role": "Rifleman", "position": {"x": 3.0, "y": 4.0} },
                { "role": "NoId" }
            ],
            "editorLayers": []
        }
    });
    let doc = MissionDocCore::new();
    let report = doc.merge_mission_payload(&payload, MergeOpts::default());

    assert_eq!(report.vehicles_added, 1, "the well-formed vehicle landed");
    assert_eq!(report.slots_added, 1, "the well-formed slot landed");

    assert_eq!(report.skipped.len(), 3, "skips: {:?}", report.skipped);
    assert!(
        report
            .skipped
            .iter()
            .any(|(k, _, r)| k == "vehicle" && r.contains("missing id")),
        "missing-id vehicle recorded: {:?}",
        report.skipped
    );
    assert!(
        report
            .skipped
            .iter()
            .any(|(k, _, r)| k == "vehicle" && r.contains("not an object")),
        "non-object vehicle recorded: {:?}",
        report.skipped
    );
    assert!(
        report
            .skipped
            .iter()
            .any(|(k, _, r)| k == "slot" && r.contains("missing id")),
        "missing-id slot recorded: {:?}",
        report.skipped
    );
}

#[test]
fn merge_offset_shifts_all_placed_entities() {
    let src = MissionDocCore::new();
    src.set_origin_init(true);
    src.add_editor_layer("lyr", "Layer", None);
    src.add_faction("faction-BLUFOR", "BLUFOR", "B");
    src.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
    src.add_slot(
        "s0", "sq-a", "lyr", 0, "SL", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    src.set_leader("sq-a", "s0");
    src.add_vehicle(
        "v0",
        "Prefab/Truck.et",
        Some(300.0),
        Some(400.0),
        None,
        None,
    );
    src.add_circle_zone("z0", "boundary", 500.0, 600.0, 50.0);
    src.set_origin_init(false);
    let payload =
        mission_payload::compile_payload(&src.small_maps_json(), &src.slots_json(), false);

    let doc = MissionDocCore::new();
    let report = doc.merge_mission_payload(
        &payload,
        MergeOpts {
            offset: Some((1000.0, 2000.0)),
        },
    );
    assert_eq!(report.slots_added, 1);
    assert_eq!(report.vehicles_added, 1);
    assert_eq!(report.zones_added, 1);

    let slots = slots_map(&doc);
    let slot = slots.as_object().unwrap().values().next().unwrap();
    assert_eq!(
        slot["position"]["x"].as_f64(),
        Some(1100.0),
        "slot x offset"
    );
    assert_eq!(
        slot["position"]["y"].as_f64(),
        Some(2200.0),
        "slot y offset"
    );

    let root = small_maps(&doc);
    let veh = root["vehiclesById"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(
        veh["position"]["x"].as_f64(),
        Some(1300.0),
        "vehicle x offset"
    );
    assert_eq!(
        veh["position"]["y"].as_f64(),
        Some(2400.0),
        "vehicle y offset"
    );

    let zone = root["zonesById"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(
        zone["shape"]["circle"]["x"].as_f64(),
        Some(1500.0),
        "zone x offset"
    );
    assert_eq!(
        zone["shape"]["circle"]["z"].as_f64(),
        Some(2600.0),
        "zone z offset"
    );
}

#[test]
fn merged_doc_round_trips_through_compile_and_hydrate() {
    let payload = template_payload_blufor_alpha();
    let doc = MissionDocCore::new();
    let _ = doc.merge_mission_payload(&payload, MergeOpts::default());
    let reloaded = save_and_reload(&doc);

    let root = small_maps(&reloaded);
    assert_eq!(root["squadsById"].as_object().unwrap().len(), 1);
    let squad = root["squadsById"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(squad["slotIds"].as_array().unwrap().len(), 2);
    assert_eq!(root["vehiclesById"].as_object().unwrap().len(), 1);
}

#[test]
fn merge_same_template_twice_lands_alongside_no_overwrite() {
    let doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_editor_layer("lyr", "Layer", None);
    doc.add_faction("faction-BLUFOR", "BLUFOR", "1st Battalion");
    doc.add_squad("sq-a", "faction-BLUFOR", "Alpha", Some("A1".into()));
    doc.add_slot(
        "res0", "sq-a", "lyr", 0, "SL", None, None, 1.0, 2.0, 0.0, 0.0,
    );
    doc.set_leader("sq-a", "res0");
    doc.set_origin_init(false);

    let payload = template_payload_blufor_alpha();

    let slot_rows = |d: &MissionDocCore| slots_map(d).as_object().unwrap().len();

    let rep1 = doc.merge_mission_payload(&payload, MergeOpts::default());
    assert_eq!(rep1.slots_added, 2, "merge 1 adds both template slots");
    assert_eq!(rep1.squads_merged, 1, "Alpha deduped onto resident");
    assert_eq!(
        slot_rows(&doc),
        3,
        "merge 1: resident res0 + two merged slots"
    );

    let rep2 = doc.merge_mission_payload(&payload, MergeOpts::default());
    assert_eq!(rep2.slots_added, 2, "merge 2 adds two MORE slots");
    assert_eq!(rep2.squads_merged, 1);

    assert_eq!(
        slot_rows(&doc),
        5,
        "merge 2's rows land ALONGSIDE merge 1's — no overwrite"
    );

    let root = small_maps(&doc);
    let alpha = &root["squadsById"]["sq-a"];
    let member_ids = id_array(&alpha["slotIds"]);
    assert_eq!(
        member_ids.len(),
        5,
        "Alpha owns res0 + 4 merged slots: {member_ids:?}"
    );
    assert!(
        has_no_duplicates(&member_ids),
        "slotIds has no duplicate id after two merges: {member_ids:?}"
    );

    let slots = slots_map(&doc);
    for sid in &member_ids {
        assert!(
            slots.get(sid).is_some(),
            "slotIds entry {sid} is a live slot row: {member_ids:?}"
        );
    }

    assert_eq!(
        rep1.slots_added + rep2.slots_added,
        4,
        "reports sum to the real net row growth (5 − 1 resident)"
    );
}

#[test]
fn mint_is_collision_proof_against_resident_ids() {
    let mut first = RemintMap::new();
    first.ensure_fresh("s0");
    let merge1_id = first.get("s0").expect("minted");
    assert_eq!(merge1_id, "mrg-1-s0", "first merge mints mrg-1-s0");

    let mut naive_second = RemintMap::new();
    naive_second.ensure_fresh("s0");
    assert_eq!(
        naive_second.get("s0").as_deref(),
        Some("mrg-1-s0"),
        "an unseeded second table reproduces the SAME id — the collision the fix removes"
    );

    let mut guarded_second = RemintMap::with_reserved(HashSet::from([merge1_id.clone()]));
    guarded_second.ensure_fresh("s0");
    let merge2_id = guarded_second.get("s0").expect("minted");
    assert_ne!(
        merge2_id, merge1_id,
        "the guarded second mint avoids the resident id"
    );
    assert_eq!(merge2_id, "mrg-2-s0", "it takes the next free seq");

    let mut deep = RemintMap::with_reserved(HashSet::from([
        "mrg-1-x".to_string(),
        "mrg-2-x".to_string(),
        "mrg-3-x".to_string(),
    ]));
    deep.ensure_fresh("x");
    assert_eq!(
        deep.get("x").as_deref(),
        Some("mrg-4-x"),
        "mint skips every resident collision, not just seq 1"
    );
}

#[test]
fn comments_never_reach_the_mod_document() {
    let (doc, token) = doc_with_one_comment();

    let payload =
        mission_payload::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    let comments = payload["comments"]
        .as_array()
        .expect("comments[] at the editor-payload root");
    assert_eq!(comments.len(), 1, "one authored comment: {payload}");
    assert_eq!(comments[0]["title"], token);
    assert_eq!(comments[0]["position"]["x"], 1_234.5);
    assert_eq!(comments[0]["position"]["z"], 6_789.5);

    let reloaded = MissionDocCore::new();
    reloaded.hydrate(&serde_json::to_string(&payload).expect("payload json"), "L");
    let rows: serde_json::Value =
        serde_json::from_str(&reloaded.comments_json()).expect("comments_json");
    assert_eq!(rows["c1"]["title"], token, "comment reloaded: {rows}");
    assert_eq!(reloaded.comment_count(), 1);

    let meta = br#"{"id":"11112222333344445555666677778888","title":"t","author":"a",
            "terrain":"everon","customTerrainName":"","maxPlayers":8,"timeOfDay":"05:30",
            "weatherPreset":"clear"}"#;
    let payload_bytes = serde_json::to_vec(&payload).expect("payload bytes");
    let mod_text = String::from_utf8(
        mission_compiler::flatten_mod_document_json(meta, &payload_bytes)
            .expect("flatten compiles"),
    )
    .expect("utf-8");
    assert!(
        !mod_text.contains(token) && !mod_text.contains("comment"),
        "a comment reached the compiled mission: {mod_text}"
    );

    let mut leaked = payload.clone();
    leaked["entities"] = serde_json::json!([{
        "id": "c1",
        "alias": token,
        "resourceName": "",
        "position": { "x": 1_234.5, "z": 6_789.5 },
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
        leaked_text.contains(token),
        "the leak probe did not reach the mod document, so step 3 proves nothing: {leaked_text}"
    );

    assert!(!mod_text.contains(token));
}

#[test]
fn the_connection_checker_fires_every_rule_and_stays_silent_on_a_clean_graph() {
    let known: HashSet<String> = ["a", "b", "c"].iter().map(|s| (*s).to_string()).collect();
    let row = |id: &str, kind: &str, from: &str, to: &str| ConnectionRow {
        id: id.into(),
        kind: kind.to_string(),
        from: from.to_string(),
        to: to.to_string(),
    };

    let clean = vec![
        row("ok1", "sync", "a", "b"),
        row("ok2", "group", "a", "b"),
        row("ok3", "group", "b", "c"),
    ];
    assert_eq!(
        validate_connection_rows(&clean, &known),
        Vec::new(),
        "a correct graph must produce NO findings, or the positives below prove nothing"
    );

    let codes = |rows: &[ConnectionRow]| -> Vec<(&'static str, String)> {
        validate_connection_rows(rows, &known)
            .into_iter()
            .map(|f| (f.code, f.connection_id.into_inner()))
            .collect()
    };

    assert!(
        codes(&[row("x", "sync", "a", "a")]).contains(&("CONN-SELF", "x".to_string())),
        "a self-link must fire CONN-SELF"
    );

    assert!(
        codes(&[row("x", "sync", "a", "ghost")]).contains(&("CONN-DANGLING", "x".to_string())),
        "an unplaced endpoint must fire CONN-DANGLING"
    );

    let dupes = codes(&[
        row("first", "sync", "a", "b"),
        row("second", "sync", "a", "b"),
    ]);
    assert!(
        dupes.contains(&("CONN-DUPLICATE", "second".to_string()))
            && !dupes.contains(&("CONN-DUPLICATE", "first".to_string())),
        "duplicate must name the LATER row, not the survivor: {dupes:?}"
    );

    let cyc = codes(&[
        row("e1", "group", "a", "b"),
        row("e2", "group", "b", "c"),
        row("e3", "group", "c", "a"),
    ]);
    assert!(
        cyc.iter().any(|(code, _)| *code == "CONN-CYCLE"),
        "an ownership cycle must fire CONN-CYCLE: {cyc:?}"
    );

    let sync_loop = codes(&[
        row("e1", "sync", "a", "b"),
        row("e2", "sync", "b", "c"),
        row("e3", "sync", "a", "c"),
    ]);
    assert!(
        !sync_loop.iter().any(|(code, _)| *code == "CONN-CYCLE"),
        "a sync loop is legal — CONN-CYCLE is for DIRECTED kinds only: {sync_loop:?}"
    );

    assert!(
        codes(&[row("x", "attachedTo", "a", "b")]).contains(&("CONN-KIND", "x".to_string())),
        "an unknown kind must fire CONN-KIND"
    );

    let messy = vec![
        row("z", "sync", "a", "a"),
        row("y", "group", "a", "ghost"),
        row("x", "sync", "ghost2", "b"),
    ];
    let once = validate_connection_rows(&messy, &known);
    for _ in 0..8 {
        assert_eq!(validate_connection_rows(&messy, &known), once);
    }
}

#[test]
fn hydrated_junk_edges_survive_the_load_and_are_reported_not_silently_dropped() {
    let doc = doc_with_connectable_things();
    assert!(doc.add_connection("good", "sync", "s0", "s1"));
    let mut payload =
        mission_payload::compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);

    payload["connections"] = serde_json::json!([
        {"id": "good", "kind": "sync", "from": "s0", "to": "s1"},
        {"id": "selfie", "kind": "sync", "from": "s0", "to": "s0"},
        {"id": "ghosted", "kind": "group", "from": "s1", "to": "nope"},

        {"id": "zz-dupe", "kind": "sync", "from": "s0", "to": "s1"},
        {"id": "weird", "kind": "attachedTo", "from": "s0", "to": "s1"},
    ]);

    let reloaded = doc_with_connectable_things();
    reloaded.hydrate(&serde_json::to_string(&payload).expect("json"), "L");
    assert_eq!(
        reloaded.connection_count(),
        5,
        "a bad row must SURVIVE the load — rejecting at hydrate destroys an author's data"
    );
    let findings: serde_json::Value =
        serde_json::from_str(&reloaded.connection_findings_json()).expect("findings");
    let pairs: Vec<(String, String)> = findings
        .as_array()
        .expect("array")
        .iter()
        .map(|f| {
            (
                f["code"].as_str().unwrap_or_default().to_string(),
                f["connectionId"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    for want in [
        ("CONN-SELF", "selfie"),
        ("CONN-DANGLING", "ghosted"),
        ("CONN-DUPLICATE", "zz-dupe"),
        ("CONN-KIND", "weird"),
    ] {
        assert!(
            pairs.contains(&(want.0.to_string(), want.1.to_string())),
            "{want:?} missing from the live findings: {pairs:?}"
        );
    }
    assert!(
        !pairs.iter().any(|(_, id)| id == "good"),
        "the sound edge must not be flagged: {pairs:?}"
    );
}

#[test]
fn add_connection_refuses_junk_and_normalises_sync_endpoints() {
    let doc = doc_with_connectable_things();
    for (id, kind, from, to, why) in [
        ("", "sync", "s0", "s1", "empty id"),
        ("k", "sync", "", "s1", "empty from"),
        ("k", "sync", "s0", "", "empty to"),
        ("k", "attachedTo", "s0", "s1", "unknown kind"),
        ("k", "sync", "s0", "s0", "self-link"),
    ] {
        assert!(
            !doc.add_connection(id, kind, from, to),
            "must refuse: {why}"
        );
        assert_eq!(doc.connection_count(), 0, "…and write nothing: {why}");
    }

    assert!(doc.add_connection("k1", "sync", "s1", "s0"));
    let rows: serde_json::Value = serde_json::from_str(&doc.connection_rows_json()).expect("rows");
    assert_eq!(
        (rows[0]["from"].as_str(), rows[0]["to"].as_str()),
        (Some("s0"), Some("s1")),
        "sync endpoints are sorted at write: {rows}"
    );

    assert!(
        !doc.add_connection("k2", "sync", "s0", "s1"),
        "sync(A,B) after sync(B,A) is a DUPLICATE, not a second edge"
    );
    assert_eq!(doc.connection_count(), 1);

    assert!(doc.add_connection("g1", "group", "s1", "s0"));
    assert!(doc.add_connection("g2", "group", "s0", "s1"));
    assert_eq!(
        doc.connection_count(),
        3,
        "directed edges are not normalised"
    );
}
