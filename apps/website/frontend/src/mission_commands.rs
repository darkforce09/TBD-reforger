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
//!
//! **T-417 — Class-R helpers** ([`compiled_export_text`], [`row_meta_missing_message`]) are ungated so
//! native `cargo test` can pin them. The wasm transport body lives in [`imp`].

/// Format the compact `/compiled` bytes for the Export Compiled download.
///
/// Returns the wire UTF-8 text **byte-identical** to `flatten_mod_document_json` / the compiled
/// route. Deliberately does **not** re-parse through `serde_json::Value` for a "pretty" download:
/// that round-trip is not byte-identical (whitespace), and without `preserve_order` it also
/// BTreeMap-sorts keys — the false "whitespace-only" comment on the old path set a trap for any
/// harness that compares against `GET /missions/:id/compiled`.
pub(crate) fn compiled_export_text(doc: &[u8]) -> Result<String, String> {
    String::from_utf8(doc.to_vec()).map_err(|e| format!("compiled document is not UTF-8: {e}"))
}

/// Author-facing message when [`ROW_META`] never arrived.
///
/// `authenticated == false` means the session is missing/expired (hydrate 401 never sets the row);
/// do not tell the author to "save a version first" — that path would also 401.
pub(crate) fn row_meta_missing_message(authenticated: bool) -> &'static str {
    if authenticated {
        "This mission has no saved server row yet — save a version first, then export."
    } else {
        "Sign in to export the compiled mission — your session is missing or expired."
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::RefCell;

    use leptos::prelude::{GetUntracked, RwSignal, Set};
    use leptos::task::spawn_local;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    use map_engine_core::mission::compile::{compile_export, compile_payload, version_body};
    use map_engine_core::mission::flatten::{flatten_mod_document_json, MissionMeta};

    use crate::auth::AuthStore;
    use crate::mission_doc::DocHandle;

    use super::{compiled_export_text, row_meta_missing_message};

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
        /// local-only / non-UUID id (the `smoke` gate route), a 404, an offline boot, **or a 401 /
        /// expired session** (hydrate never got the row; see [`row_meta_missing_message`]). There is
        /// no server document for those, and `MissionMeta::default()` would happily compile one with a
        /// blank author and `playerRange: [1, 1]`. Emitting that under the name "the document the game
        /// server will receive" is the confident-wrong-answer failure this whole ticket exists to
        /// avoid, so [`export_compiled_now`] refuses instead — and names auth failure separately from
        /// "no saved version".
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
    /// Returns the compact compiled mod document (byte-identical to `GET /compiled`'s body when the
    /// local doc matches the saved version), or a message fit to show an author.
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
    /// Returns a display message when the row never arrived (local-only id, 404, offline, **401 /
    /// expired session** — see [`ROW_META`] + [`row_meta_missing_message`]), when the editor is not
    /// mounted, or when the compile refuses (no placed slots is the common one, and it is the same
    /// `409` a game server would get).
    pub fn compiled_document_json() -> Result<String, String> {
        let Some(snap) = snapshot() else {
            return Err("Editor not ready".to_string());
        };
        let authenticated = snap.auth.access_token.get_untracked().is_some()
            && snap.auth.user.get_untracked().is_some();
        let Some(mut meta) = ROW_META.with(|r| r.borrow().as_ref().map(clone_meta)) else {
            return Err(row_meta_missing_message(authenticated).to_string());
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
        // T-417 — ship the compact wire bytes (byte-identical to `/compiled`). Do not re-parse to
        // `serde_json::Value` for a "pretty" download — that is not whitespace-only vs the route.
        compiled_export_text(&doc)
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
    ///
    /// **T-243 adds `compiled_document_json()`** — the same bytes the "Export Compiled" button
    /// downloads. It is on the bridge for the reason the other two are: a compile whose only entry
    /// point is a `<button>` and a `Blob` download is a compile no harness can read back, and this one
    /// makes a claim worth checking against a live `GET /missions/:id/compiled`. Unlike its two peers
    /// it pins nothing: its whole value is being the real output. On a failure it returns the same
    /// author-facing message the toast shows (a plain string either way — the caller can tell them
    /// apart by parsing).
    pub fn register_editor_commands(doc: DocHandle) {
        let obj = js_sys::Object::new();

        let compiled_doc = Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&compiled_document_json().unwrap_or_else(|e| e))
        }) as Box<dyn FnMut() -> JsValue>);

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
        let _ = js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("compiled_document_json"),
            compiled_doc.as_ref(),
        );
        if let Some(win) = web_sys::window() {
            let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCommands"), &obj);
        }
        // Leaked like the other editor bridges (harness reads them across the page lifetime).
        compile_save.forget();
        compile_export_fn.forget();
        compiled_doc.forget();
    }
}

#[cfg(target_arch = "wasm32")]
pub use imp::*;

#[cfg(test)]
mod tests {
    use super::{compiled_export_text, row_meta_missing_message};

