//! The orthographic map camera.
//!
//! **Role:** [`state::OrthoCamera`] with its planes and zoom band, and the method groups that
//! extend it: matrices and projection, unprojection, and the pan and zoom controls.
//! **Position:** the camera of the map engine's render engine, editing tools and readback checks,
//! and of the Mission Creator's toolbelt and the debug benches; builds on [`crate::matrix4`].
//! **Signals & state:** the camera's size, zoom, scale, target and bounds, owned by its caller.
//! **Invariants:** matches deck.gl's orthographic viewport bit for bit at integer zooms; only the
//! controls clamp, so a constructed camera holds exactly the state it was given.

mod controllers;
mod projection;
pub mod state;
mod unproject;
