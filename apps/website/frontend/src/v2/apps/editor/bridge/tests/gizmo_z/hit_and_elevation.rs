use super::*;

#[test]
fn test_hit_z_arm() {
    assert!(
        hit_z_arm(100.0, 50.0, 100.0, 100.0, 1.0),
        "Z arm above gizmo center returns None or fails"
    );
    assert!(!hit_z_arm(100.0, 110.0, 100.0, 100.0, 1.0)); // below center
    assert!(!hit_z_arm(120.0, 50.0, 100.0, 100.0, 1.0)); // too far right
    assert!(!hit_z_arm(100.0, 100.0, 100.0, 100.0, 1.0)); // XY center
    assert!(!hit_z_arm(100.0, 86.0, 100.0, 100.0, 1.0)); // center boundary
    assert!(hit_z_arm(100.0, 85.0, 100.0, 100.0, 1.0)); // exposed shaft
}

#[test]
fn test_dy_to_elevation() {
    let elev = dy_to_elevation(-10.0, 2.0);
    assert_eq!(elev, 5.0, "dy to elevation must go RED if inverted");
}
