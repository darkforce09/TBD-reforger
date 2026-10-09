//! Role: the Everon glyph and strip cases: atlas coverage, the building icon keys, the landmark
//! zoom gate and strip orientation.
//! Position: `chunk_draw_buffers::tests::everon_glyphs_and_strips`; uses the fixtures and oracles of
//! its `mod.rs`.
//! Signals & state: none; every case builds its own residency.
//! Invariants: each case compares the composed buffers with the export.

use prefab_catalog::footprint_lookups::obb_corners;

use super::*;

#[test]
fn glyph_atlas_covers_every_requested_key_and_sources_agree() {
    let atlas = world_glyphs_atlas_keys();

    let manifest: HashSet<String> = glyph_keys_from_manifest().into_iter().collect();
    assert_eq!(
        atlas, manifest,
        "world-glyphs.json vs manifest.json glyph keys diverged"
    );

    let raw = prefab_catalog::world_payload::bytes_to_json(
        &fs::read(map_assets().join("everon/objects/prefabs.json.gz")).unwrap(),
    )
    .unwrap();
    for row in narrow_prefab_rows(&raw).expect("Everon ids are whole u32s") {
        if let Some(k) = row.icon_key.as_deref() {
            assert!(atlas.contains(k), "prefab iconKey '{k}' missing from atlas");
        }
    }

    for &cls in BUILDING_CLASSES {
        for key in [building_icon_key(cls), badge_icon_key(cls)]
            .into_iter()
            .flatten()
        {
            assert!(
                atlas.contains(key),
                "classify key '{key}' (class '{cls}') missing from atlas"
            );
        }
    }
}

#[test]
fn g3_zoom_gate_below_one_only_importance_landmarks() {
    let mut r = load_everon_residency();
    let keys: HashSet<String> = glyph_keys_from_manifest().into_iter().collect();
    let prefab_class = building_class_by_prefab_u16();
    let prefab_importance = building_importance_by_prefab_u16();
    drive_fixture_chunk(&mut r, FIXTURE_CHUNK, 0.9);
    let rust = r.badge_glyph_count() as usize;
    let oracle = oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, 0.9);
    assert_eq!(
        rust, oracle,
        "@ z=0.9 Rust badge count must equal the importance-aware oracle"
    );

    assert!(
        rust > 0,
        "importanceZoom landmarks must emit below the badge band"
    );
}

#[test]
fn g1_building_icon_key_covers_normative_classes() {
    use label_layout::glyph_math::building_icon_key;
    for &cls in BUILDING_CLASSES {
        let key = building_icon_key(cls).expect(cls);
        assert_eq!(key, format!("building-{cls}"));
    }
}

#[test]
fn t152_15_g2_orientation_parity_all_prefabs() {
    let everon = map_assets().join("everon");
    let raw = prefab_catalog::world_payload::bytes_to_json(
        &fs::read(everon.join("objects/prefabs.json.gz")).unwrap(),
    )
    .unwrap();
    let fences = fence_prefab_lookup(&raw);
    let buildings = building_prefab_lookup(&raw);
    let mut samples: Vec<(f64, f64)> = fences.values().map(|f| (f.half_x, f.half_y)).collect();
    let fence_n = samples.len();
    for info in buildings.values() {
        if info.building_class == "pier" || info.building_class == "dock" {
            samples.push((info.half_x, info.half_y));
        }
    }
    let mut checked = 0usize;
    let mut worst = 0.0f64;
    for (hx, hy) in &samples {
        for yaw in [0.0f64, 37.0, 90.0, 123.0] {
            let [p0, p1] =
                road_network::cartographic_strip::obb_long_axis_endpoints(0.0, 0.0, *hx, *hy, yaw);
            let strip_ang = (p1[1] - p0[1]).atan2(p1[0] - p0[0]).to_degrees();
            let c = obb_corners(0.0, 0.0, *hx, *hy, yaw);
            let (e0x, e0y) = (c[1][0] - c[0][0], c[1][1] - c[0][1]);
            let (e1x, e1y) = (c[2][0] - c[1][0], c[2][1] - c[1][1]);
            let (fx, fy) = if e0x.hypot(e0y) >= e1x.hypot(e1y) {
                (e0x, e0y)
            } else {
                (e1x, e1y)
            };
            let fill_ang = fy.atan2(fx).to_degrees();
            let d = {
                let x = (strip_ang - fill_ang).abs() % 180.0;
                x.min(180.0 - x)
            };
            assert!(d <= 0.5, "parity {d}° for hx={hx} hy={hy} yaw={yaw}");
            worst = worst.max(d);
            checked += 1;
        }
    }
    assert!(fence_n >= 255, "expected ≥255 fence prefabs, got {fence_n}");
    assert!(
        checked >= 255 * 4,
        "parity gate must be non-vacuous, only {checked} checks"
    );
    assert!(worst <= 0.5, "worst-case parity {worst}° exceeds 0.5°");
}
