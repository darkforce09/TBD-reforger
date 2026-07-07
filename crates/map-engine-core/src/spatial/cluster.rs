//! Supercluster-compatible hierarchical cluster index over a slot SoA — the Phase 3 replacement for
//! `state/slotClusterIndex.ts` (which wraps `supercluster` for the zoomed-out LOD bubbles).
//!
//! **Class S** (structural: replaces `supercluster`) — the contract is query-result-set equality, not
//! layout identity. It faithfully mirrors supercluster 8.0.1: the same linear world→lng/lat
//! normalization the app uses (`LNG_SPAN`/`LAT_SPAN` from `slotClusterIndex.ts`), the same
//! `fround(lngX/latY)` Web-Mercator projection, the same greedy clustering with
//! `r = radius / (extent · 2^zoom)` and weighted tile-space centroids, and the same
//! `getClusters` → inverse-project path. Exact cluster *membership* on dense inputs is greedy-order
//! dependent (supercluster iterates in KDBush order; this iterates in input order), so the pinned
//! parity is: exact clusters + counts + centroids on **well-separated groups** (order-independent),
//! plus **point conservation** everywhere. The cutover swaps the brute-force neighbor scan here for a
//! per-level grid; the projection + clustering math is what parity locks.

use std::f64::consts::PI;

const EXTENT: f64 = 512.0;
/// Cluster radius in pixels — matches `new Supercluster({ radius: 60 })` in `slotClusterIndex.ts`.
const CLUSTER_RADIUS: f64 = 60.0;
const MAX_ZOOM: i32 = 16;
const MIN_ZOOM: i32 = 0;
/// Linear world→lng/lat window (mirror of `slotClusterIndex.ts` `LNG_SPAN`/`LAT_SPAN`).
const LNG_SPAN: f64 = 360.0;
const LAT_SPAN: f64 = 170.0;

/// A cluster bubble (`count > 1`, `leaf < 0`) or a lone leaf (`count == 1`, `leaf` = row handle), in
/// **world meters** — ready for the cluster render layer.
#[derive(Clone, Copy, Debug)]
pub struct ClusterMarker {
    pub x: f64,
    pub y: f64,
    pub count: u32,
    pub leaf: i64,
}

/// One node in a zoom level: tile coords in `[0,1]²`, an accumulated point count, and (for a leaf)
/// its row handle.
#[derive(Clone, Copy)]
struct Node {
    x: f64,
    y: f64,
    num_points: u32,
    leaf: i64,
}

/// A built cluster hierarchy over a set of world-space points.
pub struct ClusterIndex {
    terrain_w: f64,
    terrain_h: f64,
    /// Original world coords, indexed by leaf handle (so a leaf marker returns its exact input point,
    /// as supercluster returns `this.points[id]`).
    leaves_world: Vec<(f64, f64)>,
    /// One vec of nodes per zoom, `levels[z]` for `z` in `0..=MAX_ZOOM+1` (the leaf level is at
    /// `MAX_ZOOM+1`).
    levels: Vec<Vec<Node>>,
}

// --- projection (verbatim supercluster 8.0.1 + the app's linear normalization) ------------------

#[inline]
fn fround(v: f64) -> f64 {
    v as f32 as f64
}
#[inline]
fn norm_lng(x: f64, w: f64) -> f64 {
    x / w * LNG_SPAN - 180.0
}
#[inline]
fn norm_lat(y: f64, h: f64) -> f64 {
    y / h * LAT_SPAN - 85.0
}
#[inline]
fn lng_x(lng: f64) -> f64 {
    lng / 360.0 + 0.5
}
#[inline]
fn lat_y(lat: f64) -> f64 {
    let sin = (lat * PI / 180.0).sin();
    let y = 0.5 - 0.25 * ((1.0 + sin) / (1.0 - sin)).ln() / PI;
    y.clamp(0.0, 1.0)
}
#[inline]
fn x_lng(x: f64) -> f64 {
    (x - 0.5) * 360.0
}
#[inline]
fn y_lat(y: f64) -> f64 {
    let y2 = (180.0 - y * 360.0) * PI / 180.0;
    360.0 * y2.exp().atan() / PI - 90.0
}
#[inline]
fn world_x(lng: f64, w: f64) -> f64 {
    (lng + 180.0) / LNG_SPAN * w
}
#[inline]
fn world_y(lat: f64, h: f64) -> f64 {
    (lat + 85.0) / LAT_SPAN * h
}

/// Deck zoom → integer supercluster zoom (`slotClusterIndex.ts` `deckZoomToSuperZoom`).
#[must_use]
pub fn deck_zoom_to_super_zoom(deck_zoom: f64) -> i32 {
    let z = (deck_zoom + 8.0).round() as i32;
    z.clamp(0, 16)
}

impl ClusterIndex {
    /// Build the hierarchy over world-space points (row index = leaf handle).
    #[must_use]
    pub fn build(world: &[(f64, f64)], terrain_w: f64, terrain_h: f64) -> ClusterIndex {
        let leaves: Vec<Node> = world
            .iter()
            .enumerate()
            .map(|(i, &(wx, wy))| Node {
                x: fround(lng_x(norm_lng(wx, terrain_w))),
                y: fround(lat_y(norm_lat(wy, terrain_h))),
                num_points: 1,
                leaf: i as i64,
            })
            .collect();

        let mut levels: Vec<Vec<Node>> = vec![Vec::new(); (MAX_ZOOM + 2) as usize];
        levels[(MAX_ZOOM + 1) as usize] = leaves;
        for z in (MIN_ZOOM..=MAX_ZOOM).rev() {
            let prev = std::mem::take(&mut levels[(z + 1) as usize]);
            levels[z as usize] = cluster_level(&prev, z);
            levels[(z + 1) as usize] = prev;
        }

        ClusterIndex {
            terrain_w,
            terrain_h,
            leaves_world: world.to_vec(),
            levels,
        }
    }

