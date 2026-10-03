//! What a triangle is made of, as far as a line of sight cares.
//!
//! **Role:** [`SurfaceKind`] classifies each triangle of a BVH mesh as opaque, glass or foliage,
//! with its one-byte wire code in the sidecar's kinds section.
//! **Position:** under `bounding_volume_hierarchy`; written by the developer tools' surface
//! classification, stored by [`crate::bounding_volume_hierarchy::sidecar`], read by the filtered
//! queries of [`crate::bounding_volume_hierarchy::triangle_tree`] and the map engine's line of
//! sight.
//! **Signals & state:** none; plain data.
//! **Invariants:** the wire codes are 0, 1, 2 and never change; an unknown code decodes to `None`;
//! a triangle with no entry in a short kinds table reads as `Opaque`.

/// What one mesh triangle is made of, as a line of sight treats it; stored as a one-byte wire
/// code (0, 1, 2) in the sidecar's kinds section.
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
