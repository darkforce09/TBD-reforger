//! Role: place helpers tests.
//! Position: `editor/tools/tests` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

fn pts(v: &[(f64, f64)]) -> Vec<Pt> {
    v.iter().map(|&(x, y)| Pt::new(x, y)).collect()
}

fn approx(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

fn pt_approx(a: Pt, b: Pt, eps: f64) -> bool {
    approx(a.x, b.x, eps) && approx(a.y, b.y, eps)
}

#[test]
fn confirm_threshold_boundary() {
    assert!(!needs_confirm(0));
    assert!(!needs_confirm(10), "exactly 10 does NOT confirm");
    assert!(needs_confirm(11), "11 (>10) confirms");
    assert!(needs_confirm(500));
    assert_eq!(DESTRUCTIVE_MOVE_THRESHOLD, 10);
}

#[test]
fn bearing_cardinals_clockwise_from_north() {
    let o = Pt::new(0.0, 0.0);
    assert!(approx(bearing_from_to(o, Pt::new(0.0, 10.0)), 0.0, 1e-9));
    assert!(approx(bearing_from_to(o, Pt::new(10.0, 0.0)), 90.0, 1e-9));
    assert!(approx(bearing_from_to(o, Pt::new(0.0, -10.0)), 180.0, 1e-9));
    assert!(approx(bearing_from_to(o, Pt::new(-10.0, 0.0)), 270.0, 1e-9));
    assert!(approx(bearing_from_to(o, o), 0.0, 1e-9));
}

#[test]
fn centroid_and_spread_and_bounds() {
    let p = pts(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
    assert_eq!(centroid(&p), Pt::new(5.0, 5.0));

    assert!(approx(max_spread(&p), 50.0_f64.sqrt(), 1e-9));
    assert_eq!(bounds(&p), (0.0, 0.0, 10.0, 10.0));
    assert_eq!(centroid(&[]), Pt::new(0.0, 0.0));
    assert_eq!(max_spread(&pts(&[(3.0, 3.0)])), 0.0);
}

#[test]
fn circular_golden_radius_floor_and_cardinals() {
    let p = pts(&[(99.0, 99.0), (101.0, 99.0), (101.0, 101.0), (99.0, 101.0)]);
    let out = pattern_circular(&p);
    let c = Pt::new(100.0, 100.0);
    let r = 5.0;
    let expected = [
        Pt::new(c.x, c.y + r),
        Pt::new(c.x + r, c.y),
        Pt::new(c.x, c.y - r),
        Pt::new(c.x - r, c.y),
    ];
    for (g, e) in out.iter().zip(expected.iter()) {
        assert!(pt_approx(*g, *e, 1e-9), "circular {g:?} vs {e:?}");
    }

    for g in &out {
        assert!(approx(
            ((g.x - c.x).powi(2) + (g.y - c.y).powi(2)).sqrt(),
            r,
            1e-9
        ));
    }
}

#[test]
fn circular_golden_radius_from_spread() {
    let p = pts(&[(0.0, 0.0), (40.0, 0.0)]);
    let out = pattern_circular(&p);
    let c = Pt::new(20.0, 0.0);
    assert!(pt_approx(out[0], Pt::new(c.x, c.y + 20.0), 1e-9));
    assert!(pt_approx(out[1], Pt::new(c.x, c.y - 20.0), 1e-9));
}

#[test]
fn line_golden_horizontal_equal_spacing() {
    let p = pts(&[(0.0, 0.0), (10.0, 3.0), (20.0, 0.0)]);
    let out = pattern_line(&p);
    let cy = 1.0;
    assert!(pt_approx(out[0], Pt::new(0.0, cy), 1e-9), "{:?}", out[0]);
    assert!(pt_approx(out[1], Pt::new(10.0, cy), 1e-9), "{:?}", out[1]);
    assert!(pt_approx(out[2], Pt::new(20.0, cy), 1e-9), "{:?}", out[2]);
}

#[test]
fn grid_golden_two_by_two() {
    let p = pts(&[(1.0, 1.0), (2.0, -1.0), (-1.0, 2.0), (-2.0, -2.0)]);

    let out = pattern_grid(&p);
    let expected = [
        Pt::new(-2.5, 2.5),
        Pt::new(2.5, 2.5),
        Pt::new(-2.5, -2.5),
        Pt::new(2.5, -2.5),
    ];
    for (g, e) in out.iter().zip(expected.iter()) {
        assert!(pt_approx(*g, *e, 1e-9), "grid {g:?} vs {e:?}");
    }
}

#[test]
fn grid_golden_five_is_three_by_two() {
    let p = pts(&[(0.0, 0.0); 5]);
    let out = pattern_grid_cell(&p, 10.0);

    let expected = [
        Pt::new(-10.0, 5.0),
        Pt::new(0.0, 5.0),
        Pt::new(10.0, 5.0),
        Pt::new(-10.0, -5.0),
        Pt::new(0.0, -5.0),
    ];
    for (g, e) in out.iter().zip(expected.iter()) {
        assert!(pt_approx(*g, *e, 1e-9), "grid5 {g:?} vs {e:?}");
    }
}

#[test]
fn fill_area_deterministic_and_contained() {
    let p = pts(&[
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 100.0),
        (0.0, 100.0),
        (50.0, 50.0),
    ]);
    let ids: Vec<String> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let seed = seed_from_ids(&ids);
    let run1 = pattern_fill_area(&p, seed);
    let run2 = pattern_fill_area(&p, seed);
    assert_eq!(run1, run2, "same seed → identical scatter (reproducible)");
    let hull = convex_hull(&p);
    for g in &run1 {
        assert!(
            point_in_convex_hull(&hull, *g),
            "scattered point {g:?} must be inside the hull"
        );
    }

    let run3 = pattern_fill_area(&p, seed.wrapping_add(1));
    assert_ne!(run1, run3, "different seed → different scatter");
}

#[test]
fn fill_area_seed_order_independent() {
    let a: Vec<String> = ["s1", "s2", "s3"].iter().map(|s| s.to_string()).collect();
    let b: Vec<String> = ["s3", "s1", "s2"].iter().map(|s| s.to_string()).collect();
    assert_eq!(seed_from_ids(&a), seed_from_ids(&b));
}

#[test]
fn align_golden_all_six_edges() {
    let p = pts(&[(0.0, 0.0), (10.0, 4.0), (6.0, 10.0)]);

    let left = align_edge(&p, AlignEdge::Left);
    assert_eq!(left, pts(&[(0.0, 0.0), (0.0, 4.0), (0.0, 10.0)]));
    let right = align_edge(&p, AlignEdge::Right);
    assert_eq!(right, pts(&[(10.0, 0.0), (10.0, 4.0), (10.0, 10.0)]));
    let top = align_edge(&p, AlignEdge::Top);
    assert_eq!(top, pts(&[(0.0, 10.0), (10.0, 10.0), (6.0, 10.0)]));
    let bottom = align_edge(&p, AlignEdge::Bottom);
    assert_eq!(bottom, pts(&[(0.0, 0.0), (10.0, 0.0), (6.0, 0.0)]));
    let ch = align_edge(&p, AlignEdge::CentreH);
    assert_eq!(ch, pts(&[(5.0, 0.0), (5.0, 4.0), (5.0, 10.0)]));
    let cv = align_edge(&p, AlignEdge::CentreV);
    assert_eq!(cv, pts(&[(0.0, 5.0), (10.0, 5.0), (6.0, 5.0)]));
}

#[test]
fn space_golden_horizontal_evens_interior() {
    let p = pts(&[(0.0, 1.0), (3.0, 2.0), (10.0, 3.0)]);
    let out = space_equally(&p, SpaceAxis::Horizontal);
    assert_eq!(out, pts(&[(0.0, 1.0), (5.0, 2.0), (10.0, 3.0)]));
}

#[test]
fn space_golden_vertical_evens_interior() {
    let p = pts(&[(1.0, 0.0), (2.0, 8.0), (3.0, 10.0)]);
    let out = space_equally(&p, SpaceAxis::Vertical);
    assert_eq!(out, pts(&[(1.0, 0.0), (2.0, 5.0), (3.0, 10.0)]));
}

#[test]
fn space_golden_along_line() {
    let p = pts(&[(0.0, 0.0), (3.0, 0.0), (10.0, 0.0)]);
    let out = space_equally(&p, SpaceAxis::AlongLine);
    assert!(pt_approx(out[0], Pt::new(0.0, 0.0), 1e-9), "{:?}", out[0]);
    assert!(pt_approx(out[1], Pt::new(5.0, 0.0), 1e-9), "{:?}", out[1]);
    assert!(pt_approx(out[2], Pt::new(10.0, 0.0), 1e-9), "{:?}", out[2]);
}

#[test]
fn space_along_line_keeps_perpendicular_offset() {
    let p = pts(&[(0.0, 0.0), (5.0, 2.0), (10.0, 0.0)]);
    let out = space_equally(&p, SpaceAxis::AlongLine);

    assert!(
        approx(out[1].y, 2.0, 1e-6),
        "perp offset preserved: {:?}",
        out[1]
    );
}

#[test]
fn orient_golden_cardinals_and_face() {
    let pivot = Pt::new(0.0, 0.0);

    let anywhere = Pt::new(37.0, -12.0);
    assert_eq!(orient_yaw(Orient::North, anywhere, pivot), Some(0.0));
    assert_eq!(orient_yaw(Orient::East, anywhere, pivot), Some(90.0));
    assert_eq!(orient_yaw(Orient::South, anywhere, pivot), Some(180.0));
    assert_eq!(orient_yaw(Orient::West, anywhere, pivot), Some(270.0));

    let east_of = Pt::new(10.0, 0.0);
    assert!(approx(
        orient_yaw(Orient::FaceCentre, east_of, pivot).unwrap(),
        270.0,
        1e-9
    ));

    assert!(approx(
        orient_yaw(Orient::FaceAway, east_of, pivot).unwrap(),
        90.0,
        1e-9
    ));

    let north_of = Pt::new(0.0, 10.0);
    assert!(approx(
        orient_yaw(Orient::FaceCentre, north_of, pivot).unwrap(),
        180.0,
        1e-9
    ));

    assert_eq!(orient_yaw(Orient::FaceCentre, pivot, pivot), None);
    assert_eq!(orient_yaw(Orient::North, pivot, pivot), Some(0.0));
}

#[test]
fn garrison_golden_square_four_positions() {
    let fp = garrison_firing_positions(0.0, 0.0, 6.0, 6.0, 0.0, 4);
    assert_eq!(fp.len(), 4);

    assert!(
        pt_approx(fp[0].pos, Pt::new(-5.0, 5.0), 1e-9),
        "{:?}",
        fp[0]
    );
    assert!(approx(fp[0].yaw_deg, 0.0, 1e-9));

    assert!(pt_approx(fp[1].pos, Pt::new(5.0, 5.0), 1e-9), "{:?}", fp[1]);
    assert!(approx(fp[1].yaw_deg, 90.0, 1e-9));

    assert!(
        pt_approx(fp[2].pos, Pt::new(5.0, -5.0), 1e-9),
        "{:?}",
        fp[2]
    );
    assert!(approx(fp[2].yaw_deg, 180.0, 1e-9));

    assert!(
        pt_approx(fp[3].pos, Pt::new(-5.0, -5.0), 1e-9),
        "{:?}",
        fp[3]
    );
    assert!(approx(fp[3].yaw_deg, 270.0, 1e-9));
}

#[test]
fn garrison_caps_and_degenerate() {
    assert_eq!(
        garrison_firing_positions(0.0, 0.0, 6.0, 6.0, 0.0, 0).len(),
        0
    );
    assert_eq!(
        garrison_firing_positions(0.0, 0.0, 1.0, 6.0, 0.0, 4).len(),
        0
    );
    assert_eq!(
        garrison_firing_positions(0.0, 0.0, 0.5, 0.5, 0.0, 4).len(),
        0
    );
    assert_eq!(
        garrison_firing_positions(0.0, 0.0, 10.0, 10.0, 0.0, 8).len(),
        8
    );
}

#[test]
fn garrison_rotation_rotates_positions_and_yaw() {
    let fp0 = garrison_firing_positions(0.0, 0.0, 6.0, 6.0, 0.0, 4);
    let fp90 = garrison_firing_positions(0.0, 0.0, 6.0, 6.0, 90.0, 4);

    assert!(approx(fp90[0].yaw_deg, 90.0, 1e-9), "{:?}", fp90[0]);

    assert!(
        pt_approx(fp90[0].pos, Pt::new(5.0, 5.0), 1e-9),
        "{:?}",
        fp90[0]
    );

    assert_eq!(fp0.len(), fp90.len());
}

#[test]
fn patterns_noop_below_two() {
    let one = pts(&[(3.0, 4.0)]);
    assert_eq!(pattern_circular(&one), one);
    assert_eq!(pattern_line(&one), one);
    assert_eq!(pattern_grid(&one), one);
    assert_eq!(pattern_fill_area(&one, 42), one);
    assert_eq!(align_edge(&one, AlignEdge::Left), one);
    assert_eq!(space_equally(&one, SpaceAxis::Horizontal), one);
}

#[test]
fn principal_axis_picks_dominant_spread() {
    let p = pts(&[(-10.0, 0.5), (0.0, -0.5), (10.0, 0.5)]);
    let (ux, uy) = principal_axis(&p);
    assert!(ux.abs() > 0.99 && uy.abs() < 0.1, "axis=({ux},{uy})");

    let q = pts(&[(0.5, -10.0), (-0.5, 0.0), (0.5, 10.0)]);
    let (vx, vy) = principal_axis(&q);
    assert!(vy.abs() > 0.99 && vx.abs() < 0.1, "axis=({vx},{vy})");
}

#[test]
fn convex_hull_of_square_with_interior_point() {
    let p = pts(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
        (5.0, 5.0),
    ]);
    let hull = convex_hull(&p);
    assert_eq!(hull.len(), 4, "interior point dropped");

    assert!(point_in_convex_hull(&hull, Pt::new(5.0, 5.0)));
    assert!(point_in_convex_hull(&hull, Pt::new(0.0, 0.0)));
    assert!(!point_in_convex_hull(&hull, Pt::new(-1.0, 5.0)));
}
