//! Role: Domain regression cases.
//! Position: `streaming/scheduler/residency/t151_11_3_tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

#[test]
fn ingest_budget_policy_is_core_owned() {
    let mut r = WorldResidency::new();

    assert!(!r.ingest_budget_exhausted_at(1_000.0));
    r.begin_ingest_frame_at(1_000.0);
    assert!(!r.ingest_budget_exhausted_at(1_000.0 + APPLY_BUDGET_MS - 0.1));
    assert!(r.ingest_budget_exhausted_at(1_000.0 + APPLY_BUDGET_MS));

    let before = r.frames_over_budget();
    r.end_ingest_frame_at(1_000.0 + APPLY_BUDGET_MS + 2.0);
    assert_eq!(r.frames_over_budget(), before + 1);

    assert!(!r.ingest_budget_exhausted_at(10_000.0));
}

#[test]
fn buildings_toggle_hides_and_restores_whole_lane() {
    let mut r = residency_with_one_building();
    assert!(!r.world_building_fill().is_empty());
    assert!(!r.world_building_outline().is_empty());
    assert!(r.buildings_visible());

    r.set_glyph_toggles(true, false, false);
    assert!(
        r.world_building_fill().is_empty(),
        "fill must empty on toggle-off (P-04)"
    );
    assert!(
        r.world_building_outline().is_empty(),
        "outline must empty on toggle-off"
    );
    assert!(!r.buildings_visible());

    r.set_glyph_toggles(true, false, true);
    assert!(
        !r.world_building_fill().is_empty(),
        "fill must repopulate on toggle-on"
    );
    assert!(r.buildings_visible());
}

#[test]
fn buildings_visible_respects_zoom_gate() {
    let mut r = residency_with_one_building();
    assert!(r.buildings_visible());
    let _ = r.set_viewport(0.0, 0.0, 600.0, 600.0, -3.0);
    assert!(!r.buildings_visible());
}
