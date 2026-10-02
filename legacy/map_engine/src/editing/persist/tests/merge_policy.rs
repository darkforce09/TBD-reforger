//! Role: what the merge applies, what it refuses, and the order the write path runs in.
//! Position: `editing/persist/tests` in the map engine.
//! Signals & state: documents and counters built in the test body; the read, the merge and the
//! encode are all closures the test supplies, which is exactly how a host supplies them.
//! Invariants: a union keeps both authors' work; a record from another mission is never applied;
//! the bytes that go out after a merge are the RE-ENCODE and never the pre-merge blob.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use futures::executor::block_on;

use super::*;
use crate::data::store::MissionDocCore;

fn document_with(slot_count: u32, seed: u64) -> MissionDocCore {
    let core = MissionDocCore::new();
    core.set_origin_init(true);
    core.seed_random(slot_count, 12_800.0, 12_800.0, seed);
    core.set_origin_init(false);
    core
}

fn handle_for(core: MissionDocCore) -> DocHandle {
    Rc::new(RefCell::new(Some(core)))
}

fn slot_ids(doc: &DocHandle) -> Vec<String> {
    let guard = doc.borrow();
    let mut ids = guard.as_ref().expect("a document").materialize().ids;
    ids.sort();
    ids
}

#[test]
fn a_peers_record_merges_into_the_live_document() {
    let mine = handle_for(document_with(3, 1));
    let theirs = document_with(3, 1);
    theirs.update_slot("s0", Some("Medic".to_string()), None, None);
    let applied =
        apply_update_into_document(&mine, "mission-9", "mission-9", &theirs.encode_state());
    assert!(applied);
    let guard = mine.borrow();
    let core = guard.as_ref().expect("a document");
    assert!(core.has_content());
    assert_eq!(core.materialize().ids.len(), 3);
}

/// A pending write can outlive a route change, so the record's mission is checked and not assumed.
#[test]
fn a_record_from_another_mission_is_never_applied() {
    let mine = handle_for(document_with(2, 7));
    let before = slot_ids(&mine);
    let other = document_with(5, 9).encode_state();
    assert!(!apply_update_into_document(
        &mine,
        "mission-9",
        "mission-other",
        &other
    ));
    assert_eq!(slot_ids(&mine), before, "the document must be untouched");
}

#[test]
fn an_empty_handle_and_a_refused_blob_both_report_nothing_applied() {
    let empty: DocHandle = Rc::new(RefCell::new(None));
    assert!(!apply_update_into_document(
        &empty,
        "mission-9",
        "mission-9",
        &document_with(1, 3).encode_state()
    ));

    let live = handle_for(document_with(1, 3));
    assert!(!apply_update_into_document(
        &live,
        "mission-9",
        "mission-9",
        &[0xff, 0xfe, 0xfd, 0xfc]
    ));
}

/// The union is what makes two tabs on one key safe: each side's own blocks survive the other's.
#[test]
fn the_merge_is_a_union_and_not_a_replacement() {
    let mine = document_with(2, 11);
    let theirs = MissionDocCore::new();
    theirs.set_origin_init(true);
    theirs.apply_update(&mine.encode_state()).expect("replay");
    theirs.set_origin_init(false);
    theirs.add_slot(
        "their-slot",
        "sq",
        "layer",
        2,
        "Medic",
        None,
        None,
        100.0,
        200.0,
        0.0,
        0.0,
    );
    mine.add_slot(
        "my-slot", "sq", "layer", 3, "Rifleman", None, None, 300.0, 400.0, 0.0, 0.0,
    );

    let handle = handle_for(mine);
    assert!(apply_update_into_document(
        &handle,
        "mission-9",
        "mission-9",
        &theirs.encode_state()
    ));
    let ids = slot_ids(&handle);
    assert!(
        ids.contains(&"my-slot".to_string()) && ids.contains(&"their-slot".to_string()),
        "both authors' slots must survive the merge, got {ids:?}"
    );
}

/* ── the write path's order, with the transport supplied as closures ── */

/// A recorder for what the write path asked of its host, in the order it asked.
struct HostCalls {
    reads: Cell<u32>,
    merges: Cell<u32>,
    encodes: Cell<u32>,
    order: RefCell<Vec<&'static str>>,
}

impl HostCalls {
    fn new() -> Self {
        Self {
            reads: Cell::new(0),
            merges: Cell::new(0),
            encodes: Cell::new(0),
            order: RefCell::new(Vec::new()),
        }
    }

    fn note(&self, what: &'static str) {
        self.order.borrow_mut().push(what);
    }
}

#[test]
fn no_record_on_disk_means_the_bytes_go_out_untouched() {
    let calls = HostCalls::new();
    let out = block_on(merge_before_write(
        vec![1, 2, 3],
        || {
            calls.reads.set(calls.reads.get() + 1);
            async { None }
        },
        &|_| {
            calls.merges.set(calls.merges.get() + 1);
            true
        },
        &|| {
            calls.encodes.set(calls.encodes.get() + 1);
            vec![9, 9, 9]
        },
    ));
    assert_eq!(out, vec![1, 2, 3]);
    assert_eq!(calls.reads.get(), 1, "the record must still be read");
    assert_eq!(calls.merges.get(), 0);
    assert_eq!(
        calls.encodes.get(),
        0,
        "nothing merged, nothing to re-encode"
    );
}

/// A record byte-identical to what is about to be written carries nothing this document lacks, so
/// the O(document) merge is skipped entirely.
#[test]
fn an_identical_record_is_not_merged() {
    let calls = HostCalls::new();
    let out = block_on(merge_before_write(
        vec![1, 2, 3],
        || async { Some(vec![1, 2, 3]) },
        &|_| {
            calls.merges.set(calls.merges.get() + 1);
            true
        },
        &|| {
            calls.encodes.set(calls.encodes.get() + 1);
            vec![9, 9, 9]
        },
    ));
    assert_eq!(out, vec![1, 2, 3]);
    assert_eq!(calls.merges.get(), 0);
    assert_eq!(calls.encodes.get(), 0);
}

#[test]
fn a_refused_merge_leaves_the_outgoing_bytes_alone() {
    let calls = HostCalls::new();
    let out = block_on(merge_before_write(
        vec![1, 2, 3],
        || async { Some(vec![4, 5, 6]) },
        &|_| {
            calls.merges.set(calls.merges.get() + 1);
            false
        },
        &|| {
            calls.encodes.set(calls.encodes.get() + 1);
            vec![9, 9, 9]
        },
    ));
    assert_eq!(out, vec![1, 2, 3]);
    assert_eq!(calls.merges.get(), 1);
    assert_eq!(
        calls.encodes.get(),
        0,
        "a refused merge changed nothing, so the pre-merge bytes are still current"
    );
}

/// After a merge the document IS the union, so writing the pre-merge blob would make the read
/// decoration. The read also has to happen inside this call, not before it.
#[test]
fn a_merge_that_lands_writes_the_re_encode_and_reads_first() {
    let calls = HostCalls::new();
    let out = block_on(merge_before_write(
        vec![1, 2, 3],
        || {
            calls.note("read");
            async { Some(vec![4, 5, 6]) }
        },
        &|stored| {
            assert_eq!(stored, [4, 5, 6]);
            calls.note("merge");
            true
        },
        &|| {
            calls.note("encode");
            vec![9, 9, 9]
        },
    ));
    assert_eq!(out, vec![9, 9, 9], "the union must be re-encoded");
    assert_eq!(*calls.order.borrow(), vec!["read", "merge", "encode"]);
}
