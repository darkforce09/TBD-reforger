//! Role: the tactical lane's session — the selection, the draw, and the vertex drag.
//! Position: `doc/operations/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_draw_is_armed_only_for_a_kind_the_validator_carries_a_floor_for() {
    assert!(begin_tactical_draw("phase_line"));
    assert!(tactical_draw_armed());
    assert!(cancel_tactical_draw());

    assert!(!begin_tactical_draw("smoke_signal"));
    assert!(!tactical_draw_armed());
}

#[test]
fn clicks_build_the_draft_and_the_undo_vertex_control_walks_it_back() {
    assert!(begin_tactical_draw("curved_arrow"));
    assert_eq!(push_tactical_draw_vertex(0.0, 0.0), 1);
    assert_eq!(push_tactical_draw_vertex(10.0, 0.0), 2);
    assert_eq!(
        tactical_draft().expect("a draw is in flight").needed(),
        1,
        "a curved arrow is floored at three vertices"
    );
    assert_eq!(pop_tactical_draw_vertex(), 1);
    assert_eq!(
        tactical_draft().expect("a draw is in flight").verts,
        vec![(0.0, 0.0)]
    );
}

#[test]
fn clicks_with_no_draw_in_flight_build_nothing() {
    assert_eq!(push_tactical_draw_vertex(0.0, 0.0), 0);
    assert_eq!(pop_tactical_draw_vertex(), 0);
    assert!(!cancel_tactical_draw());
}

#[test]
fn a_draw_never_grows_past_the_point_count_the_document_carries() {
    assert!(begin_tactical_draw("phase_line"));
    for i in 0..crate::data::scenario::tactical_graphics::MAX_POINTS {
        assert_eq!(push_tactical_draw_vertex(i as f64, 0.0), i + 1);
    }
    assert_eq!(
        push_tactical_draw_vertex(1_000.0, 0.0),
        crate::data::scenario::tactical_graphics::MAX_POINTS,
        "the click past the ceiling is refused rather than counted"
    );
}

#[test]
fn a_finished_draw_retires_the_draft_and_selects_what_it_minted() {
    assert!(begin_tactical_draw("phase_line"));
    push_tactical_draw_vertex(0.0, 0.0);
    push_tactical_draw_vertex(10.0, 0.0);
    let (rows, id) = complete_tactical_draw(
        Vec::new(),
        &tactical_draft().expect("a draw is in flight"),
    );
    finish_tactical_draw(id.clone());

    assert_eq!(rows.len(), 1);
    assert!(!tactical_draw_armed());
    assert_eq!(selected_tactical_graphic(), Some(id));
}

#[test]
fn selecting_reports_only_the_changes_and_clearing_reports_only_the_first() {
    assert!(select_tactical_graphic(Some("tg_pl_1".to_string())));
    assert!(
        !select_tactical_graphic(Some("tg_pl_1".to_string())),
        "re-selecting what is already selected changes nothing"
    );
    assert!(select_tactical_graphic(None));
    assert!(!clear_tactical_selection());

    assert!(select_tactical_graphic(Some("tg_pl_1".to_string())));
    assert!(clear_tactical_selection());
}

#[test]
fn a_deleted_graphic_is_dropped_from_the_selection_and_its_neighbours_are_not() {
    select_tactical_graphic(Some("tg_pl_1".to_string()));
    forget_deleted_tactical_graphic("tg_pl_2");
    assert_eq!(selected_tactical_graphic(), Some("tg_pl_1".to_string()));

    forget_deleted_tactical_graphic("tg_pl_1");
    assert_eq!(selected_tactical_graphic(), None);
}

#[test]
fn arming_a_vertex_drag_selects_the_graphic_it_is_about_to_change() {
    assert!(!begin_tactical_vertex_drag(None));
    assert!(!tactical_vertex_drag_active());

    assert!(begin_tactical_vertex_drag(Some(("tg_pl_1".to_string(), 1))));
    assert!(tactical_vertex_drag_active());
    assert_eq!(selected_tactical_graphic(), Some("tg_pl_1".to_string()));
    assert_eq!(
        tactical_vertex_drag_preview(),
        None,
        "an armed drag that has not moved has nothing to preview"
    );
}

#[test]
fn a_moved_drag_previews_before_it_commits_and_commits_exactly_once() {
    assert!(begin_tactical_vertex_drag(Some(("tg_pl_1".to_string(), 1))));
    assert!(move_tactical_vertex_drag(5.0, 7.0));
    assert_eq!(
        tactical_vertex_drag_preview(),
        Some(("tg_pl_1".to_string(), 1, 5.0, 7.0))
    );

    assert_eq!(
        take_tactical_vertex_drag(),
        Some(("tg_pl_1".to_string(), 1, 5.0, 7.0))
    );
    assert_eq!(take_tactical_vertex_drag(), None, "the release consumed it");
    assert!(!tactical_vertex_drag_active());
}

#[test]
fn a_drag_that_never_moved_consumes_itself_without_yielding_a_write() {
    assert!(begin_tactical_vertex_drag(Some(("tg_pl_1".to_string(), 0))));
    assert_eq!(take_tactical_vertex_drag(), None);
    assert!(!tactical_vertex_drag_active());
}

#[test]
fn a_cancelled_drag_yields_nothing_and_leaves_the_document_alone() {
    assert!(!cancel_tactical_vertex_drag());
    assert!(begin_tactical_vertex_drag(Some(("tg_pl_1".to_string(), 0))));
    assert!(move_tactical_vertex_drag(5.0, 7.0));
    assert!(cancel_tactical_vertex_drag());
    assert_eq!(take_tactical_vertex_drag(), None);
    assert!(!move_tactical_vertex_drag(9.0, 9.0));
}
