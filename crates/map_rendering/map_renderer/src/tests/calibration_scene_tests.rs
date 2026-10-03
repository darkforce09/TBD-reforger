//! **Role:** the byte-exact pin of the calibration scene.
//! **Position:** `src/tests` of the map renderer, mounted from `crate::calibration_scene`.
//! **Signals & state:** none.
//! **Invariants:** the calibration quads sit at fixed world metres of the 12.8 km Everon square,
//! so their bytes are pinned exactly.

use crate::calibration_scene::*;
use render_primitives::draw::instances::QuadInstance;

#[test]
fn calibration_instance_bytes_exact() {
    let instances = calibration_instances();
    let got: &[u8] = bytemuck::cast_slice(&instances);

    let mut expect = Vec::with_capacity(64);
    for v in [
        -100.0_f32, -100.0, 100.0, 100.0, 0.0, 1.0, 0.0, 1.0, 50.0, 50.0, 90.0, 90.0, 1.0, 0.0,
        0.0, 1.0,
    ] {
        expect.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(core::mem::size_of::<QuadInstance>(), 32);
    assert_eq!(got, expect.as_slice());
}
