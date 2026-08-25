//! T-934.14 — the Mission Creator window-level KEYDOWN dispatch, moved verbatim out of
//! `MissionEditorPage`'s `on_load` block (`mission_editor.rs`) — the final Phase B2 evacuation.
//! One `Closure` lives here: the editor's own `onkeydown` (Backspace/Delete exclusivity, the
//! shared Esc dismissal stack, Ctrl/Cmd+C/X/V/Shift+V/A, Ctrl+Alt+D, Space flyTo, E/R dock
//! latches, G/[/] snap grid, 1/2/3 widget variant). It rides the SAME `EditorGestureContext`
//! the T-934.13 gesture closures use — the page builds the context once and calls
//! [`attach_editor_hotkeys`] beside `attach_canvas_gestures`; the closure BODY is byte-identical
//! to its pre-move text (the Class-S pins that used to grep `mission_editor.rs` for the arms now
//! grep this file), and the capture preamble clones from locals mirroring the page's, so the
//! capture semantics are unchanged. The window registration + `forget()` leak contract moved with
//! it.
//!
//! NOT here: `mission_history`'s Ctrl+Z/Y keydown (`state/history.rs` — the OTHER window-level
//! editor keydown, unmoved), and `state/commands_hotkeys.rs` (the save/export/clipboard COMMAND
//! registry, a different surface — the name collision is deliberate and documented in the
//! T-934 plan: this module is the CANVAS key dispatch, that one is the command palette table).

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::editor::mission_editor::plain_paste_anchor;
use crate::editor::state::history as mission_history;
use crate::editor::state::operations as editor_ops;

use super::gestures::{make_sync_los, make_sync_ruler, EditorGestureContext};

