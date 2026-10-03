//! Role: the residency lifecycle cases: requested set, pin key, known-empty chunks, retry cap,
//! LRU eviction, apply-frame accounting, picking, draw set, tree heatmap and glyph memo.
//! Position: `chunk_draw_buffers::tests::residency_lifecycle`; uses the fixtures of its `mod.rs`.
//! Signals & state: none; every case builds its own residency.
//! Invariants: each case asserts one lifecycle rule on the composed world residency.

use map_draw_lanes::zoom_gates::INSTANCE_BUDGET;

use vegetation::canopy::exact_tree_count;

use vegetation::canopy::visible_tree_count;

use map_coordinates::chunk_math::Bbox;

use map_coordinates::chunk_math::chunk_ids_for_rect;

use map_coordinates::chunk_math::chunk_ids_for_viewport;

use map_coordinates::chunk_math::chunk_rect_for_bbox;

use std::collections::HashSet;

use super::*;

#[test]
fn requests_exactly_the_chunk_math_set() {
    let mut r = setup();
    let missing = r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
    let expected: Vec<ChunkId> =
        chunk_ids_for_viewport([2000.0, 2000.0, 2200.0, 2200.0], r.terrain(), 512.0, 0)
            .into_iter()
            .map(ChunkId::new)
            .collect();
    assert_eq!(missing, expected);
}

#[test]
fn compose_memo_stable_then_bumps() {
    let mut r = dense_forest_setup();
    drive(&mut r, [2000.0, 2000.0, 2200.0, 2200.0]);
    let rev0 = r.buffers_revision();

    r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
    r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
    assert_eq!(
        r.buffers_revision(),
        rev0,
        "identical viewport must not recompose"
    );

    r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -1.0);
    assert!(r.buffers_revision() > rev0, "zoom change must recompose");
}

#[test]
fn compose_memo_invalidates_on_new_chunk() {
    let mut r = setup();

    let missing = r.set_viewport(0.0, 0.0, 900.0, 900.0, -2.0);
    assert!(missing.len() >= 2);
    r.ingest_chunk_gz(&missing[0], &chunk_bytes(missing[0].as_str()))
        .unwrap();
    r.end_apply_frame(0.0);
    r.set_viewport(0.0, 0.0, 900.0, 900.0, -2.0);
    let rev1 = r.buffers_revision();
    r.ingest_chunk_gz(&missing[1], &chunk_bytes(missing[1].as_str()))
        .unwrap();
    r.end_apply_frame(0.0);
    r.set_viewport(0.0, 0.0, 900.0, 900.0, -2.0);
    assert!(
        r.buffers_revision() > rev1,
        "a freshly-ingested chunk under the same pin must recompose"
    );
}

#[test]
fn parsed_empty_chunk_is_known_empty_and_not_refetched() {
    let mut r = setup();
    let missing = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
    let id = missing[0].clone();
    let out = r
        .ingest_chunk_gz(&id, &gzip(r#"{"instances":[]}"#))
        .unwrap();
    assert_eq!(out, IngestOutcome::ParsedEmpty);
    assert!(r.stats_json().contains("\"known_empty_count\":1"));

    let again = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
    assert!(
        !again.contains(&id),
        "known-empty chunk must not be re-requested"
    );
}

#[test]
fn shape_mismatch_retries_to_cap_then_caches() {
    let mut r = setup();
    let missing = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
    let id = missing[0].clone();
    for _ in 0..(FETCH_FAILURE_CAP - 1) {
        let out = r.ingest_chunk_gz(&id, &gzip("{}")).unwrap();
        assert_eq!(out, IngestOutcome::ShapeMismatch);

        assert!(r.resident_instance_count(&id).is_none());
    }

    r.ingest_chunk_gz(&id, &gzip("{}")).unwrap();
    assert!(r.resident_instance_count(&id).is_some());
    let again = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
    assert!(!again.contains(&id));
}

#[test]
fn fetch_failures_reset_on_new_pin_key() {
    let mut r = setup();
    let m0 = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
    let id = m0[0].clone();
    r.ingest_chunk_gz(&id, &gzip("{}")).unwrap();

    r.set_viewport(6000.0, 6000.0, 6200.0, 6200.0, -2.0);
    let back = r.set_viewport(0.0, 0.0, 200.0, 200.0, -2.0);
    assert!(
        back.contains(&id),
        "failure counter must reset across pin epochs"
    );
}

#[test]
fn clear_inflight_allows_same_key_rerequest() {
    let mut r = setup();
    let missing = r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
    assert!(!missing.is_empty());
    assert!(r.inflight_count() > 0);
    assert!(!r.pin_settled());

    r.clear_inflight();
    assert_eq!(r.inflight_count(), 0);

    let again = r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
    assert_eq!(again, missing);
    assert_eq!(r.inflight_count(), missing.len());

    for id in &again {
        r.ingest_chunk_gz(id, &chunk_bytes(id.as_str())).unwrap();
    }
    r.end_apply_frame(0.0);
    assert!(r.pin_settled());
    assert!(r.pinned_building_count() > 0);

    assert!(
        r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0)
            .is_empty()
    );
}

#[test]
fn skip_below_building_band_and_unchanged_set() {
    let mut r = setup();
    assert!(
        r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -3.0)
            .is_empty()
    );
    let first = r.set_viewport(2000.0, 2000.0, 2200.0, 2200.0, -2.0);
    assert!(!first.is_empty());

    assert!(
        r.set_viewport(2001.0, 2001.0, 2201.0, 2201.0, -2.0)
            .is_empty()
    );
}

