//! **Role:** unit tests for the fire-mission overlay geometry: lane assignment and paint order,
//! glyph quads, trimmed gun→target lines, dispersion-ellipse rings and outlines, and the
//! non-finite guards.
//! **Position:** `overlay/tests` in the map engine, declared by `overlay/fire_mission_marks.rs`.
//! **Signals & state:** none; pure functions over literal plots.
//! **Invariants:** expected positions are written out by hand in world metres, and every test
//! runs against a non-zero anchor so an anchor slip cannot pass unseen.

use super::*;
use crate::overlay::lanes::lane_order;

const ANCHOR: [f64; 2] = [6400.0, 6400.0];

fn close(a: f32, b: f64) -> bool {
    (f64::from(a) - b).abs() < 1e-3
}

fn one_gun_plot() -> FireMissionPlot {
    FireMissionPlot {
        guns: vec![[7000.0, 5000.0]],
        target: Some([7000.0, 7000.0]),
        burst: None,
        dispersion: Vec::new(),
    }
}

#[test]
fn lanes_paint_fill_below_glyphs_below_lines() {
    assert!(lane_order(FIRE_MISSION_DISPERSION_LANE) < lane_order(FIRE_MISSION_GLYPH_LANE));
    assert!(lane_order(FIRE_MISSION_GLYPH_LANE) < lane_order(FIRE_MISSION_LINE_LANE));
    assert!(lane_order(FIRE_MISSION_DISPERSION_LANE) > lane_order(LaneRole::Grid));
    assert!(lane_order(FIRE_MISSION_LINE_LANE) < lane_order(LaneRole::Marquee));
}

#[test]
fn one_gun_draws_gun_and_target_glyphs_and_one_line() {
    let marks = build_fire_mission_marks(&one_gun_plot(), 20.0, ANCHOR);
    let kinds: Vec<_> = marks.glyph_quads.iter().map(|q| q.glyph).collect();
    assert_eq!(kinds, [FireMissionGlyph::Gun, FireMissionGlyph::Target]);
    assert_eq!(marks.line_vertices.len(), 2);
    assert!(marks.dispersion_rings.is_empty());
}

#[test]
fn glyph_quad_is_an_anchor_relative_square_around_its_point() {
    let marks = build_fire_mission_marks(&one_gun_plot(), 20.0, ANCHOR);
    let gun = marks.glyph_quads[0];
    // Gun at (7000, 5000) world = (600, -1400) relative to the anchor.
    assert!(close(gun.centre[0], 600.0) && close(gun.centre[1], -1400.0));
    let expected = [
        [580.0, -1420.0],
        [620.0, -1420.0],
        [620.0, -1380.0],
        [580.0, -1380.0],
    ];
    for (corner, want) in gun.corners.iter().zip(expected) {
        assert!(
            close(corner[0], want[0]) && close(corner[1], want[1]),
            "{corner:?}"
        );
    }
    assert_eq!(gun.color, GUN_COLOR);
    assert_eq!(marks.glyph_quads[1].color, TARGET_COLOR);
}

#[test]
fn gun_target_line_stops_at_both_glyph_edges() {
    let marks = build_fire_mission_marks(&one_gun_plot(), 20.0, ANCHOR);
    let (start, end) = (marks.line_vertices[0], marks.line_vertices[1]);
    // Gun (7000, 5000) → target (7000, 7000), due north, trimmed 20 m at each end.
    assert!(
        close(start.pos[0], 600.0) && close(start.pos[1], -1380.0),
        "{start:?}"
    );
    assert!(
        close(end.pos[0], 600.0) && close(end.pos[1], 580.0),
        "{end:?}"
    );
    assert_eq!(start.color, GUN_TARGET_LINE_COLOR);
    assert_eq!(end.color, GUN_TARGET_LINE_COLOR);
}

#[test]
fn diagonal_line_is_trimmed_along_its_own_direction() {
    let plot = FireMissionPlot {
        guns: vec![[6400.0, 6400.0]],
        target: Some([6700.0, 6800.0]),
        ..FireMissionPlot::default()
    };
    let marks = build_fire_mission_marks(&plot, 50.0, ANCHOR);
    // A 300-400-500 segment: 50 m of trim is (30, 40) at each end.
    let (start, end) = (marks.line_vertices[0], marks.line_vertices[1]);
    assert!(
        close(start.pos[0], 30.0) && close(start.pos[1], 40.0),
        "{start:?}"
    );
    assert!(
        close(end.pos[0], 270.0) && close(end.pos[1], 360.0),
        "{end:?}"
    );
}

