//! Role: Module boundary for spatial/bvh/tree.
//! Position: `spatial/bvh/tree` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::spatial::bvh::sidecar::BvhParseError`.
pub use crate::spatial::bvh::sidecar::BvhParseError;

/// Re-export `crate::spatial::bvh::sidecar::BvhSidecar`.
pub use crate::spatial::bvh::sidecar::BvhSidecar;

/// Re-export `crate::spatial::bvh::sidecar::FLAG_KINDS`.
pub use crate::spatial::bvh::sidecar::FLAG_KINDS;

/// Re-export `crate::spatial::bvh::sidecar::SIDECAR_MAGIC`.
pub use crate::spatial::bvh::sidecar::SIDECAR_MAGIC;

/// Re-export `crate::spatial::bvh::sidecar::SIDECAR_VERSION`.
pub use crate::spatial::bvh::sidecar::SIDECAR_VERSION;

/// Re-export `crate::spatial::bvh::sidecar::SIDECAR_VERSION_MIN`.
pub use crate::spatial::bvh::sidecar::SIDECAR_VERSION_MIN;

/// Re-export `crate::spatial::bvh::sidecar::emit_bytes`.
pub use crate::spatial::bvh::sidecar::emit_bytes;

/// Re-export `crate::spatial::bvh::sidecar::lift_verts`.
pub use crate::spatial::bvh::sidecar::lift_verts;

/// Re-export `crate::spatial::bvh::sidecar::quantize_verts`.
pub use crate::spatial::bvh::sidecar::quantize_verts;

/// Tests.
#[cfg(test)]
#[path = "../tests/tree.rs"]
pub(crate) mod tests;

/// Re-export `crate::spatial::bvh::node::cross`.
pub use crate::spatial::bvh::node::cross;

/// Re-export `crate::spatial::bvh::node::dot`.
pub use crate::spatial::bvh::node::dot;

/// Re-export `crate::spatial::bvh::node::segment_hits_tri`.
pub use crate::spatial::bvh::node::segment_hits_tri;

/// Re-export `crate::spatial::bvh::node::sub`.
pub use crate::spatial::bvh::node::sub;

/// Re-export `crate::spatial::bvh::surface::SurfaceKind`.
pub use crate::spatial::bvh::surface::SurfaceKind;

/// Re-export `crate::spatial::bvh::traversal::Bvh`.
pub use crate::spatial::bvh::traversal::Bvh;

/// Re-export `crate::spatial::bvh::traversal::Hit`.
pub use crate::spatial::bvh::traversal::Hit;
