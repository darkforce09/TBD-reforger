//! Role: Module boundary for camera/ortho/camera.
//! Position: `camera/ortho/camera` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::camera::ortho::state::FAR`.
pub use crate::camera::ortho::state::FAR;

/// Re-export `crate::camera::ortho::state::MAX_ZOOM`.
pub use crate::camera::ortho::state::MAX_ZOOM;

/// Re-export `crate::camera::ortho::state::MIN_ZOOM`.
pub use crate::camera::ortho::state::MIN_ZOOM;

/// Re-export `crate::camera::ortho::state::NEAR`.
pub use crate::camera::ortho::state::NEAR;

/// Re-export `crate::camera::ortho::state::OrthoCamera`.
pub use crate::camera::ortho::state::OrthoCamera;
