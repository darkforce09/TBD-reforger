//! Role: goldens for the combined verdict language, the styling map, and the blocking marker.
//! Position: `editing/tools/line_of_sight/tests` in the map engine.
//! Signals & state: explicit verdict pairs built in the test body.
//! Invariants: the readout names the NEARER blocker, and an unjudged object layer never reads clear.

use super::super::object_wash::*;
use super::super::projection::{ProjectedShot, world_key};
use super::super::terrain_verdict::LosVerdict;
use super::super::wash_palette::{
    VIEWSHED_HIDDEN_RGBA, VIEWSHED_UNKNOWN_RGBA, VIEWSHED_VISIBLE_RGBA,
};
use super::*;
use crate::spatial::los::terrain::viewshed::Visibility;
use crate::spatial::los::world::map_to_engine;

fn blocked(d: f64) -> LosVerdict {
    LosVerdict::Blocked {
        blocking_dist_m: d,
        blocking_elev_m: 150.0,
    }
}

fn obj_blocked(d: f64) -> ObjectVerdict {
    ObjectVerdict::Blocked {
        dist_m: d,
        label: "FarmHouse_E_1L01_Wood".into(),
        kind: "building".into(),
    }
}

#[test]
fn combined_header_names_the_nearer_blocker_and_the_object_layer_state() {
    let clear = ObjectVerdict::Clear {
        concealment: 0.0,
        glass_panes: 0,
    };
    assert_eq!(
        format_combined(&combine(LosVerdict::Clear, clear.clone()), 640.0),
        "LoS clear · 640 m"
    );
    assert_eq!(
        format_combined(
            &combine(
                LosVerdict::Clear,
                ObjectVerdict::Clear {
                    concealment: 0.43,
                    glass_panes: 0
                }
            ),
            640.0
        ),
        "LoS clear · 640 m · canopy 43 %"
    );
    assert_eq!(
        format_combined(
            &combine(LosVerdict::Clear, ObjectVerdict::NotLoaded),
            1240.0
        ),
        "LoS clear · 1.24 km · objects not loaded"
    );
    assert_eq!(
        format_combined(&combine(LosVerdict::Clear, obj_blocked(96.4)), 640.0),
        "LoS blocked at 96 m — FarmHouse_E_1L01_Wood (building)"
    );
    assert_eq!(
        format_combined(&combine(blocked(412.0), clear.clone()), 640.0),
        "LoS blocked at 412 m — terrain"
    );
    assert_eq!(
        format_combined(&combine(blocked(412.0), ObjectVerdict::NotLoaded), 640.0),
        "LoS blocked at 412 m — terrain"
    );
    // Both block: the nearer one names the header.
    assert_eq!(
        format_combined(&combine(blocked(412.0), obj_blocked(96.0)), 640.0),
        "LoS blocked at 96 m — FarmHouse_E_1L01_Wood (building)"
    );
    assert_eq!(
        format_combined(&combine(blocked(50.0), obj_blocked(96.0)), 640.0),
        "LoS blocked at 50 m — terrain"
    );
    assert_eq!(
        format_combined(
            &combine(
                LosVerdict::Clear,
                ObjectVerdict::Provisional {
                    dist_m: 96.0,
                    label: "Barn_01".into()
                }
            ),
            640.0
        ),
        "LoS provisional at 96 m — Barn_01 (geometry loading)"
    );
    assert_eq!(
        format_combined(&combine(LosVerdict::Unknown, obj_blocked(1.0)), 640.0),
        "LoS —"
    );
}

#[test]
fn first_block_and_styling_follow_the_nearest_block() {
    assert_eq!(
        first_block_dist(&combine(LosVerdict::Clear, ObjectVerdict::NotLoaded)),
        None
    );
    assert_eq!(
        first_block_dist(&combine(blocked(412.0), obj_blocked(96.0))),
        Some(96.0)
    );
    assert_eq!(
        first_block_dist(&combine(blocked(50.0), obj_blocked(96.0))),
        Some(50.0)
    );
    assert_eq!(
        first_block_dist(&combine(
            LosVerdict::Clear,
            ObjectVerdict::Provisional {
                dist_m: 7.0,
                label: "x".into()
            }
        )),
        Some(7.0)
    );
    assert!(styling_verdict(&combine(LosVerdict::Clear, obj_blocked(96.0))).is_blocked());
    assert!(styling_verdict(&combine(LosVerdict::Clear, ObjectVerdict::NotLoaded)).is_clear());
    assert_eq!(
        styling_verdict(&combine(LosVerdict::Unknown, obj_blocked(1.0))),
        LosVerdict::Unknown
    );
    // The object block keeps the terrain's elevation when the terrain also blocks (chart marker).
    assert_eq!(
        styling_verdict(&combine(blocked(412.0), obj_blocked(96.0))),
        LosVerdict::Blocked {
            blocking_dist_m: 96.0,
            blocking_elev_m: 150.0
        }
    );
}

#[test]
fn apply_objects_moves_the_marker_to_the_nearest_block() {
    let mut shot = ProjectedShot {
        obs_px: 100.0,
        obs_py: 100.0,
        tgt_px: 300.0,
        tgt_py: 100.0,
        verdict: LosVerdict::Clear,
        total_m: 400.0,
        block_px: None,
        objects: ObjectVerdict::NotLoaded,
        key: world_key(0.0, 0.0, 400.0, 0.0),
    };
    apply_objects(&mut shot, obj_blocked(100.0));
    assert_eq!(shot.block_px, Some((150.0, 100.0)));
    assert!(styling_of(&shot).is_blocked());
    shot.verdict = blocked(40.0);
    apply_objects(&mut shot, obj_blocked(100.0));
    assert_eq!(
        shot.block_px,
        Some((120.0, 100.0)),
        "the terrain block at 40 m is nearer"
    );
    apply_objects(&mut shot, ObjectVerdict::NotLoaded);
    assert_eq!(shot.block_px, Some((120.0, 100.0)));
    shot.verdict = LosVerdict::Clear;
    apply_objects(
        &mut shot,
        ObjectVerdict::Clear {
            concealment: 0.1,
            glass_panes: 0,
        },
    );
    assert_eq!(shot.block_px, None);
}

#[test]
fn merged_palette_is_terrain_first_then_the_object_verdict() {
    assert_eq!(
        object_cell_rgba(Visibility::Hidden, ObjectCell::Clear),
        VIEWSHED_HIDDEN_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Unknown, ObjectCell::Hidden),
        VIEWSHED_UNKNOWN_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Untested),
        VIEWSHED_VISIBLE_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Clear),
        VIEWSHED_VISIBLE_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Hidden),
        OBJECT_HIDDEN_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Provisional),
        OBJECT_PROVISIONAL_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Concealed(0)),
        [34, 84, 36, 30]
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Concealed(255)),
        [34, 84, 36, 90]
    );
    // Pinned constants: the object wash must stay visually distinct from the terrain wash.
    assert_ne!(OBJECT_HIDDEN_RGBA, VIEWSHED_HIDDEN_RGBA);
    assert_eq!(OBJECT_HIDDEN_RGBA, [78, 30, 24, 110]);
    assert_eq!(OBJECT_PROVISIONAL_RGBA, [128, 92, 20, 84]);
    assert_eq!(OBJECT_LEVELS, [4, 2, 1]);
    assert_eq!(OBJECT_PASS_BUDGET_MS, 8.0);
    assert_eq!(OBJECT_FINE_RADIUS_M, 1000.0);
    assert_eq!(map_to_engine(1.0, 2.0, 3.0), [1.0, 3.0, 2.0]);
}
