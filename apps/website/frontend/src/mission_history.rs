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
//!
//! **T-189 — the unsaved-work guard.** [`register_unload_guard`] installs the `beforeunload`
//! listener that warns before a tab close / reload discards unsaved editor work. It lives here
//! because [`HistoryCtx::dirty`] is the flag it reads. It is the ONE editor listener that is not
//! `.forget()`ed: see [`UNLOAD_GUARD`] for how it is torn down without `on_cleanup` ever holding a
//! `!Send` value.
#![cfg(target_arch = "wasm32")]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use leptos::prelude::{GetUntracked, RwSignal, Set};
use map_engine_core::doc::{MissionDocCore, SlotSoa};
use map_engine_core::squad_links::build_squad_link_segments;
// T-596 — `role_id::SQUAD_LINKS` is imported, not a hand-copied `const ROLE_SQUAD_LINKS: u32 = 9`:
// the copy had no compile-time link to `lane_role_from_u32`, so a renumber would have drawn the
// squad-leader hairlines into whatever lane 9 became rather than failing the build.
use map_engine_render::draw_order::role_id;
use map_engine_render::RenderEngine;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::editor::tools::select_tool::{EngineHandle, SelectionHandle};
use crate::mission_doc::DocHandle;

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
    /// T-159.26 — unsaved-changes flag. Set by any doc-change edit (and by the T-189 IDB-restore
    /// path, whose blob IS unsaved work); cleared by a successful Save (`mission_commands`) or a
    /// hydrate/conflict adopt (`set_dirty(false)`). Drives the TopCommandStrip unsaved indicator and
    /// — since T-189 — the `beforeunload` guard in [`register_unload_guard`].
    dirty: RwSignal<bool>,
    /// T-380 — the boot gate on the debounced persist writer. The SAME `Rc<Cell<bool>>`
    /// `mission_editor::on_load` flips once the IDB restore **and** the server hydrate have both
    /// awaited, so `false` means "the live doc may still be the 8-slot fixture seed". Read by
    /// [`after_doc_change`]; see there for why the edit's data is not lost with its persist.
    ///
    /// Per-mount by construction: the handle is created fresh in `on_load` and reaches both this ctx
    /// and the boot task, so a boot task left in flight by a route-leave arms its OWN dead `Cell`,
    /// never the next mount's.
    restore_settled: Rc<Cell<bool>>,
}

/// T-189 — the parked `beforeunload` closure's type. A named alias only so the `thread_local`
/// below reads as one thing (and clears clippy's `type_complexity`).
type UnloadClosure = Closure<dyn FnMut(web_sys::Event)>;

thread_local! {
    static HISTORY_CTX: RefCell<Option<HistoryCtx>> = const { RefCell::new(None) };
    /// T-189 — the live `beforeunload` closure, parked here instead of `.forget()`ed.
    ///
    /// This is the workaround for the documented `sse.rs` trap: `on_cleanup` is `Send + Sync`-bound
    /// and a `wasm_bindgen::Closure` is `!Send`, so the cleanup can never OWN the handle. It can,
    /// however, be a zero-capture fn pointer (`Send + Sync`) that calls
    /// [`unregister_unload_guard`], which reaches this `thread_local` — wasm is single-threaded, so
    /// the cleanup always runs on the thread that installed the closure. Result: a guard that is
    /// genuinely removed on route-leave, not one more leaked listener.
    static UNLOAD_GUARD: RefCell<Option<UnloadClosure>> = const { RefCell::new(None) };
}

/// Legacy `returnValue` payload for the `beforeunload` prompt. Every current browser ignores the
/// text and shows its own generic "Leave site?" copy — the string only has to be NON-EMPTY, which is
/// what pre-119 Chromium/WebKit key the prompt off (modern engines key off `preventDefault`).
const UNSAVED_PROMPT: &str = "You have unsaved mission changes.";

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
    restore_settled: Rc<Cell<bool>>,
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
            restore_settled,
        });
    });
}

