//! Role: the progressive object pass over a viewshed raster and its merged palette.
//! Position: `editing/tools/line_of_sight` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: a cell the pass has not reached keeps its terrain colour; blocks are tested coarse-to-fine, nearest-first, under an explicit per-frame budget and an injected cell test.

use crate::spatial::los::terrain::viewshed::Viewshed;
use crate::spatial::los::terrain::viewshed::Visibility;
use crate::spatial::los::world::map_to_engine;

use super::terrain_verdict::EYE_HEIGHT_TARGET_M;
use super::wash_palette::{VIEWSHED_HIDDEN_RGBA, VIEWSHED_UNKNOWN_RGBA, VIEWSHED_VISIBLE_RGBA};

/// One glass pane's transmission (`1 − 0.05`). A readout calls a shot "through glass" rather than
/// "under canopy" when the combined concealment is explained by the panes alone.
pub const GLASS_CONCEALMENT: f64 = 0.95;

/// Per-frame budget for the object pass, in milliseconds of the host's frame.
pub const OBJECT_PASS_BUDGET_MS: f64 = 8.0;
/// The 8 m level runs only this close to the observer (an 8 m cell cannot resolve a tree beyond).
pub const OBJECT_FINE_RADIUS_M: f64 = 1000.0;
/// Coarse-to-fine block sizes in raster cells (8 m cells → 32 m, 16 m, 8 m).
pub const OBJECT_LEVELS: [usize; 3] = [4, 2, 1];
/// Re-upload the merged wash at most this often while the pass runs.
pub const OBJECT_UPLOAD_INTERVAL_MS: f64 = 100.0;

/// OBJECT-hidden cell: a warm dark wash, distinct from the terrain's cool dark so the operator
/// reads WHY ground is dead (a ridge vs a village).
pub const OBJECT_HIDDEN_RGBA: [u8; 4] = [78, 30, 24, 110];
/// Canopy-concealed cell base colour; alpha scales with the concealment (`30 + 60·c`).
pub const OBJECT_CANOPY_RGB: [u8; 3] = [34, 84, 36];
/// A cell a proxy box decided (geometry still loading): amber, clearly not final.
pub const OBJECT_PROVISIONAL_RGBA: [u8; 4] = [128, 92, 20, 84];

/// One raster cell's object verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectCell {
    /// Not reached yet (keeps the terrain colour).
    Untested,
    Clear,
    Hidden,
    /// Foliage / glass concealment, 0..=255.
    Concealed(u8),
    Provisional,
}

/// Alpha of a canopy-concealed cell for concealment `c` (0..=255 → `30 + 60·c/255`).
#[must_use]
pub fn canopy_alpha(c: u8) -> u8 {
    30 + ((60 * u32::from(c)) / 255) as u8
}

/// The merged palette: terrain first, then the object verdict over terrain-visible ground.
#[must_use]
pub fn object_cell_rgba(terrain: Visibility, obj: ObjectCell) -> [u8; 4] {
    match terrain {
        Visibility::Hidden => VIEWSHED_HIDDEN_RGBA,
        Visibility::Unknown => VIEWSHED_UNKNOWN_RGBA,
        Visibility::Visible => match obj {
            ObjectCell::Untested | ObjectCell::Clear => VIEWSHED_VISIBLE_RGBA,
            ObjectCell::Hidden => OBJECT_HIDDEN_RGBA,
            ObjectCell::Concealed(c) => [
                OBJECT_CANOPY_RGB[0],
                OBJECT_CANOPY_RGB[1],
                OBJECT_CANOPY_RGB[2],
                canopy_alpha(c),
            ],
            ObjectCell::Provisional => OBJECT_PROVISIONAL_RGBA,
        },
    }
}

/// The progressive object pass: coarse-to-fine over the terrain-VISIBLE cells of a viewshed,
/// nearest-first within a level, stepped under a per-frame budget by an injected cell test.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectPass {
    pub cols: usize,
    pub rows: usize,
    pub min_x: f64,
    pub min_y: f64,
    pub cell_m: f64,
    pub obs_x: f64,
    pub obs_y: f64,
    pub cells: Vec<ObjectCell>,
    /// Index into [`OBJECT_LEVELS`].
    pub level_idx: usize,
    /// Block anchors (cell index of the block's min corner) queued at the current level.
    pub queue: Vec<u32>,
    pub cursor: usize,
    pub generation: u32,
    pub done: bool,
    pub tested: u32,
}

