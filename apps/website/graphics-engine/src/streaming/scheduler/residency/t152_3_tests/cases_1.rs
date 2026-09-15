//! Role: Domain regression cases.
//! Position: `streaming/scheduler/residency/t152_3_tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::environment::buildings::footprint::BRIDGE_CASING_RGBA;

use crate::environment::buildings::footprint::BRIDGE_DECK_RGBA;

use crate::environment::buildings::obb::obb_corners;

use super::*;

#[test]
fn g2_building_glyph_lookup_populated() {
    let r = load_everon_residency();
    let n = r.glyph_lookup_len_for_group(2);
    assert!(
        n >= N_MIN_BUILDING_GLYPH_LOOKUP,
        "building glyph lookup {n} < N_min {N_MIN_BUILDING_GLYPH_LOOKUP}"
    );
}

#[test]
fn tree_glyphs_pack_from_real_everon_data() {
    let mut r = load_everon_residency();

    assert_eq!(
        r.glyph_lookup_len_for_group(0),
        84,
        "all real tree and vegetation prefabs must map to group-0 glyphs"
    );

    drive_fixture_chunk(&mut r, "16_2", 0.0);
    let packed = r.tree_glyph_count();
    assert!(packed > 0, "forest chunk must pack tree glyphs");
    assert_eq!(
        packed,
        r.exact_tree_count_draw(),
        "every visible tree instance must pack (no silent drops)"
    );
    assert_eq!(
        packed, 26000,
        "4 strict-draw chunks × 6500 replicated fixture trees"
    );
}

#[test]
fn glyph_atlas_covers_every_requested_key_and_sources_agree() {
    let atlas = world_glyphs_atlas_keys();

    let manifest: HashSet<String> = glyph_keys_from_manifest().into_iter().collect();
    assert_eq!(
        atlas, manifest,
        "world-glyphs.json vs manifest.json glyph keys diverged"
    );

    let raw = crate::streaming::loaders::store::bytes_to_json(
        &fs::read(map_assets().join("everon/objects/prefabs.json.gz")).unwrap(),
    )
    .unwrap();
    for row in narrow_prefab_rows(&raw) {
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
fn g4_class_r_badge_counts_match_oracle() {
    let mut r = load_everon_residency();
    let keys: HashSet<String> = glyph_keys_from_manifest().into_iter().collect();
    let prefab_class = building_class_by_prefab_u16();
    let prefab_importance = building_importance_by_prefab_u16();
    for z in [1.0, 2.0, 3.0] {
        drive_fixture_chunk(&mut r, FIXTURE_CHUNK, z);
        let rust_count = r.badge_glyph_count() as usize;
        let oracle = oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, z);
        assert_eq!(
            rust_count, oracle,
            "badge count mismatch @ z={z} chunk {FIXTURE_CHUNK}"
        );
    }
}

#[test]
fn g5_landmark_glyph_count_matches_oracle_at_z2() {
    let mut r = load_everon_residency();
    let keys: HashSet<String> = glyph_keys_from_manifest().into_iter().collect();
    let prefab_class = building_class_by_prefab_u16();
    let prefab_importance = building_importance_by_prefab_u16();
    drive_fixture_chunk(&mut r, FIXTURE_CHUNK, 2.0);
    let oracle_lm =
        oracle_landmark_glyph_count_for_chunk(&r, FIXTURE_CHUNK, &prefab_class, &keys, 2.0);
    assert!(
        oracle_lm > 0,
        "fixture chunk must include landmark buildings"
    );
    assert_eq!(
        oracle_lm, 11,
        "pinned fixture {FIXTURE_CHUNK} landmark oracle"
    );
    let lighthouse_idx = r.glyph_idx_for_key("building-lighthouse").unwrap();
    let castle_idx = r.glyph_idx_for_key("building-castle").unwrap();
    let bridge_idx = r.glyph_idx_for_key("building-bridge").unwrap();
    let rust_lm = badge_glyph_indices(&r.world_badge_glyphs())
        .iter()
        .filter(|idx| **idx == lighthouse_idx || **idx == castle_idx || **idx == bridge_idx)
        .count();
    assert!(
        rust_lm >= oracle_lm,
        "composed landmark glyphs {rust_lm} must cover fixture oracle {oracle_lm}"
    );
    assert_eq!(
        r.badge_glyph_count() as usize,
        oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, 2.0)
    );
}

#[test]
fn g6_lighthouse_instances_emit_building_lighthouse_glyph() {
    let mut r = load_everon_residency();
    let lighthouse_idx = r
        .glyph_idx_for_key("building-lighthouse")
        .expect("building-lighthouse in atlas");
    drive_fixture_chunk(&mut r, FIXTURE_CHUNK, 2.0);
    let indices = badge_glyph_indices(&r.world_badge_glyphs());
    assert!(
        indices.contains(&lighthouse_idx),
        "badge buffer must include building-lighthouse glyph @ z=2"
    );
    assert!(r.badge_glyph_count() > 0);
}

