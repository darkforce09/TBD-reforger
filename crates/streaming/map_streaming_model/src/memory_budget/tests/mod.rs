//! Tests of the memory budget model, run natively against the Everon satellite mip ladder.
//!
//! **Role:** the fixtures the cases share: the Everon mip ladder with its compressed level
//! sizes, two budgets and the DEM's metres size.
//! **Position:** test builds of `memory_budget` only.
//! **Signals & state:** none; each case builds its own ledger.
//! **Invariants:** the ladder's figures are the shipped Everon export's.

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

mod ledger_floor_walk_and_hud_cases;
