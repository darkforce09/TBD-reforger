//! Role: the zone and trigger draw machine, click by click.
//! Position: `doc/operations/entity/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

fn radius_from_rim(cx: f64, cz: f64, rim_x: f64, rim_z: f64) -> Option<(f64, f64, f64)> {
    let r = (rim_x - cx).hypot(rim_z - cz);
    (r >= 1.0).then_some((cx, cz, r))
}

fn ring_of_three_or_more(ring: &[(f64, f64)]) -> bool {
    ring.len() >= 3
}

#[test]
fn a_zone_kind_is_checked_against_the_authored_types_and_a_trigger_against_the_activations() {
    let authored = || vec!["boundary".to_string(), "objective".to_string()];
    assert!(zone_draft_kind_is_valid(
        "boundary",
        DrawTarget::Zone,
        authored
    ));
    assert!(!zone_draft_kind_is_valid(
        "invented",
        DrawTarget::Zone,
        authored
    ));
    assert!(zone_draft_kind_is_valid(
        "presence",
        DrawTarget::Trigger,
        Vec::new
    ));
    assert!(!zone_draft_kind_is_valid(
        "boundary",
        DrawTarget::Trigger,
        Vec::new
    ));
}

#[test]
fn a_circle_captures_its_centre_then_closes_on_the_rim() {
    let mut draft = begin_zone_draft(
        "boundary".to_string(),
        ZoneShape::Circle,
        DrawTarget::Zone,
        None,
    );
    assert_eq!(
        advance_zone_draft(&mut draft, 10.0, 20.0, radius_from_rim),
        ZoneDrawStep::Drawing
    );
    assert_eq!(draft.centre, Some((10.0, 20.0)));

    let closed = advance_zone_draft(&mut draft, 10.0, 25.0, radius_from_rim);
    assert_eq!(
        closed,
        ZoneDrawStep::CircleClosed {
            kind: "boundary".to_string(),
            centre: (10.0, 20.0),
            radius: 5.0,
            target: None,
            collection: DrawTarget::Zone,
        }
    );
}

#[test]
fn a_rim_the_radius_rule_refuses_keeps_the_draw_in_flight_with_its_centre() {
    let mut draft = begin_zone_draft(
        "presence".to_string(),
        ZoneShape::Circle,
        DrawTarget::Trigger,
        Some("t1".to_string()),
    );
    let _ = advance_zone_draft(&mut draft, 0.0, 0.0, radius_from_rim);
    assert_eq!(
        advance_zone_draft(&mut draft, 0.1, 0.0, radius_from_rim),
        ZoneDrawStep::Drawing
    );
    assert_eq!(draft.centre, Some((0.0, 0.0)));
}

#[test]
fn a_polygon_collects_one_vertex_per_click_and_pops_the_last() {
    let mut draft = begin_zone_draft(
        "boundary".to_string(),
        ZoneShape::Polygon,
        DrawTarget::Zone,
        None,
    );
    for (x, z) in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)] {
        assert_eq!(
            advance_zone_draft(&mut draft, x, z, radius_from_rim),
            ZoneDrawStep::Drawing
        );
    }
    assert_eq!(draft.verts.len(), 3);
    assert_eq!(pop_zone_draft_vertex(&mut draft), 2);
}

#[test]
fn a_ring_below_three_vertices_does_not_close_and_a_closed_one_carries_the_draw() {
    let mut draft = begin_zone_draft(
        "boundary".to_string(),
        ZoneShape::Polygon,
        DrawTarget::Zone,
        Some("z7".to_string()),
    );
    let _ = advance_zone_draft(&mut draft, 0.0, 0.0, radius_from_rim);
    let _ = advance_zone_draft(&mut draft, 10.0, 0.0, radius_from_rim);
    assert!(close_zone_polygon_draft(&draft, ring_of_three_or_more).is_none());

    let _ = advance_zone_draft(&mut draft, 10.0, 10.0, radius_from_rim);
    let commit = close_zone_polygon_draft(&draft, ring_of_three_or_more).expect("ring closes");
    assert_eq!(commit.kind, "boundary");
    assert_eq!(commit.target.as_deref(), Some("z7"));
    assert_eq!(commit.collection, DrawTarget::Zone);
    assert_eq!(commit.ring, vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
}

#[test]
fn a_circle_draw_never_closes_through_the_polygon_path() {
    let draft = begin_zone_draft(
        "boundary".to_string(),
        ZoneShape::Circle,
        DrawTarget::Zone,
        None,
    );
    assert!(close_zone_polygon_draft(&draft, |_| true).is_none());
}
