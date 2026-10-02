//! Role: Module boundary for doc/crdt/id_arrays/tests.
//! Position: `doc/crdt/id_arrays/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use yrs::updates::decoder::Decode;

use yrs::{Doc, MapPrelim, StateVector, Transact, Update};

fn native_doc() -> (Doc, MapRef, MapRef) {
    let doc = Doc::with_client_id(1);
    let squads = doc.get_or_insert_map("squads");
    let layers = doc.get_or_insert_map("editorLayers");
    (doc, squads, layers)
}

fn encode(doc: &Doc) -> Vec<u8> {
    doc.transact()
        .encode_state_as_update_v1(&StateVector::default())
}

fn apply(doc: &Doc, bytes: &[u8]) {
    let mut txn = doc.transact_mut();
    txn.apply_update(Update::decode_v1(bytes).expect("update"))
        .expect("apply");
}

mod cases_1;
