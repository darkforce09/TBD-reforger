//! Role: glyph math tests.
//! Position: `symbology/labels/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::symbology::labels::glyph_math::*;

#[test]
fn deck_angle_handedness() {
    assert_eq!(deck_angle_for_rotation_deg(0.0), 0.0);
    assert_eq!(deck_angle_for_rotation_deg(90.0), -90.0);
    assert_eq!(deck_angle_for_rotation_deg(180.0), -180.0);
    assert!(!deck_angle_for_rotation_deg(0.0).is_sign_negative());
    assert_eq!(deck_angle_for_rotation_deg(f64::NAN), 0.0);
}

#[test]
fn tree_size_multiplier_clamps() {
    assert!((tree_size_multiplier(None) - 1.0).abs() < 1e-12);
    assert!((tree_size_multiplier(Some(10.0)) - 1.0).abs() < 1e-12);
    assert!((tree_size_multiplier(Some(5.0)) - 1.0).abs() < 1e-12);
    assert!((tree_size_multiplier(Some(12.5)) - 1.25).abs() < 1e-12);
    assert!((tree_size_multiplier(Some(20.0)) - 1.5).abs() < 1e-12);
    assert!((tree_size_multiplier(Some(100.0)) - 1.5).abs() < 1e-12);
}

#[test]
fn glyph_size_meters_formula() {
    let expect = 24.0 / 2.0_f64.powf(REF_ZOOM);
    assert!((glyph_size_meters(24.0, Some(10.0)) - expect).abs() < 1e-9);
    let expect15 = (24.0 * 1.5) / 2.0_f64.powf(REF_ZOOM);
    assert!((glyph_size_meters(24.0, Some(20.0)) - expect15).abs() < 1e-9);
}

#[test]
fn hex_to_rgba_parses() {
    assert_eq!(hex_to_rgba(Some("#2d5a27")), [45, 90, 39, 255]);
    assert_eq!(hex_to_rgba(Some("4a7a32")), [74, 122, 50, 255]);
    assert_eq!(hex_to_rgba(Some("#abc")), [170, 187, 204, 255]);
    assert_eq!(hex_to_rgba(None), DEFAULT_GLYPH_RGBA);
    assert_eq!(hex_to_rgba(Some("nothex")), DEFAULT_GLYPH_RGBA);
}

#[test]
fn pack_icon_instance_is_20_bytes() {
    let mut v = Vec::new();
    pack_icon_instance(
        &mut v,
        1.5,
        -2.5,
        3.0,
        -90.0,
        7,
        pack_rgba_u32([45, 90, 39, 255]),
    );
    assert_eq!(v.len(), ICON_INSTANCE_STRIDE);
    assert_eq!(ICON_INSTANCE_STRIDE, 20);

    assert_eq!(f32::from_le_bytes(v[0..4].try_into().unwrap()), 1.5);
    assert_eq!(f32::from_le_bytes(v[4..8].try_into().unwrap()), -2.5);
    assert_eq!(f32::from_le_bytes(v[8..12].try_into().unwrap()), 3.0);
    let yaw = i16::from_le_bytes(v[12..14].try_into().unwrap());
    assert_eq!(yaw, yaw_to_snorm16(-90.0));
    assert_eq!(u16::from_le_bytes(v[14..16].try_into().unwrap()), 7);
}

#[test]
fn badge_keys() {
    assert_eq!(badge_icon_key("military"), Some("building-badge-military"));
    assert_eq!(badge_icon_key("residential"), None);
}

#[test]
fn building_icon_key_covers_all_building_classes() {
    for &cls in BUILDING_CLASSES {
        let key = building_icon_key(cls).expect(cls);
        assert_eq!(key, format!("building-{cls}"));
    }
    assert_eq!(building_icon_key("pier"), None);
}

#[test]
fn landmark_glyph_prefers_badge_overlay() {
    assert_eq!(
        landmark_glyph_icon_key("military"),
        Some("building-badge-military")
    );
    assert_eq!(
        landmark_glyph_icon_key("lighthouse"),
        Some("building-lighthouse")
    );
    assert_eq!(landmark_glyph_icon_key("castle"), Some("building-castle"));
}

#[test]
fn yaw_snorm16_wraps_not_clamps() {
    assert_eq!(yaw_to_snorm16(-270.0), yaw_to_snorm16(90.0));
    assert_eq!(yaw_to_snorm16(180.0), 32767);
    assert_eq!(
        yaw_to_snorm16(-180.0),
        32767,
        "-180 folds up into (-180, 180]"
    );
    assert_eq!(yaw_to_snorm16(540.0), yaw_to_snorm16(180.0));
    assert_eq!(yaw_to_snorm16(-359.0), yaw_to_snorm16(1.0));
}

#[test]
fn world_rotation_270_differs_from_180() {
    let encode = |rotation: f64| yaw_to_snorm16(deck_angle_for_rotation_deg(rotation));
    let drawn_bearing = |rotation: f64| {
        let snorm = encode(rotation);
        let screen_deg = f64::from(snorm) / 32767.0 * 180.0;
        (-screen_deg).rem_euclid(360.0)
    };
    assert_ne!(
        encode(270.0),
        encode(180.0),
        "270 and 180 share encoding — western half collapsed onto due south"
    );
    assert_eq!(
        encode(270.0),
        encode(-90.0),
        "270 IS -90 after wrap, not the clamp floor"
    );
    let b180 = drawn_bearing(180.0);
    let b270 = drawn_bearing(270.0);
    let delta = (b270 - b180).rem_euclid(360.0);
    assert!(
        (delta - 90.0).abs() < 0.05,
        "270 tip/extent bearing {b270} must differ from 180's {b180} by ~90°, got delta {delta}"
    );
}

#[test]
fn every_world_rotation_of_the_compass_gets_its_own_facing() {
    let drawn_bearing = |rotation: f64| {
        let snorm = yaw_to_snorm16(deck_angle_for_rotation_deg(rotation));
        let screen_deg = f64::from(snorm) / 32767.0 * 180.0;
        (-screen_deg).rem_euclid(360.0)
    };
    let encode = |rotation: f64| yaw_to_snorm16(deck_angle_for_rotation_deg(rotation));

    let cardinals = [encode(0.0), encode(90.0), encode(180.0), encode(270.0)];
    for (i, a) in cardinals.iter().enumerate() {
        for (j, b) in cardinals.iter().enumerate() {
            assert!(
                i == j || a != b,
                "cardinals {i} and {j} share encoding {a} — the compass has collapsed"
            );
        }
    }

    let mut prev = drawn_bearing(0.0);
    for h in 1..=360 {
        let cur = drawn_bearing(f64::from(h));
        let step = (cur - prev).rem_euclid(360.0);
        assert!(
            step > 0.0,
            "rotation {h} did not move the glyph — plateau at bearing {prev} (this is T-842)"
        );
        assert!(
            step < 2.0,
            "rotation {h} jumped {step}° — the fold is not continuous"
        );
        assert!(
            (cur - f64::from(h % 360)).abs() < 0.02 || (cur - f64::from(h % 360)).abs() > 359.9,
            "rotation {h} drew bearing {cur}"
        );
        prev = cur;
    }
}
