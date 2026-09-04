//! T-090.12.3 — the placed rows of one chunk as the occluder sees them: a compact transform per
//! row (32 B) in the engine frame, the world AABB of whatever geometry the row currently has
//! (exact descriptor bounds once expanded, the catalogue proxy box until then, none for a
//! `blocks: false` prefab), and the [`AabbTlas`] over those boxes.

use crate::geometry::rigid::Rigid;
use crate::world::chunk::WorldChunk;
use crate::world::occluder::descriptor::Bounds3;

use super::tlas::AabbTlas;

/// One chunk row: engine-frame position (`[x, y_up, z_north]`), `GetAngles()` degrees
/// (`[pitch, yaw, roll]`), uniform scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldInstance {
    pub pid: u16,
    pub pos: [f32; 3],
    pub angles_deg: [f32; 3],
    pub scale: f32,
}

impl WorldInstance {
    /// Entity → world transform (`Rigid::from_enfusion`, the T-090.11.3 pinned composition).
    #[must_use]
    pub fn rigid(&self) -> Rigid {
        Rigid::from_enfusion(
            [
                f64::from(self.pos[0]),
                f64::from(self.pos[1]),
                f64::from(self.pos[2]),
            ],
            [
                f64::from(self.angles_deg[0]),
                f64::from(self.angles_deg[1]),
                f64::from(self.angles_deg[2]),
            ],
            f64::from(self.scale),
        )
    }
}

/// The map-frame chunk row → engine-frame row (`x, y_north, z_up` → `[x, z_up, y_north]`).
#[must_use]
pub fn rows_of_chunk(chunk: &WorldChunk) -> Vec<WorldInstance> {
    let n = chunk.count as usize;
    let get = |v: &Vec<f32>, i: usize, default: f32| v.get(i).copied().unwrap_or(default);
    (0..n)
        .map(|i| WorldInstance {
            pid: chunk.prefab_idx[i],
            pos: [
                chunk.positions[2 * i],
                get(&chunk.z, i, 0.0),
                chunk.positions[2 * i + 1],
            ],
            angles_deg: [
                get(&chunk.pitch, i, 0.0),
                get(&chunk.rotations, i, 0.0),
                get(&chunk.roll, i, 0.0),
            ],
            scale: get(&chunk.scale, i, 1.0),
        })
        .collect()
}

/// An "absent" box — never crossed by anything.
pub const NO_BOX: ([f64; 3], [f64; 3]) = ([1.0; 3], [-1.0; 3]);

/// One resident chunk's rows, boxes and TLAS.
#[derive(Clone, Debug)]
pub struct ChunkOccluder {
    pub id: String,
    pub cx: i64,
    pub cy: i64,
    pub rows: Vec<WorldInstance>,
    /// World AABB per row (`NO_BOX` for rows that carry no geometry yet / never).
    pub boxes: Vec<([f64; 3], [f64; 3])>,
    pub tlas: AabbTlas,
    /// Rows whose box came from the catalogue proxy (the descriptor is not expanded yet).
    pub proxy_rows: u32,
}

impl ChunkOccluder {
    /// Build from a parsed chunk. `bounds_of(pid)` yields the object-frame bounds a row's
    /// geometry currently has (`None` = no geometry, the row never crosses anything) and whether
    /// those bounds are a proxy.
    #[must_use]
    pub fn build(
        id: &str,
        chunk: &WorldChunk,
        bounds_of: &dyn Fn(u16) -> Option<(Bounds3, bool)>,
    ) -> Self {
        let mut parts = id.split('_');
        let cx = parts.next().and_then(|v| v.parse().ok()).unwrap_or(-1);
        let cy = parts.next().and_then(|v| v.parse().ok()).unwrap_or(-1);
        let rows = rows_of_chunk(chunk);
        let mut c = Self {
            id: id.to_string(),
            cx,
            cy,
            rows,
            boxes: Vec::new(),
            tlas: AabbTlas::default(),
            proxy_rows: 0,
        };
        c.rebuild(bounds_of);
        c
    }

    /// Recompute every row's world box and the TLAS (after a descriptor expands or a BLAS is
    /// evicted).
    pub fn rebuild(&mut self, bounds_of: &dyn Fn(u16) -> Option<(Bounds3, bool)>) {
        self.proxy_rows = 0;
        self.boxes = self
            .rows
            .iter()
            .map(|r| match bounds_of(r.pid) {
                Some((b, proxy)) => {
                    if proxy {
                        self.proxy_rows += 1;
                    }
                    r.rigid().aabb_of(b.min, b.max)
                }
                None => NO_BOX,
            })
            .collect();
        self.tlas = AabbTlas::build(&self.boxes);
    }

    /// Heap bytes: rows + boxes + tree.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.rows.len() * core::mem::size_of::<WorldInstance>()
            + self.boxes.len() * core::mem::size_of::<([f64; 3], [f64; 3])>()
            + self.tlas.bytes()
    }
}
