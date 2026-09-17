use crate::v2::apps::editor::bridge::overlays::z_drag_elevation_delta;
use crate::v2::core::test_support::class_r_scrub::live_code;

/// The GESTURE file's LIVE source — comments stripped, string/char literals blanked, test modules
/// (including this one) cut. Every needle below is therefore a real call in shipped code, not
/// a reassuring note about one, which is the whole failure mode T-946.86 exists to repair:
/// wave 255 shipped `z_drag` written-and-never-read with a doc block describing the wiring.
fn live() -> String {
    live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/input/pointer_gestures.rs"
    )))
}

/// **The arm is READ, not merely written.** The wave-255 defect verbatim: `z_drag` had exactly
/// four occurrences — the declaration, two closure clones, and the pointerdown write — and
/// neither closure body ever borrowed it, so the Z arm armed and then did nothing at all.
///
/// PERTURB: delete either borrow and this goes RED.
#[test]
fn the_z_arm_is_borrowed_by_both_pointer_closures() {
    let src = live();
    assert!(
        src.contains("z_drag.borrow()"),
        "T-946.86 (.82): onpointermove must BORROW the armed z_drag — writing it at \
         pointerdown and never reading it is the wave-255 defect this repairs"
    );
    assert!(
        src.split("let onpointerup")
            .nth(1)
            .is_some_and(|s| s.contains("ov::take_z_drag(")),
        "T-946.86 (.82): onpointerup must TAKE the arm — leaving it latched strands the next \
         gesture behind a drag that already ended"
    );
}

/// **The readout writer is called.** `set_z_drag_readout` shipped with zero callers while its
/// reader was already wired into the gizmo chip, so the chip rendered and could never be
/// populated. Both halves are pinned: the call, and the tracking closure at the render site
/// (a bare expression there would run once and never re-read the value the drag writes).
#[test]
fn the_height_chip_is_written_and_its_render_site_tracks() {
    let src = live();
    assert!(
        src.contains("set_z_drag_readout(Some("),
        "T-946.86 (.82): the drag must publish the height readout — a reader with no writer \
         is a chip that can never populate"
    );
    assert!(
        src.contains("set_z_drag_readout(None)"),
        "T-946.86 (.82): the release must clear the readout, or the chip keeps the last \
         drag's height after the gesture is over"
    );
    let overlays = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/bridge/overlays.rs"
    )));
    assert!(
        overlays.contains("{move || {") && overlays.contains("read_z_drag_readout()"),
        "T-946.86 (.82): the chip's render site must be a TRACKING closure — as a bare \
         expression it is evaluated once, when the value is still None"
    );
}

/// **The capture this arm takes is the capture this arm releases.** `onpointerdown` calls
/// `set_pointer_capture` on the Z arm and none of the file's other releases belonged to it, so
/// the container held the pointer after the drag ended and every later click was retargeted.
///
/// The release is pinned as sitting BEFORE the commit: a release that only runs on the
/// success path strands the capture on every no-travel click of the arm.
#[test]
fn the_z_arm_releases_the_pointer_capture_before_it_commits() {
    let src = live();
    let src = src
        .split("let onpointerup")
        .nth(1)
        .expect("pointerup closure");
    let at_take = src
        .find("ov::take_z_drag(")
        .expect("the pointerup arm is present");
    let after = &src[at_take..];
    let at_release = after
        .find("release_pointer_capture")
        .expect("T-946.86 (.82): the Z-arm release must release the capture it took");
    let at_commit = after
        .find("arm.commit(core, delta)")
        .expect("T-946.86 (.82): the Z-arm release must commit the elevation");
    assert!(
        at_release < at_commit,
        "T-946.86 (.82): release the capture BEFORE the commit — a release reached only when \
         the document accepts the edit strands the pointer on every no-travel click"
    );
}

fn mixed_doc() -> website_map_engine::data::store::MissionDocCore {
    let core = website_map_engine::data::store::MissionDocCore::new();
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
    use super::{take_z_drag, ZDrag};
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
    use super::{z_drag_snap_step, ZDrag};
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

/// The preview and the commit must resolve the SAME number from the same inputs. Two copies of
/// "pixels to metres" would show the operator one height and store another; both call sites
/// therefore go through `z_drag_elevation_delta`, and there are exactly two of them.
#[test]
fn the_preview_and_the_commit_share_one_arithmetic() {
    let src = live();
    let calls = src.matches("z_drag_elevation_delta(").count();
    assert_eq!(
        calls, 2,
        "T-946.86 (.82): the gesture file must carry exactly two call sites — the preview and \
         the commit — found {calls}. A third would be a second vocabulary; one means a call \
         site was lost, and the preview would then show a height the commit does not store"
    );
    assert_eq!(
        src.matches("ov::z_drag_snap_step(").count(),
        2,
        "T-946.86 (.82): both call sites must resolve the snap rung the same way"
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
            "T-946.86 (.82): scale {scale} produced a non-finite elevation ({d})"
        );
    }
}
