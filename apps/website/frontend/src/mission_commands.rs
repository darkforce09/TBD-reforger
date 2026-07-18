//! T-159.20 — Save Version + Export commands for the Leptos Mission Creator.
//!
//! The compile itself is pure Rust in `map-engine-core` (`mission::compile`, unit-tested natively);
//! this module is the thin wasm glue that (a) reads the hosted `MissionDocCore`, (b) POSTs the Save
//! Version body through the authed `api_post`, (c) triggers the Export file download, and (d) installs
//! the `window.__editorCommands` smoke bridge (peer of `__missionDoc`).
//!
//! The doc/auth/mission-id live in a `thread_local` [`EditorCtx`] set from the editor's `on_load` —
//! the wasm-only `DocHandle` type can't cross the `#[cfg(target_arch = "wasm32")]` boundary into the
//! native view shell, so the buttons reach it through here instead of a hoisted handle. Every read is
//! taken as an owned snapshot before any `.await`, so no `RefCell` borrow is ever held across a yield.
#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;

use leptos::prelude::{RwSignal, Set};
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use map_engine_core::mission::compile::{compile_export, compile_payload, version_body};

use crate::auth::AuthStore;
use crate::mission_doc::DocHandle;

/// Editor context shared from `mission_editor::on_load` to the Save/Export buttons. `AuthStore` is
/// `Copy`; `doc` is the same shared `Rc` the persistence layer may swap on IDB restore (reads see the
/// swap). Held in a `thread_local` because `DocHandle` is `!Send` + wasm-only.
struct EditorCtx {
    doc: DocHandle,
    auth: AuthStore,
    mission_id: String,
    /// T-159.26 — the adopted server semver signal, updated on a successful Save (the saved
    /// version becomes the version local now derives from).
    current_semver: RwSignal<Option<String>>,
}

thread_local! {
    static EDITOR_CTX: RefCell<Option<EditorCtx>> = const { RefCell::new(None) };
}

/// Install the editor context (called once from `on_load`, after the doc is seeded/registered).
pub fn set_ctx(
    doc: DocHandle,
    auth: AuthStore,
    mission_id: String,
    current_semver: RwSignal<Option<String>>,
) {
    EDITOR_CTX.with(|c| {
        *c.borrow_mut() = Some(EditorCtx {
            doc,
            auth,
            mission_id,
            current_semver,
        });
    });
}

/// The current-semver signal, for the save-success adopt. `None` when the editor isn't mounted.
fn semver_signal() -> Option<RwSignal<Option<String>>> {
    EDITOR_CTX.with(|c| c.borrow().as_ref().map(|ctx| ctx.current_semver))
}

/// An owned snapshot of everything a command needs — taken synchronously so no borrow spans an
/// `.await`. `None` when the editor isn't mounted / the doc Option is empty.
struct Snap {
    small: String,
    slots: String,
    auth: AuthStore,
    mission_id: String,
}

fn snapshot() -> Option<Snap> {
    EDITOR_CTX.with(|c| {
        let ctx = c.borrow();
        let ctx = ctx.as_ref()?;
        let doc = ctx.doc.borrow();
        let core = doc.as_ref()?;
        Some(Snap {
            small: core.small_maps_json(),
            slots: core.slots_json(),
            auth: ctx.auth,
            mission_id: ctx.mission_id.clone(),
        })
    })
}

/// Export the current mission as a downloaded `mission-<id>.json` (React `exportJson`): compile with
/// `orbat` included, wrap in the `MissionExport` envelope, pretty-print, and trigger the browser
/// download. `version` is the current semver (envelope `version` field).
pub fn export_now(version: &str) {
    let Some(snap) = snapshot() else {
        return;
    };
    let payload = compile_payload(&snap.small, &snap.slots, true);
    let doc = compile_export(
        &payload,
        &snap.small,
        &snap.mission_id,
        version,
        &js_date_iso(),
    );
    let json = serde_json::to_string_pretty(&doc).unwrap_or_default();
    let filename = format!("mission-{}.json", snap.mission_id);
    let _ = download_json(&filename, &json);
}

