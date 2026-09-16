//! Role: cluster.
//! Position: `spatial/indexing` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::f64::consts::PI;

const EXTENT: f64 = 512.0;

const CLUSTER_RADIUS: f64 = 60.0;
const MAX_ZOOM: i32 = 16;
const MIN_ZOOM: i32 = 0;

const LNG_SPAN: f64 = 360.0;
const LAT_SPAN: f64 = 170.0;

/// A cluster bubble (`count > 1`, `leaf < 0`) or a lone leaf (`count == 1`, `leaf` = row handle), in **world meters** — ready for the cluster render layer.
#[derive(Clone, Copy, Debug)]
pub struct ClusterMarker {
    /// X.
    pub x: f64,

    /// Y.
    pub y: f64,

    /// Count.
    pub count: u32,

    /// Leaf.
    pub leaf: i64,
}

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

    leaves_world: Vec<(f64, f64)>,

    levels: Vec<Vec<Node>>,
}

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

const GRID_COLS: usize = 512;

struct Grid {
    cell_start: Vec<u32>,
    items: Vec<u32>,
}

impl Grid {
    #[inline]
    fn cell_of(x: f64, y: f64) -> (usize, usize) {
        let n = GRID_COLS as f64;
        let cx = ((x * n).floor() as isize).clamp(0, GRID_COLS as isize - 1) as usize;
        let cy = ((y * n).floor() as isize).clamp(0, GRID_COLS as isize - 1) as usize;
        (cx, cy)
    }

    fn build(nodes: &[Node]) -> Grid {
        let ncells = GRID_COLS * GRID_COLS;
        let mut cell_start = vec![0u32; ncells + 1];
        for n in nodes {
            let (cx, cy) = Grid::cell_of(n.x, n.y);
            cell_start[cy * GRID_COLS + cx + 1] += 1;
        }
        for c in 0..ncells {
            cell_start[c + 1] += cell_start[c];
        }
        let mut items = vec![0u32; nodes.len()];
        let mut cursor = cell_start.clone();
        for (i, n) in nodes.iter().enumerate() {
            let (cx, cy) = Grid::cell_of(n.x, n.y);
            let c = cy * GRID_COLS + cx;
            items[cursor[c] as usize] = i as u32;
            cursor[c] += 1;
        }
        Grid { cell_start, items }
    }

    fn within(&self, nodes: &[Node], x: f64, y: f64, r: f64, r2: f64, out: &mut Vec<usize>) {
        out.clear();
        let (cx0, cy0) = Grid::cell_of(x - r, y - r);
        let (cx1, cy1) = Grid::cell_of(x + r, y + r);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                let c = cy * GRID_COLS + cx;
                let (a, b) = (self.cell_start[c] as usize, self.cell_start[c + 1] as usize);
                for &idx in &self.items[a..b] {
                    let node = &nodes[idx as usize];
                    let dx = node.x - x;
                    let dy = node.y - y;
                    if dx * dx + dy * dy <= r2 {
                        out.push(idx as usize);
                    }
                }
            }
        }
    }
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
            let grid = Grid::build(&prev);
            levels[z as usize] = cluster_level(&prev, &grid, z);
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

    /// Clusters/leaves inside a world-meter bbox at a deck zoom (mirrors `slotClusterIndex.getClusters`). Returned centroids are world meters.
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

    /// Leaf count.
    #[must_use]
    pub fn leaf_count(&self) -> usize {
        self.leaves_world.len()
    }
}

fn cluster_level(prev: &[Node], grid: &Grid, zoom: i32) -> Vec<Node> {
    let r = CLUSTER_RADIUS / (EXTENT * 2f64.powi(zoom));
    let r2 = r * r;
    let mut processed = vec![false; prev.len()];
    let mut next: Vec<Node> = Vec::new();
    let mut neighbors: Vec<usize> = Vec::new();

    for i in 0..prev.len() {
        if processed[i] {
            continue;
        }
        processed[i] = true;
        let p = prev[i];

        grid.within(prev, p.x, p.y, r, r2, &mut neighbors);

        let mut num_points = p.num_points;
        for &k in &neighbors {
            if k != i && !processed[k] {
                num_points += prev[k].num_points;
            }
        }

        if num_points > p.num_points {
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
#[path = "tests/cluster_tests.rs"]
mod tests;
