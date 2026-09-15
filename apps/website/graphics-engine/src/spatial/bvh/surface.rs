//! Role: surface.
//! Position: `spatial/bvh` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Surface kind.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SurfaceKind {
    /// Wood, stone, brick, metal, door leaves, tree trunks: terminates visual LOS and blocks movement.
    Opaque = 0,

    /// Window panes, glass doors, display cases: visual LOS continues (a small optical concealment), movement is blocked.
    Glass = 1,

    /// Tree canopies, bushes, soft vegetation: soft cover — concealment accumulates with the depth traversed inside the volume.
    Foliage = 2,
}

impl SurfaceKind {
    /// Highest wire code the codec accepts.
    pub const MAX_CODE: u8 = 2;
}

impl SurfaceKind {
    /// Decode a wire code; `None` for anything this build does not know (the sidecar parser rejects such files rather than guessing).
    #[must_use]
    pub fn from_u8(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Opaque),
            1 => Some(Self::Glass),
            2 => Some(Self::Foliage),
            _ => None,
        }
    }
}

impl SurfaceKind {
    /// The wire code.
    #[must_use]
    pub fn code(self) -> u8 {
        self as u8
    }
}

impl SurfaceKind {
    /// Does a visual line of sight stop here? Only `Opaque` is terminal — glass and foliage are annotations the multi-hit walk continues through.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Opaque)
    }
}

/// The kind of triangle `tri` in a possibly-short table: the wrappers pass `&[]`, so a missing entry reads as `Opaque` (every triangle terminal — the pre-v2 semantics).
#[inline]
pub(crate) fn kind_of(kinds: &[SurfaceKind], tri: u32) -> SurfaceKind {
    kinds
        .get(tri as usize)
        .copied()
        .unwrap_or(SurfaceKind::Opaque)
}