/// A clone of the live doc handle (the same `Rc` the IDB restore swaps into). For the conflict
/// resolver, which needs the doc but isn't called from `on_load`'s scope. `None` before mount.
pub fn doc_handle() -> Option<crate::mission_doc::DocHandle> {
    HISTORY_CTX.with(|c| c.borrow().as_ref().map(|ctx| ctx.doc.clone()))
}

/// Mark the doc clean (a successful Save) or force a dirty state. Used by `mission_commands` on a
/// 201, by the hydrate/conflict adopt path, and by the T-189 IDB-restore path.
pub fn set_dirty(value: bool) {
    HISTORY_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.dirty.set(value);
        }
    });
}

/// T-189 — true when the live doc holds unsaved work (the same mirror the strip's `•` binds).
///
/// `try_get_untracked`, not `get_untracked`: `HISTORY_CTX` outlives the route by design (it is never
/// cleared), so after a route-leave the signal's reactive owner is disposed and a plain read would
/// panic. A disposed signal simply means "no editor, nothing to warn about".
#[must_use]
pub fn is_dirty() -> bool {
    HISTORY_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.dirty.try_get_untracked())
            .unwrap_or(false)
    })
}

/// T-189 — does this mission id have a server Save target? Only a real (UUID) mission can be saved,
/// so only there does "unsaved" mean anything: on the gate route (`/missions/smoke/edit`) and the
/// `draft` fallback the POST would 404, `dirty` could never be cleared, and the guard would prompt
/// forever. Same carve-out, same predicate as `mission_hydrate::is_uuid` — which skips the whole
/// hydrate/dirty machinery for exactly these ids so the 12 editor smokes stay untouched. (Duplicated
/// rather than shared: `is_uuid` is private to that module and `mission_hydrate` is not this
/// slice's file to change.)
fn saves_to_server(mission_id: &str) -> bool {
    let b = mission_id.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, &c)| match i {
            8 | 13 | 18 | 23 => c == b'-',
            _ => c.is_ascii_hexdigit(),
        })
}

/// T-189 — install the `beforeunload` guard: a tab close / reload / hard navigation while the doc is
/// dirty gets the browser's "Leave site?" confirmation instead of silently discarding the work.
///
/// The doc comment on [`HistoryCtx::dirty`] claimed this guard existed since T-159.26; it did not —
/// a repo-wide grep found the comment and nothing else. This is the guard.
///
/// Idempotent: re-arms exactly one listener on a remount (route-leave → route-enter), because it
/// unregisters first. No-op for a mission with no server Save target (see [`saves_to_server`]) and
/// before [`set_ctx`] has run. Pairs with [`unregister_unload_guard`] under the caller's
/// `on_cleanup` — do NOT `.forget()` this one.
pub fn register_unload_guard() {
    unregister_unload_guard();
    let Some(win) = web_sys::window() else {
        return;
    };
    let armed = HISTORY_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .is_some_and(|ctx| saves_to_server(&ctx.mission_id))
    });
    if !armed {
        return;
    }
    // Captures nothing: the dirty state is read through the thread_local at fire time, so the
    // closure can never hold a stale signal handle or a disposed owner.
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(|ev: web_sys::Event| {
        if !is_dirty() {
            return; // saved / clean → never interrupt the navigation
        }
        ev.prevent_default();
        let _ = js_sys::Reflect::set(
            &ev,
            &JsValue::from_str("returnValue"),
            &JsValue::from_str(UNSAVED_PROMPT),
        );
    });
    let _ = win.add_event_listener_with_callback("beforeunload", cb.as_ref().unchecked_ref());
    UNLOAD_GUARD.with(|g| *g.borrow_mut() = Some(cb));
}

