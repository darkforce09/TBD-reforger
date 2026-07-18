//! Forest mass geometry — **Class R** (bit-identical to `forestMassFromCorners`, `forestMass.ts:124`).
//! Per-cell marching squares over a TBDD corner-count grid (16 cases via a perimeter walk); inside =
//! `count ≥ iso`. Saddles (opposite corners inside) split into two triangles when the centre is
//! below iso. Emits `SolidPolygonLayer` closed-ring fills + iso `LineLayer` outline segments.

/// Marching-squares iso threshold in trees per corner cell.
/// **Source of truth** (T-151.5.1): matches Path B region export floor (threshold 2).
/// Deck TS mirrors this const for Class R only — do not treat `forestMass.ts` as authority.
pub const DENSITY_ISO: f64 = 2.0;
/// T-176 A2 — marching-squares iso for the **8 m canopy-blurred** tree channel (`tools/tbd-tools`
/// `density::box_blur_corners`). The blurred corner value = tree count in the (2r+1)² canopy window,
/// so this stays a tree-count threshold: fill where ≥ `CANOPY_MASS_ISO` trees fall within the
/// window → the fill hugs real clusters, clearings stay open. Separate from `DENSITY_ISO` (the 32 m
/// per-cell floor / Path B mirror, pinned by `density_iso_is_two`). Tune with
/// `density::CANOPY_KERNEL_RADIUS_CELLS`.
pub const CANOPY_MASS_ISO: f64 = 2.0;
/// Forest mass fill colour rgb (`FOREST_FILL_RGB`, `forestMass.ts:34`).
pub const FOREST_FILL_RGB: [u8; 3] = [34, 120, 60];

/// Deck binary forest geometry (transferable). `fill_positions` = closed rings; `fill_start_indices`
/// = per-ring start vertex index; `outline_segments` = iso `[x0,y0,x1,y1]` pairs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ForestMassGeometry {
    pub fill_positions: Vec<f32>,
    pub fill_start_indices: Vec<u32>,
    pub outline_segments: Vec<f32>,
}

#[derive(Clone, Copy)]
struct WalkPoint {
    x: f64,
    y: f64,
    crossing: bool,
}

#[derive(Clone, Copy)]
struct WalkCorner {
    v: f64,
    inside: bool,
    x: f64,
    y: f64,
}

/// Dedupe consecutive identical points (merge crossing flags) and unclose the ring. Mirror of
/// `dedupeRing` (`forestMass.ts:94`).
fn dedupe_ring(pts: &[WalkPoint]) -> Vec<WalkPoint> {
    let mut ring: Vec<WalkPoint> = Vec::new();
    for p in pts {
        if let Some(last) = ring.last_mut()
            && last.x == p.x
            && last.y == p.y
        {
            last.crossing = last.crossing || p.crossing;
            continue;
        }
        ring.push(*p);
    }
    while ring.len() > 1 {
        let first = ring[0];
        let last = *ring.last().unwrap();
        if first.x != last.x || first.y != last.y {
            break;
        }
        ring[0].crossing = ring[0].crossing || last.crossing;
        ring.pop();
    }
    ring
}

struct Builder {
    positions: Vec<f32>,
    start_indices: Vec<u32>,
    segments: Vec<f32>,
    vertex_count: u32,
    origin_x: f64,
    origin_y: f64,
    cell_m: f64,
    iso: f64,
}

