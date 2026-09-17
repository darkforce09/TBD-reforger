//! The Mission Creator's window-level `keydown` dispatch.
//!
//! **Role:** installs the two window-level keydown listeners the editor runs.
//! [`attach_editor_hotkeys`] carries the editor's own chords — the shared Escape dismissal stack,
//! the split Backspace / Delete arms, Ctrl/Cmd+C/X/V/Shift+V/A, Ctrl+Alt+D, Space fly-to, the E/R
//! dock latches, the G and `[`/`]` snap grid and the 1/2/3 widget variants.
//! [`register_key_handler`] carries the undo/redo shortcuts: Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z and
//! Ctrl+Y.
//! **Position:** the keyboard half of [`super`], beside the pointer closures in
//! [`super::pointer_gestures`]. The chord listener rides the same
//! [`EditorGestureContext`](super::pointer_gestures::EditorGestureContext) the pointer closures
//! do, so the page builds that context once and attaches both from it.
//! **Signals & state:** every handle and `Copy` signal the chord closure captures comes from the
//! gesture context; the undo/redo closure captures nothing at all and reads the live editor
//! through `document_host::history`'s thread-local context at fire time. Both listen on `window`
//! rather than on the container, so a chord works before the map has focus, and both leak their
//! closure the way the editor's other listeners do.
//! **Invariants:** every arm sits behind `document_host::history::in_editable_field()`, so a key
//! typed into a field is a character and never a chord. Undo and redo are reached only through
//! `document_host::history::undo` / `document_host::history::redo` — this module dispatches the
//! chord and calls across that boundary, it never steps the document's stack itself. Two
//! listeners, two disjoint key sets: `ui/modals/help_modal.rs`'s keymap census adjudicates them
//! against every other window keydown in the editor.
//!
//! Not here: `shell/document_commands.rs`, which is the save / export / clipboard COMMAND
//! registry the palette and the strip dispatch through. That is a table of named commands; this
//! is the key dispatch.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use website_map_engine::editing::tools::selection;

use crate::v2::apps::editor::bridge::document_host::history as mission_history;
use crate::v2::apps::editor::bridge::host_state::undo_grouped_gestures;
use crate::v2::apps::editor::mission_editor::plain_paste_anchor;
use website_map_engine::editing::hosted_commands as engine_ops;

use super::pointer_gestures::{make_sync_los, make_sync_ruler, EditorGestureContext};
use crate::v2::apps::editor::bridge::host_state::armed_placement;
use crate::v2::apps::editor::bridge::host_state::entity_selection;
use crate::v2::apps::editor::bridge::tactical_graphics_authoring;