/// T-189 — remove the `beforeunload` guard and free its closure. Zero-capture (so it is a plain
/// `fn` item: `Send + Sync + 'static`, i.e. `on_cleanup`-compatible) and idempotent.
///
/// Order matters: the listener is removed BEFORE the `Closure` drops, or a `beforeunload` firing in
/// between would invoke a freed closure ("closure invoked after being dropped").
pub fn unregister_unload_guard() {
    let taken = UNLOAD_GUARD.with(|g| g.borrow_mut().take());
    if let Some(cb) = taken {
        if let Some(win) = web_sys::window() {
            let _ = win
                .remove_event_listener_with_callback("beforeunload", cb.as_ref().unchecked_ref());
        }
        drop(cb);
    }
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

/// Re-read the HUD mirrors from the live doc + selection. For changes that replace the document
/// wholesale (the mount seed, the IDB restore swap), where the glyph rebind + persist of
/// [`after_doc_change`] would be wrong or redundant.
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

/// **Wave 145 F-1 — the ONE selection prune, over the whole selectable universe.**
///
/// Drops from `ctx.selection` every id the live document no longer holds, and NOTHING else. Both
/// post-change sites ([`rebind_engine_from_doc`] and [`after_doc_change`]) call this rather than
/// each carrying its own `retain`, so the universe is decided once — two copies is how one of them
/// gets widened and the other does not.
///
/// The universe is [`crate::mission_editor::selectable_ids`], read from the POST-change document:
/// slots off `slots_json` (hidden rows included — `materialize()` drops T-665 / T-701 hidden slots
/// and would deselect a slot for being invisible rather than for being gone, which is what made
/// `editor_ops::toggle_hidden` unable to toggle back), plus the vehicle / entity / comment key sets
/// off `small_maps_json`. Reading it from the settled document is what keeps the guarantee this
/// prune exists for: a row deleted or undone away is out of its map before this runs, so its id
/// still falls out and Delete can never act on it.
///
/// The universe function lives in `mission_editor` because this module is
/// `#![cfg(target_arch = "wasm32")]` end to end and can host no test that ever executes; the pin
/// `mission_editor::w145_selection_prune::the_selection_prune_runs_over_the_whole_selectable_universe`
/// reads this body back through `include_str!` and is what stops the SoA creeping back in.
/// (The path was wrong here until the wave-145 verifier caught it: it named `t784_comment_glyph`,
/// a module that does not contain this pin, so a `cargo test` filter on the cited name ran ZERO
/// tests — a citation that reads like a guarantee and resolves to nothing.)
fn prune_selection(ctx: &HistoryCtx) {
    let live = {
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };
        crate::mission_editor::selectable_ids(&core.slots_json(), &core.small_maps_json())
    };
    ctx.selection
        .borrow_mut()
        .retain(|id| live.contains(id.as_str()));
}

/// T-175 B1 — rebind the engine slot glyphs from the live doc after a **wholesale document swap**
/// (IDB restore / server hydrate) so the restored slot positions actually reach the GPU. The IDB
/// restore path previously only called [`refresh_hud`] (HUD counts, no engine rebind), so if the
/// engine-create task won the race and first-bound the seed doc, restored positions never rendered
/// until a manual edit ("first load: slots at wrong position"). Unlike [`after_doc_change`] this
/// does **not** mark dirty / bump `doc_ver` / schedule a persist — those would echo the restore back
/// as a user edit. No-op if the engine isn't mounted yet (the engine-mount handshake reruns it);
/// idempotent (a full replace), so whichever of restore / engine-mount settles last binds once from
/// the settled doc — no seed→restore flash, no double bind.
pub fn rebind_engine_from_doc() {
    HISTORY_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        // T-819 — bind the map-render SoA (crewed slots derived-hidden). OBJ still counts every
        // authored slot via `slot_count` — invisible is not gone.
        let (soa, obj) = {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return;
            };
            (
                crate::mission_editor::map_render_slot_soa(core),
                core.slot_count(),
            )
        };
        prune_selection(ctx);
        let ids = ctx.selection.borrow().clone();
        if let Some(e) = ctx.engine.borrow_mut().as_mut() {
            let tints = map_engine_core::slots_gpu::side_tints_rgba_bytes(&soa.side_keys);
            e.slots_bind_symbology(
                soa.ids.clone(),
                &soa.xy,
                &tints,
                soa_roles(&soa),
                &soa.rotations,
            );
            e.set_selection(ids);
            if let Some(doc) = ctx.doc.borrow().as_ref() {
                upload_squad_links(e, doc, &soa);
                let (vxy, valiases, vtints, vheadings) = vehicle_lane_fields();
                e.vehicles_bind_symbology(&vxy, valiases, &vtints, &vheadings);
                let (mxy, mtints, micons, mcaptions) = marker_lane_xy_tints(doc);
                e.markers_bind(&mxy, &mtints, micons, mcaptions);
                e.comments_bind_ids(&comment_lane_xy(doc), comment_lane_ids(doc));
            }
        }
        refresh_signals(ctx, obj);
    });
}

