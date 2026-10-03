//! One resident chunk as the world occluder holds it: its rows in the engine frame, their world
//! boxes and the box tree over them.
//!
//! **Role:** turns a streamed chunk's columns into [`WorldInstance`] rows ([`rows_of_chunk`]),
//! places each row by its prefab's current bounds, and keeps the chunk's boxes and box tree
//! ([`ChunkOccluder`]) rebuildable when a prefab's bounds change.
//! **Position:** built by [`crate::residency`] from `world_chunks`' decoded chunk; walked by
//! [`crate::raycast`]; read by the debug world bench through the occluder's read-outs.
//! **Signals & state:** a chunk occluder owns its rows, boxes and tree; the occluder rebuilds them.
//! **Invariants:** boxes stay parallel to rows; a row with no known bounds holds [`NO_BOX`] and
//! never crosses anything; the map frame `(x, y_north, z_up)` becomes `[x, z_up, y_north]`.

use geometry_primitives::axis_aligned_box::Bounds3;
use geometry_primitives::rigid_transform::Rigid;
use world_chunks::chunk_id::ChunkId;
use world_chunks::world_chunk::WorldChunk;

use crate::instance_box_tree::AabbTlas;

/// One chunk row: engine-frame position (`[x, y_up, z_north]`), `GetAngles()` degrees
/// (`[pitch, yaw, roll]`), uniform scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldInstance {
    /// The catalogue prefab the row places.
    pub pid: u16,

    /// The position, engine frame.
    pub pos: [f32; 3],

    /// `[pitch, yaw, roll]`, degrees.
    pub angles_deg: [f32; 3],

    /// The uniform scale.
    pub scale: f32,
}

impl WorldInstance {
    /// The rigid transform placing the prefab's object frame in the engine frame.
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
    /// The chunk's identifier.
    pub id: ChunkId,

    /// The chunk's grid column (`-1` when the identifier does not spell one).
    pub cx: i64,

    /// The chunk's grid row (`-1` when the identifier does not spell one).
    pub cy: i64,

    /// The chunk's rows, engine frame.
    pub rows: Vec<WorldInstance>,

    /// World AABB per row (`NO_BOX` for rows that carry no geometry yet / never).
    pub boxes: Vec<([f64; 3], [f64; 3])>,

    /// The box tree over `boxes`.
    pub tlas: AabbTlas,

    /// Rows whose box came from the catalogue proxy (the descriptor is not expanded yet).
    pub proxy_rows: u32,
}

impl ChunkOccluder {
    /// Builds chunk `id` from its parsed `chunk`. `bounds_of(pid)` yields the object-frame bounds
    /// a row's geometry currently has (`None` = no geometry, the row never crosses anything) and
    /// whether those bounds are a proxy.
    #[must_use]
    pub fn build(
        id: &ChunkId,
        chunk: &WorldChunk,
        bounds_of: &dyn Fn(u16) -> Option<(Bounds3, bool)>,
    ) -> Self {
        let mut parts = id.as_str().split('_');
        let cx = parts.next().and_then(|v| v.parse().ok()).unwrap_or(-1);
        let cy = parts.next().and_then(|v| v.parse().ok()).unwrap_or(-1);
        let rows = rows_of_chunk(chunk);
        let mut c = Self {
            id: id.clone(),
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

    /// Recomputes every row's world box and the box tree (after a descriptor expands or a BLAS is
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
