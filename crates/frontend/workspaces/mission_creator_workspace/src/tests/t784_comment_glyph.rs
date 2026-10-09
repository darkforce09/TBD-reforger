use super::{RouteTarget, comment_lane_xy, comment_points, pick_comment, route_target};

/// Three notes, deliberately NOT in id order in the JSON text, so a reader that trusted the
/// map's iteration order would produce a different sequence from one that sorts.
fn comments() -> String {
    serde_json::json!({
        "cmt-3": { "title": "South", "position": { "x": 300.0, "z": -30.0 } },
        "cmt-1": { "title": "North", "position": { "x": 100.0, "z": 10.0 } },
        "cmt-2": { "title": "East",  "position": { "x": 105.0, "z": 10.0 } },
    })
    .to_string()
}

/// **What is drawn IS what can be picked — both directions, over the whole set.**
///
/// The lane is `comment_points` packed, so for every glyph the lane draws there is a pick at
/// those very coordinates that returns that glyph's id, and every id a pick can return is a
/// glyph the lane drew. A second parser (which is what `mission_history` held before this
/// ticket) is exactly how those two sets start to differ.
#[test]
fn the_lane_is_the_pick_list_packed() {
    let pts = comment_points(&comments());
    let xy = comment_lane_xy(&comments());
    assert_eq!(pts.len(), 3, "all three notes must reach both surfaces");
    assert_eq!(
        xy.len(),
        pts.len() * 2,
        "T-784: the lane is two floats per picked point — no filtering between them"
    );
    let ids: Vec<&str> = pts.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        ids,
        ["cmt-1", "cmt-2", "cmt-3"],
        "T-784: sorted by id, so the lane's instance order cannot depend on serde_json's map \
         type across undo/redo/restore"
    );
    for (i, p) in pts.iter().enumerate() {
        // DRAWN ⇒ PICKABLE: hit-test at the exact coordinates the lane uploaded.
        #[allow(clippy::cast_possible_truncation)]
        let (lx, ly) = (f64::from(xy[i * 2]), f64::from(xy[i * 2 + 1]));
        assert!(
            (lx - p.x).abs() < 1e-3 && (ly - p.y).abs() < 1e-3,
            "T-784: lane vertex {i} is not the picked point {p:?}"
        );
        assert_eq!(
            pick_comment(&pts, lx, ly, 1.0).as_deref(),
            Some(p.id.as_str()),
            "T-784: a glyph drawn at ({lx}, {ly}) must be findable by a click there"
        );
    }
}

/// A comment's second axis is `z` — TWO HORIZONTALS, no height. Reading `y` would file every
/// note at northing 0 in the lane AND in the pick, which is a glyph drawn on the equator.
#[test]
fn the_second_axis_is_z_not_y() {
    let json = serde_json::json!({
        "cmt-1": { "position": { "x": 12.0, "y": 999.0, "z": 34.0 } }
    })
    .to_string();
    let pts = comment_points(&json);
    assert_eq!(
        (pts[0].x, pts[0].y),
        (12.0, 34.0),
        "T-784: `{{x, z}}`, never `y`"
    );
}

/// Nearest wins; beyond the tolerance nothing is picked (a click on empty map must stay a
/// deselect, not a phantom hit on the closest note in the mission).
#[test]
fn pick_takes_the_nearest_and_refuses_beyond_the_tolerance() {
    let pts = comment_points(&comments());
    // Between cmt-1 (100) and cmt-2 (105), one metre nearer cmt-2.
    assert_eq!(
        pick_comment(&pts, 103.0, 10.0, 20.0).as_deref(),
        Some("cmt-2"),
        "T-784: overlapping glyphs must resolve by distance, not by listing order"
    );
    assert_eq!(pick_comment(&pts, 103.0, 10.0, 1.0), None);
    assert_eq!(pick_comment(&pts, -9000.0, -9000.0, 50.0), None);
    assert_eq!(pick_comment(&[], 0.0, 0.0, 1e9), None);
    assert!(comment_points("not json").is_empty());
    assert!(comment_points("[]").is_empty());
}

/// **The resolver learned comments** — which is what lets `subject_id_routes` (the Outliner
/// row's affordance AND the dock-left search hit's) answer honestly, with no kind list on
/// either surface. The existing arms must be untouched: a widening that changed what an
/// already-resolving id resolves to would be a regression dressed as a feature.
#[test]
fn route_target_resolves_a_comment_without_disturbing_the_other_arms() {
    let root = serde_json::json!({
        "vehiclesById": { "v1": { "position": { "x": 7.0, "y": 9.0 } } },
        "entitiesById": { "e1": { "position": { "x": 1.0, "y": 2.0 } } },
        "zonesById": { "z1": { "shape": "circle", "center": { "x": 4.0, "y": 5.0 }, "radius": 3.0 } },
        "commentsById": { "cmt-1": { "position": { "x": 100.0, "z": 10.0 } } },
    });
    let not_slot = |_: &str| false;
    assert_eq!(
        route_target(&root, "cmt-1", &not_slot),
        Some(RouteTarget::Comment { x: 100.0, y: 10.0 }),
        "T-784: a commentsById row must resolve, at `{{x, z}}`"
    );
    assert_eq!(route_target(&root, "cmt-404", &not_slot), None);
    assert_eq!(
        route_target(&root, "v1", &not_slot),
        Some(RouteTarget::Vehicle { x: 7.0, y: 9.0 })
    );
    assert_eq!(
        route_target(&root, "e1", &not_slot),
        Some(RouteTarget::Entity { x: 1.0, y: 2.0 })
    );
    assert_eq!(
        route_target(&root, "s1", &|_| true),
        Some(RouteTarget::Slot)
    );
    // A comment row with no position resolves to NOTHING rather than to the origin — an
    // affordance over a click that would fly the camera to (0,0) is the T-754 defect.
    let broken = serde_json::json!({ "commentsById": { "cmt-2": { "title": "x" } } });
    assert_eq!(route_target(&broken, "cmt-2", &not_slot), None);
}