impl Builder {
    /// Mirror of the `emitRing` closure: dedupe, drop degenerate rings, push a closed ring, then
    /// extract crossing→crossing contour segments.
    fn emit_ring(&mut self, pts: &[WalkPoint]) {
        let ring = dedupe_ring(pts);
        if ring.len() < 3 {
            return;
        }
        self.start_indices.push(self.vertex_count);
        for p in &ring {
            self.positions.push(p.x as f32);
            self.positions.push(p.y as f32);
        }
        self.positions.push(ring[0].x as f32);
        self.positions.push(ring[0].y as f32);
        self.vertex_count += ring.len() as u32 + 1;
        let n = ring.len();
        for k in 0..n {
            let a = ring[k];
            let b = ring[(k + 1) % n];
            if a.crossing && b.crossing && (a.x != b.x || a.y != b.y) {
                self.segments.push(a.x as f32);
                self.segments.push(a.y as f32);
                self.segments.push(b.x as f32);
                self.segments.push(b.y as f32);
            }
        }
    }

    /// Mirror of the `crossingOn` closure.
    fn crossing_on(&self, a: &WalkCorner, b: &WalkCorner) -> WalkPoint {
        let t = (self.iso - a.v) / (b.v - a.v);
        WalkPoint {
            x: a.x + t * (b.x - a.x),
            y: a.y + t * (b.y - a.y),
            crossing: true,
        }
    }

    /// Mirror of the `emitSaddleTriangles` closure.
    fn emit_saddle_triangles(&mut self, wc: &[WalkCorner; 4]) {
        for k in 0..4 {
            let c = wc[k];
            if !c.inside {
                continue;
            }
            let prev = wc[(k + 3) % 4];
            let next = wc[(k + 1) % 4];
            self.emit_ring(&[
                WalkPoint {
                    x: c.x,
                    y: c.y,
                    crossing: false,
                },
                self.crossing_on(&c, &next),
                self.crossing_on(&c, &prev),
            ]);
        }
    }

    /// Mirror of the `marchCell` closure.
    fn march_cell(&mut self, i: usize, j: usize, v00: f64, v10: f64, v11: f64, v01: f64) {
        let in00 = v00 >= self.iso;
        let in10 = v10 >= self.iso;
        let in11 = v11 >= self.iso;
        let in01 = v01 >= self.iso;
        let x0 = self.origin_x + i as f64 * self.cell_m;
        let y0 = self.origin_y + j as f64 * self.cell_m;
        let x1 = x0 + self.cell_m;
        let y1 = y0 + self.cell_m;
        let wc = [
            WalkCorner {
                v: v00,
                inside: in00,
                x: x0,
                y: y0,
            },
            WalkCorner {
                v: v10,
                inside: in10,
                x: x1,
                y: y0,
            },
            WalkCorner {
                v: v11,
                inside: in11,
                x: x1,
                y: y1,
            },
            WalkCorner {
                v: v01,
                inside: in01,
                x: x0,
                y: y1,
            },
        ];
        let saddle = in00 == in11 && in10 == in01 && in00 != in10;
        if saddle && (v00 + v10 + v11 + v01) / 4.0 < self.iso {
            self.emit_saddle_triangles(&wc);
            return;
        }
        let mut walk: Vec<WalkPoint> = Vec::new();
        for k in 0..4 {
            let a = &wc[k];
            let b = &wc[(k + 1) % 4];
            if a.inside {
                walk.push(WalkPoint {
                    x: a.x,
                    y: a.y,
                    crossing: false,
                });
            }
            if a.inside != b.inside {
                walk.push(self.crossing_on(a, b));
            }
        }
        self.emit_ring(&walk);
    }
}