/// Selection-only refresh (click / marquee / outliner select / attributes open): pushes SEL and
/// the dock highlight mirror WITHOUT rebuilding the outliner/ORBAT node trees. Rebuilding both
/// trees (`refresh_docks`) on every click re-flattened O(n) rows per selection — the T-172 B8
/// "selection feels laggy" root cause; the row highlight is already a fine-grained `is_sel`
/// closure over `selected_ids`, so this is all a selection change needs.
pub fn refresh_selection() {
    HISTORY_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        ctx.sel_count.set(ctx.selection.borrow().len());
    });
    crate::editor_ops::refresh_selection_mirrors();
}

/// The one post-document-change sequence: materialize → prune the selection → rebind the engine
/// glyphs + tint → bump `doc_ver` → schedule the persist (**T-380: only once the boot restore has
/// settled**) → refresh the HUD.
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
    // T-819 — map-render SoA for glyph bind; `obj` stays the authored slot census.
    let (soa, obj) = {
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };
        (
            crate::mission_editor::map_render_slot_soa(core),
            core.slot_count(),
        )
    };
    prune_selection(ctx);
    let ids = ctx.selection.borrow().clone();
    if let Some(e) = ctx.engine.borrow_mut().as_mut() {
        e.set_drag(Vec::new(), 0.0, 0.0); // clear any live drag overlay
        let tints = map_engine_core::slots_gpu::side_tints_rgba_bytes(&soa.side_keys);
        e.slots_bind_symbology(
            soa.ids.clone(),
            &soa.xy,
            &tints,
            soa_roles(&soa),
            &soa.rotations,
        );
        e.set_selection(ids);
        if let Some(doc) = ctx.doc.borrow().as_ref() {
            upload_squad_links(e, doc, &soa);
            // T-573 — this is also the END of the mixed-drag VEHICLE preview: since
            // `select_tool::push_drag_preview` re-packs the vehicle lane with the dragged rows
            // offset, the lane is live state during a drag, and this unconditional re-bind from the
            // committed document is what puts it back on authored truth. A gesture that ends
            // WITHOUT a commit never reaches here — `select_tool::clear_drag_preview` covers those.
            let (vxy, valiases, vtints, vheadings) = vehicle_lane_fields();
            e.vehicles_bind_symbology(&vxy, valiases, &vtints, &vheadings);
            let (mxy, mtints, micons, mcaptions) = marker_lane_xy_tints(doc);
            e.markers_bind(&mxy, &mtints, micons, mcaptions);
            e.comments_bind_ids(&comment_lane_xy(doc), comment_lane_ids(doc));
        }
    }
    ctx.doc_ver.set(ctx.doc_ver.get().saturating_add(1));
    ctx.dirty.set(true); // T-159.26 — a committed edit is unsaved work

    // T-380 — do NOT arm the debounced writer while the boot restore is still in flight.
    //
    // The mutator path is live long before the document is: `mission_editor` seeds an 8-slot FIXTURE
    // synchronously, registers the window keydown handler synchronously, and the boot overlay is
    // deliberately `pointer-events-none`, so a Delete/Ctrl+V during boot reaches this function while
    // `doc` still holds the fixture. `schedule_edit_persist` would arm a 5 s timer whose `get_bytes`
    // encodes whatever `doc` holds *at write time* — and if the restore of a several-hundred-MB
    // record has not swapped the real core in by then, the timer files the 8-slot seed over the good
    // one. It passes every writer guard: non-empty, owner matches, not cancelled. A content-level
    // check cannot catch it either (T-374) — the seed is not empty, it has 8 real slots.
    //
    // The gate is on the WRITER, not the UI: the overlay stays click-through (the editor smokes and
    // the operator's own fast path depend on it), and the edit still lands in the document.
    //
    // Dropping the arm does not drop the operator's work:
    //   * cold boot (nothing to restore) — the edit stays in the doc, and the boot persist
    //     `mission_editor` arms right after the two awaits encodes the live doc, edit included.
    //   * restore path — the restore swaps the document wholesale, which discards the edit anyway;
    //     the gate's job is only to make sure it was never written over the real record first.
    // The same reasoning covers the hydrate/adopt tail (`mission_hydrate::adopt_payload` reaches
    // here via `after_local_edit` during boot): its content is persisted by that same boot persist.
    if ctx.restore_settled.get() {
        crate::yrs_persist::schedule_edit_persist(ctx.doc.clone(), &ctx.mission_id);
    }
    refresh_signals(ctx, obj);
}

