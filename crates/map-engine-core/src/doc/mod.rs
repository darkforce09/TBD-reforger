//! Phase 3 document core — a `yrs` (Yjs-wire-compatible) CRDT that holds the editor's slot graph in
//! Rust linear memory and materializes it into a Structure-of-Arrays (the Phase 3.0 spike toward the
//! wasm-resident document model; plan §9). Gated behind the `doc` feature.
//!
//! **Class S** (structural: `yrs` replaces `yjs`) — the parity contract with the JS `Y.Doc` is
//! *result-set equality* (the same materialized slots + the same undo/redo sequence), NOT byte-identity
//! of the CRDT encoding. The document shape mirrors `state/ydoc.ts`: a root `slots` map of nested
//! per-slot maps whose `position` is a plain JSON object (a `yrs` `Any::Map`), plus a root
//! `editorLayers` map whose `entityIds` arrays give each slot its Outliner folder.

mod apply_faction;
mod place_orbat;
mod soa;
mod store;

pub use apply_faction::{
    APPLY_ANCHOR_X, APPLY_ANCHOR_Y, ApplyFactionError, ApplyFactionResult, FactionLibraryInput,
    FactionLibraryRole, FactionLibraryVehicle, apply_faction_library,
};
pub use place_orbat::{PlaceOrbatError, place_character_under_side};
pub use soa::{NONE_IDX, STANCE_CROUCH, STANCE_PRONE, STANCE_STAND, SlotSoa};
pub use store::{
    ConnectionFinding, ConnectionKind, ConnectionRow, EntityTransformPatch, MissionDocCore,
    formation_offsets, validate_connection_rows,
};

#[cfg(test)]
mod reexport_pins {
    use super::{
        ConnectionFinding, ConnectionKind, ConnectionRow, formation_offsets,
        validate_connection_rows,
    };
    use std::collections::HashSet;

    /// T-767 — these names must resolve through `doc::`, not only as `pub` items trapped
    /// inside the private `store` module. Break any of the `pub use store::{…}` names and
    /// this fails to compile (the perturbation that proves the re-export is load-bearing).
    #[test]
    fn connection_and_formation_api_is_crate_public_via_doc() {
        assert_eq!(
            ConnectionKind::parse("sync").map(ConnectionKind::as_str),
            Some("sync")
        );
        assert_eq!(
            ConnectionKind::parse("group").map(ConnectionKind::as_str),
            Some("group")
        );
        assert_eq!(
            ConnectionKind::parse("triggerOwner").map(ConnectionKind::as_str),
            Some("triggerOwner")
        );
        assert!(ConnectionKind::parse("junk").is_none());
        assert_eq!(formation_offsets("wedge", 3).len(), 3);
        let rows: [ConnectionRow; 0] = [];
        let findings = validate_connection_rows(&rows, &HashSet::new());
        assert!(findings.is_empty());
        let _ = ConnectionFinding {
            code: "CONN-KIND",
            connection_id: String::new(),
            detail: String::new(),
        };
    }
}
