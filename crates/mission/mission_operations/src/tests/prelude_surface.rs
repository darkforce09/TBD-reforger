//! The prelude surface of the mission operations.
//!
//! **Role:** proves the entity authoring commands and the authoring session state a host keeps
//! between gestures are reachable through `mission_operations::prelude` alone.
//! **Position:** a whole-crate test, mounted from the crate root.
//! **Signals & state:** drives the thread-local cargo defaults, loadout buffer, Apply seed,
//! tactical draw machine and armed refile of the test thread.
//! **Invariants:** a host that glob-imports the prelude resolves its own document handle and then
//! calls straight into these, and the session state lives in this crate only.

use crate::prelude::*;
use mission_document::MissionDocCore;

/// The entity authoring commands a host calls after resolving its own document handle.
#[test]
fn entity_authoring_api_is_reachable_through_the_prelude() {
    assert!(placement_is_armable(ArmedPlacementKind::Character, false));
    assert!(take_rename_armed().is_none());
    cancel_refile();

    let core = MissionDocCore::new();
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

/// The session state the authoring host keeps between gestures: the installed cargo defaults and
/// the loadout buffer, the tactical draw machine, and the armed placement a canvas release commits.
#[test]
fn authoring_session_state_is_reachable_through_the_prelude() {
    set_cargo_defaults(std::collections::HashMap::new());
    assert_eq!(cargo_defaults_for("char.none"), None);
    assert_eq!(loadout_buffer_len(), 0);
    assert!(loadout_buffer().is_empty());
    assert_ne!(next_apply_seed(), next_apply_seed());

    assert!(begin_tactical_draw("phase_line"));
    assert!(tactical_draw_armed());
    assert!(cancel_tactical_draw());
    assert!(!clear_tactical_selection());

    assert!(vehicle_places_its_crew(true, false));
    let _ = ArmedPlacement::ZoneDraw;
    assert_eq!(PlacementCommit::default().selection, None);
}
