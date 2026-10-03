//! The prelude surface of the mission document.
//!
//! **Role:** proves the connection vocabulary, its validation and the formation offsets are
//! reachable through `mission_document::prelude` alone.
//! **Position:** a whole-crate test, mounted from the crate's `tests` module.
//! **Signals & state:** none.
//! **Invariants:** a caller that glob-imports the prelude names every connection kind by its wire
//! spelling and validates rows without importing a module path.

use crate::prelude::*;
use std::collections::HashSet;

#[test]
fn connection_and_formation_api_is_reachable_through_the_prelude() {
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
        connection_id: ConnectionId::from(String::new()),
        detail: String::new(),
    };
}
