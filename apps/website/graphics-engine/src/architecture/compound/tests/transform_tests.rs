//! Role: transform tests.
//! Position: `architecture/compound/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::architecture::compound::transform::*;

fn close(a: [f64; 3], b: [f64; 3], eps: f64) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() <= eps)
}

#[test]
fn rot_y_turns_x_toward_z_and_inverse_undoes() {
    let r = Rigid::rot_y(90.0);
    assert!(close(r.dir([1.0, 0.0, 0.0]), [0.0, 0.0, -1.0], 1e-12));
    assert!(close(r.dir([0.0, 0.0, 1.0]), [1.0, 0.0, 0.0], 1e-12));
    let t = Rigid::from_enfusion([3.0, 1.0, -2.0], [10.0, -35.0, 5.0], 1.25);
    let p = [7.5, -2.0, 11.0];
    let back = t.inverse().point(t.point(p));
    assert!(close(back, p, 1e-12), "{back:?}");
    let id = t.compose(&t.inverse());
    assert!(close(id.t, [0.0; 3], 1e-12) && (id.scale - 1.0).abs() < 1e-12);
    for i in 0..3 {
        for j in 0..3 {
            let want = if i == j { 1.0 } else { 0.0 };
            assert!((id.m[i][j] - want).abs() < 1e-12);
        }
    }
}

#[test]
fn quaternion_round_trips_and_matches_euler_axes() {
    let s = 0.5f64.sqrt();
    let q = Rigid::from_quat_pos([0.0, s, 0.0, s], [1.0, 2.0, 3.0]);
    let e = Rigid::from_enfusion([1.0, 2.0, 3.0], [0.0, 90.0, 0.0], 1.0);
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                (q.m[i][j] - e.m[i][j]).abs() < 1e-12,
                "yaw 90 via quat == euler"
            );
        }
    }
    let back = q.to_quat();
    assert!(
        close([back[0], back[1], back[2]], [0.0, s, 0.0], 1e-12) && (back[3] - s).abs() < 1e-12
    );
    assert!((q.yaw_deg() - 90.0).abs() < 1e-9);

    let g = Rigid::from_enfusion([0.0; 3], [88.816, -180.0, 96.7], 1.0);
    let g2 = Rigid::from_quat_pos(g.to_quat(), [0.0; 3]);
    for i in 0..3 {
        for j in 0..3 {
            assert!((g.m[i][j] - g2.m[i][j]).abs() < 1e-9);
        }
    }

    let ypr = Rigid::from_enfusion([0.0; 3], [30.0, 40.0, 50.0], 1.0);
    let manual = Rigid::rot_y(40.0)
        .compose(&Rigid::rot_x(-30.0))
        .compose(&Rigid::rot_z(-50.0));
    for i in 0..3 {
        for j in 0..3 {
            assert!((ypr.m[i][j] - manual.m[i][j]).abs() < 1e-12);
        }
    }
}

#[test]
fn nested_composition_keeps_sub_micrometre_precision() {
    let building = Rigid::from_enfusion([6400.0, 51.2, 6410.5], [0.0, 137.5, 0.0], 1.0);
    let prop = Rigid::from_enfusion([-8.87, 3.58, -5.29], [88.816, -180.0, 96.7], 1.152);
    let world_to_prop = building.compose(&prop).inverse();
    let p_world = [6412.345, 55.5, 6398.25];
    let p_prop = world_to_prop.point(p_world);
    let back = building.compose(&prop).point(p_prop);
    assert!(close(back, p_world, 1e-6), "{back:?} vs {p_world:?}");
    let (lo, hi) = prop.aabb_of([-0.5, 0.0, -0.5], [0.5, 0.8, 0.5]);
    assert!(lo.iter().zip(hi.iter()).all(|(a, b)| a < b));
}