/// T-748 — flat comment-lane `xy` for [`RenderEngine::comments_bind`]: interleaved world `[x,z,…]`
/// from `comments_json` (`commentsById`). Both bind sites in this file call it, so undo / redo /
/// restore share one feed with place/edit — a lane bound only from authoring call sites would go
/// stale the same way T-760 forbids for markers.
///
/// **T-784 — the parse itself moved to `mission_editor::comment_lane_xy`, and this is now the whole
/// of the function.** It used to be a private copy of that parse, which made the lane and the map's
/// comment PICK two independent readers of the same document — the shape T-780 refused for the
/// connection line. `mission_editor::comment_lane_xy` packs `mission_editor::comment_points`, and
/// `mission_editor::pick_comment` hit-tests that same list, so what is drawn and what a click can
/// find are one set by construction. It lives over there for a second reason too: this module is
/// `#![cfg(target_arch = "wasm32")]` in its entirety, so nothing defined here can be unit-tested
/// (which is why the T-748 feed pin has to reach this file through `include_str!` at all).
fn comment_lane_xy(doc: &MissionDocCore) -> Vec<f32> {
    crate::mission_editor::comment_lane_xy(&doc.comments_json())
}

/// **T-808 — the comment lane's SECOND column: one id per bubble**, for
/// [`RenderEngine::comments_bind_ids`]. Without it the engine holds coordinates it cannot name, so
/// no note can ever be drawn as selected (`comments_bind`'s empty-ids path marks every row
/// unselected and the amber treatment is simply never applied) — the half of T-796 the feeder never
/// delivered.
///
/// Row-aligned with [`comment_lane_xy`] **BY CONSTRUCTION**, not by agreement: both are projections
/// of the same `mission_editor::comment_points` list (id-sorted, so independent of `serde_json`'s
/// map order), one taking `x`/`z` and this one taking `id`. That is the T-784/T-748 single-reader
/// rule applied to the id column — an ids array built from any other walk of `commentsById` could
/// hand row *i*'s selection to row *j*, which is worse than no selection treatment at all.
fn comment_lane_ids(doc: &MissionDocCore) -> Vec<String> {
    crate::mission_editor::comment_lane_ids(&doc.comments_json())
}

