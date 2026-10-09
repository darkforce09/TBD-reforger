//! **Role:** the calibration scene: the two known quads the engine's first batch draws, so a
//! readback can check the frame path pixel for pixel.
//! **Position:** the map renderer; `boot.rs` uploads them as the calibration lane.
//! **Signals & state:** none; a pure function.
//! **Invariants:** the quads are measured in Everon metres relative to
//! `map_coordinates::terrain_frames::ANCHOR`; the instance layout belongs to `render_primitives`,
//! which never learns the anchor.

use render_primitives::draw::instances::QuadInstance;

/// The two calibration instances, anchor-relative: G, a green quad over world
/// `[6300,6300]…[6500,6500]` (relative `[-100,-100]…[100,100]`); R, a red quad over world
/// `[6450,6450]…[6490,6490]` (relative `[50,50]…[90,90]`), drawn after G.
#[must_use]
pub(crate) fn calibration_instances() -> [QuadInstance; 2] {
    [
        QuadInstance {
            min: [-100.0, -100.0],
            max: [100.0, 100.0],
            color: [0.0, 1.0, 0.0, 1.0],
        },
        QuadInstance {
            min: [50.0, 50.0],
            max: [90.0, 90.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
    ]
}