#[test]
fn battery_draws_one_line_per_gun() {
    let plot = FireMissionPlot {
        guns: vec![[5000.0, 5000.0], [5100.0, 5000.0], [5200.0, 5000.0]],
        target: Some([5100.0, 8000.0]),
        ..FireMissionPlot::default()
    };
    let marks = build_fire_mission_marks(&plot, 10.0, ANCHOR);
    assert_eq!(marks.line_vertices.len(), 6);
    let guns = marks
        .glyph_quads
        .iter()
        .filter(|q| q.glyph == FireMissionGlyph::Gun)
        .count();
    assert_eq!(guns, 3);
}

#[test]
fn burst_glyph_follows_the_target_only_when_present() {
    let mut plot = one_gun_plot();
    plot.burst = Some([7010.0, 7020.0]);
    let marks = build_fire_mission_marks(&plot, 20.0, ANCHOR);
    let last = *marks.glyph_quads.last().unwrap();
    assert_eq!(last.glyph, FireMissionGlyph::Burst);
    assert_eq!(last.color, BURST_COLOR);
    assert!(close(last.centre[0], 610.0) && close(last.centre[1], 620.0));
    let without = build_fire_mission_marks(&one_gun_plot(), 20.0, ANCHOR);
    assert!(
        without
            .glyph_quads
            .iter()
            .all(|q| q.glyph != FireMissionGlyph::Burst)
    );
}

#[test]
fn line_is_omitted_when_the_glyphs_touch() {
    let plot = FireMissionPlot {
        guns: vec![[7000.0, 7000.0]],
        target: Some([7000.0, 7040.0]),
        ..FireMissionPlot::default()
    };
    assert!(
        build_fire_mission_marks(&plot, 20.0, ANCHOR)
            .line_vertices
            .is_empty()
    );
    let apart = build_fire_mission_marks(&plot, 19.0, ANCHOR);
    assert_eq!(apart.line_vertices.len(), 2);
}

#[test]
fn no_target_draws_guns_without_lines() {
    let plot = FireMissionPlot {
        guns: vec![[7000.0, 7000.0]],
        ..FireMissionPlot::default()
    };
    let marks = build_fire_mission_marks(&plot, 20.0, ANCHOR);
    assert_eq!(marks.glyph_quads.len(), 1);
    assert!(marks.line_vertices.is_empty());
}

#[test]
fn dispersion_ring_lies_on_the_ellipse_with_the_major_axis_on_the_azimuth() {
    let ellipse = DispersionEllipse {
        centre: [8000.0, 9000.0],
        major_semi_axis_m: 40.0,
        minor_semi_axis_m: 15.0,
        major_axis_azimuth_rad: std::f64::consts::FRAC_PI_2,
    };
    let ring = dispersion_ring(&ellipse).unwrap();
    assert_eq!(ring.len(), DISPERSION_RING_VERTICES);
    // Azimuth 90° (east): the major axis runs along +x, the minor axis along -y (clockwise).
    assert!((ring[0][0] - 8040.0).abs() < 1e-9 && (ring[0][1] - 9000.0).abs() < 1e-9);
    let quarter = ring[DISPERSION_RING_VERTICES / 4];
    assert!((quarter[0] - 8000.0).abs() < 1e-9 && (quarter[1] - 8985.0).abs() < 1e-9);
    for p in &ring {
        let (dx, dy) = (p[0] - 8000.0, p[1] - 9000.0);
        let on_ellipse = (dx / 40.0).powi(2) + (dy / 15.0).powi(2);
        assert!((on_ellipse - 1.0).abs() < 1e-9, "{p:?} -> {on_ellipse}");
    }
}

#[test]
fn dispersion_ring_rotates_with_a_north_azimuth() {
    let ellipse = DispersionEllipse {
        centre: [0.0, 0.0],
        major_semi_axis_m: 30.0,
        minor_semi_axis_m: 10.0,
        major_axis_azimuth_rad: 0.0,
    };
    let ring = dispersion_ring(&ellipse).unwrap();
    assert!(ring[0][0].abs() < 1e-9 && (ring[0][1] - 30.0).abs() < 1e-9);
    let quarter = ring[DISPERSION_RING_VERTICES / 4];
    assert!((quarter[0] - 10.0).abs() < 1e-9 && quarter[1].abs() < 1e-9);
}

