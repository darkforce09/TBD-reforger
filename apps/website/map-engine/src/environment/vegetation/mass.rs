//! Role: mass.
//! Position: `environment/vegetation` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Canonical density iso value.
pub const DENSITY_ISO: f64 = 2.0;

/// Canonical canopy mass iso value.
pub const CANOPY_MASS_ISO: f64 = 2.0;

/// Forest mass fill colour rgb (`FOREST_FILL_RGB`, `forestMass.ts:34`).
pub const FOREST_FILL_RGB: [u8; 3] = [34, 120, 60];

/// Deck binary forest geometry (transferable). `fill_positions` = closed rings; `fill_start_indices` = per-ring start vertex index; `outline_segments` = iso `[x0,y0,x1,y1]` pairs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ForestMassGeometry {
    /// Fill positions.
    pub fill_positions: Vec<f32>,

    /// Fill start indices.
    pub fill_start_indices: Vec<u32>,

    /// Outline segments.
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

    outline_only: bool,
}

impl Builder {
    fn emit_ring(&mut self, pts: &[WalkPoint]) {
        let ring = dedupe_ring(pts);
        if ring.len() < 3 {
            return;
        }
        if !self.outline_only {
            self.start_indices.push(self.vertex_count);
            for p in &ring {
                self.positions.push(p.x as f32);
                self.positions.push(p.y as f32);
            }
            self.positions.push(ring[0].x as f32);
            self.positions.push(ring[0].y as f32);
            self.vertex_count += ring.len() as u32 + 1;
        }
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

    fn crossing_on(&self, a: &WalkCorner, b: &WalkCorner) -> WalkPoint {
        let t = (self.iso - a.v) / (b.v - a.v);
        WalkPoint {
            x: a.x + t * (b.x - a.x),
            y: a.y + t * (b.y - a.y),
            crossing: true,
        }
    }

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

/// Per-cell marching squares over a corner-count grid. Mirror of `forestMassFromCorners` (`forestMass.ts:124`).
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
        outline_only: false,
    };
    march_into(&mut b, corners, cols, rows, iso);
    ForestMassGeometry {
        fill_positions: b.positions,
        fill_start_indices: b.start_indices,
        outline_segments: b.segments,
    }
}

/// Forest outline segments from corners.
#[must_use]
pub fn forest_outline_segments_from_corners(
    corners: &[u16],
    cols: usize,
    rows: usize,
    origin_x: f64,
    origin_y: f64,
    cell_m: f64,
    iso: f64,
) -> Vec<f32> {
    let mut b = Builder {
        positions: Vec::new(),
        start_indices: Vec::new(),
        segments: Vec::new(),
        vertex_count: 0,
        origin_x,
        origin_y,
        cell_m,
        iso,
        outline_only: true,
    };
    march_into(&mut b, corners, cols, rows, iso);
    b.segments
}

fn march_into(b: &mut Builder, corners: &[u16], cols: usize, rows: usize, iso: f64) {
    if cols < 2 || rows < 2 {
        return;
    }
    for j in 0..rows - 1 {
        for i in 0..cols - 1 {
            let v00 = f64::from(corners[j * cols + i]);
            let v10 = f64::from(corners[j * cols + i + 1]);
            let v11 = f64::from(corners[(j + 1) * cols + i + 1]);
            let v01 = f64::from(corners[(j + 1) * cols + i]);

            if v00 < iso && v10 < iso && v11 < iso && v01 < iso {
                continue;
            }
            b.march_cell(i, j, v00, v10, v11, v01);
        }
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
#[path = "tests/mass_tests.rs"]
mod tests;
