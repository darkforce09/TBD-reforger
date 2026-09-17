//! Reconciles server mission state with a local draft and keeps recoverable snapshots.
//! Server adoption follows the local-versus-server content verdict. Snapshot slots preserve
//! the displaced document across an adoption or restore and are scoped to the current owner.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use website_map_engine::data::store::MissionDocCore;
use website_map_engine::editing::persist::local_versus_server::{
    classify_local_draft, server_slot_count, LocalDraftVerdict,
};
use website_map_engine::editing::persist::mission_id::is_uuid;
use website_map_engine::editing::persist::record_key::snapshot_key;
use website_map_engine::editing::persist::server_adoption::{
    adopt_payload, apply_row_meta_only, Adopt, RowMeta,
};
use website_map_engine::editing::persist::snapshot_slot::{
    capture_document_snapshot, SnapshotSlot,
};

use crate::v2::apps::editor::bridge::document_host::doc_host::DocHandle;
use crate::v2::apps::editor::bridge::document_host::history::after_local_edit;
use crate::v2::apps::editor::shell::tab_lock;
use crate::v2::core::api::dto::MissionDetail;
use crate::v2::core::auth::AuthStore;

mod server_reconciliation;
pub use server_reconciliation::{
    hydrate_from_server, resolve_conflict_local, resolve_conflict_server,
};
mod snapshot_recovery;
pub use snapshot_recovery::{
    clear_local_backups, purge_local_documents, restore_local_backup, undo_local_restore,
};
use snapshot_recovery::{
    forget_snapshot, has_snapshot, live_editor_is, set_live_editor, snapshot_local,
};

/// Toast without `expect_context`. [`restore_snapshot`] can be driven from a JS bridge closure,
/// which has no reactive Owner, and `use_toasts()` would panic there  a panic in the middle of a
/// recovery being the worst possible time for one.
fn notify(msg: &str) {
    if let Some(toasts) = use_context::<crate::v2::core::ui::toast::Toasts>() {
        toasts.message(msg);
    }
}

/// Install `window.__missionBackup`  the recovery surface for the snapshot pair, and the peer of
/// `__missionDoc` / `__missionPersist` / `__editorHistory` (a `js_sys::Object` of `.forget()`'d
/// closures). Four Promise-returning verbs, two symmetric halves:
///   * `has()`           → bool  is a pre-adopt snapshot on record for this mission?
///   * `restore()`       → bool  swap it back over the live document.
///   * `hasUndoRestore()`→ bool  is the document a restore displaced still on record?
///   * `undoRestore()`   → bool  swap *that* back; the exact inverse of `restore()`.
///
/// The bridge mutates the live document because a backup must be restorable. Both restore
/// commands refuse a mission that is not mounted in the current editor.
fn register_mission_backup(mission_id: String, doc: &DocHandle) {
    set_live_editor(&mission_id, doc);
    let obj = js_sys::Object::new();

    let has_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let id = id.clone();
            wasm_bindgen_futures::future_to_promise(async move {
                Ok(JsValue::from_bool(
                    has_snapshot(&id, SnapshotSlot::PreAdopt).await,
                ))
            })
            .into()
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let restore_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let id = id.clone();
            wasm_bindgen_futures::future_to_promise(async move {
                Ok(JsValue::from_bool(restore_local_backup(id).await))
            })
            .into()
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let has_undo_fn = {
        let id = mission_id.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let id = id.clone();
            wasm_bindgen_futures::future_to_promise(async move {
                Ok(JsValue::from_bool(
                    has_snapshot(&id, SnapshotSlot::PreRestore).await,
                ))
            })
            .into()
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let undo_restore_fn = Closure::wrap(Box::new(move || -> JsValue {
        let id = mission_id.clone();
        wasm_bindgen_futures::future_to_promise(async move {
            Ok(JsValue::from_bool(undo_local_restore(id).await))
        })
        .into()
    }) as Box<dyn FnMut() -> JsValue>);

    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("has"), has_fn.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("restore"), restore_fn.as_ref());
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("hasUndoRestore"),
        has_undo_fn.as_ref(),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("undoRestore"),
        undo_restore_fn.as_ref(),
    );
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__missionBackup"), &obj);
    }
    // Read across the page lifetime; leak like every other editor bridge.
    has_fn.forget();
    restore_fn.forget();
    has_undo_fn.forget();
    undo_restore_fn.forget();
}

//  /  /  Class-R live in `mission_title_prefer` so they run on native
// `cargo test -p website-frontend` (this file is `#![cfg(target_arch = "wasm32")]`).
//  pins both briefing Option wires into apply_row_meta (: None at both sites
// stayed green on website-frontend until this ratchet).
