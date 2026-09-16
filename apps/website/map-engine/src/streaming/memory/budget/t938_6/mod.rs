//! Role: Module boundary for streaming/memory/budget/t938_6.
//! Position: `streaming/memory/budget/t938_6` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

fn everon() -> Vec<LevelBytes> {
    const DIMS: [u32; 14] = [
        12_800, 6_400, 3_200, 1_600, 800, 400, 200, 100, 50, 25, 12, 6, 3, 1,
    ];

    const COMPRESSED: [u64; 14] = [
        110_557_660,
        30_866_380,
        8_271_166,
        2_218_572,
        583_330,
        153_506,
        42_470,
        12_086,
        3_584,
        1_138,
        328,
        126,
        86,
        38,
    ];
    DIMS.iter()
        .zip(COMPRESSED)
        .map(|(&d, compressed)| LevelBytes {
            width: d,
            height: d,
            compressed,
        })
        .collect()
}

const MB_512: u64 = 512 * MIB;

const MB_1024: u64 = 1024 * MIB;

const DEM_METERS: u64 = 6_400 * 6_400 * 4;

mod cases_1;