/// Save a new immutable version (React `saveVersion`): compile with `orbat` omitted (the server
/// re-derives), POST `{semver, editor_notes, payload}` to `/missions/:id/versions`, and reflect the
/// outcome in `status`. 409 = dup semver, 413 = too large, 401 = not signed in.
pub fn save_now(semver: String, notes: String, status: RwSignal<String>) {
    let Some(snap) = snapshot() else {
        status.set("Editor not ready".to_string());
        return;
    };
    let payload = compile_payload(&snap.small, &snap.slots, false);
    let body = version_body(&semver, &notes, &payload);
    let auth = snap.auth;
    let path = format!("/missions/{}/versions", snap.mission_id);
    let mission_id = snap.mission_id.clone();
    status.set(format!("Saving v{semver}…"));
    spawn_local(async move {
        match crate::client::api_post::<serde_json::Value>(auth, &path, body).await {
            Ok(_) => {
                status.set(format!("Saved v{semver}"));
                // T-159.26 — the saved version is now what local derives from: clear the dirty
                // flag, adopt the semver (cross-tab conflict skip), and update the current-semver
                // signal so a later Export/adopt uses it.
                crate::mission_history::set_dirty(false);
                crate::editor_session::mark_adopted(&mission_id, Some(&semver));
                if let Some(sig) = semver_signal() {
                    sig.set(Some(semver.clone()));
                }
            }
            Err((409, _)) => status.set(format!("Version {semver} already exists")),
            Err((413, _)) => status.set("Payload too large".to_string()),
            Err((401, _)) => status.set("Sign in to save".to_string()),
            Err((s, _)) => status.set(format!("Save failed ({s})")),
        }
    });
}

/// Current wall-clock ISO-8601 (`new Date().toISOString()`) — the one clock read, kept out of the
/// pure core (which takes `exported_at` as a param, so the smoke can pin it).
fn js_date_iso() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}

/// The `Blob → URL.createObjectURL → <a download> → click → revokeObjectURL` download dance
/// (mirrors the React `exportJson` DOM path).
fn download_json(filename: &str, contents: &str) -> Result<(), JsValue> {
    let win = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = win
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(contents));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("application/json");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(parts.as_ref(), &opts)?;

    let url = web_sys::Url::create_object_url_with_blob(&blob)?;

    let anchor = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    let el: &web_sys::HtmlElement = anchor.as_ref();
    el.click();

    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}

/// Install `window.__editorCommands` — the read-only compile smoke bridge (peer of `__missionDoc`,
/// same leaked-closure `js_sys::Object` idiom as `register_mission_doc`). `compile_save_json()` and
/// `compile_export_json()` return the compiled JSON strings; the export path pins `exportedAt` +
/// `missionId`/`version` to fixed values so the gate output is byte-deterministic.
pub fn register_editor_commands(doc: DocHandle) {
    let obj = js_sys::Object::new();

    let compile_save = {
        let doc = doc.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let json = doc
                .borrow()
                .as_ref()
                .map(|c| {
                    let payload = compile_payload(&c.small_maps_json(), &c.slots_json(), false);
                    serde_json::to_string(&payload).unwrap_or_default()
                })
                .unwrap_or_default();
            JsValue::from_str(&json)
        }) as Box<dyn FnMut() -> JsValue>)
    };
    let compile_export_fn = {
        let doc = doc.clone();
        Closure::wrap(Box::new(move || -> JsValue {
            let json = doc
                .borrow()
                .as_ref()
                .map(|c| {
                    let small = c.small_maps_json();
                    let payload = compile_payload(&small, &c.slots_json(), true);
                    let env = compile_export(
                        &payload,
                        &small,
                        "smoke",
                        "0.1.0",
                        "1970-01-01T00:00:00.000Z",
                    );
                    serde_json::to_string(&env).unwrap_or_default()
                })
                .unwrap_or_default();
            JsValue::from_str(&json)
        }) as Box<dyn FnMut() -> JsValue>)
    };

    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("compile_save_json"),
        compile_save.as_ref(),
    );
    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("compile_export_json"),
        compile_export_fn.as_ref(),
    );
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCommands"), &obj);
    }
    // Leaked like the other editor bridges (harness reads them across the page lifetime).
    compile_save.forget();
    compile_export_fn.forget();
}