/// Attach the editor's chord closure to the window.
///
/// The local `let` belt below unpacks the gesture context into names the closure captures, the
/// same idiom [`super::pointer_gestures::attach_canvas_gestures`] uses, so both halves of the
/// input layer capture one environment built once by the page.
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

    {
        let ruler = ruler.clone();
        let sync_ruler = sync_ruler.clone();
        let los = los.clone();
        let sync_los = sync_los;
        let viewshed = viewshed.clone();
        let engine = engine.clone();
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
                let handled = match ev.code().as_str() {
                    "Escape" if !modk => {
                        if crate::v2::core::ui::modal_stack::any_open() {
                            false
                        } else {
                            let place_acted = if armed_placement::has_pending() {
                                armed_placement::cancel_pending();
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.clear_place_preview();
                                }
                                true
                            } else {
                                false
                            };
                            let zone_draw_acted = armed_placement::cancel_zone_draw();
                            let tg_draw_acted = tactical_graphics_authoring::cancel_tactical_draw();
                            let tg_drag_acted =
                                tactical_graphics_authoring::cancel_tactical_vertex_drag();
                            if tg_drag_acted {
                                mission_history::refresh_tactical_lane();
                            }
                            let connect_acted = if engine_ops::pending_connect().is_some() {
                                engine_ops::cancel_connect();
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
                            let viewshed_acted = viewshed.borrow_mut().escape();
                            if viewshed_acted {
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.viewshed_clear();
                                }
                            }
                            place_acted
                                || zone_draw_acted
                                || tg_draw_acted
                                || tg_drag_acted
                                || connect_acted
                                || ruler_acted
                                || los_acted
                                || viewshed_acted
                        }
                    }
                    "KeyC" if modk && !ev.alt_key() && !ev.shift_key() => {
                        engine_ops::copy_selection()
                    }
                    "KeyX" if modk && !ev.alt_key() && !ev.shift_key() => {
                        engine_ops::copy_selection() && undo_grouped_gestures::delete_selection()
                    }
                    "KeyV" if modk && !ev.alt_key() && !ev.shift_key() => {
                        let rect = container.get_bounding_client_rect();
                        let view_centre = engine
                            .try_borrow()
                            .ok()
                            .and_then(|g| {
                                g.as_ref().map(|e| {
                                    selection::frozen_camera(
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
                            Some((ax, ay)) => {
                                undo_grouped_gestures::paste_at_cursor(Some(ax), Some(ay))
                            }
                            None => false,
                        }
                    }
                    "KeyV" if modk && !ev.alt_key() && ev.shift_key() => {
                        undo_grouped_gestures::paste_at_cursor(None, None)
                    }
                    "KeyA" if modk && !ev.alt_key() && !ev.shift_key() => {
                        let rect = container.get_bounding_client_rect();
                        entity_selection::select_all_in_view(rect.width(), rect.height())
                    }
                    "KeyD" if modk && ev.alt_key() && !ev.shift_key() => {
                        debug_hud_shown.set(!debug_hud_shown.get_untracked());
                        true
                    }
                    "Space" if !modk => entity_selection::center_on_selection(),
                    "Delete" if !modk => {
                        let armed = selected_connection.try_get_untracked().flatten();
                        if armed.is_some() {
                            selected_connection.set(None);
                        }
                        match armed.filter(|id| engine_ops::connection_exists(id)) {
                            Some(id) => engine_ops::delete_connection(&id),
                            None => {
                                tactical_graphics_authoring::delete_selected_tactical_graphic()
                                    || undo_grouped_gestures::delete_selection()
                            }
                        }
                    }
                    "Backspace" if !modk => {
                        chrome_hidden.set(!chrome_hidden.get_untracked());
                        true
                    }
                    "KeyE" if !modk && !ev.alt_key() && !ev.shift_key() => {
                        dock_left_collapsed.set(!dock_left_collapsed.get_untracked());
                        true
                    }
                    "KeyR" if !modk && !ev.alt_key() && !ev.shift_key() => {
                        dock_right_collapsed.set(!dock_right_collapsed.get_untracked());
                        true
                    }
                    "KeyG" if !modk && !ev.alt_key() && !ev.shift_key() => {
                        snap.set(snap.get_untracked().toggled());
                        true
                    }
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

/// Install the window `keydown` shortcuts for history: **Ctrl/Cmd+Z** undo, **Ctrl/Cmd+Shift+Z**
/// or **Ctrl+Y** redo.
///
/// `code()` rather than `key()`, so the binding is layout-independent — a modifier can remap
/// `key`. The modifier is ctrl **or** meta, Alt disqualifies, and `prevent_default` fires on a
/// *match* even when the stack is empty, so the browser's own undo can never fight the document.
/// Listens on `window` rather than on the container, so the shortcut works before the map has
/// focus, and the closure leaks like the editor's other listeners.
///
/// The step itself belongs to `document_host::history`: this closure calls
/// [`mission_history::undo`] / [`mission_history::redo`], which are the one path to the
/// document's undo stack for the toolbar buttons, these chords and the gate bridge alike.
pub fn register_key_handler() {
    let Some(win) = web_sys::window() else {
        return;
    };
    let onkeydown =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
            if mission_history::in_editable_field() {
                return;
            }
            if !(ev.ctrl_key() || ev.meta_key()) || ev.alt_key() {
                return;
            }
            match ev.code().as_str() {
                "KeyZ" if ev.shift_key() => {
                    mission_history::redo();
                }
                "KeyZ" => {
                    mission_history::undo();
                }
                "KeyY" if !ev.shift_key() => {
                    mission_history::redo();
                }
                _ => return,
            }
            ev.prevent_default();
        });
    let _ = win.add_event_listener_with_callback("keydown", onkeydown.as_ref().unchecked_ref());
    onkeydown.forget();
}
