//! **Role:** the live memory budget of one page: the thread-local ledger, the accounting calls the
//! map host and the satellite loader make on it, the page's budget settings and heap size, and the
//! published snapshot and HUD tail.
//! **Position:** `live_memory_budget` in `map_asset_loading`, over `map_streaming_model`'s
//! budget model (the ledger, the rows and the satellite floor walk); the map host and the
//! satellite loader account here, the Mission Creator's frame pump reads the HUD tail.
//! **Signals & state:** the thread-local `LEDGER`, built on first use from
//! [`configured_budget_bytes`]; the `window.__t9386` global [`publish`] writes.
//! **Invariants:** every declared change and floor claim republishes the snapshot; the wasm32 and
//! native builds define [`configured_budget_bytes`], [`heap_bytes`] and [`publish`] side by side.

thread_local! {

    static LEDGER: RefCell<Ledger> = RefCell::new(Ledger::with_budget(configured_budget_bytes()));
}
use map_streaming_model::memory_budget::Ledger;
use std::cell::RefCell;
mod published_snapshot;

/// Re-export `published_snapshot::hud_suffix`.
pub use published_snapshot::hud_suffix;

/// Re-export `published_snapshot::publish`.
pub use published_snapshot::publish;
mod platform;

/// Re-export `platform::{configured_budget_bytes,heap_bytes}`.
pub use platform::{configured_budget_bytes, heap_bytes};
mod accounting;

/// Re-export `accounting::{claim_satellite_floor,heap_mark,hold,observe_since,release,set_held,with_ledger,}`.
pub use accounting::{
    claim_satellite_floor, heap_mark, hold, observe_since, release, set_held, with_ledger,
};
#[cfg(test)]
#[path = "tests/platform_tests.rs"]
mod tests;
