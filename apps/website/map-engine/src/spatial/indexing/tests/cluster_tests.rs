//! Role: cluster tests.
//! Position: `spatial/indexing/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::indexing::cluster::*;

#[test]
fn conserves_points_at_every_zoom() {
    let n = 2000usize;
    let mut world = Vec::with_capacity(n);
    let mut s: u64 = 0x51ED_7A17;
    let mut nxt = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 33) as f64 / (1u64 << 31) as f64
    };
    for _ in 0..n {
        world.push((nxt() * 12800.0, nxt() * 12800.0));
    }
    let idx = ClusterIndex::build(&world, 12800.0, 12800.0);
    for deck_zoom in [-6.0, -5.0, -4.0] {
        let markers = idx.get_clusters(0.0, 0.0, 12800.0, 12800.0, deck_zoom);
        let total: u32 = markers.iter().map(|m| m.count).sum();
        assert_eq!(total as usize, n, "conservation @ deck_zoom {deck_zoom}");
        assert!(markers.iter().all(|m| m.count >= 1));
    }
}

#[test]
fn well_separated_blobs_cluster_deterministically() {
    let centers = [(3000.0, 3000.0), (6400.0, 6400.0), (9500.0, 9000.0)];
    let per = 5usize;
    let mut world = Vec::new();
    for &(cx, cy) in &centers {
        for j in 0..per {
            let d = (j as f64) * 8.0 - 16.0;
            world.push((cx + d, cy - d));
        }
    }
    let idx = ClusterIndex::build(&world, 12800.0, 12800.0);
    let markers = idx.get_clusters(0.0, 0.0, 12800.0, 12800.0, -6.0);
    let clusters: Vec<_> = markers.iter().filter(|m| m.count > 1).collect();
    assert_eq!(clusters.len(), 3, "one cluster per blob");
    assert!(clusters.iter().all(|c| c.count == per as u32));

    for &(cx, cy) in &centers {
        let hit = clusters
            .iter()
            .any(|c| (c.x - cx).abs() < 30.0 && (c.y - cy).abs() < 30.0);
        assert!(hit, "a cluster near ({cx},{cy})");
    }
}

#[test]
fn builds_and_conserves_at_100k() {
    let n = 100_000usize;
    let mut world = Vec::with_capacity(n);
    let mut s: u64 = 0xC0FF_EE42;
    let mut nxt = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 33) as f64 / (1u64 << 31) as f64
    };
    for _ in 0..n {
        world.push((nxt() * 12800.0, nxt() * 12800.0));
    }
    let idx = ClusterIndex::build(&world, 12800.0, 12800.0);
    for deck_zoom in [-6.0, -4.0] {
        let markers = idx.get_clusters(0.0, 0.0, 12800.0, 12800.0, deck_zoom);
        let total: u32 = markers.iter().map(|m| m.count).sum();
        assert_eq!(
            total as usize, n,
            "conservation @ 100k deck_zoom {deck_zoom}"
        );
    }
}
