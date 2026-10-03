//! What a world line-of-sight query asks and answers.
//!
//! **Role:** declares the engine frame conversion ([`map_to_engine`]), the blocking policy of the
//! yes-or-no query ([`BlockPolicy`]), one crossing of a traced segment ([`WorldEvent`]) and how it
//! was decided ([`Fidelity`]), what the segment could not see ([`Coverage`]), and the verdict of a
//! sight line ([`WorldLos`], [`WorldVerdict`]).
//! **Position:** produced by the chunk walk of [`crate::raycast`] and the evaluation of
//! [`crate::sight_line`]; read by the map engine's line-of-sight tool, the debug world bench and
//! the developer tools' world check.
//! **Signals & state:** none; plain data.
//! **Invariants:** engine frame positions are `[x, y_up, z_north]` metres; a verdict built on a
//! proxy box or over a chunk that is not resident is `Provisional`, never `Clear` or `Blocked`.

use building_interiors::blueprint::sight_line::LosHit;
use interior_line_of_sight::compound_walk::Owner;
use spatial_indexes::bounding_volume_hierarchy::surface_kind::SurfaceKind;
use world_chunks::chunk_id::ChunkId;

/// Map frame `(x, y_north, elevation)` → engine frame `[x, y_up, z_north]`.
#[must_use]
pub fn map_to_engine(x: f64, y_north: f64, elev: f64) -> [f64; 3] {
    [x, elev, y_north]
}

/// How a crossing was decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fidelity {
    /// Real BLAS geometry.
    Exact,

    /// The catalogue proxy box (descriptor or BLAS not loaded yet).
    Proxy,
}

/// The verdict of a sight line through the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldVerdict {
    /// Nothing stands on the segment and every chunk it crosses is resident.
    Clear,

    /// Exact geometry stands on the segment.
    Blocked,

    /// Decided by a proxy box, or the segment crossed a chunk that is not resident.
    Provisional,
}

/// What counts as terminal for [`crate::world_occluder::WorldOccluder::blocked`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockPolicy {
    /// A glass pane stops the segment.
    pub glass_blocks: bool,

    /// A foliage volume stops the segment.
    pub foliage_blocks: bool,

    /// A proxy box (a prefab whose meshes are not loaded yet) stops the segment.
    pub proxy_blocks: bool,
}

impl BlockPolicy {
    /// The vision model: opaque only, a proxy box counts (nothing loaded is not nothing there).
    pub const VISION: Self = Self {
        glass_blocks: false,
        foliage_blocks: false,
        proxy_blocks: true,
    };
}

/// What the segment could and could not see.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Coverage {
    /// Chunk cells the segment crosses, resident or not.
    pub chunks_crossed: u32,

    /// Chunks on the segment that are not resident.
    pub chunks_missing: Vec<ChunkId>,

    /// Prefabs crossed as proxy boxes (distinct, first-crossed first).
    pub proxy_pids: Vec<u16>,

    /// BLAS paths those proxies are waiting for (known descriptors only).
    pub blas_pending: Vec<String>,
}

/// One crossing along `obs→tgt`.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldEvent {
    /// Parametric position on the segment (0 = observer, 1 = target).
    pub t: f64,

    /// The crossing point, engine frame.
    pub pos: [f64; 3],

    /// The surface kind of the crossed triangle (`Opaque` for a proxy box).
    pub kind: SurfaceKind,

    /// The chunk the crossed row is placed in.
    pub chunk: ChunkId,

    /// The crossed row's index in its chunk.
    pub row: u32,

    /// The catalogue prefab the row places.
    pub pid: u16,

    /// The descriptor instance crossed (`Shell` for the root record or a proxy).
    pub inner: Owner,

    /// The crossed triangle's index in its instance's mesh (0 for a proxy box).
    pub tri: u32,

    /// Whether real geometry or a proxy box decided the crossing.
    pub fidelity: Fidelity,
}

/// The verdict of [`crate::world_occluder::WorldOccluder::evaluate_los`].
#[derive(Clone, Debug, PartialEq)]
pub struct WorldLos {
    /// Clear, blocked or provisional.
    pub verdict: WorldVerdict,

    /// `1 − Π(1 − cᵢ)` over the pass-through events, `1` when blocked.
    pub concealment: f64,

    /// The opaque crossing that stopped the ray.
    pub blocker: Option<WorldEvent>,

    /// Every event along the ray, each named `pid:chunk:row[/inner id]`, sorted by `t`.
    pub hits: Vec<LosHit>,

    /// What the segment could not see.
    pub coverage: Coverage,
}