    /// Measured wire key order from `GET /compiled` (ticket T-417 / ModMissionDocument field order).
    const WIRE_ORDER_COMPACT: &[u8] = br#"{"schemaVersion":"1.1","meta":{"id":"m","title":"t","author":"a","terrain":"everon","playerRange":[1,1]},"environment":{"timeOfDay":"0800","weatherPreset":"clear"},"factions":[],"orbat":{},"slots":[{"id":"s1"}],"radioPlan":{"nets":[]},"zones":[],"flow":{"briefingDurationSec":0},"winConditions":{"mode":"none"}}"#;

    #[test]
    fn class_r_compiled_export_is_byte_identical_to_wire() {
        let out = compiled_export_text(WIRE_ORDER_COMPACT).expect("utf-8");
        assert_eq!(
            out.as_bytes(),
            WIRE_ORDER_COMPACT,
            "Export Compiled must ship compact wire bytes — not a Value pretty-print"
        );
        // Top-level key order pin (the measured /compiled order from T-417).
        // Read order from the raw text — `serde_json::Value` would BTreeMap-sort without preserve_order.
        assert_eq!(
            raw_top_level_keys(&out),
            [
                "schemaVersion",
                "meta",
                "environment",
                "factions",
                "orbat",
                "slots",
                "radioPlan",
                "zones",
                "flow",
                "winConditions",
            ]
        );
    }

    #[test]
    fn class_r_value_pretty_print_is_not_byte_identical() {
        // Even with serde_json `preserve_order` (unified from map-engine-core / T-220), a
        // Value→pretty round-trip changes bytes (whitespace). The old comment claiming
        // "whitespace-only" while inviting a live `/compiled` compare set a trap; shipping
        // compact makes the download byte-identical to the flatten/`/compiled` body.
        let value: serde_json::Value = serde_json::from_slice(WIRE_ORDER_COMPACT).unwrap();
        let pretty = serde_json::to_string_pretty(&value).unwrap();
        assert_ne!(
            pretty.as_bytes(),
            WIRE_ORDER_COMPACT,
            "pretty-print must differ from compact wire bytes"
        );
        let shipped = compiled_export_text(WIRE_ORDER_COMPACT).unwrap();
        assert_eq!(shipped.as_bytes(), WIRE_ORDER_COMPACT);
        // Key order must still match wire (preserve_order keeps it; compact never reorders).
        assert_eq!(raw_top_level_keys(&shipped), raw_top_level_keys(&pretty));
        assert_eq!(
            raw_top_level_keys(&shipped)[0],
            "schemaVersion",
            "wire order starts with schemaVersion, not alpha environment"
        );
    }

    #[test]
    fn class_r_auth_failure_is_not_no_saved_row() {
        let auth = row_meta_missing_message(false);
        let no_row = row_meta_missing_message(true);
        assert!(
            auth.contains("Sign in") || auth.contains("session"),
            "unauthenticated must name auth, got: {auth}"
        );
        assert!(
            !auth.contains("save a version first"),
            "auth failure must not suggest save-first: {auth}"
        );
        assert!(
            no_row.contains("save a version first"),
            "authenticated-but-no-row keeps the save-first remedy: {no_row}"
        );
    }

