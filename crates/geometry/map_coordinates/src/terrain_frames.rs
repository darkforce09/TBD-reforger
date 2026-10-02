//! The fixed world facts of the served terrains.
//!
//! **Role:** the Everon anchor that GPU geometry is stored relative to, the view the map opens on,
//! Everon's pan bounds and the Arland centre.
//! **Position:** read by the map engine's render path (`frame`, the overlay lanes, the readback
//! checks), whose `world::scene` re-exports the Everon facts, and by whatever places a default
//! position on a terrain.
//! **Signals & state:** none; constants.
//! **Invariants:** world metres, x east and y north from the terrain's south-west corner; Everon is
//! the 12,800 m square and Arland the 4,096 m square, so each centre is half its side.

/// Scene anchor in world metres: the Everon terrain centre. Uploaded geometry is stored relative
/// to this point so f32 coordinates stay small (at most 6,400 m, an error far below a pixel at
/// every zoom; the bound is derived in the orthographic camera's `wgpu_clip_matrix` docs).
pub const ANCHOR: [f64; 2] = [6400.0, 6400.0];

/// The camera target the map opens on: the Everon terrain centre.
pub const INITIAL_TARGET: [f64; 2] = [6400.0, 6400.0];

/// The zoom the map opens on (log2 pixels per metre).
pub const INITIAL_ZOOM: f64 = -2.0;

/// Everon's world bounds in metres, `[min_x, min_y, max_x, max_y]`: the camera's pan clamp.
pub const EVERON_BOUNDS: [f64; 4] = [0.0, 0.0, 12_800.0, 12_800.0];

/// The Arland terrain centre in world metres (`4096² / 2`).
pub const ARLAND_CENTRE: [f64; 2] = [2048.0, 2048.0];