impl ObjectPass {
    /// A pass over `vs` (its rect, cells and observer); the level-0 queue is built at once.
    #[must_use]
    pub fn new(vs: &Viewshed, generation: u32) -> Self {
        let cell_m = if vs.cols > 1 {
            (vs.max_x - vs.min_x) / (vs.cols - 1) as f64
        } else {
            8.0
        };
        let mut p = Self {
            cols: vs.cols,
            rows: vs.rows,
            min_x: vs.min_x,
            min_y: vs.min_y,
            cell_m,
            obs_x: vs.obs_x,
            obs_y: vs.obs_y,
            cells: vec![ObjectCell::Untested; vs.cols * vs.rows],
            level_idx: 0,
            queue: Vec::new(),
            cursor: 0,
            generation,
            done: false,
            tested: 0,
        };
        p.build_queue(vs);
        p
    }

    /// Block size (cells) at the current level.
    #[must_use]
    pub fn block(&self) -> usize {
        OBJECT_LEVELS[self.level_idx.min(OBJECT_LEVELS.len() - 1)]
    }

    /// The current level in metres.
    #[must_use]
    pub fn level_m(&self) -> f64 {
        self.block() as f64 * self.cell_m
    }

    /// World centre of cell `(col, row)` — the raster lattice `min + index · cell`.
    #[must_use]
    pub fn cell_center(&self, col: usize, row: usize) -> (f64, f64) {
        (
            self.min_x + col as f64 * self.cell_m,
            self.min_y + row as f64 * self.cell_m,
        )
    }