#[test]
fn t152_21_landmark_early_visibility() {
    let mut r = load_everon_residency();
    let keys: HashSet<String> = glyph_keys_from_manifest().into_iter().collect();
    let prefab_class = building_class_by_prefab_u16();
    let prefab_importance = building_importance_by_prefab_u16();
    let lighthouse_idx = r.glyph_idx_for_key("building-lighthouse").unwrap();

    drive_fixture_chunk(&mut r, FIXTURE_CHUNK, -2.0);
    let n_default = r.badge_glyph_count() as usize;
    assert!(
        n_default > 0,
        "landmarks must emit badges at default zoom −2"
    );
    assert_eq!(
        n_default,
        oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, -2.0),
        "z=−2 badge count must equal the importance-aware oracle"
    );
    assert!(
        badge_glyph_indices(&r.world_badge_glyphs()).contains(&lighthouse_idx),
        "building-lighthouse glyph present at z=−2"
    );

    drive_fixture_chunk(&mut r, FIXTURE_CHUNK, -4.0);
    assert!(
        r.badge_glyph_count() > 0,
        "landmarks visible at the importanceZoom boundary −4.0"
    );
    assert_eq!(
        r.badge_glyph_count() as usize,
        oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, -4.0)
    );
    drive_fixture_chunk(&mut r, FIXTURE_CHUNK, -4.1);
    assert_eq!(
        r.badge_glyph_count(),
        0,
        "no badges past the −4 override boundary"
    );
    assert_eq!(
        oracle_badge_count(&r, &prefab_class, &prefab_importance, &keys, -4.1),
        0
    );
}

#[test]
fn t152_21_fill_deemphasis_handoff() {
    let mut r = load_everon_residency();
    let white = 235.0_f32 / 255.0;

    let has_white_fill = |buf: &[f32]| -> bool {
        buf.chunks_exact(10).any(|c| {
            (c[6] - white).abs() < 1e-3
                && (c[7] - white).abs() < 1e-3
                && (c[8] - white).abs() < 1e-3
        })
    };

    drive_fixture_chunk(&mut r, FIXTURE_CHUNK, -2.0);
    assert!(
        !has_white_fill(&r.world_building_fill()),
        "lighthouse white fill must be de-emphasized at z=−2 (glyph is the face)"
    );

    drive_fixture_chunk(&mut r, FIXTURE_CHUNK, 2.0);
    assert!(
        has_white_fill(&r.world_building_fill()),
        "lighthouse bright fill present at z≥1 (no de-emphasis above the badge band)"
    );
}

#[test]
fn g1_building_icon_key_covers_normative_classes() {
    use crate::symbology::labels::glyph_math::building_icon_key;
    for &cls in BUILDING_CLASSES {
        let key = building_icon_key(cls).expect(cls);
        assert_eq!(key, format!("building-{cls}"));
    }
}

#[test]
fn t152_15_g2_orientation_parity_all_prefabs() {
    let everon = map_assets().join("everon");
    let raw = crate::streaming::loaders::store::bytes_to_json(
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
            let [p0, p1] = crate::terrain::roads::cartographic_strip::obb_long_axis_endpoints(
                0.0, 0.0, *hx, *hy, yaw,
            );
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

#[test]
fn t152_15_pier_census_rails_casing_and_decoupling() {
    let mut r = load_everon_residency();
    drive_full_island(&mut r, 1.5);

    let piers = r.pier_strip_segment_count();
    assert!(piers > 0, "G3 anti-vacuous: pier census must be > 0");
    assert!(
        piers >= (f64::from(PIER_CENSUS) * 0.99).ceil() as u32,
        "G3: pier census {piers} < 0.99 × {PIER_CENSUS}"
    );
    assert_eq!(piers, PIER_CENSUS, "G3: exact pier census");

    let bridges = island_bridge_count(&r);
    assert!(bridges > 0, "G5 anti-vacuous: island must have bridges");
    assert_eq!(bridges, BRIDGE_CENSUS, "bridge instance census");
    assert_eq!(
        r.bridge_rail_strip_count(),
        2 * bridges as u32,
        "G5: every bridge emits exactly 2 rail strips"
    );

    let fill = r.world_building_fill();
    assert_eq!(
        fill_instances_with_color(&fill, BRIDGE_DECK_RGBA),
        bridges,
        "one warm-deck fill per bridge"
    );
    assert_eq!(
        fill_instances_with_color(&fill, BRIDGE_CASING_RGBA),
        bridges,
        "one casing rim per bridge"
    );

    let fences_on = r.fence_strip_segment_count();
    assert!(fences_on > 0, "fences must draw at z=1.5");

    r.set_fences_toggle(false);
    assert_eq!(r.fence_strip_segment_count(), 0, "G6: fences off ⇒ 0 fence");
    assert_eq!(
        r.pier_strip_segment_count(),
        piers,
        "G6: piers unaffected by fences"
    );
    assert_eq!(
        r.bridge_rail_strip_count(),
        2 * bridges as u32,
        "G6: rails unaffected by fences"
    );
    r.set_fences_toggle(true);
    assert_eq!(r.fence_strip_segment_count(), fences_on, "fences restored");

    r.set_glyph_toggles(true, false, false);
    assert_eq!(
        r.pier_strip_segment_count(),
        0,
        "G6: buildings off ⇒ 0 pier"
    );
    assert_eq!(
        r.bridge_rail_strip_count(),
        0,
        "G6: buildings off ⇒ 0 rails"
    );
    assert_eq!(
        r.fence_strip_segment_count(),
        fences_on,
        "G6: fences unaffected by buildings toggle"
    );
}