/// **T-808 — the per-row ROLE column for [`RenderEngine::slots_bind_symbology`].** The SoA stores
/// roles interned (a `roles` dictionary + a `role_idx` per row — see `SlotSoa`), and the engine
/// wants one string per row; this is that expansion, and nothing more.
///
/// An index the dictionary does not cover — `NONE_IDX` (`u32::MAX`), the sentinel for a slot with no
/// authored role — yields the empty string, which `slots_gpu::unit_role_class` resolves to the
/// rifleman default. That is deliberately the SAME picture the pre-T-808 `slots_bind_soa` drew, so
/// an unauthored slot loses nothing by this wiring.
pub(crate) fn soa_roles(soa: &SlotSoa) -> Vec<String> {
    soa.role_idx
        .iter()
        .map(|&i| soa.roles.get(i as usize).cloned().unwrap_or_default())
        .collect()
}

/// **T-808 — the four parallel vehicle-lane columns for [`RenderEngine::vehicles_bind_symbology`]**
/// (`xy`, registry alias / prefab path, packed RGBA8 side tint, compass heading), built in ONE pass
/// over [`crate::editor_ops::vehicle_rows`].
///
/// **Why one reader and not four.** `vehicle_rows` sorts by id; `MissionDocCore::vehicle_xy_flat`
/// (what this lane used to be fed) walks the `yrs` map in ITERATION order. Keeping the old call for
/// `xy` and adding the other three columns off `vehicle_rows` would have produced four arrays in two
/// different row orders — every vehicle drawn wearing another vehicle's kind, side and heading. A
/// silhouette pointing confidently the wrong way is a worse lie than the amber disc it replaces, so
/// the alignment is not asserted here, it is structural: one iterator, one `push` per column per
/// row, and the only `continue` (an unplaced vehicle) happens BEFORE any column is written.
///
/// Unplaced ORBAT vehicles (`xy == None`) are skipped, exactly as `vehicle_xy_flat` skipped rows
/// with no `position`: a vehicle that has never been dropped on the map has no lane row. `rotation`
/// is the authored COMPASS heading in degrees passed through untouched — `slots_gpu` owns the
/// screen-yaw sign flip, so converting here would double it (and `None`, an unrotated vehicle,
/// is north).
///
/// `pub(crate)` because it is THE column builder, not this module's: T-808's drag preview
/// ([`crate::editor::tools::select_tool::bind_vehicle_preview_lane`]) rebinds the same lane at dragged positions
/// and reuses these three non-positional columns rather than growing a second builder in a second
/// row order. Do not re-privatise it; write the second caller's columns here.
pub(crate) fn vehicle_lane_fields() -> (Vec<f32>, Vec<String>, Vec<u8>, Vec<f32>) {
    let rows = crate::editor_ops::vehicle_rows();
    let mut xy = Vec::with_capacity(rows.len() * 2);
    let mut aliases = Vec::with_capacity(rows.len());
    let mut tints = Vec::with_capacity(rows.len() * 4);
    let mut headings = Vec::with_capacity(rows.len());
    for r in rows {
        let Some((x, y)) = r.xy else { continue };
        #[allow(clippy::cast_possible_truncation)]
        {
            xy.push(x as f32);
            xy.push(y as f32);
            headings.push(r.rotation.unwrap_or(0.0) as f32);
        }
        // `faction-{SIDE}` when map-placed (the marker feed strips the same prefix the same way).
        let side = r
            .faction_id
            .strip_prefix("faction-")
            .unwrap_or(&r.faction_id);
        tints.extend_from_slice(&map_engine_core::slots_gpu::side_rgba(side));
        aliases.push(r.resource_name);
    }
    (xy, aliases, tints, headings)
}

