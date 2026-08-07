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

/// T-690 — the one-line author-facing summary of a compile's structured findings.
///
/// ## Why this replaces the toast rather than joining it
///
/// The compile used to say one of two things: "Downloaded the compiled mission document." or an
/// error. Everything it LEARNED — which authored values it discarded, and whose — was thrown away.
/// FNF v4's `init3DEN.sqf` is rated the single thing that framework does better than anyone else
/// precisely because Export is a build step there; TBD had the build step and none of the build
/// system. The findings now ride alongside the bytes
/// (`flatten::flatten_mod_document_json_with_diagnostics`) and are PUBLISHED to the T-655 validation
/// panel, which is the render surface and is not duplicated here. This string is only the pointer:
/// it names the count by severity so the toast says something true and finite, and sends the author
/// to the list rather than trying to be the list.
///
/// Empty findings → `None`: a clean compile gets the plain success message it always had, never a
/// celebratory "0 issues" (the panel's own empty-state doctrine).
///
/// Class-R / ungated so native `cargo test` can pin the wording without a browser (the
/// [`compiled_export_text`] precedent).
pub(crate) fn compile_diagnostics_summary(
    findings: &[map_engine_core::mission::validate::Finding],
) -> Option<String> {
    use map_engine_core::mission::validate::Severity;
    if findings.is_empty() {
        return None;
    }
    let count = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    let label = |n: usize, noun: &str| {
        if n == 1 {
            format!("1 {noun}")
        } else {
            format!("{n} {noun}s")
        }
    };
    let mut parts: Vec<String> = Vec::new();
    for (sev, noun) in [
        (Severity::Error, "error"),
        (Severity::Warning, "warning"),
        (Severity::Info, "note"),
    ] {
        let n = count(sev);
        if n > 0 {
            parts.push(label(n, noun));
        }
    }
    Some(format!(
        "The compile reported {} — see the validation panel.",
        parts.join(" · ")
    ))
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

/// T-693 (MENU-SCEN-011) — format a [`MissionDocCore::merge_mission_payload_json`] report for the
/// author: a one-line counts summary + a per-row skipped list. Class-R / ungated so native
/// `cargo test` can pin the wording without a browser (the [`compiled_export_text`] precedent).
///
/// Returns `(summary, skipped_lines)`. `summary` names only the non-zero counts (a merge that added
/// nothing says so), and pluralizes. `skipped_lines` is one `"kind id — reason"` per tolerated
/// malformed row (empty when the merge was clean). A report string that does not parse yields a
/// single skipped line naming that, so the caller never silently swallows a broken report.
pub(crate) fn format_merge_report(report_json: &str) -> (String, Vec<String>) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(report_json) else {
        return (
            "Merge produced no readable report.".to_string(),
            vec![format!("report — could not parse: {report_json}")],
        );
    };
    let n = |k: &str| v.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
    let plural = |count: u64, noun: &str| {
        if count == 1 {
            format!("1 {noun}")
        } else {
            format!("{count} {noun}s")
        }
    };
    let mut parts: Vec<String> = Vec::new();
    let slots = n("slots_added");
    if slots > 0 {
        parts.push(plural(slots, "slot"));
    }
    // Squads / factions: report merged + created distinctly so "grew a side" vs "added a side" reads.
    let sq_created = n("squads_created");
    let sq_merged = n("squads_merged");
    if sq_created > 0 {
        parts.push(plural(sq_created, "squad"));
    }
    if sq_merged > 0 {
        parts.push(format!(
            "{} merged into existing",
            plural(sq_merged, "squad")
        ));
    }
    let fac_created = n("factions_created");
    if fac_created > 0 {
        parts.push(plural(fac_created, "faction"));
    }
    for (key, noun) in [
        ("vehicles_added", "vehicle"),
        ("entities_added", "object"),
        ("zones_added", "zone"),
        ("triggers_added", "trigger"),
        ("compositions_added", "composition"),
        ("markers_added", "marker"),
    ] {
        let c = n(key);
        if c > 0 {
            parts.push(plural(c, noun));
        }
    }

    let summary = if parts.is_empty() {
        "Merge added nothing — the source mission had no mergeable content.".to_string()
    } else {
        format!("Merged {}.", parts.join(", "))
    };

    let skipped: Vec<String> = v
        .get("skipped")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|s| {
                    let kind = s
                        .get("kind")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("row");
                    let id = s
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    let reason = s
                        .get("reason")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("malformed");
                    if id.is_empty() {
                        format!("{kind} — {reason}")
                    } else {
                        format!("{kind} {id} — {reason}")
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    (summary, skipped)
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::RefCell;

    use leptos::prelude::{GetUntracked, RwSignal, Set};
    use leptos::task::spawn_local;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    use map_engine_core::mission::compile::{compile_export, compile_payload, version_body};
    use map_engine_core::mission::flatten::{
        flatten_mod_document_json_with_diagnostics, MissionMeta,
    };
    use map_engine_core::mission::validate::Finding;

    /// T-690 — what a compile hands the command layer: the download text and the structured
    /// findings, from one compile. Aliased so the entry point's signature stays on one line, which
    /// is what `class_r_source_forbids_value_pretty_on_compiled_export` locates it by.
    type CompiledWithDiagnostics = (String, Vec<Finding>);

    use crate::auth::AuthStore;
    use crate::mission_doc::DocHandle;

    use super::{compiled_export_text, format_merge_report, row_meta_missing_message};

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
        compiled_document_json_with_diagnostics().map(|(text, _)| text)
    }

    /// T-690 — the compile, with the structured result it produced ALONGSIDE the bytes.
    ///
    /// This is the body [`compiled_document_json`] projects: one compile, one document, one finding
    /// list. Splitting it the other way round (a second compile just for the findings) is the shape
    /// `flatten_mod_document_json_full` exists to forbid — two compiles are two things that can
    /// disagree about what was compiled.
    ///
    /// The findings are `map_engine_core::mission::validate::Finding`s — the T-657 vocabulary
    /// (`rule_id` / `severity` / `primitive` / `message` / `subject` / `subject_id`), reused so the
    /// T-655 panel renders a compile finding through exactly the same row as a validation finding
    /// and click-to-select works on both.
    ///
    /// # Errors
    /// Same three refusals as [`compiled_document_json`] — a missing row, an unmounted editor, or a
    /// compile that produced no document. A FINDING is never one of them.
    pub fn compiled_document_json_with_diagnostics() -> Result<CompiledWithDiagnostics, String> {
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

        let (doc, findings) =
            flatten_mod_document_json_with_diagnostics(&meta_bytes, &payload_bytes)?;
        // T-417 — ship the compact wire bytes (byte-identical to `/compiled`). Do not re-parse to
        // `serde_json::Value` for a "pretty" download — that is not whitespace-only vs the route.
        compiled_export_text(&doc).map(|text| (text, findings))
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

    /// Trigger the server-truth download and report the outcome (T-243).
    ///
    /// **T-690 — this is where the compile stops being a pass/fail.** The compile now returns a
    /// structured finding list alongside the bytes; this publishes that list to the T-655 validation
    /// panel ([`crate::validation_panel::publish_compile_findings`]) and lets the toast shrink back
    /// to what a toast is good at — a one-line verdict with a pointer. The panel is the render
    /// surface and is deliberately not duplicated here.
    ///
    /// The publish happens even when the list is EMPTY, and that is load-bearing: a clean compile
    /// must CLEAR the previous compile's findings, or the panel would show a stale build report
    /// after the author fixed everything in it.
    ///
    /// `toasts` is resolved at component setup by the caller — `use_toasts()` is an `expect_context`
    /// and would panic from a DOM handler, the `RowMirror` precedent.
    pub fn export_compiled_now(toasts: crate::toast::Toasts) {
        let mission_id = EDITOR_CTX
            .with(|c| c.borrow().as_ref().map(|ctx| ctx.mission_id.clone()))
            .unwrap_or_default();
        match compiled_document_json_with_diagnostics() {
            Ok((json, findings)) => {
                let filename = format!("mission-{mission_id}.compiled.json");
                if let Err(e) = download_json(&filename, &json) {
                    toasts.error(format!("Could not start the download: {e:?}"));
                    return;
                }
                // The findings reach the panel through the engine's own row type, so a compile
                // finding renders — and click-to-selects on its `subject_id` — exactly like a
                // validation finding. Published AFTER the download starts: a diagnostic is not a
                // refusal, and the file the author asked for is not held back by one.
                let summary = super::compile_diagnostics_summary(&findings);
                crate::validation_panel::publish_compile_findings(
                    findings
                        .iter()
                        .map(crate::validation_panel::PanelFinding::from_finding)
                        .collect(),
                );
                // Naming the staleness is the whole reason this is a toast and not a silent download:
                // the file is the CURRENT document, which is only what a game server would fetch once
                // this state is saved.
                let staleness = if crate::mission_history::is_dirty() {
                    Some(
                        "Downloaded the compiled mission document — compiled from your unsaved \
                         changes, so the server still serves the last saved version.",
                    )
                } else {
                    None
                };
                match (staleness, summary) {
                    (Some(stale), Some(s)) => toasts.message(format!("{stale} {s}")),
                    (Some(stale), None) => toasts.message(stale.to_string()),
                    (None, Some(s)) => {
                        toasts.message(format!("Downloaded the compiled mission document. {s}"))
                    }
                    (None, None) => toasts.success("Downloaded the compiled mission document."),
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
                    // flag and update the current-semver signal so a later Export/adopt uses it.
                    // (T-370 removed the `editor_session::mark_adopted` call that sat here: T-352
                    // had already emptied it, and T-223 replaced the semver marker it once wrote
                    // with the content test in `mission_hydrate::classify_local`.)
                    crate::mission_history::set_dirty(false);
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

    /// T-693 (MENU-SCEN-011) — one row in the "Merge Mission…" picker: a mission the author can merge
    /// FROM. `id` feeds [`merge_mission_now`]; `title` is the label.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct MissionPick {
        /// The mission id (`GET /missions/:id`).
        pub id: String,
        /// The mission's display title.
        pub title: String,
    }

    /// T-693 — the author's OTHER missions, for the "Merge Mission…" picker.
    ///
    /// Reuses the SPA's own list client (`GET /missions?scope=mine`, the same call
    /// `missions::MissionLibraryPage` makes) through [`crate::client::api_get`], which owns the
    /// single-flight refresh — so this adds no second auth path. The CURRENT mission is filtered out
    /// (you cannot merge a mission into itself). Titles come straight off the `MissionCard` rows.
    ///
    /// # Errors
    /// A display string when the list request fails (offline / 401 / server error).
    pub async fn other_missions(
        auth: AuthStore,
        exclude_id: &str,
    ) -> Result<Vec<MissionPick>, String> {
        use crate::dto::{MissionCard, Paginated};
        match crate::client::api_get::<Paginated<MissionCard>>(auth, "/missions?scope=mine").await {
            Ok(page) => Ok(page
                .data
                .into_iter()
                .filter(|c| c.id != exclude_id)
                .map(|c| MissionPick {
                    id: c.id,
                    title: c.title,
                })
                .collect()),
            Err((401, _)) => Err("Sign in to list your missions.".to_string()),
            Err((s, msg)) => Err(match msg {
                Some(m) if !m.is_empty() => format!("Could not load your missions ({s}): {m}"),
                _ => format!("Could not load your missions ({s})."),
            }),
        }
    }

    /// T-693 (MENU-SCEN-011) — merge another mission (`source_id`) into the CURRENT document.
    ///
    /// Fetches the source's latest payload (`GET /missions/:id` → `current_version.json_payload`, the
    /// same superset [`crate::mission_hydrate`] loads), runs [`MissionDocCore::merge_mission_payload_json`]
    /// on the hosted doc, and reports the outcome via toasts: a counts line plus, when the merge
    /// tolerated malformed rows, an error toast listing each skipped row (the T-657 totality contract
    /// made visible to the author). `offset` is the optional template placement delta.
    ///
    /// The whole merge is one undo step (the core opens one txn), so a mistaken merge is one Ctrl+Z.
    /// The borrow of the hosted `MissionDocCore` is taken and released synchronously AFTER the
    /// `.await` — never held across the yield (the module's borrow-safety contract).
    pub fn merge_mission_now(
        source_id: String,
        offset: Option<(f64, f64)>,
        toasts: crate::toast::Toasts,
    ) {
        let Some((doc, auth)) =
            EDITOR_CTX.with(|c| c.borrow().as_ref().map(|ctx| (ctx.doc.clone(), ctx.auth)))
        else {
            toasts.error("Editor not ready.");
            return;
        };
        let path = format!("/missions/{source_id}");
        spawn_local(async move {
            let detail =
                match crate::client::api_get::<crate::dto::MissionDetail>(auth, &path).await {
                    Ok(d) => d,
                    Err((401, _)) => {
                        toasts.error("Sign in to merge a mission.");
                        return;
                    }
                    Err((404, _)) => {
                        toasts.error("That mission no longer exists.");
                        return;
                    }
                    Err((s, _)) => {
                        toasts.error(format!("Could not load the mission to merge ({s})."));
                        return;
                    }
                };
            // The editor superset lives in `current_version.json_payload`; an empty `{}` (a
            // never-saved source) has nothing to merge — say so rather than run an empty merge.
            let payload = detail.current_version.as_ref().map(|v| &v.json_payload);
            let is_empty =
                payload.is_none_or(|p| p.as_object().is_none_or(serde_json::Map::is_empty));
            if is_empty {
                toasts.message(format!(
                    "\"{}\" has no saved content to merge yet.",
                    detail.title
                ));
                return;
            }
            let payload_json = payload
                .map(std::string::ToString::to_string)
                .unwrap_or_default();

            // Borrow the doc only now (post-await), run the merge, drop the borrow before toasting.
            let report_json = {
                let borrow = doc.borrow();
                let Some(core) = borrow.as_ref() else {
                    toasts.error("Editor not ready.");
                    return;
                };
                core.merge_mission_payload_json(&payload_json, offset)
            };
            // A merge is a document mutation, so it must run the SAME post-mutation tail every editor
            // mutator ends on (`editor_ops` mutators all call this): materialize → prune the selection
            // → rebind the engine slot/vehicle glyphs so the merged rows reach the GPU (they are
            // invisible on the map otherwise) → bump `doc_ver` (which drives the validation panel's
            // re-check and the attributes re-read) → set dirty → schedule the IDB persist → refresh the
            // HUD counts. `set_dirty(true)` alone (the prior code) did only the last-but-two of those,
            // so a wired merge reported success by toast while the map showed nothing. The `EDITOR_CTX`
            // doc borrow was dropped at the end of the block above; `after_local_edit` takes its own
            // `HISTORY_CTX` borrow, so this is not held across the earlier `.await`.
            crate::mission_history::after_local_edit();

            let (summary, skipped) = format_merge_report(&report_json);
            toasts.success(format!("{summary} (Ctrl+Z to undo.)"));
            if !skipped.is_empty() {
                let head = if skipped.len() == 1 {
                    "1 row was skipped:".to_string()
                } else {
                    format!("{} rows were skipped:", skipped.len())
                };
                toasts.error(format!("{head} {}", skipped.join("; ")));
            }
        });
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

        // T-690 — the compile's structured findings as JSON, so a harness can read back what the
        // build step LEARNED and not only what it emitted. Same argument as `compiled_document_json`
        // above: a result whose only exit is a floating card is a result no harness can check.
        // Returns `[{ruleId, severity, primitive, message, subject, subjectId}]`, `[]` on a clean
        // compile, and `{"error": "…"}` on a refusal — the three cases are distinguishable by shape.
        let compiled_diags = Closure::wrap(Box::new(move || -> JsValue {
            let out = match compiled_document_json_with_diagnostics() {
                Ok((_, findings)) => {
                    let rows: Vec<serde_json::Value> = findings
                        .iter()
                        .map(|f| {
                            serde_json::json!({
                                "ruleId": f.rule_id,
                                "severity": f.severity.as_str(),
                                "primitive": f.primitive.tag(),
                                "message": f.message,
                                "subject": f.subject,
                                "subjectId": f.subject_id,
                            })
                        })
                        .collect();
                    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
                }
                Err(e) => serde_json::json!({ "error": e }).to_string(),
            };
            JsValue::from_str(&out)
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
        // T-693 — merge a payload JSON string into the hosted doc and return the report JSON. Unlike
        // its read-only peers this one MUTATES (the merge is one undo step in-core), so a smoke that
        // calls it should undo after. Takes the payload as a JS string arg; no offset (the harness
        // exercises the authored-position path). Returns the [`MergeReport`] JSON.
        let merge_fn = {
            let doc = doc.clone();
            Closure::wrap(Box::new(move |payload: JsValue| -> JsValue {
                let payload_json = payload.as_string().unwrap_or_default();
                let out = doc
                    .borrow()
                    .as_ref()
                    .map(|c| c.merge_mission_payload_json(&payload_json, None))
                    .unwrap_or_default();
                JsValue::from_str(&out)
            }) as Box<dyn FnMut(JsValue) -> JsValue>)
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
        let _ = js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("compiled_diagnostics_json"),
            compiled_diags.as_ref(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("merge_mission_json"),
            merge_fn.as_ref(),
        );
        if let Some(win) = web_sys::window() {
            let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCommands"), &obj);
        }
        // Leaked like the other editor bridges (harness reads them across the page lifetime).
        compile_save.forget();
        compile_export_fn.forget();
        compiled_doc.forget();
        compiled_diags.forget();
        merge_fn.forget();
    }
}

#[cfg(target_arch = "wasm32")]
pub use imp::*;

#[cfg(test)]
mod tests {
    use super::{compiled_export_text, format_merge_report, row_meta_missing_message};

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
    ///
    /// **T-690 moved the needle one function down, and did not loosen it.** The transport body now
    /// lives in `compiled_document_json_with_diagnostics` (the compile returns findings alongside
    /// the bytes, and `compiled_document_json` is a projection of it — one compile, not two). So the
    /// pin reads the body that actually calls the compile, AND asserts the projection is thin: if
    /// `compiled_document_json` ever grows its own compile again, the second assertion fires and the
    /// two paths can no longer drift into shipping different bytes.
    #[test]
    fn class_r_source_forbids_value_pretty_on_compiled_export() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        const SRC: &str = include_str!("mission_commands.rs");
        let production = live_code(SRC);
        let code = only_body(
            &production,
            "pub fn compiled_document_json_with_diagnostics()",
        );
        assert!(
            code.contains("compiled_export_text(&doc)"),
            "the compile path must ship via compiled_export_text"
        );
        assert!(
            !code.contains("to_string_pretty"),
            "the compile path must not pretty-print the compiled doc"
        );
        // A live `serde_json::Value` binding in the return path is the old defect.
        assert!(
            !code.contains("let value: serde_json::Value"),
            "the compile path must not re-parse through Value for download"
        );
        // …and the plain entry point is a PROJECTION of it, never a second compile.
        let plain = only_body(&production, "pub fn compiled_document_json()");
        assert!(
            plain.contains("compiled_document_json_with_diagnostics()"),
            "compiled_document_json must project the diagnostics body, not compile again; got:\n{plain}"
        );
        assert!(
            !plain.contains("flatten_mod_document_json"),
            "compiled_document_json must not run its own compile; got:\n{plain}"
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

    /// T-693 Class-R — the report formatter names the non-zero counts, distinguishes squads
    /// merged-into-existing from squads created, and lists each skipped row.
    #[test]
    fn class_r_merge_report_formats_counts_and_skips() {
        let report = r#"{
            "slots_added": 3, "squads_merged": 1, "squads_created": 2,
            "factions_merged": 1, "factions_created": 0, "vehicles_added": 1,
            "entities_added": 0, "zones_added": 0, "triggers_added": 1,
            "compositions_added": 0, "markers_added": 0,
            "skipped": [{"kind":"slot","id":"","reason":"missing id"}]
        }"#;
        let (summary, skipped) = format_merge_report(report);
        assert!(summary.contains("3 slots"), "counts slots: {summary}");
        assert!(
            summary.contains("2 squads"),
            "counts created squads: {summary}"
        );
        assert!(
            summary.contains("1 squad merged into existing"),
            "names merged squads distinctly: {summary}"
        );
        assert!(summary.contains("1 vehicle"), "singular vehicle: {summary}");
        assert!(summary.contains("1 trigger"), "counts triggers: {summary}");
        assert!(
            !summary.contains("faction"),
            "zero created factions omitted from the summary: {summary}"
        );
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0], "slot — missing id");
    }

    /// T-693 Class-R — an empty merge says so rather than "Merged .".
    #[test]
    fn class_r_merge_report_empty_is_named() {
        let report = r#"{"slots_added":0,"squads_merged":0,"squads_created":0,
            "factions_merged":0,"factions_created":0,"vehicles_added":0,"entities_added":0,
            "zones_added":0,"triggers_added":0,"compositions_added":0,"markers_added":0,"skipped":[]}"#;
        let (summary, skipped) = format_merge_report(report);
        assert!(
            summary.contains("added nothing"),
            "empty merge named: {summary}"
        );
        assert!(skipped.is_empty());
    }

    /// T-693 Class-R — an unparseable report degrades to a named skip, never a silent success.
    #[test]
    fn class_r_merge_report_unparseable_degrades() {
        let (summary, skipped) = format_merge_report("{not json");
        assert!(
            summary.contains("no readable report"),
            "unparseable named: {summary}"
        );
        assert_eq!(skipped.len(), 1);
        assert!(skipped[0].contains("could not parse"));
    }

    /// T-693 Class-R (MAJOR-1) — **`merge_mission_now` runs the full post-mutation tail.** A merge is a
    /// document mutation; ending it on `set_dirty(true)` alone (the prior code) skipped the engine
    /// rebind (merged rows never reached the GPU — invisible on the map), the `doc_ver` bump (the
    /// validation panel never re-checked), and the persist schedule. The tail is `after_local_edit`,
    /// the same call every `editor_ops` mutator ends on. This pins the live source (the function is
    /// wasm-gated, so there is no native runtime seam — the source is the contract) through the
    /// `class_r_scrub` extractor, so a regression to a bare `set_dirty` re-arms the bug loudly. The
    /// scrubber blanks comments, so the `set_dirty(true)` named in this function's doc-comment cannot
    /// satisfy the needle — only a live call can.
    #[test]
    fn class_r_merge_mission_now_runs_the_after_local_edit_tail() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        const SRC: &str = include_str!("mission_commands.rs");
        let production = live_code(SRC);
        let code = only_body(&production, "pub fn merge_mission_now");
        assert!(
            code.contains("after_local_edit"),
            "merge_mission_now must run the post-mutation tail (after_local_edit), not just set_dirty"
        );
        // The prior defect: the tail was a bare `set_dirty(true)` and nothing else. `after_local_edit`
        // already sets dirty (via `after_doc_change`), so a live `set_dirty(true)` here would be the
        // regression — the merge doing only the dirty flag again.
        assert!(
            !code.contains("set_dirty(true)"),
            "merge_mission_now must not end on a bare set_dirty(true) — after_local_edit sets dirty"
        );
    }

    /* ══════════ T-690 — the compile's structured result ══════════ */

    use map_engine_core::mission::validate::{Finding, Primitive, Severity};

    fn finding(rule_id: &'static str, severity: Severity, subject_id: Option<&str>) -> Finding {
        Finding {
            rule_id,
            severity,
            primitive: Primitive::PerObjectInvariant,
            message: "the compile dropped a value".to_string(),
            subject: "/editor/slots/0/rank".to_string(),
            subject_id: subject_id.map(ToString::to_string),
        }
    }

    /// A clean compile gets the message it always had — never a celebratory "0 issues".
    #[test]
    fn a_clean_compile_produces_no_diagnostics_summary() {
        assert_eq!(super::compile_diagnostics_summary(&[]), None);
    }

    /// The toast shrinks to a verdict + a pointer: counts by severity, worst first, only the
    /// non-zero rungs, correctly pluralised, and it names where the list actually lives.
    #[test]
    fn the_diagnostics_summary_counts_by_severity_and_points_at_the_panel() {
        let findings = [
            finding("COMPILE-DROP-SQUAD-LEADER", Severity::Warning, Some("sq1")),
            finding("COMPILE-DROP-SLOT-RANK", Severity::Info, Some("s1")),
            finding("COMPILE-DROP-SLOT-TAG", Severity::Info, Some("s1")),
        ];
        let s = super::compile_diagnostics_summary(&findings).expect("some findings");
        assert!(s.contains("1 warning"), "{s}");
        assert!(s.contains("2 notes"), "{s}");
        assert!(!s.contains("error"), "no zero-count rung may appear: {s}");
        assert!(
            s.contains("validation panel"),
            "the toast must point at the render surface rather than try to be it: {s}"
        );
    }

    /// **The feed.** The compile's findings reach the T-655 panel through the panel's OWN row type,
    /// so a compile finding renders — and click-to-selects on its `subject_id` — exactly like a
    /// validation finding. No second panel, no parallel vocabulary.
    #[test]
    fn compile_findings_reach_the_validation_panel() {
        use crate::validation_panel::{
            evaluate_now, publish_compile_findings, PanelFinding, Rollup,
        };

        // Baseline: nothing published, nothing shown (no payload source is registered on the host).
        publish_compile_findings(Vec::new());
        assert!(
            Rollup::of(&evaluate_now()).is_empty(),
            "the panel starts empty"
        );

        let findings = [
            finding("COMPILE-DROP-SQUAD-LEADER", Severity::Warning, Some("sq1")),
            finding("COMPILE-DROP-SLOT-RANK", Severity::Info, Some("s1")),
        ];
        publish_compile_findings(findings.iter().map(PanelFinding::from_finding).collect());

        let rows = evaluate_now();
        assert_eq!(rows.len(), 2, "{rows:?}");
        let rollup = Rollup::of(&rows);
        assert_eq!((rollup.errors, rollup.warnings, rollup.infos), (0, 1, 1));
        assert_eq!(rollup.chip_text(), "1 warning · 1 info");
        // The owning entity id survived, which is what makes the row clickable — the T-657
        // `subject_id` vocabulary reused rather than a parallel one invented.
        let leader = rows
            .iter()
            .find(|r| r.rule_id == "COMPILE-DROP-SQUAD-LEADER")
            .expect("the leader finding rendered");
        assert_eq!(leader.subject_id.as_deref(), Some("sq1"));
        assert!(leader.is_selectable());

        // A clean compile CLEARS the previous build report — otherwise the panel would show a stale
        // list after the author fixed everything in it.
        publish_compile_findings(Vec::new());
        assert!(
            Rollup::of(&evaluate_now()).is_empty(),
            "a clean compile must clear the previous compile's findings"
        );
    }

    /// Class-R — the export path FEEDS the shipped panel and does not grow one of its own.
    ///
    /// The ticket's own constraint ("owns only the compiler and the command layer so the panel stays
    /// a single claimant") is the kind that decays silently: a second list rendered next to the
    /// download button would look fine and would be a second claimant. This reads the live body.
    #[test]
    fn class_r_the_export_publishes_to_the_panel_and_builds_no_second_one() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        const SRC: &str = include_str!("mission_commands.rs");
        let production = live_code(SRC);
        let code = only_body(&production, "pub fn export_compiled_now(");
        assert!(
            code.contains("validation_panel::publish_compile_findings("),
            "export_compiled_now must publish the compile's findings to the T-655 panel; got:\n{code}"
        );
        assert!(
            code.contains("compiled_document_json_with_diagnostics()"),
            "export_compiled_now must take the bytes AND the findings from one compile; got:\n{code}"
        );
        // A `view!` here would be a second render surface for the same findings.
        assert!(
            !code.contains("view!"),
            "export_compiled_now must not render a panel of its own; got:\n{code}"
        );
        // …and the summary must not try to be the list: no per-finding message in the toast.
        assert!(
            !code.contains("f.message"),
            "the toast is a pointer, not the list; got:\n{code}"
        );
    }
}
