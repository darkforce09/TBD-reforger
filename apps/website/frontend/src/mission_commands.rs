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
use map_engine_core::mission::flatten::{flatten_mod_document_json, MissionMeta};

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

    /// **T-243 — the mission ROW, as `GET /missions/:id` last served it.**
    ///
    /// Deliberately NOT part of [`EditorCtx`]: `set_ctx` runs synchronously at mount, and this
    /// arrives later from `mission_hydrate::hydrate_from_server`'s `await`. Folding it in would
    /// have meant either an `Option` field nobody could keep honest or an ordering assumption
    /// between a mount and a fetch.
    ///
    /// **`None` is load-bearing and must stay refusable.** It means the row never arrived — a
    /// local-only / non-UUID id (the `smoke` gate route), a 404, an offline boot. There is no
    /// server document for those, and `MissionMeta::default()` would happily compile one with a
    /// blank author and `playerRange: [1, 1]`. Emitting that under the name "the document the game
    /// server will receive" is the confident-wrong-answer failure this whole ticket exists to
    /// avoid, so [`export_compiled_now`] refuses instead.
    static ROW_META: RefCell<Option<MissionMeta>> = const { RefCell::new(None) };
}

/// Record the mission row for the server-truth Export (T-243). Called by
/// `mission_hydrate::hydrate_from_server` on every successful `GET /missions/:id`, including the
/// fresh-mission and warm-IDB branches — the row is what the compile needs, and it is equally real
/// whichever way the payload half was resolved.
pub fn set_row_meta(detail: &crate::dto::MissionDetail) {
    ROW_META.with(|r| *r.borrow_mut() = Some(detail.compiled_meta()));
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

/// **T-243 — download the document the game server will actually receive.**
///
/// Returns the pretty-printed compiled mod document, or a message fit to show an author.
///
/// ## Why this exists at all
///
/// `GET /missions/:id/compiled` takes a `ServiceAuth` (`handlers::missions::get_compiled_mission`)
/// — it answers game servers, not browsers — so an author has no way to fetch it. Until this,
/// "Export JSON" downloaded [`compile_export`]'s `MissionExport` envelope: the editor SUPERSET,
/// `{exportFormatVersion, missionId, title, …, payload}`, whose `payload` is the editor graph. That
/// is the right file for re-importing into the editor and it is **not** the mod document — it has
/// no `slots[]`, no `orbat`, no `radioPlan`, no `winConditions`, and the mod cannot load it. So
/// there was no way to see the compiled document before a game server did.
///
/// ## Why the answer can be trusted
///
/// This runs `flatten_mod_document_json` — the same `map-engine-core` compile `/compiled` runs,
/// over the same two inputs:
///
///   * the **row**, from `GET /missions/:id` ([`ROW_META`]), which is where the server gets
///     `author`, `maxPlayers` and the fallback time/weather;
///   * the **save-shaped payload** (`include_orbat = false`) — byte-for-byte what
///     `POST /missions/:id/versions` stores and therefore what `/compiled` later reads. `orbat` is
///     omitted for the same reason the save omits it: the flatten derives its own from `editor`,
///     and including it would put a key in the preview's input that the stored version never has.
///
/// Their agreement is not asserted here — it is pinned natively, on both halves, by
/// `website-api`'s `client_twin_is_byte_identical_to_the_compiled_route` (the compile) and
/// `dto::r_api::compiled_meta_is_the_row_the_server_compiles_from` (the row).
///
/// ## The one honest difference, and it is the point
///
/// `/compiled` serves the last **saved** version; this compiles the document **as it is now**,
/// unsaved edits included. That is what makes it useful — you can see what a save would ship
/// before shipping it — but it means a dirty document previews something the server does not yet
/// have. The caller says so in the toast rather than hiding it.
///
/// # Errors
/// Returns a display message when the row never arrived (local-only id, 404, offline — see
/// [`ROW_META`]), when the editor is not mounted, or when the compile refuses (no placed slots is
/// the common one, and it is the same `409` a game server would get).
pub fn compiled_document_json() -> Result<String, String> {
    let Some(snap) = snapshot() else {
        return Err("Editor not ready".to_string());
    };
    let Some(mut meta) = ROW_META.with(|r| r.borrow().as_ref().map(clone_meta)) else {
        return Err(
            "This mission has no saved server row yet — save a version first, then export."
                .to_string(),
        );
    };
    // Carry the mission id from the route when the row's is blank, matching the envelope export's
    // `meta.id`-then-route fallback (`compile_export`). `mission_doc_id` in the flatten normalizes
    // whatever lands here into the schema's id space either way.
    if meta.id.is_empty() {
        meta.id = snap.mission_id.clone();
    }
    let payload = compile_payload(&snap.small, &snap.slots, false);
    let payload_bytes = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    let meta_bytes = serde_json::to_vec(&meta).map_err(|e| e.to_string())?;

    let doc = flatten_mod_document_json(&meta_bytes, &payload_bytes)?;
    // Re-parse to pretty-print. The COMPACT bytes are the contract (they are what `/compiled`
    // serves); this is a whitespace-only reformat for a human reading the file, and it is done here
    // rather than in core so the shared compile keeps returning the server's exact bytes.
    let value: serde_json::Value = serde_json::from_slice(&doc).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
}

/// `MissionMeta` is a plain data carrier in core and deliberately not `Clone` (it is an input type
/// built once per compile); this is the local copy out of the `thread_local` so no borrow is held
/// across the compile below.
fn clone_meta(m: &MissionMeta) -> MissionMeta {
    MissionMeta {
        id: m.id.clone(),
        title: m.title.clone(),
        author: m.author.clone(),
        terrain: m.terrain.clone(),
        custom_terrain_name: m.custom_terrain_name.clone(),
        max_players: m.max_players,
        time_of_day: m.time_of_day.clone(),
        weather_preset: m.weather_preset.clone(),
    }
}

/// Trigger the server-truth download and report the outcome (T-243). `toasts` is resolved at
/// component setup by the caller — `use_toasts()` is an `expect_context` and would panic from a
/// DOM handler, the `RowMirror` precedent.
pub fn export_compiled_now(toasts: crate::toast::Toasts) {
    let mission_id = EDITOR_CTX
        .with(|c| c.borrow().as_ref().map(|ctx| ctx.mission_id.clone()))
        .unwrap_or_default();
    match compiled_document_json() {
        Ok(json) => {
            let filename = format!("mission-{mission_id}.compiled.json");
            if let Err(e) = download_json(&filename, &json) {
                toasts.error(format!("Could not start the download: {e:?}"));
                return;
            }
            // Naming the staleness is the whole reason this is a toast and not a silent download:
            // the file is the CURRENT document, which is only what a game server would fetch once
            // this state is saved.
            if crate::mission_history::is_dirty() {
                toasts.message(
                    "Downloaded the compiled mission document — compiled from your unsaved changes, \
                     so the server still serves the last saved version.",
                );
            } else {
                toasts.success("Downloaded the compiled mission document.");
            }
        }
        Err(e) => toasts.error(e),
    }
}

/// Export the current mission as a downloaded `mission-<id>.json` (React `exportJson`): compile with
/// `orbat` included, wrap in the `MissionExport` envelope, pretty-print, and trigger the browser
/// download. `version` is the current semver (envelope `version` field).
///
/// **This is the editor SUPERSET, not the mod document** — it round-trips back into the editor and
/// the mod cannot load it. The compiled document an author ships is
/// [`export_compiled_now`] (T-243); both are kept because they answer different questions.
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
///
/// T-181.44 — a 400 from `create_version` carries the *list* of things wrong with the payload
/// (schema violations plus the wire-safety findings), and this used to collapse all of it into
/// "Save failed (400)". `findings` takes the per-problem lines so the dialog can name them; the
/// headline stays short because `status` is also rendered in the top strip.
pub fn save_now(
    semver: String,
    notes: String,
    status: RwSignal<String>,
    findings: RwSignal<Vec<String>>,
) {
    findings.set(Vec::new());
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
                // T-191 fix — expire the conflict backup pair. This 201 is the one moment those
                // whole-document IDB records stop being anybody's last copy, and nothing else ever
                // deleted them: they accumulated one doc per mission ever conflicted, forever, while
                // `__missionBackup.has()` kept offering a weeks-old document that a restore would
                // swap over current work. Rationale in `mission_hydrate::clear_local_backups`.
                crate::mission_hydrate::clear_local_backups(&mission_id);
                if let Some(sig) = semver_signal() {
                    sig.set(Some(semver.clone()));
                }
            }
            Err((409, _)) => status.set(format!("Version {semver} already exists")),
            Err((413, _)) => status.set("Payload too large".to_string()),
            Err((401, _)) => status.set("Sign in to save".to_string()),
            Err((s, msg)) => {
                let (head, rows) = crate::client::split_error_lines(msg.as_deref());
                let head = head.filter(|h| !h.is_empty());
                status.set(match (&head, rows.len()) {
                    (Some(h), 0) => format!("Save rejected ({s}): {h}"),
                    (Some(h), n) => format!("Save rejected ({s}): {h} — {n} problem(s) below"),
                    (None, _) => format!("Save failed ({s})"),
                });
                findings.set(rows);
            }
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
pub(crate) fn download_json(filename: &str, contents: &str) -> Result<(), JsValue> {
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
