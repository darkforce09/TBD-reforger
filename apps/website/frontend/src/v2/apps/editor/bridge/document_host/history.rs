//! Editor document history and render lane refresh.
#![cfg(target_arch = "wasm32")]

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::editor_context;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use website_map_engine::editing::tools::selection;

use leptos::prelude::{GetUntracked, RwSignal, Set};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use website_map_engine::data::store::MissionDocCore;
use website_map_engine::data::store::SlotSoa;
use website_map_engine::frame::engine::RenderEngine;
use website_map_engine::overlay::lanes::role_id;
use website_map_engine::overlay::symbology::links::squad_links::build_squad_link_segments;

use crate::v2::apps::editor::bridge::document_host::doc_host::DocHandle;
use crate::v2::apps::editor::bridge::tactical_graphics_authoring;
use selection::SelectionHandle;
use website_map_engine::frame::EngineHandle;

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
    dirty: RwSignal<bool>,
    restore_settled: Rc<Cell<bool>>,
}

type UnloadClosure = Closure<dyn FnMut(web_sys::Event)>;

thread_local! {
    static HISTORY_CTX: RefCell<Option<HistoryCtx>> = const { RefCell::new(None) };
    static UNLOAD_GUARD: RefCell<Option<UnloadClosure>> = const { RefCell::new(None) };
}

const UNSAVED_PROMPT: &str = "You have unsaved mission changes.";

/// Installs the document and render handles used by history commands.
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
    website_map_engine::editing::history::install_host(
        website_map_engine::editing::history::HistoryHost {
            after_document_change: after_local_edit,
        },
    );
}

/// Returns the active mission document handle when installed.
pub fn doc_handle() -> Option<crate::v2::apps::editor::bridge::document_host::doc_host::DocHandle> {
    HISTORY_CTX.with(|c| c.borrow().as_ref().map(|ctx| ctx.doc.clone()))
}

/// Sets the unsaved-changes state for the active mission.
pub fn set_dirty(value: bool) {
    HISTORY_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.dirty.set(value);
        }
    });
}

/// Reports whether the active mission has unsaved changes.
#[must_use]
pub fn is_dirty() -> bool {
    HISTORY_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.dirty.try_get_untracked())
            .unwrap_or(false)
    })
}

fn saves_to_server(mission_id: &str) -> bool {
    let b = mission_id.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, &c)| match i {
            8 | 13 | 18 | 23 => c == b'-',
            _ => c.is_ascii_hexdigit(),
        })
}

/// Prompts before unloading a dirty mission.
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

/// Removes the unload prompt for the current editor mount.
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

/// Applies one undo step and refreshes editor state.
pub fn undo() -> bool {
    website_map_engine::editing::history::undo()
}

/// Applies one redo step and refreshes editor state.
pub fn redo() -> bool {
    website_map_engine::editing::history::redo()
}

/// Refreshes history and presentation state after a local edit.
pub fn after_local_edit() {
    HISTORY_CTX.with(|c| {
        let guard = c.borrow();
        if let Some(ctx) = guard.as_ref() {
            after_doc_change(ctx);
        }
    });
}

/// Updates object and selection counts in the status bar.
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

fn prune_selection(ctx: &HistoryCtx) {
    let live = {
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };
        crate::v2::apps::editor::mission_editor::selectable_ids(
            &core.slots_json(),
            &core.small_maps_json(),
        )
    };
    ctx.selection
        .borrow_mut()
        .retain(|id| live.contains(id.as_str()));
}

/// Rebuilds engine lanes from the settled mission document.
pub fn rebind_engine_from_doc() {
    HISTORY_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let (soa, obj) = {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return;
            };
            (
                crate::v2::apps::editor::mission_editor::map_render_slot_soa(core),
                core.slot_count(),
            )
        };
        prune_selection(ctx);
        let ids = ctx.selection.borrow().clone();
        if let Some(e) = ctx.engine.borrow_mut().as_mut() {
            let tints =
                website_map_engine::overlay::symbology::roles::classify::side_tints_rgba_bytes(
                    &soa.side_keys,
                );
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
                upload_tactical_graphics(e, doc);
            }
        }
        refresh_signals(ctx, obj);
    });
}

/// Updates selection presentation after a document change.
pub fn refresh_selection() {
    HISTORY_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        ctx.sel_count.set(ctx.selection.borrow().len());
    });
    editor_context::refresh_selection_mirrors();
}

fn after_doc_change(ctx: &HistoryCtx) {
    let (soa, obj) = {
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };
        (
            crate::v2::apps::editor::mission_editor::map_render_slot_soa(core),
            core.slot_count(),
        )
    };
    prune_selection(ctx);
    let ids = ctx.selection.borrow().clone();
    if let Some(e) = ctx.engine.borrow_mut().as_mut() {
        e.set_drag(Vec::new(), 0.0, 0.0); // clear any live drag overlay
        let tints = website_map_engine::overlay::symbology::roles::classify::side_tints_rgba_bytes(
            &soa.side_keys,
        );
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
            upload_tactical_graphics(e, doc);
        }
    }
    ctx.doc_ver.set(ctx.doc_ver.get().saturating_add(1));
    ctx.dirty.set(true); // A committed edit is unsaved work.

    if ctx.restore_settled.get() {
        crate::v2::apps::editor::shell::persist::schedule_edit_persist(
            ctx.doc.clone(),
            &ctx.mission_id,
        );
    }
    refresh_signals(ctx, obj);
}

mod render_lanes;
pub use render_lanes::refresh_tactical_lane;
use render_lanes::{
    comment_lane_ids, comment_lane_xy, marker_lane_xy_tints, upload_squad_links,
    upload_tactical_graphics,
};
pub(crate) use render_lanes::{soa_roles, vehicle_lane_fields};

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
    editor_context::refresh_docks();
}

/// Reports whether focus is inside an editable field.
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
    if el
        .dyn_ref::<web_sys::HtmlElement>()
        .is_some_and(web_sys::HtmlElement::is_content_editable)
    {
        return true;
    }
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

/// Installs the editor history command bridge.
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
