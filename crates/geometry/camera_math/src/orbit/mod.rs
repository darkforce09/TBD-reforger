//! The doll preview's orbit camera.
//!
//! **Role:** the fixed orbit around the doll (`camera`) and its two view-projections
//! ([`projection`]), one for picking and one for the render uniform.
//! **Position:** called by the map engine's doll renderer, picking, scene model and readback
//! check; builds on [`crate::matrix4`].
//! **Signals & state:** none; constants and pure functions of the yaw and the viewport size.
//! **Invariants:** picking and rendering share one view-projection; the render form is the
//! picking form with the depth remap in front and nothing else.

mod camera;
pub mod projection;
