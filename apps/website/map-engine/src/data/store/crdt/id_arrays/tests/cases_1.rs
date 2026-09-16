//! Role: Domain regression cases.
//! Position: `doc/crdt/id_arrays/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn read_append_retain_move_over_yarray() {
    let (doc, squads, _layers) = native_doc();
    {
        let mut txn = doc.transact_mut();
        let sq = squads.insert(&mut txn, "sq", MapPrelim::from([("id", "sq")]));
        insert_empty_native(&mut txn, &sq, SLOT_IDS);
    }
    {
        let mut txn = doc.transact_mut();
        append_id(&mut txn, &squads, "sq", SLOT_IDS, "a");
        append_id(&mut txn, &squads, "sq", SLOT_IDS, "b");
        append_id(&mut txn, &squads, "sq", SLOT_IDS, "c");
        append_id(&mut txn, &squads, "sq", SLOT_IDS, "b");
    }
    {
        let txn = doc.transact();
        assert_eq!(read_ids(&txn, &squads, "sq", SLOT_IDS), ["a", "b", "c"]);
        assert!(is_native_array(&txn, &squads, "sq", SLOT_IDS));
    }
    {
        let mut txn = doc.transact_mut();
        let Out::YMap(sq) = squads.get(&txn, "sq").unwrap() else {
            panic!("squad");
        };
        retain_in(&mut txn, &sq, SLOT_IDS, &HashSet::from(["b"]));
    }
    {
        let txn = doc.transact();
        assert_eq!(read_ids(&txn, &squads, "sq", SLOT_IDS), ["a", "c"]);
    }
    {
        let mut txn = doc.transact_mut();
        let Out::YMap(sq) = squads.get(&txn, "sq").unwrap() else {
            panic!("squad");
        };
        move_id(&mut txn, &sq, SLOT_IDS, "c", 0);
    }
    let txn = doc.transact();
    assert_eq!(read_ids(&txn, &squads, "sq", SLOT_IDS), ["c", "a"]);
}

#[test]
fn both_forms_read_identically() {
    let (doc, squads, _layers) = native_doc();
    {
        let mut txn = doc.transact_mut();
        let legacy = squads.insert(&mut txn, "leg", MapPrelim::from([("id", "leg")]));
        legacy.insert(
            &mut txn,
            SLOT_IDS,
            Any::Array(vec![Any::String("x".into()), Any::String("y".into())].into()),
        );
        let native = squads.insert(&mut txn, "nat", MapPrelim::from([("id", "nat")]));
        replace_native(
            &mut txn,
            &native,
            SLOT_IDS,
            &["x".to_string(), "y".to_string()],
        );
    }
    let txn = doc.transact();
    assert_eq!(
        read_ids(&txn, &squads, "leg", SLOT_IDS),
        read_ids(&txn, &squads, "nat", SLOT_IDS)
    );
    assert_eq!(read_ids(&txn, &squads, "leg", SLOT_IDS), ["x", "y"]);
    assert!(!is_native_array(&txn, &squads, "leg", SLOT_IDS));
    assert!(is_native_array(&txn, &squads, "nat", SLOT_IDS));
}

#[test]
fn hydrate_migration_promotes_legacy_any_array() {
    let (doc, squads, layers) = native_doc();
    {
        let mut txn = doc.transact_mut();
        let sq = squads.insert(&mut txn, "sq", MapPrelim::from([("id", "sq")]));
        sq.insert(
            &mut txn,
            SLOT_IDS,
            Any::Array(vec![Any::String("s1".into())].into()),
        );
        let ly = layers.insert(&mut txn, "L", MapPrelim::from([("id", "L")]));
        ly.insert(
            &mut txn,
            ENTITY_IDS,
            Any::Array(vec![Any::String("s1".into())].into()),
        );
        migrate_legacy_id_lists(&mut txn, &squads, &layers);
    }
    let txn = doc.transact();
    assert!(
        is_native_array(&txn, &squads, "sq", SLOT_IDS),
        "legacy slotIds must become YArray"
    );
    assert!(
        is_native_array(&txn, &layers, "L", ENTITY_IDS),
        "legacy entityIds must become YArray"
    );
    assert_eq!(read_ids(&txn, &squads, "sq", SLOT_IDS), ["s1"]);
    assert_eq!(read_ids(&txn, &layers, "L", ENTITY_IDS), ["s1"]);
}

