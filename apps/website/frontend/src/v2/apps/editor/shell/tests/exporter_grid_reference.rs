//! The clipboard exporter's grid reference against the map furniture's own edge labels.
//!
//! This pin lives frontend-side because it spans the wall: the edge labels it compares against are
//! drawn by `editor::panels::toolbelt` inside the frontend's own dock geometry, and the frontend may
//! import the engine while the engine may never import the frontend.

use website_map_engine::camera::ortho::state::OrthoCamera;
use website_map_engine::editing::commands::selection_digest::format_grid_ref;

use crate::v2::apps::editor::panels::toolbelt::{edge_eastings, edge_northings, GRID_STEP_M};
use crate::v2::apps::editor::shell::layout::{DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX};

/// **The clipboard and the screen can never disagree.**
///
/// The map pane draws grid-reference labels on its top and left edges. A mission maker reads an
/// easting off the top edge and a northing off the left edge and says the pair out loud. This
/// asserts that an entity standing exactly on one of those intersections exports precisely those
/// two label strings, in that order, separated by one space. The labels are taken from
/// `edge_eastings` / `edge_northings` themselves, not recomputed, so a change to the furniture's
/// formatting fails HERE rather than shipping a clipboard convention nobody reconciled.
#[test]
fn the_exporter_grid_ref_is_the_map_furnitures_own_label_text() {
    let (w, h) = (1600.0_f64, 900.0_f64);
    let mut cam = OrthoCamera::new(w, h, 6400.0, 6400.0, -2.0);
    cam.set_bounds(0.0, 0.0, 12_800.0, 12_800.0);
    let pane_right = w - DOCK_RIGHT_PX;

    let eastings = edge_eastings(&cam, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
    let northings = edge_northings(&cam, DOCK_LEFT_PX, STRIP_TOP_PX, h);
    assert!(
        !eastings.is_empty() && !northings.is_empty(),
        "the fixture camera must actually show grid labels to compare against"
    );

    let mut checked = 0usize;
    for e in &eastings {
        // The world X the label sits on, snapped to its 1 km line.
        let wx = (cam.unproject_xy(e.pos_px, STRIP_TOP_PX)[0] / GRID_STEP_M).round() * GRID_STEP_M;
        for n in &northings {
            let wy =
                (cam.unproject_xy(DOCK_LEFT_PX, n.pos_px)[1] / GRID_STEP_M).round() * GRID_STEP_M;
            assert_eq!(
                format_grid_ref(wx, wy),
                format!("{} {}", e.text, n.text),
                "an entity on the intersection of the '{}' easting and the '{}' northing must \
                 export exactly what the map edges print",
                e.text,
                n.text
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 4,
        "expected a grid of intersections, got {checked}"
    );

    // …and a position BETWEEN two labelled lines reads inside the band they bracket — the
    // six-figure read the furniture invites (1250 m sits between the "010" and "020" eastings,
    // and 4800 m between the "040" and "050" northings).
    assert_eq!(format_grid_ref(1250.0, 4800.0), "012 048");
}
