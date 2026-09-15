//! Role: reexports.
//! Position: `doc/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{
    ConnectionFinding, ConnectionKind, ConnectionRow, formation_offsets, validate_connection_rows,
};
use std::collections::HashSet;

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
