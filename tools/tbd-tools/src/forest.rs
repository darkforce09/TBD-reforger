//! T-165.4 — Path B derived-hull forest regions (1:1 port of
//! `scripts/map-assets/lib/forest-regions.mjs`; t090_8_forest_vegetation_regions.md §Export).
//! Pure + deterministic (F6: same instances → same rings). Ship identity (F2, by construction):
//! `Σ regions.treeCount + unassignedTrees == trees.len()`.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{Value, json};

use crate::geometry::cell_of;

pub const REGION_CELL_M: f64 = 32.0;
pub const DENSITY_THRESHOLD: u32 = 2;
pub const MIN_COMPONENT_CELLS: usize = 8;
pub const DOMINANT_SHARE: f64 = 0.66;

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

pub struct Tree {
    pub x: f64,
    pub y: f64,
    pub class: String,
}

pub struct ForestDerivation {
    pub regions: Vec<Value>,
    pub unassigned_trees: u64,
    pub binned_tree_count: u64,
    pub dense_cell_count: u64,
    pub component_count: usize,
    pub kept_component_count: usize,
}

/// Derive forest regions from tree instances (see module header).
#[must_use]
pub fn derive_forest_regions(
    trees: &[Tree],
    world_size_m: f64,
    terrain_id: &str,
) -> ForestDerivation {
    let cell_m = REGION_CELL_M;
    let cells = (world_size_m / cell_m).round() as usize;

    // 1. bin
    let mut counts = vec![0u32; cells * cells];
    let cell_idx = |t: &Tree| -> usize {
        cell_of(t.y, cell_m, world_size_m) as usize * cells
            + cell_of(t.x, cell_m, world_size_m) as usize
    };
    for t in trees {
        counts[cell_idx(t)] += 1;
    }
    let mut dense = vec![false; cells * cells];
    let mut dense_cell_count = 0u64;
    for (k, &c) in counts.iter().enumerate() {
        if c >= DENSITY_THRESHOLD {
            dense[k] = true;
            dense_cell_count += 1;
        }
    }

    // 2. 8-connected components (deterministic row-major seed order; DFS stack like the .mjs).
    let mut comp = vec![-1i32; cells * cells];
    struct Component {
        cells: Vec<usize>,
        min_cx: usize,
        min_cy: usize,
        first_idx: usize,
        orig_id: usize,
    }
    let mut components: Vec<Component> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for seed in 0..dense.len() {
        if !dense[seed] || comp[seed] != -1 {
            continue;
        }
        let id = components.len() as i32;
        let mut member = Vec::new();
        let (mut min_cx, mut min_cy) = (usize::MAX, usize::MAX);
        stack.push(seed);
        comp[seed] = id;
        while let Some(k) = stack.pop() {
            member.push(k);
            let cy = k / cells;
            let cx = k - cy * cells;
            min_cx = min_cx.min(cx);
            min_cy = min_cy.min(cy);
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = cx as i64 + dx;
                    let ny = cy as i64 + dy;
                    if nx < 0 || nx >= cells as i64 || ny < 0 || ny >= cells as i64 {
                        continue;
                    }
                    let nk = ny as usize * cells + nx as usize;
                    if dense[nk] && comp[nk] == -1 {
                        comp[nk] = id;
                        stack.push(nk);
                    }
                }
            }
        }
        member.sort_unstable();
        components.push(Component {
            first_idx: member[0],
            cells: member,
            min_cx,
            min_cy,
            orig_id: id as usize,
        });
    }
    let component_count = components.len();

    // 3. keep + order; remap kept ids (discarded → -1).
    let mut kept: Vec<&Component> = components
        .iter()
        .filter(|c| c.cells.len() >= MIN_COMPONENT_CELLS)
        .collect();
    kept.sort_by(|a, b| {
        a.min_cy
            .cmp(&b.min_cy)
            .then(a.min_cx.cmp(&b.min_cx))
            .then(a.first_idx.cmp(&b.first_idx))
    });
    let region_id_by_orig: HashMap<usize, usize> = kept
        .iter()
        .enumerate()
        .map(|(i, c)| (c.orig_id, i))
        .collect();
    let mut region_of_cell = vec![-1i32; cells * cells];
    for (k, &id) in comp.iter().enumerate() {
        if id != -1
            && let Some(&r) = region_id_by_orig.get(&(id as usize))
        {
            region_of_cell[k] = r as i32;
        }
    }

    // 4. species tally + exact per-region counts.
    let mut species_tally: Vec<BTreeMap<String, u64>> =
        kept.iter().map(|_| BTreeMap::new()).collect();
    let mut tree_counts = vec![0u64; kept.len()];
    let mut unassigned_trees = 0u64;
    for t in trees {
        let r = region_of_cell[cell_idx(t)];
        if r == -1 {
            unassigned_trees += 1;
            continue;
        }
        tree_counts[r as usize] += 1;
        *species_tally[r as usize]
            .entry(t.class.clone())
            .or_insert(0) += 1;
    }

    // 5. rings + aggregates.
    let regions: Vec<Value> = kept
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let member_set: HashSet<usize> = c.cells.iter().copied().collect();
            let rings: Vec<Vec<[f64; 2]>> = trace_rings(&member_set, cells)
                .into_iter()
                .map(|ring| {
                    ring.into_iter()
                        .map(|(gx, gy)| [gx as f64 * cell_m, gy as f64 * cell_m])
                        .collect()
                })
                .collect();
            let mut tally: Vec<(&String, &u64)> = species_tally[i].iter().collect();
            tally.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            let total = tree_counts[i];
            let dominant = if let Some((cls, n)) = tally.first() {
                if total > 0 && (**n as f64) / (total as f64) >= DOMINANT_SHARE {
                    (*cls).clone()
                } else {
                    "mixed".to_string()
                }
            } else {
                "unknown".to_string()
            };
            let area_ha = round4((c.cells.len() as f64 * cell_m * cell_m) / 10_000.0);
            json!({
                "id": format!("forest-{terrain_id}-{:03}", i + 1),
                "kind": "forest",
                "polygon": rings,
                "treeCount": total,
                "dominantSpeciesClass": dominant,
                "densityPerHa": if area_ha > 0.0 { round4(total as f64 / area_ha) } else { 0.0 },
                "areaHa": area_ha,
                "coverType": "soft",
                "source": "derived-hull",
            })
        })
        .collect();

    ForestDerivation {
        regions,
        unassigned_trees,
        binned_tree_count: trees.len() as u64,
        dense_cell_count,
        component_count,
        kept_component_count: kept.len(),
    }
}

