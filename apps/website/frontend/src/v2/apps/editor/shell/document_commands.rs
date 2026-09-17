//! Browser transport for editor save, export, merge, and clipboard commands.
//! The map engine decides document content; this module handles API requests, downloads,
//! clipboard promises, toasts, and the editor command bridge.

pub use website_map_engine::editing::commands::export_text::{
    apply_row_metadata_to_export, compile_diagnostics_summary, compiled_export_text,
    export_gesture_is_duplicate, live_doc_title, row_meta_missing_message,
};
pub use website_map_engine::editing::commands::merge_report::{
    duplicate_slot_id_report, format_merge_report,
};
pub use website_map_engine::editing::commands::selection_digest::{
    classnames_text, count_noun, grid_position_text, resolve_selected_entities,
    selection_summary_text, SelectedEntity,
};

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::RefCell;

    use crate::v2::core::ui::toast::Toasts;
    use leptos::prelude::{GetUntracked, RwSignal, Set};
    use leptos::task::spawn_local;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    use website_map_engine::data::scenario::compile::compile_export;
    use website_map_engine::data::scenario::compile::compile_payload;
    use website_map_engine::data::scenario::compile::version_body;
    use website_map_engine::data::scenario::flatten::flatten_mod_document_json_with_diagnostics;
    use website_map_engine::data::scenario::flatten::MissionMeta;
    use website_map_engine::data::scenario::validate::Finding;

    /// what a compile hands the command layer: the download text and the structured
    /// findings, from one compile. Aliased so the entry point's signature stays on one line, which
    /// is what `class_r_source_forbids_value_pretty_on_compiled_export` locates it by.
    type CompiledWithDiagnostics = (String, Vec<Finding>);

    use crate::v2::apps::editor::bridge::document_host::doc_host::DocHandle;
    use crate::v2::core::auth::AuthStore;

    use super::{
        compiled_export_text, format_merge_report, resolve_selected_entities,
        row_meta_missing_message, SelectedEntity,
    };

    /// Editor context shared from `mission_editor::on_load` to the Save/Export buttons. `AuthStore` is
    /// `Copy`; `doc` is the same shared `Rc` the persistence layer may swap on IDB restore (reads see the
    /// swap). Held in a `thread_local` because `DocHandle` is `!Send` + wasm-only.
    struct EditorCtx {
        doc: DocHandle,
        auth: AuthStore,
        mission_id: String,
        /// the adopted server semver signal, updated on a successful Save (the saved
        /// version becomes the version local now derives from).
        current_semver: RwSignal<Option<String>>,
    }

    thread_local! {
        static EDITOR_CTX: RefCell<Option<EditorCtx>> = const { RefCell::new(None) };

        /// ** the mission ROW, as `GET /missions/:id` last served it.**
        ///
        /// Deliberately NOT part of [`EditorCtx`]: `set_ctx` runs synchronously at mount, and this
        /// arrives later from `mission_hydrate::hydrate_from_server`'s `await`. Folding it in would
        /// have meant either an `Option` field nobody could keep honest or an ordering assumption
        /// between a mount and a fetch.
        ///
        /// **`None` is load-bearing and must stay refusable.** It means the row never arrived  a
        /// local-only / non-UUID id (the `smoke` gate route), a 404, an offline boot, **or a 401 /
        /// expired session** (hydrate never got the row; see [`row_meta_missing_message`]). There is
        /// no server document for those, and `MissionMeta::default()` would happily compile one with a
        /// blank author and `playerRange: [1, 1]`. Emitting that under the name "the document the game
        /// server will receive" is the confident-wrong-answer failure this whole ticket exists to
        /// avoid, so [`export_compiled_now`] refuses instead  and names auth failure separately from
        /// "no saved version".
        static ROW_META: RefCell<Option<MissionMeta>> = const { RefCell::new(None) };

        /// ** the missions-row columns [`MissionMeta`] deliberately omits.** `compiled_meta()`
        /// keeps `max_players` for the flatten but drops `game_mode` (and never carried the library
        /// `briefing` / `thumbnail_url`). That omission is why `eden_settings::ShapeMirror` had to
        /// invent a second GET on every dialog open. Same boot hydrate fills both cells; successful
        /// shape PATCHes refresh this one so a reopen mid-flight is not the only path to the new mode.
        static ROW_HYDRATE: RefCell<Option<HydratedRow>> = const { RefCell::new(None) };

        /// ** the last export activation's `Event.timeStamp`.** The single-cell state
        /// behind [`super::export_gesture_is_duplicate`]: [`begin_export_gesture`] records the stamp
        /// of the activation it lets through, and rejects the next one only when it carries the SAME
        /// stamp (the DOM's synthesised pointerup/click double). A `Cell` (not a `RefCell`) because a
        /// single `f64` copy needs no borrow, and a `thread_local` for the same wasm-single-thread
        /// reason [`MIRROR`](crate::v2::apps::editor::ui::docks::top_strip) is.
        static LAST_EXPORT_STAMP: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    }

    /// shape/presentation columns from the missions row, beside [`ROW_META`].
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct HydratedRow {
        pub game_mode: String,
        pub max_players: i64,
        pub briefing: String,
        pub thumbnail_url: String,
    }

    /// Record the mission row for the server-truth Export (). Called by
    /// `mission_hydrate::hydrate_from_server` on every successful `GET /missions/:id`, including the
    /// fresh-mission and warm-IDB branches  the row is what the compile needs, and it is equally real
    /// whichever way the payload half was resolved.
    ///
    /// ** also records [`HydratedRow`].** `MissionMeta` still has no `game_mode`; the hydrate
    /// cell is the getter-facing half so ShapeMirror can seed without inventing values.
    pub fn set_row_meta(detail: &crate::v2::core::api::dto::MissionDetail) {
        ROW_META.with(|r| *r.borrow_mut() = Some(detail.compiled_meta()));
        ROW_HYDRATE.with(|h| {
            *h.borrow_mut() = Some(HydratedRow {
                game_mode: detail.game_mode.clone(),
                max_players: detail.max_players,
                briefing: detail.briefing.clone().unwrap_or_default(),
                thumbnail_url: detail.thumbnail_url.clone().unwrap_or_default(),
            });
        });
    }

    /// `max_players` from boot hydrate / last successful row GET. `None` when [`ROW_META`]
    /// never arrived (same refuse conditions as Export Compiled).
    pub(crate) fn row_max_players() -> Option<i64> {
        ROW_META.with(|r| r.borrow().as_ref().map(|m| m.max_players))
    }

    /// game mode + presentation columns retained beside [`ROW_META`].
    pub(crate) fn hydrated_row() -> Option<HydratedRow> {
        ROW_HYDRATE.with(|h| h.borrow().clone())
    }

    /// a successful shape open-GET (or a full row refresh) replaces the hydrate cell.
    pub(crate) fn note_hydrated_row(row: HydratedRow) {
        ROW_HYDRATE.with(|h| *h.borrow_mut() = Some(row));
    }

    /// a successful `game_mode` PATCH keeps the hydrate cell honest without waiting for reopen.
    pub(crate) fn note_hydrated_game_mode(game_mode: &str) {
        ROW_HYDRATE.with(|h| {
            if let Some(row) = h.borrow_mut().as_mut() {
                row.game_mode = game_mode.to_string();
            }
        });
    }

    /// a successful briefing / thumbnail PATCH updates only the column that landed.
    pub(crate) fn note_hydrated_presentation(briefing: Option<&str>, thumbnail_url: Option<&str>) {
        ROW_HYDRATE.with(|h| {
            if let Some(row) = h.borrow_mut().as_mut() {
                if let Some(b) = briefing {
                    row.briefing = b.to_string();
                }
                if let Some(t) = thumbnail_url {
                    row.thumbnail_url = t.to_string();
                }
            }
        });
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

    /// An owned snapshot of everything a command needs  taken synchronously so no borrow spans an
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

    /// the live document's duplicate (callsign, slot id) pairs, or empty when
    /// there is no editor context yet.
    ///
    /// Separate from [`snapshot`] because `Snap` is a VALUE snapshot (JSON strings) and
    /// [`website_map_engine::data::store::operations::slot_ids::duplicate_slot_ids`] takes the
    /// `MissionDocCore` itself  it needs `doc.slot_exists`, which the JSON alone cannot answer.
    /// Same one-borrow discipline as `snapshot`: one `EDITOR_CTX` borrow, released before the
    /// caller does anything else.
    fn live_duplicate_slot_ids() -> Vec<(String, String)> {
        EDITOR_CTX.with(|c| {
            let ctx = c.borrow();
            let Some(ctx) = ctx.as_ref() else {
                return Vec::new();
            };
            let doc = ctx.doc.borrow();
            let Some(core) = doc.as_ref() else {
                return Vec::new();
            };
            website_map_engine::data::store::operations::slot_ids::duplicate_slot_ids(core)
        })
    }

    mod compilation;
    use compilation::*;
    pub use compilation::*;
    mod exports;
    use exports::*;
    pub use exports::*;
    mod mission_saving;
    use mission_saving::*;
    pub use mission_saving::*;
    mod mission_merge;
    use mission_merge::*;
    pub use mission_merge::*;
    mod clipboard;
    use clipboard::*;
    pub use clipboard::*;

    /// Install `window.__editorCommands`  the read-only compile smoke bridge (peer of `__missionDoc`,
    /// same leaked-closure `js_sys::Object` idiom as `register_mission_doc`). `compile_save_json()` and
    /// `compile_export_json()` return the compiled JSON strings; the export path pins `exportedAt` +
    /// `missionId`/`version` to fixed values so the gate output is byte-deterministic.
    ///
    /// ** adds `compiled_document_json()`** the same bytes the "Export Compiled" button
    /// downloads. It is on the bridge for the reason the other two are: a compile whose only entry
    /// point is a `<button>` and a `Blob` download is a compile no harness can read back, and this one
    /// makes a claim worth checking against a live `GET /missions/:id/compiled`. Unlike its two peers
    /// it pins nothing: its whole value is being the real output. On a failure it returns the same
    /// author-facing message the toast shows (a plain string either way  the caller can tell them
    /// apart by parsing).
    pub fn register_editor_commands(doc: DocHandle) {
        let obj = js_sys::Object::new();

        let compiled_doc = Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&compiled_document_json().unwrap_or_else(|e| e))
        }) as Box<dyn FnMut() -> JsValue>);

        // the compile's structured findings as JSON, so a harness can read back what the
        // build step LEARNED and not only what it emitted. Same argument as `compiled_document_json`
        // above: a result whose only exit is a floating card is a result no harness can check.
        // Returns `[{ruleId, severity, primitive, message, subject, subjectId}]`, `[]` on a clean
        // compile, and `{"error": "…"}` on a refusal  the three cases are distinguishable by shape.
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
        // merge a payload JSON string into the hosted doc and return the report JSON. Unlike
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

        // one closure per exporter over the shared [`export_preview_json`] reader.
        let clipboard_grid = Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&export_preview_json("grid"))
        }) as Box<dyn FnMut() -> JsValue>);
        let clipboard_classnames = Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&export_preview_json("classnames"))
        }) as Box<dyn FnMut() -> JsValue>);
        let clipboard_summary = Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&export_preview_json("summary"))
        }) as Box<dyn FnMut() -> JsValue>);

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
        // the three clipboard exporters, readable. Same argument as `compiled_document_json`
        // above: an exporter whose only exit is a `navigator.clipboard` write is an exporter no
        // harness can read back, and the clipboard is not readable in a headless gate.
        for (name, closure) in [
            ("clipboard_grid_json", &clipboard_grid),
            ("clipboard_classnames_json", &clipboard_classnames),
            ("clipboard_summary_json", &clipboard_summary),
        ] {
            let _ = js_sys::Reflect::set(&obj, &JsValue::from_str(name), closure.as_ref());
        }
        if let Some(win) = web_sys::window() {
            let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCommands"), &obj);
        }
        // Leaked like the other editor bridges (harness reads them across the page lifetime).
        compile_save.forget();
        compile_export_fn.forget();
        compiled_doc.forget();
        compiled_diags.forget();
        merge_fn.forget();
        clipboard_grid.forget();
        clipboard_classnames.forget();
        clipboard_summary.forget();
    }
}

#[cfg(target_arch = "wasm32")]
pub use imp::*;

/// The grid-reference exporter is pinned against the map furniture's own edge labels, which only
/// the frontend draws  so the pin sits here, on the side of the wall that can read both.
#[cfg(test)]
#[path = "tests/exporter_grid_reference.rs"]
mod exporter_grid_reference_tests;

#[cfg(test)]
#[path = "tests/document_commands/source_contracts.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/document_commands/duplicate_slot_guard.rs"]
mod t946_86_duplicate_guard;
