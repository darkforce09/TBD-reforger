//! Every way a BVH sidecar buffer is refused.
//!
//! **Role:** [`BvhParseError`], the error [`crate::bounding_volume_hierarchy::sidecar::BvhSidecar::parse`]
//! returns, one variant per structural check, with its message.
//! **Position:** under `bounding_volume_hierarchy`; returned by the sidecar parser, wrapped by the
//! crate's [`crate::Error`].
//! **Signals & state:** none; plain data.
//! **Invariants:** every variant carries the indices and counts that locate the defect; a variant
//! is never reused for a second check.

use crate::bounding_volume_hierarchy::flat_tree_build::MAX_PARSE_DEPTH;
use crate::bounding_volume_hierarchy::sidecar::{SIDECAR_VERSION, SIDECAR_VERSION_MIN};
use crate::bounding_volume_hierarchy::surface_kind::SurfaceKind;

/// Every way [`BvhSidecar::parse`](crate::bounding_volume_hierarchy::sidecar::BvhSidecar::parse) rejects bytes. The battery in `tests/triangle_tree_tests.rs` exercises one case per variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BvhParseError {
    /// The buffer is shorter than the 32-byte header.
    TooShort {
        /// Bytes in the buffer.
        len: usize,
    },

    /// The first four bytes are not the `TBVH` magic; carries the bytes read.
    BadMagic([u8; 4]),

    /// The header version lies outside [`SIDECAR_VERSION_MIN`]..=[`SIDECAR_VERSION`]; carries
    /// the version read.
    UnsupportedVersion(u32),

    /// A flag bit unknown to the header's version is set, or a reserved header byte (24..32) is
    /// not zero.
    NonZeroReserved,

    /// The header declares zero vertices, zero triangles or zero nodes.
    EmptyMesh,

    /// The buffer length differs from the length the header's counts and flags imply.
    LengthMismatch {
        /// Bytes the header implies.
        expected: u64,
        /// Bytes the buffer holds.
        actual: u64,
    },

    /// A vertex has a NaN or infinite coordinate.
    NonFiniteVert {
        /// Index of the offending vertex.
        vert: u32,
    },

    /// A triangle names a vertex index at or past the vertex count.
    TriIndexOutOfBounds {
        /// Index of the offending triangle.
        tri: u32,
    },

    /// A node's box minimum or maximum has a NaN or infinite component.
    NonFiniteNodeBound {
        /// Index of the offending node.
        node: u32,
    },

    /// A leaf node's `tri_order` range (first slot plus count) ends past the triangle count.
    LeafRangeOutOfBounds {
        /// Index of the offending node.
        node: u32,
    },

    /// An internal node's child pair lies past the node table or does not point strictly forward
    /// of the node itself.
    ChildOutOfBounds {
        /// Index of the offending node.
        node: u32,
    },

    /// The walk from the root reaches a node a second time (two parents share a child).
    NodeRevisited {
        /// Index of the offending node.
        node: u32,
    },

    /// Some nodes the header declares are unreachable from the root.
    OrphanNodes {
        /// Nodes reachable from the root.
        visited: u32,
        /// Nodes the header declares.
        nnodes: u32,
    },

    /// The walk from the root goes deeper than `MAX_PARSE_DEPTH` levels.
    TreeTooDeep,

    /// The leaves do not cover every `tri_order` slot exactly once (a slot is covered twice or
    /// some slots are left uncovered).
    LeafCoverageMismatch {
        /// `tri_order` slots the leaves cover.
        covered: u64,
        /// Triangles the header declares.
        ntris: u32,
    },

    /// A `tri_order` slot holds a triangle index at or past the triangle count.
    TriOrderOutOfBounds {
        /// Index of the offending `tri_order` slot.
        slot: u32,
    },

    /// `tri_order` names the same triangle twice, so it is not a permutation of the triangles.
    TriOrderNotPermutation {
        /// Index of the offending triangle.
        tri: u32,
    },

    /// A kinds byte this build cannot decode (see [`SurfaceKind::from_u8`]).
    UnknownKind {
        /// Index of the offending triangle.
        tri: u32,
        /// The kinds byte read.
        code: u8,
    },

    /// A non-zero byte in the kinds section's alignment padding.
    KindsPadding,
}

impl core::fmt::Display for BvhParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooShort { len } => write!(f, "sidecar too short: {len} bytes < 32-byte header"),
            Self::BadMagic(m) => write!(f, "bad magic {m:?} (want TBVH)"),
            Self::UnsupportedVersion(v) => write!(
                f,
                "unsupported sidecar version {v} (want {SIDECAR_VERSION_MIN}..={SIDECAR_VERSION})"
            ),
            Self::NonZeroReserved => write!(f, "reserved header words / flag bits are not zero"),
            Self::EmptyMesh => write!(f, "empty mesh (zero verts, tris, or nodes)"),
            Self::LengthMismatch { expected, actual } => {
                write!(f, "header implies {expected} bytes, file has {actual}")
            }
            Self::NonFiniteVert { vert } => write!(f, "vertex {vert} has a non-finite component"),
            Self::TriIndexOutOfBounds { tri } => {
                write!(f, "triangle {tri} indexes past the vertex table")
            }
            Self::NonFiniteNodeBound { node } => write!(f, "node {node} has a non-finite bound"),
            Self::LeafRangeOutOfBounds { node } => {
                write!(f, "leaf node {node} range exceeds tri_order")
            }
            Self::ChildOutOfBounds { node } => {
                write!(
                    f,
                    "internal node {node} children out of bounds or not forward-only"
                )
            }
            Self::NodeRevisited { node } => write!(f, "node {node} is reachable twice (diamond)"),
            Self::OrphanNodes { visited, nnodes } => {
                write!(
                    f,
                    "only {visited} of {nnodes} nodes reachable from the root"
                )
            }
            Self::TreeTooDeep => write!(f, "tree depth exceeds {MAX_PARSE_DEPTH}"),
            Self::LeafCoverageMismatch { covered, ntris } => {
                write!(
                    f,
                    "leaves do not tile tri_order exactly ({covered} of {ntris} slots)"
                )
            }
            Self::TriOrderOutOfBounds { slot } => {
                write!(f, "tri_order slot {slot} indexes past the triangle table")
            }
            Self::TriOrderNotPermutation { tri } => {
                write!(f, "tri_order repeats triangle {tri}")
            }
            Self::UnknownKind { tri, code } => {
                write!(
                    f,
                    "triangle {tri} has surface-kind code {code} (max {})",
                    SurfaceKind::MAX_CODE
                )
            }
            Self::KindsPadding => write!(f, "kinds section padding is not zero"),
        }
    }
}

impl std::error::Error for BvhParseError {}
