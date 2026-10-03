//! **Role:** the live ledger's calls: hold, set, release, heap marks and the satellite floor
//! claim.
//! **Position:** `live_memory_budget` in `map_asset_loading`; the map host's boot and terrain
//! load and the satellite loader call them.
//! **Signals & state:** mutates the thread-local `LEDGER`; republishes the snapshot after each
//! declared change and floor claim.
//! **Invariants:** a floor claim reserves exactly the chosen level's cost.

use super::*;
use map_streaming_model::memory_budget::{Asset, FloorWalk, LevelBytes, floor_for_budget};

/// Run `f` against the live ledger.
pub fn with_ledger<R>(f: impl FnOnce(&mut Ledger) -> R) -> R {
    LEDGER.with(|l| f(&mut l.borrow_mut()))
}

/// Record `bytes` on the live budget unconditionally. See [`Ledger::hold`].
pub fn hold(a: Asset, bytes: u64) {
    with_ledger(|l| l.hold(a, bytes));
    publish();
}

/// Replace `a`'s held bytes on the live budget. See [`Ledger::set_held`].
pub fn set_held(a: Asset, bytes: u64) {
    with_ledger(|l| l.set_held(a, bytes));
    publish();
}

/// Release `bytes` held by `a`. See [`Ledger::release`].
pub fn release(a: Asset, bytes: u64) {
    with_ledger(|l| l.release(a, bytes));
    publish();
}

/// A linear-memory mark to hand back to [`observe_since`].
#[must_use]
pub fn heap_mark() -> u64 {
    heap_bytes()
}

/// Attribute the linear memory claimed since `mark` to `a`. Accumulates, so a loader that runs in several passes ends up with the total it caused rather than the largest single pass.
pub fn observe_since(a: Asset, mark: u64) {
    let now = heap_bytes();
    with_ledger(|l| l.add_growth(a, now.saturating_sub(mark)));
}

/// Choose the satellite's mip floor under the live budget and reserve what that level will cost.
#[must_use]
pub fn claim_satellite_floor(levels: &[LevelBytes], base: usize) -> FloorWalk {
    let walk = with_ledger(|l| {
        let walk = floor_for_budget(levels, base, l);

        l.reserve(Asset::Satellite, walk.bytes);
        l.set_satellite_floor(walk.base, walk.raised());
        walk
    });
    publish();
    walk
}