    fn build_queue(&mut self, vs: &Viewshed) {
        let b = self.block();
        let fine = b == 1;
        let mut q: Vec<(f64, u32)> = Vec::new();
        let mut row0 = 0usize;
        while row0 < self.rows {
            let mut col0 = 0usize;
            while col0 < self.cols {
                // A block is queued when any of its cells is terrain-visible.
                let mut any = false;
                'scan: for r in row0..(row0 + b).min(self.rows) {
                    for c in col0..(col0 + b).min(self.cols) {
                        if vs.at(c, r) == Visibility::Visible {
                            any = true;
                            break 'scan;
                        }
                    }
                }
                if any {
                    let (cx, cy) = self.block_center(col0, row0);
                    let d = (cx - self.obs_x).hypot(cy - self.obs_y);
                    if !fine || d <= OBJECT_FINE_RADIUS_M {
                        q.push((d, (row0 * self.cols + col0) as u32));
                    }
                }
                col0 += b;
            }
            row0 += b;
        }
        q.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        self.queue = q.into_iter().map(|(_, i)| i).collect();
        self.cursor = 0;
    }

    /// World centre of the block anchored at `(col0, row0)`.
    /// Re-queue every block that still holds a `Provisional` cell at the finest level, so a wash
    /// finished while BLAS were in flight retests those cells once they land (called by the tick
    /// when the occluder's residency signature changes). Returns the number of blocks queued;
    /// `0` leaves the pass untouched.
    pub fn requeue_provisional(&mut self, vs: &Viewshed) -> usize {
        let has_provisional = self.cells.contains(&ObjectCell::Provisional);
        if !has_provisional {
            return 0;
        }
        self.level_idx = OBJECT_LEVELS.len() - 1;
        let b = self.block();
        let mut q: Vec<(f64, u32)> = Vec::new();
        let mut row0 = 0usize;
        while row0 < self.rows {
            let mut col0 = 0usize;
            while col0 < self.cols {
                let mut any = false;
                'scan: for r in row0..(row0 + b).min(self.rows) {
                    for c in col0..(col0 + b).min(self.cols) {
                        if self.cells[r * self.cols + c] == ObjectCell::Provisional
                            && vs.at(c, r) == Visibility::Visible
                        {
                            any = true;
                            break 'scan;
                        }
                    }
                }
                if any {
                    let (cx, cy) = self.block_center(col0, row0);
                    let d = (cx - self.obs_x).hypot(cy - self.obs_y);
                    q.push((d, (row0 * self.cols + col0) as u32));
                }
                col0 += b;
            }
            row0 += b;
        }
        q.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        self.queue = q.into_iter().map(|(_, i)| i).collect();
        self.cursor = 0;
        self.done = self.queue.is_empty();
        self.queue.len()
    }

    fn block_center(&self, col0: usize, row0: usize) -> (f64, f64) {
        let b = self.block();
        let c = col0 + (b.min(self.cols - col0).saturating_sub(1)) / 2;
        let r = row0 + (b.min(self.rows - row0).saturating_sub(1)) / 2;
        self.cell_center(c, r)
    }

    /// `(tested cells, blocks queued at this level, cursor)` for a progress readout.
    #[must_use]
    pub fn progress(&self) -> (u32, usize, usize) {
        (self.tested, self.queue.len(), self.cursor)
    }

    /// Cell census `(clear, hidden, concealed, provisional, untested)` — the HUD / smoke-bridge
    /// summary of what the wash has decided so far.
    #[must_use]
    pub fn counts(&self) -> (u32, u32, u32, u32, u32) {
        let mut c = (0u32, 0u32, 0u32, 0u32, 0u32);
        for cell in &self.cells {
            match cell {
                ObjectCell::Clear => c.0 += 1,
                ObjectCell::Hidden => c.1 += 1,
                ObjectCell::Concealed(_) => c.2 += 1,
                ObjectCell::Provisional => c.3 += 1,
                ObjectCell::Untested => c.4 += 1,
            }
        }
        c
    }

    /// Run the pass until `budget_ms` of `now()` time has elapsed or it completes. `eye_z` is the
    /// observer's eye elevation; `ground(x, y)` the DEM; `test(obs, tgt)` the object test in the
    /// engine frame — `None` means the occluder was not reachable this frame (the pass waits,
    /// nothing is marked). Returns whether any cell changed.
    pub fn step(
        &mut self,
        vs: &Viewshed,
        eye_z: f64,
        ground: &dyn Fn(f64, f64) -> Option<f64>,
        test: &dyn Fn([f64; 3], [f64; 3]) -> Option<ObjectCell>,
        budget_ms: f64,
        now: &dyn Fn() -> f64,
    ) -> bool {
        if self.done {
            return false;
        }
        let start = now();
        let obs = map_to_engine(self.obs_x, self.obs_y, eye_z);
        let mut changed = false;
        loop {
            if self.cursor >= self.queue.len() {
                if self.level_idx + 1 < OBJECT_LEVELS.len() {
                    self.level_idx += 1;
                    self.build_queue(vs);
                    continue;
                }
                self.done = true;
                break;
            }
            if now() - start >= budget_ms {
                break;
            }
            let anchor = self.queue[self.cursor] as usize;
            let (col0, row0) = (anchor % self.cols, anchor / self.cols);
            let (cx, cy) = self.block_center(col0, row0);
            let verdict = match ground(cx, cy) {
                Some(g) => match test(obs, map_to_engine(cx, cy, g + EYE_HEIGHT_TARGET_M)) {
                    Some(v) => v,
                    None => break, // occluder not reachable this frame — retry the same block
                },
                None => ObjectCell::Untested,
            };
            self.cursor += 1;
            self.tested += 1;
            let b = self.block();
            for r in row0..(row0 + b).min(self.rows) {
                for c in col0..(col0 + b).min(self.cols) {
                    if vs.at(c, r) == Visibility::Visible {
                        let i = r * self.cols + c;
                        if self.cells[i] != verdict {
                            self.cells[i] = verdict;
                            changed = true;
                        }
                    }
                }
            }
        }
        changed
    }
}

/// The merged RGBA8 raster (`cols * rows * 4`, north row first like `encode_viewshed_rgba`).
#[must_use]
pub fn encode_viewshed_rgba_merged(vs: &Viewshed, pass: &ObjectPass) -> Vec<u8> {
    let mut out = Vec::with_capacity(vs.cols * vs.rows * 4);
    for r in (0..vs.rows).rev() {
        for c in 0..vs.cols {
            let obj = if pass.cols == vs.cols && pass.rows == vs.rows {
                pass.cells[r * vs.cols + c]
            } else {
                ObjectCell::Untested
            };
            out.extend_from_slice(&object_cell_rgba(vs.at(c, r), obj));
        }
    }
    out
}

#[cfg(test)]
#[path = "tests/object_wash.rs"]
mod tests;
