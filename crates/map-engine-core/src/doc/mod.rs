//! Phase 3 document core — a `yrs` (Yjs-wire-compatible) CRDT that holds the editor's slot graph in
//! Rust linear memory and materializes it into a Structure-of-Arrays (the Phase 3.0 spike toward the
//! wasm-resident document model; plan §9). Gated behind the `doc` feature.
//!
//! **Class S** (structural: `yrs` replaces `yjs`) — the parity contract with the JS `Y.Doc` is
//! *result-set equality* (the same materialized slots + the same undo/redo sequence), NOT byte-identity
//! of the CRDT encoding. The document shape mirrors `state/ydoc.ts`: a root `slots` map of nested
//! per-slot maps whose `position` is a plain JSON object (a `yrs` `Any::Map`), plus a root
//! `editorLayers` map whose `entityIds` arrays give each slot its Outliner folder.

mod soa;
mod store;

pub use soa::{NONE_IDX, STANCE_CROUCH, STANCE_PRONE, STANCE_STAND, SlotSoa};
pub use store::MissionDocCore;