/// T-760 / **T-790** — the marker lane args for [`RenderEngine::markers_bind`]: interleaved world
/// `[x,z,…]`, packed RGBA8 side tints, per-marker canonical glyph ids, and one caption per marker.
///
/// **T-790 moved the parse to `mission_editor::marker_lane_fields`** and this delegates to it, the
/// same T-784/T-748 move that put `comment_lane_xy`'s parse in `mission_editor`: this module is
/// `#![cfg(target_arch = "wasm32")]` end to end, so a parse living here cannot be unit-tested, and
/// the `icon → glyph` mapping and caption extraction are exactly the logic that must be. Kept called
/// from BOTH bind sites so undo/redo/restore share one feed — a lane bound only from authoring call
/// sites would go stale exactly the way the ticket forbids.
fn marker_lane_xy_tints(doc: &MissionDocCore) -> (Vec<f32>, Vec<u8>, Vec<String>, Vec<String>) {
    crate::mission_editor::marker_lane_fields(&doc.briefing_marker_rows_json())
}

/// T-180.4 — thin dirty→upload: collect inputs from doc, geometry in core, hairline role 9.
fn upload_squad_links(e: &mut RenderEngine, doc: &MissionDocCore, soa: &SlotSoa) {
    let mut xy_by_slot: HashMap<String, (f32, f32)> = HashMap::with_capacity(soa.ids.len());
    for (i, id) in soa.ids.iter().enumerate() {
        let x = soa.xy[i * 2];
        let y = soa.xy[i * 2 + 1];
        xy_by_slot.insert(id.clone(), (x, y));
    }
    let inputs = doc.squad_link_inputs();
    let verts = build_squad_link_segments(&inputs, &xy_by_slot);
    #[allow(clippy::cast_possible_truncation)]
    let segment_count = (verts.len() / 12) as u32;
    e.upload_hairline_segments(role_id::SQUAD_LINKS, &verts, segment_count, true);
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
///
/// **T-785 — this is the LAST line of defence, and the reason it is read directly off
/// `document.activeElement`.** Every editor chord (E/R collapse the docks, Space recentres the
/// camera, G flips snap, Ctrl+A / Backspace / copy-paste) sits behind this guard at the top of the
/// `mission_editor` keydown closure and behind [`register_key_handler`] here — so whatever this
/// returns is what decides "typed character" vs "chord". The Attributes/rename remount bug
/// (fixed in `attributes.rs::text_field` and `eden_dock_left.rs`) worked by dropping focus to
/// `<body>` mid-word, at which point this correctly reported "not editable" and the tail of the
/// word ran as chords. Keeping focus is the root fix; reading the LIVE `activeElement` tag and
/// contentEditable state here (never a cached "is a field open?" flag) is what makes the guard
/// track focus the instant it moves, so a field that loses focus can never keep swallowing keys and
/// a chord can never fire while a field truly holds focus.
pub fn in_editable_field() -> bool {
    let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
    else {
        return false;
    };
    // Native form controls by tag. `activeElement` is `<body>` (or `<html>`) when nothing is
    // focused, so this is false exactly when a chord SHOULD be allowed to fire.
    if matches!(el.tag_name().as_str(), "INPUT" | "SELECT" | "TEXTAREA") {
        return true;
    }
    // contentEditable hosts (rich-text widgets). `is_content_editable` already resolves the
    // inherited `inherit` case, so a caret inside a nested span of an editable region counts.
    if el
        .dyn_ref::<web_sys::HtmlElement>()
        .is_some_and(web_sys::HtmlElement::is_content_editable)
    {
        return true;
    }
    // Belt-and-braces for editable widgets that are neither a native control nor contentEditable but
    // announce themselves as text entry (ARIA `role="textbox"`/`searchbox`, or a `contenteditable`
    // attribute a browser has not reflected onto `isContentEditable`). Reading the attribute
    // directly means a custom field can opt into the same typing vs chord split without being an
    // `<input>`.
    if el
        .get_attribute("contenteditable")
        .is_some_and(|v| v != "false")
    {
        return true;
    }
    matches!(
        el.get_attribute("role").as_deref(),
        Some("textbox" | "searchbox")
    )
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
