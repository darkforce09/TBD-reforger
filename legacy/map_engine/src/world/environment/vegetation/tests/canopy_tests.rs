//! Role: canopy tests.
//! Position: `world/environment/vegetation/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::streaming::loaders::chunk::WorldChunk;
use crate::world::environment::classify::class_code;
use crate::world::environment::vegetation::canopy::*;

fn chunk_with_tree_rows(id: &str, n: usize) -> WorldChunk {
    let mut c = WorldChunk {
        id: id.to_string(),
        count: n as u32,
        positions: vec![0.0; n * 2],
        prefab_idx: vec![0; n],
        rotations: vec![0.0; n],
        z: vec![0.0; n],
        cls_codes: vec![class_code("tree"); n],
        rows_by_class: HashMap::new(),
        ..Default::default()
    };
    c.rows_by_class
        .insert(class_code("tree"), (0..n as u32).collect::<Vec<_>>());
    c
}

#[test]
fn r1_exact_tree_count_hand_sum() {
    let mut chunks = HashMap::new();
    chunks.insert("1_1".into(), chunk_with_tree_rows("1_1", 10));
    chunks.insert("1_2".into(), chunk_with_tree_rows("1_2", 7));
    let draw = vec!["1_1".into(), "1_2".into()];

    assert_eq!(exact_tree_count(&chunks, &draw, 0.0), 17);
    assert_eq!(exact_tree_count(&chunks, &draw, 2.0), 17);
}

#[test]
fn r2_heatmap_boundary() {
    assert!(!heatmap_trees(INSTANCE_BUDGET));
    assert!(heatmap_trees(INSTANCE_BUDGET + 1));
    assert!(!heatmap_trees(0));
}

#[test]
fn r3_texel_sum_equals_exact() {
    let mut chunks = HashMap::new();
    chunks.insert("0_0".into(), chunk_with_tree_rows("0_0", 3));
    chunks.insert("1_0".into(), chunk_with_tree_rows("1_0", 5));
    chunks.insert("0_1".into(), chunk_with_tree_rows("0_1", 2));
    let grid = pack_density_grid_r32(&chunks, 2, 2);
    let draw = vec!["0_0".into(), "1_0".into()];
    let exact = exact_tree_count(&chunks, &draw, 0.0) as u64;
    assert_eq!(density_texel_sum_for_draw_ids(&grid, 2, &draw), exact);
    assert_eq!(exact, 8);
}

#[test]
fn everon_grid_dims() {
    assert_eq!(density_grid_dims(12800.0, 12800.0, 512.0), (25, 25));
}

#[test]
fn visible_tree_count_full_and_zero_fraction() {
    let mut chunks = HashMap::new();

    chunks.insert("1_1".into(), chunk_with_tree_rows("1_1", 10));
    let draw = vec!["1_1".into()];

    assert_eq!(
        visible_tree_count(&chunks, &draw, [0.0, 0.0, 2048.0, 2048.0], 512.0),
        10
    );

    assert_eq!(
        visible_tree_count(&chunks, &draw, [2048.0, 2048.0, 4096.0, 4096.0], 512.0),
        0
    );
}

#[test]
fn visible_tree_count_partial_fraction_floors() {
    let mut chunks = HashMap::new();
    chunks.insert("0_0".into(), chunk_with_tree_rows("0_0", 10));
    let draw = vec!["0_0".into()];

    assert_eq!(
        visible_tree_count(&chunks, &draw, [0.0, 0.0, 256.0, 512.0], 512.0),
        5
    );

    assert_eq!(
        visible_tree_count(&chunks, &draw, [0.0, 0.0, 256.0, 256.0], 512.0),
        2
    );
}

#[test]
fn visible_tree_count_multi_chunk_mixed_coverage() {
    let mut chunks = HashMap::new();
    chunks.insert("0_0".into(), chunk_with_tree_rows("0_0", 100));
    chunks.insert("1_0".into(), chunk_with_tree_rows("1_0", 100));
    let draw = vec!["0_0".into(), "1_0".into()];

    assert_eq!(
        visible_tree_count(&chunks, &draw, [0.0, 0.0, 768.0, 512.0], 512.0),
        150
    );
}
