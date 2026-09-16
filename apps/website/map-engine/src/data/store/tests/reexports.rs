//! Role: reexports.
//! Position: `doc/tests` in the map engine's headless mission data domain.
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

/// The authoring operations the entity boundary re-exports are reachable through `data::store`
/// and nothing else: a host resolves its own document handle and then calls straight into these.
#[test]
fn entity_authoring_api_is_crate_public_via_doc() {
    use super::operations::entity::{
        ArmedPlacementKind, LayerDrag, ZoneDrawStep, begin_zone_draft, cancel_refile,
        close_zone_polygon_draft, ensure_layer, placement_is_armable, take_rename_armed,
    };
    use super::operations::zones::{DrawTarget, ZoneShape};

    assert!(placement_is_armable(ArmedPlacementKind::Character, false));
    assert!(take_rename_armed().is_none());
    cancel_refile();

    let core = super::MissionDocCore::new();
    assert_eq!(
        ensure_layer(&core, None, "layer-1", "Layer 1").layer_id,
        "layer-1"
    );

    let draft = begin_zone_draft(
        "boundary".to_string(),
        ZoneShape::Polygon,
        DrawTarget::Zone,
        None,
    );
    assert!(close_zone_polygon_draft(&draft, |ring| ring.len() >= 3).is_none());
    let _ = LayerDrag::Folder(String::new());
    let _ = ZoneDrawStep::Drawing;
}