/// Attach the editor keydown closure to the window. The local `let` belt below mirrors the
/// page's `on_load` environment name-for-name (the T-934.13 idiom), so the moved block — its
/// capture preamble and the whole closure body — is byte-identical to the pre-move
/// `mission_editor.rs` text.
pub(crate) fn attach_editor_hotkeys(ctx: &EditorGestureContext) {
    let container = ctx.container.clone();
    let engine = ctx.engine.clone();
    let ruler = ctx.ruler.clone();
    let los = ctx.los.clone();
    let viewshed = ctx.viewshed.clone();
    let cursor = ctx.cursor;
    let snap = ctx.snap;
    let widget_variant = ctx.widget_variant;
    let selected_connection = ctx.selected_connection;
    let chrome_hidden = ctx.chrome_hidden;
    let dock_left_collapsed = ctx.dock_left_collapsed;
    let dock_right_collapsed = ctx.dock_right_collapsed;
    let debug_hud_shown = ctx.debug_hud_shown;
    let sync_ruler = make_sync_ruler(ctx);
    let sync_los = make_sync_los(ctx);

    // T-159.26 — editor keyboard actions (MissionCreatorPage onKeyDown): Delete
    // (remove selection), Space (center on centroid), Ctrl/Cmd+C/V (copy/paste at cursor).
    // T-669 completes the clipboard: Ctrl/Cmd+X cuts (copy then delete) and
    // Ctrl/Cmd+Shift+V pastes at the SOURCE position instead of at the cursor.
    // T-662 — Backspace is here too, but bound to hide-chrome (`chrome_hidden`), NOT delete.
    // A SEPARATE window keydown from the undo/redo one (which owns Ctrl+Z/Y) — each guards
    // its own keys, both skip editable fields. `cursor` feeds the paste anchor (world coords).
    {
        // T-642 — the ruler chain + its reactive sync, so the Esc arm can dismiss it.
        let ruler = ruler.clone();
        let sync_ruler = sync_ruler.clone();
        // T-643 — the LoS capture + its reactive sync SHARE this same Esc seam (Decision 3):
        // rather than add a second window keydown listener (T-726, the window-Esc pile-up, is
        // pending — a new UNGUARDED listener would make it worse), LoS hooks the ruler's
        // existing Escape arm below, so the eventual T-726 fix covers both tools at once.
        let los = los.clone();
        let sync_los = sync_los;
        // T-644 — the VIEWSHED sub-mode joins the SAME Esc seam (no new window listener — T-726
        // is pending): the keydown arm below also calls `viewshed.escape()` and, on a real
        // dismissal, drops the engine wash lane. `engine` is cloned in so the arm can call
        // `viewshed_clear()` when the wash is dismissed.
        let viewshed = viewshed.clone();
        let engine = engine.clone();
        // T-649 SEL-ALL-001 — the Ctrl/Cmd+A arm needs the canvas CSS size, because Eden
        // scopes Select All to what is ON SCREEN. The container is the same element every
        // pointer gesture measures for its frozen camera, so Ctrl+A and a full-canvas
        // marquee drag are measured against the identical rect.
        let container = container.clone();
        let onkeydown =
            Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
                if mission_history::in_editable_field() {
                    return;
                }
                let modk = ev.ctrl_key() || ev.meta_key();
                let (cx, cy) = match cursor.get_untracked() {
                    Some((x, y, _)) => (Some(x), Some(y)),
                    None => (None, None),
                };
                // Each arm returns whether it acted; prevent the browser default once.
                let handled = match ev.code().as_str() {
                    // T-642/T-643 — Esc is the SHARED two-step escalating dismissal (Decision
                    // 3) for BOTH measure tools. The ruler: first Esc drops the in-progress
                    // tail (keeps a legged measure placed), a second clears the placed ruler.
                    // LoS mirrors it: first Esc drops the in-progress observer, a second clears
                    // the placed shot. Only one tool is ever non-empty at a time (switching
                    // tools clears the other's overlay), so calling BOTH `.escape()`s here is
                    // safe — the inactive tool's state is empty and its `.escape()` is a false
                    // no-op. Esc only "acts" (→ prevent_default) when SOMETHING was dismissed;
                    // an Esc with neither tool placed falls through untouched (never swallowed).
                    "Escape" if !modk => {
                        // T-726 — yield to any open overlay (context menu / dialogs /
                        // pickers). Those register with `modal_stack`; without this guard,
                        // Esc closing the menu also steps the measure machines (wave108
                        // MAJOR-2; LoS/viewshed victims in waves 109–110).
                        if crate::core::ui::modal_stack::any_open() {
                            false
                        } else {
                            // T-723 — Esc disarms an armed place BEFORE the measure-tool seam.
                            // `cancel_pending` was unreachable from the keyboard; Eden stamp
                            // cancel is Esc (and RMB on pointerup). Clear the ghost with the arm.
                            let place_acted = if editor_ops::has_pending() {
                                editor_ops::cancel_pending();
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.clear_place_preview();
                                }
                                true
                            } else {
                                false
                            };
                            // T-792 — Esc during an in-progress ZONE/TRIGGER draw (circle
                            // centre placed, or polygon with ≥1 vertex) abandons the pending
                            // geometry. A zone draw is multi-click and deliberately SURVIVES
                            // `cancel_pending` (so the panel's own Close/Undo/Cancel buttons,
                            // whose pointerup bubbles to the map, don't destroy the ring) —
                            // `has_pending()` above is therefore false-negative for a draw,
                            // and F-31 was: arm Circle, click centre, Esc → the "click the
                            // rim" hint STAYED and the next click still completed the circle.
                            // `cancel_zone_draw` is the ONE cancel a draw honours; it clears
                            // the draft and bumps the dock tick so the rim/vertex hint (gated
                            // on `zone_draft()` under `doc_tick`) vanishes — the same effect
                            // as the panel Cancel button, now on the keyboard. The draft is
                            // SHARED by the trigger tool (`begin_zone_draw(.., Trigger)` sets
                            // the identical `Pending::Zone`), so this one call also cancels an
                            // in-progress trigger draw. It "acts" only when a draw was in
                            // flight, so an Esc with no draw falls through untouched — and
                            // when it DOES act, feeding it to the `||` below prevents the
                            // browser default and stops any lower Esc layer (dialog/menu/tab)
                            // consuming the SAME press (one-Esc-one-layer, T-813/T-814).
                            let zone_draw_acted = editor_ops::cancel_zone_draw();
                            // T-768 — Esc disarms an armed connect the same way it disarms an
                            // armed place (T-723). Completing stays LMB pick / RMB Complete.
                            let connect_acted = if editor_ops::pending_connect().is_some() {
                                editor_ops::cancel_connect();
                                true
                            } else {
                                false
                            };
                            let ruler_acted = ruler.borrow_mut().escape();
                            if ruler_acted {
                                sync_ruler();
                            }
                            let los_acted = los.borrow_mut().escape();
                            if los_acted {
                                sync_los();
                            }
                            // T-644 — the viewshed's Esc is one step (clear the placed
                            // observer + raster); on a real dismissal also drop the engine wash
                            // lane. Like the ray, only one LoS lane is ever non-empty at a time
                            // (the sub-mode toggle clears the other), so calling it unconditionally
                            // is safe — an empty viewshed's `.escape()` is a false no-op.
                            let viewshed_acted = viewshed.borrow_mut().escape();
                            if viewshed_acted {
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.viewshed_clear();
                                }
                            }
                            place_acted
                                || zone_draw_acted
                                || connect_acted
                                || ruler_acted
                                || los_acted
                                || viewshed_acted
                        }
                    }
                    "KeyC" if modk && !ev.alt_key() && !ev.shift_key() => {
                        editor_ops::copy_selection()
                    }
                    // T-669 ACTION-CUT-001 — Ctrl/Cmd+X is COPY, then DELETE, in that order
                    // and SHORT-CIRCUITED. `copy_selection` returns false when there was
                    // nothing to put on the clipboard (empty selection, or the ops context /
                    // doc is not up yet), and a cut that could not copy must NOT delete —
                    // that would be a silent destructive Delete wearing an X. `&&` is exactly
                    // that guarantee: `delete_selection` never runs unless the clipboard took
                    // the snapshot first. Both halves are pre-existing `editor_ops`
                    // primitives, so this arm adds no new doc write and no new undo step
                    // beyond the one `delete_selection` already files.
                    //
                    // Census: X was bound by NEITHER window-level editor keydown before this
                    // slice (this file's nor `mission_history`'s Ctrl+Z/Y one) — pinned by
                    // `t669_cut_key_census`. It carries the same guard shape as the C / V
                    // arms it sits between, so the top-of-closure `in_editable_field()` guard
                    // keeps Ctrl+X meaning "cut the text" while the operator is typing in an
                    // Attributes field.
                    "KeyX" if modk && !ev.alt_key() && !ev.shift_key() => {
                        editor_ops::copy_selection() && editor_ops::delete_selection()
                    }
                    // T-743 — THE PLAIN PASTE ALWAYS CARRIES AN ANCHOR. It used to hand
                    // `paste_at_cursor` the raw `cx`/`cy`, which are `None` whenever the
                    // pointer is off the map (over any chrome panel, or before the first
                    // pointermove) — and a `None` anchor is now the paste-at-original
                    // instruction, which is emphatically not what a plain Ctrl/Cmd+V means.
                    // So the fallback is resolved HERE, where the camera is, instead of in
                    // the document core where the two intents used to share one branch:
                    // cursor if there is one, otherwise the centre of the visible map. See
                    // `plain_paste_anchor` for why that is the fallback and not a no-op.
                    //
                    // The centre is unprojected against `frozen_camera` — the SAME camera the
                    // CUR read-out and every pick use — so "the middle of the view" means the
                    // same world point here as it would if the operator had put the pointer
                    // there and pressed Ctrl+V. `try_borrow` rather than `borrow`: this is a
                    // window-level listener that can fire during a frame that already holds
                    // the engine, and a panic in a keydown takes the whole editor with it.
                    "KeyV" if modk && !ev.alt_key() && !ev.shift_key() => {
                        let rect = container.get_bounding_client_rect();
                        let view_centre = engine
                            .try_borrow()
                            .ok()
                            .and_then(|g| {
                                g.as_ref().map(|e| {
                                    crate::editor::tools::select_tool::frozen_camera(
                                        rect.width(),
                                        rect.height(),
                                        e.target_x(),
                                        e.target_y(),
                                        e.zoom(),
                                    )
                                    .unproject_xy(rect.width() / 2.0, rect.height() / 2.0)
                                })
                            })
                            .filter(|c| c[0].is_finite() && c[1].is_finite())
                            .map(|c| (c[0], c[1]));
                        match plain_paste_anchor(cx.zip(cy), view_centre) {
                            Some((ax, ay)) => editor_ops::paste_at_cursor(Some(ax), Some(ay)),
                            None => false,
                        }
                    }
                    // T-669 ACTION-PASTE-ORIG-001 — Ctrl/Cmd+Shift+V pastes with NO cursor
                    // anchor. `paste_at_cursor`'s anchor is `Option`al and that option IS the
                    // feature: `Some(ax, ay)` translates the clip's centroid onto that point
                    // (the plain paste arm above), `None` leaves every slot on its SOURCE
                    // coordinates.
                    //
                    // T-743 — AND IT NOW MEANS THAT EXACTLY. This comment used to carry a
                    // wrinkle: `Doc::paste_slots`' no-anchor arm added a 20 m `PASTE_NUDGE`
                    // to both axes for byte-parity with the JS `ydoc.pasteSlots`, so the
                    // command named "paste at the source position" put every slot 20 m off
                    // it. The operator retired that parity (it was a migration safety net,
                    // not a contract); the arm translates by nothing, and the help row's
                    // "source position" is now a literal statement rather than an
                    // approximation. The off-map plain paste that used to share the nudged
                    // branch is answered in the arm above.
                    //
                    // MUTUAL EXCLUSION with the plain paste arm: that arm guards
                    // `!ev.shift_key()`, this one guards `ev.shift_key()`, and both require
                    // `modk && !ev.alt_key()`. One `KeyboardEvent` has exactly one `shiftKey`
                    // value, so at most one of the pair can ever match — they partition the
                    // Ctrl+V space rather than overlapping it, and the order they appear in
                    // is therefore irrelevant. Pinned by
                    // `the_two_paste_arms_are_mutually_exclusive`.
                    "KeyV" if modk && !ev.alt_key() && ev.shift_key() => {
                        editor_ops::paste_at_cursor(None, None)
                    }
                    // T-649 SEL-ALL-001 — Ctrl/Cmd+A selects everything IN VIEW. Eden scopes
                    // Select All to the viewport, not to the whole mission, so this hands the
                    // container's live CSS size to `select_all_in_view`, which runs the
                    // marquee's own `pick_rect` over the on-screen rect — an entity parked
                    // off-screen is deliberately NOT selected.
                    //
                    // Census: `KeyA` was bound by NEITHER window-level editor keydown before
                    // this slice (this file's nor `mission_history`'s Ctrl+Z/Y one) — pinned
                    // by `t649_ctrl_a_census`. It sits beside `KeyC` / `KeyV` because it is
                    // the same modifier family and the same top-of-closure
                    // `in_editable_field()` guard is what keeps Ctrl+A meaning "select the
                    // text" while the operator is typing in an Attributes field.
                    //
                    // Returning "acted" is load-bearing: `prevent_default` below is what
                    // stops the browser's own Select All blue-washing the editor chrome.
                    "KeyA" if modk && !ev.alt_key() && !ev.shift_key() => {
                        let rect = container.get_bounding_client_rect();
                        editor_ops::select_all_in_view(rect.width(), rect.height())
                    }
                    // T-635 — Ctrl/Cmd+Alt+D toggles the telemetry HUD (default hidden).
                    // Behind the same `in_editable_field()` guard at the top of this closure,
                    // so it never fires while typing in an Attributes field. It always "acts"
                    // (flips the signal) → `prevent_default` below. This gates TELEMETRY only;
                    // mission-correctness diagnostics stay always-on (see the `debug_hud_shown`
                    // declaration note, framework_synthesis §D.4 #7).
                    "KeyD" if modk && ev.alt_key() && !ev.shift_key() => {
                        debug_hud_shown.set(!debug_hud_shown.get_untracked());
                        true
                    }
                    "Space" if !modk => editor_ops::center_on_selection(),
                    // T-662 — Delete still removes the selection. Backspace is NO LONGER an
                    // alias for Delete; it toggles the Eden chrome (hide/show interface), so
                    // the two keys are now split arms. Backspace always "acts" (it flips the
                    // signal), so `prevent_default` fires below to keep the browser from
                    // treating it as a Back navigation.
                    // T-780 — `CONN-DEL-001` ON THE MAP. Eden deletes a connection by
                    // selecting its line and pressing Del; T-672 could only offer the
                    // panel's per-row button because there was no line. There is now, so
                    // Del over a selected edge removes it.
                    //
                    // It calls `editor_ops::delete_connection` — the EXACT function the
                    // panel's Delete button calls, which is the whole reason this arm is
                    // three lines. A map-side `core.remove_connection` here would be a
                    // second deletion path: a second place to keep the `after_local_edit`
                    // tail, the `connection_count` guard and the one-txn-one-Ctrl+Z promise
                    // correct, and the second one is always the one that rots (T-241).
                    //
                    // A BRANCH INSIDE THE ARM, not a second `"Delete"` arm with a guard.
                    // Both would behave identically, but a new guard TERM has to be readable
                    // by `eden_help`'s keymap census (`parse_guard`) — the census refuses to
                    // guess at a term it cannot parse, because a misread guard would silently
                    // widen or narrow the binding it reports collisions for. `Delete` keeps
                    // its one arm and its one censused `!modk` guard; what changed is what
                    // the arm DOES, which is not a keymap fact.
                    //
                    // [wave 142 F-1] THE ARM RESOLVES ITS SELECTION AGAINST THE DOCUMENT
                    // before it fires. `editor_ops` reconciles the signal on every selection
                    // and every document change (see the declaration), but the branch here
                    // must not DEPEND on that having run: what an armed id is worth is a
                    // question about the live graph, and asking it one expression before the
                    // delete costs nothing. An id the document no longer holds falls through
                    // to the entity delete rather than being handed to a verb that can only
                    // answer `false` — which would swallow the keypress in silence.
                    //
                    // `connection_exists` is the SAME question `delete_connection` gates on,
                    // so the branch taken here and the write that follows cannot disagree.
                    //
                    // The armed id is dropped either way. On the delete it is the disarm (the
                    // edge is gone; leaving it selected would leave Del pointing at a row that
                    // no longer exists, and a Ctrl+Z would silently re-arm it). On the stale
                    // branch it is the same disarm one keypress earlier than the reconcile.
                    "Delete" if !modk => {
                        let armed = selected_connection.try_get_untracked().flatten();
                        if armed.is_some() {
                            selected_connection.set(None);
                        }
                        match armed.filter(|id| editor_ops::connection_exists(id)) {
                            Some(id) => editor_ops::delete_connection(&id),
                            None => editor_ops::delete_selection(),
                        }
                    }
                    "Backspace" if !modk => {
                        chrome_hidden.set(!chrome_hidden.get_untracked());
                        true
                    }
                    // T-638 — E toggles the LEFT dock (Entity List), R the RIGHT (Asset
                    // Browser). Bare keys only (no Ctrl/Cmd/Alt/Shift) so Ctrl+R stays a
                    // browser reload and Alt/Shift combos are untouched; the top-of-closure
                    // `in_editable_field()` guard already keeps them from firing while typing
                    // in an Attributes field. Each always "acts" (flips its latch) → the
                    // reflow + centre-hold run off the Effect that observes the signal.
                    "KeyE" if !modk && !ev.alt_key() && !ev.shift_key() => {
                        dock_left_collapsed.set(!dock_left_collapsed.get_untracked());
                        true
                    }
                    "KeyR" if !modk && !ev.alt_key() && !ev.shift_key() => {
                        dock_right_collapsed.set(!dock_right_collapsed.get_untracked());
                        true
                    }
                    // ══════════════════════ T-648 — the snap grid + transform widget ══════
                    // KEY-GRID-001 — `G` toggles the snap-grid MASTER latch. Census: `KeyG`
                    // is bound by NOTHING in this editor keydown or `mission_history`'s (the
                    // only two window-level editor keydowns) — see the census pin
                    // `t648_keydown_census`. Bare key only (no Ctrl/Cmd/Alt/Shift), behind
                    // the top-of-closure `in_editable_field()` guard like E/R, so it never
                    // fires while typing. Chosen over Eden's `odiaeresis`/`;` keysym
                    // artefacts (the ticket's instruction) — a plain letter mnemonic for
                    // "grid". Always acts (flips the latch) → prevent_default below.
                    "KeyG" if !modk && !ev.alt_key() && !ev.shift_key() => {
                        snap.set(snap.get_untracked().toggled());
                        true
                    }
                    // TOOLBAR-GRID-MOVE-001 — `[` / `]` DECREASE / INCREASE the active snap
                    // step. Census: `BracketLeft`/`BracketRight` are bound by nothing in
                    // either editor keydown. They step the ladder of the CURRENT widget
                    // variant (translate variant → translation ladder, rotate variant →
                    // rotation ladder), so the one pair of keys tunes whichever grid the
                    // operator is working in. Clamped at both ends by `SnapState::stepped`.
                    // Only "act" (→ prevent_default) when a keypress at a ladder end still
                    // reports a change is unnecessary — we always return true because the
                    // key is ours regardless, and `[`/`]` have no browser default worth
                    // preserving inside the editor.
                    "BracketLeft" if !modk && !ev.alt_key() && !ev.shift_key() => {
                        let axis = widget_variant.get_untracked().snap_axis();
                        snap.set(snap.get_untracked().stepped(axis, -1));
                        true
                    }
                    "BracketRight" if !modk && !ev.alt_key() && !ev.shift_key() => {
                        let axis = widget_variant.get_untracked().snap_axis();
                        snap.set(snap.get_untracked().stepped(axis, 1));
                        true
                    }
                    // WIDGET-CYCLE-001 — `1` / `2` / `3` select the widget VARIANT, numbered
                    // to MATCH Eden's widget row EXACTLY (T-795, pixel-verified): `1` No Widget
                    // / `2` Translate / `3` Rotate. This is the Space-collision decision: Eden
                    // cycles variants on Space, but TBD's Space stays flyTo
                    // (`center_on_selection`, the arm above), and Eden's `1`-`5` direct keys
                    // are free here (census: no `Digit*` binding anywhere in the frontend), so
                    // `1`/`2`/`3` dissolve the clash without touching Space. Before T-795 these
                    // were OFF BY ONE (1=Translate, 2=Rotate, 3=nothing) — an Eden author's
                    // muscle memory armed the wrong mode three ways over; the renumber makes
                    // the keys mean what Eden means. `4`/`5` (Area Scaling / Area) stay
                    // RESERVED-UNBOUND — no area-scale variant yet (a transform selection is
                    // slots + vehicles, neither of which scales; see `WidgetVariant`'s doc);
                    // the numbering matches Eden so a later slice binds them without renumbering.
                    // Bare digit only (behind the `in_editable_field()` guard at the top of this
                    // closure, so a digit typed into an attribute field never flips a mode).
                    // `from_digit` always changes the variant for 1/2/3 → act.
                    "Digit1" if !modk && !ev.alt_key() && !ev.shift_key() => {
                        widget_variant.set(widget_variant.get_untracked().from_digit(1));
                        true
                    }
                    "Digit2" if !modk && !ev.alt_key() && !ev.shift_key() => {
                        widget_variant.set(widget_variant.get_untracked().from_digit(2));
                        true
                    }
                    "Digit3" if !modk && !ev.alt_key() && !ev.shift_key() => {
                        widget_variant.set(widget_variant.get_untracked().from_digit(3));
                        true
                    }
                    _ => false,
                };
                if handled {
                    ev.prevent_default();
                }
            });
        if let Some(win) = web_sys::window() {
            let _ =
                win.add_event_listener_with_callback("keydown", onkeydown.as_ref().unchecked_ref());
        }
        onkeydown.forget();
    }
}
