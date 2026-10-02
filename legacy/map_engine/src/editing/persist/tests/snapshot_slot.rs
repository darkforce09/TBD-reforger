//! Role: the snapshot pair's arithmetic, and what a capture will and will not store.
//! Position: `editing/persist/tests` in the map engine.
//! Signals & state: documents built in the test body; the store is a recording closure.
//! Invariants: the two slots are exact inverses of each other, their suffixes are distinct and
//! cannot collide with a mission id, and a capture hands the store bytes only when there is a
//! document worth storing.

use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::data::store::MissionDocCore;
use crate::editing::persist::record_key::{scoped_key, snapshot_key, split_scoped_key};

fn authored_handle(slots: u32) -> DocHandle {
    let core = MissionDocCore::new();
    core.set_origin_init(true);
    core.seed_random(slots, 12_800.0, 12_800.0, 11);
    core.set_origin_init(false);
    Rc::new(RefCell::new(Some(core)))
}

#[test]
fn a_restore_and_its_inverse_are_exact_opposites() {
    assert_eq!(
        SnapshotSlot::PreAdopt.counterpart(),
        SnapshotSlot::PreRestore
    );
    assert_eq!(
        SnapshotSlot::PreRestore.counterpart(),
        SnapshotSlot::PreAdopt
    );
    for slot in [SnapshotSlot::PreAdopt, SnapshotSlot::PreRestore] {
        assert_eq!(
            slot.counterpart().counterpart(),
            slot,
            "the pair is a pair, not a stack: two swaps land back where they started"
        );
    }
}

#[test]
fn the_two_slots_are_told_apart_by_suffix_and_by_name() {
    assert_ne!(
        SnapshotSlot::PreAdopt.suffix(),
        SnapshotSlot::PreRestore.suffix()
    );
    assert_ne!(
        SnapshotSlot::PreAdopt.label(),
        SnapshotSlot::PreRestore.label()
    );
    assert_eq!(SnapshotSlot::PreAdopt.label(), "pre-adopt");
    assert_eq!(SnapshotSlot::PreRestore.label(), "pre-restore");
}

/// The suffix is the whole of what keeps the debounced draft write off a snapshot: the draft is
/// written under the bare mission id, and a canonical id can never carry one of these.
#[test]
fn a_snapshot_key_is_the_mission_id_plus_a_suffix_a_mission_id_cannot_have() {
    let id = "3f2504e0-4f89-11d3-9a0c-0305e82c3301";
    for slot in [SnapshotSlot::PreAdopt, SnapshotSlot::PreRestore] {
        let logical = snapshot_key(id, slot.suffix());
        assert!(logical.starts_with(id));
        assert_ne!(
            logical, id,
            "a snapshot never lands on the live draft's key"
        );
        assert!(!crate::editing::persist::mission_id::is_uuid(&logical));
        // …and it is scoped to its account exactly as the live draft is.
        let physical = scoped_key("1234", &logical);
        assert_eq!(
            split_scoped_key(&physical),
            Some(("1234", logical.as_str()))
        );
    }
    assert_ne!(
        snapshot_key(id, SnapshotSlot::PreAdopt.suffix()),
        snapshot_key(id, SnapshotSlot::PreRestore.suffix())
    );
}

#[test]
fn a_capture_hands_the_store_the_documents_own_bytes_and_reports_its_slots() {
    let doc = authored_handle(6);
    let stored: RefCell<Vec<Vec<u8>>> = RefCell::new(Vec::new());
    let slots = capture_document_snapshot(&doc, &|bytes| stored.borrow_mut().push(bytes));
    assert_eq!(slots, Some(6));
    let stored = stored.into_inner();
    assert_eq!(stored.len(), 1, "the store is called exactly once");
    let replayed = MissionDocCore::new();
    replayed
        .apply_update(&stored[0])
        .expect("the captured bytes replay");
    assert_eq!(replayed.slot_count(), 6);
}

/// The capture is the pre-swap document by construction: the encode happens inside the call, before
/// the caller has replaced anything.
#[test]
fn the_captured_bytes_are_the_document_as_it_was_when_the_capture_ran() {
    let doc = authored_handle(5);
    let stored: RefCell<Vec<Vec<u8>>> = RefCell::new(Vec::new());
    capture_document_snapshot(&doc, &|bytes| stored.borrow_mut().push(bytes)).expect("captured");
    // Whatever the caller does next, the bytes already in hand still hold five slots.
    *doc.borrow_mut() = Some(MissionDocCore::new());
    let replayed = MissionDocCore::new();
    replayed
        .apply_update(&stored.borrow()[0])
        .expect("the captured bytes replay");
    assert_eq!(replayed.slot_count(), 5);
    assert_eq!(
        doc.borrow().as_ref().map(MissionDocCore::slot_count),
        Some(0)
    );
}

#[test]
fn there_is_nothing_to_capture_without_a_document() {
    let doc: DocHandle = Rc::new(RefCell::new(None));
    let calls = RefCell::new(0_u32);
    assert_eq!(
        capture_document_snapshot(&doc, &|_| *calls.borrow_mut() += 1),
        None
    );
    assert_eq!(calls.into_inner(), 0, "the store is never called");
}

/// An empty document still encodes to bytes, so "is it empty" is asked of the encode and not of a
/// length threshold — and a document that holds nothing is still worth a slot, because replacing it
/// is still a replacement the operator may want back.
#[test]
fn an_empty_document_still_encodes_to_a_storable_snapshot() {
    let doc: DocHandle = Rc::new(RefCell::new(Some(MissionDocCore::new())));
    let stored: RefCell<Vec<Vec<u8>>> = RefCell::new(Vec::new());
    assert_eq!(
        capture_document_snapshot(&doc, &|bytes| stored.borrow_mut().push(bytes)),
        Some(0)
    );
    assert_eq!(stored.borrow().len(), 1);
    assert!(!stored.borrow()[0].is_empty());
}