    #[inline]
    fn limit_zoom(z: i32) -> i32 {
        z.clamp(MIN_ZOOM, MAX_ZOOM + 1)
    }

    /// Clusters/leaves inside a world-meter bbox at a deck zoom (mirrors
    /// `slotClusterIndex.getClusters`). Returned centroids are world meters.
    #[must_use]
    pub fn get_clusters(
        &self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        deck_zoom: f64,
    ) -> Vec<ClusterMarker> {
        let z = Self::limit_zoom(deck_zoom_to_super_zoom(deck_zoom));
        let level = &self.levels[z as usize];

        // supercluster: range(lngX(minLng), latY(maxLat), lngX(maxLng), latY(minLng)) — latY inverts.
        let bx0 = lng_x(norm_lng(min_x, self.terrain_w));
        let bx1 = lng_x(norm_lng(max_x, self.terrain_w));
        let by0 = lat_y(norm_lat(min_y, self.terrain_h));
        let by1 = lat_y(norm_lat(max_y, self.terrain_h));
        let (tminx, tmaxx) = (bx0.min(bx1), bx0.max(bx1));
        let (tminy, tmaxy) = (by0.min(by1), by0.max(by1));

        let mut out = Vec::new();
        for n in level {
            if n.x < tminx || n.x > tmaxx || n.y < tminy || n.y > tmaxy {
                continue;
            }
            if n.num_points > 1 {
                let lng = x_lng(n.x);
                let lat = y_lat(n.y);
                out.push(ClusterMarker {
                    x: world_x(lng, self.terrain_w),
                    y: world_y(lat, self.terrain_h),
                    count: n.num_points,
                    leaf: -1,
                });
            } else {
                let (wx, wy) = self.leaves_world[n.leaf as usize];
                out.push(ClusterMarker {
                    x: wx,
                    y: wy,
                    count: 1,
                    leaf: n.leaf,
                });
            }
        }
        out
    }

    #[must_use]
    pub fn leaf_count(&self) -> usize {
        self.leaves_world.len()
    }
}

/// One clustering pass over `prev` at `zoom` — a faithful transcription of supercluster's `_cluster`
/// (greedy over the input order; neighbors within `r`; weighted tile-space centroid).
fn cluster_level(prev: &[Node], zoom: i32) -> Vec<Node> {
    let r = CLUSTER_RADIUS / (EXTENT * 2f64.powi(zoom));
    let r2 = r * r;
    let mut processed = vec![false; prev.len()];
    let mut next: Vec<Node> = Vec::new();

    for i in 0..prev.len() {
        if processed[i] {
            continue;
        }
        processed[i] = true;
        let p = prev[i];

        let neighbors: Vec<usize> = (0..prev.len())
            .filter(|&k| {
                let dx = prev[k].x - p.x;
                let dy = prev[k].y - p.y;
                dx * dx + dy * dy <= r2
            })
            .collect();

        // Count still-unprocessed neighbors (supercluster: those with zoom > current).
        let mut num_points = p.num_points;
        for &k in &neighbors {
            if k != i && !processed[k] {
                num_points += prev[k].num_points;
            }
        }

        if num_points > p.num_points {
            // Cluster forms — weighted centroid over the seed + its unprocessed neighbors.
            let mut wx = p.x * f64::from(p.num_points);
            let mut wy = p.y * f64::from(p.num_points);
            for &k in &neighbors {
                if k == i || processed[k] {
                    continue;
                }
                processed[k] = true;
                wx += prev[k].x * f64::from(prev[k].num_points);
                wy += prev[k].y * f64::from(prev[k].num_points);
            }
            next.push(Node {
                x: wx / f64::from(num_points),
                y: wy / f64::from(num_points),
                num_points,
                leaf: -1,
            });
        } else {
            next.push(p);
        }
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Total point conservation: summing counts over the whole terrain at any zoom returns N.
    #[test]
    fn conserves_points_at_every_zoom() {
        // Deterministic LCG points in [0, 12800)².
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

    /// Three tight, far-apart blobs collapse to three clusters at the tested zoom, regardless of
    /// iteration order — each cluster carries the blob size and sits at the blob centroid.
    #[test]
    fn well_separated_blobs_cluster_deterministically() {
        let centers = [(3000.0, 3000.0), (6400.0, 6400.0), (9500.0, 9000.0)];
        let per = 5usize;
        let mut world = Vec::new();
        for &(cx, cy) in &centers {
            for j in 0..per {
                let d = (j as f64) * 8.0 - 16.0; // ±16 m — far inside the ~375 m radius @ super-zoom 2
                world.push((cx + d, cy - d));
            }
        }
        let idx = ClusterIndex::build(&world, 12800.0, 12800.0);
        let markers = idx.get_clusters(0.0, 0.0, 12800.0, 12800.0, -6.0); // super-zoom 2
        let clusters: Vec<_> = markers.iter().filter(|m| m.count > 1).collect();
        assert_eq!(clusters.len(), 3, "one cluster per blob");
        assert!(clusters.iter().all(|c| c.count == per as u32));
        // Each cluster sits near its blob centroid (mean of ±16 m spread ≈ the center).
        for &(cx, cy) in &centers {
            let hit = clusters
                .iter()
                .any(|c| (c.x - cx).abs() < 30.0 && (c.y - cy).abs() < 30.0);
            assert!(hit, "a cluster near ({cx},{cy})");
        }
    }
}