/// Per-cell marching squares over a corner-count grid. Mirror of `forestMassFromCorners`
/// (`forestMass.ts:124`).
#[must_use]
pub fn forest_mass_from_corners(
    corners: &[u16],
    cols: usize,
    rows: usize,
    origin_x: f64,
    origin_y: f64,
    cell_m: f64,
    iso: f64,
) -> ForestMassGeometry {
    let mut b = Builder {
        positions: Vec::new(),
        start_indices: Vec::new(),
        segments: Vec::new(),
        vertex_count: 0,
        origin_x,
        origin_y,
        cell_m,
        iso,
    };
    if cols < 2 || rows < 2 {
        return ForestMassGeometry::default();
    }
    for j in 0..rows - 1 {
        for i in 0..cols - 1 {
            let v00 = f64::from(corners[j * cols + i]);
            let v10 = f64::from(corners[j * cols + i + 1]);
            let v11 = f64::from(corners[(j + 1) * cols + i + 1]);
            let v01 = f64::from(corners[(j + 1) * cols + i]);
            // Case 0 fast path — most of a 16×16 chunk is not forest boundary.
            if v00 < iso && v10 < iso && v11 < iso && v01 < iso {
                continue;
            }
            b.march_cell(i, j, v00, v10, v11, v01);
        }
    }
    ForestMassGeometry {
        fill_positions: b.positions,
        fill_start_indices: b.start_indices,
        outline_segments: b.segments,
    }
}

/// N3 forest fill-α ladder (`forestFillAlpha`, `forestMass.ts:227`).
#[must_use]
pub fn forest_fill_alpha(deck_zoom: f64) -> f64 {
    if deck_zoom < -2.5 {
        0.45
    } else if deck_zoom <= 1.0 {
        0.35
    } else if deck_zoom <= 3.0 {
        0.12
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_below_iso_is_empty() {
        let corners = vec![0u16; 9];
        let g = forest_mass_from_corners(&corners, 3, 3, 0.0, 0.0, 32.0, DENSITY_ISO);
        assert!(g.fill_positions.is_empty());
        assert!(g.fill_start_indices.is_empty());
        assert!(g.outline_segments.is_empty());
    }

    #[test]
    fn full_cell_is_a_closed_quad_with_no_contour() {
        // All four corners ≥ iso → case 15: 4 corners + 1 closing vertex, no crossings.
        let corners = vec![5u16; 4];
        let g = forest_mass_from_corners(&corners, 2, 2, 0.0, 0.0, 32.0, DENSITY_ISO);
        assert_eq!(g.fill_start_indices.len(), 1);
        assert_eq!(g.fill_positions.len(), 5 * 2); // 5 vertices × (x,y)
        assert!(g.outline_segments.is_empty());
    }

    #[test]
    fn single_inside_corner_emits_triangle_with_one_contour_edge() {
        // v00 inside, others out → triangle (corner + 2 crossings) + one iso segment.
        let corners = vec![5u16, 0, 0, 0]; // BL, BR, TL... row-major (j*cols+i): [c00,c10,c01,c11]?
        // layout row-major 2×2: idx0=(0,0)=v00, idx1=(1,0)=v10, idx2=(0,1)=v01, idx3=(1,1)=v11
        let g = forest_mass_from_corners(&corners, 2, 2, 0.0, 0.0, 32.0, DENSITY_ISO);
        assert_eq!(g.fill_start_indices.len(), 1);
        // triangle: 3 verts + 1 close = 4 → 8 floats
        assert_eq!(g.fill_positions.len(), 8);
        // one crossing→crossing contour edge
        assert_eq!(g.outline_segments.len(), 4);
    }

    #[test]
    fn alpha_ladder() {
        assert_eq!(forest_fill_alpha(-3.0), 0.45);
        assert_eq!(forest_fill_alpha(0.0), 0.35);
        assert_eq!(forest_fill_alpha(2.0), 0.12);
        assert_eq!(forest_fill_alpha(9.0), 0.0);
        assert_eq!(FOREST_FILL_RGB, [34, 120, 60]);
    }

    #[test]
    fn density_iso_is_two() {
        assert!((DENSITY_ISO - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn count_one_corner_empty_at_default_iso() {
        // Path B floor: lone count-1 corners are not forest mass.
        let corners = vec![1u16, 0, 0, 0];
        let g = forest_mass_from_corners(&corners, 2, 2, 0.0, 0.0, 32.0, DENSITY_ISO);
        assert!(g.fill_positions.is_empty());
        assert!(g.outline_segments.is_empty());
    }
}
