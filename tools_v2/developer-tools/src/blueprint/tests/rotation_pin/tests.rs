use super::*;
use website_map_engine::world::architecture::compound::transform::Rigid;

#[test]
fn rigid_from_enfusion_is_the_y_x_z_hypothesis() {
    let angles = [14.991, 90.532, -90.279];
    let m = RIGID_HYPOTHESIS.matrix(angles);
    let r = Rigid::from_enfusion([0.0; 3], angles, 1.0);
    for (i, row) in m.iter().enumerate() {
        for (j, mij) in row.iter().enumerate() {
            assert!((*mij - r.m[i][j]).abs() < 1e-12, "{i}{j}");
        }
    }
    let back = RIGID_HYPOTHESIS.decompose(&m);
    for k in 0..3 {
        assert!((back[k] - angles[k]).abs() < 1e-5, "{back:?}");
    }
    assert_eq!(Hypothesis::all().len(), 48);
}

/// The pin: GarbageContainer_01 (tilted 3.0°/4.75°) with its lid child (pitch -55°) as
/// recorded by the Workbench recon on 2026-09-03.
#[test]
fn garbage_container_lid_pins_y_x_z_with_negated_pitch_and_roll() {
    let root = crate::repository_paths::test_repo_root();
    let fx = load_fixture(&root.join(
        "tools_v2/developer-tools/test_fixtures/blueprint/rotation_pin_GarbageContainer_01.json",
    ))
    .unwrap();
    let scores = score_all(&fx);
    let winner = &scores[0];
    assert_eq!(
        winner.hypothesis,
        RIGID_HYPOTHESIS,
        "winner {} (rel {:.4} m, yaw err {:.4}°)",
        winner.hypothesis.name(),
        winner.rel_err_m,
        winner.yaw_err_deg
    );
    assert!(
        winner.rel_err_m < 0.005,
        "rel err {:.4} m",
        winner.rel_err_m
    );
    assert!(
        winner.yaw_err_deg < 0.05,
        "yaw err {:.4}°",
        winner.yaw_err_deg
    );
    let runner = &scores[1];
    assert!(
        runner.total() > 4.0 * winner.total().max(0.002),
        "no clear margin: {} total {:.5} vs {} total {:.5}",
        winner.hypothesis.name(),
        winner.total(),
        runner.hypothesis.name(),
        runner.total()
    );
}