#[test]
fn dispersion_outline_is_a_closed_line_list_in_the_line_lane() {
    let plot = FireMissionPlot {
        dispersion: vec![DispersionEllipse {
            centre: [7000.0, 7000.0],
            major_semi_axis_m: 25.0,
            minor_semi_axis_m: 12.0,
            major_axis_azimuth_rad: 0.7,
        }],
        ..FireMissionPlot::default()
    };
    let marks = build_fire_mission_marks(&plot, 20.0, ANCHOR);
    assert_eq!(marks.dispersion_rings.len(), 1);
    let ring = &marks.dispersion_rings[0];
    let outline = &marks.line_vertices;
    assert_eq!(outline.len(), 2 * DISPERSION_RING_VERTICES);
    for (i, segment) in outline.chunks(2).enumerate() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        assert!(close(segment[0].pos[0], a[0] - 6400.0) && close(segment[0].pos[1], a[1] - 6400.0));
        assert!(close(segment[1].pos[0], b[0] - 6400.0) && close(segment[1].pos[1], b[1] - 6400.0));
        assert_eq!(segment[0].color, DISPERSION_OUTLINE_COLOR);
    }
    assert_eq!(outline.last().unwrap().pos, outline[0].pos);
}

#[test]
fn degenerate_ellipses_are_skipped() {
    let base = DispersionEllipse {
        centre: [7000.0, 7000.0],
        major_semi_axis_m: 25.0,
        minor_semi_axis_m: 12.0,
        major_axis_azimuth_rad: 0.0,
    };
    let bad = [
        DispersionEllipse {
            major_semi_axis_m: 0.0,
            ..base
        },
        DispersionEllipse {
            minor_semi_axis_m: -1.0,
            ..base
        },
        DispersionEllipse {
            major_semi_axis_m: f64::NAN,
            ..base
        },
        DispersionEllipse {
            major_axis_azimuth_rad: f64::INFINITY,
            ..base
        },
        DispersionEllipse {
            centre: [f64::NAN, 7000.0],
            ..base
        },
    ];
    for ellipse in bad {
        assert_eq!(dispersion_ring(&ellipse), None, "{ellipse:?}");
    }
    let plot = FireMissionPlot {
        dispersion: bad.to_vec(),
        ..FireMissionPlot::default()
    };
    assert_eq!(
        build_fire_mission_marks(&plot, 20.0, ANCHOR),
        FireMissionMarks::default()
    );
}

#[test]
fn non_finite_positions_are_skipped_and_every_output_is_finite() {
    let plot = FireMissionPlot {
        guns: vec![[f64::NAN, 5000.0], [7000.0, 5000.0], [f64::INFINITY, 0.0]],
        target: Some([7000.0, 7000.0]),
        burst: Some([f64::NAN, f64::NAN]),
        dispersion: Vec::new(),
    };
    let marks = build_fire_mission_marks(&plot, 20.0, ANCHOR);
    let kinds: Vec<_> = marks.glyph_quads.iter().map(|q| q.glyph).collect();
    assert_eq!(kinds, [FireMissionGlyph::Gun, FireMissionGlyph::Target]);
    assert_eq!(marks.line_vertices.len(), 2);
    let quad_points = marks
        .glyph_quads
        .iter()
        .flat_map(|q| q.corners.into_iter().chain([q.centre]));
    let line_points = marks.line_vertices.iter().map(|v| v.pos);
    assert!(
        quad_points
            .chain(line_points)
            .all(|p| p[0].is_finite() && p[1].is_finite())
    );

    let no_target = FireMissionPlot {
        target: Some([7000.0, f64::NAN]),
        ..plot
    };
    let marks = build_fire_mission_marks(&no_target, 20.0, ANCHOR);
    assert!(marks.line_vertices.is_empty());
    assert!(
        marks
            .glyph_quads
            .iter()
            .all(|q| q.glyph == FireMissionGlyph::Gun)
    );
}

#[test]
fn unusable_glyph_size_draws_no_glyphs_and_full_lines() {
    for size in [0.0, -5.0, f64::NAN, f64::INFINITY] {
        let marks = build_fire_mission_marks(&one_gun_plot(), size, ANCHOR);
        assert!(marks.glyph_quads.is_empty(), "size {size}");
        let (start, end) = (marks.line_vertices[0], marks.line_vertices[1]);
        assert!(
            close(start.pos[1], -1400.0) && close(end.pos[1], 600.0),
            "size {size}"
        );
    }
}
