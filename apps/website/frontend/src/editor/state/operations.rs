//! T-159.22 — the dock commands: outliner select / active layer, and palette drag-to-place.
//!
//! Peer of `mission_history` / `mission_commands`, and the same shape for the same reason: the doc /
//! engine / selection handles are `!Send` wasm-only `Rc`s that can't cross the
//! `#[cfg(target_arch = "wasm32")]` boundary into the native view shell, so the dock buttons reach
//! them through a `thread_local` [`OpsCtx`] set from `mission_editor::on_load` — exactly how the
//! Undo button reaches the undo stack.
//!
//! **Placement (T-180.1):** each `place_at` calls
//! [`map_engine_core::doc::place_character_under_side`] under [`OpsCtx::active_side`] (default
//! `BLUFOR`), which ensures `faction-{SIDE}`, mints a **new** squad, adds the slot as sole member /
//! leader, and files it under the resolved layer ([`ensure_layer`]). Layer mint stays LOCAL so it is
//! **undoable** — a boot-time layer would break the save/export gate (`smoke_save_export_editor`
//! uses the seed only). The ORBAT tree derives from squads (`build_orbat`). Seed slots still carry a
//! dangling `squadId` with no squad in the map — they list under Unfiled until placed-through.
//!
//! Consequence: the **first** place is multiple undo steps (layer + faction + squad + slot + leader
//! are separate core transactions); every later place under an existing layer/faction is fewer.
//!
//! **Borrow discipline** (the `mission_history` rule): each `pub fn` opens exactly one `OPS_CTX`
//! borrow; doc `borrow_mut`s are scoped so they drop before `mission_history::after_local_edit`
//! opens its read borrows.
#![cfg(target_arch = "wasm32")]

// T-934.7 — `operations.rs` is now a FAÇADE: the module body was split into the six submodules
// below (same-commit mechanical move; bodies unchanged). Every public item is re-exported so the
// `crate::editor::state::operations::X` paths (and the `editor_ops` aliases) keep working, and the
// whole tree stays wasm32-gated by the `#![cfg]` above. The Class-R/S source guards that pinned
// patterns in this file now `include_str!` the submodule that holds their pattern.
// NOTE (T-934.6): the `use crate::editor::state::history as mission_history;` alias lives in each
// submodule so the `mission_history::…` guard needles stay stable across the move.

pub mod attrs;
pub mod cargo;
pub mod compositions;
pub mod context;
pub mod entity;
pub mod transform;

pub use attrs::*;
pub use cargo::*;
pub use compositions::*;
pub use context::*;
pub use entity::*;
pub use transform::*;
