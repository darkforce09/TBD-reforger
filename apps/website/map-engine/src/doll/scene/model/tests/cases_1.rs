//! Role: Domain regression cases.
//! Position: `doll/scene/model/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::math::glmat4::perspective_no;

use crate::camera::math::glmat4::transform_vector;

use super::*;

#[test]
fn region_keys_count_and_uniqueness() {
    assert_eq!(REGION_KEYS.len(), 14);
    let mut sorted = REGION_KEYS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 14, "duplicate region key");
}

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
fn mesh_counts_exact() {
    let (cv, ci) = mesh_cube();
    assert_eq!(cv.len(), 24 * 6);
    assert_eq!(ci.len(), 36);
    let (yv, yi) = mesh_cylinder(16);
    assert_eq!(yv.len(), (4 * 16 + 2 * 17) * 6);
    assert_eq!(yi.len(), 6 * 16 + 2 * 3 * 16);
}

#[test]
fn perspective_golden() {
    let m = perspective_no(0.6109, 1.5, 0.1, 100.0);
    let f = 1.0 / (0.6109_f64 / 2.0).tan();
    assert_eq!(m[0], f / 1.5);
    assert_eq!(m[5], f);
    assert_eq!(m[11], -1.0);
    let nf = 1.0 / (0.1 - 100.0);
    assert_eq!(m[10], (100.0 + 0.1) * nf);
    assert_eq!(m[14], 2.0 * 100.0 * 0.1 * nf);
    assert_eq!(m[15], 0.0);

    let inf = perspective_no(0.6109, 1.5, 0.1, f64::INFINITY);
    assert_eq!(inf[10], -1.0);
    assert_eq!(inf[14], -0.2);
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
fn state_colors_distinct_and_hover_lifts() {
    let e = state_color(STATE_EMPTY, false);
    let q = state_color(STATE_EQUIPPED, false);
    let a = state_color(STATE_ACTIVE, false);
    assert_ne!(e, q);
    assert_ne!(q, a);
    assert_ne!(e, a);
    for s in [STATE_EMPTY, STATE_EQUIPPED, STATE_ACTIVE] {
        let base = state_color(s, false);
        let hover = state_color(s, true);
        assert_ne!(base, hover, "hover must lift state {s}");
        assert_eq!(hover[3], 1.0);
        assert!(hover.iter().all(|c| *c <= 1.0));
    }
    for c in [e, q, a] {
        assert_eq!(c[3], 1.0, "opaque pipeline — no alpha");
    }
}

#[test]
fn anchors_every_region_projects_inside_the_viewport() {
    let (w, h) = (800.0, 600.0);
    for (i, key) in REGION_KEYS.iter().enumerate() {
        let idx = i32::try_from(i).unwrap();
        let (x, y) = anchor_px(0.0, w, h, idx).unwrap_or_else(|| panic!("{key} anchor"));
        assert!(x > 0.0 && x < w, "{key} x={x}");
        assert!(y > 0.0 && y < h, "{key} y={y}");
    }
    assert!(anchor_px(0.0, w, h, 99).is_none(), "unknown region hides");
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
