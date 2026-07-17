//! T-159.21 — undo/redo for the Leptos Mission Creator, on the hosted `MissionDocCore` stack.
//!
//! There is no second stack: `MissionDocCore` owns a `yrs` `UndoManager` scoped to the LOCAL origin
//! (`store.rs`), so only user gestures are undoable — the INIT-origin seed / hydrate / IDB restore
//! are not. `capture_timeout_millis: 0` makes every transaction its own step, so one drag-move =
//! one undo. This module is the thin app-side driver, and it is the **only** path: the toolbar
//! buttons, the keyboard shortcuts, and the `__editorHistory` gate bridge all funnel through
//! [`undo`] / [`redo`], so the gate can't prove a path the user doesn't take.
//!
//! Peer of `mission_commands`: the doc/engine/selection handles are `!Send` wasm-only `Rc`s that
//! can't cross the `#[cfg(target_arch = "wasm32")]` boundary into the native view shell, so the
//! buttons reach them through a `thread_local` [`HistoryCtx`] set from `mission_editor::on_load`
//! rather than a hoisted handle.
//!
//! **Borrow discipline:** each `pub fn` opens exactly one `HISTORY_CTX` borrow and hands a
//! `&HistoryCtx` to the private helpers; a private helper never calls a `pub fn` (no re-entrancy).
//! `undo`/`redo` take `&mut MissionDocCore`, so their `borrow_mut` is scoped and dropped before
//! [`after_doc_change`] opens its read borrows.
#![cfg(target_arch = "wasm32")]

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use leptos::prelude::{RwSignal, Set};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use map_engine_core::doc::MissionDocCore;

use crate::mission_doc::DocHandle;
use crate::select_tool::{EngineHandle, SelectionHandle};

/// Everything a history command needs, shared from `mission_editor::on_load`. `doc` is the same
/// `Rc` the IDB restore swaps into, so undo/redo always see the live document. The four signals are
/// the HUD mirrors (see [`refresh_signals`]).
struct HistoryCtx {
    doc: DocHandle,
    engine: EngineHandle,
    selection: SelectionHandle,
    doc_ver: Rc<Cell<u32>>,
    mission_id: String,
    can_undo: RwSignal<bool>,
    can_redo: RwSignal<bool>,
    obj_count: RwSignal<usize>,
    sel_count: RwSignal<usize>,
    /// T-159.26 — unsaved-changes flag. Set by any doc-change edit; cleared by a successful Save
    /// (`mark_saved`) or a hydrate/conflict adopt (`set_dirty(false)`). Drives the TopCommandStrip
    /// unsaved indicator and the beforeunload guard.
    dirty: RwSignal<bool>,
}

thread_local! {
    static HISTORY_CTX: RefCell<Option<HistoryCtx>> = const { RefCell::new(None) };
}

/// Install the history context (once, from `on_load`, after the doc is seeded/registered).
#[allow(clippy::too_many_arguments)]
pub fn set_ctx(
    doc: DocHandle,
    engine: EngineHandle,
    selection: SelectionHandle,
    doc_ver: Rc<Cell<u32>>,
    mission_id: String,
    can_undo: RwSignal<bool>,
    can_redo: RwSignal<bool>,
    obj_count: RwSignal<usize>,
    sel_count: RwSignal<usize>,
    dirty: RwSignal<bool>,
) {
    HISTORY_CTX.with(|c| {
        *c.borrow_mut() = Some(HistoryCtx {
            doc,
            engine,
            selection,
            doc_ver,
            mission_id,
            can_undo,
            can_redo,
            obj_count,
            sel_count,
            dirty,
        });
    });
}

/// A clone of the live doc handle (the same `Rc` the IDB restore swaps into). For the conflict
/// resolver, which needs the doc but isn't called from `on_load`'s scope. `None` before mount.
pub fn doc_handle() -> Option<crate::mission_doc::DocHandle> {
    HISTORY_CTX.with(|c| c.borrow().as_ref().map(|ctx| ctx.doc.clone()))
}

/// Mark the doc clean (a successful Save) or force a dirty state. Used by `mission_commands` on a
/// 201 and by the hydrate/conflict adopt path.
pub fn set_dirty(value: bool) {
    HISTORY_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.dirty.set(value);
        }
    });
}

