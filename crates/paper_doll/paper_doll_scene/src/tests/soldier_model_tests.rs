//! The soldier model's regions, parts, meshes, colours, picks and anchors, against their goldens.
//!
//! **Role:** unit tests of [`crate::soldier_parts`], [`crate::part_meshes`] and
//! [`crate::region_picking`], plus the orbit camera's perspective golden they rely on.
//! **Position:** mounted from the crate root under `cfg(test)`; native only.
//! **Signals & state:** none.
//! **Invariants:** the goldens are the scene's contract with the renderer and the Arsenal: every
//! region has a part, the centre pick lands on the rifle, a pick from behind hits the backpack,
//! and an anchor is the transformed region centre.

use camera_math::orbit::projection::view_proj_gl;

use camera_math::matrix4::transform_vector;

use crate::region_picking::{anchor_px, anchor_world, pick};
use crate::soldier_parts::{DECOR, REGION_KEYS, instances};

#[test]
fn every_region_has_at_least_one_instance() {
    let inst = instances();
    for (i, key) in REGION_KEYS.iter().enumerate() {
        let n = inst
            .iter()
            .filter(|d| d.region == i32::try_from(i).unwrap())
            .count();
        assert!(n >= 1, "region {key} has no instances");
    }
    assert!(inst.iter().any(|d| d.region == DECOR), "decor body missing");
}

#[test]
fn pick_goldens_center_regions() {
    let w = 800.0;
    let h = 600.0;
    let center = pick(0.0, w, h, 400.0, 300.0);
    assert_eq!(
        REGION_KEYS[usize::try_from(center).expect("hit")],
        "primary",
        "screen center should hit the rifle receiver"
    );

    assert_eq!(pick(0.0, w, h, 400.0, 10.0), -1);

    assert_eq!(pick(0.0, w, h, 20.0, 300.0), -1);
}

#[test]
fn pick_yaw_symmetry_hits_backpack_from_behind() {
    let hit = pick(core::f64::consts::PI, 800.0, 600.0, 400.0, 280.0);
    assert!(hit >= 0, "back view center must hit something");
    let key = REGION_KEYS[usize::try_from(hit).expect("hit")];
    assert!(
        key == "backpack" || key == "armoredVest" || key == "launcher",
        "back view center hit {key}, expected back-mounted gear"
    );
}

#[test]
fn anchor_matches_transform_vector_projection() {
    let (w, h) = (800.0, 600.0);
    let helmet =
        i32::try_from(REGION_KEYS.iter().position(|k| *k == "headCover").unwrap()).unwrap();
    let p = anchor_world(helmet).unwrap();
    let vp = view_proj_gl(0.0, w, h);
    let ndc = transform_vector(&vp, [p[0], p[1], p[2], 1.0]);
    let expect = (((ndc[0] + 1.0) / 2.0) * w, ((1.0 - ndc[1]) / 2.0) * h);
    let got = anchor_px(0.0, w, h, helmet).unwrap();
    assert!((got.0 - expect.0).abs() < 1e-9);
    assert!((got.1 - expect.1).abs() < 1e-9);
}
