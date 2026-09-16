//! Role: the gestures that must collapse into ONE undo step, and the host clock the grouping needs.
//! Position: `editor/state/operations` in the frontend editor adapter.
//! Signals & state: none of its own; the grouping is the engine's, over the hosted document.
//! Invariants: a gesture the operator experienced as one act undoes as one act — a delete that
//! spans comments, a connection cascade and slots, a paste that mints a layer first, an align
//! that moves many rows.

#![cfg(target_arch = "wasm32")]

use super::{entity, transform};
pub use website_map_engine::editing::batch::with_batch;
use website_map_engine::editing::tools::placement::AlignEdge;

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
    with_batch("align", || transform::align_selection(edge))
}

/// Wasm boot: feed `Date.now` into map-engine-core (no `wasm-bindgen` in that crate).
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn install_undo_gesture_clock() {
    website_map_engine::data::store::install_wasm_now(|| js_sys::Date::now() as u64);
}
