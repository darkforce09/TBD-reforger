//! Role: the GPU-free part of the frame vocabulary: draw-order, pipeline and bind-group ids,
//! damage tracking and the camera uniform.
//! Position: `frame` in `render_primitives`; the graphics engine's frame packet and batches
//! carry these types.
//! Signals & state: none here; `damage::RenderDamage` holds flags its owner mutates.
//! Invariants: **these types reference geometry and opaque keys ONLY.** A field named for a
//! thing in the world being drawn is a bug in the boundary, not a convenience.

/// The camera uniform.
pub mod camera;

/// Damage tracking: which frames need submitting at all.
pub mod damage;

/// Opaque draw-order, pipeline and bind-group keys.
pub mod ids;
