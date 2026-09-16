//! Role: what the slot fingerprint is stable under, and what it is sensitive to.
//! Position: `editing/persist/tests` in the map engine.
//! Signals & state: documents built in the test body.
//! Invariants: a document and a fresh peer that replayed its update fingerprint identically, while
//! their encoded bytes need not — which is the whole reason a reload comparison uses this and not a
//! byte compare. Any change to a slot's data changes the string.

use super::*;

fn document_with(slot_count: u32, seed: u64) -> MissionDocCore {
    let core = MissionDocCore::new();
    core.set_origin_init(true);
    core.seed_random(slot_count, 12_800.0, 12_800.0, seed);
    core.set_origin_init(false);
    core
}

#[test]
fn an_empty_document_fingerprints_to_nothing() {
    assert_eq!(slots_digest(&MissionDocCore::new()), "");
}

#[test]
fn the_same_document_fingerprints_the_same_way_twice() {
    let core = document_with(8, 42);
    assert_eq!(slots_digest(&core), slots_digest(&core));
    assert_eq!(slots_digest(&core).lines().count(), 8);
}

/// The property a reload comparison rests on: replaying an update reproduces the slot data even
/// when it does not reproduce the bytes.
#[test]
fn a_fresh_peer_that_replayed_the_update_fingerprints_identically() {
    let core = document_with(8, 42);
    let peer = MissionDocCore::new();
    peer.set_origin_init(true);
    peer.apply_update(&core.encode_state()).expect("replay");
    peer.set_origin_init(false);
    assert_eq!(slots_digest(&peer), slots_digest(&core));
}

#[test]
fn moving_a_slot_changes_the_fingerprint() {
    let core = document_with(8, 42);
    let before = slots_digest(&core);
    core.set_slot_position("s3", 1.0, 2.0, 3.0, 4.0);
    assert_ne!(slots_digest(&core), before);
}

#[test]
fn changing_a_slot_role_changes_the_fingerprint() {
    let core = document_with(8, 42);
    let before = slots_digest(&core);
    core.update_slot("s5", Some("Medic".to_string()), None, None);
    assert_ne!(slots_digest(&core), before);
}

/// Rows come out in a total order that the arbitrary materialize order cannot reach, and every
/// slot is present exactly once.
#[test]
fn the_rows_are_in_a_total_order_and_name_every_slot_once() {
    let digest = slots_digest(&document_with(12, 7));
    let rows: Vec<&str> = digest.lines().collect();
    let mut sorted = rows.clone();
    sorted.sort_unstable();
    assert_eq!(rows, sorted, "the rows must come out sorted");

    let mut ids: Vec<&str> = rows
        .iter()
        .map(|row| row.split('|').next().expect("each row starts with its id"))
        .collect();
    ids.sort_unstable();
    let unique = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), unique, "one row per slot id");
    assert_eq!(unique, 12);
}