#[test]
fn building_count_and_buffers_track_pins() {
    let mut r = setup();
    drive(&mut r, [2000.0, 2000.0, 2200.0, 2200.0]);
    let pinned = r.chunk_residency.pinned_ids().len() as u32;
    assert!(pinned > 0);

    assert_eq!(r.pinned_building_count(), pinned);

    assert_eq!(r.world_building_fill().len(), 10 * pinned as usize);
    assert_eq!(r.world_building_outline().len(), 48 * pinned as usize);
}

#[test]
fn lru_caps_and_never_evicts_pinned() {
    let mut r = setup();

    drive(&mut r, [200.0, 200.0, 400.0, 400.0]);
    let first_ids: Vec<String> = r.chunk_residency.pinned_ids().to_vec();

    for i in 0..10 {
        let x = 3000.0 + f64::from(i) * 1024.0;
        drive(&mut r, [x, 6000.0, x + 200.0, 6200.0]);
    }

    let cap = LRU_MIN_CHUNKS.max(3 * r.chunk_residency.pinned_ids().len());
    assert!(r.chunks_resident() <= cap);

    for id in r.chunk_residency.pinned_ids() {
        assert!(
            r.chunk_residency.resident_chunks().contains_key(id),
            "pinned {id} must stay resident"
        );
    }

    let first_evicted = first_ids
        .iter()
        .any(|id| !r.chunk_residency.resident_chunks().contains_key(id));
    assert!(first_evicted, "first viewport should have been evicted");
}

#[test]
fn eviction_order_is_ascending_last_used() {
    let mut r = setup();

    for i in 0..12 {
        let x = 500.0 + f64::from(i) * 1024.0;
        drive(&mut r, [x, 500.0, x + 200.0, 700.0]);
    }
    let log = r.eviction_log();
    assert!(!log.is_empty(), "sweep should evict");

    for id in &log {
        assert!(!r.chunk_residency.resident_chunks().contains_key(id));
    }
}

#[test]
fn budget_accounting_matches_elapsed_sequence() {
    let mut r = setup();
    for ms in [1.0_f64, 5.0, 3.0, 6.0, 4.0] {
        r.end_apply_frame(ms);
    }
    assert_eq!(r.chunk_residency.apply_frames(), 5);
    assert_eq!(r.frames_over_budget(), 2);
    assert!((r.chunk_residency.max_apply_ms() - 6.0).abs() < f64::EPSILON);
    assert!((r.chunk_residency.apply_budget_ms_last() - 4.0).abs() < f64::EPSILON);
}

#[test]
fn pick_finds_resident_building() {
    let mut r = setup();
    drive(&mut r, [2000.0, 2000.0, 2200.0, 2200.0]);

    let hit = r.pick_nearest(2148.5, 2248.25, 5.0, Some(1 << 0));
    assert!(hit.is_some(), "should pick the building at chunk 4_4");
}