/// Undo the last LOCAL transaction; `true` if anything was undone. No-op (and `false`) on an empty
/// stack, so callers can fire it unconditionally.
pub fn undo() -> bool {
    HISTORY_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        // Scoped: `undo` needs `&mut`, `after_doc_change` needs `&` — the RefMut must be gone first.
        let did = {
            let mut d = ctx.doc.borrow_mut();
            d.as_mut().is_some_and(MissionDocCore::undo)
        };
        if did {
            after_doc_change(ctx);
        }
        did
    })
}

/// Redo the last undone transaction; `true` if anything was redone.
pub fn redo() -> bool {
    HISTORY_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let did = {
            let mut d = ctx.doc.borrow_mut();
            d.as_mut().is_some_and(MissionDocCore::redo)
        };
        if did {
            after_doc_change(ctx);
        }
        did
    })
}

/// Run the post-mutation sequence after a mutator the caller already committed (the T-159.19 drag
/// commit). Same path undo/redo take — see [`after_doc_change`].
pub fn after_local_edit() {
    HISTORY_CTX.with(|c| {
        let guard = c.borrow();
        if let Some(ctx) = guard.as_ref() {
            after_doc_change(ctx);
        }
    });
}

/// Re-read the HUD mirrors from the live doc + selection. For changes that don't touch the document
/// (click / marquee select) or that replace it wholesale (the IDB restore swap), where the glyph
/// rebind + persist of [`after_doc_change`] would be wrong or redundant.
pub fn refresh_hud() {
    HISTORY_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let obj = ctx
            .doc
            .borrow()
            .as_ref()
            .map_or(0, MissionDocCore::slot_count);
        refresh_signals(ctx, obj);
    });
}

/// The one post-document-change sequence: materialize → prune the selection → rebind the engine
/// glyphs + tint → bump `doc_ver` → schedule the persist → refresh the HUD.
///
/// Both the drag commit and undo/redo run it, so a slot set that changed under the app can never
/// leave a stale glyph cache or a selection pointing at dead ids — undoing an *add* deletes slots,
/// which is why the prune isn't optional even though today's only mutator is a move.
///
/// Equivalent to the inline T-159.19 commit it replaces: at a Move commit the selection already
/// equals the moved ids (`select_tool::compute_move_ids` returns the selection when the dragged slot
/// is in it, and the promotion assigns `selection = ids` when it isn't), so rebinding from the
/// selection binds the same set the old code bound from `ids`.
fn after_doc_change(ctx: &HistoryCtx) {
    let Some(soa) = ctx.doc.borrow().as_ref().map(MissionDocCore::materialize) else {
        return;
    };
    {
        let live: HashSet<&str> = soa.ids.iter().map(String::as_str).collect();
        ctx.selection
            .borrow_mut()
            .retain(|id| live.contains(id.as_str()));
    }
    let ids = ctx.selection.borrow().clone();
    if let Some(e) = ctx.engine.borrow_mut().as_mut() {
        e.set_drag(Vec::new(), 0.0, 0.0); // clear any live drag overlay
        e.slots_bind_soa(soa.ids.clone(), &soa.xy);
        e.set_selection(ids);
    }
    ctx.doc_ver.set(ctx.doc_ver.get().saturating_add(1));
    ctx.dirty.set(true); // T-159.26 — a committed edit is unsaved work
    crate::yrs_persist::schedule_edit_persist(ctx.doc.clone(), &ctx.mission_id);
    refresh_signals(ctx, soa.ids.len());
}

/// Push the doc/selection state onto the HUD signals. `MissionDocCore` has no change subscription,
/// so the Undo/Redo `disabled` state + the OBJ/SEL readouts are pull-mirrors refreshed at every
/// mutation site (React's `UndoController.subscribe` does the same job with a callback).
fn refresh_signals(ctx: &HistoryCtx, obj: usize) {
    let (cu, cr) = ctx
        .doc
        .borrow()
        .as_ref()
        .map_or((false, false), |c| (c.can_undo(), c.can_redo()));
    ctx.can_undo.set(cu);
    ctx.can_redo.set(cr);
    ctx.obj_count.set(obj);
    ctx.sel_count.set(ctx.selection.borrow().len());
    // T-159.22 — the dock mirrors (outliner tree + selected ids) are pull-mirrors on the same
    // footing as OBJ/SEL, so they refresh from the same single point: every mutation site funnels
    // here (place / drag-move / undo / redo / click / marquee / the IDB restore swap). `editor_ops`
    // holds its own ctx and borrows its own `Rc`s, so this can't reenter `HISTORY_CTX`.
    crate::editor_ops::refresh_docks();
}

