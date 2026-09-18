//! Source pin for the heartbeat's foreign-key wiring.

use crate::match_telemetry::handlers::ingest_parsing::tests::{
    collapse_ws, production_half, strip_rust_comments,
};

const HEARTBEAT_SRC: &str = include_str!("../server_heartbeat.rs");

/// Class-R: the heartbeat's `server_statuses` write must route its error through
/// `foreign_key_error`, and the fallthrough must still be `e.into()`.
///
/// The helper being correct proves nothing if the call site never calls it — that is the
/// signature defect this program keeps finding, so pin the wiring, not just the helper.
/// Perturbation: restore a bare `.await?` on that statement and this goes red (measured —
/// it is how the 500 was reproduced).
#[test]
fn heartbeat_status_write_maps_foreign_key_violations() {
    let production = production_half(HEARTBEAT_SRC);
    let hb_start = production
        .find("pub async fn ingest_server_status")
        .expect("ingest_server_status must exist");
    let hb_after = &production[hb_start..];
    let hb_end = hb_after[1..]
        .find("\npub async fn ")
        .map(|i| i + 1)
        .unwrap_or(hb_after.len());
    let collapsed = collapse_ws(&strip_rust_comments(&hb_after[..hb_end]));

    assert!(
        collapsed.contains("INSERT INTO server_statuses"),
        "guard on the right handler"
    );
    // Asserted as fragments, not one literal: rustfmt is free to re-wrap the closure, and a
    // pin that only matches today's line breaks fails for a reason that has nothing to do
    // with the defect.
    assert!(
        collapsed.contains("foreign_key_error(&e)"),
        "the server_statuses write must route its error through foreign_key_error \
         (perturbation: bare `.await?` → 500 on an unregistered server_id)"
    );
    assert!(
        collapsed.contains("e.into()"),
        "…and must still hand anything foreign_key_error declines to the 500 path"
    );
}
