//! Role: satellite.
//! Position: `streaming/memory/budget` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

/// One satellite mip level, reduced to the two figures the budget cares about.
#[derive(Clone, Copy, Debug)]
pub struct LevelBytes {
    /// Level width in pixels.
    pub width: u32,

    /// Level height in pixels.
    pub height: u32,

    /// Compressed.
    pub compressed: u64,
}

impl LevelBytes {
    /// Decoded RGBA for this level: 4 B per pixel, the figure `tex_layer_write_rgba` is handed.
    #[must_use]
    pub fn rgba(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height) * 4
    }
}

/// Peak wasm-heap bytes `load_unified_full` holds when its base level is `base`.
#[must_use]
pub fn satellite_resident_bytes(levels: &[LevelBytes], base: usize) -> u64 {
    levels
        .iter()
        .skip(base)
        .fold(0u64, |acc, l| acc.saturating_add(l.rgba() + l.compressed))
}

/// The outcome of walking the mip ladder for a level the budget will accept.
#[derive(Clone, Debug)]
pub struct FloorWalk {
    /// The level the GPU limit chose — the ceiling this walk started from.
    pub requested: usize,

    /// The level to load from.
    pub base: usize,

    /// [`satellite_resident_bytes`] at that level.
    pub bytes: u64,

    /// Every level rejected on the way down, with what it would have cost and why it was refused.
    pub rejected: Vec<(usize, u64, Decision)>,
}

impl FloorWalk {
    /// How many levels of resolution the budget cost.
    #[must_use]
    pub fn raised(&self) -> u32 {
        u32::try_from(self.base.saturating_sub(self.requested)).unwrap_or(u32::MAX)
    }
}

/// Walk down from `base` until the ledger accepts a level, raising the floor by one each time.
#[must_use]
pub fn floor_for_budget(levels: &[LevelBytes], base: usize, ledger: &Ledger) -> FloorWalk {
    let mut rejected = Vec::new();
    let mut lvl = base;
    while lvl < levels.len() {
        let bytes = satellite_resident_bytes(levels, lvl);
        match ledger.decide(bytes) {
            Decision::Ok => {
                return FloorWalk {
                    requested: base,
                    base: lvl,
                    bytes,
                    rejected,
                };
            }
            d => rejected.push((lvl, bytes, d)),
        }
        lvl += 1;
    }
    let last = levels.len().saturating_sub(1);
    FloorWalk {
        requested: base,
        base: last,
        bytes: satellite_resident_bytes(levels, last),
        rejected,
    }
}