/// True when focus is in a text-entry field, where Ctrl+Z means "undo my typing", not "undo the
/// mission" — the strip's semver `<input>` is on this very page. Mirrors the React host handler's
/// INPUT/SELECT/TEXTAREA/contentEditable guard, read off `activeElement` (the shortcut listens on
/// `window`, so the event target is the focused node or `<body>`).
pub fn in_editable_field() -> bool {
    let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
    else {
        return false;
    };
    if matches!(el.tag_name().as_str(), "INPUT" | "SELECT" | "TEXTAREA") {
        return true;
    }
    el.dyn_ref::<web_sys::HtmlElement>()
        .is_some_and(web_sys::HtmlElement::is_content_editable)
}

/// Install the window `keydown` shortcuts (spec C5): **Ctrl/Cmd+Z** undo, **Ctrl/Cmd+Shift+Z** or
/// **Ctrl+Y** redo.
///
/// Mirrors the React host handler (T-052): `code()` not `key()` (layout-independent — a modifier can
/// remap `key`), mod = ctrl **or** meta, Alt disqualifies, and `prevent_default` fires on a *match*
/// even when the stack is empty so the browser's own undo can never fight the document. Listens on
/// `window` (not the container) so the shortcut works before the map is focused. The closure leaks
/// like the editor's other listeners.
pub fn register_key_handler() {
    let Some(win) = web_sys::window() else {
        return;
    };
    let onkeydown =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
            if in_editable_field() {
                return;
            }
            if !(ev.ctrl_key() || ev.meta_key()) || ev.alt_key() {
                return;
            }
            match ev.code().as_str() {
                "KeyZ" if ev.shift_key() => {
                    redo();
                }
                "KeyZ" => {
                    undo();
                }
                "KeyY" if !ev.shift_key() => {
                    redo();
                }
                _ => return,
            }
            ev.prevent_default();
        });
    let _ = win.add_event_listener_with_callback("keydown", onkeydown.as_ref().unchecked_ref());
    onkeydown.forget();
}

/// Install `window.__editorHistory` — the read-only Class R gate bridge (peer of `__missionDoc` /
/// `__editorSelection`: a `js_sys::Object` of `.forget()`'d closures). Fields:
///   * `can_undo()` → bool
///   * `can_redo()` → bool
///   * `undo_depth()` → number — how many steps are stacked (T-159.22.1)
///
/// `undo_depth` is the *capture-side* half of the one-txn-one-step invariant: `can_undo` only says
/// "≥ 1", which is exactly why the T-159.22 granularity defect could hide behind a green gate. It
/// lets the smoke separate "two gestures pushed one item" (capture) from "one undo consumed two
/// items" (pop) without a debugger.
///
/// Read-only **by design**: the gate drives undo via the real keyboard shortcut and redo via a real
/// button click, so it proves the user's paths rather than a bridge-only one.
pub fn register_editor_history() {
    let obj = js_sys::Object::new();
    let can_undo_fn = Closure::wrap(Box::new(|| -> JsValue {
        JsValue::from_bool(HISTORY_CTX.with(|c| {
            c.borrow().as_ref().is_some_and(|ctx| {
                ctx.doc
                    .borrow()
                    .as_ref()
                    .is_some_and(MissionDocCore::can_undo)
            })
        }))
    }) as Box<dyn FnMut() -> JsValue>);
    let can_redo_fn = Closure::wrap(Box::new(|| -> JsValue {
        JsValue::from_bool(HISTORY_CTX.with(|c| {
            c.borrow().as_ref().is_some_and(|ctx| {
                ctx.doc
                    .borrow()
                    .as_ref()
                    .is_some_and(MissionDocCore::can_redo)
            })
        }))
    }) as Box<dyn FnMut() -> JsValue>);

    let undo_depth_fn = Closure::wrap(Box::new(|| -> JsValue {
        JsValue::from_f64(HISTORY_CTX.with(|c| {
            c.borrow().as_ref().map_or(0.0, |ctx| {
                ctx.doc
                    .borrow()
                    .as_ref()
                    .map_or(0.0, |d| d.undo_depth() as f64)
            })
        }))
    }) as Box<dyn FnMut() -> JsValue>);

    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("can_undo"), can_undo_fn.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("can_redo"), can_redo_fn.as_ref());
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("undo_depth"),
        undo_depth_fn.as_ref(),
    );
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorHistory"), &obj);
    }
    can_undo_fn.forget();
    can_redo_fn.forget();
    undo_depth_fn.forget();
}
