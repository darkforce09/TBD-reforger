//! Unit tests of the shared flat-tree build core: leaf tiling, box containment, the two limits,
//! the subset order and determinism.

use crate::bounding_volume_hierarchy::flat_tree_build::{
    BuildLimits, BvhNode, FlatTree, ItemBounds, build_flat_tree,
};

const WORLD_BOXES: BuildLimits = BuildLimits {
    leaf_max: 4,
    max_depth: 48,
};

fn grid_boxes(n: usize) -> Vec<ItemBounds> {
    (0..n)
        .map(|i| {
            let x = (i % 17) as f64 * 3.0;
            let y = (i / 17) as f64 * 5.0;
            let z = (i % 3) as f64;
            ItemBounds::of_box([x, y, z], [x + 1.5, y + 2.0, z + 4.0])
        })
        .collect()
}

/// Every leaf, with its depth.
fn leaves(tree: &FlatTree) -> Vec<(BvhNode, usize)> {
    let mut out = Vec::new();
    let mut stack = vec![(0usize, 0usize)];
    while let Some((node, depth)) = stack.pop() {
        let n = tree.nodes[node];
        if n.count > 0 {
            out.push((n, depth));
        } else {
            stack.push((n.left_first as usize, depth + 1));
            stack.push((n.left_first as usize + 1, depth + 1));
        }
    }
    out
}

fn contains(node: &BvhNode, item: &ItemBounds) -> bool {
    (0..3).all(|a| f64::from(node.min[a]) <= item.lo[a] && item.hi[a] <= f64::from(node.max[a]))
}

#[test]
fn flat_tree_leaves_tile_the_order_and_respect_the_leaf_limit() {
    let items = grid_boxes(300);
    let tree = build_flat_tree(&items, (0..300).collect(), WORLD_BOXES);
    let mut covered = vec![0u32; 300];
    for (leaf, _) in leaves(&tree) {
        assert!(leaf.count as usize <= WORLD_BOXES.leaf_max);
        for &i in &tree.order[leaf.left_first as usize..(leaf.left_first + leaf.count) as usize] {
            covered[i as usize] += 1;
            assert!(contains(&leaf, &items[i as usize]));
        }
    }
    assert!(covered.iter().all(|&c| c == 1));
}

#[test]
fn flat_tree_internal_boxes_contain_their_children() {
    let items = grid_boxes(120);
    let tree = build_flat_tree(&items, (0..120).collect(), BuildLimits::TRIANGLES);
    for n in tree.nodes.iter().filter(|n| n.count == 0) {
        for child in [n.left_first as usize, n.left_first as usize + 1] {
            let c = tree.nodes[child];
            assert!((0..3).all(|a| n.min[a] <= c.min[a] && c.max[a] <= n.max[a]));
        }
    }
}

#[test]
fn flat_tree_stops_at_the_depth_limit() {
    let items = grid_boxes(200);
    let limits = BuildLimits {
        leaf_max: 1,
        max_depth: 3,
    };
    let tree = build_flat_tree(&items, (0..200).collect(), limits);
    let all = leaves(&tree);
    assert!(all.iter().all(|&(_, depth)| depth <= 3));
    assert!(all.iter().any(|&(leaf, _)| leaf.count > 1));
}

#[test]
fn flat_tree_reads_only_the_items_its_order_names() {
    let mut items = grid_boxes(40);
    items[7] = ItemBounds::of_box([f64::NAN; 3], [f64::NAN; 3]);
    let order: Vec<u32> = (0..40).filter(|&i| i != 7).collect();
    let tree = build_flat_tree(&items, order, WORLD_BOXES);
    assert_eq!(tree.order.len(), 39);
    assert!(!tree.order.contains(&7));
    assert!(
        tree.nodes
            .iter()
            .all(|n| n.min.iter().all(|v| v.is_finite()))
    );
}

#[test]
fn flat_tree_of_nothing_is_empty_and_coincident_items_are_one_leaf() {
    assert_eq!(
        build_flat_tree(&grid_boxes(3), Vec::new(), WORLD_BOXES),
        FlatTree::default()
    );
    let same = vec![ItemBounds::of_box([1.0; 3], [2.0; 3]); 50];
    let tree = build_flat_tree(&same, (0..50).collect(), WORLD_BOXES);
    assert_eq!(tree.nodes.len(), 1);
    assert_eq!(tree.nodes[0].count, 50);
}

#[test]
fn flat_tree_build_is_deterministic() {
    let items = grid_boxes(257);
    let a = build_flat_tree(&items, (0..257).collect(), WORLD_BOXES);
    let b = build_flat_tree(&items, (0..257).collect(), WORLD_BOXES);
    assert_eq!(a, b);
}

#[test]
fn flat_tree_of_exactly_leaf_max_items_is_one_leaf() {
    let items = grid_boxes(WORLD_BOXES.leaf_max + 1);
    let fits = build_flat_tree(&items, (0..4).collect(), WORLD_BOXES);
    assert_eq!(fits.nodes.len(), 1);
    assert_eq!(fits.nodes[0].count as usize, WORLD_BOXES.leaf_max);
    let splits = build_flat_tree(&items, (0..5).collect(), WORLD_BOXES);
    assert_eq!(splits.nodes.len(), 3);
}
