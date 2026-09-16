//! Role: the gestures that must collapse into ONE undo step, the prompt a bulk gesture asks before
//! it commits, and the host clock the grouping needs.
//! Position: `editor/state/operations` in the frontend editor adapter.
//! Signals & state: none of its own; the grouping is the engine's, over the hosted document.
//! Invariants: a gesture the operator experienced as one act undoes as one act — a delete that
//! spans comments, a connection cascade and slots, a paste that mints a layer first, an align
//! that moves many rows. The confirmation is the HOST's: the engine takes it as a closure and
//! calls it before committing, so a browser dialog never lives inside the engine.

#![cfg(target_arch = "wasm32")]

use super::entity;
pub use website_map_engine::editing::batch::with_batch;
use website_map_engine::editing::hosted_commands::selection_transform;
use website_map_engine::editing::tools::placement::{needs_confirm, AlignEdge};

/// Facade wrapper — multi-txn delete (comments + connection cascade + slots) is one Ctrl+Z.
pub fn delete_selection() -> bool {
    with_batch("delete-selection", entity::delete_selection)
}

/// Facade wrapper — paste is one group even if layer mint + `paste_slots` split.
pub fn paste_at_cursor(cx: Option<f64>, cy: Option<f64>) -> bool {
    with_batch("paste", || entity::paste_at_cursor(cx, cy))
}

/// Facade wrapper — align the selection as one group.
pub fn align_selection(edge: AlignEdge) -> bool {
    with_batch("align", || {
        selection_transform::align_selection(edge, confirm_bulk)
    })
}

/// Ask before a bulk move the operator cannot eyeball. Below the threshold it answers yes without
/// a dialog, so a two-entity align never interrupts anyone. The message names the undo escape
/// because the whole move is one group.
pub fn confirm_bulk(n: usize, verb: &str) -> bool {
    if !needs_confirm(n) {
        return true;
    }
    let msg = format!("This will {verb} {n} entities. Continue? (Ctrl+Z undoes the whole op.)");
    web_sys::window()
        .and_then(|w| w.confirm_with_message(&msg).ok())
        .unwrap_or(false)
}

/// Confirm for bulk ops that are still N undo steps (loadout apply/remove — no atomic batch yet).
pub(super) fn confirm_bulk_n_step(n: usize, verb: &str) -> bool {
    if !needs_confirm(n) {
        return true;
    }
    let msg = format!("This will {verb} {n} entities. Continue?");
    web_sys::window()
        .and_then(|w| w.confirm_with_message(&msg).ok())
        .unwrap_or(false)
}

/// Wasm boot: feed `Date.now` into the engine's document store.
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn install_undo_gesture_clock() {
    website_map_engine::data::store::install_wasm_now(|| js_sys::Date::now() as u64);
}