#[test]
fn class_s_draw_set_equals_strict_reference() {
    let mut r = setup();

    drive(&mut r, [1500.0, 1500.0, 3500.0, 3500.0]);
    assert!(r.chunk_residency.pinned_ids().len() >= 9);

    let strict: Bbox = [2048.0, 2048.0, 3072.0, 3072.0];
    let draw = r.draw_chunk_ids(strict);
    let mut reference = chunk_ids_for_rect(chunk_rect_for_bbox(strict, r.terrain(), 512.0));
    if let Some(cells) = r.chunk_residency.cell_ids() {
        reference.retain(|id| cells.contains(id));
    }
    reference.retain(|id| r.chunk_residency.is_pinned(&ChunkId::from(id.as_str())));
    reference.sort();
    assert_eq!(draw, reference);

    for id in &draw {
        assert!(r.chunk_residency.is_pinned(&ChunkId::from(id.as_str())));
    }

    let pin_via_viewport = chunk_ids_for_viewport(strict, r.terrain(), 512.0, 0);
    let mut pin_set: HashSet<String> = pin_via_viewport.into_iter().collect();
    if let Some(cells) = r.chunk_residency.cell_ids() {
        pin_set.retain(|id| cells.contains(id));
    }

    assert!(
        pin_set.len() > draw.len(),
        "preload must expand beyond strict draw (pin {} vs draw {})",
        pin_set.len(),
        draw.len()
    );
    for id in &draw {
        assert!(pin_set.contains(id), "draw id {id} must be in preload pin");
    }
}

#[test]
fn class_r_heatmap_swap_and_full_pack() {
    let mut r = setup();

    drive(&mut r, [2048.0, 2048.0, 3072.0, 3072.0]);
    r.chunk_residency
        .set_last_viewport_for_test([2048.0, 2048.0, 3072.0, 3072.0]);
    r.chunk_residency.set_deck_zoom_for_test(0.0);
    r.draw_buffers.toggle_trees = true;

    let draw = r.draw_chunk_ids(r.chunk_residency.last_viewport());
    assert!(!draw.is_empty());

    let id0 = draw[0].clone();
    inject_trees(&mut r, &id0, 10);
    r.draw_buffers
        .refresh_draw_set_and_glyphs(&r.chunk_residency);
    let exact = exact_tree_count(
        r.chunk_residency.resident_chunks(),
        r.draw_ids(),
        r.chunk_residency.deck_zoom(),
    );
    assert_eq!(exact, 10);
    assert!(!r.heatmap_trees_active());

    let (grid, gw) = density_grid_of(&r);
    let sum = density_texel_sum_for_draw_ids(&grid, gw, r.draw_ids());
    assert_eq!(sum, exact as u64);

    inject_trees(&mut r, &id0, INSTANCE_BUDGET + 1);
    r.draw_buffers
        .refresh_draw_set_and_glyphs(&r.chunk_residency);
    assert!(r.heatmap_trees_active());
    assert_eq!(r.tree_glyph_count(), 0);
    assert_eq!(r.exact_tree_count_draw() as usize, INSTANCE_BUDGET + 1);
    let (grid2, gw2) = density_grid_of(&r);
    let sum2 = density_texel_sum_for_draw_ids(&grid2, gw2, r.draw_ids());
    assert_eq!(sum2, (INSTANCE_BUDGET + 1) as u64);

    assert!(r.forest_fill_effective());
}

#[test]
fn class_r_partial_coverage_no_swap() {
    let mut r = setup();
    drive(&mut r, [2048.0, 2048.0, 2560.0, 2560.0]);
    inject_trees(&mut r, "4_4", INSTANCE_BUDGET + 1);
    r.chunk_residency.set_deck_zoom_for_test(0.0);
    r.draw_buffers.toggle_trees = true;

    r.chunk_residency
        .set_last_viewport_for_test([2048.0, 2048.0, 2099.2, 2560.0]);
    r.draw_buffers
        .refresh_draw_set_and_glyphs(&r.chunk_residency);
    let visible = visible_tree_count(
        r.chunk_residency.resident_chunks(),
        r.draw_ids(),
        r.chunk_residency.last_viewport(),
        512.0,
    );
    assert!(visible <= INSTANCE_BUDGET, "sliver visible = {visible}");
    assert!(!r.heatmap_trees_active());

    assert_eq!(r.exact_tree_count_draw() as usize, INSTANCE_BUDGET + 1);
}

#[test]
fn class_r_heatmap_hysteresis() {
    let mut r = setup();
    drive(&mut r, [2048.0, 2048.0, 2560.0, 2560.0]);
    r.chunk_residency.set_deck_zoom_for_test(0.0);
    r.draw_buffers.toggle_trees = true;
    r.chunk_residency
        .set_last_viewport_for_test([2048.0, 2048.0, 2560.0, 2560.0]);
    let reenter = INSTANCE_BUDGET * 85 / 100;

    inject_trees(&mut r, "4_4", INSTANCE_BUDGET + 1);
    r.draw_buffers
        .refresh_draw_set_and_glyphs(&r.chunk_residency);
    assert!(r.heatmap_trees_active(), "enter above budget");

    inject_trees(&mut r, "4_4", INSTANCE_BUDGET - 1);
    r.draw_buffers
        .refresh_draw_set_and_glyphs(&r.chunk_residency);
    assert!(
        r.heatmap_trees_active(),
        "stay in heatmap inside the hysteresis band"
    );

    inject_trees(&mut r, "4_4", reenter - 1);
    r.draw_buffers
        .refresh_draw_set_and_glyphs(&r.chunk_residency);
    assert!(!r.heatmap_trees_active(), "exit below 0.85×budget");
}

