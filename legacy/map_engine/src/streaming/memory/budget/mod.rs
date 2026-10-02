//! Role: Module boundary for streaming/memory/budget.
//! Position: `streaming/memory/budget` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

thread_local! {

    static LEDGER: RefCell<Ledger> = RefCell::new(Ledger::with_budget(configured_budget_bytes()));
}
use std::cell::RefCell;
mod model;

/// Re-export `model::{Asset,DEFAULT_BUDGET_MB,Decision,Entry,Ledger,MIB}`.
pub use model::{Asset, DEFAULT_BUDGET_MB, Decision, Entry, Ledger, MIB};
mod ledger;
mod stats;

/// Re-export `stats::hud_suffix`.
pub use stats::hud_suffix;

/// Re-export `stats::publish`.
#[cfg(not(target_arch = "wasm32"))]
pub use stats::publish;

/// Re-export `stats::publish`.
#[cfg(target_arch = "wasm32")]
pub use stats::publish;
mod satellite;

/// Re-export `satellite::{FloorWalk,LevelBytes,floor_for_budget,satellite_resident_bytes}`.
pub use satellite::{FloorWalk, LevelBytes, floor_for_budget, satellite_resident_bytes};
mod platform;

/// Re-export `platform::{configured_budget_bytes,heap_bytes}`.
#[cfg(not(target_arch = "wasm32"))]
pub use platform::{configured_budget_bytes, heap_bytes};

/// Re-export `platform::{configured_budget_bytes,heap_bytes}`.
#[cfg(target_arch = "wasm32")]
pub use platform::{configured_budget_bytes, heap_bytes};
mod accounting;

/// Re-export `accounting::{claim_satellite_floor,heap_mark,hold,observe_since,release,set_held,with_ledger,}`.
pub use accounting::{
    claim_satellite_floor, heap_mark, hold, observe_since, release, set_held, with_ledger,
};
#[cfg(test)]
mod t938_6;
