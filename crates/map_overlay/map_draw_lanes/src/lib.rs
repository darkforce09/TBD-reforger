//! The map's named draw lanes and its zoom legibility gates.
//!
//! **Role:** names every lane the map draws and ranks them into paint order
//! ([`lane_roles`]), and decides which world classes are legible at a zoom and which contour
//! interval a zoom shows ([`zoom_gates`]).
//! **Position:** map overlay tier 1, over `render_primitives` for the renderer's opaque lane key.
//! The overlay crates name the lanes their marks land on; the map engine's frame builder,
//! residency, draw buffers and editing lanes read both modules.
//! **Signals & state:** none; names, rank tables, constants and pure functions.
//! **Invariants:** paint order is the rank in `lane_roles::lane_order`; the browser wire ids
//! never change; the gates are legibility rules, never culling.

pub mod lane_roles;
pub mod prelude;
pub mod zoom_gates;
