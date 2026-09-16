//! Role: Module boundary for doll/scene/model.
//! Position: `doll/scene/model` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::doll::scene::instances::CLEAR_COLOR`.
pub use crate::doll::scene::instances::CLEAR_COLOR;

/// Re-export `crate::doll::scene::instances::DECOR`.
pub use crate::doll::scene::instances::DECOR;

/// Re-export `crate::doll::scene::instances::DollInstance`.
pub use crate::doll::scene::instances::DollInstance;

/// Re-export `crate::doll::scene::instances::MeshKind`.
pub use crate::doll::scene::instances::MeshKind;

/// Re-export `crate::doll::scene::instances::REGION_KEYS`.
pub use crate::doll::scene::instances::REGION_KEYS;

/// Re-export `crate::doll::scene::instances::STATE_ACTIVE`.
pub use crate::doll::scene::instances::STATE_ACTIVE;

/// Re-export `crate::doll::scene::instances::STATE_EMPTY`.
pub use crate::doll::scene::instances::STATE_EMPTY;

/// Re-export `crate::doll::scene::instances::STATE_EQUIPPED`.
pub use crate::doll::scene::instances::STATE_EQUIPPED;

/// Re-export `crate::doll::scene::instances::decor_color`.
pub use crate::doll::scene::instances::decor_color;

/// Re-export `crate::doll::scene::instances::instances`.
pub use crate::doll::scene::instances::instances;

/// Re-export `crate::doll::scene::instances::state_color`.
pub use crate::doll::scene::instances::state_color;

/// Re-export `crate::camera::orbit::projection::view_proj_gl`.
pub use crate::camera::orbit::projection::view_proj_gl;

/// Re-export `crate::camera::orbit::projection::view_proj_wgpu`.
pub use crate::camera::orbit::projection::view_proj_wgpu;

/// Re-export `crate::doll::interaction::picking::anchor_px`.
pub use crate::doll::interaction::picking::anchor_px;

/// Re-export `crate::doll::interaction::picking::anchor_world`.
pub use crate::doll::interaction::picking::anchor_world;

/// Re-export `crate::doll::interaction::picking::pick`.
pub use crate::doll::interaction::picking::pick;

/// Re-export `crate::doll::scene::mesh::mesh_cube`.
pub use crate::doll::scene::mesh::mesh_cube;

/// Re-export `crate::doll::scene::mesh::mesh_cylinder`.
pub use crate::doll::scene::mesh::mesh_cylinder;
#[cfg(test)]
mod tests;