    /// T-417 Class-R — the Export Compiled download must ship the compact wire bytes.
    ///
    /// # Cure 2 (scrub-then-grep), and why not cure 1 (T-601)
    ///
    /// This is the closest call of the six pins T-601 converted, because the invariant *does* have
    /// a runtime signature: [`compiled_export_text`]. But that half is already pinned by value —
    /// [`class_r_compiled_export_is_byte_identical_to_wire`] feeds it the golden wire bytes and
    /// asserts the output is byte-identical. What is left over is only "the download path calls
    /// it", and [`compiled_document_json`] lives inside `#[cfg(target_arch = "wasm32")] mod imp`
    /// on top of `snapshot()`, `ROW_META`, `compile_payload` and `flatten_mod_document_json`. A
    /// cure-1 harness would have to model four crate seams, and a pin whose preamble is bigger
    /// than the code under test is a pin nobody will keep honest. Recorded as a deliberate choice,
    /// not an oversight: if the wasm boundary ever moves, this is the pin to promote.
    ///
    /// T-601 replaced the ad-hoc `split(…).nth(1).split("fn clone_meta")` slice — which silently
    /// depended on `clone_meta` staying the next item, and would have returned the *first* of two
    /// `compiled_document_json` definitions without a word — with the shared scrubber's
    /// ambiguity-refusing extractor.
    #[test]
    fn class_r_source_forbids_value_pretty_on_compiled_export() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        const SRC: &str = include_str!("mission_commands.rs");
        let production = live_code(SRC);
        let code = only_body(&production, "pub fn compiled_document_json()");
        assert!(
            code.contains("compiled_export_text(&doc)"),
            "compiled_document_json must ship via compiled_export_text"
        );
        assert!(
            !code.contains("to_string_pretty"),
            "compiled_document_json must not pretty-print the compiled doc"
        );
        // A live `serde_json::Value` binding in the return path is the old defect.
        assert!(
            !code.contains("let value: serde_json::Value"),
            "compiled_document_json must not re-parse through Value for download"
        );
        // The ROW_META docs are PROSE, so they are read from the raw file on purpose — the
        // scrubber's whole job is to delete prose, and asserting a doc string against scrubbed
        // source would be a pin that can only ever fail.
        let prose = SRC.split("mod tests {").next().expect("tests marker");
        assert!(
            prose.contains("401") && prose.contains("expired session"),
            "ROW_META docs must name 401 / expired session"
        );
    }

    /// **T-601 — calibration for the export pin above.**
    ///
    /// The needle it cannot do without is `compiled_export_text(&doc)`. Every wrapper in the
    /// battery must stop satisfying it, or a `to_string_pretty` download could ship while this
    /// pin reported the compact wire path was live.
    #[test]
    fn the_export_pin_rejects_every_dead_code_wrapper() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let needle = "compiled_export_text(&doc)";
        let attacks: [(&str, String); 12] = [
            (
                "if true == false",
                format!("if true == false {{ {needle}; }}"),
            ),
            ("loop { break; … }", format!("loop {{ break; {needle}; }}")),
            (
                "#[cfg(any())]",
                format!("#[cfg(any())] fn d() {{ {needle}; }}"),
            ),
            ("while false", format!("while false {{ {needle}; }}")),
            ("if !true", format!("if !true {{ {needle}; }}")),
            ("if 1 > 2", format!("if 1 > 2 {{ {needle}; }}")),
            (
                "if std::hint::black_box(false)",
                format!("if std::hint::black_box(false) {{ {needle}; }}"),
            ),
            (
                "const C: bool = false; if C",
                format!("const C: bool = false;\nfn d() {{ if C {{ {needle}; }} }}"),
            ),
            ("return; above", format!("fn d() {{ return; {needle}; }}")),
            (
                "#[cfg(any())] mod shadow",
                format!("#[cfg(any())] mod shadow {{ fn d() {{ {needle}; }} }}"),
            ),
            (
                "match guard",
                format!("match () {{ _ if false => {{ {needle}; }} _ => {{}} }}"),
            ),
            ("comment", format!("// {needle}")),
        ];
        for (label, body) in attacks {
            let forged =
                format!("pub fn compiled_document_json() {{\n    {body}\n}}\n#[cfg(test)]\n");
            assert!(
                !live_code(&forged).contains(needle),
                "{label}: the compact-bytes needle survived scrubbing — this pin would report a \
                 live wire-bytes download over code the build never runs"
            );
        }
        for (label, forged) in [
            (
                "shadow copy in a live mod, no cfg",
                "pub fn compiled_document_json() { good(); }\n\
                 mod real { pub fn compiled_document_json() { bad(); } }\n#[cfg(test)]\n",
            ),
            (
                "shadow copy in an impl",
                "pub fn compiled_document_json() { good(); }\n\
                 impl T { pub fn compiled_document_json() { bad(); } }\n#[cfg(test)]\n",
            ),
        ] {
            let scrubbed = live_code(forged);
            let caught = std::panic::catch_unwind(|| {
                only_body(&scrubbed, "pub fn compiled_document_json()")
            })
            .is_err();
            assert!(
                caught,
                "{label}: the old `split(…).nth(1)` slice would have taken the first of two \
                 definitions without saying so"
            );
        }
        let live = format!("pub fn compiled_document_json() {{\n    {needle}\n}}\n#[cfg(test)]\n");
        assert!(live_code(&live).contains(needle));
    }

    /// First-level object key order from a JSON object string (no full parse → no Map reorder).
    fn raw_top_level_keys(json: &str) -> Vec<&str> {
        let mut keys = Vec::new();
        let bytes = json.as_bytes();
        let mut i = 0;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        assert_eq!(bytes.get(i), Some(&b'{'));
        i += 1;
        let mut depth = 1u32;
        let mut in_string = false;
        let mut escape = false;
        let mut key_start: Option<usize> = None;
        let mut expect_key = true;
        while i < bytes.len() {
            let b = bytes[i];
            if in_string {
                if escape {
                    escape = false;
                } else if b == b'\\' {
                    escape = true;
                } else if b == b'"' {
                    in_string = false;
                    if expect_key && depth == 1 {
                        if let Some(s) = key_start {
                            keys.push(&json[s..i]);
                            expect_key = false;
                        }
                    }
                    key_start = None;
                }
                i += 1;
                continue;
            }
            match b {
                b'"' => {
                    in_string = true;
                    if expect_key && depth == 1 {
                        key_start = Some(i + 1);
                    }
                }
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                b',' if depth == 1 => expect_key = true,
                b':' if depth == 1 => expect_key = false,
                _ => {}
            }
            i += 1;
        }
        keys
    }
}
