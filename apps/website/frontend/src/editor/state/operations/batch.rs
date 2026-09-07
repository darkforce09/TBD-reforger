//! T-937.2 — explicit undo batches. `with_batch(label, f)` brackets `begin_group` /
//! `end_group` on the live [`MissionDocCore`] so paste, delete-selection and align
//! undo as one group even when they open several LOCAL transactions.
#![cfg(target_arch = "wasm32")]

use super::context::OPS_CTX;
use super::{entity, transform};

/// Run `f` inside one undo group labelled `label` (the label is for call-site intent;
/// yrs stack items carry no per-item meta here).
pub fn with_batch<F, R>(label: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let _ = label;
    begin_group();
    struct EndOnDrop;
    impl Drop for EndOnDrop {
        fn drop(&mut self) {
            end_group();
        }
    }
    let _guard = EndOnDrop;
    f()
}

fn begin_group() {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };
        core.begin_group();
    });
}

fn end_group() {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let mut d = ctx.doc.borrow_mut();
        let Some(core) = d.as_mut() else {
            return;
        };
        core.end_group();
    });
}

/// Facade wrapper — multi-txn delete (comments + connection cascade + slots) is one Ctrl+Z.
pub fn delete_selection() -> bool {
    with_batch("delete-selection", entity::delete_selection)
}

/// Facade wrapper — paste is one group even if layer mint + `paste_slots` split.
pub fn paste_at_cursor(cx: Option<f64>, cy: Option<f64>) -> bool {
    with_batch("paste", || entity::paste_at_cursor(cx, cy))
}

/// Facade wrapper — align the selection as one group.
pub fn align_selection(edge: crate::editor::tools::place_helpers::AlignEdge) -> bool {
    with_batch("align", || transform::align_selection(edge))
}

/// Wasm boot: feed `Date.now` into map-engine-core (no `wasm-bindgen` in that crate).
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn install_undo_gesture_clock() {
    map_engine_core::doc::install_wasm_now(|| js_sys::Date::now() as u64);
}
