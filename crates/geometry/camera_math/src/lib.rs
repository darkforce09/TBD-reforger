//! The cameras' f64 arithmetic.
//!
//! **Role:** the orthographic map camera ([`ortho::state::OrthoCamera`]) that reproduces deck.gl's
//! orthographic viewport, the doll preview's orbit camera ([`orbit::projection`]) and the
//! gl-matrix 4×4 routines both build their matrices from ([`matrix4`]).
//! **Position:** geometry tier 1, depending on `map_coordinates` for JavaScript rounding. The map
//! engine's render engine, editing tools, readback checks and doll call it; the Mission Creator's
//! toolbelt, the map view and the debug benches read the orthographic camera.
//! **Signals & state:** the camera is plain owned state, moved by its caller's `&mut` calls; the
//! rest is pure functions.
//! **Invariants:** every product stays in f64 and a port keeps its JavaScript operand order, so
//! the orthographic camera matches deck.gl's goldens to zero ULP once its scale matches; a caller
//! casts to f32 only for the final render uniform.

mod dimensions;
pub mod matrix4;
pub mod orbit;
pub mod ortho;
pub mod prelude;
