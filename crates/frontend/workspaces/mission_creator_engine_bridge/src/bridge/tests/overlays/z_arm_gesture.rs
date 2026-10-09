//! The Z-arm drag: mixed-kind elevation commits share one undo group, a foreign pointer
//! cannot steal the arm, the snap rung quantises, and a degenerate camera never reaches the
//! document.

use crate::bridge::overlays::z_drag_elevation_delta;

fn mixed_doc() -> mission_document::MissionDocCore {
    let core = mission_document::MissionDocCore::new();
    core.set_origin_init(true);
    core.add_slot(
        "roof",
        "sq",
        "layer",
        0,
        "Rifleman",
        None,
        None,
        100.0,
        200.0,
        50.123456789,
        30.0,
    );
    core.add_slot(
        "ground", "sq", "layer", 1, "Rifleman", None, None, 110.0, 210.0, -7.25, 45.0,
    );
    core.add_vehicle(
        "vehicle",
        "Vehicle.et",
        Some(150.0),
        Some(250.0),
        Some(81.5),
        Some(90.0),
    );
    core.set_origin_init(false);
    core
}

#[test]
fn authored_mixed_elevations_commit_and_undo_together() {
    use super::ZDrag;
    let mut core = mixed_doc();
    let slots_before: serde_json::Value = serde_json::from_str(&core.slots_json()).unwrap();
    let maps_before: serde_json::Value = serde_json::from_str(&core.small_maps_json()).unwrap();
    let ids = ["roof".into(), "ground".into(), "vehicle".into()];
    let arm = ZDrag::begin(&core, &ids, 7, 100.0, 2.0).unwrap();
    assert_eq!(arm.height(2.0), 52.123456789);
    assert_eq!(core.undo_depth(), 0, "preview must not mutate");
    assert!(arm.commit(&mut core, 2.0));
    let slots: serde_json::Value = serde_json::from_str(&core.slots_json()).unwrap();
    let maps: serde_json::Value = serde_json::from_str(&core.small_maps_json()).unwrap();
    assert_eq!(slots["roof"]["position"]["z"], 52.123456789);
    assert_eq!(slots["ground"]["position"]["z"], -5.25);
    assert_eq!(
        maps["vehiclesById"]["vehicle"]["position"],
        serde_json::json!({"x":150,"y":250,"z":83.5,"rotation":90})
    );
    assert_eq!(core.undo_depth(), 1, "all kinds must share one undo group");
    assert!(core.undo());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&core.slots_json()).unwrap(),
        slots_before
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).unwrap(),
        maps_before
    );
}

#[test]
fn cancelling_then_unrelated_pointerup_cannot_commit() {
    use super::{ZDrag, take_z_drag};
    let mut core = mixed_doc();
    let before: serde_json::Value = serde_json::from_str(&core.slots_json()).unwrap();
    let mut active = ZDrag::begin(&core, &["roof".into()], 7, 100.0, 2.0);
    assert!(
        take_z_drag(&mut active, 8).is_none(),
        "unrelated release cannot steal the arm"
    );
    assert!(active.is_some());
    assert!(
        take_z_drag(&mut active, 7).is_some(),
        "pointercancel consumes initiating arm"
    );
    for pointer in [8, 7] {
        if let Some(arm) = take_z_drag(&mut active, pointer) {
            arm.commit(&mut core, 2.0);
        }
    }
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&core.slots_json()).unwrap(),
        before
    );
    assert_eq!(core.undo_depth(), 0);
}

#[test]
fn vehicle_only_drag_uses_same_snap_and_shift_suspension() {
    use super::{ZDrag, z_drag_snap_step};
    let mut core = mixed_doc();
    let snap = super::transform::SnapState {
        enabled: true,
        translate_rung: 2,
        rotate_rung: 0,
    };
    let snapped = z_drag_snap_step(snap, false);
    let suspended = z_drag_snap_step(snap, true);
    assert_eq!(snapped, 5.0);
    assert_eq!(suspended, 0.0);
    let arm = ZDrag::begin(&core, &["vehicle".into()], 7, 100.0, 2.0).unwrap();
    let delta = z_drag_elevation_delta(96.0, 100.0, 2.0, suspended);
    assert_eq!(arm.height(delta), 83.5);
    assert!(arm.commit(&mut core, delta));
    assert_eq!(core.undo_depth(), 1);
    assert!(core.undo());
    let arm = ZDrag::begin(&core, &["vehicle".into()], 7, 100.0, 2.0).unwrap();
    assert!(!arm.commit(&mut core, z_drag_elevation_delta(96.0, 100.0, 2.0, snapped)));
    assert_eq!(
        core.undo_depth(),
        0,
        "zero snapped travel must not create an undo step"
    );
}

/// Up is +Z, and the ladder quantises. `dy_to_elevation` inverts the screen axis (`-dy/scale`)
/// and `snap_elevation` rounds to the rung; step `0.0` is passthrough.
#[test]
fn dragging_up_raises_and_the_rung_quantises() {
    // 20 px up at 2 px/m = 10 m up, unsnapped.
    assert_eq!(z_drag_elevation_delta(80.0, 100.0, 2.0, 0.0), 10.0);
    // Down is negative.
    assert_eq!(z_drag_elevation_delta(120.0, 100.0, 2.0, 0.0), -10.0);
    // 11 m of travel on the 5 m rung rounds to 10.
    assert_eq!(z_drag_elevation_delta(78.0, 100.0, 2.0, 5.0), 10.0);
    // No travel is no change, on any rung.
    assert_eq!(z_drag_elevation_delta(100.0, 100.0, 2.0, 5.0), 0.0);
}

/// A degenerate camera must not reach the document. `-dy / 0.0` is infinite, and an infinite
/// elevation lands as a JSON `null` the schema rejects at SAVE time — a long way from the drag
/// that caused it, which is the kind of distance that makes a defect expensive.
#[test]
fn a_degenerate_camera_scale_cannot_produce_an_infinite_elevation() {
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let d = z_drag_elevation_delta(80.0, 100.0, scale, 0.0);
        assert!(
            d.is_finite(),
            "scale {scale} produced a non-finite elevation ({d})"
        );
    }
}