/* ── rectilinear boundary trace (interior on the LEFT; see .mjs header) ── */

#[derive(Clone)]
struct Edge {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    used: bool,
}

fn trace_rings(member_set: &HashSet<usize>, cells: usize) -> Vec<Vec<(i64, i64)>> {
    // startKey (x0,y0) → edges in deterministic insert order.
    let mut edges: BTreeMap<(i64, i64), Vec<Edge>> = BTreeMap::new();
    let add_edge =
        |edges: &mut BTreeMap<(i64, i64), Vec<Edge>>, x0: i64, y0: i64, x1: i64, y1: i64| {
            edges.entry((x0, y0)).or_default().push(Edge {
                x0,
                y0,
                x1,
                y1,
                used: false,
            });
        };
    let mut sorted: Vec<usize> = member_set.iter().copied().collect();
    sorted.sort_unstable();
    let n = cells as i64;
    for k in sorted {
        let y = (k / cells) as i64;
        let x = (k % cells) as i64;
        let has = |xx: i64, yy: i64| {
            xx >= 0
                && xx < n
                && yy >= 0
                && yy < n
                && member_set.contains(&(yy as usize * cells + xx as usize))
        };
        if y == 0 || !has(x, y - 1) {
            add_edge(&mut edges, x, y, x + 1, y); // bottom
        }
        if x == n - 1 || !has(x + 1, y) {
            add_edge(&mut edges, x + 1, y, x + 1, y + 1); // right
        }
        if y == n - 1 || !has(x, y + 1) {
            add_edge(&mut edges, x + 1, y + 1, x, y + 1); // top
        }
        if x == 0 || !has(x - 1, y) {
            add_edge(&mut edges, x, y + 1, x, y); // left
        }
    }

    // Deterministic start order: (y, x) ascending — matches the .mjs sort.
    let mut all_starts: Vec<(i64, i64)> = edges.keys().copied().collect();
    all_starts.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));

    let mut rings: Vec<Vec<(i64, i64)>> = Vec::new();
    for start_key in all_starts {
        let count = edges.get(&start_key).map(Vec::len).unwrap_or(0);
        for first_i in 0..count {
            {
                let list = edges.get(&start_key).unwrap();
                if list[first_i].used {
                    continue;
                }
            }
            let first = {
                let list = edges.get_mut(&start_key).unwrap();
                list[first_i].used = true;
                list[first_i].clone()
            };
            let mut verts: Vec<(i64, i64)> = vec![(first.x0, first.y0)];
            let mut cur = first.clone();
            let max_steps = cells * cells * 4;
            let mut guard = 0usize;
            loop {
                guard += 1;
                assert!(
                    guard <= max_steps,
                    "trace_rings: non-terminating walk (bug)"
                );
                let dx = cur.x1 - cur.x0;
                let dy = cur.y1 - cur.y0;
                if cur.x1 == first.x0 && cur.y1 == first.y0 {
                    break;
                }
                let next = {
                    let list = edges
                        .get_mut(&(cur.x1, cur.y1))
                        .map(std::mem::take)
                        .unwrap_or_default();
                    // rank: left(0), straight(1), right(2) relative to incoming (dx,dy).
                    let dir_rank = |e: &Edge| -> u8 {
                        let ex = e.x1 - e.x0;
                        let ey = e.y1 - e.y0;
                        if ex == -dy && ey == dx {
                            0
                        } else if ex == dx && ey == dy {
                            1
                        } else if ex == dy && ey == -dx {
                            2
                        } else {
                            3
                        }
                    };
                    let mut best: Option<usize> = None;
                    for (i, e) in list.iter().enumerate() {
                        if e.used {
                            continue;
                        }
                        if best.is_none_or(|b| dir_rank(e) < dir_rank(&list[b])) {
                            best = Some(i);
                        }
                    }
                    let idx = best.expect("trace_rings: dead-end walk (bug)");
                    let mut list = list;
                    list[idx].used = true;
                    let next = list[idx].clone();
                    edges.insert((cur.x1, cur.y1), list);
                    next
                };
                let ndx = next.x1 - next.x0;
                let ndy = next.y1 - next.y0;
                if ndx != dx || ndy != dy {
                    verts.push((next.x0, next.y0));
                }
                cur = next;
            }
            // Seam collinearity: drop the start vertex when it is not a turn.
            if verts.len() >= 3 {
                let (x0, y0) = verts[0];
                let (x1, y1) = verts[1];
                let (xl, yl) = verts[verts.len() - 1];
                let first_dir = ((x1 - x0).signum(), (y1 - y0).signum());
                let close_dir = ((x0 - xl).signum(), (y0 - yl).signum());
                if first_dir == close_dir {
                    verts.remove(0);
                }
            }
            rings.push(canonicalize_ring(verts));
        }
    }

    rings.sort_by(|a, b| {
        shoelace(b)
            .abs()
            .partial_cmp(&shoelace(a).abs())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| min_key(a).cmp(&min_key(b)))
    });
    rings
}

fn canonicalize_ring(verts: Vec<(i64, i64)>) -> Vec<(i64, i64)> {
    let mut best = 0usize;
    for i in 1..verts.len() {
        let (bx, by) = verts[best];
        let (x, y) = verts[i];
        if y < by || (y == by && x < bx) {
            best = i;
        }
    }
    let mut rotated: Vec<(i64, i64)> = verts[best..]
        .iter()
        .chain(verts[..best].iter())
        .copied()
        .collect();
    rotated.push(rotated[0]);
    rotated
}

fn shoelace(ring: &[(i64, i64)]) -> f64 {
    let mut s = 0i64;
    for i in 0..ring.len().saturating_sub(1) {
        s += ring[i].0 * ring[i + 1].1 - ring[i + 1].0 * ring[i].1;
    }
    s as f64 / 2.0
}

fn min_key(ring: &[(i64, i64)]) -> i64 {
    ring.iter()
        .map(|(x, y)| y * 1_000_000 + x)
        .min()
        .unwrap_or(i64::MAX)
}