#[test]
fn skip_migration_leaves_legacy_any_array() {
    let (doc, squads, layers) = native_doc();
    {
        let mut txn = doc.transact_mut();
        let sq = squads.insert(&mut txn, "sq", MapPrelim::from([("id", "sq")]));
        sq.insert(
            &mut txn,
            SLOT_IDS,
            Any::Array(vec![Any::String("s1".into())].into()),
        );
    }
    let txn = doc.transact();
    assert!(
        !is_native_array(&txn, &squads, "sq", SLOT_IDS),
        "without migrate the field stays Any::Array"
    );
    assert_eq!(read_ids(&txn, &squads, "sq", SLOT_IDS), ["s1"]);
    let _ = layers;
}

#[test]
fn concurrent_yarray_appends_both_survive() {
    let a = Doc::with_client_id(0xA1);
    let b = Doc::with_client_id(0xB2);
    let a_squads = a.get_or_insert_map("squads");
    {
        let mut txn = a.transact_mut();
        let sq = a_squads.insert(&mut txn, "sq", MapPrelim::from([("id", "sq")]));
        insert_empty_native(&mut txn, &sq, SLOT_IDS);
    }
    apply(&b, &encode(&a));
    let b_squads = b.get_or_insert_map("squads");
    {
        let mut txn = a.transact_mut();
        append_id(&mut txn, &a_squads, "sq", SLOT_IDS, "from-a");
    }
    {
        let mut txn = b.transact_mut();
        append_id(&mut txn, &b_squads, "sq", SLOT_IDS, "from-b");
    }
    apply(&a, &encode(&b));
    apply(&b, &encode(&a));
    let txn = a.transact();
    let mut ids = read_ids(&txn, &a_squads, "sq", SLOT_IDS);
    ids.sort();
    assert_eq!(ids, ["from-a", "from-b"]);
}

#[test]
fn concurrent_any_array_clone_rewrite_drops_an_id() {
    fn clone_rewrite_append(txn: &mut TransactionMut, map: &MapRef, key: &str, id: &str) {
        if let Some(Out::YMap(container)) = map.get(txn, key) {
            let mut next: Vec<Any> = match container.get(txn, SLOT_IDS) {
                Some(Out::Any(Any::Array(arr))) => arr.iter().cloned().collect(),
                _ => Vec::new(),
            };
            next.push(Any::String(id.into()));
            container.insert(txn, SLOT_IDS, Any::Array(next.into()));
        }
    }

    let a = Doc::with_client_id(0xA1);
    let b = Doc::with_client_id(0xB2);
    let a_squads = a.get_or_insert_map("squads");
    {
        let mut txn = a.transact_mut();
        let sq = a_squads.insert(&mut txn, "sq", MapPrelim::from([("id", "sq")]));
        sq.insert(&mut txn, SLOT_IDS, Any::Array(Vec::new().into()));
    }
    apply(&b, &encode(&a));
    let b_squads = b.get_or_insert_map("squads");
    {
        let mut txn = a.transact_mut();
        clone_rewrite_append(&mut txn, &a_squads, "sq", "from-a");
    }
    {
        let mut txn = b.transact_mut();
        clone_rewrite_append(&mut txn, &b_squads, "sq", "from-b");
    }
    apply(&a, &encode(&b));
    apply(&b, &encode(&a));
    let txn = a.transact();
    let ids = read_ids(&txn, &a_squads, "sq", SLOT_IDS);
    assert_eq!(
        ids.len(),
        1,
        "Any::Array clone-rewrite must drop one concurrent id (the T-937.1 defect); got {ids:?}"
    );
}