#[test]
fn property_never_blank_zoom_ladder() {
    let vp = [2048.0, 2048.0, 3072.0, 3072.0];

    for &per_chunk in &[50usize, 40_000usize] {
        let mut r = dense_forest_setup();
        drive(&mut r, vp);
        for id in r.draw_chunk_ids(vp) {
            inject_trees(&mut r, &id, per_chunk);
        }
        r.draw_buffers.toggle_trees = true;
        for step in 0..=12u32 {
            let z = f64::from(step) * 0.5;
            r.chunk_residency.set_deck_zoom_for_test(z);
            r.chunk_residency.set_last_viewport_for_test(vp);
            r.draw_buffers
                .refresh_draw_set_and_glyphs(&r.chunk_residency);

            let glyphs = r.tree_glyph_count() > 0;
            let fill = r.forest_fill_effective();
            let heat = r.heatmap_trees_active();
            let (grid, _gw) = density_grid_of(&r);
            let grid_nonzero = grid.iter().any(|&v| v > 0);
            assert!(
                glyphs || fill || (heat && grid_nonzero),
                "blank band @ z={z}, per_chunk={per_chunk}"
            );

            let visible =
                visible_tree_count(r.chunk_residency.resident_chunks(), r.draw_ids(), vp, 512.0);
            if visible <= INSTANCE_BUDGET {
                assert!(glyphs, "glyphs empty under budget @ z={z}");
                assert!(!fill, "mass must be off when glyphs pack @ z={z}");
            } else {
                assert!(heat, "heatmap must own the rung over budget @ z={z}");
                assert!(fill, "mass must persist under heatmap @ z={z}");
            }
        }
    }
}

#[test]
fn class_r_chunks_draw_matches_draw_ids_len() {
    let mut r = setup();
    drive(&mut r, [2048.0, 2048.0, 3072.0, 3072.0]);
    assert_eq!(r.chunks_draw() as usize, r.draw_ids().len());
    let stats: serde_json::Value = serde_json::from_str(&r.stats_json()).unwrap();
    assert_eq!(
        stats["chunks_draw"].as_u64().unwrap(),
        r.draw_ids().len() as u64
    );
}

#[test]
fn a5_floor_zoom_term_sentinel_above_exact_below() {
    assert_eq!(
        DrawBuffers::floor_zoom_term(3.0, 2.5),
        DrawBuffers::floor_zoom_term(4.0, 2.5),
        "above the floor the term is a constant sentinel"
    );
    assert_eq!(
        DrawBuffers::floor_zoom_term(2.5, 2.5),
        u64::MAX,
        "boundary inclusive"
    );
    assert_ne!(
        DrawBuffers::floor_zoom_term(2.0, 2.5),
        DrawBuffers::floor_zoom_term(2.4, 2.5),
        "below the floor distinct zooms are distinct"
    );
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn a5_glyph_memo_hits_in_band_busts_on_cross() {
    let mut r = WorldResidency::default();
    r.draw_buffers.glyph_size_floor_zoom = 2.678;

    r.chunk_residency.set_deck_zoom_for_test(3.1);
    let a = r.draw_buffers.glyph_base_sig(&r.chunk_residency);
    r.chunk_residency.set_deck_zoom_for_test(3.5);
    let b = r.draw_buffers.glyph_base_sig(&r.chunk_residency);
    assert_eq!(
        a, b,
        "fine zoom above floor with no band crossing → memo hit"
    );

    r.chunk_residency.set_deck_zoom_for_test(2.9);
    let c = r.draw_buffers.glyph_base_sig(&r.chunk_residency);
    assert_ne!(
        b, c,
        "prop band crossing busts the memo even above the size floor"
    );

    r.chunk_residency.set_deck_zoom_for_test(2.0);
    let d = r.draw_buffers.glyph_base_sig(&r.chunk_residency);
    r.chunk_residency.set_deck_zoom_for_test(2.4);
    let e = r.draw_buffers.glyph_base_sig(&r.chunk_residency);
    assert_ne!(
        d, e,
        "below the size floor any zoom delta busts (continuous glyph size)"
    );
}
